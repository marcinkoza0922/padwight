use std::time::Duration;

use super::*;
use crate::nav::REPEAT_DELAY;
use crate::config::{Cluster, GRID_MAX, MenuItem};

#[test]
fn a_two_column_list_moves_down_and_up_through_both_columns_and_left_and_right_switch() {
    // Ten rows: left column 0..5, right column 5..10.
    assert_eq!(list_step(4, 10, (0, 1)), 5, "down from the bottom of the left goes to the top of the right");
    assert_eq!(list_step(5, 10, (0, -1)), 4, "up from the top of the right goes to the bottom of the left");
    assert_eq!(list_step(9, 10, (0, 1)), 0, "down from the bottom of the right goes to the first option");
    assert_eq!(list_step(0, 10, (0, -1)), 9, "up from the first option goes to the bottom of the right");
    assert_eq!(list_step(2, 10, (1, 0)), 7, "right moves to the same row of the right column");
    assert_eq!(list_step(9, 10, (-1, 0)), 4, "left from the bottom of the right lands on the left column's last row");
    // A one-column list keeps its linear wrap.
    assert_eq!(list_step(7, 8, (0, 1)), 0);
    assert_eq!(list_step(0, 8, (0, -1)), 7);
}

#[test]
fn a_two_column_list_scrolls_only_once_the_cursor_leaves_the_visible_rows() {
    // Twenty rows: 10 per column, 8 visible per column.
    assert_eq!(list_start(7, 20), 0);
    assert_eq!(list_start(8, 20), 1);
    assert_eq!(list_start(9, 20), 2);
    // The right column's rows scroll with the left's: its row 7 is on screen, row 9 scrolls.
    assert_eq!(list_start(17, 20), 0);
    assert_eq!(list_start(19, 20), 2);
    assert_eq!(list_start(0, 8), 0);
}


fn key(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

fn item(label: &str, action: ButtonAction) -> MenuItem {
    MenuItem { label: label.into(), action, button: None, weight: 1.0 }
}

fn menu(name: &str, kind: MenuKind, items: Vec<MenuItem>) -> Menu {
    Menu { name: name.into(), kind, items, cancel: None, style: OverlayStyle::default() }
}

fn numbers(n: usize) -> Vec<MenuItem> {
    (1..=n).map(|i| item(&i.to_string(), key(&format!("KEY_{i}")))).collect()
}

fn press(s: &mut MenuSession, menus: &[Menu], b: Button) -> Option<MenuOutcome> {
    let now = Instant::now();
    let first = s.handle(menus, InputEvent::Button(b, true), now);
    let second = s.handle(menus, InputEvent::Button(b, false), now);
    first.or(second)
}

fn chose(out: Option<MenuOutcome>) -> Option<String> {
    match out {
        Some(MenuOutcome::Choose { action: ButtonAction::Keys(k), .. }) => Some(k[0].clone()),
        _ => None,
    }
}

fn held_by(b: Button) -> Opener {
    Opener { buttons: vec![b], ..Opener::default() }
}

#[test]
fn radial_aims_with_the_stick_and_chooses_on_release() {
    let menus = [menu("Weapons", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(4))];
    let mut s = MenuSession::open(&menus, "Weapons", held_by(Button::LeftBumper)).unwrap();
    let now = Instant::now();
    // Right on the stick is the second of four items (1 at the top, clockwise).
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), now);
    assert_eq!(s.view(&menus).unwrap().selected, Some(1));
    // Down is the third; the aim sticks when the stick springs back.
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.0), now);
    s.handle(&menus, InputEvent::Axis(Axis::RightY, 0.9), now);
    s.handle(&menus, InputEvent::Axis(Axis::RightY, 0.0), now);
    assert_eq!(s.view(&menus).unwrap().selected, Some(2));
    // Other buttons don't close it; letting go of the opener chooses.
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::South, false), now), None);
    assert_eq!(chose(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), now)).as_deref(), Some("KEY_3"));
}

