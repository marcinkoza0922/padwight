//! The Overview's "What's happening": the latest presses of the controller in use with what each
//! did, beside what Padwight sent the game for them (gamepad buttons, keys, mouse buttons), and the profile switches the daemon made,
//! each with the reason for it.

use iced::widget::{column, row};

use super::*;
use crate::inputlog::LogView;

/// Switches listed; the daemon keeps more.
const SHOWN_SWITCHES: usize = 8;

/// How long ago a switch was, in words.
fn ago(ms: u64) -> String {
    let secs = ms / 1000;
    match secs {
        0..=4 => "just now".into(),
        5..=59 => format!("{secs} s ago"),
        60..=3599 => format!("{} min ago", secs / 60),
        _ => format!("{} h ago", secs / 3600),
    }
}

impl App {
    /// One feed as a box: its lines, or a hint while there are none.
    fn feed_box<'a>(&self, feed: Option<&LogView>, hint: &'static str) -> Element<'a, Message> {
        match feed {
            Some(feed) if !feed.lines.is_empty() => crate::overlay::draw::log_panel(feed, self.preview_font(), &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still()),
            _ => text(hint).size(13).color(MUTED_COLOR).into(),
        }
    }

    /// The heading of the presses feed, naming the controller it follows: only inputs from that one show.
    fn presses_heading(&self) -> String {
        let name = self.feeds.as_ref().map(|f| f.controller.as_str()).filter(|n| !n.is_empty());
        match (self.picked_controller(), name) {
            (Some(d), _) => format!("Pressed on {}", d.name),
            (None, Some(n)) => format!("Pressed on {n}"),
            (None, None) => "Pressed on the controller".into(),
        }
    }

    pub(super) fn view_activity(&self) -> Element<'_, Message> {
        // The picture above already asks for a press, so the feeds only say they're empty.
        let running = self.status.is_some();
        let presses = self.feed_box(self.feeds.as_ref().map(|f| &f.presses), if running { "Nothing pressed yet." } else { NEEDS_DAEMON });
        let exports = self.feed_box(self.feeds.as_ref().map(|f| &f.exports), if running { "Nothing sent yet." } else { NEEDS_DAEMON });

        let switches = self.status.as_ref().map(|s| s.switches.as_slice()).unwrap_or_default();
        let mut list = column![].spacing(10);
        if switches.is_empty() {
            let none = if self.status.is_some() { "No profile switches yet." } else { NEEDS_DAEMON };
            list = list.push(text(none).size(13).color(MUTED_COLOR));
        }
        for s in switches.iter().take(SHOWN_SWITCHES) {
            let setup = s.game.as_deref().unwrap_or("General");
            list = list.push(
                row![
                    text(ago(s.age_ms)).size(12).color(MUTED_COLOR).width(Length::Fixed(90.0)),
                    column![text(format!("{setup} › {}", s.profile)), text(&s.why).size(12).color(MUTED_COLOR)]
                        .spacing(2)
                        .width(Length::Fill),
                ]
                .spacing(12),
            );
        }

        column![
            row![
                text("What's happening").size(20),
                help(
                    "Left: the latest presses on the controller picked in the header (or the last one used, with \
                     All controllers), one line per burst, and what each did if it's remapped. Middle: what Padwight \
                     sent the game, labeled as the game's controller shows it, plus keys, mouse buttons and wheel \
                     turns. Right: each profile switch and the rule or action behind it."
                        .into(),
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            // Three feeds side by side, so none sits alone under the others.
            row![
                column![text(self.presses_heading()).size(14), presses].spacing(6).width(Length::Fill),
                column![text("Sent to the game").size(14), exports].spacing(6).width(Length::Fill),
                column![text("Profile switches").size(14), list].spacing(6).width(Length::Fill),
            ]
            .spacing(16),
        ]
        .spacing(10)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::tests::*,
        inputlog::{LogCell, LogLine},
        ipc::{Feeds, SwitchEvent},
    };

    #[test]
    fn ages_read_in_words() {
        assert_eq!(ago(0), "just now");
        assert_eq!(ago(42_000), "42 s ago");
        assert_eq!(ago(125_000), "2 min ago");
        assert_eq!(ago(7_300_000), "2 h ago");
    }

    #[test]
    fn overview_shows_the_feed_and_the_switches_with_their_reasons() {
        let mut app = app();
        app.status = Some(Status {
            switches: vec![SwitchEvent {
                age_ms: 12_000,
                game: Some("Doom".into()),
                profile: "Play".into(),
                why: "Doom matches Executable “doom.exe”".into(),
            }],
            ..Default::default()
        });
        let cell = LogCell { glyph: Vec::new(), label: Some("Fire".into()), hold_ms: Some(80), count: 1, held: false };
        let feed = LogView {
            name: "feed".into(),
            style: OverlayStyle::default(),
            opacity: 1.0,
            heading: String::new(),
            lines: vec![LogLine { opacity: 1.0, cells: vec![cell] }],
            show_labels: true,
            show_holds: true,
        };
        let feeds = Feeds { controller: "Pad".into(), presses: feed.clone(), exports: feed.clone() };
        let _ = app.update(Message::LiveFeed(Some(Box::new(feeds.clone()))));
        assert_eq!(app.feeds, Some(feeds));
        let _ = app.view();

        let _ = app.update(Message::LiveFeed(None));
        assert_eq!(app.feeds, None);
        let _ = app.view();
    }
}
