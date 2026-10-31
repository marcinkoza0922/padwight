//! The layer surface: binding the globals, making the surface, and every commit made to it.
//! Each commit goes through here, so the rule that a buffer only goes on at the size last acked
//! is checked in one place (`Sizing::attached`), as is the rule that a frame is drawn only into
//! a buffer the compositor has released (`Slots::check_free`), and each is logged. Each frame
//! drawn asks for a frame callback in its own commit (`Pacer`). A frame that comes out the same
//! as the buffer it went into is not committed at all; one that changed damages only what
//! changed (`render::Screen::paint`).

use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use wayland_client::{
    QueueHandle,
    globals::GlobalList,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::WlOutput,
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
    monitors::Monitors,
    pacing::Pacer,
    picture::{Buffer, Picture},
    scale::{Mode, Scale},
    shm::{Formats, Pool},
    sizing::{Commit, Sizing, Target},
    slots::Layout,
};
use crate::overlay::{
    render::{Damage, Point, Screen, raster, screen_ops},
    timing::{timing_log, unix_ns},
    ui::Overlay,
};

/// The surface and what it needs, also the state the protocol's events are dispatched to.
pub(super) struct Window {
    pub(super) shm: WlShm,
    /// Kept to make the surface again (`remake`).
    makers: Makers,
    pub(super) formats: Formats,
    pub(super) surface: WlSurface,
    pub(super) layer: ZwlrLayerSurfaceV1,
    /// Kept alive: it is what sends the preferred scale.
    _fractional: Option<WpFractionalScaleV1>,
    viewport: Option<WpViewport>,
    /// Where the focused window is, to put the surface on that monitor (`remake`).
    pub(super) monitors: Monitors,
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
    /// Draws the screen, keeping each panel's picture and what the last frames repainted.
    screen: Screen,
    /// The compositor closed the surface.
    pub(super) closed: bool,
    /// When the last frame drawn went on, for the loop to tell the overlay (`Overlay::presented`).
    pub(super) presented: Option<Instant>,
}

/// The globals a surface is made from.
struct Makers {
    layer_shell: ZwlrLayerShellV1,
    compositor: WlCompositor,
    fractional: Option<WpFractionalScaleManagerV1>,
    viewporter: Option<WpViewporter>,
}

/// The objects of one layer surface.
struct Surface {
    surface: WlSurface,
    layer: ZwlrLayerSurfaceV1,
    fractional: Option<WpFractionalScaleV1>,
    viewport: Option<WpViewport>,
}

