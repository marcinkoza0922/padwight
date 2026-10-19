//! The token picker: a window that lists every `{token}` and formatting tag by group, each with
//! a preview of what it shows, and puts the one clicked into the text it was opened from.
//! Buttons are drawn for every kind of controller, and every icon is shown.

use std::collections::HashSet;

use iced::widget::{button, center, column, container, mouse_area, opaque, row, scrollable, space, text};

use super::*;
use crate::info::{Entry, Fold, PadFamily};

/// The text a token goes into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TokenTarget {
    InfoCell(usize, usize, usize),
    InfoTitle(usize),
    MenuLabel(usize, usize),
    /// The heading of the mode or layer being edited.
    LayerHeading,
}

/// The open picker.
pub(super) struct TokenPicker {
    target: TokenTarget,
    /// The folds that are open.
    open: HashSet<&'static str>,
    query: String,
}

/// `piece` after `text`: a token joined as `join_token` does, or any other text just as it is.
fn append(text: &mut String, piece: &str, spaced: bool) {
    if spaced {
        join_token(text, piece);
    } else {
        text.push_str(piece);
    }
}

/// `token` after `text`, with a space between unless the text ends in one or the token ends a tag.
pub(super) fn join_token(text: &mut String, token: &str) {
    if !text.is_empty() && !text.ends_with(' ') && !token.starts_with("{/") {
        text.push(' ');
    }
    text.push_str(token);
}

/// The small button that opens the picker for `target`.
pub(super) fn insert_button<'a>(target: TokenTarget) -> Element<'a, Message> {
    tooltip(
        button(text("Insert…").size(13)).style(style::secondary).on_press(Message::OpenTokenPicker(target)),
        container(text("Buttons, live values, icons and text formatting").size(13)).padding(8).style(style::tooltip),
        tooltip::Position::Top,
    )
    .into()
}

