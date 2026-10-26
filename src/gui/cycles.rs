//! The Cycles tab: lists of steps one input steps through, a step per press, for a game's states
//! that rule each other out (game speeds, stances).

use iced::widget::{column, row};

use super::*;
use crate::config::{
    Cycle,
    problems::{cycle_step_problem, cycles_problem},
};

impl App {
    fn cycles(&self) -> &Vec<Cycle> {
        &self.game().cycles
    }

    fn cycles_mut(&mut self) -> &mut Vec<Cycle> {
        &mut self.game_mut().cycles
    }

    pub(super) fn cycle_names(&self) -> Vec<&String> {
        self.cycles().iter().map(|c| &c.name).collect()
    }

    pub(super) fn cycles_have_problem(&self, names: &Names) -> bool {
        cycles_problem(self.cycles(), names).is_some()
    }

    pub(super) fn update_cycles(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetAction(Target::CycleStep(c, s), action) => {
                if let Some(step) = self.cycles_mut().get_mut(c).and_then(|c| c.steps.get_mut(s)) {
                    *step = action;
                }
            }
            Message::ToggleCycle(i) => {
                if !self.open_cycles.remove(&i) {
                    self.open_cycles.insert(i);
                }
            }
            Message::NewCycle => {
                let name = self.new_item_name(ItemKind::Cycle, "Cycle");
                let steps = vec![ButtonAction::Keys(Vec::new()), ButtonAction::Keys(Vec::new())];
                // New ones go first, right under the button that made them, already open.
                self.cycles_mut().insert(0, Cycle { name, steps });
                self.move_open_cards(ItemKind::Cycle, open_cards::after_inserting(1));
                self.open_cycles.insert(0);
            }
            Message::DeleteCycle(i) => {
                if i < self.cycles().len() {
                    self.cycles_mut().remove(i);
                    self.move_open_cards(ItemKind::Cycle, open_cards::after_removing(i));
                }
            }
            Message::RenameCycle(i, name) => {
                if self.item_name_free(ItemKind::Cycle, Some(i), &name)
                    && let Some(c) = self.cycles_mut().get_mut(i)
                {
                    let old = std::mem::replace(&mut c.name, name.clone());
                    // Keep every mapping of this cycle pointing at it.
                    self.follow_item_rename(ItemKind::Cycle, &old, &name);
                }
            }
            Message::AddCycleStep(i) => {
                if let Some(c) = self.cycles_mut().get_mut(i) {
                    c.steps.push(ButtonAction::Keys(Vec::new()));
                }
            }
            Message::MoveCycleStep(c, i, up) => {
                if let Some(c) = self.cycles_mut().get_mut(c) {
                    let j = if up { i.checked_sub(1) } else { Some(i + 1).filter(|j| *j < c.steps.len()) };
                    if let Some(j) = j {
                        c.steps.swap(i, j);
                    }
                }
            }
            Message::RemoveCycleStep(c, i) => {
                if let Some(c) = self.cycles_mut().get_mut(c)
                    && i < c.steps.len()
                {
                    c.steps.remove(i);
                }
            }
            other => return self.update_logs(other),
        }
        Task::none()
    }

    pub(super) fn view_cycles(&self, names: &Names) -> Element<'_, Message> {
        let add = items_header(
            ItemKind::Cycle,
            button(text("+ New cycle")).style(style::secondary).on_press(Message::NewCycle).into(),
            "A cycle presses the next of its steps each time, starting over after the last.",
            "A cycle switches between a game's states that rule each other out, such as game speeds or \
             standing, crouching and lying down, from one button. Each press presses its next step: a key, \
             a gamepad button, a macro, or several inputs at once. After the last step it starts over at the \
             first. Map it with the \"Cycle…\" action; another button with \"Step back\" ticked goes the other way."
                .into(),
            self.cycles().is_empty().then_some("This setup has no cycles yet."),
        );
        let mut col = column![add].spacing(16);
        for (i, c) in self.cycles().iter().enumerate() {
            col = col.push(self.view_cycle_card(i, c, names));
        }
        col.into()
    }

    /// A cycle as its own collapsible card: a summary line, or its steps when open.
    fn view_cycle_card<'a>(&'a self, ci: usize, c: &'a Cycle, names: &Names) -> Element<'a, Message> {
        let open = self.open_cycles.contains(&ci);
        let problem = self.name_problem(ItemKind::Cycle, ci, &c.name).or_else(|| c.steps.iter().find_map(|s| cycle_step_problem(s, names)));
        let title = text(if c.name.is_empty() { "(unnamed)" } else { &c.name }).size(18);
        let title = if problem.is_some() { title.color(ERROR_COLOR) } else { title };
        let n = c.steps.len();
        let shown = if open || n == 0 {
            format!("{n} step{}", if n == 1 { "" } else { "s" })
        } else {
            c.steps.iter().map(ButtonAction::summary).collect::<Vec<_>>().join(" → ")
        };
        let mut header = row![
            card_title(open, title).on_press(Message::ToggleCycle(ci)),
            text(shown).size(13).color(MUTED_COLOR),
            space::horizontal(),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        if let Some(problem) = &problem {
            header = header.push(text(format!("⚠ {problem}")).size(12).color(ERROR_COLOR));
        }
        if open {
            header = header.push(delete_item("Delete cycle", Message::DeleteCycle(ci)));
        }
        let mut col = column![header].spacing(12);
        if open {
            col = col.push(view_cycle_editor(ci, c, names));
        }
        container(col).padding(14).width(Length::Fill).style(style::card).into()
    }
}