#[test]
fn radial_released_without_aiming_just_closes_and_trigger_openers_work() {
    let menus = [menu("W", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(8))];
    let mut s = MenuSession::open(&menus, "W", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), Instant::now()), Some(MenuOutcome::Close));

    let opener = Opener { trigger: Some(Trigger::Left), ..Opener::default() };
    let mut s = MenuSession::open(&menus, "W", opener).unwrap();
    let now = Instant::now();
    s.handle(&menus, InputEvent::Axis(Axis::LeftTrigger, 1.0), now);
    // Up-left is the last of eight.
    s.handle(&menus, InputEvent::Axis(Axis::RightX, -0.7), now);
    s.handle(&menus, InputEvent::Axis(Axis::RightY, -0.7), now);
    assert_eq!(chose(s.handle(&menus, InputEvent::Axis(Axis::LeftTrigger, 0.1), now)).as_deref(), Some("KEY_8"));
}

#[test]
fn cascade_fires_slots_and_opens_submenus_with_back() {
    let menus = [
        menu(
            "Root",
            MenuKind::Directional { cluster: Cluster::DPad },
            vec![item("Up", key("KEY_U")), item("More", ButtonAction::OpenMenu("Sub".into()))],
        ),
        menu("Sub", MenuKind::Directional { cluster: Cluster::DPad }, vec![item("Deep", key("KEY_D"))]),
    ];
    let mut s = MenuSession::open(&menus, "Root", Opener::default()).unwrap();
    assert_eq!(press(&mut s, &menus, Button::DpadDown), None, "an empty slot does nothing");
    assert_eq!(press(&mut s, &menus, Button::DpadRight), None, "opens the submenu");
    assert_eq!(s.view(&menus).unwrap().title, "Sub");
    assert_eq!(press(&mut s, &menus, Button::East), None, "B goes back one level");
    assert_eq!(s.view(&menus).unwrap().title, "Root");
    assert_eq!(press(&mut s, &menus, Button::DpadRight), None);
    assert_eq!(chose(press(&mut s, &menus, Button::DpadUp)).as_deref(), Some("KEY_D"));
}

#[test]
fn face_button_cascade_cancels_with_select() {
    let menus = [menu("F", MenuKind::Directional { cluster: Cluster::FaceButtons }, numbers(4))];
    let mut s = MenuSession::open(&menus, "F", Opener::default()).unwrap();
    assert_eq!(chose(press(&mut s, &menus, Button::East)).as_deref(), Some("KEY_2"), "East is slot 2 (right)");
    let mut s = MenuSession::open(&menus, "F", Opener::default()).unwrap();
    assert_eq!(press(&mut s, &menus, Button::Select), Some(MenuOutcome::Close));
}

#[test]
fn list_moves_with_dpad_and_stick_and_wraps() {
    let menus = [menu("Pause", MenuKind::List, numbers(3))];
    let mut s = MenuSession::open(&menus, "Pause", Opener::default()).unwrap();
    press(&mut s, &menus, Button::DpadUp);
    assert_eq!(s.view(&menus).unwrap().selected, Some(2), "wraps to the bottom");
    let now = Instant::now();
    s.handle(&menus, InputEvent::Axis(Axis::LeftY, 0.9), now);
    assert_eq!(s.view(&menus).unwrap().selected, Some(0));
    // Holding repeats.
    assert!(s.tick(&menus, now + REPEAT_DELAY));
    assert_eq!(s.view(&menus).unwrap().selected, Some(1));
    s.handle(&menus, InputEvent::Axis(Axis::LeftY, 0.0), now);
    assert_eq!(s.next_deadline(), None);
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_2"));
    let mut s = MenuSession::open(&menus, "Pause", Opener::default()).unwrap();
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close));
}

