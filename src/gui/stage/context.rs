//! The other overlays, drawn faintly where they go so the one on the stage can be placed around
//! them: the setup's menus, info and log overlays, and the keyboard, numpad, media controls and
//! in-game menu in the look this setup uses. Each can be shown or hidden; at first the info and
//! log overlays are shown, as they can be up with anything. An info or log overlay at the edited
//! one's own spot can't be: on screen the two stack there rather than one sitting over the other.

use iced::widget::{column, stack};

use super::*;
use crate::config::ScreenPosition;

/// How strongly the overlays around the edited one are drawn.
const FAINT: f32 = 0.45;
/// How tall the list of overlays to show gets before it scrolls: about nine of them.
const AROUND_HEIGHT: f32 = 270.0;

impl App {
    /// The overlays that can be drawn around `target`.
    pub(super) fn around_choices(&self, target: StageTarget) -> Vec<StageTarget> {
        let menus = (0..self.menus().len()).map(StageTarget::Menu);
        let infos = (0..self.infos().len()).map(StageTarget::Info);
        let logs = (0..self.logs().len()).map(StageTarget::Log);
        let panels = [Panel::Keyboard, Panel::Numpad, Panel::Media, Panel::InGameMenu]
            .into_iter()
            .filter(|p| !matches!(target, StageTarget::Panel(t) | StageTarget::Setup(t) if t == *p))
            .map(|p| if self.stage_style(StageTarget::Setup(p)).is_some() { StageTarget::Setup(p) } else { StageTarget::Panel(p) });
        menus.chain(infos).chain(logs).chain(panels).filter(|t| *t != target && !self.stacks_with(target, *t)).collect()
    }

    /// What is drawn around an overlay when the stage opens: the info and log overlays.
    pub(super) fn default_around(&self) -> Vec<StageTarget> {
        let infos = (0..self.infos().len()).map(StageTarget::Info);
        infos.chain((0..self.logs().len()).map(StageTarget::Log)).collect()
    }

    /// Whether `other` stacks with `target` at its spot on screen: two info or log overlays there.
    fn stacks_with(&self, target: StageTarget, other: StageTarget) -> bool {
        let stacking = |t: StageTarget| matches!(t, StageTarget::Info(_) | StageTarget::Log(_));
        let spot = |t: StageTarget| self.stage_style(t).map(|s| s.position);
        stacking(target) && stacking(other) && spot(target) == spot(other)
    }

    /// The overlays the stage draws around the edited one, faintly.
    pub(super) fn shown_around(&self, stage: &Stage) -> Vec<StageTarget> {
        let choices = self.around_choices(stage.target);
        choices.into_iter().filter(|t| stage.shows_around(*t)).collect()
    }

    pub(super) fn stage_context<'a>(&self, stage: &Stage, display: iced::Size) -> Element<'a, Message> {
        let faint = Anim { clock: stage.clock(), ..Anim::faint(FAINT) };
        let shown = self.shown_around(stage);
        let drawn = |t: StageTarget| {
            let style = self.stage_style(t)?;
            let fit = Fit::of(style, display);
            let panel = self.stage_panel(t, style, &fit, &faint)?;
            Some((style, fit, panel))
        };
        let mut layers: Vec<Element<'a, Message>> = Vec::new();
        // Info and log overlays at one spot stack up there, placed by the first one's style, as
        // the overlay process draws them; menus and the keyboard go over them.
        let stacking = |t: &&StageTarget| matches!(t, StageTarget::Info(_) | StageTarget::Log(_));
        for spot in ScreenPosition::GRID {
            let here: Vec<_> = shown.iter().filter(stacking).filter_map(|t| drawn(*t)).filter(|(style, ..)| style.position == spot).collect();
            if let Some((style, fit, _)) = here.first() {
                let (style, fit) = ((*style).clone(), *fit);
                let panels = here.into_iter().map(|(.., panel)| panel).collect::<Vec<_>>();
                layers.push(crate::overlay::draw::place(column(panels).spacing(12).into(), &style, &fit, &Anim::still()));
            }
        }
        for (style, fit, panel) in shown.iter().filter(|t| !stacking(t)).filter_map(|t| drawn(*t)) {
            layers.push(crate::overlay::draw::place(panel, style, &fit, &Anim::still()));
        }
        stack(layers).into()
    }

    /// The Layout tab's list of the other overlays, each to show around this one or not.
    pub(super) fn around_picker<'a>(&self, stage: &Stage) -> Element<'a, Message> {
        let choices = self.around_choices(stage.target);
        let all = |label: &'static str, on: bool| button(text(label).size(13)).style(style::secondary).on_press(Message::Stage(StageMessage::AroundAll(on)));
        let mut boxes = column![].spacing(6);
        for t in choices {
            let shown = stage.shows_around(t);
            boxes = boxes.push(checkbox(shown).label(self.stage_title(t)).on_toggle(move |on| Message::Stage(StageMessage::Around(t, on))));
        }
        // Past about nine overlays the list scrolls, so it doesn't push the rest of the page away.
        let scrolling = container(scrollable(container(boxes).padding(iced::Padding::ZERO.right(14)))).max_height(AROUND_HEIGHT);
        let mut list = column![row![all("All", true), all("None", false)].spacing(6), scrolling].spacing(6);
        let stacked = self.stacked_with(stage.target);
        if stacked > 0 {
            let others = if stacked == 1 { "1 more overlay is".to_string() } else { format!("{stacked} more overlays are") };
            list = list.push(note(format!("{others} at this spot and stack with this one on screen, so they aren't drawn around it.")));
        }
        list.into()
    }

    /// How many of the setup's info and log overlays stack with `target` at its spot.
    pub(super) fn stacked_with(&self, target: StageTarget) -> usize {
        let infos = (0..self.infos().len()).map(StageTarget::Info);
        infos.chain((0..self.logs().len()).map(StageTarget::Log)).filter(|t| *t != target && self.stacks_with(target, *t)).count()
    }
}

impl Stage {
    /// Whether `t` is drawn around the edited overlay.
    pub(super) fn shows_around(&self, t: StageTarget) -> bool {
        self.around.iter().any(|a| same_overlay(*a, t))
    }
}

/// Whether `a` and `b` are one overlay. A panel is the same in either look, so it stays shown
/// around the edited one when the setup takes or drops its own.
pub(super) fn same_overlay(a: StageTarget, b: StageTarget) -> bool {
    match (a, b) {
        (StageTarget::Panel(p) | StageTarget::Setup(p), StageTarget::Panel(q) | StageTarget::Setup(q)) => p == q,
        _ => a == b,
    }
}
