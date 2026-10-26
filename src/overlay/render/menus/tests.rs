use super::*;
use crate::info::Glyphs;
use crate::overlay::render::{UNDER_PROMPT, menu_frame, menu_panel, menu_style};
use crate::{
    config::{MenuKind, Span},
    debug::scenes,
    overlay::{
        OverlayView,
        render::{Fill, Op, Rect, balanced, sample},
    },
};

fn menu(scene: &str) -> MenuView {
    match scenes::frame(scene).unwrap().active {
        Some(OverlayView::Menu(m)) => m,
        other => panic!("{scene} is not a menu: {other:?}"),
    }
}

fn style(m: &MenuView) -> Style {
    menu_style(m, None, Anim::still(), Glyphs::default(), false)
}

fn laid(node: &Node) -> Vec<Op> {
    node.ops(Point::ZERO, f32::INFINITY).1
}

/// The boxes of the items (the big outlined ones, not the glyphs' discs) in a body.
fn boxes(ops: &[Op]) -> Vec<Rect> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Rect { rect, border: Some(_), .. } if rect.w >= 100.0 && rect.h >= 40.0 => Some(*rect),
            _ => None,
        })
        .collect()
}

fn fills(ops: &[Op]) -> Vec<Rgba> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Rect { rect, fill: Some(Fill::Solid(c)), border: Some(_), .. } if rect.w >= 100.0 && rect.h >= 40.0 => Some(*c),
            _ => None,
        })
        .collect()
}

fn list_of(m: &MenuView) -> Vec<Rect> {
    boxes(&laid(&kinds::list(m, &style(m), 1.0)))
}

fn with_items(m: &MenuView, n: usize) -> MenuView {
    let mut m = m.clone();
    let first = m.items[0].clone();
    m.items = (0..n).map(|i| crate::menu::ItemView { label: format!("Item {i}"), ..first.clone() }).collect();
    m.selected = Some(0);
    m.kind = MenuKind::List;
    m
}

#[test]
fn a_list_stacks_equal_rows_six_apart() {
    let m = menu("menu-list");
    let rows = list_of(&m);
    assert_eq!(rows.len(), m.items.len());
    for pair in rows.windows(2) {
        assert_eq!((pair[0].x, pair[0].w, pair[0].h), (pair[1].x, pair[1].w, pair[1].h));
        assert!((pair[1].y - pair[0].bottom() - 6.0).abs() < 0.01, "{pair:?}");
    }
    // The width is the label's allowance, held between 300 and 560.
    assert!(rows[0].w >= 300.0 && rows[0].w <= 560.0);
    assert!(rows[0].h >= 44.0);
}

#[test]
fn a_long_list_has_two_columns_and_scrolls_with_markers() {
    let base = menu("menu-list");
    let two = with_items(&base, 12);
    let rows = list_of(&two);
    assert_eq!(rows.len(), 12);
    let (left, right): (Vec<Rect>, Vec<Rect>) = rows.iter().partition(|r| r.x == rows[0].x);
    assert_eq!((left.len(), right.len()), (6, 6));
    assert!((right[0].x - (left[0].right() + 14.0)).abs() < 0.01);
    // One column on request, however long.
    let one = MenuView { one_column: true, ..two.clone() };
    assert_eq!(list_of(&one).len(), crate::menu::VISIBLE_ROWS);
    // Past eight rows to a column, the rest is scrolled out: markers add a line above and below.
    let many = with_items(&base, 24);
    let rows = list_of(&many);
    assert_eq!(rows.len(), 2 * crate::menu::VISIBLE_ROWS);
    let flat = laid(&kinds::list(&many, &style(&many), 1.0));
    let bottom = rows.iter().map(Rect::bottom).fold(0.0, f32::max);
    let top = rows.iter().map(|r| r.y).fold(f32::MAX, f32::min);
    let ink: Vec<Rect> = flat.iter().filter_map(|op| if let Op::Text { ink, .. } = op { Some(*ink) } else { None }).collect();
    assert!(ink.iter().any(|i| i.bottom() > bottom), "a ▼ marker under the rows");
    assert!(top >= 0.0 && !ink.iter().any(|i| i.y < top), "no ▲ marker at the start");
}

