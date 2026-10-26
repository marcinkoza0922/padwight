use super::*;
use crate::{
    config::{BackgroundImage, Frame, FrameKind, ImageFit, Ornament, Vignette},
    debug::scenes,
    overlay::{OverlayView, render::{self, Rgba, bounds, balanced, sample, text::Spec}},
};

/// A block of `w` by `h` that paints itself in `color`.
fn block(w: f32, h: f32, color: Rgba) -> Node {
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        ops.push(Op::Rect { rect, radii: Corners::ZERO, fill: Some(color.into()), border: None });
    })
    .width(w)
    .height(h)
}

fn plain() -> OverlayStyle {
    let mut style = OverlayStyle::default();
    style.lite.plain = true;
    style
}

fn laid(node: &Node, width: f32) -> (render::Point, Vec<Op>) {
    node.ops(render::Point::ZERO, width)
}

fn count(ops: &[Op], is: impl Fn(&Op) -> bool) -> usize {
    ops.iter().filter(|op| is(op)).count()
}

#[test]
fn the_box_is_its_contents_and_padding() {
    let config = OverlayStyle::default();
    let panel = framed(block(100.0, 50.0, Rgba::WHITE), &Style::of(&config, None), &config, 10.0, &Fit::contents());
    let (size, ops) = laid(&panel, f32::INFINITY);
    assert_eq!((size.x, size.y), (120.0, 70.0));
    assert!(balanced(&ops));
}

#[test]
fn a_set_size_and_a_max_size_hold_the_box() {
    let config = OverlayStyle::default();
    let style = Style::of(&config, None);
    let fit = Fit { width: Some(300.0), height: Some(90.0), ..Fit::contents() };
    let (size, _) = laid(&framed(block(100.0, 50.0, Rgba::WHITE), &style, &config, 10.0, &fit), 1000.0);
    assert_eq!((size.x, size.y), (300.0, 90.0));
    let fit = Fit { max_width: 80.0, max_height: 60.0, ..Fit::contents() };
    let (size, _) = laid(&framed(block(100.0, 50.0, Rgba::WHITE), &style, &config, 10.0, &fit), 1000.0);
    assert_eq!((size.x, size.y), (80.0, 60.0));
}

#[test]
fn contents_are_cut_off_past_the_max_height_less_the_padding() {
    let config = plain();
    let style = Style::of(&config, None);
    let fit = Fit { max_height: 200.0, ..Fit::contents() };
    let (size, ops) = laid(&framed(block(100.0, 500.0, Rgba::WHITE), &style, &config, 10.0, &fit), 1000.0);
    assert_eq!(size.y, 200.0);
    // The contents' own clip stops where the bottom padding starts, and nothing is seen below the box (the 500 tall block would reach 510).
    let cut = ops.iter().find_map(|op| match op {
        Op::Clip { rect, radii } if *radii == Corners::ZERO => Some(*rect),
        _ => None,
    });
    assert_eq!(cut.map(|r| (r.y, r.h)), Some((10.0, 180.0)));
    assert_eq!(bounds(&ops).map(|b| b.bottom()), Some(200.0));
}

#[test]
fn the_box_is_rounded_and_everything_in_it_is_clipped_to_it() {
    let config = OverlayStyle::default();
    let style = Style::of(&config, None);
    let (size, ops) = laid(&framed(block(200.0, 100.0, Rgba::WHITE), &style, &config, 10.0, &Fit::contents()), 1000.0);
    let radii = Corners::all(style.corners * PANEL_CORNER);
    assert!(radii.top_left > 0.0);
    let clips: Vec<_> = ops.iter().filter_map(|op| if let Op::Clip { rect, radii } = op { Some((*rect, *radii)) } else { None }).collect();
    assert!(clips.contains(&(Rect::new(0.0, 0.0, size.x, size.y), radii)));
    // The shadow and the box come first, outside every clip.
    assert!(matches!(ops[0], Op::Shadow { .. }));
    assert!(matches!(ops[1], Op::Rect { .. }));
    assert!(matches!(ops[2], Op::Clip { .. }));
}

#[test]
fn a_gradient_drifts_with_the_clock_only_when_it_is_set_to() {
    let gradient = |drift| OverlayStyle { gradient: Some(crate::config::Gradient { color: "#ff0000".into(), opacity: 1.0, angle: 90.0, drift }), ..OverlayStyle::default() };
    let angle = |config: &OverlayStyle, clock: f32| {
        let style = Style { anim: crate::motion::Anim { clock, ..crate::motion::Anim::still() }, ..Style::of(config, None) };
        let (_, ops) = laid(&framed(block(10.0, 10.0, Rgba::WHITE), &style, config, 0.0, &Fit::contents()), 100.0);
        ops.iter().find_map(|op| match op {
            Op::Rect { fill: Some(render::Fill::Linear(l)), .. } => Some(l.angle),
            _ => None,
        })
    };
    let (still, drifting) = (gradient(false), gradient(true));
    assert_eq!(angle(&still, 0.0), angle(&still, 3.0));
    assert_ne!(angle(&drifting, 0.0), angle(&drifting, 3.0));
}

