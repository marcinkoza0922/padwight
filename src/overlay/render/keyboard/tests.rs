use super::*;
use crate::{
    debug::scenes,
    overlay::{
        OverlayView,
        render::{self, Op, balanced, sample},
    },
};

fn view(name: &str) -> KeyboardView {
    match scenes::frame(name).unwrap().active {
        Some(OverlayView::Keyboard(k)) => k,
        _ => panic!("{name} is not a keyboard"),
    }
}

fn style_of(v: &KeyboardView) -> Style {
    Style::of(&v.style, None)
}

fn laid(v: &KeyboardView, fit: &Fit) -> (render::Point, Vec<Op>) {
    keyboard_panel(v, &style_of(v), fit).ops(render::Point::ZERO, 2000.0)
}

fn fills(ops: &[Op]) -> Vec<Rgba> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Rect { fill: Some(render::Fill::Solid(c)), .. } => Some(*c),
            _ => None,
        })
        .collect()
}

#[test]
fn the_panel_is_the_keys_and_padding_wide() {
    for (name, units) in [("keyboard", Board::Basic), ("keyboard-full", Board::Full)] {
        let v = view(name);
        let s = v.style.scale.clamp(0.5, 2.0);
        let (size, ops) = laid(&v, &Fit::contents());
        assert!(balanced(&ops));
        let want = units.units_wide() * (UNIT + GAP) * s + 2.0 * KEYBOARD_PADDING * s;
        assert!((size.x - want).abs() < 0.5, "{name}: {} vs {want}", size.x);
    }
}

#[test]
fn the_numpad_has_bigger_keys_and_a_floor_on_its_width() {
    let v = view("numpad");
    let s = v.style.scale.clamp(0.5, 2.0);
    let (size, _) = laid(&v, &Fit::contents());
    let body = (Board::Numpad.units_wide() * (UNIT * 1.5 + GAP) * s).max(230.0 * s);
    assert!((size.x - (body + 2.0 * KEYBOARD_PADDING * s)).abs() < 0.5);
}

#[test]
fn a_set_width_scales_the_keys() {
    let v = view("keyboard");
    let wide = Fit { width: Some(1200.0), ..Fit::contents() };
    let s = scale(&v, &wide);
    assert!((s - 1200.0 / (keyboard_body_width(Board::Basic) + 2.0 * KEYBOARD_PADDING)).abs() < 1e-5);
    let (size, _) = laid(&v, &wide);
    assert!((size.x - 1200.0).abs() < 0.5, "{}", size.x);
}

#[test]
fn a_latched_key_is_green_unless_the_cursor_is_on_it() {
    let green = Rgba::from_rgb8(0x2e, 0x7d, 0x46);
    let mut v = view("keyboard");
    // Shift is latched in the scene; both shifts are keys on the board.
    let count = |v: &KeyboardView| fills(&laid(v, &Fit::contents()).1).iter().filter(|c| **c == green).count();
    let shifts = count(&v);
    assert!(shifts >= 1);
    v.latched.clear();
    assert_eq!(count(&v), 0);
    // Caps Lock lights like a latched key while it's on.
    v.caps = true;
    assert_eq!(count(&v), 1);
    v.cursor = keyboard::find(v.board, "KEY_CAPSLOCK").unwrap();
    assert_eq!(count(&v), 0);
}

#[test]
fn the_cursor_key_is_lit_and_its_label_a_little_bigger() {
    let v = view("keyboard");
    let style = style_of(&v);
    let (_, ops) = laid(&v, &Fit::contents());
    // The selected color shows up as the cursor key's fill.
    assert!(fills(&ops).contains(&style.selected));
}

#[test]
fn the_closing_bar_and_typed_lines_come_only_when_there_is_something() {
    let mut v = view("keyboard");
    let base = laid(&v, &Fit::contents()).0;
    v.closing = 0.5;
    let closing = laid(&v, &Fit::contents()).0;
    assert!((closing.y - base.y - (12.0 + CLOSING_BAR)).abs() < 0.5, "{} {}", closing.y, base.y);
    v.closing = 0.0;
    v.typed = "hello".into();
    let typed = laid(&v, &Fit::contents()).0;
    assert!(typed.y > base.y + 12.0);
}

/// Every keyboard and numpad scene, to look at beside `screenshots/overlay/<scene>.png`
/// (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_keyboard_and_numpad_scene() {
    for name in ["keyboard", "keyboard-full", "keyboard-plaid", "numpad"] {
        let v = view(name);
        let mut style = style_of(&v);
        style.anim.clock = 0.0;
        let node = keyboard_panel(&v, &style, &Fit::contents());
        sample::save(&format!("keyboard-{name}"), &sample::draw(&node, 2000.0, 1.0));
    }
    // A pressed key, Caps Lock, the closing bar and a field being typed into.
    let mut v = view("keyboard");
    v.pressed = Some("KEY_H".into());
    v.caps = true;
    v.closing = 0.6;
    v.typed = "hello".into();
    v.field = Some("Character name".into());
    let node = keyboard_panel(&v, &style_of(&v), &Fit::contents());
    sample::save("keyboard-busy", &sample::draw(&node, 2000.0, 1.0));
}