#[test]
fn dividers_sit_between_rows_when_the_style_has_them() {
    let m = menu("menu-styled");
    assert!(m.style.ornament.dividers.is_some());
    let ops = laid(&kinds::list(&m, &style(&m), 1.0));
    let lines = ops.iter().filter(|op| matches!(op, Op::Rect { border: None, fill: Some(_), rect, .. } if rect.h <= 2.0)).count();
    assert_eq!(lines, m.items.len() - 1);
}

#[test]
fn a_submenu_arrow_is_at_the_rows_right_end_in_a_list_but_beside_the_label_elsewhere() {
    let m = menu("menu-list");
    let ops = laid(&kinds::list(&m, &style(&m), 1.0));
    let rows = boxes(&ops);
    let arrows: Vec<Rect> = ops.iter().filter_map(|op| if let Op::Text { ink, .. } = op { Some(*ink) } else { None }).collect();
    // The widest right edge of any text sits at the row's padded right edge: the ▸.
    let right = arrows.iter().map(Rect::right).fold(0.0, f32::max);
    assert!((right - (rows[0].right() - 14.0)).abs() < 2.0, "{right} vs {}", rows[0].right());
}

#[test]
fn a_grid_merges_cells_and_leaves_the_covered_ones_out() {
    let mut m = menu("menu-grid");
    let plain = boxes(&laid(&kinds::grid(&m, &style(&m), 1.0)));
    assert!(plain.iter().all(|r| (r.w, r.h) == (150.0, 64.0)));
    assert_eq!(plain.len(), m.items.len());
    m.items[0].span = Span { columns: 2, rows: 1 };
    let merged = boxes(&laid(&kinds::grid(&m, &style(&m), 1.0)));
    assert_eq!(merged.len(), m.items.len() - 1);
    assert_eq!((merged[0].w, merged[0].h), (308.0, 64.0));
    m.items[0].span = Span { columns: 1, rows: 2 };
    let tall = boxes(&laid(&kinds::grid(&m, &style(&m), 1.0)));
    assert!(tall.iter().any(|r| (r.w, r.h) == (150.0, 136.0)));
}

#[test]
fn a_directional_menu_is_a_cross_of_wide_boxes() {
    let m = menu("menu-directional");
    let slots = boxes(&laid(&kinds::directional(&m, &style(&m), 1.0)));
    assert_eq!(slots.len(), m.items.len().min(4));
    assert!(slots.iter().all(|r| (r.w, r.h) == (190.0, 52.0)));
    let (top, left, right, bottom) = (slots[0], slots[1], slots[2], slots[3]);
    assert_eq!(top.x, bottom.x);
    assert!(left.x < top.x && right.x > top.x);
    assert!((right.x - left.right() - 40.0).abs() < 0.01 || slots.len() < 4);
    assert!((left.y - top.bottom() - 10.0).abs() < 0.01);
}

#[test]
fn a_carousel_has_a_big_selected_item_between_smaller_ones() {
    let mut m = menu("menu-carousel");
    m = with_items(&MenuView { kind: MenuKind::List, ..m }, 7);
    m.kind = MenuKind::Carousel { controls: crate::config::CarouselControls::Bumpers };
    m.selected = Some(2);
    let strip = boxes(&laid(&kinds::carousel(&m, &style(&m), 1.0)));
    assert_eq!(strip.len(), 5);
    let sizes: Vec<(f32, f32)> = strip.iter().map(|r| (r.w, r.h)).collect();
    assert_eq!(sizes, [(120.0, 60.0), (120.0, 60.0), (170.0, 80.0), (120.0, 60.0), (120.0, 60.0)]);
    // On a touchpad all of them stay put, in order.
    m.touch = true;
    let touch = boxes(&laid(&kinds::carousel(&m, &style(&m), 1.0)));
    assert_eq!(touch.len(), 7);
    assert_eq!(touch.iter().filter(|r| r.w == 170.0).count(), 1);
}