#[test]
fn grid_moves_in_four_directions_and_wraps_within_rows_and_columns() {
    // 1 2 3
    // 4 5 6
    // 7
    let menus = [menu("G", MenuKind::Grid { columns: 3 }, numbers(7))];
    let mut s = MenuSession::open(&menus, "G", Opener::default()).unwrap();
    let at = |s: &MenuSession| s.view(&menus).unwrap().selected;
    press(&mut s, &menus, Button::DpadRight);
    press(&mut s, &menus, Button::DpadDown);
    assert_eq!(at(&s), Some(4), "item 5");
    press(&mut s, &menus, Button::DpadDown);
    assert_eq!(at(&s), Some(1), "column 2 has no third row, so it wraps to the top");
    press(&mut s, &menus, Button::DpadLeft);
    press(&mut s, &menus, Button::DpadLeft);
    assert_eq!(at(&s), Some(2), "wraps to the end of the row");
    press(&mut s, &menus, Button::DpadRight);
    press(&mut s, &menus, Button::DpadUp);
    assert_eq!(at(&s), Some(6), "column 1 wraps to the short last row");
    press(&mut s, &menus, Button::DpadRight);
    assert_eq!(at(&s), Some(6), "the short row wraps on itself");
    // The left stick moves the way it leans most, and holding repeats.
    let now = Instant::now();
    s.handle(&menus, InputEvent::Axis(Axis::LeftY, -0.9), now);
    assert_eq!(at(&s), Some(3));
    s.handle(&menus, InputEvent::Axis(Axis::LeftX, 0.95), now);
    assert_eq!(at(&s), Some(4), "leaning further right turns it into a step right");
    assert!(s.tick(&menus, now + REPEAT_DELAY));
    assert_eq!(at(&s), Some(5));
    s.handle(&menus, InputEvent::Axis(Axis::LeftY, 0.0), now);
    s.handle(&menus, InputEvent::Axis(Axis::LeftX, 0.0), now);
    assert_eq!(s.next_deadline(), None);
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_6"));
    assert!(s.view(&menus).unwrap().hint.contains("↑↓←→"));
}

#[test]
fn grid_steps_cover_every_cell() {
    for columns in 1..=GRID_MAX {
        for n in 1..=GRID_MAX * GRID_MAX {
            for cursor in 0..n {
                for dir in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let next = grid_step(cursor, n, columns, dir);
                    assert!(next < n, "{columns} columns, {n} items, {cursor} {dir:?}");
                    assert_eq!(grid_step(next, n, columns, (-dir.0, -dir.1)), cursor, "steps undo");
                }
            }
        }
    }
}

#[test]
fn a_button_menu_has_no_cursor_and_each_item_has_its_own_button() {
    let mut items = numbers(3);
    items[0].button = Some(Button::LeftBumper);
    items[1].button = Some(Button::DpadDown);
    items[2].button = Some(Button::South);
    let menus = [menu("Q", MenuKind::Buttons, items)];
    let mut s = MenuSession::open(&menus, "Q", Opener::default()).unwrap();
    let view = s.view(&menus).unwrap();
    assert_eq!(view.items[0].button.as_deref(), Some("LB"));
    assert_eq!(view.selected, None, "nothing to move");
    assert_eq!(chose(press(&mut s, &menus, Button::LeftBumper)).as_deref(), Some("KEY_1"));
    let mut s = MenuSession::open(&menus, "Q", Opener::default()).unwrap();
    assert_eq!(chose(press(&mut s, &menus, Button::DpadDown)).as_deref(), Some("KEY_2"), "the D-pad picks, not moves");
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_3"), "A is item 3's button");
    assert_eq!(press(&mut s, &menus, Button::DpadUp), None, "a button no item has does nothing");
    s.handle(&menus, InputEvent::Axis(Axis::LeftY, 0.9), Instant::now());
    assert_eq!(s.view(&menus).unwrap().selected, None);
}

#[test]
fn in_a_held_button_menu_an_items_button_beats_cancel() {
    let mut items = numbers(2);
    items[0].button = Some(Button::East);
    let menus = [menu("Q", MenuKind::Buttons, items)];
    let mut s = MenuSession::open(&menus, "Q", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(chose(press(&mut s, &menus, Button::East)).as_deref(), Some("KEY_1"));
    // With nothing to let go of, B stays the way out.
    let mut s = MenuSession::open(&menus, "Q", Opener::default()).unwrap();
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close));
}

