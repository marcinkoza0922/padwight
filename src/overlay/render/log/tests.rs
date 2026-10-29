use super::*;
use crate::{
    config::OverlayStyle,
    debug::scenes,
    inputlog::LogLine,
    overlay::render::{Op, Point, balanced, sample},
};

fn cell(label: Option<&str>, hold: Option<u32>, held: bool, count: u16) -> LogCell {
    LogCell { glyph: vec![Segment::Text("A".into())], label: label.map(Into::into), hold_ms: hold, count, held }
}

fn view(lines: Vec<Vec<LogCell>>) -> LogView {
    LogView {
        name: String::new(),
        style: OverlayStyle::log(),
        opacity: 1.0,
        heading: String::new(),
        lines: lines.into_iter().map(|cells| LogLine { opacity: 1.0, cells }).collect(),
        show_labels: true,
        show_holds: true,
    }
}

fn laid(v: &LogView, fit: &Fit) -> (Point, Vec<Op>) {
    log_panel(v, None, fit).ops(Point::ZERO, 2000.0)
}

/// The alphas of the texts drawn.
fn text_alphas(ops: &[Op]) -> Vec<f32> {
    ops.iter().filter_map(|op| if let Op::Text { color, .. } = op { Some(color.a) } else { None }).collect()
}

#[test]
fn no_lines_is_nothing() {
    let (size, ops) = laid(&view(vec![]), &Fit::contents());
    assert_eq!((size.x, size.y), (0.0, 0.0));
    assert!(ops.is_empty());
}

#[test]
fn labels_and_hold_times_make_a_cell_taller() {
    let bare = laid(&view(vec![vec![cell(None, None, true, 1)]]), &Fit::contents()).0;
    let full = laid(&view(vec![vec![cell(Some("Jump"), Some(120), true, 1)]]), &Fit::contents()).0;
    assert!(full.y > bare.y + 20.0, "{full:?} {bare:?}");
    let mut v = view(vec![vec![cell(Some("Jump"), Some(120), true, 1)]]);
    v.show_labels = false;
    v.show_holds = false;
    assert_eq!(laid(&v, &Fit::contents()).0, bare);
}

#[test]
fn a_heading_sits_over_the_lines() {
    let mut v = view(vec![vec![cell(None, None, true, 1)]]);
    let bare = laid(&v, &Fit::contents()).0;
    v.heading = "Sent".into();
    let headed = laid(&v, &Fit::contents()).0;
    assert!(headed.y > bare.y + 10.0, "{headed:?} {bare:?}");
}

#[test]
fn a_press_count_widens_the_cell() {
    let once = laid(&view(vec![vec![cell(None, None, true, 1)]]), &Fit::contents()).0;
    let thrice = laid(&view(vec![vec![cell(None, None, true, 3)]]), &Fit::contents()).0;
    assert!(thrice.x > once.x + 5.0);
}

#[test]
fn a_let_go_input_is_dimmer_than_a_held_one() {
    let held = text_alphas(&laid(&view(vec![vec![cell(Some("Jump"), None, true, 1)]]), &Fit::contents()).1);
    let gone = text_alphas(&laid(&view(vec![vec![cell(Some("Jump"), None, false, 1)]]), &Fit::contents()).1);
    assert_eq!(held.len(), gone.len());
    assert!(!held.is_empty());
    for (h, g) in held.iter().zip(&gone) {
        assert!((g - h * RELEASED).abs() < 0.01, "{h} {g}");
    }
}

#[test]
fn a_fading_line_fades_its_cells_only() {
    let mut v = view(vec![vec![cell(Some("Jump"), None, true, 1)], vec![cell(Some("Jump"), None, true, 1)]]);
    v.lines[1].opacity = 0.5;
    let alphas = text_alphas(&laid(&v, &Fit::contents()).1);
    let half = alphas.len() / 2;
    for (a, b) in alphas[..half].iter().zip(&alphas[half..]) {
        assert!((b - a * 0.5).abs() < 0.01, "{a} {b}");
    }
}

#[test]
fn a_line_wraps_when_the_panel_is_held_to_its_max_width() {
    let line: Vec<LogCell> = (0..12).map(|_| cell(Some("Jump"), Some(100), true, 1)).collect();
    let v = view(vec![line]);
    let free = laid(&v, &Fit::contents()).0;
    let held = laid(&v, &Fit { max_width: 300.0, ..Fit::contents() });
    assert!(held.0.x <= 300.01 && held.0.y > free.y + 20.0, "{:?} {free:?}", held.0);
    assert!(balanced(&held.1));
}

#[test]
fn the_panel_is_exported() {
    let v = view(vec![vec![cell(None, None, true, 1)]]);
    let node = crate::overlay::render::log_panel(&v, None, &Fit::contents());
    assert_eq!(node.ops(Point::ZERO, 2000.0).0, laid(&v, &Fit::contents()).0);
    assert_eq!(crate::overlay::render::shorten("a"), "a");
}

#[test]
fn long_labels_are_cut_short() {
    assert_eq!(shorten("Jump"), "Jump");
    let cut = shorten("a very long macro name that goes on");
    assert_eq!(cut.chars().count(), 22);
    assert!(cut.ends_with('…'));
}

/// Draws every log scene with the new renderer, to look at beside
/// `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_log_scene_drawn_as_a_log_panel() {
    let mut drawn = 0;
    for (name, _) in scenes::SCENES {
        let frame = scenes::frame(name).unwrap();
        for (i, v) in frame.logs.iter().enumerate() {
            let panel = log_panel(v, None, &Fit::contents());
            let tag = if i == 0 { String::new() } else { format!("-{i}") };
            sample::save(&format!("log-{name}{tag}"), &sample::draw(&panel, 1000.0, 1.0));
            drawn += 1;
        }
    }
    assert!(drawn >= 1);
}
