//! The Guide overlay's center menu: Quick Settings, Edit Controls, the keyboard and numpad, mouse
//! mode, media controls, Switch Profile and, while Steam runs, the Steam overlay. The daemon keeps the
//! cursor and runs the chosen item; the engine delivers the presses. The rows fit one column.

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
    /// Opens Switch Profile: the game's profiles, and a new one.
    SwitchProfile,
    /// Sends the real Guide button to the virtual pad, which opens Steam's overlay.
    Steam,
}

/// The menu's rows. The Steam overlay is only there while Steam runs.
pub fn items(steam: bool) -> Vec<Item> {
    let mut items = vec![
        Item::QuickSettings,
        Item::EditControls,
        Item::Keyboard,
        Item::Numpad,
        Item::MouseMode,
        Item::Media,
        Item::SwitchProfile,
    ];
    if steam {
        items.push(Item::Steam);
    }
    items
}

fn label(item: Item, profile: &str) -> String {
    match item {
        Item::QuickSettings => "Quick Settings".into(),
        Item::EditControls => "Edit Controls".into(),
        Item::Keyboard => "On-screen keyboard".into(),
        Item::Numpad => "On-screen numpad".into(),
        Item::MouseMode => "Mouse mode".into(),
        Item::Media => "Media controls".into(),
        Item::SwitchProfile => format!("Switch Profile (now {profile})"),
        Item::Steam => "Steam overlay".into(),
    }
}

/// The menu as the overlay draws it, with `cursor` on one of its rows.
pub fn view(steam: bool, cursor: usize, profile: &str, style: OverlayStyle) -> MenuView {
    let items = items(steam)
        .into_iter()
        .map(|item| ItemView {
            label: label(item, profile),
            submenu: matches!(item, Item::QuickSettings | Item::SwitchProfile),
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
        hint: "{confirm} choose · {back} close".into(),
        style,
        touch: false,
        aim: None,
        prompt: None,
        one_column: false,
        figure: None,
    }
}

/// Whether Steam is running: some process is `steam.exe`.
#[cfg(windows)]
pub fn steam_running() -> bool {
    crate::focus::running_processes().iter().any(|p| p.exe.eq_ignore_ascii_case("steam.exe"))
}

/// Whether Steam is running: some process is named `steam`.
#[cfg(target_os = "linux")]
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
        assert!(!items(false).contains(&Item::Steam));
        assert_eq!(items(true).last(), Some(&Item::Steam));
    }

    #[test]
    fn quick_settings_leads_the_menu_and_edit_controls_follows_it() {
        assert_eq!(&items(false)[..2], &[Item::QuickSettings, Item::EditControls]);
    }

    #[test]
    fn the_menu_fits_one_column_even_with_steam() {
        assert_eq!(crate::menu::list_columns(items(true).len()), 1);
    }

    #[test]
    fn the_view_names_the_profile_and_marks_the_cursor() {
        let menu = view(false, 1, "Gameplay", OverlayStyle::default());
        assert_eq!(menu.selected, Some(1));
        assert_eq!(menu.items.last().unwrap().label, "Switch Profile (now Gameplay)");
        assert_eq!(menu.items.len(), items(false).len());
    }
}
