//! The question asked when a second controller shows up that Padwight hasn't seen: how should it
//! be set up. `NewPadSession` turns controller input into the answer; the overlay process draws
//! the list as a menu. See `docs/development/per-controller-setups.md`.

use std::time::{Duration, Instant};

use crate::{
    config::{Button, OverlayStyle},
    input::InputEvent,
    overlay::OverlayView,
};

/// An unanswered question goes away by itself and is asked again later, so it can't hold the
/// controller forever.
const IDLE_CLOSE: Duration = Duration::from_secs(60);

/// What was answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewPadOutcome {
    /// Follow the active profile, as a pad with no settings of its own does.
    Defaults,
    /// Use the profile choices of the controller with this id.
    Copy(String),
    /// Hand the controller's input to the game unchanged.
    PassThrough,
    /// Not now: it passes through until it is answered.
    Later,
}

pub struct NewPadSession {
    pad: String,
    choices: Vec<(String, NewPadOutcome)>,
    selected: usize,
    last_input: Instant,
    stick_dir: crate::nav::StickDir,
    repeat: crate::nav::Repeat,
}

impl NewPadSession {
    /// A question for the controller called `pad`; `others` are the controllers it can copy
    /// from, as (id, name).
    pub fn new(pad: &str, others: &[(String, String)], now: Instant) -> Self {
        let mut choices = vec![("Use the defaults".to_string(), NewPadOutcome::Defaults)];
        choices.extend(others.iter().map(|(id, name)| (format!("Copy settings from {name}"), NewPadOutcome::Copy(id.clone()))));
        choices.push(("Pass through".into(), NewPadOutcome::PassThrough));
        NewPadSession {
            pad: pad.to_string(),
            choices,
            selected: 0,
            last_input: now,
            stick_dir: crate::nav::StickDir::default(),
            repeat: crate::nav::Repeat::default(),
        }
    }

    /// The highlighted row, for the sounds of its steps.
    pub(crate) fn position(&self) -> usize {
        self.selected
    }

    pub fn view(&self, style: &OverlayStyle) -> OverlayView {
        use crate::menu::{ItemView, MenuView};
        let items = self.choices.iter().map(|(label, _)| ItemView { label: label.clone(), ..ItemView::plain() }).collect();
        OverlayView::Menu(MenuView {
            title: format!("New controller: {}", self.pad),
            kind: crate::config::MenuKind::List,
            items,
            selected: Some(self.selected),
            picks: 0,
            crumbs: vec!["Set up a controller".into()],
            hint: "{up} {down} choose · {confirm} pick · {back} later".into(),
            style: style.clone(),
            touch: false,
            aim: None,
            prompt: None,
            one_column: false,
            figure: None,
        })
    }

    pub fn handle(&mut self, ev: InputEvent, now: Instant) -> Option<NewPadOutcome> {
        if matches!(ev, InputEvent::Button(..)) {
            self.last_input = now;
        }
        let dir = match ev {
            InputEvent::Button(b, pressed) if crate::nav::dpad_dir(b).is_some() => {
                let dir = crate::nav::dpad_dir(b)?;
                if !pressed {
                    self.repeat.release(dir);
                    return None;
                }
                Some(dir)
            }
            InputEvent::Axis(crate::input::Axis::LeftY, v) => match self.stick_dir.update((0.0, v))? {
                Some(dir) => {
                    self.last_input = now;
                    Some(dir)
                }
                None => {
                    self.repeat.stop();
                    return None;
                }
            },
            _ => None,
        };
        if let Some(dir) = dir {
            if dir.1 != 0 {
                self.repeat.start(dir, now);
                self.move_cursor(dir.1);
            }
            return None;
        }
        match ev {
            InputEvent::Button(Button::South, true) => Some(self.choices[self.selected].1.clone()),
            InputEvent::Button(Button::East, true) => Some(NewPadOutcome::Later),
            _ => None,
        }
    }

    fn move_cursor(&mut self, dy: i32) {
        self.selected = (self.selected as i32 + dy).rem_euclid(self.choices.len() as i32) as usize;
    }

    /// Repeats a held direction once it's due. Returns whether the highlight moved.
    pub fn tick(&mut self, now: Instant) -> bool {
        let Some(dir) = self.repeat.due(now) else { return false };
        self.move_cursor(dir.1);
        true
    }

    pub fn expired(&self, now: Instant) -> bool {
        self.last_input + IDLE_CLOSE <= now
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        let idle = self.last_input + IDLE_CLOSE;
        Some(self.repeat.deadline().map_or(idle, |r| r.min(idle)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(s: &mut NewPadSession, b: Button) -> Option<NewPadOutcome> {
        let now = Instant::now();
        s.handle(InputEvent::Button(b, true), now)
    }

    fn session() -> NewPadSession {
        NewPadSession::new("Pad", &[("a".into(), "Couch".into())], Instant::now())
    }

    #[test]
    fn the_first_row_is_the_defaults() {
        assert_eq!(press(&mut session(), Button::South), Some(NewPadOutcome::Defaults));
    }

    #[test]
    fn down_reaches_the_copy_row_and_then_pass_through() {
        let mut s = session();
        assert_eq!(press(&mut s, Button::DpadDown), None);
        assert_eq!(press(&mut s, Button::South), Some(NewPadOutcome::Copy("a".into())));
        let mut s = session();
        press(&mut s, Button::DpadUp);
        assert_eq!(press(&mut s, Button::South), Some(NewPadOutcome::PassThrough));
    }

    #[test]
    fn back_leaves_the_question_for_later() {
        assert_eq!(press(&mut session(), Button::East), Some(NewPadOutcome::Later));
    }
}