impl App {
    pub(super) fn update_tokens(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenTokenPicker(target) => {
                self.token_picker = Some(TokenPicker { target, open: HashSet::from(["buttons"]), query: String::new() });
            }
            Message::CloseTokenPicker => self.token_picker = None,
            Message::ToggleTokenFold(id) => {
                if let Some(p) = &mut self.token_picker
                    && !p.open.remove(id)
                {
                    p.open.insert(id);
                }
            }
            Message::TokenSearch(query) => {
                if let Some(p) = &mut self.token_picker {
                    p.query = query;
                }
            }
            Message::PickToken(token) => {
                if let Some(target) = self.token_picker.as_ref().map(|p| p.target.clone()) {
                    self.insert_text(&target, &token, true);
                }
            }
            other => return self.update_sound_picker(other),
        }
        Task::none()
    }

    /// Puts `piece` at the end of the text `target` names, spaced from it like a token if `spaced`.
    pub(super) fn insert_text(&mut self, target: &TokenTarget, piece: &str, spaced: bool) {
        match *target {
            TokenTarget::InfoCell(i, r, c) => {
                if let Some(cell) = self.infos_mut().get_mut(i).and_then(|o| o.rows.get_mut(r)).and_then(|row| row.get_mut(c)) {
                    append(cell, piece, spaced);
                }
            }
            TokenTarget::InfoTitle(i) => {
                if let Some(o) = self.infos_mut().get_mut(i) {
                    append(o.title.get_or_insert_with(String::new), piece, spaced);
                }
            }
            TokenTarget::MenuLabel(m, i) => {
                if let Some(item) = self.menus_mut().get_mut(m).and_then(|m| m.items.get_mut(i)) {
                    append(&mut item.label, piece, spaced);
                }
            }
            TokenTarget::LayerHeading => {
                if let Some(l) = self.edited_mut() {
                    append(l.indicator_title.get_or_insert_with(String::new), piece, spaced);
                }
            }
        }
    }

    /// The text `target` names, as it is now.
    pub(super) fn token_target_text(&self, target: &TokenTarget) -> String {
        match *target {
            TokenTarget::InfoCell(i, r, c) => self.game().info.get(i).and_then(|o| o.rows.get(r)).and_then(|row| row.get(c)).cloned(),
            TokenTarget::InfoTitle(i) => self.game().info.get(i).and_then(|o| o.title.clone()),
            TokenTarget::MenuLabel(m, i) => self.game().menus.get(m).and_then(|m| m.items.get(i)).map(|item| item.label.clone()),
            TokenTarget::LayerHeading => self.edited().and_then(|l| l.indicator_title.clone()),
        }
        .unwrap_or_default()
    }

    pub(super) fn view_token_picker<'a>(&'a self, picker: &'a TokenPicker) -> Element<'a, Message> {
        let font = self.preview_font();
        let current = self.token_target_text(&picker.target);
        let family = self.config.info_glyphs;
        let mine = if current.is_empty() {
            text("(empty)").size(13).color(MUTED_COLOR).into()
        } else {
            chip(&current, family, font)
        };
        let query = picker.query.trim().to_lowercase();
        let mut folds = column![].spacing(10);
        for fold in crate::info::folds() {
            let entries: Vec<&Entry> = fold.entries.iter().filter(|e| query.is_empty() || e.insert.to_lowercase().contains(&query) || e.about.to_lowercase().contains(&query)).collect();
            if entries.is_empty() {
                continue;
            }
            let open = !query.is_empty() || picker.open.contains(fold.id);
            let mut block = column![row![
                disclosure(&format!("{} ({})", fold.title, entries.len()), open, Message::ToggleTokenFold(fold.id)),
                text(fold.blurb).size(12).color(MUTED_COLOR)
            ]
            .spacing(10)
            .align_y(Alignment::Center)]
            .spacing(6);
            if open {
                block = block.push(fold_body(&fold, &entries, family, font));
            }
            folds = folds.push(container(block).padding(10).width(Length::Fill).style(style::inset));
        }
        let dialog = container(
            column![
                text("Insert a token").size(20),
                row![text("Your text").size(13).color(MUTED_COLOR), mine].spacing(10).align_y(Alignment::Center),
                field("Search tokens", &picker.query).on_input(Message::TokenSearch).width(Length::Fill),
                scrollable(folds).height(Length::Fixed(430.0)),
                row![
                    text("Each click adds the token to the end of your text.").size(12).color(MUTED_COLOR),
                    space::horizontal(),
                    button(text("Done")).on_press(Message::CloseTokenPicker),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(12),
        )
        .padding(20)
        .width(Length::Fixed(820.0))
        .style(style::dialog);
        let backdrop = container(center(opaque(dialog)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style { background: Some(Color { a: 0.6, ..Color::BLACK }.into()), ..container::Style::default() });
        opaque(mouse_area(backdrop).on_press(Message::CloseTokenPicker))
    }
}

/// What `text` looks like on screen, for beside the field that holds it, or `None` when it has no
/// `{token}` and reads the same in the field.
pub(super) fn token_preview<'a>(text: &str, family: PadFamily, font: iced::Font) -> Option<Element<'a, Message>> {
    text.contains('{').then(|| chip(text, family, font))
}

/// A preview chip: what `cell` shows on a controller of `family`.
fn chip<'a>(cell: &str, family: PadFamily, font: iced::Font) -> Element<'a, Message> {
    crate::overlay::draw::chip(&crate::info::preview(cell, family), font)
}

/// A fold's entries: a table, a grid of icons, or a list.
fn fold_body<'a>(fold: &Fold, entries: &[&Entry], family: PadFamily, font: iced::Font) -> Element<'a, Message> {
    if fold.icons {
        let cards = entries.iter().map(|e| {
            let name = e.insert.trim_start_matches("{icon:").trim_end_matches('}').to_string();
            let own = e.about == "This game's own";
            let label = if own { format!("{name} ★") } else { name };
            button(column![chip(&e.insert, family, font), text(label).size(11)].spacing(4).align_x(Alignment::Center))
                .style(style::secondary)
                .padding(6)
                .width(Length::Fixed(96.0))
                .on_press(Message::PickToken(e.insert.clone()))
                .into()
        });
        let mut grid = column![iced::widget::row(cards).spacing(6).wrap().vertical_spacing(6)].spacing(6);
        grid = grid.push(text("★ carried by this game's pack: it works in this game only. Add your own under Details → Pack files.").size(12).color(MUTED_COLOR));
        return grid.into();
    }
    let families = fold.families();
    let mut body = column![].spacing(8);
    if !families.is_empty() {
        let mut head = row![space().width(Length::Fixed(190.0))].spacing(8);
        for f in families {
            head = head.push(text(f.to_string()).size(12).color(MUTED_COLOR).width(Length::Fixed(110.0)));
        }
        body = body.push(head);
    }
    for e in entries {
        let mut line = row![text(e.insert.clone()).font(iced::Font::MONOSPACE).size(13).width(Length::Fixed(190.0))].spacing(8).align_y(Alignment::Center);
        if families.is_empty() {
            let shown: Element<'a, Message> = if e.sample.is_empty() { text("—").color(MUTED_COLOR).into() } else { chip(&e.sample, family, font) };
            line = line.push(container(shown).width(Length::Fixed(150.0)));
        } else {
            for f in families {
                line = line.push(container(chip(&e.sample, *f, font)).width(Length::Fixed(110.0)));
            }
        }
        if families.is_empty() {
            line = line.push(text(e.about).size(12).color(MUTED_COLOR).width(Length::Fill));
        } else {
            line = line.push(space::horizontal());
        }
        line = line.push(button(text("Insert").size(13)).style(style::secondary).on_press(Message::PickToken(e.insert.clone())));
        let mut item = column![line].spacing(2);
        if !families.is_empty() {
            item = item.push(row![space().width(Length::Fixed(190.0)), text(e.about).size(12).color(MUTED_COLOR)].spacing(8));
        }
        body = body.push(item);
    }
    body.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn tokens_are_added_to_a_cell_a_title_and_a_menu_label() {
        let mut app = with_game();
        let _ = app.update(Message::NewInfo);
        let _ = app.update(Message::SetInfoCell(0, 0, 0, "Jump".into()));
        for (target, token) in [
            (TokenTarget::InfoCell(0, 0, 0), "{south}"),
            (TokenTarget::InfoCell(0, 0, 0), "{b}"),
            (TokenTarget::InfoCell(0, 0, 0), "{/b}"),
            (TokenTarget::InfoTitle(0), "{icon:bow}"),
        ] {
            let _ = app.update(Message::OpenTokenPicker(target));
            let _ = app.update(Message::PickToken(token.into()));
        }
        assert_eq!(app.game().info[0].rows[0][0], "Jump {south} {b}{/b}");
        assert_eq!(app.game().info[0].title.as_deref(), Some("{icon:bow}"));
        let _ = app.update(Message::CloseTokenPicker);
        assert!(app.token_picker.is_none());

        let _ = app.update(Message::NewMenu(crate::config::MenuKindTag::List));
        let _ = app.update(Message::OpenTokenPicker(TokenTarget::MenuLabel(0, 0)));
        let _ = app.update(Message::PickToken("{icon:potion}".into()));
        assert!(app.game().menus[0].items[0].label.ends_with("{icon:potion}"));
    }

    #[test]
    fn the_picker_lists_every_fold_and_the_game_s_own_icons() {
        let mut app = with_game();
        app.game_mut().assets.add_icon("flame", "<svg viewBox='0 0 8 8'></svg>").unwrap();
        let _ = app.update(Message::NewInfo);
        let _ = app.update(Message::OpenTokenPicker(TokenTarget::InfoCell(0, 0, 0)));
        for fold in crate::info::folds() {
            let _ = app.update(Message::ToggleTokenFold(fold.id));
        }
        let _ = app.view();
        let _ = app.update(Message::TokenSearch("gyro".into()));
        let _ = app.view();
        assert!(crate::info::folds().iter().any(|f| f.icons && f.entries.iter().any(|e| e.insert == "{icon:flame}")));
    }

    #[test]
    fn a_token_joins_what_is_there_with_a_space_unless_it_closes_a_tag() {
        let mut text = String::new();
        join_token(&mut text, "{b}");
        join_token(&mut text, "{/b}");
        assert_eq!(text, "{b}{/b}");
        join_token(&mut text, "{south}");
        assert_eq!(text, "{b}{/b} {south}");
    }
}

