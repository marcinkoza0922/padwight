//! Edit Controls' menu and info overlay pages: a game's menus (their items, the items' actions
//! and buttons, the order and kind) and whether each info overlay is always up and where. Names
//! and texts need a keyboard, so they're left to the settings window.

use crate::{
    config::{Button, ButtonAction, Config, GRID_MAX, InfoOverlay, Menu, MenuItem, MenuKind, MenuKindTag, OverlayStyle, ScreenPosition},
    menu::ItemView,
    config::ItemKind,
    pack::game_references,
};

use super::{EditStep, LayoutEditor, Page, active_game_mut, buttons::Slot, views::{add_item, item, remove_item}};

/// The label a new item has until its action names it.
const NEW_ITEM: &str = "New item";

/// The rows of a menu item's page: its action, then (in a button menu) its button, then
/// moving and removing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemRow {
    Action,
    Button,
    Up,
    Down,
    Remove,
}

fn item_rows(menu: &Menu) -> Vec<ItemRow> {
    let button = matches!(menu.kind, MenuKind::Buttons).then_some(ItemRow::Button);
    [ItemRow::Action].into_iter().chain(button).chain([ItemRow::Up, ItemRow::Down, ItemRow::Remove]).collect()
}

fn menus(config: &Config) -> &[Menu] {
    &config.active_game().menus
}

fn menu_mut(config: &mut Config, m: usize) -> Option<&mut Menu> {
    active_game_mut(config)?.menus.get_mut(m)
}

/// Whether anything in the game opens menu `m`, so it can't be deleted.
fn menu_in_use(config: &Config, m: usize) -> bool {
    let game = config.active_game();
    game.menus.get(m).is_some_and(|menu| game_references(game).contains(&(ItemKind::Menu, menu.name.clone())))
}

/// The item's action (for the editor's action page, through [`Slot::MenuItem`]).
pub(super) fn item_action(config: &Config, m: usize, i: usize) -> ButtonAction {
    menus(config).get(m).and_then(|menu| menu.items.get(i)).map_or(ButtonAction::Disabled, |item| item.action.clone())
}

/// Sets an item's action. An item still named for its old action (or new) is named for the new
/// one, since there's no keyboard to name it here.
pub(super) fn set_item_action(config: &mut Config, m: usize, i: usize, action: ButtonAction) {
    let Some(item) = menu_mut(config, m).and_then(|menu| menu.items.get_mut(i)) else { return };
    if item.label.is_empty() || item.label == NEW_ITEM || item.label == item.action.summary() {
        item.label = action.summary();
    }
    item.action = action;
}

/// A menu's name, for page titles.
pub(super) fn menu_name(config: &Config, m: usize) -> Option<&str> {
    menus(config).get(m).map(|menu| menu.name.as_str())
}

fn menus_page(config: &Config) -> (Vec<ItemView>, &'static str) {
    let mut rows: Vec<ItemView> =
        menus(config).iter().map(|m| item(format!("{} · {} · {} items", m.name, m.kind.tag().short(), m.items.len()))).collect();
    rows.push(add_item("Add menu".into()));
    (rows, "{south} open or add · {east} back")
}

fn menu_page(config: &Config, m: usize) -> (Vec<ItemView>, &'static str) {
    let Some(menu) = menus(config).get(m) else { return (Vec::new(), "{east} back") };
    let mut rows: Vec<ItemView> = menu
        .items
        .iter()
        .map(|it| ItemView { buttons: it.button.into_iter().collect(), keyword: it.action.keyword(), ..item(format!("{}: {}", it.label, it.action.summary())) })
        .collect();
    rows.push(add_item("Add item".into()));
    rows.push(item(format!("Kind: {}", menu.kind.tag().short())));
    rows.push(if menu_in_use(config, m) { item("Opened by a button: can't delete".into()) } else { remove_item("Delete menu".into()) });
    (rows, "{south} open, add or change · {east} back")
}

