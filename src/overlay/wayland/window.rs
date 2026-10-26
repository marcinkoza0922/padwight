//! The layer surface: binding the globals, making the surface, and every commit made to it.
//! Each commit goes through here, so the rule that a buffer only goes on at the size last acked
//! is checked in one place (`Sizing::attached`), and each is logged.

use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use tiny_skia::Pixmap;
use wayland_client::{
    QueueHandle,
    globals::GlobalList,
    protocol::{wl_buffer::WlBuffer, wl_compositor::WlCompositor, wl_shm::WlShm, wl_surface::WlSurface},
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
    shm::{self, Formats},
    sizing::{Commit, Sizing, Target},
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
    /// Kept for step 5.4, which draws at the scale they give.
    pub(super) _fractional: Option<WpFractionalScaleV1>,
    pub(super) _viewport: Option<WpViewport>,
    /// The preferred scale from wp_fractional_scale_v1, in 120ths. Drawing is at scale 1 until
    /// step 5.4.
    pub(super) scale120: Option<u32>,
    pub(super) sizing: Sizing,
    /// What the last configure asks to commit, for the loop to do.
    pub(super) todo: Commit,
    /// Buffers committed and not yet released by the compositor.
    pub(super) held: Vec<WlBuffer>,
    /// The compositor closed the surface.
    pub(super) closed: bool,
    /// The screen drawn last, kept to draw the next into.
    canvas: Option<Pixmap>,
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
            _viewport: viewport,
            scale120: None,
            sizing: Sizing::new(),
            todo: Commit::Nothing,
            held: Vec::new(),
            closed: false,
            canvas: None,
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
        match clear {
            Some((w, h)) => {
                let pixels = vec![0; w as usize * h as usize * 4];
                self.commit((w, h), &pixels, Why::Clear, qh)
            }
            None => {
                self.surface.commit();
                Ok(())
            }
        }
    }

    /// Puts a clear 1×1 buffer on: the surface while hidden.
    pub(super) fn blank(&mut self, qh: &QueueHandle<Window>) -> Result<()> {
        self.commit((1, 1), &[0; 4], Why::Blank, qh)
    }

    /// Draws the screen at its configured size, as it is now, and commits it.
    pub(super) fn draw(&mut self, screen: &mut dyn Screen, overlay: &Overlay, qh: &QueueHandle<Window>) -> Result<()> {
        let Some((w, h)) = self.sizing.screen() else { return Ok(()) };
        let started = Instant::now();
        let now = started;
        let mut canvas = match self.canvas.take() {
            Some(c) if (c.width(), c.height()) == (w, h) => c,
            _ => Pixmap::new(w, h).context("the screen's picture")?,
        };
        screen.draw(overlay, now, 1.0, &mut canvas.as_mut());
        timing_log(&format!("built {} {} {w}x{h}", unix_ns(), started.elapsed().as_nanos()));
        let committed = self.commit((w, h), canvas.data(), Why::Draw, qh);
        self.canvas = Some(canvas);
        committed
    }

    /// Lets go of the screen's picture, while hidden.
    pub(super) fn forget(&mut self) {
        self.canvas = None;
    }

    /// Attaches `pixels` as a `w`×`h` buffer, damages it all and commits.
    fn commit(&mut self, (w, h): (u32, u32), pixels: &[u8], why: Why, qh: &QueueHandle<Window>) -> Result<()> {
        if !self.sizing.may_attach((w, h)) {
            // Sizing hands out only sizes it may attach at; this is a bug, not the compositor's.
            timing_log(&format!("commit-refused {} {w}x{h} {why:?}", unix_ns()));
            debug_assert!(false, "a {w}x{h} buffer would go on at the wrong size");
            return Ok(());
        }
        let buffer = shm::buffer(&self.shm, self.formats, (w, h), pixels, qh)?;
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, w as i32, h as i32);
        self.surface.commit();
        self.sizing.attached((w, h));
        self.held.push(buffer);
        let now = unix_ns();
        timing_log(&format!("commit {now} {w}x{h} {why:?} {:?}", self.sizing.target()));
        if let Why::Draw = why {
            timing_log(&format!("present {now}"));
        }
        Ok(())
    }
}
