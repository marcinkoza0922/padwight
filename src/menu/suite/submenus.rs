//! Opening a menu from a menu: only between menus with the same close button, which then goes
//! back a level when tapped and closes the whole menu when held. The submenu takes the place of
//! the menu it was opened from, with breadcrumbs back to it.

use std::time::Duration;

use super::*;
use crate::config::{CarouselControls, Cluster};

fn radial() -> MenuKind {
    MenuKind::Radial { stick: Stick::Right, boxes: false }
}

fn closing_with(cancel: Option<Button>, m: Menu) -> Menu {
    Menu { cancel, no_cancel: cancel.is_none(), ..m }
}

/// Top (a list) opens Middle (a grid), which opens Bottom (a list); each has two numbered items
/// after its link.
fn three_deep() -> [Menu; 3] {
    let link = |to: &str| MenuItem::new(to, open_menu(to));
    let with_link = |to: &str| {
        let mut items = vec![link(to)];
        items.extend(numbers(2));
        items
    };
    [menu("Top", MenuKind::List, with_link("Middle")), menu("Middle", MenuKind::Grid { columns: 3 }, with_link("Bottom")), menu("Bottom", MenuKind::List, numbers(3))]
}

#[test]
fn which_menus_can_open_which() {
    let kinds = [
        MenuKind::List,
        MenuKind::Grid { columns: 2 },
        MenuKind::Carousel { controls: CarouselControls::Bumpers },
        MenuKind::Directional { cluster: Cluster::DPad },
        MenuKind::Buttons,
        radial(),
    ];
    let closes = [None, Some(Button::East), Some(Button::Start)];
    for parent_kind in kinds {
        for child_kind in kinds {
            for parent_close in closes {
                for child_close in closes {
                    let parent = closing_with(parent_close, menu("P", parent_kind, numbers(4)));
                    let child = closing_with(child_close, menu("C", child_kind, numbers(4)));
                    let expected = !matches!(parent_kind, MenuKind::Radial { .. }) && parent_close.is_some() && parent_close == child_close;
                    assert_eq!(can_open(&parent, &child), expected, "{parent_kind:?} ({parent_close:?}) opening {child_kind:?} ({child_close:?})");
                }
            }
        }
    }
    let list = menu("L", MenuKind::List, numbers(2));
    assert!(!can_open(&list, &menu("Empty", MenuKind::List, vec![])), "an empty menu opens nothing");
    assert!(can_open(&list, &menu("Me", MenuKind::List, numbers(1))), "a menu can open another list with B, by default");
    assert!(!can_open(&menu("D", MenuKind::Directional { cluster: Cluster::DPad }, numbers(4)), &list), "a directional menu has no close button by default");
}

#[test]
fn a_submenu_takes_the_place_of_its_parent_with_breadcrumbs_back() {
    let menus = three_deep();
    let mut pad = Pad::open(&menus, "Top");
    assert_eq!((pad.title(), pad.crumbs()), ("Top".to_string(), Vec::<String>::new()));
    assert_eq!(pad.press(Button::South), None, "opening a submenu runs nothing");
    assert_eq!((pad.title(), pad.crumbs()), ("Middle".to_string(), vec!["Top".to_string()]));
    assert_eq!(pad.view().kind, MenuKind::Grid { columns: 3 }, "the grid's own view, in place of the list");
    pad.press(Button::South);
    assert_eq!((pad.title(), pad.crumbs()), ("Bottom".to_string(), vec!["Top".to_string(), "Middle".to_string()]));
}

#[test]
fn tapping_the_close_button_goes_back_one_level_to_where_the_cursor_was() {
    let mut menus = three_deep();
    // The link sits second in Top, so going back must keep the cursor on it.
    menus[0].items.swap(0, 1);
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::DpadDown);
    pad.press(Button::South);
    pad.press(Button::DpadRight);
    pad.press(Button::DpadLeft);
    pad.press(Button::South);
    assert_eq!(pad.title(), "Bottom");
    assert_eq!(pad.press(Button::East), None, "a tap doesn't close");
    assert_eq!((pad.title().as_str(), pad.on().as_str()), ("Middle", "Bottom"));
    assert_eq!(pad.press(Button::East), None);
    assert_eq!((pad.title().as_str(), pad.on().as_str()), ("Top", "Middle"));
    assert_eq!(pad.press(Button::East), Some(MenuOutcome::Close), "at the top it closes");
}

#[test]
fn going_back_waits_for_the_close_button_to_come_up() {
    let menus = three_deep();
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::South);
    assert_eq!(pad.down(Button::East), None);
    pad.wait(HOLD_TO_CLOSE - Duration::from_millis(1));
    assert!(!pad.expired());
    assert_eq!(pad.title(), "Middle", "still down: nothing yet");
    assert_eq!(pad.up(Button::East), None);
    assert_eq!(pad.title(), "Top", "let go just in time: back");
    assert!(!pad.expired());
}

#[test]
fn holding_the_close_button_in_a_submenu_closes_the_whole_menu() {
    let menus = three_deep();
    for depth in 1..=2 {
        let mut pad = Pad::open(&menus, "Top");
        for _ in 0..depth {
            pad.press(Button::South);
        }
        pad.down(Button::East);
        assert_eq!(pad.session.next_deadline(), Some(pad.now + HOLD_TO_CLOSE), "the daemon wakes to close it");
        pad.wait(HOLD_TO_CLOSE);
        assert!(pad.expired(), "{depth} deep");
    }
}

