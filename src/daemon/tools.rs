//! The Tools menu over the game: Guide + right trigger opens it, and it takes the controller until
//! it's closed. What a tool sends out goes to the game as typed keys, or to the clipboard.

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
        self.active = Some(Active::Tools(ToolsSession::new(), device));
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Menu, Some(sound::Feedback::Open));
    }

    /// Feeds a press to the Tools menu, and does what it asks. Returns whether the menu closed.
    pub(super) fn tools_input(&mut self, ev: InputEvent) -> bool {
        let InputEvent::Button(b, true) = ev else { return false };
        let Some(Active::Tools(session, _)) = &mut self.active else { return true };
        let before = session.cursor();
        let outcome = session.handle(b);
        let (picked, moved) = (b == Button::South, session.cursor() != before);
        self.overlay_sound(sound::SoundOverlay::Menu, sound::Feedback::of(picked, moved));
        match outcome {
            None => {
                self.broadcast_overlay();
                false
            }
            Some(Outcome::Close) => {
                self.close_overlay();
                true
            }
            Some(Outcome::Type(text)) => {
                self.close_overlay();
                self.type_text(&text);
                true
            }
            Some(Outcome::Copy(text)) => {
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
