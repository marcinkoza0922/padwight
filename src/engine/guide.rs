//! The Guide button, the Guide overlay's own presses, and mouse mode.

use super::*;

impl Engine {
    /// The Guide button is handled here, whatever the profile maps it to, so it works the same
    /// in every game. A tap toggles the Guide overlay; holding it turns the Guide mode on, so
    /// Guide + a button runs that button's Guide shortcut.
    pub(super) fn guide_button(&mut self, pressed: bool) {
        match (pressed, self.guide_held) {
            // In mouse mode, Guide only exits it. Counted as a chord so letting go doesn't open the overlay.
            (true, None) if self.mouse_mode => {
                self.set_mouse_mode(false);
                self.guide_held = Some(true);
            }
            (true, None) => {
                self.guide_held = Some(false);
                self.layers_changed = true;
            }
            (false, Some(chorded)) => {
                self.guide_held = None;
                self.layers_changed = true;
                if !chorded {
                    self.set_guide_open(!self.guide_open);
                }
            }
            _ => {}
        }
    }

    /// A button went down: it makes the Guide hold a chord, and it closes an open Guide overlay
    /// unless it drives the overlay's menu. The press itself still runs, under the Guide layer,
    /// if it's a Guide shortcut.
    pub(super) fn other_button_pressed(&mut self, b: Button, now: Instant) {
        if self.is_trigger_member(b) {
            return;
        }
        if let Some(chorded) = self.guide_held.as_mut() {
            *chorded = true;
        }
        if self.guide_open {
            // The menu hears A and B where the confirm and back buttons are.
            let b = if self.face_swap { crate::info::nintendo_face(b) } else { b };
            let b = self.menu_keys.logical(b, true);
            if guide_nav(b) {
                self.guide_nav.push(b);
                if matches!(b, Button::DpadUp | Button::DpadDown | Button::LeftStickUp | Button::LeftStickDown) {
                    self.guide_repeat = Some((b, now + GUIDE_REPEAT_DELAY));
                }
            } else {
                self.set_guide_open(false);
            }
        }
    }

    /// Takes the menu presses made since the last call (see [`guide_nav`]).
    pub fn take_guide_nav(&mut self) -> Vec<Button> {
        std::mem::take(&mut self.guide_nav)
    }

    /// Opens the Guide overlay, if it's down.
    pub fn open_guide_overlay(&mut self) {
        if !self.guide_open {
            self.set_guide_open(true);
        }
    }

    /// Closes the Guide overlay, if it's up.
    pub fn close_guide_overlay(&mut self) {
        if self.guide_open {
            self.set_guide_open(false);
        }
    }

    /// Turns the Guide overlay off and mouse mode on: the overlay's Mouse mode item.
    pub fn enter_mouse_mode(&mut self) {
        self.set_guide_open(false);
        self.set_mouse_mode(true);
    }

    pub(super) fn set_guide_open(&mut self, open: bool) {
        self.guide_open = open;
        self.guide_repeat = None;
        self.layers_changed = true;
    }

    /// Queues a repeat of the held menu direction once it's due.
    pub(super) fn repeat_guide_nav(&mut self, now: Instant) {
        if let Some((b, due)) = self.guide_repeat
            && now >= due
        {
            self.guide_nav.push(b);
            self.guide_repeat = Some((b, now + GUIDE_REPEAT_RATE));
        }
    }

    pub(super) fn set_mouse_mode(&mut self, on: bool) {
        self.mouse_mode = on;
        self.layers_changed = true;
    }

    /// Sets which buttons confirm and go back in the Guide overlay's menu.
    pub fn set_menu_keys(&mut self, keys: crate::config::MenuKeys) {
        self.menu_keys = keys;
    }

    /// Sets what opens the Guide overlay. Whatever the old opener had begun is dropped.
    pub fn set_opener(&mut self, opener: GuideOpener) {
        if self.opener != opener {
            self.opener = opener;
            self.guide_held = None;
            self.layers_changed = true;
        }
    }

    /// Whether `b` is the button that opens the overlay by being held or tapped.
    pub(super) fn is_held_opener(&self, b: Button) -> bool {
        match &self.opener {
            GuideOpener::Guide => b == Button::Guide,
            GuideOpener::Button(held) => b == *held,
            GuideOpener::Triggered(_) => false,
        }
    }

    /// Whether `b` is part of a gesture or combo that opens the overlay, which then isn't a
    /// press that closes it.
    pub(super) fn is_trigger_member(&self, b: Button) -> bool {
        matches!(&self.opener, GuideOpener::Triggered(members) if members.contains(&b))
    }

    /// Handles the Guide button, mouse mode's entry, and the overlay's own presses. Returns true
    /// when the press is theirs, so the profile doesn't see it.
    pub(super) fn guide_routes(&mut self, b: Button, pressed: bool, now: Instant) -> bool {
        if !pressed && self.guide_repeat.is_some_and(|(r, _)| r == b) {
            self.guide_repeat = None;
        }
        if self.is_held_opener(b) {
            self.guide_button(pressed);
            return true;
        }
        // Guide is the Guide overlay's alone: moved off it, it does nothing and never reaches the game.
        if b == Button::Guide {
            return true;
        }
        if pressed {
            self.other_button_pressed(b, now);
        }
        false
    }
}

/// Whether a button drives the Guide overlay's menu rather than closing the overlay: the D-pad
/// and left stick move the highlight, A chooses and B closes, and the right stick turns the
/// Mappings' pages (its sideways presses do nothing, but they don't close the overlay either).
pub(super) fn guide_nav(b: Button) -> bool {
    matches!(
        b,
        Button::DpadUp
            | Button::DpadDown
            | Button::DpadLeft
            | Button::DpadRight
            | Button::LeftStickUp
            | Button::LeftStickDown
            | Button::LeftStickLeft
            | Button::LeftStickRight
            | Button::RightStickUp
            | Button::RightStickDown
            | Button::RightStickLeft
            | Button::RightStickRight
            | Button::South
            | Button::East
    )
}
