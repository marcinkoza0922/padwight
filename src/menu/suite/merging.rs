//! Merging grid cells: an item takes over the cells beside or below it, the items there hide
//! until it's split again, and the cursor, touchpad and overlay all treat it as one big cell.

use std::collections::BTreeSet;

use super::*;
use crate::config::GRID_MAX;

/// A small, fixed pseudo-random sequence, so the grids below are the same on every run.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % sides
    }
}

/// Grids of every width and size, with some items spanning several cells.
fn random_grids() -> Vec<(Vec<Span>, usize)> {
    let mut dice = Dice(7);
    let mut grids = Vec::new();
    for columns in 1..=GRID_MAX {
        for n in 1..=GRID_MAX * GRID_MAX {
            for _ in 0..4 {
                let spans = (0..n)
                    .map(|_| match dice.roll(3) {
                        0 => Span { columns: 1 + dice.roll(3) as u8, rows: 1 + dice.roll(3) as u8 },
                        _ => Span::ONE,
                    })
                    .collect();
                grids.push((spans, columns));
            }
        }
    }
    grids
}

fn visible(grid: &GridLayout, n: usize) -> BTreeSet<usize> {
    (0..n).filter(|&i| grid.cells(i).is_some()).collect()
}

const DIRECTIONS: [Step; 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

#[test]
fn every_cell_has_one_item_and_hidden_items_are_exactly_the_covered_ones() {
    for (spans, columns) in random_grids() {
        let n = spans.len();
        let grid = GridLayout::new(&spans, columns);
        let ctx = format!("{columns} columns, spans {spans:?}");
        assert_eq!(grid.rows, n.div_ceil(grid.columns), "{ctx}");
        for cell in 0..n {
            assert!(grid.at(cell % grid.columns, cell / grid.columns).is_some(), "cell {cell} has no item: {ctx}");
        }
        for (i, span) in spans.iter().enumerate() {
            match grid.cells(i) {
                Some(at) => {
                    assert_eq!((at.column, at.row), (i % grid.columns, i / grid.columns), "an item sits in its own cell: {ctx}");
                    assert!(at.columns <= usize::from(span.columns) && at.rows <= usize::from(span.rows), "never more than its span: {ctx}");
                    for row in at.row..at.row + at.rows {
                        for column in at.column..at.column + at.columns {
                            assert_eq!(grid.at(column, row), Some(i), "item {i} owns its cells: {ctx}");
                        }
                    }
                    assert_eq!(grid.covered_by(i), None, "{ctx}");
                }
                None => assert!(grid.covered_by(i).is_some_and(|owner| owner < i), "a hidden item is under an earlier one: {ctx}"),
            }
        }
    }
}

#[test]
fn the_cursor_only_lands_on_shown_items_and_can_reach_them_all() {
    for (spans, columns) in random_grids() {
        let n = spans.len();
        let grid = GridLayout::new(&spans, columns);
        let shown = visible(&grid, n);
        let mut reached = BTreeSet::from([0]);
        let mut todo = vec![0];
        while let Some(at) = todo.pop() {
            for dir in DIRECTIONS {
                let next = grid.step(at, dir);
                assert!(shown.contains(&next), "{columns} columns, spans {spans:?}: {at} {dir:?} landed on hidden {next}");
                if reached.insert(next) {
                    todo.push(next);
                }
            }
        }
        assert_eq!(reached, shown, "{columns} columns, spans {spans:?}");
    }
}

#[test]
fn a_merge_covers_exactly_what_it_says_hides_what_it_takes_and_a_split_brings_them_back() {
    for (spans, columns) in random_grids() {
        let n = spans.len();
        let before = GridLayout::new(&spans, columns);
        for i in visible(&before, n) {
            for right in [true, false] {
                let Some(span) = before.merged(i, right) else { continue };
                let ctx = format!("{columns} columns, spans {spans:?}, merging {i} right: {right}");
                let mut merged = spans.clone();
                merged[i] = span;
                let after = GridLayout::new(&merged, columns);
                assert_eq!((after.columns, after.rows), (before.columns, before.rows), "the grid stays the same size: {ctx}");
                let at = after.cells(i).unwrap();
                assert_eq!((at.columns, at.rows), (usize::from(span.columns), usize::from(span.rows)), "no clipping: {ctx}");
                let old = before.cells(i).unwrap();
                assert_eq!(at.columns * at.rows, old.columns * old.rows + if right { old.rows } else { old.columns }, "one more column or row: {ctx}");
                for k in visible(&before, n).into_iter().filter(|&k| k != i) {
                    match after.covered_by(k) {
                        Some(owner) => assert_eq!(owner, i, "only the merged item hides others: {ctx}"),
                        None => assert_eq!(after.cells(k), before.cells(k), "items outside stay put: {ctx}"),
                    }
                }
                merged[i] = Span::ONE;
                let split = GridLayout::new(&merged, columns);
                assert!((0..n).all(|k| split.covered_by(k) != Some(i)), "split, it hides nothing: {ctx}");
            }
        }
    }
}

/// ```text
/// A A B      item 1 merged over cells 1, 2, 4 and 5
/// A A B      item 3 merged over cells 3 and 6
/// 7 8 9
/// ```
fn merged_menu() -> Menu {
    grid(3, 9, &[(0, 2, 2), (2, 1, 2)])
}

#[test]
fn the_cursor_moves_over_a_merged_cell_as_one_and_picks_its_item() {
    let menus = [merged_menu()];
    let mut pad = Pad::open(&menus, "Grid");
    assert_eq!(pad.on(), "1");
    let mut walk = |b: Button, to: &str| {
        pad.press(b);
        assert_eq!(pad.on(), to, "{b:?}");
    };
    walk(Button::DpadRight, "3");
    walk(Button::DpadRight, "1");
    walk(Button::DpadDown, "7");
    walk(Button::DpadRight, "8");
    walk(Button::DpadUp, "1");
    walk(Button::DpadLeft, "3");
    walk(Button::DpadDown, "9");
    walk(Button::DpadLeft, "8");
    walk(Button::DpadDown, "1");
    assert_eq!(chose(pad.press(Button::South)).as_deref(), Some("KEY_1"));
    pad.press(Button::DpadRight);
    assert_eq!(pad.on(), "3");
    assert_eq!(chose(pad.press(Button::South)).as_deref(), Some("KEY_3"));
}

#[test]
fn the_touchpad_finds_a_merged_item_anywhere_in_its_cells() {
    let m = merged_menu();
    let at = |x, y| item_at(&m, (x, y), 1.0);
    assert_eq!([at(0.1, 0.1), at(0.5, 0.5), at(0.1, 0.6), at(0.6, 0.1)], [Some(0); 4], "all four of item 1's cells");
    assert_eq!([at(0.9, 0.1), at(0.9, 0.6)], [Some(2); 2]);
    assert_eq!([at(0.1, 0.9), at(0.5, 0.9), at(0.9, 0.9)], [Some(6), Some(7), Some(8)]);
    let shapes = item_shapes(&m);
    let third = 1.0 / 3.0;
    assert_eq!(shapes[0], ItemShape::Cell(0.0, 0.0, 2.0 * third, 2.0 * third));
    assert_eq!(shapes[2], ItemShape::Cell(2.0 * third, 0.0, 1.0, 2.0 * third));
    assert_eq!([1, 3, 4, 5].map(|i| shapes[i]), [ItemShape::Hidden; 4]);
    assert_eq!(shapes[7], ItemShape::Cell(third, 2.0 * third, 2.0 * third, 1.0));
}

#[test]
fn the_overlay_gets_each_items_span_and_draws_merged_grids() {
    let menus = [merged_menu()];
    let pad = Pad::open(&menus, "Grid");
    let view = pad.view();
    assert_eq!(view.items.iter().map(|item| item.span).collect::<Vec<_>>(), menus[0].items.iter().map(|item| item.span).collect::<Vec<_>>());
    assert_eq!(touch_view(&menus[0], Some(0), crate::config::TouchChoose::Lift, 0).items[0].span, Span { columns: 2, rows: 2 });
    let look = crate::overlay::draw::MenuLook { colorblind: false, family: Default::default(), nintendo_layout: false, anim: Default::default() };
    for (spans, columns) in random_grids().into_iter().step_by(7) {
        let mut m = grid(columns as u8, spans.len(), &[]);
        for (item, span) in m.items.iter_mut().zip(spans) {
            item.span = span;
        }
        let menus = [m];
        let view = Pad::open(&menus, "Grid").view();
        let _: iced::Element<'_, ()> = crate::overlay::draw::menu_panel(&view, iced::Font::DEFAULT, look, &crate::overlay::fit::Fit::contents());
    }
}

#[test]
fn spans_are_saved_only_where_an_item_is_merged() {
    let m = merged_menu();
    let text = toml::to_string_pretty(&m).unwrap();
    assert_eq!(text.matches("span").count(), 2, "{text}");
    assert_eq!(toml::from_str::<Menu>(&text).unwrap(), m);
}
