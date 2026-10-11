//! The Guide overlay's center menu: Quick Settings, Edit Controls, the keyboard and numpad, mouse
//! mode, media controls, the next profile and, while Steam runs, the Steam overlay. The daemon keeps the
//! cursor and runs the chosen item; the engine delivers the presses.

use crate::{
    config::OverlayStyle,
    menu::{ItemView, MenuView},
};

/// One row of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    QuickSettings,
    /// Opens the Edit Controls editor directly, not through Quick Settings.
    EditControls,
    Keyboard,
    Numpad,
    MouseMode,
    Media,
    NextProfile,
    /// Sends the real Guide button to the virtual pad, which opens Steam's overlay.
    Steam,
    /// One of the menu items picked lately, by its place in the recent list (oldest first).
    Recent(usize),
}

/// A menu item picked lately, which the Guide menu offers again.
#[derive(Debug, Clone, PartialEq)]
pub struct Recent {
    pub menu: String,
    pub item: usize,
    pub label: String,
    pub action: crate::config::ButtonAction,
}

/// How many recent picks the menu offers.
pub const RECENT: usize = 3;

/// Adds a pick to the recent ones, newest last: a pick already there moves to the end, and the
/// oldest goes once there are more than [`RECENT`].
pub fn remember(recent: &mut Vec<Recent>, pick: Recent) {
    recent.retain(|r| !(r.menu == pick.menu && r.item == pick.item));
    recent.push(pick);
    if recent.len() > RECENT {
        recent.remove(0);
    }
}

/// The menu's rows, then the `recent` picks (newest last, so Up from the top reaches it). The
/// Steam overlay is only there while Steam runs.
pub fn items(steam: bool, recent: usize) -> Vec<Item> {
    let mut items = vec![
        Item::QuickSettings,
        Item::EditControls,
        Item::Keyboard,
        Item::Numpad,
        Item::MouseMode,
        Item::Media,
        Item::NextProfile,
    ];
    if steam {
        items.push(Item::Steam);
    }
    items.extend((0..recent).map(Item::Recent));
    items
}

fn label(item: Item, profile: &str, recent: &[Recent]) -> String {
    match item {
        Item::Recent(i) => format!("Again: {}", recent.get(i).map_or("", |r| r.label.as_str())),
        Item::QuickSettings => "Quick Settings".into(),
        Item::EditControls => "Edit Controls".into(),
        Item::Keyboard => "On-screen keyboard".into(),
        Item::Numpad => "On-screen numpad".into(),
        Item::MouseMode => "Mouse mode".into(),
        Item::Media => "Media controls".into(),
        Item::NextProfile => format!("Next profile (now {profile})"),
        Item::Steam => "Steam overlay".into(),
    }
}

/// The menu as the overlay draws it, with `cursor` on one of its rows.
pub fn view(steam: bool, cursor: usize, profile: &str, style: OverlayStyle, recent: &[Recent]) -> MenuView {
    let items = items(steam, recent.len())
        .into_iter()
        .map(|item| ItemView {
            label: label(item, profile, recent),
            submenu: item == Item::QuickSettings,
            keyword: match item {
                Item::Recent(i) => recent.get(i).and_then(|r| r.action.keyword()),
                _ => None,
            },
            ..ItemView::plain()
        })
        .collect();
    MenuView {
        title: "Guide".into(),
        kind: crate::config::MenuKind::List,
        items,
        selected: Some(cursor),
        picks: 0,
        crumbs: Vec::new(),
        hint: "{south} choose · {east} close".into(),
        style,
        touch: false,
        aim: None,
    }
}

/// Whether Steam is running: some process is named `steam`.
pub fn steam_running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else { return false };
    dir.flatten().any(|entry| {
        std::fs::read_to_string(entry.path().join("comm")).is_ok_and(|name| name.trim() == "steam")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steam_overlay_is_listed_only_while_steam_runs() {
        assert!(!items(false, 0).contains(&Item::Steam));
        assert_eq!(items(true, 0).last(), Some(&Item::Steam));
    }

    #[test]
    fn quick_settings_leads_the_menu_and_edit_controls_follows_it() {
        assert_eq!(&items(false, 0)[..2], &[Item::QuickSettings, Item::EditControls]);
    }

    #[test]
    fn the_view_names_the_profile_and_marks_the_cursor() {
        let menu = view(false, 1, "Gameplay", OverlayStyle::default(), &[]);
        assert_eq!(menu.selected, Some(1));
        assert_eq!(menu.items.last().unwrap().label, "Next profile (now Gameplay)");
        assert_eq!(menu.items.len(), items(false, 0).len());
    }

    #[test]
    fn recent_picks_come_last_newest_at_the_end_and_dont_repeat() {
        let pick = |item: usize| Recent { menu: "Belt".into(), item, label: format!("Item {item}"), action: crate::config::ButtonAction::Screenshot };
        let mut recent = Vec::new();
        for i in [0, 1, 2, 0, 3] {
            remember(&mut recent, pick(i));
        }
        assert_eq!(recent.iter().map(|r| r.item).collect::<Vec<_>>(), [2, 0, 3], "0 moved to the end, 1 fell off");
        let menu = view(false, 0, "P", OverlayStyle::default(), &recent);
        assert_eq!(menu.items.last().unwrap().label, "Again: Item 3");
        assert_eq!(items(false, 3).last(), Some(&Item::Recent(2)));
    }
}
