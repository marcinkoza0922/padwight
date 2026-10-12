use std::time::Duration;

use super::*;
use crate::nav::REPEAT_DELAY;
use crate::config::{Cluster, MenuItem};

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
    MenuItem::new(label, action)
}

fn menu(name: &str, kind: MenuKind, items: Vec<MenuItem>) -> Menu {
    Menu { name: name.into(), kind, items, cancel: None, no_cancel: false, start: 0, style: OverlayStyle::default() }
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
    ]
    .map(|m| Menu { cancel: Some(Button::East), ..m });
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
fn face_button_cascade_has_no_close_button_unless_given_one() {
    let menus = [menu("F", MenuKind::Directional { cluster: Cluster::FaceButtons }, numbers(4))];
    let mut s = MenuSession::open(&menus, "F", Opener::default()).unwrap();
    assert_eq!(chose(press(&mut s, &menus, Button::East)).as_deref(), Some("KEY_2"), "East is slot 2 (right)");
    let mut s = MenuSession::open(&menus, "F", Opener::default()).unwrap();
    assert_eq!(press(&mut s, &menus, Button::Select), None);
    let menus = [Menu { cancel: Some(Button::Select), ..menus[0].clone() }];
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
    assert_eq!(s.next_deadline(), Some(now + IDLE_CLOSE), "no more repeats, only the idle close");
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
    assert_eq!(s.next_deadline(), Some(now + IDLE_CLOSE), "no more repeats, only the idle close");
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_6"));
    assert!(s.view(&menus).unwrap().hint.contains("{dpad}"));
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
    assert_eq!(view.items[0].buttons, [Button::LeftBumper]);
    assert_eq!(view.selected, None, "nothing to move");
    assert_eq!(chose(press(&mut s, &menus, Button::LeftBumper)).as_deref(), Some("KEY_1"));
    let mut s = MenuSession::open(&menus, "Q", Opener::default()).unwrap();
    assert_eq!(chose(press(&mut s, &menus, Button::DpadDown)).as_deref(), Some("KEY_2"), "the D-pad picks, not moves");
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_3"), "A is item 3's button");
    assert_eq!(press(&mut s, &menus, Button::DpadUp), None, "a button no item has does nothing");
    let mut s = MenuSession::open(&menus, "Q", Opener::default()).unwrap();
    s.handle(&menus, InputEvent::Axis(Axis::LeftY, 0.9), Instant::now());
    assert_eq!(s.view(&menus).unwrap().selected, None, "the stick moves nothing either");
}

#[test]
fn in_a_held_button_menu_an_items_button_beats_cancel() {
    let mut items = numbers(2);
    items[0].button = Some(Button::East);
    let menus = [Menu { cancel: Some(Button::East), ..menu("Q", MenuKind::Buttons, items) }];
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
    assert!(s.view(&menus).unwrap().hint.contains("{dpad}"));
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
        Menu { cancel: Some(Button::East), ..menu("W", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(4)) },
    ];
    let mut s = MenuSession::open(&menus, "Root", held_by(Button::LeftBumper)).unwrap();
    press(&mut s, &menus, Button::South);
    assert_eq!(s.view(&menus).unwrap().title, "Sub");
    assert!(s.view(&menus).unwrap().hint.contains("{east} back"));
    assert_eq!(press(&mut s, &menus, Button::East), None);
    assert_eq!(s.view(&menus).unwrap().title, "Root");
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close), "B at the top closes");

    let mut s = MenuSession::open(&menus, "W", held_by(Button::LeftBumper)).unwrap();
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), Instant::now());
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close), "closes without choosing");
}

#[test]
fn the_bumpers_jump_a_page_through_a_long_list() {
    let menus = [menu("Long", MenuKind::List, numbers(30)), menu("Short", MenuKind::List, numbers(10))];
    let mut s = MenuSession::open(&menus, "Long", Opener::default()).unwrap();
    press(&mut s, &menus, Button::RightBumper);
    assert_eq!(s.view(&menus).unwrap().selected, Some(VISIBLE_ROWS));
    for _ in 0..5 {
        press(&mut s, &menus, Button::RightBumper);
    }
    assert_eq!(s.view(&menus).unwrap().selected, Some(29), "stops at the end");
    press(&mut s, &menus, Button::LeftBumper);
    assert_eq!(s.view(&menus).unwrap().selected, Some(29 - VISIBLE_ROWS));
    assert!(s.view(&menus).unwrap().hint.contains("{lb} {rb} page"));
    let mut short = MenuSession::open(&menus, "Short", Opener::default()).unwrap();
    press(&mut short, &menus, Button::RightBumper);
    assert_eq!(short.view(&menus).unwrap().selected, Some(0), "a list that fits doesn't page");
}