fn item_page(config: &Config, m: usize, i: usize) -> (Vec<ItemView>, &'static str) {
    let Some(menu) = menus(config).get(m) else { return (Vec::new(), "{east} back") };
    let Some(it) = menu.items.get(i) else { return (Vec::new(), "{east} back") };
    let rows = item_rows(menu)
        .into_iter()
        .map(|row| match row {
            ItemRow::Action => ItemView { keyword: it.action.keyword(), ..item(format!("Action: {}", it.action.summary())) },
            ItemRow::Button => ItemView {
                buttons: it.button.into_iter().collect(),
                ..item(if it.button.is_some() { "Its button (change)".into() } else { "Its button: none (choose)".into() })
            },
            ItemRow::Up => item("Move up".into()),
            ItemRow::Down => item("Move down".into()),
            ItemRow::Remove => remove_item("Remove item".into()),
        })
        .collect();
    (rows, "{south} choose · {east} back")
}

fn infos_page(config: &Config) -> (Vec<ItemView>, &'static str) {
    let rows = config
        .active_game()
        .info
        .iter()
        .map(|o| item(format!("{} · {} · {}", o.name, if o.always { "always up" } else { "shown by an action" }, o.style.position)))
        .collect();
    (rows, "{south} open · {east} back")
}

fn info_page(config: &Config, i: usize) -> (Vec<ItemView>, &'static str) {
    let Some(o) = config.active_game().info.get(i) else { return (Vec::new(), "{east} back") };
    let rows = vec![item(format!("Always on screen: {}", if o.always { "yes" } else { "no" })), item(format!("Position: {}", o.style.position))];
    (rows, "{south} change · {left} {right} move it · {east} back")
}

fn info_mut(config: &mut Config, i: usize) -> Option<&mut InfoOverlay> {
    active_game_mut(config)?.info.get_mut(i)
}

/// The next of the nine screen spots, `dir` steps on, in reading order.
fn next_position(at: ScreenPosition, dir: i32) -> ScreenPosition {
    let grid = ScreenPosition::GRID;
    let i = grid.iter().position(|p| *p == at).unwrap_or(0) as i32;
    grid[(i + dir).rem_euclid(grid.len() as i32) as usize]
}

/// The next kind a menu can be, `dir` steps on. A directional menu holds four items, so it's
/// skipped for a longer menu; a grid gets enough columns for its rows.
fn next_kind(menu: &Menu, dir: i32) -> MenuKind {
    let tags = MenuKindTag::ALL;
    let at = tags.iter().position(|t| *t == menu.kind.tag()).unwrap_or(0) as i32;
    let fits = |tag: &MenuKindTag| *tag != MenuKindTag::Directional || menu.items.len() <= 4;
    let next = (1..=tags.len() as i32).map(|step| tags[(at + step * dir).rem_euclid(tags.len() as i32) as usize]).find(fits).unwrap_or(menu.kind.tag());
    match MenuKind::default_for(next) {
        MenuKind::Grid { columns } => MenuKind::Grid { columns: columns.max(menu.items.len().div_ceil(GRID_MAX) as u8) },
        kind => kind,
    }
}

/// The button after `current` in the picks a button menu's item can have, `None` among them.
fn next_button(current: Option<Button>) -> Option<Button> {
    let picks: Vec<Option<Button>> = [None].into_iter().chain(Button::ALL.into_iter().filter(|&b| b != Button::Guide).map(Some)).collect();
    let at = picks.iter().position(|p| *p == current).unwrap_or(0);
    picks[(at + 1) % picks.len()]
}

/// The rows and hint of a menu or info overlay page.
pub(super) fn page(page: Page, config: &Config) -> (Vec<ItemView>, &'static str) {
    match page {
        Page::Menus => menus_page(config),
        Page::Menu(m) => menu_page(config, m),
        Page::MenuItem(m, i) => item_page(config, m, i),
        Page::Infos => infos_page(config),
        Page::Info(i) => info_page(config, i),
        _ => (Vec::new(), "{east} back"),
    }
}

