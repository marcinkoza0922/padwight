//! The menu editor's side of the menu features, run with the rest of the suite by
//! `cargo nextest run suite` (the session side is `menu::suite`): close button choices by kind,
//! which items may open which menus, what saving refuses, and editing a grid's cells.

use super::*;
use crate::config::{Cluster, Span};
use crate::gui::tests::*;
use crate::menu::GridLayout;

fn menu(name: &str, kind: MenuKind, cancel: Option<Button>) -> Menu {
    let items = (1..=4).map(|i| crate::config::MenuItem::new(i.to_string(), ButtonAction::Keys(vec![format!("KEY_{i}")]))).collect();
    Menu { name: name.into(), kind, items, cancel, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() }
}

#[test]
fn new_menus_get_their_kinds_close_button() {
    for (tag, default) in [
        (MenuKindTag::Radial, None),
        (MenuKindTag::Directional, None),
        (MenuKindTag::Buttons, None),
        (MenuKindTag::List, Some(Button::East)),
        (MenuKindTag::Grid, Some(Button::East)),
        (MenuKindTag::Carousel, Some(Button::East)),
    ] {
        let mut app = app();
        let _ = app.update(Message::NewMenu(tag));
        let m = &app.menus()[0];
        assert_eq!((m.cancel_button(), CloseChoice::of(m)), (default, CloseChoice::Default(default)), "{tag:?}");
        let choices = close_choices(m);
        assert_eq!(choices[0], CloseChoice::Default(default), "{tag:?}: the default comes first");
        assert_eq!(choices.contains(&CloseChoice::None), default.is_some(), "{tag:?}: “None” only when the default isn't none already");
        assert!(!choices.contains(&CloseChoice::Button(Button::Guide)), "{tag:?}");
        assert!(CloseChoice::Default(default).to_string().ends_with("(the default)"), "{tag:?}");
    }
}

#[test]
fn a_menu_without_a_default_close_button_can_be_given_one_and_set_back() {
    let mut app = app();
    let _ = app.update(Message::NewMenu(MenuKindTag::Radial));
    let _ = app.update(Message::SetMenuCancel(0, CloseChoice::Button(Button::East)));
    assert_eq!(app.menus()[0].cancel_button(), Some(Button::East));
    let _ = app.update(Message::SetMenuCancel(0, CloseChoice::Default(None)));
    assert_eq!(app.menus()[0].cancel_button(), None);
    // Switching a list to a directional menu leaves it on the new kind's default.
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    let _ = app.update(Message::SetMenuKind(0, MenuKind::Directional { cluster: Cluster::DPad }));
    assert_eq!(app.menus()[0].cancel_button(), None);
}

#[test]
fn items_offer_only_the_menus_they_can_open() {
    let menus = [
        menu("List", MenuKind::List, None),
        menu("Grid", MenuKind::Grid { columns: 2 }, None),
        menu("Start list", MenuKind::List, Some(Button::Start)),
        menu("Wheel", MenuKind::Radial { stick: Stick::Right, boxes: false }, Some(Button::East)),
        menu("Pad", MenuKind::Directional { cluster: Cluster::DPad }, None),
    ];
    let reachable: Vec<&Menu> = menus.iter().collect();
    let names = Names { menus: menus.iter().map(|m| m.name.clone()).collect(), ..Names::default() };
    let (kinds, offered) = item_choices(&menus[0], &names, &reachable);
    assert!(kinds.contains(&ActionKind::Menu));
    assert_eq!(offered.menus, ["Grid", "Wheel"], "the same close button, any kind, not itself");
    let (_, offered) = item_choices(&menus[2], &names, &reachable);
    assert!(offered.menus.is_empty(), "nothing else closes with Start");
    let (kinds, _) = item_choices(&menus[3], &names, &reachable);
    assert_eq!(kinds, RADIAL_ITEM_KINDS, "a radial menu opens none");
    let (kinds, _) = item_choices(&menus[4], &names, &reachable);
    assert!(!kinds.contains(&ActionKind::Menu), "no close button, no submenus");
    assert!(kinds.contains(&ActionKind::Keys));
}

#[test]
fn saving_refuses_a_toggled_menu() {
    let mut app = app();
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    let name = app.menus()[0].name.clone();
    app.config.general.profiles[0].set_button(Button::North, ButtonAction::toggle(ButtonAction::OpenMenu(name.clone())));
    let err = app.validate().unwrap();
    assert!(err.contains("can't be toggled"), "{err}");
    app.config.general.profiles[0].set_button(Button::North, ButtonAction::OpenMenu(name));
    assert_eq!(app.validate(), None);
}