#[test]
fn ornaments_add_their_layers() {
    let config = OverlayStyle {
        ornament: Ornament {
            vignette: Some(Vignette { color: "#000000".into(), opacity: 0.6, reach: 30.0 }),
            frame: Some(Frame { kind: FrameKind::Gothic, size: 40.0, width: 2.0, color: "#ffffff".into(), opacity: 1.0 }),
            ..Ornament::default()
        },
        ..OverlayStyle::default()
    };
    let style = Style::of(&config, None);
    let (_, ops) = laid(&framed(block(200.0, 100.0, Rgba::WHITE), &style, &config, 10.0, &Fit::contents()), 1000.0);
    assert_eq!(count(&ops, |op| matches!(op, Op::Rect { fill: Some(render::Fill::Linear(_)), .. })), 4);
    assert_eq!(count(&ops, |op| matches!(op, Op::Svg { .. })), 4);
    // Plain styles draw none of them.
    let config = OverlayStyle { lite: crate::config::Lite { plain: true, ..Default::default() }, ..config };
    let (_, ops) = laid(&framed(block(200.0, 100.0, Rgba::WHITE), &Style::of(&config, None), &config, 10.0, &Fit::contents()), 1000.0);
    assert_eq!(count(&ops, |op| matches!(op, Op::Svg { .. })), 0);
}

#[test]
fn a_picture_goes_under_the_contents_and_a_filter_runs_from_the_panel_corner() {
    let config = OverlayStyle {
        background_image: Some(BackgroundImage { file: "none.svg".into(), opacity: 0.5, fit: ImageFit::Cover }),
        ornament: Ornament { wood: Some(crate::config::Texture::new(0.8)), ..Ornament::default() },
        ..OverlayStyle::default()
    };
    let style = Style::of(&config, None);
    let (_, ops) = laid(&framed(block(200.0, 100.0, Rgba::WHITE), &style, &config, 10.0, &Fit::contents()), 1000.0);
    let at = |is: fn(&Op) -> bool| ops.iter().position(is);
    let (picture, pattern, contents) = (at(|op| matches!(op, Op::Svg { .. })), at(|op| matches!(op, Op::Pattern { .. })), at(|op| matches!(op, Op::Rect { fill: Some(render::Fill::Solid(c)), .. } if *c == Rgba::WHITE)));
    assert!(picture < pattern && pattern < contents, "{picture:?} {pattern:?} {contents:?}");
    let Some(Op::Pattern { origin, rect, .. }) = ops.get(pattern.unwrap()) else { panic!() };
    assert_eq!((origin.x, origin.y), (rect.x, rect.y));
}

/// Every scene that shows the sample list menu in some style, by name.
fn style_scenes() -> Vec<&'static str> {
    scenes::SCENES
        .iter()
        .map(|(name, _)| *name)
        .filter(|n| n.starts_with("frame-") || (n.starts_with("menu-") && !n.starts_with("menu-radial")) && !matches!(*n, "menu-list" | "menu-buttons" | "menu-grid" | "menu-directional" | "menu-face" | "menu-carousel"))
        .collect()
}

/// A stand-in for the menu's contents: a title and five boxes, as big as the sample menu's.
fn placeholder(style: &Style) -> Node {
    let item = |_: usize| block(378.0, 44.0, style.item).max_width(f32::INFINITY);
    let mut column = vec![Node::text(Spec::new("ACTIONS", 20.0), style.background_text).center_x(378.0)];
    column.extend((0..5).map(item));
    Node::column(column).spacing(14.0).width(378.0)
}

/// Draws every ornament and filter scene's style as a framed panel with placeholder contents, to
/// look at beside `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_ornament_and_filter_scene_drawn_as_a_framed_panel() {
    for name in style_scenes() {
        let Some(OverlayView::Menu(menu)) = scenes::frame(name).unwrap().active else { continue };
        let config = menu.style;
        let style = Style::of(&config, None);
        let fit = Fit::contents();
        let panel = render::framed(placeholder(&style), &style, &config, 26.0, &fit);
        sample::save(&format!("panel-{name}"), &sample::draw(&panel, 1000.0, 1.0));
    }
}
