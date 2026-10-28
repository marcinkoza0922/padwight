//! The Tools menu over the game: Guide + right trigger opens it, and it takes the controller until
//! it's closed. What a tool sends out goes to the game as typed keys, or to the clipboard. The
//! Doodle screen draws over the game instead, and keeps its doodles after it's closed.

use super::*;
use crate::tools::{Outcome, ToolsSession};

impl Daemon {
    /// Opens the Tools menu for controller `device`, which pressed Guide + right trigger.
    pub(super) fn open_tools(&mut self, device: u64) {
        if self.active.is_some() || !self.ensure_overlay_process() {
            return;
        }
        self.release_mappings();
        log!("tools open");
        self.active = Some(Active::Tools(Box::new(ToolsSession::new()), device));
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Menu, Some(sound::Feedback::Open));
    }

    /// Feeds an input to the Tools menu, and does what it asks. Returns whether the menu closed.
    pub(super) fn tools_input(&mut self, ev: InputEvent) -> bool {
        let Some(Active::Tools(session, _)) = &mut self.active else { return true };
        let (b, down) = match ev {
            InputEvent::Button(b, down) => (b, down),
            InputEvent::Axis(axis, value) => {
                session.axis(&mut self.doodles, axis, value, Instant::now());
                return false;
            }
            InputEvent::Touchpad(_) => return false,
        };
        let before = session.cursor();
        let doodling = session.in_doodle();
        let outcome = session.handle(b, down, &mut self.doodles, Instant::now());
        let (picked, moved) = (down && b == Button::South, session.cursor() != before);
        if !doodling {
            self.overlay_sound(sound::SoundOverlay::Menu, sound::Feedback::of(picked, moved));
        }
        match outcome {
            None => {
                self.broadcast_overlay();
                false
            }
            Some(outcome) => self.tools_outcome(outcome),
        }
    }

    /// Moves the Doodle cursor by the stick, and lets the marks it carries go on. A Doodle screen
    /// that East has left, or Guide has, closes the menu.
    pub(super) fn tick_doodle(&mut self, dt: f32) {
        let Some(Active::Tools(session, _)) = &mut self.active else { return };
        match session.tick(&mut self.doodles, dt, Instant::now()) {
            Some(outcome) => {
                self.tools_outcome(outcome);
            }
            None => self.broadcast_overlay(),
        }
    }

    /// Whether the Doodle screen is up and has something to move, so the screen is ticked.
    pub(super) fn doodle_ticking(&self) -> bool {
        matches!(&self.active, Some(Active::Tools(session, _)) if session.needs_tick())
    }

    /// The Doodle cursor, while the Doodle screen is up.
    pub(super) fn doodle_cursor(&self) -> Option<crate::overlay::DoodleCursor> {
        match &self.active {
            Some(Active::Tools(session, _)) => session.doodle_cursor(),
            _ => None,
        }
    }

    /// The Doodle screen's HUD, while it's up.
    pub(super) fn doodle_hud(&self) -> Option<crate::overlay::DoodleHud> {
        match &self.active {
            Some(Active::Tools(session, _)) => session.doodle_hud(self.config.active_menu_style().clone()),
            _ => None,
        }
    }

    /// Does what a tool asks: closes the menu, and types or copies. Returns whether it closed.
    fn tools_outcome(&mut self, outcome: Outcome) -> bool {
        match outcome {
            Outcome::Close => {
                self.close_overlay();
                true
            }
            Outcome::Type(text) => {
                self.close_overlay();
                self.type_text(&text);
                true
            }
            Outcome::Copy(text) => {
                self.close_overlay();
                self.copy_text(text);
                true
            }
        }
    }

    /// Types `text` into the game, the way the on-screen keyboard would.
    fn type_text(&mut self, text: &str) {
        let actions = crate::overlay::typing(text);
        self.apply_keyboard(actions);
    }

    /// Copies `text` to the clipboard, off the daemon's thread, and toasts whether it worked.
    fn copy_text(&mut self, text: String) {
        let tx = self.tx.clone();
        thread::spawn(move || {
            let lines = match crate::clipboard::copy(&text) {
                Ok(()) => vec![format!("Copied {text}")],
                Err(reason) => vec!["Could not copy".into(), reason],
            };
            let _ = tx.send(Msg::Toast(lines));
        });
    }
}
