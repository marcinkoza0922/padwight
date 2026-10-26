use super::*;
use crate::{
    config::{ButtonAction, Menu, MenuItem, MenuKind, OverlayStyle, Stick, TouchChoose, TouchpadConfig, TouchpadMotion},
    debug::scenes,
    figures::{Figure, trigger},
    overlay::{OverlayView, render::{self, Op, sample, text::shape}},
};

fn laid(node: &Node) -> Vec<Op> {
    node.ops(Point::ZERO, f32::INFINITY).1
}

fn texts(ops: &[Op]) -> Vec<(Point, Rect)> {
    ops.iter().filter_map(|op| if let Op::Text { at, ink, .. } = op { Some((*at, *ink)) } else { None }).collect()
}

fn svgs(ops: &[Op]) -> Vec<Rect> {
    ops.iter().filter_map(|op| if let Op::Svg { rect, .. } = op { Some(*rect) } else { None }).collect()
}

fn style() -> Style {
    Style::of(&OverlayStyle::default(), None)
}

fn pulled() -> Figure {
    Figure::Trigger { press_at: Some(0.5), zones: Vec::new(), live: Some(0.4) }
}

#[test]
fn the_caption_is_above_the_picture_and_the_note_below_all_centered() {
    for scale in [1.0, 1.5] {
        let ops = laid(&figure(&pulled(), &style(), scale));
        let (text, svg) = (texts(&ops), svgs(&ops));
        assert_eq!((text.len(), svg.len()), (2, 1));
        let caption = shape(&Spec::new("Travel", 13.0 * scale)).size;
        let note = shape(&Spec::new("Pulled 40%", 12.0 * scale)).size;
        let picture = svg[0];
        assert_eq!((picture.w, picture.h), (trigger::W * scale, trigger::H * scale));
        assert_eq!(text[0].0.y, 0.0);
        assert_eq!(picture.y, caption.y + 4.0 * scale);
        assert_eq!(text[1].0.y, picture.bottom() + 4.0 * scale);
        // Each is centered on the widest of them.
        let middle = picture.x + picture.w / 2.0;
        assert!((text[0].0.x + caption.x / 2.0 - middle).abs() < 0.01);
        assert!((text[1].0.x + note.x / 2.0 - middle).abs() < 0.01);
    }
}

#[test]
fn pictures_are_ten_apart_and_each_has_its_caption() {
    let fig = Figure::Gyro { curve: Some(crate::config::SpeedCurve::default()), speed: Some(30.0), trail: Vec::new(), at: (0.0, 0.0) };
    let ops = laid(&figure(&fig, &style(), 1.0));
    let (text, svg) = (texts(&ops), svgs(&ops));
    assert_eq!(svg.len(), 2);
    // A caption and a note for each picture.
    assert_eq!(text.len(), 4);
    let note_h = shape(&Spec::new("Turning 30°/s: 1.00× sensitivity", 12.0)).size.y;
    assert!((text[2].0.y - (text[1].0.y + note_h + 10.0)).abs() < 0.01);
}

fn grid_pad() -> (TouchpadConfig, Menu) {
    let items = ["A", "B"].map(|l| MenuItem::new(l, ButtonAction::Disabled)).to_vec();
    let menu = Menu { name: "Grid".into(), kind: MenuKind::Grid { columns: 2 }, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() };
    let pad = TouchpadConfig { motion: TouchpadMotion::Menu { menu: "Grid".into(), choose: TouchChoose::Click, overlay: true }, ..TouchpadConfig::default() };
    (pad, menu)
}