#[test]
fn a_menu_under_a_question_is_dimmed() {
    let m = menu("unsaved");
    assert!(m.prompt.is_some());
    let fit = Fit::contents();
    let alone = MenuView { prompt: None, ..m.clone() };
    let (under, plain) = (laid(&menu_panel(&m, &style(&m), &fit).unwrap()), laid(&menu_panel(&alone, &style(&alone), &fit).unwrap()));
    let (a, b) = (fills(&under), fills(&plain));
    let (a, b) = (a.iter().map(|c| c.a).fold(0.0, f32::max), b.iter().map(|c| c.a).fold(0.0, f32::max));
    assert!(b > 0.0 && (a / b - UNDER_PROMPT).abs() < 0.01, "{a} {b}");
}

#[test]
fn the_panel_has_crumbs_title_about_and_hint_and_the_editors_figure_beside_the_list() {
    let m = menu("edit-trigger");
    assert!(m.figure.is_some() && !m.crumbs.is_empty());
    let ops = laid(&menu_panel(&m, &style(&m), &Fit::contents()).unwrap());
    assert!(balanced(&ops));
    let svg = ops.iter().find_map(|op| if let Op::Svg { rect, .. } = op { Some(*rect) } else { None }).expect("the figure's drawing");
    let rows = boxes(&ops).into_iter().filter(|r| r.w > 200.0 && r.h < 100.0).collect::<Vec<_>>();
    assert!(svg.x >= rows[0].right() + 24.0 - 0.01);
}

#[test]
fn a_radial_menu_has_no_panel_here() {
    let m = menu("menu-radial");
    assert!(menu_panel(&m, &style(&m), &Fit::contents()).is_none());
}

#[test]
fn a_note_has_no_box_and_a_picked_row_swells() {
    let mut m = menu("menu-list");
    m.items[3].tone = Tone::Note;
    let with = list_of(&m).len();
    assert_eq!(with, m.items.len() - 1);
    let anim = Anim::still();
    let c = cell(&m, 1, &Style { anim, ..style(&m) }, 17.0);
    assert!(c.state.selected && !c.note);
}

fn is_menu_scene(name: &str) -> Option<MenuView> {
    let frame = scenes::frame(name).ok()?;
    match frame.active {
        Some(OverlayView::Menu(m)) if !matches!(m.kind, MenuKind::Radial { .. }) => Some(m),
        _ => None,
    }
}

/// Every scene with a menu in it, its menu drawn in a framed panel, to look at beside
/// `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_menu_scene_drawn_as_a_framed_panel() {
    let mut drawn = 0;
    for (name, _) in scenes::SCENES {
        let Some(m) = is_menu_scene(name) else { continue };
        let panel = menu_panel(&m, &style(&m), &Fit::contents()).unwrap();
        let ops = laid(&panel);
        assert!(balanced(&ops), "{name}");
        sample::save(&format!("menu-{name}"), &sample::draw(&panel, 1000.0, 1.0));
        drawn += 1;
    }
    assert!(drawn > 30, "{drawn}");
}

#[test]
fn a_radial_menus_body_can_be_framed_with_its_title_and_hint() {
    let m = menu("menu-radial");
    let style = style(&m);
    let body = Node::space().width(200.0).height(200.0);
    let ops = laid(&menu_frame(&m, body, &style, 1.0, &Fit::contents()));
    assert!(balanced(&ops));
    let text = ops.iter().filter(|op| matches!(op, Op::Text { .. })).count();
    assert!(text >= 2, "the title and the hint");
}
