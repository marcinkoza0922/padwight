//! What `{guide}`, `{confirm}` and `{back}` stand for. It's the Guide button unless the player moved what opens the Guide
//! overlay (see `config::shortcuts`), and then it's that input: a button's glyph, a combo of
//! glyphs, or a gesture's name and a glyph. Every process that draws text sets it (the daemon, the
//! settings window, and the overlay from each frame it gets), so the token reads the same in
//! hints, menus, info overlays and layer sheets. `{confirm}` and `{back}` are the buttons that
//! confirm and go back in menus and panels (A and B unless moved), which the overlays' hints use.

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use super::Token;
use crate::config::{Button, GestureKind, MenuKeys, Shortcut, ShortcutInput, Shortcuts};

/// The input that opens the Guide overlay, in a form that is `Copy` and travels in a frame. The
/// default is the Guide button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenerLabel {
    /// The buttons; none and not `off` means Guide.
    buttons: [Option<Button>; 4],
    gesture: Option<GestureKind>,
    /// Nothing opens it.
    off: bool,
}

impl OpenerLabel {
    pub fn of(shortcuts: &Shortcuts) -> Self {
        match shortcuts.opener() {
            None => OpenerLabel { off: true, ..Default::default() },
            Some(input) if input == ShortcutInput::button(Button::Guide) => OpenerLabel::default(),
            Some(input) => {
                let mut buttons = [None; 4];
                for (slot, b) in buttons.iter_mut().zip(&input.buttons) {
                    *slot = Some(*b);
                }
                OpenerLabel { buttons, gesture: input.gesture, off: false }
            }
        }
    }

    /// The token and text parts that say it.
    fn parts(self) -> Vec<Result<String, Token>> {
        if self.off {
            return vec![Ok("(no Guide button set)".into())];
        }
        let buttons: Vec<Button> = self.buttons.into_iter().flatten().collect();
        if buttons.is_empty() {
            return vec![Err(Token::Button(Button::Guide))];
        }
        let mut parts = Vec::new();
        if let Some(kind) = self.gesture {
            parts.push(Ok(format!("{} ", kind.to_string().to_lowercase())));
        }
        for (i, b) in buttons.into_iter().enumerate() {
            if i > 0 {
                parts.push(Ok(" + ".into()));
            }
            parts.push(Err(Token::Button(b)));
        }
        parts
    }

    /// `parts` with every `{guide}` in them said as this.
    pub(super) fn expand(self, parts: Vec<Result<String, Token>>) -> Vec<Result<String, Token>> {
        if self == OpenerLabel::default() {
            return parts;
        }
        parts
            .into_iter()
            .flat_map(|part| match part {
                Err(Token::Button(Button::Guide)) => self.parts(),
                other => vec![other],
            })
            .collect()
    }
}

static OPENER: RwLock<OpenerLabel> = RwLock::new(OpenerLabel { buttons: [None; 4], gesture: None, off: false });

/// Sets what `{guide}` says, for this process.
pub fn set_opener(label: OpenerLabel) {
    if let Ok(mut now) = OPENER.write() {
        *now = label;
    }
}

pub(super) fn opener() -> OpenerLabel {
    OPENER.read().map(|l| *l).unwrap_or_default()
}

static MENU_KEYS: RwLock<MenuKeys> = RwLock::new(MenuKeys { confirm: Button::South, back: Button::East });

/// Sets what `{confirm}` and `{back}` say, for this process.
pub fn set_menu_keys(keys: MenuKeys) {
    if let Ok(mut now) = MENU_KEYS.write() {
        *now = keys;
    }
}

/// The button a `{confirm}` or `{back}` token stands for.
pub(super) fn menu_key(name: &str) -> Option<Button> {
    let keys = MENU_KEYS.read().map(|k| *k).unwrap_or_default();
    match name {
        "confirm" => Some(keys.confirm),
        "back" => Some(keys.back),
        _ => None,
    }
}

static PANEL_KEYS: RwLock<Option<Shortcuts>> = RwLock::new(None);

/// Sets which buttons the media controls' and numpad's keys sit on, for this process.
pub fn set_panel_keys(shortcuts: &Shortcuts) {
    if let Ok(mut now) = PANEL_KEYS.write() {
        *now = Some(shortcuts.clone());
    }
}

/// The button a `{play}`, `{next}`, `{previous}`, `{player}`, `{mute}`, `{media_close}`,
/// `{backspace}`, `{enter}` or `{numpad_close}` token stands for.
pub(super) fn panel_key(name: &str) -> Option<Button> {
    let key = match name {
        "play" => Shortcut::MediaPlay,
        "next" => Shortcut::MediaNext,
        "previous" => Shortcut::MediaPrevious,
        "player" => Shortcut::MediaPlayer,
        "mute" => Shortcut::MediaMute,
        "backspace" => Shortcut::NumpadBackspace,
        "enter" => Shortcut::NumpadEnter,
        "media_close" => Shortcut::MediaClose,
        "numpad_close" => Shortcut::NumpadClose,
        "keyboard_backspace" => Shortcut::KeyboardBackspace,
        "keyboard_space" => Shortcut::KeyboardSpace,
        "keyboard_enter" => Shortcut::KeyboardEnter,
        "keyboard_left" => Shortcut::KeyboardLeft,
        "keyboard_right" => Shortcut::KeyboardRight,
        "keyboard_close" => Shortcut::KeyboardClose,
        _ => return None,
    };
    let fallback = key.default_input().buttons[0];
    Some(PANEL_KEYS.read().ok().and_then(|keys| keys.as_ref().map(|s| s.button(key))).unwrap_or(fallback))
}
