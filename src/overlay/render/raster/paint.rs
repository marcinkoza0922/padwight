//! Colors, fills and vector paths in tiny-skia's terms.

use tiny_skia::{GradientStop, LinearGradient, LineCap, LineJoin, Paint, PathBuilder, Shader, SpreadMode};

use super::Pen;
use crate::overlay::render::{Cap, Fill, Join, Path, Rect, Rgba, Rule, Seg, Stroke};

/// A straight color as tiny-skia takes it, each channel held to 0 to 1 (NaN as 0).
pub(super) fn color(c: Rgba) -> tiny_skia::Color {
    let v = |x: f32| if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
    tiny_skia::Color::from_rgba(v(c.r), v(c.g), v(c.b), v(c.a)).unwrap_or(tiny_skia::Color::TRANSPARENT)
}

/// What paints `fill` over a shape whose box is `bounds`: a gradient spans the box, by iced's
/// angle maths, padded past its ends. A gradient with no stops is black, as in iced, and one
/// that can't span the box (a box of no size) is its last stop's color.
pub(super) fn shader(fill: &Fill, bounds: Rect) -> Shader<'static> {
    let linear = match fill {
        Fill::Solid(c) => return Shader::SolidColor(color(*c)),
        Fill::Linear(linear) => linear,
    };
    let Some(last) = linear.stops.last() else { return Shader::SolidColor(tiny_skia::Color::BLACK) };
    let (start, end) = linear.points(bounds);
    let stops = linear.stops.iter().map(|s| GradientStop::new(s.offset, color(s.color))).collect();
    let (start, end) = (tiny_skia::Point::from_xy(start.x, start.y), tiny_skia::Point::from_xy(end.x, end.y));
    LinearGradient::new(start, end, stops, SpreadMode::Pad, tiny_skia::Transform::identity()).unwrap_or(Shader::SolidColor(color(last.color)))
}

/// An anti-aliased paint of `fill` over `bounds`.
pub(super) fn paint(fill: &Fill, bounds: Rect) -> Paint<'static> {
    Paint { shader: shader(fill, bounds), anti_alias: true, ..Paint::default() }
}

pub(super) fn rule(rule: Rule) -> tiny_skia::FillRule {
    match rule {
        Rule::NonZero => tiny_skia::FillRule::Winding,
        Rule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    }
}

/// The path in tiny-skia's terms, or `None` when it has no area or line to draw.
pub(super) fn outline(path: &Path) -> Option<tiny_skia::Path> {
    let mut b = PathBuilder::new();
    for seg in &path.segs {
        match *seg {
            Seg::MoveTo(p) => b.move_to(p.x, p.y),
            Seg::LineTo(p) => b.line_to(p.x, p.y),
            Seg::QuadTo(c, p) => b.quad_to(c.x, c.y, p.x, p.y),
            Seg::CubicTo(c1, c2, p) => b.cubic_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y),
            Seg::Close => b.close(),
        }
    }
    b.finish()
}

/// Fills a path by its rule, a gradient spanning its bounds, then strokes it.
pub(super) fn path(pen: &mut Pen, path: &Path, fill: Option<&Fill>, fill_rule: Rule, stroke: Option<&Stroke>) {
    let (Some(shape), Some(bounds)) = (outline(path), path.bounds()) else { return };
    if let Some(fill) = fill {
        pen.pixmap.fill_path(&shape, &paint(fill, bounds), rule(fill_rule), pen.transform, pen.mask);
    }
    let Some(stroke) = stroke.filter(|s| s.width > 0.0 && s.color.a > 0.0) else { return };
    let line = tiny_skia::Stroke {
        width: stroke.width,
        line_cap: match stroke.cap {
            Cap::Butt => LineCap::Butt,
            Cap::Round => LineCap::Round,
            Cap::Square => LineCap::Square,
        },
        line_join: match stroke.join {
            Join::Miter => LineJoin::Miter,
            Join::Round => LineJoin::Round,
            Join::Bevel => LineJoin::Bevel,
        },
        ..tiny_skia::Stroke::default()
    };
    pen.pixmap.stroke_path(&shape, &paint(&Fill::Solid(stroke.color), bounds), &line, pen.transform, pen.mask);
}
