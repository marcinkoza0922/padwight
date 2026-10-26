//! The kinds of input a shortcut can sit on, as the shortcut rows' first dropdown offers them.

use super::*;

/// What kind of input a shortcut sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gui) enum InputKind {
    Button,
    DoubleTap,
    TripleTap,
    LongPress,
    Combo,
    Off,
}

impl InputKind {
    pub(super) const ALL: [InputKind; 6] = [InputKind::Button, InputKind::DoubleTap, InputKind::TripleTap, InputKind::LongPress, InputKind::Combo, InputKind::Off];

    pub(super) fn of(input: Option<&ShortcutInput>) -> Self {
        match input {
            None => InputKind::Off,
            Some(i) => match i.gesture {
                Some(GestureKind::DoubleTap) => InputKind::DoubleTap,
                Some(GestureKind::TripleTap) => InputKind::TripleTap,
                Some(GestureKind::LongPress) => InputKind::LongPress,
                // Not offered for shortcuts: they act on the press like a plain button.
                Some(GestureKind::OnPress | GestureKind::OnRelease) => InputKind::Button,
                None if i.is_combo() => InputKind::Combo,
                None => InputKind::Button,
            },
        }
    }

    fn gesture(self) -> Option<GestureKind> {
        match self {
            InputKind::DoubleTap => Some(GestureKind::DoubleTap),
            InputKind::TripleTap => Some(GestureKind::TripleTap),
            InputKind::LongPress => Some(GestureKind::LongPress),
            _ => None,
        }
    }

    /// `input` as this kind: the buttons it has are kept as far as they fit.
    pub(super) fn apply(self, input: Option<&ShortcutInput>, fallback: Button) -> Option<ShortcutInput> {
        let mut buttons = input.map(|i| i.buttons.clone()).unwrap_or_default();
        if buttons.is_empty() {
            buttons.push(fallback);
        }
        match self {
            InputKind::Off => None,
            InputKind::Combo => {
                if buttons.len() < 2 {
                    let other = ShortcutInput::choices().into_iter().find(|b| !buttons.contains(b) && *b != Button::Guide);
                    buttons.extend(other);
                }
                Some(ShortcutInput { buttons, gesture: None })
            }
            kind => Some(kind.gesture().map_or_else(|| ShortcutInput::button(buttons[0]), |g| ShortcutInput::gesture(buttons[0], g))),
        }
    }
}

impl fmt::Display for InputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            InputKind::Button => "Button",
            InputKind::DoubleTap => "Double tap",
            InputKind::TripleTap => "Triple tap",
            InputKind::LongPress => "Long press",
            InputKind::Combo => "Combo",
            InputKind::Off => "Off",
        })
    }
}
