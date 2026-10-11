//! Key labels in the user's keyboard layout: what each key types, read from XKB, so the on-screen
//! keyboard shows "z" on the key a German layout types z with, and "!" over "1" with Shift on.
//!
//! The layout is the first one the desktop is set to use (KDE, GNOME, Sway, Hyprland, the X11 or
//! console settings), else the one the locale suggests, else US. `PADWIGHT_KEYBOARD_LAYOUT`
//! (`de`, `fr(azerty)`) overrides it.

use std::{collections::HashMap, str::FromStr, sync::OnceLock};

use xkbcommon::xkb;

use super::{Board, Key};

/// What a key types, unshifted and shifted.
#[derive(Debug, Clone, PartialEq)]
struct Typed {
    plain: String,
    shifted: String,
}

static LABELS: OnceLock<HashMap<&'static str, Typed>> = OnceLock::new();

/// The label on `key`'s cap: what it types in the user's layout (shifted with `shift`, and letters
/// in capitals with `caps`), or its name for a key that types no character. The numpad keeps its
/// own labels, since it always types digits.
pub fn cap_label(key: &Key, board: Board, shift: bool, caps: bool) -> String {
    if board == Board::Numpad {
        return key.label.to_string();
    }
    match labels().get(key.code) {
        Some(typed) => pick(typed, shift, caps).to_string(),
        None => key.label.to_string(),
    }
}

/// Whether `code` needs Shift to type `text` in the user's layout (AZERTY's digits do); `None`
/// when it doesn't type it either way, or the layout is unknown.
pub fn needs_shift(code: &str, text: &str) -> Option<bool> {
    let typed = labels().get(code)?;
    if typed.plain == text {
        Some(false)
    } else {
        (typed.shifted == text).then_some(true)
    }
}

/// The label a key shows: Caps Lock swaps a letter's case, as Shift does.
fn pick(typed: &Typed, shift: bool, caps: bool) -> &str {
    let letter = typed.plain.chars().all(char::is_alphabetic) && typed.plain != typed.shifted;
    if shift != (caps && letter) { &typed.shifted } else { &typed.plain }
}

fn labels() -> &'static HashMap<&'static str, Typed> {
    LABELS.get_or_init(|| {
        let (layout, variant) = detect();
        compile(&layout, &variant).or_else(|| compile("us", "")).unwrap_or_default()
    })
}

/// What every key of the keyboards types in `layout` (and `variant`), or `None` if XKB doesn't
/// know the layout.
fn compile(layout: &str, variant: &str) -> Option<HashMap<&'static str, Typed>> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_names(&context, "", "", layout, variant, None, xkb::KEYMAP_COMPILE_NO_FLAGS)?;
    let mut typed = HashMap::new();
    for key in [Board::Basic, Board::Full].into_iter().flat_map(Board::keys) {
        let Ok(code) = evdev::KeyCode::from_str(key.code) else { continue };
        // XKB numbers keys 8 above the kernel.
        let keycode = xkb::Keycode::new(u32::from(code.code()) + 8);
        let at = |level| keymap.key_get_syms_by_level(keycode, 0, level).first().and_then(|&sym| printable(sym));
        let Some(plain) = at(0) else { continue };
        let shifted = at(1).unwrap_or_else(|| plain.clone());
        typed.insert(key.code, Typed { plain, shifted });
    }
    Some(typed)
}

/// The character a keysym types, as a label: dead keys show their accent, and keys that type
/// nothing visible (space, Enter, Tab) have none.
fn printable(sym: xkb::Keysym) -> Option<String> {
    let dead = match xkb::keysym_get_name(sym).as_str() {
        "dead_grave" => Some("`"),
        "dead_acute" => Some("´"),
        "dead_circumflex" => Some("^"),
        "dead_tilde" => Some("~"),
        "dead_diaeresis" => Some("¨"),
        "dead_cedilla" => Some("¸"),
        "dead_abovering" => Some("°"),
        "dead_caron" => Some("ˇ"),
        "dead_ogonek" => Some("˛"),
        "dead_doubleacute" => Some("˝"),
        "dead_abovedot" => Some("˙"),
        "dead_macron" => Some("¯"),
        "dead_breve" => Some("˘"),
        _ => None,
    };
    if let Some(accent) = dead {
        return Some(accent.into());
    }
    let text = xkb::keysym_to_utf8(sym);
    (!text.is_empty() && !text.chars().any(|c| c.is_control() || c.is_whitespace())).then_some(text)
}

