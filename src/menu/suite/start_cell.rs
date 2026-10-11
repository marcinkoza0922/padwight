//! The cell a grid's cursor starts on: (1, 1) unless the menu says otherwise, every time it
//! opens, whether at the top or as a submenu.

use super::*;

fn starting_at(start: usize, m: Menu) -> Menu {
    Menu { start, ..m }
}

#[test]
fn a_grid_starts_top_left_unless_told_otherwise() {
    let menus = [grid(3, 9, &[])];
    assert_eq!(menus[0].start, 0);
    assert_eq!(Pad::open(&menus, "Grid").on(), "1");
    for start in 0..9 {
        let menus = [starting_at(start, grid(3, 9, &[]))];
        assert_eq!(Pad::open(&menus, "Grid").selected(), Some(start), "start {start}");
    }
}

#[test]
fn starting_on_a_merged_cell_starts_on_the_item_over_it() {
    // 1 1 3
    // 1 1 6
    for (start, on) in [(0, "1"), (1, "1"), (3, "1"), (4, "1"), (2, "3"), (5, "6")] {
        let menus = [starting_at(start, grid(3, 6, &[(0, 2, 2)]))];
        assert_eq!(Pad::open(&menus, "Grid").on(), on, "cell {start}");
    }
}

#[test]
fn a_start_past_the_last_cell_falls_back_to_the_first_item() {
    let menus = [starting_at(99, grid(3, 4, &[]))];
    assert_eq!(Pad::open(&menus, "Grid").on(), "1");
}

#[test]
fn it_starts_there_every_time_it_opens_not_where_it_was_left() {
    let menus = [starting_at(4, grid(3, 9, &[]))];
    let mut pad = Pad::open(&menus, "Grid");
    pad.press(Button::DpadRight);
    assert_eq!(pad.on(), "6");
    pad.up(Button::LeftBumper);
    assert_eq!(Pad::open(&menus, "Grid").on(), "5");
}

#[test]
fn a_grid_opened_as_a_submenu_starts_on_its_start_cell_too() {
    let menus = [menu("Top", MenuKind::List, vec![MenuItem::new("Grid", open_menu("Grid"))]), starting_at(7, grid(3, 9, &[]))];
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::South);
    assert_eq!((pad.title().as_str(), pad.on().as_str()), ("Grid", "8"));
    pad.press(Button::DpadLeft);
    pad.press(Button::East);
    pad.press(Button::South);
    assert_eq!(pad.on(), "8", "and again after going back and in");
}

#[test]
fn only_a_grid_cares_about_the_start_cell_and_it_is_saved_only_when_set() {
    let list = starting_at(3, menu("L", MenuKind::List, numbers(5)));
    let menus = [list];
    assert_eq!(Pad::open(&menus, "L").on(), "1", "lists start at the top");
    let text = toml::to_string_pretty(&grid(3, 4, &[])).unwrap();
    assert!(!text.contains("start"), "{text}");
    let moved = starting_at(2, grid(3, 4, &[]));
    assert_eq!(toml::from_str::<Menu>(&toml::to_string_pretty(&moved).unwrap()).unwrap().start, 2);
}
