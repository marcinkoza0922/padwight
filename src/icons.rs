//! Icons drawn in text with `{icon:name}`: a set comes with the app (`assets/icons`), and a pack
//! can carry its own SVGs (see `config::PackAssets`). A pack's icons work only while its game is
//! the one in use: every process that draws text sets them (`set_pack_icons`), as the daemon
//! does when the game changes. An icon's `currentColor` is drawn in the text's color.

use std::{collections::BTreeMap, sync::RwLock};

/// The icons built into the app, by name.
const BUILT_IN: [(&str, &str); 16] = [
    ("axe", include_str!("../assets/icons/axe.svg")),
    ("bow", include_str!("../assets/icons/bow.svg")),
    ("car", include_str!("../assets/icons/car.svg")),
    ("coin", include_str!("../assets/icons/coin.svg")),
    ("gem", include_str!("../assets/icons/gem.svg")),
    ("heart", include_str!("../assets/icons/heart.svg")),
    ("key", include_str!("../assets/icons/key.svg")),
    ("map", include_str!("../assets/icons/map.svg")),
    ("pda", include_str!("../assets/icons/pda.svg")),
    ("pistol", include_str!("../assets/icons/pistol.svg")),
    ("potion", include_str!("../assets/icons/potion.svg")),
    ("rifle", include_str!("../assets/icons/rifle.svg")),
    ("shield", include_str!("../assets/icons/shield.svg")),
    ("skull", include_str!("../assets/icons/skull.svg")),
    ("sword", include_str!("../assets/icons/sword.svg")),
    ("wheel", include_str!("../assets/icons/wheel.svg")),
];

/// The icons of the game in use, by name.
static PACK: RwLock<BTreeMap<String, String>> = RwLock::new(BTreeMap::new());

/// Sets the icons `{icon:…}` can name besides the built-in ones: the active game's own.
pub fn set_pack_icons(icons: &BTreeMap<String, String>) {
    if let Ok(mut pack) = PACK.write()
        && *pack != *icons
    {
        pack.clone_from(icons);
    }
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
    if ["<script", "<image", "<foreignobject", "href=\"http", "href='http", "<!entity"].iter().any(|bad| lower.contains(bad)) {
        return Err("an icon can't hold scripts, pictures or links".into());
    }
    Ok(())
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
}