/// The user's layout and variant, from the first place that names one.
fn detect() -> (String, String) {
    let found = std::env::var("PADWIGHT_KEYBOARD_LAYOUT")
        .ok()
        .and_then(|v| parse_spec(&v))
        .or_else(from_xkb_env)
        .or_else(from_kde)
        .or_else(from_gnome)
        .or_else(from_wm_configs)
        .or_else(from_system)
        .or_else(from_locale);
    found.unwrap_or_else(|| ("us".into(), String::new()))
}

/// A layout written `de`, `de(nodeadkeys)` or `de+nodeadkeys`, or the first of a list (`us,de`).
fn parse_spec(spec: &str) -> Option<(String, String)> {
    let first = spec.split(',').next()?.trim();
    let (layout, variant) = match first.split_once(['(', '+']) {
        Some((layout, rest)) => (layout, rest.trim_end_matches(')')),
        None => (first, ""),
    };
    (!layout.is_empty()).then(|| (layout.to_string(), variant.to_string()))
}

/// The first of a comma-separated layout list, with the first of its variant list.
fn first_of(layouts: &str, variants: &str) -> Option<(String, String)> {
    let layout = layouts.split(',').next()?.trim();
    let variant = variants.split(',').next().unwrap_or("").trim();
    (!layout.is_empty()).then(|| (layout.to_string(), variant.to_string()))
}

fn from_xkb_env() -> Option<(String, String)> {
    let layouts = std::env::var("XKB_DEFAULT_LAYOUT").ok()?;
    first_of(&layouts, &std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default())
}

/// The value of `key` in a `key=value` (or `key value`, `key = value`) settings text.
fn setting<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix(key)?;
        let value = rest.trim_start().strip_prefix('=').unwrap_or(rest).trim();
        (!value.is_empty() && rest.starts_with([' ', '=', '\t'])).then(|| value.trim_matches('"'))
    })
}

fn config_file(path: &str) -> Option<String> {
    std::fs::read_to_string(dirs::config_dir()?.join(path)).ok()
}

fn from_kde() -> Option<(String, String)> {
    let text = config_file("kxkbrc")?;
    first_of(setting(&text, "LayoutList")?, setting(&text, "VariantList").unwrap_or(""))
}

fn from_gnome() -> Option<(String, String)> {
    let out = std::process::Command::new("gsettings").args(["get", "org.gnome.desktop.input-sources", "sources"]).output().ok()?;
    parse_gnome_sources(&String::from_utf8_lossy(&out.stdout))
}

/// The first XKB source of GNOME's `[('xkb', 'de+nodeadkeys'), ('ibus', …)]`.
fn parse_gnome_sources(text: &str) -> Option<(String, String)> {
    let at = text.find("('xkb', '")? + "('xkb', '".len();
    let spec = &text[at..at + text[at..].find('\'')?];
    parse_spec(spec)
}

/// Sway's `xkb_layout` and Hyprland's `kb_layout`.
fn from_wm_configs() -> Option<(String, String)> {
    let sway = config_file("sway/config").and_then(|t| Some((setting_anywhere(&t, "xkb_layout")?, setting_anywhere(&t, "xkb_variant"))));
    let hypr = || config_file("hypr/hyprland.conf").and_then(|t| Some((setting_anywhere(&t, "kb_layout")?, setting_anywhere(&t, "kb_variant"))));
    let (layouts, variants) = sway.or_else(hypr)?;
    first_of(&layouts, &variants.unwrap_or_default())
}

/// A setting found on any line, indented or inside a block (`input * { xkb_layout de }`).
fn setting_anywhere(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let words = line.trim().trim_end_matches(['}', ';']);
        let at = words.find(key)?;
        setting(&words[at..], key).map(|v| v.trim_matches(['"', '\'']).to_string())
    })
}

/// The X11 settings `localectl` writes, then the console's.
fn from_system() -> Option<(String, String)> {
    let xorg = std::fs::read_to_string("/etc/X11/xorg.conf.d/00-keyboard.conf").ok().and_then(|text| {
        let option = |name: &str| {
            text.lines().find_map(|l| l.trim().strip_prefix("Option")?.trim().strip_prefix(&format!("\"{name}\""))?.trim().strip_prefix('"')?.split('"').next().map(str::to_string))
        };
        first_of(&option("XkbLayout")?, &option("XkbVariant").unwrap_or_default())
    });
    xorg.or_else(|| {
        let text = std::fs::read_to_string("/etc/vconsole.conf").ok()?;
        first_of(setting(&text, "XKBLAYOUT")?, setting(&text, "XKBVARIANT").unwrap_or(""))
    })
}

