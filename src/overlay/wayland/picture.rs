//! What goes into a buffer: the screen, painted by the overlay's own renderer
//! (`render::Screen`) over what the buffer held, or a clear one.
//!
//! The renderer keeps the damage of the last few frames committed and repaints a reused buffer
//! from it, by the buffer's age (`slots.rs`). That holds only if each non-empty paint is
//! committed and each empty one isn't (`Window::commit` sees to it), and if the renderer is
//! told to forget whenever a buffer gets something it didn't paint: a clear one as the surface
//! hides, or a new pool's.

use std::time::Instant;

use tiny_skia::PixmapMut;
use wayland_client::protocol::wl_shm;

use crate::overlay::{
    render::{Damage, Op, Screen, swap_red_blue},
    timing::{timing_log, unix_ns},
};

/// What to put in the buffer.
pub(super) enum Picture<'a> {
    /// A clear buffer: the screen's last before the surface shrinks, or the hidden 1×1 one.
    Clear,
    /// The screen's list (`render::screen_ops`), at `scale` device pixels to a logical one; its
    /// build began at `started`, for the timing log.
    Screen { ops: &'a [Op], scale: f32, started: Instant },
}

/// A free buffer to put a picture into: `width`×`height` pixels in `format`, holding the frame
/// committed `age` frames back (0: unknown).
pub(super) struct Buffer<'a> {
    pub(super) pixels: &'a mut [u8],
    pub(super) size: (u32, u32),
    pub(super) format: wl_shm::Format,
    pub(super) age: u64,
}

impl Picture<'_> {
    /// Puts the picture into `buffer` and returns what changed in it: empty when the screen is
    /// the same as on that buffer, which is then to be left uncommitted.
    pub(super) fn paint(&self, screen: &mut Screen, buffer: Buffer) -> Damage {
        let Buffer { pixels, size: (width, height), format, age } = buffer;
        match *self {
            Picture::Clear => {
                pixels.fill(0);
                // Committed, but not painted by the screen: what it remembers is out of step.
                screen.forget();
                Damage::full(width, height)
            }
            Picture::Screen { ops, scale, started } => {
                let listed = started.elapsed();
                let mut canvas = PixmapMut::from_bytes(pixels, width, height).expect("a buffer is as big as its size");
                let damage = screen.paint(ops, scale, &mut canvas, age);
                if format == wl_shm::Format::Argb8888 {
                    // Without ABGR8888 (KWin lacks it), what was repainted is swapped into
                    // ARGB8888's order in place; the rest was swapped when it was painted.
                    swap_red_blue(pixels, width, &damage);
                }
                let total = started.elapsed();
                let (list, paint) = (listed.as_nanos(), (total - listed).as_nanos());
                timing_log(&format!("built {} {} {width}x{height} list {list} paint {paint} pictures {}", unix_ns(), total.as_nanos(), screen.drawn()));
                damage
            }
        }
    }
}
