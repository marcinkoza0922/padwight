//! The rasterizer: draws a display list into a tiny-skia pixmap at a device scale, op by op in
//! order. Boxes, shadows and gradients follow iced tiny-skia's maths (`quad`, `shadow`), so a
//! panel looks as it does under iced.
//!
//! A list comes out the same whatever whole pixel the pixmap's corner lies on, as a panel's
//! picture drawn in tiles needs (`screen/tiles.rs`): shapes are rasterized whole and laid
//! through their coverage (`cover`), shadows worked out whole (`shadow`), pictures and patterns
//! placed on whole pixels copied or stretched from their corner (`blit`), and glyphs placed on
//! the same sub-pixel bins (`text/paint.rs`). Shapes and shadows are kept, so one seen again
//! is only laid down.
//!
//! A clip intersects the mask of the clips around it. A layer's ops are drawn into a pixmap of
//! their own, as big as their ink once the layer has moved and scaled them, at the scale they
//! will be seen at: the layer is laid down on whole pixels at its opacity, never resampled, so a
//! zoom stays sharp and smooth. Clips inside a layer are drawn in its pixmap, so they move and
//! scale with it.
//!
//! Pixmaps hold premultiplied RGBA, tiny-skia's own order; colors in the list are straight.

mod blit;
mod cover;
mod kept;
mod paint;
mod pictures;
mod quad;
mod shadow;

#[cfg(test)]
mod tests;

use tiny_skia::{FillRule, FilterQuality, IntRect, Mask, Pixmap, PixmapMut, PixmapPaint, Transform};

use super::{Corners, Layer, Op, Rect, text};

pub(super) use blit::{mul8, over};

/// Drops every picture, SVG, pattern and shape's coverage kept, as when the overlay hides.
pub fn forget() {
    pictures::forget();
    forget_shapes();
}

/// Drops the coverage of every shape and every shadow kept, as when a screen's pictures go
/// (`Screen::forget`): they are made again from a panel's list as quickly as its picture is.
pub fn forget_shapes() {
    cover::forget();
    shadow::forget();
}

/// A list drawn into a pixmap of its own (see `picture`).
pub struct Picture {
    pub pixmap: Pixmap,
    /// Where the pixmap's top left corner lies, in device pixels.
    pub origin: (i32, i32),
}

/// Where an op draws: the pixmap, how logical pixels map onto it (a scale and a translation,
/// never a rotation or skew) and the mask of the clips it is in.
pub(super) struct Pen<'a, 'b> {
    pub pixmap: &'a mut PixmapMut<'b>,
    pub transform: Transform,
    pub mask: Option<&'a Mask>,
}

impl Pen<'_, '_> {
    /// Device pixels per logical one.
    pub fn scale(&self) -> f32 {
        self.transform.sx
    }

    /// Where box `r` lands on the pixmap.
    pub fn device(&self, r: Rect) -> Rect {
        let t = self.transform;
        Rect::new(r.x * t.sx + t.tx, r.y * t.sy + t.ty, r.w * t.sx, r.h * t.sy)
    }

    /// Where box `r`'s edges land on the pixmap: left, top, right, bottom. Each is its logical
    /// edge moved, so it lands the same however far the pixmap's corner is (a box's right edge
    /// taken as its left plus its width can round differently).
    pub fn edges(&self, r: Rect) -> [f32; 4] {
        let t = self.transform;
        [r.x * t.sx + t.tx, r.y * t.sy + t.ty, r.right() * t.sx + t.tx, r.bottom() * t.sy + t.ty]
    }
}

/// Draws `ops` onto `pixmap`, logical pixels mapped by `transform` (a scale and a translation
/// only), over what is there. A close with nothing open is passed over, and a group left open
/// runs to the end.
pub fn draw(ops: &[Op], pixmap: &mut PixmapMut, transform: Transform) {
    let mut at = 0;
    while at < ops.len() {
        at = run(ops, at, pixmap, transform, None);
    }
}

/// Draws `ops` at `scale` into a clear pixmap just big enough for their ink, or `None` when they
/// draw nothing.
pub fn picture(ops: &[Op], scale: f32) -> Option<Picture> {
    let ink = super::bounds(ops)?;
    let area = covering(Rect::new(ink.x * scale, ink.y * scale, ink.w * scale, ink.h * scale), None)?;
    let mut pixmap = Pixmap::new(area.width(), area.height())?;
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, -area.x() as f32, -area.y() as f32);
    draw(ops, &mut pixmap.as_mut(), transform);
    Some(Picture { pixmap, origin: (area.x(), area.y()) })
}

/// Draws the ops from `at` until the close of the group they are in (or the end), and returns
/// where to go on from.
fn run(ops: &[Op], mut at: usize, pixmap: &mut PixmapMut, transform: Transform, mask: Option<&Mask>) -> usize {
    while let Some(op) = ops.get(at) {
        at += 1;
        match op {
            Op::Clip { rect, radii } => at = clip(ops, at, &mut Pen { pixmap, transform, mask }, *rect, *radii),
            Op::Layer(layer) => at = layered(ops, at, &mut Pen { pixmap, transform, mask }, layer),
            Op::Unclip | Op::Unlayer => return at,
            op => one(op, &mut Pen { pixmap, transform, mask }),
        }
    }
    at
}