/// The layout the locale's country uses (`de_DE` is `de`, `en_GB` is `gb`); English elsewhere
/// is US.
fn from_locale() -> Option<(String, String)> {
    let locale = ["LC_ALL", "LC_CTYPE", "LANG"].iter().find_map(|v| std::env::var(v).ok().filter(|l| !l.is_empty()))?;
    layout_for_locale(&locale).map(|l| (l, String::new()))
}

fn layout_for_locale(locale: &str) -> Option<String> {
    let name = locale.split(['.', '@']).next()?;
    let (language, country) = name.split_once('_')?;
    match (language, country) {
        ("en", "GB") => Some("gb".into()),
        ("en", _) | ("C", _) => Some("us".into()),
        (_, country) => Some(country.to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: &'static str) -> &'static Key {
        Board::Basic.keys().find(|k| k.code == code).unwrap()
    }

    #[test]
    fn a_layout_labels_the_keys_with_what_they_type() {
        // Needs XKB's data, which any desktop with a keyboard has.
        let Some(de) = compile("de", "") else { return };
        assert_eq!(de["KEY_Y"], Typed { plain: "z".into(), shifted: "Z".into() });
        assert_eq!(de["KEY_1"].shifted, "!");
        assert!(!de.contains_key("KEY_SPACE"), "space types nothing to draw");
        assert!(!de.contains_key("KEY_ENTER"));
        let us = compile("us", "").unwrap();
        assert_eq!(us["KEY_2"].shifted, "@");
        if let Some(fr) = compile("fr", "") {
            assert_eq!(fr["KEY_1"].shifted, "1", "AZERTY types digits with Shift");
        }
    }

    #[test]
    fn shift_and_caps_lock_pick_the_case() {
        let letter = Typed { plain: "q".into(), shifted: "Q".into() };
        let digit = Typed { plain: "1".into(), shifted: "!".into() };
        assert_eq!(pick(&letter, false, false), "q");
        assert_eq!(pick(&letter, true, false), "Q");
        assert_eq!(pick(&letter, false, true), "Q");
        assert_eq!(pick(&letter, true, true), "q", "Shift with Caps Lock types small letters");
        assert_eq!(pick(&digit, false, true), "1", "Caps Lock leaves digits alone");
        assert_eq!(pick(&digit, true, false), "!");
    }

    #[test]
    fn named_keys_and_the_numpad_keep_their_labels() {
        assert_eq!(cap_label(key("KEY_ENTER"), Board::Basic, false, false), "Enter");
        let numpad_one = Board::Numpad.keys().find(|k| k.code == "KEY_1").unwrap();
        assert_eq!(cap_label(numpad_one, Board::Numpad, true, false), "1");
    }

    #[test]
    fn layouts_are_read_from_the_settings_people_write() {
        assert_eq!(parse_spec("de(nodeadkeys)"), Some(("de".into(), "nodeadkeys".into())));
        assert_eq!(parse_spec("fr+azerty"), Some(("fr".into(), "azerty".into())));
        assert_eq!(parse_spec("us,de"), Some(("us".into(), String::new())));
        assert_eq!(first_of("pl,us", ",dvorak"), Some(("pl".into(), String::new())));
        assert_eq!(parse_gnome_sources("[('xkb', 'de+nodeadkeys'), ('xkb', 'us')]"), Some(("de".into(), "nodeadkeys".into())));
        assert_eq!(parse_gnome_sources("@a(ss) []"), None);
        assert_eq!(setting("[Layout]\nLayoutList=us,de\nUse=true", "LayoutList"), Some("us,de"));
        assert_eq!(setting("XKBLAYOUT=\"de\"", "XKBLAYOUT"), Some("de"));
        assert_eq!(setting_anywhere("input * {\n    xkb_layout de,us\n}", "xkb_layout").as_deref(), Some("de,us"));
        assert_eq!(setting_anywhere("input {\n  kb_layout = fr\n}", "kb_layout").as_deref(), Some("fr"));
        assert_eq!(layout_for_locale("de_DE.UTF-8").as_deref(), Some("de"));
        assert_eq!(layout_for_locale("en_GB.UTF-8").as_deref(), Some("gb"));
        assert_eq!(layout_for_locale("en_AU.UTF-8").as_deref(), Some("us"));
        assert_eq!(layout_for_locale("pt_BR.UTF-8").as_deref(), Some("br"));
    }
}
