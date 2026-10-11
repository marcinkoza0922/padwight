//! Each kind's default close button: none for radial, directional and button menus, which put
//! themselves away as they pick, and B for grids, lists and carousels.

use super::*;
use crate::config::{CarouselControls, Cluster};

fn radial() -> MenuKind {
    MenuKind::Radial { stick: Stick::Right, boxes: false }
}

fn every_kind() -> [(MenuKind, Option<Button>); 7] {
    [
        (radial(), None),
        (MenuKind::Directional { cluster: Cluster::DPad }, None),
        (MenuKind::Directional { cluster: Cluster::FaceButtons }, None),
        (MenuKind::Buttons, None),
        (MenuKind::List, Some(Button::East)),
        (MenuKind::Grid { columns: 2 }, Some(Button::East)),
        (MenuKind::Carousel { controls: CarouselControls::Bumpers }, Some(Button::East)),
    ]
}

#[test]
fn each_kind_has_its_default_close_button() {
    for (kind, default) in every_kind() {
        let m = menu("M", kind, numbers(4));
        assert_eq!((m.default_cancel(), m.cancel_button()), (default, default), "{kind:?}");
    }
}

#[test]
fn a_chosen_close_button_or_none_beats_the_default() {
    for (kind, _) in every_kind() {
        let start = Menu { cancel: Some(Button::Start), ..menu("M", kind, numbers(4)) };
        assert_eq!(start.cancel_button(), Some(Button::Start), "{kind:?}");
        let none = Menu { no_cancel: true, cancel: Some(Button::Start), ..menu("M", kind, numbers(4)) };
        assert_eq!(none.cancel_button(), None, "{kind:?}");
    }
}

#[test]
fn b_closes_a_grid_list_or_carousel_straight_away() {
    for (kind, default) in every_kind() {
        if default.is_none() {
            continue;
        }
        let menus = [menu("M", kind, numbers(4))];
        for opener in [held_by(Button::LeftBumper), Opener::default()] {
            let mut pad = Pad::open_with(&menus, "M", opener);
            assert!(pad.view().hint.contains("{east}"), "{kind:?}: {}", pad.view().hint);
            assert_eq!(pad.down(Button::East), Some(MenuOutcome::Close), "{kind:?}");
        }
    }
}

#[test]
fn b_does_nothing_in_a_radial_menu_by_default() {
    let menus = [menu("W", radial(), numbers(4))];
    let mut pad = Pad::open(&menus, "W");
    assert_eq!(pad.press(Button::East), None);
    assert!(!pad.view().hint.contains("{east}"), "{}", pad.view().hint);
    pad.session.handle(&menus, InputEvent::Axis(Axis::RightX, 0.9), pad.now);
    assert_eq!(chose(pad.up(Button::LeftBumper)).as_deref(), Some("KEY_2"), "letting go still chooses");
}

#[test]
fn directional_and_button_menus_put_themselves_away_as_they_pick() {
    let mut quick = numbers(2);
    quick[0].button = Some(Button::East);
    quick[1].button = Some(Button::North);
    let menus = [
        menu("Pad", MenuKind::Directional { cluster: Cluster::DPad }, numbers(4)),
        menu("Faces", MenuKind::Directional { cluster: Cluster::FaceButtons }, numbers(4)),
        menu("Quick", MenuKind::Buttons, quick),
    ];
    for opener in [held_by(Button::LeftBumper), Opener::default()] {
        let mut pad = Pad::open_with(&menus, "Pad", opener.clone());
        assert_eq!(pad.press(Button::East), None, "B is no close button here");
        assert!(matches!(pad.press(Button::DpadDown), Some(MenuOutcome::Choose { item: 2, close: true, .. })));
        // B is a slot of the face buttons now that it isn't kept back for closing.
        let mut pad = Pad::open_with(&menus, "Faces", opener.clone());
        assert!(matches!(pad.press(Button::East), Some(MenuOutcome::Choose { item: 1, close: true, .. })));
        // And it can choose a button menu's item.
        let mut pad = Pad::open_with(&menus, "Quick", opener);
        assert!(matches!(pad.press(Button::East), Some(MenuOutcome::Choose { item: 0, close: true, .. })));
    }
}

#[test]
fn a_menu_without_a_close_button_still_goes_when_its_opener_is_let_go() {
    for (kind, _) in every_kind() {
        let menus = [Menu { no_cancel: true, ..menu("M", kind, numbers(4)) }];
        let mut pad = Pad::open(&menus, "M");
        assert!(pad.view().hint.contains("let go"), "{kind:?}: {}", pad.view().hint);
        assert!(pad.up(Button::LeftBumper).is_some(), "{kind:?}");
    }
}
