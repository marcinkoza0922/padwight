//! Boxes, ported from iced tiny-skia's `draw_quad` (iced_tiny_skia 0.14,
//! `engine.rs`): the same radii and border clamps, the same fill rule and border strokes, and
//! the shadow in `shadow.rs`, so a box looks as it does under iced.

use std::f32::consts::{FRAC_PI_2, PI};

use tiny_skia::{FillRule, Mask, PathBuilder};

use super::{Pen, cover, paint};
use crate::overlay::render::{Corners, Fill, Outline, Point, Rect};

/// The radii in iced's order, each held to half the box's shorter side.
pub(super) fn radii(rect: Rect, radii: Corners) -> [f32; 4] {
    radii.to_array().map(|r| r.max(0.0).min(rect.w / 2.0).min(rect.h / 2.0))
}

/// A rounded box as iced builds it (`rounded_rectangle`): a plain rect with no rounding, a
/// circle when every corner is half of a square, and otherwise lines and quarter arcs, clockwise
/// from the top left. `None` for a box of no size.
pub(super) fn rounded(rect: Rect, [tl, tr, br, bl]: [f32; 4]) -> Option<tiny_skia::Path> {
    let Rect { x, y, w, h } = rect;
    if !(w > 0.0 && h > 0.0) {
        return None;
    }
    if [tl, tr, br, bl] == [0.0; 4] {
        return Some(PathBuilder::from_rect(tiny_skia::Rect::from_xywh(x, y, w, h)?));
    }
    if [tr, br, bl] == [tl; 3] && tl == w / 2.0 && tl == h / 2.0 {
        return PathBuilder::from_circle(x + w / 2.0, y + h / 2.0, tl);
    }
    let mut b = PathBuilder::new();
    b.move_to(x + tl, y);
    b.line_to(x + w - tr, y);
    // Each corner turns a quarter clockwise around its circle's center.
    corner(&mut b, Point::new(x + w - tr, y + tr), tr, -FRAC_PI_2);
    line_to(&mut b, x + w, y + h - br);
    corner(&mut b, Point::new(x + w - br, y + h - br), br, 0.0);
    line_to(&mut b, x + bl, y + h);
    corner(&mut b, Point::new(x + bl, y + h - bl), bl, FRAC_PI_2);
    line_to(&mut b, x, y + tl);
    corner(&mut b, Point::new(x + tl, y + tl), tl, PI);
    // Iced leaves it open, which a fill doesn't see, but a border's ends then leave a notch in a
    // square top left corner; closed, the corner is mitered like the others.
    b.close();
    b.finish()
}

/// A line to a point, unless the path is there already (iced's `maybe_line_to`).
fn line_to(b: &mut PathBuilder, x: f32, y: f32) {
    if b.last_point() != Some(tiny_skia::Point::from_xy(x, y)) {
        b.line_to(x, y);
    }
}

/// A quarter turn of radius `r` around `center`, from the angle `from` (radians, y down)
/// clockwise, as the cubics kurbo makes of iced's SVG arc at its tolerance of 0.1.
fn corner(b: &mut PathBuilder, center: Point, r: f32, from: f32) {
    if r <= 0.0 {
        return;
    }
    // kurbo's count of pieces per whole turn for that tolerance, and a quarter of it.
    let per_turn = (1.1163 * f64::from(r) / 0.1).powf(1.0 / 6.0).max(3.999_999);
    let pieces = (per_turn / 4.0).ceil();
    let step = FRAC_PI_2 as f64 / pieces;
    let arm = 4.0 / 3.0 * (step / 4.0).tan();
    let (cx, cy, r) = (f64::from(center.x), f64::from(center.y), f64::from(r));
    let at = |a: f64| (cx + r * a.cos(), cy + r * a.sin());
    // The tangent at `a`, clockwise, `arm` of the radius long.
    let tangent = |a: f64| (-r * arm * a.sin(), r * arm * a.cos());
    for i in 0..pieces as usize {
        let (a0, a1) = (f64::from(from) + step * i as f64, f64::from(from) + step * (i + 1) as f64);
        let ((x0, y0), (x1, y1), (dx0, dy0), (dx1, dy1)) = (at(a0), at(a1), tangent(a0), tangent(a1));
        b.cubic_to((x0 + dx0) as f32, (y0 + dy0) as f32, (x1 - dx1) as f32, (y1 - dy1) as f32, x1 as f32, y1 as f32);
    }
}

/// A box, filled even-odd, with its border inside it: stroked along the box shrunk by half the
/// width, its corners rounded that much less. Where a corner is rounder than nothing but
/// tighter than half the width, the border's corners are square and cut to the box's own.
pub(super) fn rect(pen: &mut Pen, rect: Rect, corners: Corners, fill: Option<&Fill>, border: Option<Outline>) {
    let fill_radii = radii(rect, corners);
    let Some(path) = rounded(rect, fill_radii) else { return };
    if let Some(fill) = fill {
        cover::fill(pen, &path, &paint::paint(fill, rect), FillRule::EvenOdd);
    }
    let Some(border) = border.filter(|b| b.color.a > 0.0) else { return };
    let width = border.width.min(rect.w / 2.0).min(rect.h / 2.0);
    if width.is_nan() || width <= 0.0 {
        return;
    }
    let inner = Rect::new(rect.x + width / 2.0, rect.y + width / 2.0, rect.w - width, rect.h - width);
    let mut simple = true;
    let mut border_radii = corners.to_array();
    for r in &mut border_radii {
        *r = if *r <= 0.0 {
            0.0
        } else if *r > width / 2.0 {
            *r - width / 2.0
        } else {
            simple = false;
            0.0
        }
        .min(inner.w / 2.0)
        .min(inner.h / 2.0);
    }
    let paint = paint::paint(&Fill::Solid(border.color), rect);
    // A border as wide as half the box covers all of it.
    let Some(line) = rounded(inner, border_radii) else {
        cover::fill(pen, &path, &paint, FillRule::EvenOdd);
        return;
    };
    let stroke = tiny_skia::Stroke { width, ..tiny_skia::Stroke::default() };
    if simple {
        cover::stroke(pen, &line, &paint, &stroke);
        return;
    }
    let mask = match pen.mask {
        Some(outer) => {
            let mut mask = outer.clone();
            cover::mask(&mut mask, &path, FillRule::EvenOdd, pen.transform, true);
            Some(mask)
        }
        None => Mask::new(pen.pixmap.width(), pen.pixmap.height()).map(|mut mask| {
            cover::mask(&mut mask, &path, FillRule::EvenOdd, pen.transform, false);
            mask
        }),
    };
    if let Some(mask) = mask {
        cover::stroke(&mut Pen { pixmap: pen.pixmap, transform: pen.transform, mask: Some(&mask) }, &line, &paint, &stroke);
    }
}