impl LayoutEditor {
    /// Chooses the highlighted row of a menu or info overlay page.
    pub(super) fn choose_on_menu_pages(&mut self, config: &mut Config) -> EditStep {
        match self.page {
            Page::Menus => self.choose_menus_row(config),
            Page::Menu(m) => self.choose_menu_row(m, config),
            Page::MenuItem(m, i) => self.choose_menu_item_row(m, i, config),
            Page::Infos => {
                self.push(Page::Info(self.cursor));
                EditStep::Stay
            }
            Page::Info(i) => self.choose_info_row(i, config),
            _ => EditStep::Stay,
        }
    }

    fn choose_menus_row(&mut self, config: &mut Config) -> EditStep {
        if self.cursor < menus(config).len() {
            self.push(Page::Menu(self.cursor));
            return EditStep::Stay;
        }
        let Some(game) = active_game_mut(config) else { return EditStep::Stay };
        let taken = |n: &str| game.menus.iter().any(|m| m.name == n);
        let name = (1..).map(|n| format!("Menu {n}")).find(|n| !taken(n)).unwrap_or_default();
        game.menus.push(Menu { name, kind: MenuKind::List, items: Vec::new(), cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() });
        let new = game.menus.len() - 1;
        self.push(Page::Menu(new));
        EditStep::Changed
    }

    fn choose_menu_row(&mut self, m: usize, config: &mut Config) -> EditStep {
        let Some(len) = menus(config).get(m).map(|menu| menu.items.len()) else { return EditStep::Stay };
        match self.cursor.checked_sub(len) {
            None => {
                self.push(Page::MenuItem(m, self.cursor));
                EditStep::Stay
            }
            Some(0) => {
                let Some(menu) = menu_mut(config, m) else { return EditStep::Stay };
                menu.items.push(MenuItem::new(NEW_ITEM, ButtonAction::Disabled));
                self.push(Page::MenuItem(m, len));
                EditStep::Changed
            }
            Some(1) => self.adjust_menu(m, 1, config),
            Some(_) if !menu_in_use(config, m) => {
                if let Some(game) = active_game_mut(config) {
                    game.menus.remove(m);
                }
                self.pop();
                EditStep::Changed
            }
            Some(_) => EditStep::Stay,
        }
    }

    /// Left and right on a menu's Kind row change its kind.
    pub(super) fn adjust_menu(&mut self, m: usize, dir: i32, config: &mut Config) -> EditStep {
        let Some(menu) = menu_mut(config, m) else { return EditStep::Stay };
        if self.cursor != menu.items.len() + 1 {
            return EditStep::Stay;
        }
        menu.kind = next_kind(menu, dir);
        EditStep::Changed
    }

    fn choose_menu_item_row(&mut self, m: usize, i: usize, config: &mut Config) -> EditStep {
        let Some(menu) = menus(config).get(m) else { return EditStep::Stay };
        let Some(&row) = item_rows(menu).get(self.cursor) else { return EditStep::Stay };
        let len = menu.items.len();
        let Some(menu) = menu_mut(config, m) else { return EditStep::Stay };
        match row {
            ItemRow::Action => {
                self.push(Page::Action(Slot::MenuItem(m, i)));
                return EditStep::Stay;
            }
            ItemRow::Button => menu.items[i].button = next_button(menu.items[i].button),
            ItemRow::Up | ItemRow::Down => {
                let to = if row == ItemRow::Up { i.checked_sub(1) } else { (i + 1 < len).then_some(i + 1) };
                let Some(to) = to else { return EditStep::Stay };
                menu.items.swap(i, to);
                // Follow the item to where it went.
                self.page = Page::MenuItem(m, to);
                if let Some((_, cursor)) = self.history.last_mut() {
                    *cursor = to;
                }
            }
            ItemRow::Remove => {
                menu.items.remove(i);
                self.pop();
                self.cursor = self.cursor.min(len.saturating_sub(1));
            }
        }
        EditStep::Changed
    }

    /// Whether the cursor is on a menu page's delete row (of the menu, or of an item).
    pub(super) fn on_menu_delete_row(&self, config: &Config) -> bool {
        match self.page {
            Page::Menu(m) => menus(config).get(m).is_some_and(|menu| self.cursor == menu.items.len() + 2 && !menu_in_use(config, m)),
            Page::MenuItem(m, _) => menus(config).get(m).is_some_and(|menu| item_rows(menu).get(self.cursor) == Some(&ItemRow::Remove)),
            _ => false,
        }
    }

