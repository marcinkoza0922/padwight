//! The guided tour: a card in the corner that walks a new user through the window, showing each
//! page it talks about. It starts the first time the window opens, and App settings can start it
//! again.

use iced::widget::{column, row};

use super::*;

/// Moves through the tour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TourMove {
    Start,
    Next,
    Back,
    /// Skipped, or finished.
    End,
}

/// What a step shows behind its card.
enum Place {
    Overview,
    /// General's Buttons tab.
    General,
    Settings,
    /// The Manual, open at this chapter's file.
    Chapter(&'static str),
}

struct Step {
    title: &'static str,
    body: &'static str,
    place: Place,
}

const STEPS: &[Step] = &[
    Step {
        title: "Welcome to Padwight",
        body: "Padwight runs in the background and changes what your controller does: buttons to keys, a stick \
               to the mouse, a different layout for each game.\n\nThis is the Overview: your controller with what \
               each button does right now, your latest presses, and the controllers Padwight has found.",
        place: Place::Overview,
    },
    Step {
        title: "Setups and profiles",
        body: "Each game can have a setup; General is used when none matches. A setup holds profiles: \
               complete layouts you can switch between.\n\nClick a button's row to change what it does, or \
               click Find by pressing and press the button on your controller.",
        place: Place::General,
    },
    Step {
        title: "Save & apply",
        body: "Edits take effect when you press Save & apply at the bottom of the window. Revert discards \
               them, and Ctrl+Z undoes the last one.",
        place: Place::General,
    },
    Step {
        title: "Your games",
        body: "+ Add setup in the sidebar lists the setups that come with Padwight, installed games first. \
               Once added, a game's setup takes over whenever the game is in front.",
        place: Place::General,
    },
    Step {
        title: "In the game",
        body: "Tap Guide (the Xbox, PlayStation or Home button) to see what every button does. If Padwight has \
               a setup for the game in front that you haven't added, Guide offers to add it.\n\nPress Guide and \
               Start together for the in-game menu, to switch profiles and change settings without leaving the \
               game.",
        place: Place::Chapter("guide-button.md"),
    },
    Step {
        title: "App settings",
        body: "The window's theme, automatic switching, the default profile, and how the overlays look. You can \
               take this tour again from the Help card on this page.",
        place: Place::Settings,
    },
    Step {
        title: "The Manual",
        body: "The Manual explains everything Padwight does, and its search box finds any topic. Enjoy your \
               games!",
        place: Place::Chapter("welcome.md"),
    },
];

/// Marks that the tour has been seen, beside the config file.
fn seen_path() -> std::path::PathBuf {
    Config::path().with_file_name("tour-seen")
}

/// Whether the tour should start by itself: it hasn't been shown on this computer yet.
pub(super) fn first_run() -> bool {
    !cfg!(test) && !seen_path().exists()
}

/// Remembers that the tour has been shown, so it doesn't start by itself again.
fn mark_seen() {
    if cfg!(test) {
        return;
    }
    let path = seen_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, "");
}

impl App {
    pub(super) fn move_tour(&mut self, step: TourMove) -> Task<Message> {
        let at = match (step, self.tour) {
            (TourMove::Start, _) => Some(0),
            (TourMove::Next, Some(i)) if i + 1 < STEPS.len() => Some(i + 1),
            (TourMove::Back, Some(i)) => Some(i.saturating_sub(1)),
            _ => None,
        };
        self.tour = at;
        let Some(i) = at else { return Task::none() };
        mark_seen();
        self.show_place(&STEPS[i].place)
    }

    /// Shows what a step is about behind its card.
    fn show_place(&mut self, place: &Place) -> Task<Message> {
        self.dialog = None;
        self.picker = None;
        self.action_picker = None;
        match place {
            Place::Overview | Place::Settings => {
                self.page = if matches!(place, Place::Overview) { Page::Overview } else { Page::Settings };
                self.reset_page_state();
            }
            Place::General => {
                if self.page != Page::Game(None) {
                    self.show_game(None);
                }
                self.game_tab = GameTab::Profiles;
                self.profile_tab = ProfileTab::Buttons;
            }
            Place::Chapter(file) => {
                self.page = Page::Manual;
                self.manual_chapter = manual::chapter_of(file).unwrap_or(0);
                self.reset_page_state();
            }
        }
        iced::widget::operation::snap_to("main", scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) })
    }

    /// The tour's card for step `i`, in the bottom right corner above the Save bar. Only the card
    /// takes clicks, so the page behind it can still be used.
    pub(super) fn view_tour(&self, i: usize) -> Element<'_, Message> {
        let step = &STEPS[i];
        let last = i + 1 == STEPS.len();
        let mut buttons = row![
            button(text("Skip tour").size(14)).style(button::text).on_press(Message::Tour(TourMove::End)),
            space::horizontal(),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if i > 0 {
            buttons = buttons.push(button(text("Back").size(14)).style(style::secondary).on_press(Message::Tour(TourMove::Back)));
        }
        let next = if last { (text("Done").size(14), TourMove::End) } else { (text("Next").size(14), TourMove::Next) };
        buttons = buttons.push(button(next.0).style(style::primary).on_press(Message::Tour(next.1)));
        let card = column![
            text(format!("Tour · {} of {}", i + 1, STEPS.len())).size(12).color(style_accent()),
            text(step.title).size(20),
            text(step.body).size(14),
            buttons,
        ]
        .spacing(10);
        let card = container(card).width(400).padding(18).style(style::dialog);
        container(opaque(card))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
            .padding(iced::Padding { top: 20.0, right: 24.0, bottom: 72.0, left: 20.0 })
            .into()
    }

    /// App settings' Help card, which starts the tour again.
    pub(super) fn view_help_card(&self) -> Element<'_, Message> {
        section(
            "Help",
            Some(
                "System check looks at the permissions Padwight needs to create its virtual gamepad, read your \
                 controller and use its gyro (or run padwight doctor in a terminal). The tour is the short walk \
                 through the window shown on first start."
                    .into(),
            ),
            vec![
                self.system_check_row(),
                button(text("Take the tour")).style(style::secondary).on_press(Message::Tour(TourMove::Start)).into(),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tour_walks_its_steps_and_ends() {
        let mut app = super::super::tests::app();
        assert_eq!(app.tour, None, "tests never start the tour by themselves");
        let _ = app.update(Message::Tour(TourMove::Start));
        assert_eq!((app.tour, &app.page), (Some(0), &Page::Overview));
        let _ = app.update(Message::Tour(TourMove::Back));
        assert_eq!(app.tour, Some(0), "Back on the first step stays there");
        for i in 1..STEPS.len() {
            let _ = app.update(Message::Tour(TourMove::Next));
            assert_eq!(app.tour, Some(i));
            let _ = app.view();
        }
        assert_eq!(app.page, Page::Manual);
        let _ = app.update(Message::Tour(TourMove::Back));
        assert_eq!(app.page, Page::Settings);
        let _ = app.update(Message::Tour(TourMove::Next));
        let _ = app.update(Message::Tour(TourMove::Next));
        assert_eq!(app.tour, None, "Next on the last step ends it");
        let _ = app.update(Message::Tour(TourMove::Start));
        let _ = app.update(Message::Tour(TourMove::End));
        assert_eq!(app.tour, None);
    }

    #[test]
    fn every_chapter_the_tour_opens_exists() {
        for step in STEPS {
            if let Place::Chapter(file) = step.place {
                assert!(manual::chapter_of(file).is_some(), "{file}");
            }
        }
    }
}
