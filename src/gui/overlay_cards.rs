//! The App settings cards for the on-screen keyboard, numpad, media controls and in-game menu.

use iced::widget::row;

use super::*;

impl App {
    pub(super) fn view_keyboard_card(&self) -> Element<'_, Message> {
        let keyboard_open = self.status.as_ref().is_some_and(|s| s.overlay_visible);
        let appearance_open = self.open_appearance.contains(&None);
        // Opening it is a line of its own, apart from the Appearance disclosure, which comes last
        // so what it shows opens right under it.
        let mut rows: Vec<Element<'_, Message>> = vec![
            button(text(if keyboard_open { "Close it" } else { "Open it now" }).size(13))
                .style(style::secondary)
                .on_press(Message::ToggleOverlay)
                .into(),
            labeled(
                "Keys",
                row![
                    iced::widget::radio("Basic: what typing into a game needs", false, Some(self.config.full_keyboard), Message::SetFullKeyboard).size(16),
                    iced::widget::radio("Full: function and editing keys too", true, Some(self.config.full_keyboard), Message::SetFullKeyboard).size(16),
                ]
                .spacing(16)
                .into(),
            ),
            appearance_row(appearance_open, Message::ToggleAppearance(None), StageTarget::Panel(stage::Panel::Keyboard)),
        ];
        if appearance_open {
            let style = &self.config.keyboard_style;
            rows.push(style_editor(style, Rc::new(Message::SetKeyboardStyle)));
            rows.extend(self.spot_note(style.position, true));
            let board = if self.config.full_keyboard { crate::keyboard::Board::Full } else { crate::keyboard::Board::Basic };
            let sample = crate::overlay::KeyboardView {
                board,
                style: preview_style(style),
                cursor: crate::keyboard::find(board, "KEY_H").unwrap_or_default(),
                latched: vec!["KEY_LEFTSHIFT".into()],
                pressed: None,
                closing: 0.0,
                caps: false,
                typed: String::new(),
                field: None,
            };
            rows.push(preview(crate::overlay::draw::keyboard_panel(&sample, self.preview_font(), self.glyphs(), &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still())));
        }
        section(
            "On-screen keyboard",
            Some(
                "A keyboard over everything, typed with the controller: D-pad or stick to move, A to \
                 press, X backspace (hold to keep deleting), Y space, Start enter, LB and RB move the text \
                 cursor, hold LT for Shift, hold B or press Guide to close. Keys are labelled in your \
                 keyboard layout. Map \"On-screen keyboard\" to a button or gesture to open it from the controller."
                    .into(),
            ),
            rows,
        )
    }

    pub(super) fn view_media_card(&self) -> Element<'_, Message> {
        let mut rows: Vec<Element<'_, Message>> = vec![appearance_row(self.media_appearance, Message::ToggleMediaAppearance, StageTarget::Panel(stage::Panel::Media))];
        if self.media_appearance {
            let style = &self.config.media_style;
            rows.push(style_editor(style, Rc::new(Message::SetMediaStyle)));
            rows.extend(self.spot_note(style.position, true));
            let sample = crate::media::MediaView::sample(preview_style(style));
            rows.push(preview(crate::overlay::draw::media_panel(&sample, self.preview_font(), self.glyphs(), &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still())));
        }
        section(
            "Media controls",
            Some(
                "The panel for the player you're listening to, shown over the game with Guide + LB. \
                 Pick where it sits and how it looks."
                    .into(),
            ),
            rows,
        )
    }

    pub(super) fn view_in_game_menu_card(&self) -> Element<'_, Message> {
        let mut rows: Vec<Element<'_, Message>> = vec![appearance_row(self.menu_appearance, Message::ToggleInGameMenuAppearance, StageTarget::Panel(stage::Panel::InGameMenu))];
        if self.menu_appearance {
            let style = &self.config.menu_style;
            rows.push(style_editor(style, Rc::new(Message::SetInGameMenuStyle)));
            rows.extend(self.spot_note(style.position, true));
            let sample = crate::system_menu::main_page(preview_style(style), 0);
            rows.push(preview(crate::overlay::draw::menu_panel(&sample, self.preview_font(), self.menu_look(), &crate::overlay::fit::Fit::contents())));
        }
        section(
            "In-game menu",
            Some(
                "The menu Guide + Start opens over a game, with Quick Settings and Edit Controls. Edit \
                 Controls uses this look too. Pick where it sits and how it looks."
                    .into(),
            ),
            rows,
        )
    }

    pub(super) fn view_numpad_card(&self) -> Element<'_, Message> {
        let numpad_open = self.status.as_ref().is_some_and(|s| s.numpad_visible);
        let mut rows: Vec<Element<'_, Message>> = vec![
            button(text(if numpad_open { "Close it" } else { "Open it now" }).size(13))
                .style(style::secondary)
                .on_press(Message::ToggleNumpad)
                .into(),
            appearance_row(self.numpad_appearance, Message::ToggleNumpadAppearance, StageTarget::Panel(stage::Panel::Numpad)),
        ];
        if self.numpad_appearance {
            let style = &self.config.numpad_style;
            rows.push(style_editor(style, Rc::new(Message::SetNumpadStyle)));
            rows.extend(self.spot_note(style.position, true));
            let sample = crate::overlay::KeyboardView {
                board: crate::keyboard::Board::Numpad,
                style: preview_style(style),
                cursor: Layout::Numpad.home(),
                latched: Vec::new(),
                pressed: None,
                closing: 0.0,
                caps: false,
                typed: String::new(),
                field: None,
            };
            rows.push(preview(crate::overlay::draw::keyboard_panel(&sample, self.preview_font(), self.glyphs(), &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still())));
        }
        section(
            "On-screen numpad",
            Some(
                "Digits 0–9 and a dot, typed with the controller: D-pad or stick to move, A to press, \
                 X backspace, Start enter, hold B to close. Map \"On-screen numpad\" to a button or \
                 gesture to open it from the controller."
                    .into(),
            ),
            rows,
        )
    }
}
