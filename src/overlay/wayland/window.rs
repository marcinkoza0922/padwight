//! The layer surface: binding the globals, making the surface, and every commit made to it.
//! Each commit goes through here, so the rule that a buffer only goes on at the size last acked
//! is checked in one place (`Sizing::attached`), as is the rule that a frame is drawn only into
//! a buffer the compositor has released (`Slots::check_free`), and each is logged. Each frame
//! drawn asks for a frame callback in its own commit (`Pacer`).

use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use tiny_skia::PixmapMut;
use wayland_client::{
    QueueHandle,
    globals::GlobalList,
    protocol::{
        wl_compositor::WlCompositor,
        wl_shm::{self, WlShm},
        wl_surface::WlSurface,
    },
};
use wayland_protocols::wp::{
    fractional_scale::v1::client::{wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, wp_fractional_scale_v1::WpFractionalScaleV1},
    viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
};

use super::{
    picture::Screen,
    pacing::Pacer,
    scale::{Mode, Scale},
    shm::{Formats, Pool, rgba_to_argb},
    sizing::{Commit, Sizing, Target},
    slots::Layout,
};
use crate::overlay::{
    timing::{timing_log, unix_ns},
    ui::Overlay,
};

/// The surface and what it needs, also the state the protocol's events are dispatched to.
pub(super) struct Window {
    pub(super) shm: WlShm,
    pub(super) formats: Formats,
    pub(super) surface: WlSurface,
    pub(super) layer: ZwlrLayerSurfaceV1,
    /// Kept alive: it is what sends the preferred scale.
    _fractional: Option<WpFractionalScaleV1>,
    viewport: Option<WpViewport>,
    /// The scale the compositor prefers; buffers are made at it.
    pub(super) scale: Scale,
    /// The scale changed since the last commit: what is on needs drawing again at the new one.
    pub(super) rescaled: bool,
    pub(super) sizing: Sizing,
    /// What the last configure asks to commit, for the loop to do.
    pub(super) todo: Commit,
    /// The buffers, at the size last committed: the screen's while shown, 1×1 while hidden.
    pub(super) pool: Option<Pool>,
    /// The number the last pool made got, so a release meant for an older pool is passed over.
    pools: u64,
    /// When to draw: on frame callbacks while anything moves.
    pub(super) pacer: Pacer,
    /// The compositor closed the surface.
    pub(super) closed: bool,
}

/// Why a buffer is committed, for the log.
#[derive(Debug, Clone, Copy)]
enum Why {
    Draw,
    Clear,
    Blank,
}

impl Window {
    /// Binds the globals and makes the surface: on the overlay layer, 1×1 in a corner, never
    /// taking keyboard focus, with clicks passing through. It has its first commit, without a
    /// buffer, and waits for its first configure.
    pub(super) fn new(globals: &GlobalList, qh: &QueueHandle<Window>) -> Result<Window> {
        let layer_shell: ZwlrLayerShellV1 = globals.bind(qh, 1..=4, ()).map_err(|_| {
            anyhow!("the overlay window could not start: the compositor lacks wlr-layer-shell (as GNOME does)")
        })?;
        let compositor: WlCompositor = globals.bind(qh, 4..=6, ()).context("the compositor's wl_compositor is too old")?;
        let shm: WlShm = globals.bind(qh, 1..=1, ()).context("the compositor has no wl_shm")?;
        let fractional: Option<WpFractionalScaleManagerV1> = globals.bind(qh, 1..=1, ()).ok();
        let viewporter: Option<WpViewporter> = globals.bind(qh, 1..=1, ()).ok();

        let surface = compositor.create_surface(qh, ());
        let fractional = fractional.map(|m| m.get_fractional_scale(&surface, qh, ()));
        let viewport = viewporter.map(|v| v.get_viewport(&surface, qh, ()));
        // An empty input region: clicks and touches go to whatever is underneath.
        let region = compositor.create_region(qh, ());
        surface.set_input_region(Some(&region));
        region.destroy();
        let layer = layer_shell.get_layer_surface(&surface, None, Layer::Overlay, "padwight_overlay".into(), qh, ());
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_exclusive_zone(-1);
        let window = Window {
            shm,
            formats: Formats::default(),
            surface,
            layer,
            _fractional: fractional,
            scale: Scale::new(viewport.is_some()),
            viewport,
            rescaled: false,
            sizing: Sizing::new(),
            todo: Commit::Nothing,
            pool: None,
            pools: 0,
            pacer: Pacer::default(),
            closed: false,
        };
        window.ask(Target::Corner);
        window.surface.commit();
        Ok(window)
    }

