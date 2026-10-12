//! The font dropdowns: each name is set in its own font, so the choice can be judged by eye.
//! A pick list draws every entry in one font, so this is a button that opens a list below it.

use iced::widget::{button, column, container, row, scrollable, space, text};

use super::*;

/// Which font dropdown is meant: the App settings page's or a setup's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FontSlot {
    Global,
    Game,
}

/// The names to offer: the fonts the app knows, then `current` if it is something else (set by
/// a pack, say).
fn font_names(current: Option<&str>) -> Vec<String> {
    let mut names = crate::font::names();
    if let Some(c) = current.filter(|c| !names.iter().any(|n| n == c)) {
        names.push(c.to_string());
    }
    names
}

impl App {
    /// A dropdown of fonts: `unset` is the entry for no font, and `on_pick` makes the message
    /// for a choice (`None` for that entry).
    pub(super) fn font_picker<'a>(&'a self, slot: FontSlot, unset: &'static str, current: Option<&'a str>, on_pick: fn(Option<String>) -> Message) -> Element<'a, Message> {
        let shown = text(current.unwrap_or(unset).to_string()).font(crate::font::resolve(current)).width(Length::Fill);
        let open = self.font_menu == Some(slot);
        let head = button(row![shown, text(if open { "▴" } else { "▾" }).size(12)].spacing(8).align_y(Alignment::Center))
            .style(style::secondary)
            .padding([6, 10])
            .width(SETTING_WIDTH)
            .on_press(Message::ToggleFontMenu(slot));
        if !open {
            return head.into();
        }
        let entry = |label: String, font: iced::Font, on: bool, pick: Option<String>| {
            let b = button(text(label).font(font)).width(Length::Fill).padding([5, 10]).on_press(on_pick(pick));
            if on { b.into() } else { Element::from(b.style(style::secondary)) }
        };
        let mut list = column![entry(unset.to_string(), iced::Font::DEFAULT, current.is_none(), None)].spacing(2);
        for name in font_names(current) {
            let on = current == Some(name.as_str());
            list = list.push(entry(name.clone(), crate::font::resolve(Some(&name)), on, Some(name)));
        }
        let menu = container(scrollable(list.padding(4)).height(Length::Fixed(260.0))).width(SETTING_WIDTH).style(style::inset);
        column![head, menu, space().height(2)].spacing(2).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn the_list_offers_the_known_fonts_and_a_pack_s_odd_one() {
        assert_eq!(font_names(Some("Fira")).len(), crate::font::names().len() + 1);
        assert_eq!(font_names(Some("Inter")).len(), crate::font::names().len());
    }

    #[test]
    fn a_font_menu_opens_closes_and_closes_on_a_pick() {
        let mut app = with_game();
        let _ = app.update(Message::ToggleFontMenu(FontSlot::Global));
        assert_eq!(app.font_menu, Some(FontSlot::Global));
        let _ = app.update(Message::SelectPage(Page::Settings));
        let _ = app.view();
        let _ = app.update(Message::SetOverlayFont(Some("Cinzel".into())));
        assert_eq!(app.font_menu, None);
        let _ = app.update(Message::ToggleFontMenu(FontSlot::Global));
        let _ = app.update(Message::ToggleFontMenu(FontSlot::Global));
        assert_eq!(app.font_menu, None);
    }
}
