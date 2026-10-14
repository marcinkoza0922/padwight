//! App settings' Shortcuts tab: where the Guide overlay's opener, the Guide mode's shortcuts and
//! mouse mode's keys sit. They're the same in every setup (see `config::shortcuts`).

use iced::widget::{column, row};

use super::*;
use crate::config::{Group, MouseModeExtras, Shortcut, ShortcutInput};

/// The tabs of the App settings page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum SettingsTab {
    #[default]
    General,
    Shortcuts,
}

impl fmt::Display for SettingsTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SettingsTab::General => "General",
            SettingsTab::Shortcuts => "Controller shortcuts",
        })
    }
}

/// The App settings tabs' state: which tab is on show, and the shortcut button waiting for a
/// controller press (the shortcut, and which of its buttons).
#[derive(Debug, Default)]
pub(super) struct Ui {
    pub(super) tab: SettingsTab,
    pub(super) listening: Option<(Shortcut, usize)>,
}

/// What kind of input a shortcut sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputKind {
    Button,
    DoubleTap,
    TripleTap,
    LongPress,
    Combo,
    Off,
}

impl InputKind {
    const ALL: [InputKind; 6] = [InputKind::Button, InputKind::DoubleTap, InputKind::TripleTap, InputKind::LongPress, InputKind::Combo, InputKind::Off];