#[test]
fn a_list_with_a_close_button_stays_up_until_the_opener_is_pressed_again() {
    let menus = [menu("L", MenuKind::List, numbers(3))];
    let mut s = MenuSession::open(&menus, "L", held_by(Button::LeftBumper)).unwrap();
    let now = Instant::now();
    assert!(s.view(&menus).unwrap().hint.contains("{lb} again"), "says so while still held");
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), now), None, "stays up when let go");
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_1"), "picks still work");
    let mut s = MenuSession::open(&menus, "L", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), now), None);
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, true), now), Some(MenuOutcome::Close));
}

#[test]
fn other_held_menus_close_when_let_go_and_r3_no_longer_keeps_them() {
    let grid = menu("G", MenuKind::Grid { columns: 2 }, numbers(4));
    let wheel = Menu { cancel: Some(Button::East), ..menu("W", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(4)) };
    let menus = [grid, wheel];
    let now = Instant::now();
    let mut s = MenuSession::open(&menus, "G", held_by(Button::LeftBumper)).unwrap();
    assert!(!s.view(&menus).unwrap().hint.contains("{r3}"));
    press(&mut s, &menus, Button::RightStick);
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), now), Some(MenuOutcome::Close));
    let mut s = MenuSession::open(&menus, "W", held_by(Button::LeftBumper)).unwrap();
    press(&mut s, &menus, Button::RightStick);
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), now);
    assert_eq!(chose(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), now)).as_deref(), Some("KEY_2"), "a wheel chooses as it's let go");
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
fn a_pick_closes_the_menu_but_opening_a_submenu_does_not() {
    let menus = [
        menu("Root", MenuKind::List, vec![item("Up", key("KEY_U")), item("More", ButtonAction::OpenMenu("Sub".into()))]),
        menu("Sub", MenuKind::List, vec![item("Deep", key("KEY_D"))]),
    ];
    let mut s = MenuSession::open(&menus, "Root", held_by(Button::LeftBumper)).unwrap();
    let picked = press(&mut s, &menus, Button::South);
    assert!(matches!(picked, Some(MenuOutcome::Choose { .. })), "a pick, which closes it: {picked:?}");
    let mut s = MenuSession::open(&menus, "Root", held_by(Button::LeftBumper)).unwrap();
    press(&mut s, &menus, Button::DpadDown);
    assert_eq!(press(&mut s, &menus, Button::South), None, "a submenu isn't a pick");
    assert_eq!(s.view(&menus).unwrap().title, "Sub");
    // A list with a close button stays up when let go; the next press closes the submenu and
    // its parent together.
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), Instant::now()), None);
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, true), Instant::now()), Some(MenuOutcome::Close));
    let mut s = MenuSession::open(&menus, "Root", held_by(Button::LeftBumper)).unwrap();
    press(&mut s, &menus, Button::DpadDown);
    press(&mut s, &menus, Button::South);
    assert_eq!(chose(press(&mut s, &menus, Button::South)).as_deref(), Some("KEY_D"), "a pick in a submenu closes it all");
}

#[test]
fn directional_and_button_menus_fire_and_close_at_once() {
    let mut quick = numbers(2);
    quick[1].button = Some(Button::North);
    let menus = [
        menu(
            "Dir",
            MenuKind::Directional { cluster: Cluster::DPad },
            vec![item("Up", key("KEY_U")), item("More", ButtonAction::OpenMenu("Sub".into()))],
        ),
        menu("Sub", MenuKind::Directional { cluster: Cluster::DPad }, vec![item("Deep", key("KEY_D"))]),
        menu("Quick", MenuKind::Buttons, quick),
    ];
    // A submenu needs the same close button as its parent.
    let mut menus = menus.map(|m| Menu { cancel: Some(Button::Select), ..m });
    menus[2].cancel = None;
    for opener in [held_by(Button::LeftBumper), Opener::default()] {
        let mut s = MenuSession::open(&menus, "Dir", opener.clone()).unwrap();
        let picked = press(&mut s, &menus, Button::DpadUp);
        assert!(matches!(picked, Some(MenuOutcome::Choose { .. })), "{picked:?}");
        // A slot that opens a submenu still opens it; picking there closes everything.
        let mut s = MenuSession::open(&menus, "Dir", opener.clone()).unwrap();
        assert_eq!(press(&mut s, &menus, Button::DpadRight), None);
        assert_eq!(s.view(&menus).unwrap().title, "Sub");
        assert!(matches!(press(&mut s, &menus, Button::DpadUp), Some(MenuOutcome::Choose { .. })));
        let mut s = MenuSession::open(&menus, "Quick", opener).unwrap();
        assert!(!s.view(&menus).unwrap().hint.contains("pin"), "nothing to pin a quick menu for");
        assert!(matches!(press(&mut s, &menus, Button::North), Some(MenuOutcome::Choose { item: 1, .. })));
    }
}

