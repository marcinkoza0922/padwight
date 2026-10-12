//! The menu features as a whole, one module each, run with `cargo nextest run suite`: menus are
//! never toggles, each kind's default close button, merged grid cells, the cell a grid starts on,
//! and submenus with breadcrumbs, back and hold to close. Each goes from the config through to
//! what the overlay and touchpad show; the editor's side is in `gui::menu_suite`.

use std::time::Instant;

use super::*;
use crate::config::{MenuItem, Span};

mod close_buttons;
mod merging;
mod screens;
mod start_cell;
mod submenus;
mod toggles;

pub(super) fn key(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

pub(super) fn open_menu(name: &str) -> ButtonAction {
    ButtonAction::OpenMenu(name.into())
}

pub(super) fn menu(name: &str, kind: MenuKind, items: Vec<MenuItem>) -> Menu {
    Menu { name: name.into(), kind, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() }
}

/// Items "1" to "n", typing KEY_1 to KEY_n.
pub(super) fn numbers(n: usize) -> Vec<MenuItem> {
    (1..=n).map(|i| MenuItem::new(i.to_string(), key(&format!("KEY_{i}")))).collect()
}

/// A grid of `n` numbered items, `columns` wide, with these items spanning more than one cell.
pub(super) fn grid(columns: u8, n: usize, spans: &[(usize, u8, u8)]) -> Menu {
    let mut items = numbers(n);
    for &(i, columns, rows) in spans {
        items[i].span = Span { columns, rows };
    }
    menu("Grid", MenuKind::Grid { columns }, items)
}

pub(super) fn held_by(b: Button) -> Opener {
    Opener { buttons: vec![b], ..Opener::default() }
}

/// A menu session that tracks its own clock, so holds and timeouts can be tested.
pub(super) struct Pad<'a> {
    pub menus: &'a [Menu],
    pub session: MenuSession,
    pub now: Instant,
}

impl<'a> Pad<'a> {
    /// `name` opened by holding LB.
    pub fn open(menus: &'a [Menu], name: &str) -> Self {
        Self::open_with(menus, name, held_by(Button::LeftBumper))
    }

    pub fn open_with(menus: &'a [Menu], name: &str, opener: Opener) -> Self {
        let session = MenuSession::open(menus, name, opener).unwrap_or_else(|| panic!("{name} didn't open"));
        Pad { menus, session, now: Instant::now() }
    }

    pub fn down(&mut self, b: Button) -> Option<MenuOutcome> {
        self.session.handle(self.menus, InputEvent::Button(b, true), self.now)
    }

    pub fn up(&mut self, b: Button) -> Option<MenuOutcome> {
        self.session.handle(self.menus, InputEvent::Button(b, false), self.now)
    }

    /// A quick press and release.
    pub fn press(&mut self, b: Button) -> Option<MenuOutcome> {
        let down = self.down(b);
        let up = self.up(b);
        down.or(up)
    }

    pub fn wait(&mut self, d: std::time::Duration) {
        self.now += d;
    }

    pub fn expired(&self) -> bool {
        self.session.expired(self.now)
    }

    pub fn view(&self) -> MenuView {
        self.session.view(self.menus).expect("the menu has a view")
    }

    pub fn title(&self) -> String {
        self.view().title
    }

    pub fn crumbs(&self) -> Vec<String> {
        self.view().crumbs
    }

    pub fn selected(&self) -> Option<usize> {
        self.view().selected
    }

    /// The label of the item under the cursor.
    pub fn on(&self) -> String {
        let view = self.view();
        view.selected.map(|i| view.items[i].label.clone()).unwrap_or_default()
    }
}

/// The key an outcome types, if it chose a key item.
pub(super) fn chose(out: Option<MenuOutcome>) -> Option<String> {
    match out {
        Some(MenuOutcome::Choose { action: ButtonAction::Keys(k), .. }) => Some(k[0].clone()),
        _ => None,
    }
}