    /// Sets the anchors and size for `target`, for the next commit.
    fn ask(&self, target: Target) {
        let anchor = match target {
            Target::Corner => Anchor::Top | Anchor::Right,
            Target::Screen => Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        };
        let (w, h) = target.asked();
        self.layer.set_anchor(anchor);
        self.layer.set_size(w, h);
        timing_log(&format!("surface {} ask {target:?} {w}x{h}", unix_ns()));
    }

    /// Asks for the whole screen, with nothing drawn until its configure comes.
    pub(super) fn grow(&self) {
        self.ask(Target::Screen);
        self.surface.commit();
    }

    /// Asks for the corner; clears what's on screen in the same commit, if anything is.
    pub(super) fn shrink(&mut self, clear: Option<(u32, u32)>, qh: &QueueHandle<Window>) -> Result<()> {
        self.ask(Target::Corner);
        self.pacer.hidden();
        match clear {
            Some(size) => self.commit(size, Why::Clear, qh, |pixels, _| pixels.fill(0)),
            None => {
                self.surface.commit();
                Ok(())
            }
        }
    }

    /// Puts a clear 1×1 buffer on: the surface while hidden. The screen's pool goes with it,
    /// so hidden, the overlay keeps only a few bytes of buffers.
    pub(super) fn blank(&mut self, qh: &QueueHandle<Window>) -> Result<()> {
        self.commit((1, 1), Why::Blank, qh, |pixels, _| pixels.fill(0))
    }

    /// Draws the screen at its configured size, as it is at `now` (the animation clock), and
    /// commits it with a frame callback asked for. Skipped while the compositor holds every
    /// buffer, and drawn again once one is released.
    pub(super) fn draw(&mut self, screen: &mut dyn Screen, overlay: &Overlay, now: Instant, qh: &QueueHandle<Window>) -> Result<()> {
        let Some(logical) = self.sizing.screen() else { return Ok(()) };
        let (w, h) = self.scale.device(logical);
        let factor = self.scale.factor();
        let started = Instant::now();
        self.commit(logical, Why::Draw, qh, |pixels, format| {
            // Drawn straight into the buffer, which holds an older frame (its lease's age);
            // until step 4.3 the whole screen is drawn over it.
            let mut direct = PixmapMut::from_bytes(pixels, w, h).expect("a buffer is as big as its size");
            screen.draw(overlay, now, factor, &mut direct);
            if format == wl_shm::Format::Argb8888 {
                // Without ABGR8888 (KWin lacks it), swapped into ARGB8888's order in place,
                // rather than drawn into a canvas of its own and converted: one screen less of
                // memory, and the same pass over the pixels.
                rgba_to_argb(pixels);
            }
            timing_log(&format!("built {} {} {w}x{h}", unix_ns(), started.elapsed().as_nanos()));
        })
    }

    /// The scale has changed: puts what is on again at the new one, at once. A screen is drawn;
    /// the 1×1 buffer of the hidden surface is made again (a buffer scale must divide the
    /// buffer's size, so the old one can't stay on).
    pub(super) fn rescale(&mut self, screen: &mut dyn Screen, overlay: &Overlay, now: Instant, qh: &QueueHandle<Window>) -> Result<()> {
        if !std::mem::take(&mut self.rescaled) {
            return Ok(());
        }
        if self.sizing.screen().is_some() {
            self.draw(screen, overlay, now, qh)
        } else if self.sizing.target() == Target::Corner && self.sizing.is_blank() {
            self.blank(qh)
        } else {
            Ok(())
        }
    }

