use super::*;
use crate::{
    config::ScreenPosition,
    debug::scenes,
    overlay::render::{balanced, bounds},
};

const DISPLAY: Point = Point::new(1920.0, 1080.0);

fn ops_of(scene: &str) -> Vec<Op> {
    let frame = scenes::frame(scene).unwrap();
    screen_ops(&frame, &Shown::settled(&frame), DISPLAY)
}

fn ink(ops: &[Op]) -> Rect {
    bounds(ops).expect("something is drawn")
}

#[test]
fn every_scene_draws_balanced_ops() {
    for (name, _) in scenes::SCENES {
        let Ok(frame) = scenes::frame(name) else { continue };
        let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
        assert!(balanced(&ops), "{name}");
    }
}

#[test]
fn a_centered_menu_is_centered_on_the_screen() {
    let r = ink(&ops_of("menu-list"));
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    assert!((cx - 960.0).abs() < 40.0 && (cy - 540.0).abs() < 40.0, "{r:?}");
}

#[test]
fn an_info_panel_keeps_its_margin_from_the_edge() {
    let frame = scenes::frame("info").unwrap();
    let spot = frame.info[0].style.position;
    let r = ink(&screen_ops(&frame, &Shown::settled(&frame), DISPLAY));
    if spot == ScreenPosition::TopLeft {
        assert!(r.x >= place::EDGE_MARGIN - 1.0 && r.y >= place::EDGE_MARGIN - 1.0, "{r:?}");
    }
    assert!(r.w < 1920.0 && r.h < 1080.0, "{r:?}");
}

#[test]
fn the_guide_stands_in_a_row_with_notes_and_mappings() {
    let frame = scenes::frame("guide").unwrap();
    let shown = Shown::settled(&frame);
    let cx = Context::of(&frame, DISPLAY);
    assert!(stacks::guide_up(&frame, &shown, &cx));
    let r = ink(&screen_ops(&frame, &shown, DISPLAY));
    assert!(r.x < 100.0 && r.right() > 1920.0 - 100.0, "Notes at the left edge and Mappings at the right: {r:?}");
}

#[test]
fn the_guide_row_is_off_without_its_notes() {
    let mut frame = scenes::frame("guide").unwrap();
    frame.info.clear();
    let cx = Context::of(&frame, DISPLAY);
    assert!(!stacks::guide_up(&frame, &Shown::settled(&frame), &cx));
}

#[test]
fn peek_draws_no_panels() {
    let mut frame = scenes::frame("menu-list").unwrap();
    frame.peek = true;
    assert!(bounds(&screen_ops(&frame, &Shown::settled(&frame), DISPLAY)).is_none());
}

#[test]
fn a_prompt_is_drawn_over_its_menu() {
    let frame = scenes::frame("unsaved").unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    assert!(balanced(&ops));
    let mut bare = frame.clone();
    if let Some(OverlayView::Menu(m)) = &mut bare.active {
        m.prompt = None;
    }
    let without = screen_ops(&bare, &Shown::settled(&bare), DISPLAY);
    assert!(ops.len() > without.len(), "the question adds to the menu");
}

#[test]
fn a_panel_fades_and_zooms_as_a_layer_about_its_center() {
    let frame = scenes::frame("menu-list").unwrap();
    let mut shown = Shown::settled(&frame);
    let mut anim = Anim::faint(0.5);
    anim.feel.zoom = 0.5;
    shown.panels[0].1 = anim;
    let ops = screen_ops(&frame, &shown, DISPLAY);
    let layers: Vec<_> = ops.iter().filter_map(|op| if let Op::Layer(l) = op { Some(*l) } else { None }).collect();
    assert_eq!(layers.len(), 1);
    assert!(matches!(ops.first(), Some(Op::Layer(_))) && matches!(ops.last(), Some(Op::Unlayer)), "the panel is the whole list");
    assert!(layers[0].scale < 1.0 && (layers[0].opacity - 0.5).abs() < 1e-4, "{:?}", layers[0]);
    assert!((layers[0].about.x - 960.0).abs() < 40.0);
}

#[test]
fn every_panel_is_a_layer_of_its_own() {
    let frame = scenes::frame("guide").unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    // Notes, the menu and Mappings: three layers at the top level, side by side.
    let mut depth = 0;
    let mut top = Vec::new();
    for op in &ops {
        match op {
            Op::Layer(_) | Op::Clip { .. } if depth == 0 => {
                assert!(matches!(op, Op::Layer(_)), "only panels at the top level");
                top.push(op);
                depth += 1;
            }
            Op::Layer(_) | Op::Clip { .. } => depth += 1,
            Op::Unlayer | Op::Unclip => depth -= 1,
            _ => assert!(depth > 0, "nothing drawn outside a panel: {op:?}"),
        }
    }
    assert!(top.len() >= 3, "{}", top.len());
}

#[test]
fn the_screen_picture_is_the_display_at_scale() {
    let frame = scenes::frame("menu-list").unwrap();
    let image = screen(&frame, Point::new(400.0, 300.0), 2.0, Rgba::rgb(0.2, 0.3, 0.4)).unwrap();
    assert_eq!((image.width, image.height), (800, 600));
    assert_eq!(image.pixel(0, 0), [51, 77, 102, 255]);
}