#[test]
fn a_two_column_list_switches_columns_with_left_and_right() {
    let menus = [menu("Long", MenuKind::List, numbers(10)), menu("Short", MenuKind::List, numbers(3))];
    let mut s = MenuSession::open(&menus, "Long", Opener::default()).unwrap();
    press(&mut s, &menus, Button::DpadRight);
    assert_eq!(s.view(&menus).unwrap().selected, Some(5), "the top of the right column");
    s.handle(&menus, InputEvent::Axis(Axis::LeftX, -0.9), Instant::now());
    assert_eq!(s.view(&menus).unwrap().selected, Some(0), "the stick goes back left");
    assert!(s.view(&menus).unwrap().hint.contains("↑↓←→"));
    // One column: left and right do nothing.
    let mut s = MenuSession::open(&menus, "Short", Opener::default()).unwrap();
    press(&mut s, &menus, Button::DpadRight);
    assert_eq!(s.view(&menus).unwrap().selected, Some(0));
}

#[test]
fn b_backs_out_of_a_held_submenu_and_cancels_a_held_radial() {
    let menus = [
        menu("Root", MenuKind::List, vec![item("More", ButtonAction::OpenMenu("Sub".into())), item("Two", key("KEY_2"))]),
        menu("Sub", MenuKind::List, vec![item("Deep", key("KEY_D"))]),
        menu("W", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(4)),
    ];
    let mut s = MenuSession::open(&menus, "Root", held_by(Button::LeftBumper)).unwrap();
    press(&mut s, &menus, Button::South);
    assert_eq!(s.view(&menus).unwrap().title, "Sub");
    assert!(s.view(&menus).unwrap().hint.contains("B back"));
    assert_eq!(press(&mut s, &menus, Button::East), None);
    assert_eq!(s.view(&menus).unwrap().title, "Root");
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close), "B at the top closes");

    let mut s = MenuSession::open(&menus, "W", held_by(Button::LeftBumper)).unwrap();
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), Instant::now());
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close), "closes without choosing");
}

#[test]
fn a_late_tick_steps_once() {
    let menus = [menu("L", MenuKind::List, numbers(8))];
    let mut s = MenuSession::open(&menus, "L", Opener::default()).unwrap();
    let now = Instant::now();
    s.handle(&menus, InputEvent::Button(Button::DpadDown, true), now);
    assert!(s.tick(&menus, now + Duration::from_secs(2)));
    assert_eq!(s.view(&menus).unwrap().selected, Some(2));
}

#[test]
fn carousel_controls() {
    for (controls, next) in [
        (CarouselControls::Bumpers, InputEvent::Button(Button::RightBumper, true)),
        (CarouselControls::DPad, InputEvent::Button(Button::DpadRight, true)),
        (CarouselControls::RightStick, InputEvent::Axis(Axis::RightX, 0.9)),
        (CarouselControls::Triggers, InputEvent::Axis(Axis::RightTrigger, 1.0)),
    ] {
        let menus = [menu("C", MenuKind::Carousel { controls }, numbers(3))];
        let mut s = MenuSession::open(&menus, "C", Opener::default()).unwrap();
        s.handle(&menus, next, Instant::now());
        assert_eq!(s.view(&menus).unwrap().selected, Some(1), "{controls:?}");
        assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_2"), "{controls:?}");
    }
}

#[test]
fn held_menu_stays_up_for_picks_and_closes_with_its_submenu_on_release() {
    let menus = [
        menu(
            "Root",
            MenuKind::Directional { cluster: Cluster::DPad },
            vec![item("Up", key("KEY_U")), item("More", ButtonAction::OpenMenu("Sub".into()))],
        ),
        menu("Sub", MenuKind::Directional { cluster: Cluster::DPad }, vec![item("Deep", key("KEY_D"))]),
    ];
    let mut s = MenuSession::open(&menus, "Root", held_by(Button::LeftBumper)).unwrap();
    let picked = press(&mut s, &menus, Button::DpadUp);
    assert!(matches!(picked, Some(MenuOutcome::Choose { close: false, .. })), "{picked:?}");
    press(&mut s, &menus, Button::DpadRight);
    assert_eq!(s.view(&menus).unwrap().title, "Sub");
    assert_eq!(chose(press(&mut s, &menus, Button::DpadUp)).as_deref(), Some("KEY_D"));
    assert_eq!(s.view(&menus).unwrap().title, "Sub", "still up after a pick");
    // Letting go closes the submenu and its parent together.
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), Instant::now()), Some(MenuOutcome::Close));
}

