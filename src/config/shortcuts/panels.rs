//! The buttons the media controls and the on-screen numpad use for their own actions: play or
//! pause, the tracks, the player, mute, backspace and enter. They're app-wide like the rest (see
//! `super`), and they move like the menu buttons: a button another key of the same panel has
//! trades places with the key's old one, so each key keeps one button and none is shared. A button
//! no key of the panel holds is simply taken, and the button the key left does nothing there.
//!
//! The panels' overlay code still reads the buttons they were written with. A press of a key's
//! button is handed over as that button, and the menu buttons aren't applied to it.

use super::{Button, Group, Shortcut, Shortcuts};

/// The buttons a panel key can move to. The D-pad (cursor, volume, seek) and Guide are never among
/// them.
pub(super) const CHOICES: [Button; 14] = [
    Button::South,
    Button::East,
    Button::North,
    Button::West,
    Button::LeftBumper,
    Button::RightBumper,
    Button::Start,
    Button::Select,
    Button::LeftStick,
    Button::RightStick,
    Button::LeftPaddle,
    Button::RightPaddle,
    Button::LeftPaddle2,
    Button::RightPaddle2,
];

/// Where a press in a panel goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPress {
    /// A panel key's button: the button the panel's code expects for that key.
    Key(Button),
    /// The panel's own button for a key that has moved away, so it does nothing here.
    Ignored,
    /// Not a panel key: the menu buttons apply as usual.
    Other,
}

impl Shortcut {
    /// Whether the shortcut is one of a panel's keys.
    pub fn is_panel_key(self) -> bool {
        matches!(self.group(), Group::Media | Group::Numpad | Group::Keyboard)
    }
}

impl Shortcuts {
    /// What a press of `b` does in the panel `group` (media or numpad).
    pub fn panel_press(&self, group: Group, b: Button) -> PanelPress {
        if let Some(key) = Shortcut::of(group).find(|k| self.button(*k) == b) {
            return PanelPress::Key(key.default_input().buttons[0]);
        }
        if Shortcut::of(group).any(|k| k.default_input().buttons[0] == b) {
            return PanelPress::Ignored;
        }
        PanelPress::Other
    }
}
