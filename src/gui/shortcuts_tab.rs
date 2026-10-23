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

mod input_kind;

pub(super) use input_kind::InputKind;

/// The most buttons in a combo.
const MOST_COMBO: usize = 4;

/// The width of a shortcut's kind dropdown. Rows that take a plain button leave this much room
/// instead, so every button picker on the tab starts in the same place.
const KIND_WIDTH: f32 = 130.0;

/// The button that waits for a press on the controller, then takes it.
const SET_BY_PRESSING: &str = "Set by pressing";

/// A button in a shortcut's picker, named as the controller in use prints it, as the Buttons tab
/// names it.
#[derive(Debug, Clone, PartialEq)]
struct NamedButton(Button, String);

impl fmt::Display for NamedButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.1)
    }
}

/// A shortcut's name in its row. The card's title already says whose keys these are, so the
/// keyboard's and numpad's don't say it again (the warnings, read out of context, still do).
fn row_label(s: Shortcut) -> &'static str {
    match s {
        Shortcut::NumpadBackspace | Shortcut::KeyboardBackspace => "Backspace",
        Shortcut::NumpadEnter | Shortcut::KeyboardEnter => "Enter",
        Shortcut::NumpadClose | Shortcut::KeyboardClose => "Close",
        Shortcut::KeyboardSpace => "Space",
        other => other.label(),
    }
}

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

    /// `buttons` as a picker's choices, named for the controller in use.
    fn named(&self, buttons: impl IntoIterator<Item = Button>) -> Vec<NamedButton> {
        let family = self.shown_pad().family;
        buttons.into_iter().map(|b| NamedButton(b, crate::info::button_name(b, family))).collect()
    }

    pub(super) fn view_shortcuts(&self) -> Element<'_, Message> {
        let shortcuts = &self.config.shortcuts;
        let mut page = column![
            row![
                note("These apply to every setup and can't be changed per game."),
                help(
                    "A shortcut can be a button, a double tap, triple tap or long press of one, or a combo. \
                     The Guide button is reserved for the Guide overlay."
                        .into()
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(16);
        let clashes = Shortcut::ALL.iter().filter(|s| !shortcuts.warnings(**s).is_empty()).count();
        if shortcuts.opener_off() {
            page = page.push(
                text("⚠ Nothing opens the Guide overlay, so its menu, Notes, Mappings, the Guide shortcuts and mouse mode are out of reach.")
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
        let mut rows = vec![self.shortcut_row(s)];
        let conflicts = self.opener_conflicts();
        if !conflicts.is_empty() {
            rows.push(text("These setups already use that input in a profile, so it will do both. Move one:").size(13).color(WARNING_COLOR).into());
            for (setup, profiles) in conflicts {
                let detail: Vec<String> = profiles.into_iter().map(|(profile, why)| format!("{profile}: {why}")).collect();
                rows.push(text(format!("⚠ {setup} — {}", detail.join(" "))).size(13).color(WARNING_COLOR).into());
            }
        }
        section(
            Group::Opener.title(),
            Some(
                "Tap to open or close the Guide overlay. On a single button, holding it while pressing a shortcut \
                 also works. As a double tap, triple tap, long press or combo, it only opens and closes the overlay, \
                 and shortcuts are pressed while it's open. It also ends mouse mode and closes the panels it opened, \
                 and overlay hints that say \"press Guide\" show it instead. Handy for a controller without a Guide \
                 button, e.g. Start + Select. Guide still doesn't reach the game or Steam when something else opens \
                 the overlay."
                    .into(),
            ),
            rows,
        )
    }

    fn view_shortcut_group(&self, group: Group) -> Element<'_, Message> {
        let help_text = match group {
            Group::Guide => {
                "Press these with Guide held, or alone with the overlay open. Other buttons do nothing there and \
                 never reach the game. Fixed actions are also on every profile's Buttons tab."
            }
            Group::Media => {
                "The buttons the media controls use for play or pause, tracks, player and mute. Picking a button \
                 another key uses swaps the two; picking an unused one leaves the key's old button doing nothing \
                 here. Close, Back and the opener close the controls. These keys win over the menu buttons when \
                 they share one."
            }
            Group::Keyboard => {
                "The buttons the on-screen keyboard uses for backspace, space, enter, moving the text cursor and \
                 closing; they swap like the media controls'. Press is the confirm button, and holding Close (East \
                 by default) closes it. The keyboard's cursor and Shift can't be moved."
            }
            Group::Numpad => {
                "The buttons the on-screen numpad uses for backspace and enter; they swap like the media controls'. \
                 Holding Close (East by default) closes it. The numpad's cursor and press can't be moved."
            }
            Group::Menu => {
                "The buttons that choose and go back in the Guide menu, keyboard, numpad, media controls, in-game \
                 menu, list, grid and carousel menus, and the library offer. The one you pick swaps with A (or B). \
                 The D-pad and Guide aren't offered, since the D-pad moves through menus. A menu with its own close \
                 button keeps it, and directional and button menus, which use the face buttons themselves, aren't \
                 changed."
            }
            _ => {
                "The keys mouse mode puts on the buttons. The sticks and triggers (move, scroll, click) are fixed. \
                 Other buttons do nothing in mouse mode, nor do the gyro and touchpad unless ticked below."
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
        let help = text("In the in-game editor, hold this to hide the editor and see the overlay underneath.").size(13).color(MUTED_COLOR);
        // The picker starts where the other rows' button pickers do, past their kind column.
        let line = row![space().width(KIND_WIDTH), pick].spacing(8).align_y(Alignment::Center);
        column![labeled_in(190.0, "Peek while editing", line.into()), help].spacing(2).into()
    }

    /// Whether the gyro and touchpad move the pointer in mouse mode.
    fn mouse_mode_extras(&self) -> Element<'_, Message> {
        let extras = self.config.mouse_mode;
        column![
            row![
                checkbox(extras.gyro)
                    .label("Gyro moves the pointer")
                    .on_toggle(move |gyro| Message::SetMouseModeExtras(MouseModeExtras { gyro, ..extras })),
                help("At the profile's gyro mouse speed, or a default speed if its gyro aims a stick or is off.".into()),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
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
        let now = self.named([self.config.shortcuts.button(s)]).pop();
        let choices = self.named(s.choices().unwrap_or_default().iter().copied());
        let listening = self.shortcuts_ui.listening == Some((s, 0));
        let mut line = row![
            space().width(KIND_WIDTH),
            dropdown(choices, now, move |b: NamedButton| Message::SetShortcut(s, Some(ShortcutInput::button(b.0)))).width(190),
            button(text(if listening { "Press a button…" } else { SET_BY_PRESSING }).size(13))
                .style(style::secondary)
                .on_press(Message::ListenShortcut(if listening { None } else { Some((s, 0)) })),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if self.config.shortcuts.is_moved(s) {
            line = line.push(button(text("Reset").size(13)).style(style::secondary).on_press(Message::ResetShortcut(s)));
        }
        let mut out = column![labeled_in(190.0, row_label(s), line.wrap().into())].spacing(2);
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
        let mut line = row![dropdown(kinds, Some(kind), move |k: InputKind| Message::SetShortcut(s, k.apply(current.as_ref(), fallback))).width(KIND_WIDTH)]
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
                    dropdown(self.named(choices), self.named([*b]).pop(), move |picked: NamedButton| {
                        let mut i = base.clone();
                        i.buttons[at] = picked.0;
                        Message::SetShortcut(s, Some(i.tidied()))
                    })
                    .width(190),
                );
                let listening = self.shortcuts_ui.listening == Some((s, at));
                line = line.push(
                    button(text(if listening { "Press a button…" } else { SET_BY_PRESSING }).size(13))
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
        let mut out = column![labeled_in(190.0, row_label(s), line.wrap().into())].spacing(2);
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
