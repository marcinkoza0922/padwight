//! Icons drawn in text with `{icon:name}`: a set comes with the app (`assets/icons`), and a pack
//! can carry its own SVGs (see `config::PackAssets`). A pack's icons work only while its game is
//! the one in use: every process that draws text sets them (`set_pack_icons`), as the daemon
//! does when the game changes. An icon's `currentColor` is drawn in the text's color.

use std::{
    collections::BTreeMap,
    sync::{Mutex, RwLock},
};

/// The icons built into the app (`assets/icons`, embedded by `build.rs`), by name, sorted.
mod built_in {
    include!(concat!(env!("OUT_DIR"), "/icons.rs"));
}
use built_in::BUILT_IN;

/// The icons of the game in use, by name.
static PACK: RwLock<BTreeMap<String, String>> = RwLock::new(BTreeMap::new());

/// What [`PACK`] was last set from, so setting it again (every overlay frame does) is free.
static PACK_SOURCE: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

/// Sets the icons `{icon:…}` can name besides the built-in ones: the active game's own. One that
/// fails [`check_svg`] is left out: a pack is checked when imported, but a game file can be
/// older than a check, or edited by hand.
pub fn set_pack_icons(icons: &BTreeMap<String, String>) {
    let mut source = PACK_SOURCE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if source.as_ref() == Some(icons) {
        return;
    }
    if let Ok(mut pack) = PACK.write() {
        *pack = icons.iter().filter(|(_, svg)| check_svg(svg).is_ok()).map(|(name, svg)| (name.clone(), svg.clone())).collect();
    }
    *source = Some(icons.clone());
}

/// The names of the built-in icons.
pub fn built_in_names() -> Vec<String> {
    BUILT_IN.iter().map(|(name, _)| (*name).to_string()).collect()
}

/// The names of the icons the game in use carries, sorted.
pub fn pack_names() -> Vec<String> {
    PACK.read().map(|pack| pack.keys().cloned().collect()).unwrap_or_default()
}

/// The SVG for icon `name` (the game's own first, then built in), drawn in `ink`. `None` when
/// there is no such icon.
pub fn svg(name: &str, [r, g, b]: [u8; 3]) -> Option<String> {
    let own = PACK.read().ok().and_then(|pack| pack.get(name).cloned());
    let body = match own {
        Some(body) => body,
        None => BUILT_IN.iter().find(|(known, _)| *known == name).map(|(_, body)| (*body).to_string())?,
    };
    Some(body.replace("currentColor", &format!("#{r:02x}{g:02x}{b:02x}")))
}

/// Names are letters, digits, `-` and `_`.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Whether `svg` is something to draw: SVG markup, small enough to carry in a pack, with nothing
/// in it that reaches outside the file.
pub fn check_svg(svg: &str) -> Result<(), String> {
    const MAX: usize = 64 * 1024;
    if svg.len() > MAX {
        return Err(format!("an icon is at most {} KB", MAX / 1024));
    }
    let lower = svg.to_lowercase();
    if !lower.contains("<svg") {
        return Err("not an SVG".into());
    }
    if ["<script", "<image", "<foreignobject", "<!entity"].iter().any(|bad| lower.contains(bad)) || !links_stay_inside(&lower) {
        return Err("an icon can't hold scripts, pictures or links".into());
    }
    Ok(())
}

/// Whether every `href` in `svg` names a part of the file itself (`#id`). The renderer reads
/// any other as a file from disk, whole: `<feImage href="/dev/zero">` would never finish.
fn links_stay_inside(svg: &str) -> bool {
    svg.match_indices("href").all(|(at, _)| {
        let Some(value) = svg[at + 4..].trim_start().strip_prefix('=') else { return true };
        let value = value.trim_start();
        value.strip_prefix(['"', '\'']).unwrap_or(value).starts_with('#')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_icons_are_named_and_recolored() {
        assert!(built_in_names().contains(&"sword".to_string()));
        let picture = svg("sword", [0x10, 0x20, 0x30]).unwrap();
        assert!(picture.contains("#102030") && !picture.contains("currentColor"));
        assert!(svg("no-such-icon", [0, 0, 0]).is_none());
    }

    #[test]
    fn every_built_in_icon_is_clean_and_tinted() {
        assert!(BUILT_IN.len() > 100, "build.rs embeds the whole assets/icons folder");
        for (name, body) in BUILT_IN {
            assert!(valid_name(name), "{name}");
            check_svg(body).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(body.contains("currentColor"), "{name} takes the text's color");
        }
    }

    #[test]
    fn an_icon_name_is_a_plain_word() {
        assert!(valid_name("my-icon_2"));
        assert!(!valid_name("../secret"));
        assert!(!valid_name("a/b"));
        assert!(!valid_name(""));
    }

    #[test]
    fn only_clean_svg_is_accepted() {
        assert!(check_svg("<svg viewBox='0 0 8 8'><path d='M0 0h8v8z' fill='currentColor'/></svg>").is_ok());
        assert!(check_svg("<html></html>").is_err());
        assert!(check_svg("<svg><script>alert(1)</script></svg>").is_err());
        assert!(check_svg("<svg><image href='http://x/y.png'/></svg>").is_err());
    }

    #[test]
    fn an_icon_can_t_read_a_file_from_disk() {
        for link in ["href=\"/dev/zero\"", "xlink:href='/dev/zero'", "href = \"pic.png\"", "href=\" #a\""] {
            let svg = format!("<svg><filter id='f'><feImage {link}/></filter><rect filter='url(#f)'/></svg>");
            assert!(check_svg(&svg).is_err(), "{link}");
        }
        assert!(check_svg("<svg><defs><path id='a' d='M0 0h8'/></defs><use href='#a'/><use xlink:href=\"#a\"/></svg>").is_ok());
    }

    #[test]
    fn a_game_s_icon_that_fails_the_check_is_never_drawn() {
        let good = "<svg viewBox='0 0 8 8'><path d='M0 0h8v8z' fill='currentColor'/></svg>";
        let bad = "<svg><filter id='f'><feImage href='/dev/zero'/></filter><rect filter='url(#f)'/></svg>";
        set_pack_icons(&BTreeMap::from([("good".to_string(), good.to_string()), ("bad".to_string(), bad.to_string())]));
        assert_eq!(pack_names(), ["good"]);
        assert!(svg("bad", [0, 0, 0]).is_none());
    }
}