#[test]
fn submenus_must_match_and_radial_menus_open_none() {
    let menus = [
        menu("List", MenuKind::List, vec![item("Wheel", ButtonAction::OpenMenu("Wheel".into()))]),
        menu("Wheel", MenuKind::Radial { stick: Stick::Right, boxes: false }, vec![item("Back", ButtonAction::OpenMenu("List".into()))]),
    ];
    let mut s = MenuSession::open(&menus, "List", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(press(&mut s, &menus, Button::South), None);
    assert_eq!(s.view(&menus).unwrap().title, "List", "a list doesn't open a radial menu");
    let mut s = MenuSession::open(&menus, "Wheel", held_by(Button::LeftBumper)).unwrap();
    s.handle(&menus, InputEvent::Axis(Axis::RightY, -0.9), Instant::now());
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), Instant::now()), Some(MenuOutcome::Close));
    assert!(!can_open(&menus[1], &menus[0]) && !can_open(&menus[0], &menus[1]));
    assert!(can_open(&menus[0], &menus[0]));
}

#[test]
fn toggled_menu_closes_on_the_next_press() {
    let menus = [menu("W", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(4))];
    let opener = Opener { buttons: vec![Button::LeftBumper], toggled: true, ..Opener::default() };
    let mut s = MenuSession::open(&menus, "W", opener).unwrap();
    let now = Instant::now();
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), now), None, "stays up when let go");
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), now);
    assert!(s.view(&menus).unwrap().hint.contains("LB again"));
    assert_eq!(chose(s.handle(&menus, InputEvent::Button(Button::LeftBumper, true), now)).as_deref(), Some("KEY_2"));
}

#[test]
fn openers_already_let_go_open_nothing_unless_toggled() {
    let menus = [menu("L", MenuKind::List, numbers(2))];
    let mut s = MenuSession::open(&menus, "L", held_by(Button::LeftBumper)).unwrap();
    assert!(!s.prime(&menus, [], []), "a tap can't hold a menu up");
    let toggled = Opener { buttons: vec![Button::LeftBumper], toggled: true, ..Opener::default() };
    let mut s = MenuSession::open(&menus, "L", toggled).unwrap();
    assert!(s.prime(&menus, [], []));
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, true), Instant::now()), Some(MenuOutcome::Close));
    // With nothing to hold (the command line), B closes it.
    let mut s = MenuSession::open(&menus, "L", Opener::default()).unwrap();
    assert!(s.prime(&menus, [], []));
    assert!(s.view(&menus).unwrap().hint.contains("B close"));
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close));
}

#[test]
fn stick_direction_openers_hold_the_menu() {
    let menus = [menu("L", MenuKind::List, numbers(2))];
    let mut s = MenuSession::open(&menus, "L", held_by(Button::LeftStickUp)).unwrap();
    assert!(s.prime(&menus, [], [(Axis::LeftY, -0.9)]));
    let now = Instant::now();
    assert_eq!(s.handle(&menus, InputEvent::Axis(Axis::LeftY, -0.5), now), None);
    assert_eq!(s.handle(&menus, InputEvent::Axis(Axis::LeftY, -0.1), now), Some(MenuOutcome::Close));
}

#[test]
fn missing_or_empty_menus_do_not_open() {
    let menus = [menu("Empty", MenuKind::List, vec![])];
    assert!(MenuSession::open(&menus, "Empty", Opener::default()).is_none());
    assert!(MenuSession::open(&menus, "Nope", Opener::default()).is_none());
}