    /// The compositor's preferred scale was heard; a change is drawn at the next look.
    pub(super) fn scale_changed(&mut self, changed: bool) {
        if changed {
            self.rescaled = true;
            self.pacer.changed();
        }
    }

    /// Whether a frame could be drawn now: false while the compositor holds every buffer.
    pub(super) fn can_draw(&self) -> bool {
        self.pool.as_ref().is_none_or(|p| p.slots.any_free())
    }

    /// Tells the compositor how the next buffer maps onto a surface of `logical` size.
    fn put_on_scale(&self, (w, h): (u32, u32)) {
        let (scale, dest) = match self.scale.mode() {
            Mode::Viewport => (1, (w as i32, h as i32)),
            Mode::Buffer(n) => (n as i32, (-1, -1)),
        };
        self.surface.set_buffer_scale(scale);
        if let Some(viewport) = &self.viewport {
            viewport.set_destination(dest.0, dest.1);
        }
    }

    /// Fills a free `w`×`h` buffer with `fill`, attaches it, damages it all and commits. Makes
    /// a new pool when the size or format has changed; with every buffer held, commits nothing.
    fn commit(&mut self, logical: (u32, u32), why: Why, qh: &QueueHandle<Window>, fill: impl FnOnce(&mut [u8], wl_shm::Format)) -> Result<()> {
        let (w, h) = logical;
        if !self.sizing.may_attach(logical) {
            // Sizing hands out only sizes it may attach at; this is a bug, not the compositor's.
            timing_log(&format!("commit-refused {} {w}x{h} {why:?}", unix_ns()));
            debug_assert!(false, "a {w}x{h} buffer would go on at the wrong size");
            return Ok(());
        }
        let (dw, dh) = self.scale.device(logical);
        let (layout, format) = (Layout { width: dw, height: dh }, self.formats.format());
        // A pool for another size or format is kept until the new one's buffer is committed.
        let old = match self.pool.take() {
            Some(pool) if pool.fits(layout, format) => {
                self.pool = Some(pool);
                None
            }
            old => {
                self.pools += 1;
                self.pool = Some(Pool::new(&self.shm, self.pools, layout, format, qh).context("the screen's buffers")?);
                timing_log(&format!("pool {} {dw}x{dh} {format:?}", unix_ns()));
                old
            }
        };
        let pool = self.pool.as_mut().expect("there is a pool: made above if not before");
        let Some(lease) = pool.slots.lease() else {
            timing_log(&format!("commit-skipped {} {w}x{h} {why:?} held {}", unix_ns(), pool.slots.held()));
            match why {
                Why::Draw => self.pacer.skipped(),
                // A shrink's request still has to go: without the clear buffer, the last frame
                // stays on until the compositor answers it.
                Why::Clear | Why::Blank => self.surface.commit(),
            }
            return Ok(());
        };
        fill(pool.pixels(lease), format);
        pool.attach(lease, &self.surface);
        self.put_on_scale(logical);
        if let Why::Draw = why {
            // Asked for before the commit, which it belongs to.
            let id = self.pacer.drawn(Instant::now());
            self.surface.frame(qh, id);
        }
        self.surface.commit();
        drop(old);
        self.sizing.attached(logical);
        let now = unix_ns();
        timing_log(&format!(
            "commit {now} {w}x{h} {why:?} {:?} slot {} age {} buffer {dw}x{dh} {:?}",
            self.sizing.target(),
            lease.slot,
            lease.age,
            self.scale.mode()
        ));
        if let Why::Draw = why {
            timing_log(&format!("present {now}"));
        }
        Ok(())
    }
}