/// Where the group opened just before `at` ends: the index of its close (or the end), and the
/// index after it.
fn close(ops: &[Op], mut at: usize) -> (usize, usize) {
    let mut depth = 0;
    while let Some(op) = ops.get(at) {
        match op {
            Op::Clip { .. } | Op::Layer(_) => depth += 1,
            Op::Unclip | Op::Unlayer if depth == 0 => return (at, at + 1),
            Op::Unclip | Op::Unlayer => depth -= 1,
            _ => {}
        }
        at += 1;
    }
    (at, at)
}

/// Draws a clip's group cut to its rounded box, and the mask it is in.
fn clip(ops: &[Op], at: usize, pen: &mut Pen, rect: Rect, radii: Corners) -> usize {
    let Some(path) = quad::rounded(rect, quad::radii(rect, radii)) else { return close(ops, at).1 };
    let mask = match pen.mask {
        Some(outer) => {
            let mut mask = outer.clone();
            cover::mask(&mut mask, &path, FillRule::Winding, pen.transform, true);
            mask
        }
        None => {
            let Some(mut mask) = Mask::new(pen.pixmap.width(), pen.pixmap.height()) else { return close(ops, at).1 };
            cover::mask(&mut mask, &path, FillRule::Winding, pen.transform, false);
            mask
        }
    };
    // A clip that cuts nothing off this pixmap (a tile inside a panel's rounded box) is left
    // out, which cuts the same.
    let mask = mask.data().iter().any(|m| *m < 255).then_some(&mask);
    run(ops, at, pen.pixmap, pen.transform, mask)
}

/// Draws a layer's group into a pixmap of its own at the scale it is seen at, then lays that
/// down on whole pixels at the layer's opacity.
fn layered(ops: &[Op], at: usize, pen: &mut Pen, layer: &Layer) -> usize {
    let (end, next) = close(ops, at);
    let inner = &ops[at..end];
    let opacity = layer.opacity.clamp(0.0, 1.0);
    if !(opacity > 0.0 && layer.scale > 0.0) {
        return next;
    }
    // Logical pixels to this pixmap's, through the layer: `pen.transform` after `layer.apply`.
    let t = pen.transform;
    let (s, about, shift) = (layer.scale, layer.about, layer.shift);
    let seen = Transform::from_row(
        t.sx * s,
        0.0,
        0.0,
        t.sy * s,
        t.sx * (about.x * (1.0 - s) + shift.x) + t.tx,
        t.sy * (about.y * (1.0 - s) + shift.y) + t.ty,
    );
    let Some(ink) = super::bounds(inner) else { return next };
    let on = Rect::new(ink.x * seen.sx + seen.tx, ink.y * seen.sy + seen.ty, ink.w * seen.sx, ink.h * seen.sy);
    let Some(area) = covering(on, Some((pen.pixmap.width(), pen.pixmap.height()))) else { return next };
    let Some(mut pixmap) = Pixmap::new(area.width(), area.height()) else { return next };
    let (x, y) = (area.x(), area.y());
    draw(inner, &mut pixmap.as_mut(), seen.post_translate(-x as f32, -y as f32));
    let paint = PixmapPaint { opacity, quality: FilterQuality::Nearest, ..PixmapPaint::default() };
    pen.pixmap.draw_pixmap(x, y, pixmap.as_ref(), &paint, Transform::identity(), pen.mask);
    next
}

/// The whole pixels that `r` (device pixels) touches, inside a `within` pixmap when given.
fn covering(r: Rect, within: Option<(u32, u32)>) -> Option<IntRect> {
    let (mut x0, mut y0, mut x1, mut y1) = (r.x.floor(), r.y.floor(), r.right().ceil(), r.bottom().ceil());
    if let Some((w, h)) = within {
        (x0, y0, x1, y1) = (x0.max(0.0), y0.max(0.0), x1.min(w as f32), y1.min(h as f32));
    }
    // Far past any screen, as a guard against a runaway size.
    let limit = 16_384.0;
    if !(x1 > x0 && y1 > y0) || x1 - x0 > limit || y1 - y0 > limit {
        return None;
    }
    IntRect::from_ltrb(x0 as i32, y0 as i32, x1 as i32, y1 as i32)
}

/// Draws one op that isn't a group marker.
fn one(op: &Op, pen: &mut Pen) {
    match op {
        Op::Shadow { rect, radii, shadow, spread } => shadow::shadow(pen, *rect, *radii, *shadow, *spread),
        Op::Rect { rect, radii, fill, border } => quad::rect(pen, *rect, *radii, fill.as_ref(), *border),
        Op::Text { run, at, color, .. } => {
            let (w, h) = (pen.pixmap.width(), pen.pixmap.height());
            let Some(pixmap) = PixmapMut::from_bytes(pen.pixmap.data_mut(), w, h) else { return };
            text::draw(*run, *at, *color, &mut text::Canvas { pixmap, transform: pen.transform, mask: pen.mask });
        }
        Op::Path { path, fill, rule, stroke } => paint::path(pen, path, fill.as_ref(), *rule, stroke.as_ref()),
        Op::Picture { file, rect, fit, opacity } => pictures::picture(pen, file, *rect, *fit, *opacity),
        Op::Svg { svg, rect, fit, opacity } => pictures::svg(pen, svg, *rect, *fit, *opacity),
        Op::Pattern { pattern, rect, origin, cut, opacity } => pictures::pattern(pen, pattern, (*rect, *origin), *cut, *opacity),
        Op::Clip { .. } | Op::Unclip | Op::Layer(_) | Op::Unlayer => {}
    }
}