    pub(super) fn of(input: Option<&ShortcutInput>) -> Self {
        match input {
            None => InputKind::Off,
            Some(i) => match i.gesture {
                Some(GestureKind::DoubleTap) => InputKind::DoubleTap,
                Some(GestureKind::TripleTap) => InputKind::TripleTap,
                Some(GestureKind::LongPress) => InputKind::LongPress,
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

/// The most buttons in a combo.
const MOST_COMBO: usize = 4;

impl App {
    /// Setups with a profile that uses what the opener takes: the setup, then each such profile
    /// and why.
    pub(super) fn opener_conflicts(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.config
            .all_games()
            .filter_map(|(key, game)| {
                let found = self.config.shortcut_conflicts(game);
                (!found.is_empty()).then(|| (key.unwrap_or("General").to_string(), found))
            })
            .collect()
    }

    /// Changes where `s` sits. A moved shortcut that lands on its default isn't "moved".
    pub(super) fn set_shortcut(&mut self, s: Shortcut, input: Option<ShortcutInput>) {
        self.config.shortcuts.set(s, input);
    }

    /// A press while listening goes to the shortcut's button `at`. Guide can only be the
    /// opener's plain button; anything else is ignored and listening goes on.
    pub(super) fn heard_shortcut_button(&mut self, b: Button) -> bool {
        let Some((s, at)) = self.shortcuts_ui.listening else { return false };
        let allowed = match s.choices() {
            Some(choices) => choices.contains(&b),
            None => ShortcutInput::choices().contains(&b) || (s == Shortcut::OpenGuide && b == Button::Guide),
        };
        if !allowed {
            return false;
        }
        let mut input = self.config.shortcuts.input(s).unwrap_or_else(|| s.default_input());
        match input.buttons.get_mut(at) {
            Some(slot) => *slot = b,
            None => input.buttons.push(b),
        }
        // Guide is only the opener's plain button.
        if input.uses(Button::Guide) && (input.buttons.len() > 1 || input.gesture.is_some()) {
            input.buttons.retain(|x| *x != Button::Guide);
            if input.buttons.is_empty() {
                input.buttons.push(Button::Start);
            }
        }
        self.shortcuts_ui.listening = None;
        self.set_shortcut(s, Some(input.tidied()));
        true
    }

    pub(super) fn view_shortcuts(&self) -> Element<'_, Message> {
        let shortcuts = &self.config.shortcuts;
        let mut page = column![
            note(
                "These apply to every setup and can't be changed per game. A shortcut can sit on a button, a \
                 double tap, triple tap or long press of one, or a combo of buttons. The Guide button belongs to \
                 the Guide overlay: it can't be given to anything else."
            ),
        ]
        .spacing(16);
        let clashes = Shortcut::ALL.iter().filter(|s| !shortcuts.warnings(**s).is_empty()).count();
        if shortcuts.opener_off() {
            page = page.push(
                text("⚠ Nothing opens the Guide overlay, so its menu, Notes and Mappings can't be reached, and neither can the Guide shortcuts or mouse mode.")
                    .size(13)
                    .color(WARNING_COLOR),
            );
        }
        if clashes > 0 {
            let which = if clashes == 1 { "One shortcut shares an input" } else { &format!("{clashes} shortcuts share an input") };
            page = page.push(text(format!("⚠ {which} with another. They're marked below.")).size(13).color(WARNING_COLOR));
        }
        page = page.push(self.view_opener());
        for group in [Group::Guide, Group::Mouse, Group::Media, Group::Numpad, Group::Keyboard, Group::Menu] {
            page = page.push(self.view_shortcut_group(group));
        }
        if !shortcuts.is_default() {
            page = page.push(button(text("Put every shortcut back").size(13)).style(style::secondary).on_press(Message::ResetAllShortcuts));
        }
        page.into()
    }

    fn view_opener(&self) -> Element<'_, Message> {
        let s = Shortcut::OpenGuide;
        let mut rows = vec![
            note(
                "Tap it to open the Guide overlay, and tap it again to close it. Held as a button, it also turns on the \
                 Guide mode while you press a shortcut. As a double tap, triple tap, long press or combo it opens and \
                 closes the overlay, and the shortcuts are then pressed with the overlay open. It also ends mouse mode and closes the panels it opened. The overlays' hints that say \"press Guide\" show it. \
                 Handy for a controller without a Guide button, such as Start + Select. Guide still doesn't reach the game \
                 or Steam when something else opens the overlay.",
            )
            .into(),
            self.shortcut_row(s),
        ];
        let conflicts = self.opener_conflicts();
        if !conflicts.is_empty() {
            rows.push(text("These setups already use that input in a profile, so it will do both. Move one of them:").size(13).color(WARNING_COLOR).into());
            for (setup, profiles) in conflicts {
                let detail: Vec<String> = profiles.into_iter().map(|(profile, why)| format!("{profile}: {why}")).collect();
                rows.push(text(format!("⚠ {setup} — {}", detail.join(" "))).size(13).color(WARNING_COLOR).into());
            }
        }
        section(Group::Opener.title(), None, rows)
    }

    fn view_shortcut_group(&self, group: Group) -> Element<'_, Message> {
        let help_text = match group {
            Group::Guide => {
                "Press these with Guide held, or alone with the overlay open. Anything else does nothing there, so it \
                 never reaches the game. Fixed actions are also on the Buttons tab of any profile."
            }
            Group::Media => {
                "The buttons the media controls use for play or pause, the tracks, the player and mute. A button \
                 another of these keys has trades places with the key's old one. A button none of them has takes \
                 the key, and the one it left does nothing in the media controls. Back and the opener still close \
                 them, and Close is what closes them as well as the opener. These keys take a press before the \
                 menu buttons do, so a shared button does the key's job here."
            }
            Group::Keyboard => {
                "The buttons the on-screen keyboard uses for backspace, space, enter, moving the text cursor and \
                 closing, with the same trading as the media controls. Press is the confirm button, and Close (East \
                 unless moved) closes it when held. The keyboard's cursor and shift don't move."
            }
            Group::Numpad => {
                "The buttons the on-screen numpad uses for backspace and enter, with the same trading as the media \
                 controls. Close (East unless moved) closes the numpad when held. The numpad's cursor and its press \
                 don't move."
            }
            Group::Menu => {
                "The buttons that choose and go back in the Guide overlay's menu, the keyboard, numpad, media controls, \
                 the in-game menu, menus with a list, grid or carousel, and the library offer. The one you pick and \
                 A (or B) trade places, so each still does what the other did. The D-pad and Guide aren't offered, \
                 as the D-pad moves the menus. A menu that sets its own close button keeps it, and menus that read \
                 the face buttons themselves (directional and button menus) aren't changed."
            }
            _ => {
                "The keys mouse mode gives the buttons. The sticks and triggers (move, scroll, click) stay where they are. \
                 Other buttons do nothing in mouse mode, and neither do the gyro and touchpad unless you tick them below."
            }
        };
        let mut rows: Vec<_> = Shortcut::of(group).map(|s| if s.choices().is_some() { self.button_row(s) } else { self.shortcut_row(s) }).collect();
        if group == Group::Mouse {
            rows.push(self.mouse_mode_extras());
        }
        if group == Group::Menu {
            rows.push(self.peek_trigger_row());
        }
        section(group.title(), Some(help_text.into()), rows)
    }

    /// The trigger that hides the in-game editor while held, to show the overlay being edited.
    fn peek_trigger_row(&self) -> Element<'_, Message> {
        let pick = dropdown(crate::config::PeekTrigger::ALL, Some(self.config.peek_trigger), Message::SetPeekTrigger).width(190);
        let help = text("Hold it in the in-game editor, on a page about a menu or overlay, to hide the editor and see the overlay.").size(13).color(MUTED_COLOR);
        column![labeled_in(190.0, "Show the overlay being edited", pick.into()), help].spacing(2).into()
    }

