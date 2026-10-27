//! The layers of a panel besides its box and contents: the picture and the filters under the
//! contents, the vignette and the frame marks over them. Each is a node as big as the panel.

use std::sync::Arc;

use crate::{
    assets,
    config::{BackgroundImage, Frame, OverlayStyle, Paint, Vignette},
    overlay::{
        art::{self, Filter},
        render::{Corners, Fill, Fit, Length, Linear, Markup, Node, Op, PANEL_CORNER, PatternRef, Point, Rect, Rgba, Style, Svg, paint},
    },
};

/// Where a corner is in the panel, 0 or 1 on each axis; its mark's arms run away from it.
const CORNERS: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

/// A node as big as the panel that draws what `paint` does.
fn layer(paint: impl Fn(Rect, &mut Vec<Op>) + 'static) -> Node {
    Node::leaf(paint).width(Length::Fill).height(Length::Fill)
}

/// The layers under the contents, bottom up: the picture, then the filters (see
/// `art::filters` for their order), cut to the panel's rounded box.
pub(super) fn under(style: &Style, config: &OverlayStyle) -> Vec<Node> {
    let mut layers = Vec::new();
    if !config.lite.plain
        && let Some(picture) = &config.background_image
    {
        layers.push(picture_layer(picture, style.fade));
    }
    let round = style.corners * PANEL_CORNER;
    for filter in art::filters(config, false, Some(style.anim.clock)) {
        layers.push(match filter {
            Filter::Pattern(pattern) => {
                let pattern = PatternRef::new(pattern);
                let opacity = (pattern.pattern().opacity * style.fade).clamp(0.0, 1.0);
                // The picture starts where the panel does, so a filter runs on unbroken.
                layer(move |rect, ops| {
                    let origin = Point::new(rect.x, rect.y);
                    ops.push(Op::Pattern { pattern: pattern.clone(), rect, origin, cut: art::Shape::Rounded(round), opacity });
                })
            }
            Filter::Tint(tint) => {
                let [r, g, b] = tint.rgb;
                let color = Rgba::from_rgba8(r, g, b, (tint.opacity * style.fade).clamp(0.0, 1.0));
                layer(move |rect, ops| {
                    ops.push(Op::Rect { rect, radii: Corners::all(round), fill: Some(Fill::Solid(color)), border: None });
                })
            }
        });
    }
    layers
}

/// The layers over the contents, bottom up: the vignette's four edges, then the frame's marks.
pub(super) fn over(style: &Style, config: &OverlayStyle) -> Vec<Node> {
    let mut layers = Vec::new();
    if config.lite.plain {
        return layers;
    }
    if let Some(v) = &config.ornament.vignette {
        let edges = vignette(v, style.fade);
        layers.push(layer(move |rect, ops| {
            for fill in &edges {
                ops.push(Op::Rect { rect, radii: Corners::ZERO, fill: Some(fill.clone()), border: None });
            }
        }));
    }
    if let Some(frame) = &config.ornament.frame {
        let s = config.scale.clamp(0.5, 2.0);
        let marks = marks(frame, s, style.fade);
        let reach = frame.size.max(1.0) * s;
        layers.push(layer(move |rect, ops| {
            for ((cx, cy), mark) in &marks {
                let at = Rect::new(rect.x + cx * (rect.w - reach), rect.y + cy * (rect.h - reach), reach, reach);
                ops.push(Op::Svg { svg: Svg::Markup(mark.clone()), rect: at, fit: Fit::Contain, opacity: 1.0 });
            }
        }));
    }
    layers
}

/// The picture behind the contents, covering the panel as its fit says. A PNG or JPEG is drawn
/// as a picture, an SVG as a drawing.
fn picture_layer(bg: &BackgroundImage, fade: f32) -> Node {
    let path = assets::locate("backgrounds", &bg.file);
    let fit = Fit::from(bg.fit);
    let opacity = (bg.opacity * fade).clamp(0.0, 1.0);
    let file: Arc<std::path::Path> = Arc::from(path.as_path());
    let svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    layer(move |rect, ops| {
        let file = file.clone();
        ops.push(if svg { Op::Svg { svg: Svg::File(file), rect, fit, opacity } } else { Op::Picture { file, rect, fit, opacity } });
    })
}

/// Darkens each edge of the panel: a gradient from the vignette's color at the edge to none at
/// its reach, for the top, bottom, left and right edges in turn.
fn vignette(v: &Vignette, fade: f32) -> Vec<Fill> {
    let reach = (v.reach / 100.0).clamp(0.05, 0.5);
    let p = Paint::new(&v.color, v.opacity);
    let edge = Rgba { a: (p.opacity * fade).clamp(0.0, 1.0), ..paint(&p, [0, 0, 0]) };
    let clear = Rgba { a: 0.0, ..edge };
    // Gradient angles run from the start edge: 180 is top to bottom, so it darkens the top.
    [180.0_f32, 0.0, 90.0, 270.0]
        .into_iter()
        .map(|angle| Fill::Linear(Linear::new(angle.to_radians()).stop(0.0, edge).stop(reach, clear).stop(1.0, clear)))
        .collect()
}

/// The frame's mark at each corner, as SVG at scale `s`, in the frame's color.
fn marks(f: &Frame, s: f32, fade: f32) -> Vec<((f32, f32), Markup)> {
    let ink = Paint::new(&f.color, 1.0).rgb().unwrap_or([0xff, 0xff, 0xff]);
    CORNERS.iter().map(|&corner| (corner, Markup::new(art::frame::markup(f, ink, corner, s, fade)))).collect()
}
