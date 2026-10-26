//! Boxes and their shadows, ported from iced tiny-skia's `draw_quad` (iced_tiny_skia 0.14,
//! `engine.rs`): the same radii and border clamps, the same fill rule and border strokes, and
//! the same shadow, sampled the same way, so a box looks as it does under iced.

use std::f32::consts::{FRAC_PI_2, PI};

use tiny_skia::{FillRule, IntSize, Mask, PathBuilder, Pixmap, PixmapPaint, Transform};

use super::{Pen, paint};
use crate::overlay::render::{Corners, Fill, Outline, Point, Rect, Shadow};

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
        pen.pixmap.fill_path(&path, &paint::paint(fill, rect), FillRule::EvenOdd, pen.transform, pen.mask);
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
        pen.pixmap.fill_path(&path, &paint, FillRule::EvenOdd, pen.transform, pen.mask);
        return;
    };
    let stroke = tiny_skia::Stroke { width, ..tiny_skia::Stroke::default() };
    if simple {
        pen.pixmap.stroke_path(&line, &paint, &stroke, pen.transform, pen.mask);
        return;
    }
    let mask = match pen.mask {
        Some(outer) => {
            let mut mask = outer.clone();
            mask.intersect_path(&path, FillRule::EvenOdd, true, pen.transform);
            Some(mask)
        }
        None => Mask::new(pen.pixmap.width(), pen.pixmap.height()).map(|mut mask| {
            mask.fill_path(&path, FillRule::EvenOdd, true, pen.transform);
            mask
        }),
    };
    if let Some(mask) = mask {
        pen.pixmap.stroke_path(&line, &paint, &stroke, pen.transform, Some(&mask));
    }
}

/// A box's shadow by iced's formula: each device pixel of the shadow's box, sampled at its top
/// left corner, takes the shadow's color times one less the smoothstep, over the blur either
/// side, of its distance outside the rounded box. Inside the box the distance is 0, so a soft
/// shadow is half strength there, as in iced. With no blur the shadow is the box itself,
/// unsoftened (iced's sum gives 0/0 there, which tiny-skia reads as full strength).
///
/// The box is grown by `spread` first, its rounded corners with it.
pub(super) fn shadow(pen: &mut Pen, rect: Rect, corners: Corners, shadow: Shadow, spread: f32) {
    if shadow.color.a.is_nan() || shadow.color.a <= 0.0 {
        return;
    }
    let base = radii(rect, corners);
    let rect = rect.outset(spread);
    if rect.is_empty() {
        return;
    }
    let radii = base.map(|r| if r > 0.0 { (r + spread).max(0.0).min(rect.w / 2.0).min(rect.h / 2.0) } else { 0.0 });
    let (s, blur, (dx, dy)) = (pen.scale(), shadow.blur.max(0.0), shadow.offset);
    let reach = pen.device(Rect::new(rect.x + dx - blur, rect.y + dy - blur, rect.w + 2.0 * blur, rect.h + 2.0 * blur));
    // Whole pixels as iced takes them (`as u32`), then only those on the pixmap.
    let (x0, y0) = (reach.x as u32, reach.y as u32);
    let x1 = (x0 + reach.w as u32).min(pen.pixmap.width());
    let y1 = (y0 + reach.h as u32).min(pen.pixmap.height());
    let Some(size) = IntSize::from_wh(x1.saturating_sub(x0), y1.saturating_sub(y0)) else { return };
    let body = pen.device(rect);
    let half = (body.w / 2.0, body.h / 2.0);
    let center = (body.x + dx * s + half.0, body.y + dy * s + half.1);
    let radii = radii.map(|r| r * s);
    let mut pixels = Vec::with_capacity(size.width() as usize * size.height() as usize * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let d = distance((x as f32 - center.0, y as f32 - center.1), half, radii).max(0.0);
            let strength = if blur > 0.0 { 1.0 - smoothstep(-blur * s, blur * s, d) } else if d <= 0.0 { 1.0 } else { 0.0 };
            let mut c = paint::color(shadow.color);
            c.apply_opacity(strength);
            let p = c.to_color_u8().premultiply();
            pixels.extend_from_slice(&[p.red(), p.green(), p.blue(), p.alpha()]);
        }
    }
    let Some(picture) = Pixmap::from_vec(pixels, size) else { return };
    pen.pixmap.draw_pixmap(x0 as i32, y0 as i32, picture.as_ref(), &PixmapPaint::default(), Transform::identity(), pen.mask);
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let x = ((x - a) / (b - a)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How far `to_center` (from the box's middle) lies outside a box of `half` its size with
/// corners `radii` round, negative inside (iced's `rounded_box_sdf`).
fn distance(to_center: (f32, f32), half: (f32, f32), radii: [f32; 4]) -> f32 {
    let radius = match (to_center.0 > 0.0, to_center.1 > 0.0) {
        (true, true) => radii[2],
        (true, false) => radii[1],
        (false, true) => radii[3],
        (false, false) => radii[0],
    };
    let x = (to_center.0.abs() - half.0 + radius).max(0.0);
    let y = (to_center.1.abs() - half.1 + radius).max(0.0);
    (x.powf(2.0) + y.powf(2.0)).sqrt() - radius
}