/// An open cycle's name and steps.
fn view_cycle_editor<'a>(ci: usize, c: &'a Cycle, names: &Names) -> Element<'a, Message> {
    let last = c.steps.len().saturating_sub(1);
    let mut steps = column![].spacing(8);
    for (i, step) in c.steps.iter().enumerate() {
        let target = Target::CycleStep(ci, i);
        let editor = action_editor(step, Button::South, CYCLE_STEP_KINDS, set_action(target), KeyField::root(target), names);
        let small = |label: &'static str, msg: Option<Message>| button(text(label).size(13)).style(style::secondary).on_press_maybe(msg);
        steps = steps.push(
            container(
                row![
                    container(text(format!("{}.", i + 1))).width(28).padding(iced::Padding::ZERO.top(6)),
                    fill_x(editor),
                    small("↑", (i > 0).then_some(Message::MoveCycleStep(ci, i, true))),
                    small("↓", (i < last).then_some(Message::MoveCycleStep(ci, i, false))),
                    small("✕", Some(Message::RemoveCycleStep(ci, i))),
                ]
                .spacing(8)
                .align_y(Alignment::Start),
            )
            .padding(8)
            .style(style::inset),
        );
    }
    let note = if c.steps.len() < 2 {
        "A cycle needs at least two steps to switch between."
    } else {
        "The first press presses step 1, the next step 2, and so on, starting over after the last. A step \
         stays pressed while the button is held. List the state the game starts in last, so the first \
         press moves on from it."
    };
    column![
        labeled("Name", field("Cycle name", &c.name).on_input(move |n| Message::RenameCycle(ci, n)).width(220).into()),
        text("Steps").size(16),
        steps,
        row![button(text("+ Add step").size(13)).style(style::secondary).on_press(Message::AddCycleStep(ci))],
        text(note).size(12).color(MUTED_COLOR),
    ]
    .spacing(10)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn cycles_are_made_with_two_steps_edited_and_renamed_with_their_mappings() {
        let mut app = app();
        let _ = app.update(Message::NewCycle);
        assert_eq!(app.config.general.cycles[0].name, "Cycle");
        assert_eq!(app.config.general.cycles[0].steps.len(), 2);
        assert!(app.open_cycles.contains(&0), "a new cycle opens");
        pick(&mut app, KeyField::root(Target::CycleStep(0, 0)), false, &["KEY_2"]);
        let _ = app.update(Message::SetAction(Target::CycleStep(0, 1), ButtonAction::Gamepad(Button::South)));
        let _ = app.update(Message::AddCycleStep(0));
        let _ = app.update(Message::MoveCycleStep(0, 2, true));
        assert_eq!(
            app.config.general.cycles[0].steps,
            [ButtonAction::Keys(vec!["KEY_2".into()]), ButtonAction::Keys(Vec::new()), ButtonAction::Gamepad(Button::South)]
        );
        let _ = app.update(Message::RemoveCycleStep(0, 1));
        assert_eq!(app.config.general.cycles[0].steps.len(), 2);

        let mapped = ButtonAction::Cycle { name: "Cycle".into(), back: true };
        app.config.general.profiles[0].set_button(Button::North, mapped);
        let _ = app.update(Message::RenameCycle(0, "Game speed".into()));
        assert_eq!(app.config.general.profiles[0].button(Button::North), &ButtonAction::Cycle { name: "Game speed".into(), back: true });
        let _ = app.update(Message::NewCycle);
        let _ = app.update(Message::RenameCycle(0, "Game speed".into()));
        assert_eq!(app.config.general.cycles[0].name, "Cycle", "a name in use is refused");
        assert!(app.open_cycles.contains(&1), "the open card moved down with its cycle");
        let _ = app.update(Message::DeleteCycle(0));
        assert_eq!(app.config.general.cycles.len(), 1);
        assert!(app.open_cycles.contains(&0));
    }

    #[test]
    fn a_step_that_isnt_an_input_or_a_missing_cycle_is_a_problem() {
        let mut app = app();
        let _ = app.update(Message::NewCycle);
        assert!(!app.cycles_have_problem(&app.names()));
        app.config.general.cycles[0].steps[0] = ButtonAction::Mode("Mouse mode".into());
        assert!(app.cycles_have_problem(&app.names()));
        let names = app.names();
        let gone = ButtonAction::Cycle { name: "Gone".into(), back: false };
        assert_eq!(crate::config::problems::action_problem(&gone, &names).as_deref(), Some("missing cycle “Gone”"));
    }
}
