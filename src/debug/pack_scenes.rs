//! A pack's own overlays as frames, for `padwight debug overlay <file>.padpack`: each of its
//! menus open with the cursor on the second item, its keyboard and numpad, and each of its info
//! overlays, drawn in its font, styles and icons, to check them without a daemon or the game.

use std::{collections::HashSet, path::Path};

use anyhow::{Context, Result};

use crate::{
    config::OverlayStyle,
    engine::Opener,
    info::{self, Live, PadFamily},
    keyboard::{self, Board},
    menu::MenuSession,
    overlay::{KeyboardView, OverlayFrame, OverlayView},
};

/// Every frame the pack at `path` draws, named `<file>-<what>`.
pub fn frames(path: &Path) -> Result<Vec<(String, OverlayFrame)>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let game = crate::pack::parse(&text).with_context(|| format!("{} is not a pack", path.display()))?.to_game();
    crate::font::set_pack_fonts(&game.assets.fonts);
    let stem = path.file_stem().map_or("pack".into(), |s| s.to_string_lossy().into_owned());
    let mut taken = HashSet::new();
    let base = OverlayFrame { font: game.overlay_font.clone(), pack_icons: Some(game.assets.icons.clone()), ..OverlayFrame::default() };
    let mut out = Vec::new();
    for menu in &game.menus {
        let opener = Opener { buttons: vec![crate::config::Button::LeftBumper], ..Opener::default() };
        let Some(session) = MenuSession::open(&game.menus, &menu.name, opener) else { continue };
        let Some(mut view) = session.view(&game.menus) else { continue };
        view.selected = Some(1.min(menu.items.len().saturating_sub(1)));
        out.push((unique(&mut taken, &format!("{stem}-menu"), &menu.name), OverlayFrame { active: Some(OverlayView::Menu(view)), ..base.clone() }));
    }
    for (board, style) in [(Board::Basic, game.keyboard_style.clone().unwrap_or_else(OverlayStyle::keyboard)), (Board::Numpad, game.numpad_style.clone().unwrap_or_else(OverlayStyle::numpad))] {
        let cursor = if board == Board::Numpad { board.home() } else { keyboard::find(board, "KEY_H").unwrap_or_default() };
        let view = KeyboardView { board, style, cursor, latched: Vec::new(), pressed: None, closing: 0.0, caps: false, typed: String::new(), caret: 0, field: None };
        let name = if board == Board::Numpad { "numpad" } else { "keyboard" };
        out.push((format!("{stem}-{name}"), OverlayFrame { active: Some(OverlayView::Keyboard(view)), ..base.clone() }));
    }
    let live = Live::sample(PadFamily::Xbox);
    for overlay in &game.info {
        out.push((unique(&mut taken, &format!("{stem}-info"), &overlay.name), OverlayFrame { info: vec![info::resolve(overlay, &live)], ..base.clone() }));
    }
    Ok(out)
}

/// `<prefix>-<name>` with `name` cut down to lowercase words joined by `-`, numbered `-2`, `-3`…
/// when an earlier name gave the same file name, so no frame overwrites another.
fn unique(taken: &mut HashSet<String>, prefix: &str, name: &str) -> String {
    let words: Vec<String> = name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_owned).collect();
    let base = if words.is_empty() { prefix.to_owned() } else { format!("{prefix}-{}", words.join("-")) };
    let mut file = base.clone();
    for n in 2.. {
        if taken.insert(file.clone()) {
            break;
        }
        file = format!("{base}-{n}");
    }
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_pack_draws_its_menus_keyboard_and_numpad() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("packs/thief-2.padpack");
        let names: Vec<String> = frames(&path).unwrap().into_iter().map(|(name, _)| name).collect();
        assert!(names.contains(&"thief-2-menu-weapons".to_string()), "{names:?}");
        assert!(names.contains(&"thief-2-keyboard".to_string()) && names.contains(&"thief-2-numpad".to_string()));
    }

    #[test]
    fn names_that_give_the_same_file_name_are_numbered() {
        let mut taken = HashSet::new();
        let names: Vec<String> = ["Items", "items!", "Items 2", "", "?!"].iter().map(|name| unique(&mut taken, "p-menu", name)).collect();
        assert_eq!(names, ["p-menu-items", "p-menu-items-2", "p-menu-items-2-2", "p-menu", "p-menu-2"]);
    }
}
