//! What the surface shows: the seam where the screen renderer (`render/screen.rs`) plugs in.
//! Until it does, a test picture stands in.

use std::time::Instant;

use tiny_skia::{PixmapMut, Transform};

use crate::overlay::{
    render::{Corners, Layer, Op, Outline, Rect, Rgba, raster},
    ui::Overlay,
};

/// Draws the screen.
pub(super) trait Screen {
    /// Draws what `overlay` shows at `now` onto `canvas`, the whole surface in device pixels,
    /// premultiplied RGBA, at `scale` device pixels to a logical one. The canvas is a reused
    /// buffer holding an earlier screen, possibly with red and blue swapped (ARGB8888): every
    /// pixel has to be cleared or drawn over, and none read before that.
    fn draw(&mut self, overlay: &Overlay, now: Instant, scale: f32, canvas: &mut PixmapMut<'_>);
}

/// A stand-in for the screen renderer: a rounded box in the middle of the screen, a third of
/// its size, at the opacity of the most opaque thing shown, so fades can be seen.
pub(super) struct TestPicture;

impl Screen for TestPicture {
    fn draw(&mut self, overlay: &Overlay, now: Instant, scale: f32, canvas: &mut PixmapMut<'_>) {
        canvas.fill(tiny_skia::Color::TRANSPARENT);
        let opacity = overlay.strongest(now);
        if opacity <= 0.0 {
            return;
        }
        let (w, h) = (canvas.width() as f32 / scale, canvas.height() as f32 / scale);
        let rect = Rect::new(w / 3.0, h / 3.0, w / 3.0, h / 3.0);
        let ops = [
            Op::Layer(Layer::faded(opacity)),
            Op::Rect {
                rect,
                radii: Corners::all(24.0),
                fill: Some(Rgba::from_rgba8(24, 28, 36, 0.9).into()),
                border: Some(Outline { width: 3.0, color: Rgba::from_rgb8(120, 180, 255) }),
            },
            Op::Unlayer,
        ];
        raster::draw(&ops, canvas, Transform::from_scale(scale, scale));
    }
}