    fn choose_info_row(&mut self, i: usize, config: &mut Config) -> EditStep {
        if self.cursor == 0 {
            let Some(o) = info_mut(config, i) else { return EditStep::Stay };
            o.always = !o.always;
            return EditStep::Changed;
        }
        self.adjust_info(i, 1, config)
    }

    /// Left and right on an info overlay's Position row move it round the screen.
    pub(super) fn adjust_info(&mut self, i: usize, dir: i32, config: &mut Config) -> EditStep {
        let Some(o) = info_mut(config, i).filter(|_| self.cursor == 1) else { return EditStep::Stay };
        o.style.position = next_position(o.style.position, dir);
        EditStep::Changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;

    fn config() -> Config {
        let mut config = Config::default();
        config.general.profiles = vec![Profile::passthrough("Gamepad")];
        config
    }

    #[test]
    fn a_menu_is_built_in_game_item_by_item() {
        let mut config = config();
        let mut editor = LayoutEditor::new();
        editor.page = Page::Menus;
        editor.cursor = 0;
        assert_eq!(editor.choose_menus_row(&mut config), EditStep::Changed, "adds a menu");
        assert_eq!(editor.page, Page::Menu(0));
        editor.cursor = 0;
        assert_eq!(editor.choose_menu_row(0, &mut config), EditStep::Changed, "adds an item");
        assert_eq!(editor.page, Page::MenuItem(0, 0));
        set_item_action(&mut config, 0, 0, ButtonAction::Screenshot);
        assert_eq!(menus(&config)[0].items[0].label, ButtonAction::Screenshot.summary(), "named for its action");
        set_item_action(&mut config, 0, 0, ButtonAction::ToggleOverlay);
        assert_eq!(menus(&config)[0].items[0].label, ButtonAction::ToggleOverlay.summary(), "and renamed with it");
        // A second item, moved above the first.
        editor.back();
        editor.cursor = 1;
        editor.choose_menu_row(0, &mut config);
        editor.cursor = item_rows(&menus(&config)[0]).iter().position(|r| *r == ItemRow::Up).unwrap();
        assert_eq!(editor.choose_menu_item_row(0, 1, &mut config), EditStep::Changed);
        assert_eq!(menus(&config)[0].items[0].label, NEW_ITEM);
        assert_eq!(editor.page, Page::MenuItem(0, 0), "following the item");
    }

    #[test]
    fn kinds_cycle_and_skip_what_the_menu_cant_be() {
        let item = MenuItem::new("x", ButtonAction::Disabled);
        let mut menu = Menu { name: "M".into(), kind: MenuKind::Radial { stick: crate::config::Stick::Right, boxes: false }, items: vec![item; 5], cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() };
        assert_eq!(next_kind(&menu, 1).tag(), MenuKindTag::List, "five items don't fit four directions");
        menu.items.truncate(4);
        assert_eq!(next_kind(&menu, 1).tag(), MenuKindTag::Directional);
        assert_eq!(next_kind(&menu, -1).tag(), MenuKindTag::Grid, "and round the other way");
    }

    #[test]
    fn a_menu_a_button_opens_cant_be_deleted_and_info_overlays_move() {
        let mut config = config();
        let mut editor = LayoutEditor::new();
        editor.page = Page::Menus;
        editor.choose_menus_row(&mut config);
        config.general.profiles[0].set_button(Button::Select, ButtonAction::OpenMenu("Menu 1".into()));
        editor.cursor = 2;
        assert!(!editor.on_menu_delete_row(&config), "in use");
        assert_eq!(editor.choose_menu_row(0, &mut config), EditStep::Stay);
        assert_eq!(menus(&config).len(), 1);
        assert_eq!(next_position(ScreenPosition::TopLeft, -1), ScreenPosition::BottomRight);
        assert_eq!(next_button(None), Some(Button::South));
    }
}
