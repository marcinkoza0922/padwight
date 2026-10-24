//! Leaving the in-game menu: the question about unsaved changes, Guide closing it, and the
//! holds it waits in (for a controller, or for Save or Discard once remapping is off).

use super::*;

impl SystemMenu {
    /// Leaves the menu's top page, `how`: back to the Guide overlay, or closes. Unsaved changes
    /// ask first.
    pub(super) fn leave(&mut self, config: &Config, how: Exit) -> Option<Outcome> {
        if *self.saved == *config {
            return Some(if self.from_guide && how == Exit::Back { Outcome::ReturnToGuide } else { Outcome::Close });
        }
        let left = std::mem::replace(&mut self.screen, Screen::Main);
        self.screen = Screen::Confirm(Box::new(left), self.cursor, how);
        None
    }

    /// Guide was pressed: closes the menu outright, unless there are unsaved changes, which it
    /// asks about first (the question then needs an answer).
    pub fn guide_pressed(&mut self, config: &Config) -> Option<Outcome> {
        if let Screen::Typing(editor, _) = &self.screen {
            self.screen = Screen::Editor(editor.clone());
            return None;
        }
        match self.screen {
            _ if self.hold.is_some() => None,
            Screen::Confirm(..) => None,
            _ if *self.saved == *config => Some(Outcome::Close),
            _ => self.leave(config, Exit::Close),
        }
    }

    /// The config as it was when the menu opened, which Discard puts back.
    pub fn saved(&self) -> &Config {
        &self.saved
    }

    /// Whether `config` has changes the menu hasn't saved.
    pub fn unsaved(&self, config: &Config) -> bool {
        *self.saved != *config
    }

    /// Remapping was turned on or off elsewhere, which is saved at once: it isn't a change of
    /// the menu's.
    pub fn remapping_set(&mut self, enabled: bool) {
        self.saved.enabled = enabled;
    }

    /// Puts the menu on hold, `why`. A held direction stops repeating.
    pub fn hold(&mut self, why: Hold) {
        self.hold = Some(why);
        self.repeat.stop();
        self.stick = (0.0, 0.0);
        self.stick_dir = crate::nav::StickDir::default();
        self.remapping_armed = false;
    }

    /// Takes the menu off hold, where it was.
    pub fn carry_on(&mut self) {
        self.hold = None;
    }

    /// Why the menu is on hold, if it is.
    pub fn on_hold(&self) -> Option<Hold> {
        self.hold
    }

    /// Input while on hold: once remapping is off, A saves and X discards, then the menu closes.
    pub(super) fn held(&self, hold: Hold, ev: InputEvent) -> Option<Outcome> {
        match (hold, ev) {
            (Hold::RemappingOff, InputEvent::Button(Button::South, true)) => Some(Outcome::Save { back: false }),
            (Hold::RemappingOff, InputEvent::Button(Button::West, true)) => Some(Outcome::Discard { saved: self.saved.clone(), back: false }),
            _ => None,
        }
    }

    /// Readies the menu to be put aside with its unsaved changes, to come back later where they
    /// were made: off hold, and back from the unsaved-changes question if it was asking.
    pub fn put_aside(&mut self) {
        self.carry_on();
        self.keep_editing();
    }

    /// Back from the unsaved-changes question to the screen it was asked on.
    pub(super) fn keep_editing(&mut self) {
        if let Screen::Confirm(left, cursor, _) = std::mem::replace(&mut self.screen, Screen::Main) {
            self.screen = *left;
            self.cursor = cursor;
        }
    }

    /// Whether answering the unsaved-changes question goes back to the Guide overlay, not out.
    pub(super) fn answer_goes_back(&self) -> bool {
        matches!(self.screen, Screen::Confirm(_, _, Exit::Back)) && self.from_guide
    }

    pub(super) fn discard(&self) -> Option<Outcome> {
        Some(Outcome::Discard { saved: self.saved.clone(), back: self.answer_goes_back() })
    }

    /// The unsaved-changes question: A saves, X discards, B keeps editing.
    pub(super) fn confirm_step(&mut self, step: Step) -> Option<Outcome> {
        match step {
            Step::Pick => Some(Outcome::Save { back: self.answer_goes_back() }),
            Step::Back => {
                self.keep_editing();
                None
            }
            Step::Up | Step::Down | Step::Left | Step::Right => None,
        }
    }
}
