//! What the in-game menu draws: its pages, the keyboard while typing into a field, and the
//! questions it asks over them.

use std::time::Instant;

use super::*;

impl SystemMenu {
    /// Whether the editor's page shows the left stick live, so it doesn't move the cursor.
    pub(super) fn shows_left_stick(&self) -> bool {
        matches!(&self.screen, Screen::Editor(e) if e.live_stick() == Some(crate::config::Stick::Left))
    }

    /// The overlay the editor's page shows on screen where it goes.
    pub fn preview(&self) -> Option<crate::layout_editor::Preview> {
        match &self.screen {
            Screen::Editor(e) => e.preview(),
            _ => None,
        }
    }

    /// Whether the editor's page has a picture that follows the controller, so input and motion
    /// redraw it.
    pub fn live_picture(&self, config: &Config) -> bool {
        matches!(&self.screen, Screen::Editor(e) if e.figure(config, &crate::figures::Live::default()).is_some())
    }

    /// What the overlay draws now: the keyboard while typing into a field, else the menu.
    pub fn overlay_view(&self, config: &Config, live: &crate::figures::Live) -> OverlayView {
        match &self.screen {
            Screen::Typing(_, typing) => {
                let mut view = typing.0.view(Instant::now());
                view.style = config.active_keyboard_style().clone();
                OverlayView::Keyboard(view)
            }
            Screen::Editor(editor) if self.hold.is_none() => {
                let mut view = self.view(config);
                view.figure = editor.figure(config, live).map(Box::new);
                OverlayView::Menu(view)
            }
            _ => OverlayView::Menu(self.view(config)),
        }
    }

    /// The menu the overlay draws. The unsaved-changes question is drawn over the screen it was
    /// asked on.
    pub fn view(&self, config: &Config) -> MenuView {
        let mut view = match &self.screen {
            Screen::Confirm(left, cursor, how) => {
                let mut view = self.screen_view(left, *cursor, config);
                view.prompt = Some(Box::new(self.unsaved_prompt(*how, view.style.clone())));
                view
            }
            screen => self.screen_view(screen, self.cursor, config),
        };
        if let Some(hold) = self.hold {
            view.prompt = Some(Box::new(hold_prompt(hold, view.style.clone())));
        }
        view
    }

    /// The question asked when the menu is left with changes not saved, in the look of the offer
    /// to add a game.
    pub(super) fn unsaved_prompt(&self, how: Exit, mut style: OverlayStyle) -> OfferView {
        // Solid, so the page under it doesn't show through the words.
        style.background.opacity = 1.0;
        let then = if self.from_guide && how == Exit::Back { "go back to the Guide" } else { "close the menu" };
        OfferView {
            title: "Save your changes?".into(),
            detail: "You have unsaved changes.".into(),
            answers: [("{confirm}", "Save"), ("{west}", "Discard"), ("{back}", "Keep editing")]
                .iter()
                .map(|(b, t)| ((*b).into(), (*t).into()))
                .collect(),
            hint: format!("Save or Discard to {then}."),
            style,
        }
    }

    pub(super) fn screen_view(&self, screen: &Screen, cursor: usize, config: &Config) -> MenuView {
        let style = config.active_menu_style().clone();
        match screen {
            Screen::Main => main_page(style, cursor),
            Screen::Editor(editor) | Screen::Typing(editor, _) => editor.view(config, &[self.parent()]),
            Screen::Confirm(left, cursor, _) => self.screen_view(left, *cursor, config),
            Screen::Profiles => list_page(
                "Switch Profile",
                profiles::rows(config).into_iter().map(|r| profiles::item(r, config, self.features.as_deref())).collect(),
                cursor,
                vec![self.parent()],
                style,
            ),
            Screen::NewProfile => list_page(
                "New profile",
                profiles::new_rows(config).into_iter().map(|r| profiles::new_item(r, config)).collect(),
                cursor,
                vec![self.parent(), "Switch Profile".into()],
                style,
            ),
            Screen::Quick => MenuView {
                title: "Quick Settings".into(),
                kind: crate::config::MenuKind::List,
                items: quick::rows(config, &self.saved)
                    .into_iter()
                    .map(|row| {
                        let label = match row {
                            Row::Remapping if self.remapping_armed => "Press {confirm} again to turn remapping off".into(),
                            row => quick::label(row, config, &self.saved),
                        };
                        ItemView { tone: if row == Row::Remapping { Tone::Remove } else { Tone::Normal }, label, ..ItemView::plain() }
                    })
                    .collect(),
                selected: Some(cursor),
                picks: 0,
                crumbs: vec![self.parent()],
                hint: "{leftright} or {confirm} change · hold {rb} for big steps · {back} back".into(),
                about: String::new(),
                style,
                touch: false,
                aim: None,
                prompt: None,
                one_column: false,
                figure: None,
            },
        }
    }
}

/// A page that is a plain list: {confirm} chooses, {back} goes back.
fn list_page(title: &str, items: Vec<ItemView>, cursor: usize, crumbs: Vec<String>, style: OverlayStyle) -> MenuView {
    MenuView {
        title: title.into(),
        kind: crate::config::MenuKind::List,
        items,
        selected: Some(cursor),
        picks: 0,
        crumbs,
        hint: "{confirm} choose · {back} back".into(),
        about: String::new(),
        style,
        touch: false,
        aim: None,
        prompt: None,
        one_column: false,
        figure: None,
    }
}

/// What the menu shows while on hold, in the look of the unsaved-changes question.
fn hold_prompt(hold: Hold, mut style: OverlayStyle) -> OfferView {
    style.background.opacity = 1.0;
    let (title, detail, answers, hint) = match hold {
        Hold::RemappingOff => (
            "Remapping is off",
            "It was turned off while you had unsaved changes here.",
            vec![("{confirm}", "Save"), ("{west}", "Discard")],
            "Save or Discard to hand the controller back to the game.",
        ),
    };
    OfferView {
        title: title.into(),
        detail: detail.into(),
        answers: answers.into_iter().map(|(b, t)| (b.into(), t.into())).collect(),
        hint: hint.into(),
        style,
    }
}

/// The menu's first page, drawn with `style`; the settings previews show it too.
pub fn main_page(style: OverlayStyle, cursor: usize) -> MenuView {
    MenuView {
        title: TITLE.into(),
        kind: crate::config::MenuKind::List,
        items: vec![
            ItemView { label: "Quick Settings".into(), button: None, submenu: true, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
            ItemView { label: "Edit Controls".into(), button: None, submenu: false, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
        ],
        selected: Some(cursor),
        picks: 0,
        crumbs: Vec::new(),
        hint: "{confirm} choose · {back} close".into(),
        about: String::new(),
        style,
        touch: false,
        aim: None,
        prompt: None,
        one_column: false,
        figure: None,
    }
}