#[test]
fn the_hint_says_what_the_close_button_does_at_each_level() {
    let menus = three_deep();
    let mut pad = Pad::open(&menus, "Top");
    let top = pad.view().hint;
    assert!(top.contains("{back} to close") && !top.contains("back (hold"), "{top}");
    pad.press(Button::South);
    assert!(pad.view().hint.contains("{back} back (hold to close)"), "{}", pad.view().hint);
    let mut pad = Pad::open_with(&menus, "Top", Opener::default());
    pad.press(Button::South);
    assert!(pad.view().hint.contains("{back} back (hold to close)"), "unwatched: {}", pad.view().hint);
}

#[test]
fn a_custom_close_button_works_as_back_too() {
    let menus = three_deep().map(|m| Menu { cancel: Some(Button::Start), ..m });
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::South);
    assert_eq!(pad.press(Button::East), None);
    assert_eq!(pad.title(), "Middle", "B isn't the close button here");
    assert!(pad.view().hint.contains("{start} back"));
    pad.press(Button::Start);
    assert_eq!(pad.title(), "Top");
    pad.press(Button::South);
    pad.down(Button::Start);
    pad.wait(HOLD_TO_CLOSE);
    assert!(pad.expired());
}

#[test]
fn a_link_to_a_menu_with_another_close_button_does_nothing() {
    let mut menus = three_deep();
    menus[1].cancel = Some(Button::Start);
    let mut pad = Pad::open(&menus, "Top");
    assert_eq!(pad.press(Button::South), None);
    assert_eq!(pad.title(), "Top");
    let mut menus = three_deep();
    menus[0].no_cancel = true;
    let mut pad = Pad::open(&menus, "Top");
    assert_eq!(pad.press(Button::South), None);
    assert_eq!(pad.title(), "Top", "nor from a menu without a close button");
}

#[test]
fn picks_in_a_submenu_run_and_the_opener_closes_everything() {
    let menus = three_deep();
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::South);
    pad.press(Button::South);
    pad.press(Button::DpadDown);
    assert_eq!(chose(pad.press(Button::South)).as_deref(), Some("KEY_2"), "a pick, which closes every level");
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::South);
    pad.press(Button::South);
    assert_eq!(pad.title(), "Bottom");
    assert_eq!(pad.up(Button::LeftBumper), None, "a list with a close button stays up when let go");
    assert_eq!(pad.down(Button::LeftBumper), Some(MenuOutcome::Close));
}

#[test]
fn a_list_left_up_goes_back_and_closes_from_a_submenu_the_same_way() {
    let menus = three_deep();
    let mut pad = Pad::open(&menus, "Top");
    assert_eq!(pad.up(Button::LeftBumper), None, "stays up");
    pad.press(Button::South);
    pad.press(Button::East);
    assert_eq!(pad.title(), "Top");
    pad.press(Button::South);
    pad.down(Button::East);
    pad.wait(HOLD_TO_CLOSE);
    assert!(pad.expired());
}

#[test]
fn any_kind_with_the_same_close_button_can_be_a_submenu() {
    let to = |kind| closing_with(Some(Button::East), menu("Sub", kind, numbers(4)));
    let directional = closing_with(Some(Button::East), menu("Top", MenuKind::Directional { cluster: Cluster::DPad }, vec![MenuItem::new("Sub", open_menu("Sub"))]));
    // A directional menu opening a radial one: the wheel chooses as the opener is let go.
    let menus = [directional, to(radial())];
    let mut pad = Pad::open(&menus, "Top");
    assert_eq!(pad.press(Button::DpadUp), None);
    assert_eq!((pad.title(), pad.crumbs()), ("Sub".to_string(), vec!["Top".to_string()]));
    pad.session.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), pad.now);
    assert_eq!(chose(pad.up(Button::LeftBumper)).as_deref(), Some("KEY_2"));
    // And B still goes back out of the wheel.
    let mut pad = Pad::open(&menus, "Top");
    pad.press(Button::DpadUp);
    pad.press(Button::East);
    assert_eq!(pad.title(), "Top");
    for kind in [MenuKind::Carousel { controls: CarouselControls::Bumpers }, MenuKind::Buttons, MenuKind::Grid { columns: 2 }] {
        let menus = [menu("Top", MenuKind::List, vec![MenuItem::new("Sub", open_menu("Sub"))]), to(kind)];
        let mut pad = Pad::open(&menus, "Top");
        pad.press(Button::South);
        assert_eq!(pad.view().kind, kind);
        pad.press(Button::East);
        assert_eq!(pad.title(), "Top", "{kind:?}");
    }
}

#[test]
fn menus_that_open_each_other_just_keep_adding_crumbs() {
    let menus = [
        menu("A", MenuKind::List, vec![MenuItem::new("B", open_menu("B"))]),
        menu("B", MenuKind::List, vec![MenuItem::new("A", open_menu("A"))]),
    ];
    let mut pad = Pad::open(&menus, "A");
    pad.press(Button::South);
    pad.press(Button::South);
    assert_eq!((pad.title(), pad.crumbs()), ("A".to_string(), vec!["A".to_string(), "B".to_string()]));
    pad.press(Button::East);
    pad.press(Button::East);
    assert_eq!(pad.title(), "A");
    assert_eq!(pad.press(Button::East), Some(MenuOutcome::Close));
}
