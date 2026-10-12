//! Icons drawn in front of menu items. A set comes with the app (`assets/icons`), and any SVG
//! the user puts in the icons folder (see `assets`) is an icon too, named by its file name
//! without `.svg`. An icon's `currentColor` is drawn in the item's text color.

use crate::assets;

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

/// Every icon's name: the built-in ones, then the user's, without repeats.
pub fn names() -> Vec<String> {
    let mut names: Vec<String> = BUILT_IN.iter().map(|(name, _)| (*name).to_string()).collect();
    for name in user_names() {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The SVG for icon `name` (built in or the user's), drawn in `ink`. `None` when there is no
/// such icon.
pub fn svg(name: &str, [r, g, b]: [u8; 3]) -> Option<String> {
    let body = match BUILT_IN.iter().find(|(known, _)| *known == name) {
        Some((_, body)) => (*body).to_string(),
        None => user_svg(name)?,
    };
    Some(body.replace("currentColor", &format!("#{r:02x}{g:02x}{b:02x}")))
}

/// Names are the file name: letters, digits, `-` and `_`, so one can't reach another folder.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn user_svg(name: &str) -> Option<String> {
    if !valid_name(name) {
        return None;
    }
    std::fs::read_to_string(assets::folder("icons").join(format!("{name}.svg"))).ok()
}

/// The names of the SVG files in the icons folder, sorted.
fn user_names() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(assets::folder("icons")) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("svg")))
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
        .filter(|name| valid_name(name))
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_icons_are_named_and_recolored() {
        assert!(names().contains(&"sword".to_string()));
        let picture = svg("sword", [0x10, 0x20, 0x30]).unwrap();
        assert!(picture.contains("#102030") && !picture.contains("currentColor"));
        assert!(svg("no-such-icon", [0, 0, 0]).is_none());
    }

    #[test]
    fn a_user_icon_name_cannot_leave_its_folder() {
        assert!(valid_name("my-icon_2"));
        assert!(!valid_name("../secret"));
        assert!(!valid_name("a/b"));
        assert!(!valid_name(""));
    }
}