impl Makers {
    /// A surface on the overlay layer, 1×1 in a corner, never taking keyboard focus, with clicks
    /// passing through. On `output` when given; otherwise the compositor chooses, as it is made.
    fn surface(&self, qh: &QueueHandle<Window>, output: Option<&WlOutput>) -> Surface {
        let surface = self.compositor.create_surface(qh, ());
        let fractional = self.fractional.as_ref().map(|m| m.get_fractional_scale(&surface, qh, ()));
        let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(&surface, qh, ()));
        // An empty input region: clicks and touches go to whatever is underneath.
        let region = self.compositor.create_region(qh, ());
        surface.set_input_region(Some(&region));
        region.destroy();
        let layer = self.layer_shell.get_layer_surface(&surface, output, Layer::Overlay, "padwight_overlay".into(), qh, ());
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_exclusive_zone(-1);
        Surface { surface, layer, fractional, viewport }
    }
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
            anyhow!("the overlay window could not start: the compositor lacks wlr-layer-shell (as GNOME and Cinnamon do)")
        })?;
        let compositor: WlCompositor = globals.bind(qh, 4..=6, ()).context("the compositor's wl_compositor is too old")?;
        let shm: WlShm = globals.bind(qh, 1..=1, ()).context("the compositor has no wl_shm")?;
        let fractional: Option<WpFractionalScaleManagerV1> = globals.bind(qh, 1..=1, ()).ok();
        let viewporter: Option<WpViewporter> = globals.bind(qh, 1..=1, ()).ok();

        let makers = Makers { layer_shell, compositor, fractional, viewporter };
        let Surface { surface, layer, fractional, viewport } = makers.surface(qh, None);
        let window = Window {
            shm,
            makers,
            monitors: Monitors::new(globals, qh),
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
            screen: Screen::new(),
            closed: false,
            presented: None,
        };
        window.ask(Target::Corner);
        window.surface.commit();
        Ok(window)
    }

    /// Whether the surface is hidden, in its corner, with nothing asked of it.
    pub(super) fn is_hidden(&self) -> bool {
        self.sizing.target() == Target::Corner
    }

    /// Makes the surface again, in place of the hidden one. The compositor chooses an output for
    /// a layer surface as it is made, and the surface would stay there (so it is told the focused window's monitor, where the compositor says it): a game that moved to
    /// another monitor would keep its overlay on the old one. What the surface needs to know
    /// afresh goes back to its start; the scale, the formats and the pacer's count carry over.
    pub(super) fn remake(&mut self, qh: &QueueHandle<Window>) {
        let Surface { surface, layer, fractional, viewport } = self.makers.surface(qh, self.monitors.focused());
        if let Some(old) = self._fractional.take() {
            old.destroy();
        }
        if let Some(old) = self.viewport.take() {
            old.destroy();
        }
        self.layer.destroy();
        self.surface.destroy();
        self.surface = surface;
        self.layer = layer;
        self._fractional = fractional;
        self.viewport = viewport;
        self.sizing = Sizing::new();
        self.todo = Commit::Nothing;
        // Buffers drawn into a new surface are all new to the compositor: a new pool paints them whole.
        self.pool = None;
        self.screen.forget();
        self.rescaled = false;
        self.ask(Target::Corner);
        self.surface.commit();
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

    /// Asks for the corner; clears what's on screen in the same commit, if anything is. The
    /// panels' pictures go, and most of what the rasterizer keeps: showing again draws them anew.
    pub(super) fn shrink(&mut self, clear: Option<(u32, u32)>, qh: &QueueHandle<Window>) -> Result<()> {
        self.ask(Target::Corner);
        self.pacer.hidden();
        self.screen.forget();
        raster::shrink();
        match clear {
            Some(size) => self.commit(size, Why::Clear, &Picture::Clear, qh),
            None => {
                self.surface.commit();
                Ok(())
            }
        }
    }

    /// Puts a clear 1×1 buffer on: the surface while hidden. The screen's pool goes with it,
    /// so hidden, the overlay keeps only a few bytes of buffers, and the memory freed since
    /// `shrink` goes back to the system.
    pub(super) fn blank(&mut self, qh: &QueueHandle<Window>) -> Result<()> {
        self.commit((1, 1), Why::Blank, &Picture::Clear, qh)?;
        give_back_freed();
        Ok(())
    }

    /// Draws the screen at its configured size, as it is at `now` (the animation clock), and
    /// commits what changed with a frame callback asked for. Skipped while the compositor holds
    /// every buffer, and drawn again once one is released; not committed when nothing changed.
    pub(super) fn draw(&mut self, overlay: &Overlay, now: Instant, qh: &QueueHandle<Window>) -> Result<()> {
        let Some(logical) = self.sizing.screen() else { return Ok(()) };
        let started = Instant::now();
        let shown = overlay.showing(now);
        let ops = screen_ops(overlay.frame(), &shown, Point::new(logical.0 as f32, logical.1 as f32));
        let scale = self.scale.factor();
        self.commit(logical, Why::Draw, &Picture::Screen { ops: &ops, scale, started }, qh)
    }

    /// The scale has changed: puts what is on again at the new one, at once. A screen is drawn;
    /// the 1×1 buffer of the hidden surface is made again (a buffer scale must divide the
    /// buffer's size, so the old one can't stay on).
    pub(super) fn rescale(&mut self, overlay: &Overlay, now: Instant, qh: &QueueHandle<Window>) -> Result<()> {
        if !std::mem::take(&mut self.rescaled) {
            return Ok(());
        }
        if self.sizing.screen().is_some() {
            self.draw(overlay, now, qh)
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

    /// Makes the pool for buffers of `layout` and `format`, unless the one there is fits:
    /// returns the pool it replaces.
    fn pool_for(&mut self, layout: Layout, format: wl_shm::Format, qh: &QueueHandle<Window>) -> Result<Option<Pool>> {
        match self.pool.take() {
            Some(pool) if pool.fits(layout, format) => {
                self.pool = Some(pool);
                Ok(None)
            }
            old => {
                self.pools += 1;
                self.pool = Some(Pool::new(&self.shm, self.pools, layout, format, qh).context("the screen's buffers")?);
                // New buffers hold nothing the screen painted, and are clear: the first frames
                // into them paint only where the screen draws something.
                self.screen.forget_onto_clear((layout.width, layout.height), self.scale.factor());
                timing_log(&format!("pool {} {}x{} {format:?}", unix_ns(), layout.width, layout.height));
                Ok(old)
            }
        }
    }

    /// Puts `picture` into a free buffer for a surface of `logical` size, attaches it, damages
    /// what changed and commits. Makes a new pool when the size or format has changed; with
    /// every buffer held, or nothing changed, commits nothing.
    fn commit(&mut self, logical: (u32, u32), why: Why, picture: &Picture, qh: &QueueHandle<Window>) -> Result<()> {
        let (w, h) = logical;
        if !self.sizing.may_attach(logical) {
            // Sizing hands out only sizes it may attach at; this is a bug, not the compositor's.
            timing_log(&format!("commit-refused {} {w}x{h} {why:?}", unix_ns()));
            debug_assert!(false, "a {w}x{h} buffer would go on at the wrong size");
            return Ok(());
        }
        let (dw, dh) = self.scale.device(logical);
        let format = self.formats.format();
        // A pool for another size or format is kept until the new one's buffer is committed.
        let old = self.pool_for(Layout { width: dw, height: dh }, format, qh)?;
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
        let buffer = Buffer { pixels: pool.pixels(lease), size: (dw, dh), format, age: lease.age };
        let damage = picture.paint(&mut self.screen, buffer);
        // Nothing committed from the pool yet: its first buffer goes on whatever it holds.
        let first = pool.slots.fresh();
        if damage.is_empty() && !first {
            // The same as the buffer holds, and so as what is on: nothing to commit, and no
            // frame counted, so the screen's damage history stays in step with the buffers'.
            timing_log(&format!("commit-unchanged {} {w}x{h} {why:?} slot {} age {}", unix_ns(), lease.slot, lease.age));
            self.pacer.unchanged(Instant::now());
            return Ok(());
        }
        // The first buffer at a size is all new to the compositor, its clear parts too. (Had it
        // painted nothing, the screen's history is a frame short: later frames repaint more.)
        let damage = if first { Damage::full(dw, dh) } else { damage };
        pool.attach(lease, &self.surface, &damage);
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
            "commit {now} {w}x{h} {why:?} {:?} slot {} age {} damage {} {} buffer {dw}x{dh} {:?}",
            self.sizing.target(),
            lease.slot,
            lease.age,
            damage.patches().len(),
            damage.patches().iter().map(|p| i64::from(p.width()) * i64::from(p.height())).sum::<i64>(),
            self.scale.mode()
        ));
        if let Why::Draw = why {
            self.presented = Some(Instant::now());
            timing_log(&format!("present {now}"));
        }
        Ok(())
    }
}

/// Hands the memory freed on hiding back to the system: glibc keeps freed heap memory for the
/// process otherwise, and the panels' pictures and tiles are many middling allocations.
fn give_back_freed() {
    #[cfg(target_env = "gnu")]
    // SAFETY: malloc_trim only releases memory that is free; nothing of ours points into it.
    unsafe {
        libc::malloc_trim(0);
    }
}