    /// Whether the gyro and touchpad move the pointer in mouse mode.
    fn mouse_mode_extras(&self) -> Element<'_, Message> {
        let extras = self.config.mouse_mode;
        column![
            checkbox(extras.gyro)
                .label("Gyro moves the pointer, as the profile's gyro does (at a default speed if it aims a stick or is off)")
                .on_toggle(move |gyro| Message::SetMouseModeExtras(MouseModeExtras { gyro, ..extras })),
            checkbox(extras.touchpad)
                .label("Touchpad moves the pointer, and pressing it clicks")
                .on_toggle(move |touchpad| Message::SetMouseModeExtras(MouseModeExtras { touchpad, ..extras })),
        ]
        .spacing(8)
        .into()
    }

    /// A single button from the few `s` can take (see [`Shortcut::choices`]): confirm or back, or a
    /// panel key.
    fn button_row(&self, s: Shortcut) -> Element<'_, Message> {
        let now = self.config.shortcuts.button(s);
        let choices = s.choices().unwrap_or_default().to_vec();
        let listening = self.shortcuts_ui.listening == Some((s, 0));
        let mut line = row![
            dropdown(choices, Some(now), move |b: Button| Message::SetShortcut(s, Some(ShortcutInput::button(b)))).width(190),
            button(text(if listening { "Press a button…" } else { "Listen" }).size(13))
                .style(style::secondary)
                .on_press(Message::ListenShortcut(if listening { None } else { Some((s, 0)) })),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if self.config.shortcuts.is_moved(s) {
            line = line.push(button(text("Reset").size(13)).style(style::secondary).on_press(Message::ResetShortcut(s)));
        }
        let mut out = column![labeled_in(190.0, s.label(), line.wrap().into())].spacing(2);
        for w in self.config.shortcuts.warnings(s) {
            out = out.push(text(format!("⚠ {w}")).size(13).color(WARNING_COLOR));
        }
        out.into()
    }

    /// One shortcut: its kind, its buttons, and what's wrong with it.
    fn shortcut_row(&self, s: Shortcut) -> Element<'_, Message> {
        let input = self.config.shortcuts.input(s);
        let kind = InputKind::of(input.as_ref());
        let opener = s == Shortcut::OpenGuide;
        // Off is a choice for everything; the opener warns instead of stopping you.
        let kinds = InputKind::ALL;
        let current = input.clone();
        let fallback = if opener { Button::Start } else { s.default_input().buttons[0] };
        let mut line = row![dropdown(kinds, Some(kind), move |k: InputKind| Message::SetShortcut(s, k.apply(current.as_ref(), fallback))).width(130)]
            .spacing(8)
            .align_y(Alignment::Center);
        if let Some(input) = &input {
            for (at, b) in input.buttons.iter().enumerate() {
                let mut choices = ShortcutInput::choices();
                if opener && kind == InputKind::Button {
                    choices.insert(0, Button::Guide);
                }
                let base = input.clone();
                line = line.push(
                    dropdown(choices, Some(*b), move |picked: Button| {
                        let mut i = base.clone();
                        i.buttons[at] = picked;
                        Message::SetShortcut(s, Some(i.tidied()))
                    })
                    .width(190),
                );
                let listening = self.shortcuts_ui.listening == Some((s, at));
                line = line.push(
                    button(text(if listening { "Press a button…" } else { "Listen" }).size(13))
                        .style(style::secondary)
                        .on_press(if listening { Message::ListenShortcut(None) } else { Message::ListenShortcut(Some((s, at))) }),
                );
                if kind == InputKind::Combo && input.buttons.len() > 2 {
                    let mut without = input.clone();
                    without.buttons.remove(at);
                    line = line.push(button(text("✕").size(13)).style(style::secondary).on_press(Message::SetShortcut(s, Some(without))));
                }
            }
            if kind == InputKind::Combo && input.buttons.len() < MOST_COMBO {
                let mut more = input.clone();
                if let Some(next) = ShortcutInput::choices().into_iter().find(|b| !more.buttons.contains(b)) {
                    more.buttons.push(next);
                    line = line.push(button(text("+ Button").size(13)).style(style::secondary).on_press(Message::SetShortcut(s, Some(more))));
                }
            }
        }
        if self.config.shortcuts.is_moved(s) {
            line = line.push(button(text("Reset").size(13)).style(style::secondary).on_press(Message::ResetShortcut(s)));
        }
        let mut out = column![labeled_in(190.0, s.label(), line.wrap().into())].spacing(2);
        for w in self.config.shortcuts.warnings(s) {
            out = out.push(text(format!("⚠ {w}")).size(13).color(WARNING_COLOR));
        }
        if kind == InputKind::Off && opener {
            out = out.push(text("⚠ Not set: nothing opens the Guide overlay.").size(13).color(WARNING_COLOR));
        }
        out.into()
    }
}

#[cfg(test)]
mod tests;
