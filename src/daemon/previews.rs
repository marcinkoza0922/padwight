//! The info or log overlay the in-game editor's page is about, shown where it goes while it's
//! edited: an info overlay with its live values, a log overlay with sample presses.

use super::*;
use crate::{config::LogSource, info::Glyphs, inputlog::LogView, layout_editor::Preview};

impl Daemon {
    /// `info` and `logs` with the overlay the editor's page is about put in (in place of itself,
    /// if it was already up).
    pub(super) fn with_preview(&self, mut info: Vec<crate::info::InfoView>, mut logs: Vec<LogView>, values: &crate::info::Live, now: Instant) -> (Vec<crate::info::InfoView>, Vec<LogView>) {
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
                    let LogSource::Input(s) = &o.source;
                    let sample = crate::inputlog::Inputs::sample();
                    let glyphs = Glyphs { family: values.family, nintendo_layout: values.nintendo_layout };
                    let view = crate::inputlog::log_view(&sample.entries(None), s, &o.style, glyphs, sample.now.unwrap_or(now));
                    logs.retain(|v| v.name != o.name);
                    logs.push(LogView { name: o.name.clone(), opacity: 1.0, ..view });
                }
            }
            None => {}
        }
        (info, logs)
    }
}