#[test]
fn submenus_need_the_same_close_button_and_radial_menus_open_none() {
    let wheel = MenuKind::Radial { stick: Stick::Right, boxes: false };
    let menus = [
        menu("List", MenuKind::List, vec![item("Wheel", ButtonAction::OpenMenu("Wheel".into())), item("Grid", ButtonAction::OpenMenu("Grid".into()))]),
        menu("Wheel", wheel, vec![item("Back", ButtonAction::OpenMenu("List".into()))]),
        menu("Grid", MenuKind::Grid { columns: 2 }, numbers(2)),
        Menu { cancel: Some(Button::Start), ..menu("Start", MenuKind::List, numbers(2)) },
        Menu { cancel: Some(Button::East), ..menu("Wheel B", wheel, vec![item("Back", ButtonAction::OpenMenu("List".into()))]) },
    ];
    let mut s = MenuSession::open(&menus, "List", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(press(&mut s, &menus, Button::South), None);
    assert_eq!(s.view(&menus).unwrap().title, "List", "the radial menu has no close button to go back with");
    press(&mut s, &menus, Button::DpadDown);
    press(&mut s, &menus, Button::South);
    let view = s.view(&menus).unwrap();
    assert_eq!((view.title.as_str(), view.crumbs), ("Grid", vec!["List".to_string()]), "a grid opens from a list, with a crumb back");
    let mut s = MenuSession::open(&menus, "Wheel", held_by(Button::LeftBumper)).unwrap();
    s.handle(&menus, InputEvent::Axis(Axis::RightY, -0.9), Instant::now());
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), Instant::now()), Some(MenuOutcome::Close));
    assert!(!can_open(&menus[1], &menus[0]) && !can_open(&menus[0], &menus[1]));
    assert!(can_open(&menus[0], &menus[2]) && can_open(&menus[0], &menus[4]));
    assert!(!can_open(&menus[0], &menus[3]), "B and Start don't match");
    assert!(!can_open(&menus[4], &menus[0]), "a radial menu opens none, even with a close button");
}

#[test]
fn the_close_button_backs_out_of_a_submenu_or_held_closes_it() {
    let menus = [
        menu("Top", MenuKind::List, vec![item("Sub", ButtonAction::OpenMenu("Sub".into()))]),
        menu("Sub", MenuKind::List, numbers(2)),
    ];
    let now = Instant::now();
    let mut s = MenuSession::open(&menus, "Top", held_by(Button::LeftBumper)).unwrap();
    press(&mut s, &menus, Button::South);
    assert!(s.view(&menus).unwrap().hint.contains("{east} back (hold to close)"));
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::East, true), now), None);
    assert_eq!(s.view(&menus).unwrap().title, "Sub", "nothing yet while it's down");
    assert_eq!(s.next_deadline(), Some(now + HOLD_TO_CLOSE));
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::East, false), now), None);
    assert_eq!(s.view(&menus).unwrap().title, "Top", "a short press goes back");
    assert!(!s.expired(now + HOLD_TO_CLOSE * 2));

    press(&mut s, &menus, Button::South);
    s.handle(&menus, InputEvent::Button(Button::East, true), now);
    assert!(!s.expired(now + HOLD_TO_CLOSE / 2));
    assert!(s.expired(now + HOLD_TO_CLOSE), "held, it closes the whole menu");
    // At the top a press closes it straight away.
    let mut s = MenuSession::open(&menus, "Top", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::East, true), now), Some(MenuOutcome::Close));
}

#[test]
fn a_grid_starts_on_its_start_cell_or_the_item_merged_over_it() {
    let mut items = numbers(6);
    items[1].span = Span { columns: 2, rows: 1 };
    let grid = |start| Menu { start, ..menu("G", MenuKind::Grid { columns: 3 }, items.clone()) };
    let at = |m: Menu| {
        let menus = [m];
        MenuSession::open(&menus, "G", Opener::default()).unwrap().view(&menus).unwrap().selected
    };
    assert_eq!(at(grid(0)), Some(0));
    assert_eq!(at(grid(4)), Some(4));
    assert_eq!(at(grid(2)), Some(1), "cell 3 is under item 2");
    assert_eq!(at(grid(40)), Some(0), "past the end, the first item");
}

