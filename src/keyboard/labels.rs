//! Key labels in the user's keyboard layout: what each key types, so the on-screen keyboard
//! shows "z" on the key a German layout types z with, and "!" over "1" with Shift on. The layout
//! comes from XKB (`xkb.rs`).

use std::{collections::HashMap, sync::OnceLock};

use super::{Board, Key};

/// What a key types, unshifted and shifted.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Typed {
    pub plain: String,
    pub shifted: String,
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

/// The key of the basic keyboard that types `c` in the user's layout, and whether Shift has to be
/// held for it. `None` when no key there types it.
pub fn key_for(c: char) -> Option<(&'static str, bool)> {
    let text = c.to_string();
    super::Board::Basic.rows().iter().flat_map(|row| row.iter()).filter(|key| !key.code.is_empty()).find_map(|key| {
        [false, true].into_iter().find(|&shift| typed(key.code, shift, false).as_deref() == Some(text.as_str())).map(|shift| (key.code, shift))
    })
}

/// What `code` types in the user's layout, with `shift` and `caps`; `None` for a key that types
/// no character.
pub fn typed(code: &str, shift: bool, caps: bool) -> Option<String> {
    labels().get(code).map(|t| pick(t, shift, caps).to_string())
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
    LABELS.get_or_init(super::layout::labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: &'static str) -> &'static Key {
        Board::Basic.keys().find(|k| k.code == code).unwrap()
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

}