#[test]
fn the_touchpad_is_its_drawing_with_labels_centered_in_their_cells() {
    let (pad, menu) = grid_pad();
    let menus = [&menu];
    let picture = PadPicture { settings: Some((&pad, &menus)), touches: &[(0.9, 0.5)], click: false, height: 0.5, width: 300.0 };
    let (size, ops) = render::touchpad(&picture).ops(Point::ZERO, f32::INFINITY);
    assert_eq!((size.x, size.y), (300.0, 150.0));
    assert_eq!(svgs(&ops), [Rect::new(0.0, 0.0, 300.0, 150.0)]);
    // Two columns: each label centered across its half, less the 2 on either side.
    let text = texts(&ops);
    assert_eq!(text.len(), 2);
    let a = shape(&Spec::new("A", LABEL_SIZE)).size;
    assert!((text[0].0.x + a.x / 2.0 - 75.0).abs() < 0.01, "{:?}", text[0]);
    let b = shape(&Spec::new("B", LABEL_SIZE)).size;
    assert!((text[1].0.x + b.x / 2.0 - 225.0).abs() < 0.01, "{:?}", text[1]);
    // And down the middle of the cell's height.
    assert!((text[0].0.y + a.y / 2.0 - 75.0).abs() < 0.01);
    // Cut off at the cell.
    assert!(ops.iter().any(|op| matches!(op, Op::Clip { rect, .. } if *rect == Rect::new(2.0, 0.0, 146.0, 150.0))));
}

#[test]
fn a_picked_label_is_white_and_the_others_are_not() {
    let (pad, menu) = grid_pad();
    let menus = [&menu];
    let picture = PadPicture { settings: Some((&pad, &menus)), touches: &[(0.9, 0.5)], click: false, height: 0.5, width: 300.0 };
    let colors: Vec<Rgba> = laid(&touchpad(&picture)).iter().filter_map(|op| if let Op::Text { color, .. } = op { Some(*color) } else { None }).collect();
    assert_eq!(colors[0], Rgba::rgb(LABEL_RGB[0], LABEL_RGB[1], LABEL_RGB[2]));
    assert_eq!(colors[1], Rgba::WHITE);
}

#[test]
fn a_long_label_wraps_inside_its_box() {
    let pad = TouchpadConfig { motion: TouchpadMotion::Joystick { stick: Stick::Left, relative: true, radius: 0.2, deadzone: 0.1 }, ..TouchpadConfig::default() };
    let picture = PadPicture { settings: Some((&pad, &[])), touches: &[], click: false, height: 0.5, width: 120.0 };
    let ops = laid(&touchpad(&picture));
    let text = texts(&ops);
    assert_eq!(text.len(), 1);
    let (at, ink) = text[0];
    // "Left\ncenters where you touch" is several lines, held inside the 112 wide box.
    assert!(ink.right() <= 112.0 + 4.0 + 1.0, "{ink:?}");
    assert!(at.x >= 4.0 - 0.01 && at.y >= 0.0);
}

/// Every editor scene with a figure, its figure drawn in a framed panel.
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_editor_figure_drawn_in_a_framed_panel() {
    let mut drawn = 0;
    for name in ["edit-stick", "edit-gyro", "edit-trigger", "edit-touchpad", "edit-menu", "edit-controls"] {
        let Some(OverlayView::Menu(menu)) = scenes::frame(name).unwrap().active else { continue };
        let Some(fig) = menu.figure.as_deref() else { continue };
        let style = Style::of(&menu.style, None);
        let scale = menu.style.scale.clamp(0.5, 2.0);
        let panel = render::framed(render::figure(fig, &style, scale), &style, &menu.style, 26.0, &crate::overlay::fit::Fit::contents());
        sample::save(&format!("figure-{name}"), &sample::draw(&panel, 1000.0, 1.0));
        // The figure alone on the panel's color, to lay over the same crop of iced's picture.
        let ground = Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
            ops.push(Op::Rect { rect, radii: render::Corners::ZERO, fill: Some(style.background.into()), border: None });
        })
        .width(Length::Fill)
        .height(Length::Fill);
        let bare = Node::stack(render::figure(fig, &style, scale)).under(ground);
        sample::save(&format!("figure-{name}-bare"), &sample::draw(&bare, 1000.0, 1.0));
        drawn += 1;
    }
    assert!(drawn >= 3);
}