#[test]
fn openers_already_let_go_open_nothing_but_a_list_that_stays_up() {
    let menus = [menu("L", MenuKind::List, numbers(2)), menu("G", MenuKind::Grid { columns: 2 }, numbers(2))];
    let mut s = MenuSession::open(&menus, "G", held_by(Button::LeftBumper)).unwrap();
    assert!(!s.prime(&menus, [], []), "a tap can't hold a menu up");
    let mut s = MenuSession::open(&menus, "L", held_by(Button::LeftBumper)).unwrap();
    assert!(s.prime(&menus, [], []), "a list with a close button opens on a tap");
    assert_eq!(press(&mut s, &menus, Button::LeftBumper), Some(MenuOutcome::Close), "and the next tap closes it");
    // With nothing to hold (the command line), B closes it.
    let mut s = MenuSession::open(&menus, "L", Opener::default()).unwrap();
    assert!(s.prime(&menus, [], []));
    assert!(s.view(&menus).unwrap().hint.contains("{east} close"));
    assert_eq!(press(&mut s, &menus, Button::East), Some(MenuOutcome::Close));
}

#[test]
fn stick_direction_openers_hold_the_menu() {
    let menus = [menu("L", MenuKind::Grid { columns: 2 }, numbers(2))];
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

#[test]
fn a_radial_menu_shows_where_its_stick_points() {
    let menus = [menu("W", MenuKind::Radial { stick: Stick::Right, boxes: false }, numbers(4)), menu("L", MenuKind::List, numbers(2))];
    let mut s = MenuSession::open(&menus, "W", held_by(Button::LeftBumper)).unwrap();
    let now = Instant::now();
    assert_eq!(s.view(&menus).unwrap().aim, None, "resting in the middle");
    s.handle(&menus, InputEvent::Axis(Axis::RightX, 0.5), now);
    assert_eq!(s.view(&menus).unwrap().aim, Some(90.0), "right, before it's far enough to choose");
    s.handle(&menus, InputEvent::Axis(Axis::RightY, 0.5), now);
    assert_eq!(s.view(&menus).unwrap().aim, Some(135.0));
    let mut l = MenuSession::open(&menus, "L", held_by(Button::LeftBumper)).unwrap();
    l.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), now);
    assert_eq!(l.view(&menus).unwrap().aim, None, "only radial menus have an aim");
}

#[test]
fn a_menu_nothing_holds_up_closes_when_left_alone() {
    let menus = [menu("L", MenuKind::List, numbers(2))];
    let now = Instant::now();
    let unwatched = MenuSession::open(&menus, "L", Opener::default()).unwrap();
    assert!(!unwatched.expired(now));
    assert!(unwatched.expired(now + IDLE_CLOSE + Duration::from_secs(1)));
    assert!(unwatched.next_deadline().is_some());
    let held = MenuSession::open(&menus, "L", held_by(Button::LeftBumper)).unwrap();
    assert!(!held.expired(now + IDLE_CLOSE * 10), "a held menu stays while it's held");
}

#[test]
fn a_menu_can_have_another_close_button_or_none_and_then_closes_when_let_go() {
    let start = Menu { cancel: Some(Button::Start), ..menu("S", MenuKind::List, numbers(2)) };
    let none = Menu { no_cancel: true, ..menu("N", MenuKind::List, numbers(2)) };
    let menus = [start, none];
    let mut s = MenuSession::open(&menus, "S", held_by(Button::LeftBumper)).unwrap();
    assert!(s.view(&menus).unwrap().hint.contains("{start} to close"));
    assert_eq!(press(&mut s, &menus, Button::East), None, "B isn't the close button here");
    assert_eq!(press(&mut s, &menus, Button::Start), Some(MenuOutcome::Close));

    let mut s = MenuSession::open(&menus, "N", held_by(Button::LeftBumper)).unwrap();
    let hint = s.view(&menus).unwrap().hint;
    assert!(hint.ends_with("let go to close"), "{hint}");
    assert_eq!(press(&mut s, &menus, Button::East), None);
    assert_eq!(s.handle(&menus, InputEvent::Button(Button::LeftBumper, false), Instant::now()), Some(MenuOutcome::Close));
}

#[test]
fn a_menu_that_closes_as_it_picks_shows_the_item_it_picked() {
    let mut items = numbers(2);
    items[1].button = Some(Button::North);
    let menus = [menu("Quick", MenuKind::Buttons, items)];
    let mut s = MenuSession::open(&menus, "Quick", held_by(Button::LeftBumper)).unwrap();
    assert_eq!(s.view(&menus).unwrap().selected, None);
    press(&mut s, &menus, Button::North);
    let view = s.view(&menus).unwrap();
    assert_eq!((view.selected, view.picks), (Some(1), 1), "lit and counted, for the overlay to show");
}