#[test]
fn saving_checks_submenus_close_with_the_same_button() {
    let mut app = app();
    let _ = app.update(Message::NewMenu(MenuKindTag::Grid));
    let _ = app.update(Message::RenameMenu(0, "Sub".into()));
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    app.config.general.menus[0].items[0].action = ButtonAction::OpenMenu("Sub".into());
    assert_eq!(app.validate(), None, "a list can open a grid: both close with B");
    let _ = app.update(Message::SetMenuCancel(1, CloseChoice::Button(Button::Start)));
    assert!(app.validate().unwrap().contains("needs the same close button"));
    let _ = app.update(Message::SetMenuCancel(1, CloseChoice::Default(Some(Button::East))));
    let _ = app.update(Message::SetMenuCancel(0, CloseChoice::None));
    assert!(app.validate().unwrap().contains("give this menu a close button"));
}

/// A new grid of `n` items, `columns` wide, open in the editor.
fn grid_app(columns: u8, n: usize) -> App {
    let mut app = app();
    let _ = app.update(Message::NewMenu(MenuKindTag::Grid));
    let _ = app.update(Message::SetMenuKind(0, MenuKind::Grid { columns }));
    while app.menus()[0].items.len() < n {
        let _ = app.update(Message::AddMenuItem(0));
    }
    app.menus_mut()[0].items.truncate(n);
    app
}

fn layout(app: &App) -> GridLayout {
    GridLayout::of(&app.menus()[0]).unwrap()
}

#[test]
fn the_editor_merges_cells_into_one_item_and_splits_them_back() {
    let mut app = grid_app(3, 9);
    // Merge item 1 right, then down: it covers four cells.
    for right in [true, false] {
        let span = layout(&app).merged(0, right).unwrap();
        let _ = app.update(Message::SetMenuItemSpan(0, 0, span));
    }
    assert_eq!(app.menus()[0].items[0].span, Span { columns: 2, rows: 2 });
    assert_eq!([1, 3, 4].map(|i| layout(&app).covered_by(i)), [Some(0); 3]);
    assert_eq!(app.menus()[0].items.len(), 9, "the covered items are kept");
    assert_eq!(layout(&app).merged(2, false).map(|s| (s.columns, s.rows)), Some((1, 2)), "the next column merges on its own");
    assert_eq!(layout(&app).merged(4, true), None, "a hidden item can't merge");
    assert_eq!(app.validate(), None, "a merged grid saves");
    let _ = app.update(Message::SetMenuItemSpan(0, 0, Span::ONE));
    assert!((0..9).all(|i| layout(&app).covered_by(i).is_none()), "split, every item is back");
}

#[test]
fn the_editor_sets_the_start_cell_and_a_new_kind_resets_cells_and_start() {
    let mut app = grid_app(3, 6);
    assert_eq!(app.menus()[0].start, 0, "(1, 1) by default");
    let _ = app.update(Message::SetMenuStart(0, 5));
    let menus = app.menus().to_vec();
    assert_eq!(crate::menu::MenuSession::open(&menus, &menus[0].name, Opener::default()).unwrap().cursor(), Some(5));
    let _ = app.update(Message::SetMenuItemSpan(0, 1, Span { columns: 2, rows: 1 }));
    let _ = app.update(Message::SetMenuKind(0, MenuKind::List));
    assert_eq!((app.menus()[0].start, app.menus()[0].items[1].span), (0, Span::ONE), "a list has no cells");
}

#[test]
fn the_grid_editor_and_its_preview_build_with_merged_cells() {
    let mut app = grid_app(3, 9);
    let _ = app.update(Message::SetMenuItemSpan(0, 0, Span { columns: 2, rows: 2 }));
    let _ = app.update(Message::SetMenuItemSpan(0, 2, Span { columns: 1, rows: 3 }));
    let _ = app.update(Message::SetMenuStart(0, 7));
    let _ = app.update(Message::SelectGameTab(GameTab::Menus));
    let _ = app.view();
    let _ = app.update(Message::ToggleAppearance(Some(0)));
    let _ = app.view();
    assert!(grid_start_picker(0, &app.menus()[0]).is_some());
    assert!(grid_start_picker(0, &menu("L", MenuKind::List, None)).is_none(), "only grids have a start cell");
}
