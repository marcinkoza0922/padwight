//! "Go to" buttons: from an action that names a macro, menu, overlay, layer or mode to that item's
//! card on its own tab.

use super::*;

impl App {
    /// Shows the tab of the current setup's item `name`, with its card open.
    pub(super) fn go_to_item(&mut self, kind: ItemKind, name: &str) -> Task<Message> {
        let find = |names: Vec<&str>| names.iter().position(|n| *n == name);
        let game = self.game();
        let (tab, index) = match kind {
            ItemKind::Macro => (GameTab::Macros, game.macros.iter().position(|m| m.name == name)),
            ItemKind::Menu => (GameTab::Menus, game.menus.iter().position(|m| m.name == name)),
            ItemKind::Info => (GameTab::Info, game.info.iter().position(|o| o.name == name)),
            ItemKind::Log => (GameTab::Logs, game.logs.iter().position(|o| o.name == name)),
            ItemKind::Layer => (GameTab::Layers, find(game.layers.iter().map(|l| l.name.as_str()).collect())),
            ItemKind::Mode => (GameTab::Modes, find(game.modes.iter().map(|l| l.name.as_str()).collect())),
        };
        let Some(i) = index else { return Task::none() };
        self.game_tab = tab;
        self.found = None;
        match kind {
            ItemKind::Macro => {
                self.open_macros.insert(i);
            }
            ItemKind::Menu => {
                self.open_menus.insert(i);
            }
            ItemKind::Info => {
                self.open_infos.insert(i);
            }
            ItemKind::Log => {
                self.open_logs.insert(i);
            }
            ItemKind::Layer => {
                self.layer = i;
                self.expanded.clear();
            }
            ItemKind::Mode => {
                self.mode = i;
                self.expanded.clear();
            }
        }
        iced::widget::operation::snap_to("main", scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) })
    }
}

/// A link to the item `name` of `kind`, to put beside the picker that names it. Nothing for an
/// item that doesn't exist, or a built-in mode, which has no card.
pub(super) fn goto_button<'a>(kind: ItemKind, name: &str, exists: bool) -> Option<Element<'a, Message>> {
    let built_in = kind == ItemKind::Mode && crate::config::built_in_modes().iter().any(|m| m.name == name);
    (exists && !built_in).then(|| {
        button(text(format!("Go to {} ›", kind.noun())).size(13).color(style_accent()))
            .style(style::secondary)
            .padding([2, 10])
            .on_press(Message::GoToItem(kind, name.to_string()))
            .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::with_game;

    #[test]
    fn goes_to_the_named_layer() {
        let mut app = with_game();
        let _ = app.update(Message::SelectGameTab(GameTab::Layers));
        let _ = app.update(Message::NewLayer);
        let _ = app.update(Message::NewLayer);
        let name = app.game().layers[1].name.clone();
        let _ = app.update(Message::SelectGameTab(GameTab::Profiles));
        let _ = app.update(Message::GoToItem(ItemKind::Layer, name));
        assert_eq!(app.game_tab, GameTab::Layers);
        assert_eq!(app.layer, 1);
    }

    #[test]
    fn built_in_modes_and_missing_items_get_no_button() {
        let built_in = crate::config::built_in_modes()[0].name.clone();
        assert!(goto_button(ItemKind::Mode, &built_in, true).is_none());
        assert!(goto_button(ItemKind::Macro, "x", false).is_none());
        assert!(goto_button(ItemKind::Macro, "x", true).is_some());
    }
}
