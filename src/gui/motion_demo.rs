//! The motion visualizer: a small menu that opens, moves its highlight, picks and closes in the
//! chosen motion style, over and over with a pause between runs, so a style can be judged
//! without opening a real overlay.

use std::time::{Duration, Instant};

use iced::widget::{container, row};

use super::*;
use crate::motion::{Anim, MotionStyle, Timing};

/// When the highlight moves to the second item, the pick lands and the close starts, in
/// seconds into a run.
const MOVE_AT: f32 = 1.2;
const PICK_AT: f32 = 2.0;
const CLOSE_AT: f32 = 2.05;
/// A run, with the still gap at its end before the next one.
const RUN: f32 = 4.2;
/// How big the sample menu is drawn, against its size on screen, so all of it fits the card.
const PREVIEW_SCALE: f32 = 0.6;
/// How often the visualizer redraws.
const FRAME: Duration = Duration::from_millis(40);
/// Stands for "hasn't happened yet" in a [`Timing`].
const NOT_YET: f32 = f32::INFINITY;

/// The visualizer's state: the style it shows and how far into a run it is.
pub(super) struct MotionDemo {
    pub style: MotionStyle,
    started: Instant,
    at: f32,
}

impl MotionDemo {
    pub fn new() -> Self {
        MotionDemo { style: MotionStyle::default(), started: Instant::now(), at: 0.0 }
    }

    /// Moves the run on to `now`.
    pub fn tick(&mut self, now: Instant) {
        self.at = now.saturating_duration_since(self.started).as_secs_f32() % RUN;
    }

    /// Starts a run from the beginning, showing `style`.
    pub fn show(&mut self, style: MotionStyle) {
        *self = MotionDemo { style, started: Instant::now(), at: 0.0 };
    }

    /// Whether the highlight has moved on to the second item.
    fn moved(&self) -> bool {
        self.at >= MOVE_AT
    }

    /// When things happened, as the overlay would see them this far into the run.
    fn timing(&self) -> Timing {
        let since = |at: f32| if self.at >= at { self.at - at } else { NOT_YET };
        Timing {
            opened: self.at,
            closed: (self.at >= CLOSE_AT).then_some(self.at - CLOSE_AT),
            moved: since(MOVE_AT),
            from: self.moved().then_some(0),
            picked: since(PICK_AT),
        }
    }

    fn anim(&self) -> Anim {
        Anim { feel: self.style.feel(), timing: self.timing(), clock: self.at }
    }
}

impl App {
    /// Whether a motion visualizer is on the page shown, so the clock should run.
    pub(super) fn motion_demo_shown(&self) -> bool {
        match self.page {
            Page::Settings => true,
            Page::Game(_) => self.game_tab == GameTab::Details && self.game().motion.is_some(),
            _ => false,
        }
    }

    /// Starts the visualizer on the menu style of the page now shown, so it opens on a style the
    /// page uses rather than the last one previewed elsewhere.
    pub(super) fn sync_motion_demo(&mut self) {
        if !self.motion_demo_shown() {
            return;
        }
        let motion = match self.page {
            Page::Game(_) => self.game().motion.unwrap_or(self.config.motion),
            _ => self.config.motion,
        };
        let style = motion.get(crate::motion::OverlayKind::Menu);
        if style != self.motion_demo.style {
            self.motion_demo.show(style);
        }
    }

    pub(super) fn motion_demo_clock(&self) -> Subscription<Message> {
        if self.motion_demo_shown() { iced::time::every(FRAME).map(Message::MotionTick) } else { Subscription::none() }
    }

    /// The visualizer: a style to preview and the looping menu in it.
    pub(super) fn view_motion_demo(&self) -> Element<'_, Message> {
        let demo = &self.motion_demo;
        let mut look = self.menu_look();
        look.anim = demo.anim();
        let sample = crate::system_menu::main_page(OverlayStyle { scale: PREVIEW_SCALE, ..OverlayStyle::default() }, usize::from(demo.moved()));
        let panel = crate::overlay::draw::menu_panel(&sample, self.preview_font(), look, &crate::overlay::fit::Fit::contents());
        let controls = row![
            dropdown(MotionStyle::ALL, Some(demo.style), Message::MotionDemoStyle).width(160),
            help("A sample menu opens, moves, picks and closes in this style, then pauses and repeats.".into()),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        // Room above and below for a style that slides the panel in from off its place.
        let stage = container(container(panel).center_x(Length::Fill)).width(Length::Fill).padding([30, 8]).clip(true).style(style::inset);
        let body = row![controls, stage].spacing(14).align_y(Alignment::Center);
        iced::widget::column![rule::horizontal(1), labeled("Preview", body.into())].spacing(12).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    fn at(style: MotionStyle, secs: f32) -> MotionDemo {
        MotionDemo { style, started: Instant::now(), at: secs }
    }

    #[test]
    fn a_run_opens_moves_picks_closes_and_goes_quiet() {
        let subtle = |secs| at(MotionStyle::Subtle, secs).anim();
        assert!(subtle(0.0).opacity() < 0.01, "starts closed");
        assert!(subtle(0.5).opacity() > 0.99, "open");
        assert!(subtle(MOVE_AT + 0.01).lit(false, true) > 0.9, "the first item is still lit just as the highlight leaves");
        assert!(subtle(PICK_AT + 0.01).flash() > 0.0 || subtle(PICK_AT + 0.1).flash() > 0.0, "the pick flashes");
        assert!(subtle(RUN - 0.1).opacity() < 0.01, "gone before the next run");
        assert!(!subtle(RUN - 0.1).is_busy() || subtle(RUN - 0.1).opacity() < 0.01);
    }

    #[test]
    fn a_tick_keeps_the_run_within_its_length() {
        let mut demo = MotionDemo::new();
        demo.tick(demo.started + Duration::from_secs_f32(RUN * 3.0 + 0.5));
        assert!((demo.at - 0.5).abs() < 0.01);
    }

    #[test]
    fn the_visualizer_follows_the_style_last_set_and_only_runs_where_it_is_shown() {
        let mut app = with_game();
        let _ = app.update(Message::SelectPage(Page::Settings));
        assert!(app.motion_demo_shown());
        let _ = app.update(Message::SetMotion(crate::motion::OverlayKind::Menu, MotionStyle::Grim));
        assert_eq!(app.motion_demo.style, MotionStyle::Grim);
        let _ = app.update(Message::MotionDemoStyle(MotionStyle::Brutal));
        let _ = app.update(Message::MotionTick(Instant::now()));
        let _ = app.view();
        assert_eq!(app.motion_demo.style, MotionStyle::Brutal);
        let _ = app.update(Message::SelectPage(Page::Manual));
        assert!(!app.motion_demo_shown());
    }
}
