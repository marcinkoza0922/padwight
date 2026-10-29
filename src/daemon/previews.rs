//! The info or log overlay the in-game editor's page is about, shown where it goes while it's
//! edited: an info overlay with its live values, a log overlay with sample presses.

use super::*;
use crate::{info::Glyphs, inputlog::LogView, layout_editor::Preview};

impl Daemon {
    /// The menu the editor's page is about, drawn under the editor where it goes.
    pub(super) fn preview_menu(&self) -> Option<crate::menu::MenuView> {
        let Some(Active::Settings(menu, _)) = &self.active else { return None };
        let Some(Preview::Menu(m, item)) = menu.preview() else { return None };
        crate::layout_editor::menu_view(&self.config, m, item)
    }

    /// Whether the editor is held back to show what it's editing: a page that has an overlay on
    /// screen, and the peek trigger pulled.
    pub(super) fn peeking(&self) -> bool {
        matches!(&self.active, Some(Active::Settings(menu, _)) if menu.preview().is_some())
            && self.config.peek_trigger.pulled(self.menu_live.triggers)
    }

    /// `info` and `logs` with the overlay the editor's page is about put in (in place of itself,
    /// if it was already up).
    pub(super) fn with_preview(&self, mut info: Vec<crate::info::InfoView>, mut logs: Vec<LogView>, values: &crate::info::Live) -> (Vec<crate::info::InfoView>, Vec<LogView>) {
        let Some(Active::Settings(menu, _)) = &self.active else { return (info, logs) };
        let game = self.config.active_game();
        match menu.preview() {
            Some(Preview::Info(i)) => {
                if let Some(o) = game.info.get(i) {
                    info.retain(|v| v.name != o.name);
                    info.push(crate::info::resolve(o, values));
                }
            }
            Some(Preview::Log(i)) => {
                if let Some(o) = game.logs.get(i) {
                    let glyphs = Glyphs { family: values.family, nintendo_layout: values.nintendo_layout };
                    let view = crate::inputlog::Inputs::sample_view(&o.source, &o.style, glyphs);
                    logs.retain(|v| v.name != o.name);
                    logs.push(LogView { name: o.name.clone(), opacity: 1.0, ..view });
                }
            }
            Some(Preview::Menu(..)) | None => {}
        }
        (info, logs)
    }
}
