use super::*;
use crate::{
    config::OverlayStyle,
    debug::scenes,
    overlay::render::{Op, Point, balanced, sample},
};

fn text(t: &str) -> Vec<Segment> {
    vec![Segment::Text(t.into())]
}

fn view(rows: Vec<Vec<Vec<Segment>>>, title: Option<Vec<Segment>>) -> InfoView {
    InfoView { name: String::new(), style: OverlayStyle::info(), title, rows, opacity: 1.0 }
}

fn laid(view: &InfoView, fit: &Fit) -> (Point, Vec<Op>) {
    info_panel(view, None, fit, &Anim::still()).ops(Point::ZERO, 2000.0)
}

fn size(view: &InfoView, fit: &Fit) -> Point {
    laid(view, fit).0
}

#[test]
fn columns_are_as_wide_as_their_widest_cell() {
    let rows = vec![vec![3.0, 10.0], vec![7.0], vec![5.0, 4.0]];
    assert_eq!(column_widths(&rows, 1000.0, 20.0, |w| *w), [7.0, 10.0]);
    assert_eq!(column_widths::<f32>(&[], 1000.0, 20.0, |w| *w), Vec::<f32>::new());
}

#[test]
fn the_widest_columns_give_way_when_the_table_is_held_to_a_width() {
    let rows = vec![vec![100.0, 300.0, 300.0]];
    // 700 of cells and 60 of gutters, in 560: 200 too wide, shared by the two widest.
    assert_eq!(column_widths(&rows, 560.0, 20.0, |w| *w), [100.0, 200.0, 200.0]);
    let widths = column_widths(&rows, 400.0, 20.0, |w| *w);
    assert_eq!(widths.iter().sum::<f32>() + 60.0, 400.0);
    assert!(widths[0] <= 100.0 && widths[0] <= widths[1]);
}

#[test]
fn a_wider_cell_in_any_row_widens_the_column_for_all() {
    let one = view(vec![vec![text("a"), text("b")], vec![text("c"), text("d")]], None);
    let wide = view(vec![vec![text("a"), text("b")], vec![text("a much longer cell"), text("d")]], None);
    let fit = Fit::contents();
    assert!(size(&wide, &fit).x > size(&one, &fit).x + 50.0);
    assert!(balanced(&laid(&wide, &fit).1));
}

#[test]
fn a_title_sits_over_the_rows() {
    let rows = vec![vec![text("a")]];
    let plain = size(&view(rows.clone(), None), &Fit::contents());
    let titled = size(&view(rows, Some(text("Title"))), &Fit::contents());
    assert!(titled.y > plain.y + 16.0);
}

#[test]
fn a_cell_wraps_when_the_panel_is_held_to_its_max_width() {
    let long = "words that go on for longer than the panel is allowed to be wide ".repeat(2);
    let v = view(vec![vec![text("name"), text(&long)]], None);
    let free = size(&v, &Fit::contents());
    let held = size(&v, &Fit { max_width: 300.0, ..Fit::contents() });
    assert!(held.x <= 300.0 + 0.01, "{held:?}");
    assert!(held.y > free.y + 10.0, "{held:?} {free:?}");
}

#[test]
fn the_panel_and_its_table_are_exported() {
    let v = view(vec![vec![text("a")]], None);
    let node = crate::overlay::render::info_panel(&v, None, &Fit::contents(), &Anim::still());
    assert_eq!(node.ops(Point::ZERO, 2000.0).0, size(&v, &Fit::contents()));
    assert_eq!(crate::overlay::render::column_widths(&[vec![1.0f32]], 100.0, 0.0, |w| *w), [1.0]);
}

#[test]
fn padding_scales_with_the_style() {
    let mut v = view(vec![vec![text("a")]], None);
    let one = size(&v, &Fit::contents());
    v.style.scale = 2.0;
    let two = size(&v, &Fit::contents());
    assert!(two.x > one.x * 1.6 && two.y > one.y * 1.6);
}

#[test]
fn a_faded_out_panel_draws_invisible_text() {
    let mut v = view(vec![vec![text("a")]], None);
    v.opacity = 0.0;
    let (_, ops) = laid(&v, &Fit::contents());
    assert!(ops.iter().all(|op| !matches!(op, Op::Text { color, .. } if color.a > 0.0)));
}

/// Draws every info scene with the new renderer, to look at beside
/// `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_info_scene_drawn_as_an_info_panel() {
    let mut drawn = 0;
    for (name, _) in scenes::SCENES {
        let frame = scenes::frame(name).unwrap();
        if let Some(icons) = &frame.pack_icons {
            crate::icons::set_pack_icons(icons);
        }
        for (i, v) in frame.info.iter().enumerate() {
            let panel = info_panel(v, None, &Fit::contents(), &Anim::still());
            let tag = if i == 0 { String::new() } else { format!("-{i}") };
            sample::save(&format!("info-{name}{tag}"), &sample::draw(&panel, 1000.0, 1.0));
            drawn += 1;
        }
    }
    assert!(drawn >= 4);
}
