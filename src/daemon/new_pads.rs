//! Asking how to set up a controller Padwight hasn't seen, when another is already in use, and
//! acting on the answer. Until it is answered the controller passes through.

use super::*;
use crate::new_pad::{NewPadOutcome, NewPadSession};

/// How long after a question is put off that it is asked again.
const ASK_AGAIN: Duration = Duration::from_secs(5 * 60);

impl Daemon {
    /// Remembers a controller that has just been managed. The first one needs no question; one
    /// that joins a controller already in use is asked about.
    pub(super) fn remember_controller(&mut self, id: &str, name: &str) {
        if id.is_empty() {
            return;
        }
        if let Some(entry) = self.config.controllers.iter_mut().find(|c| c.id == id) {
            entry.name = name.to_string();
            return;
        }
        let others = self.devices.values().any(|d| !d.id.is_empty() && d.id != id);
        if others {
            self.config.controllers.push(crate::config::ControllerEntry { id: id.to_string(), pending: true, ..Default::default() });
            self.offers.new_pad_after = None;
        } else {
            self.config.answer_new_controller(id, None, false);
        }
        if let Some(entry) = self.config.controllers.iter_mut().find(|c| c.id == id) {
            entry.name = name.to_string();
        }
        if let Err(e) = self.config.save_settings() {
            log!("saving config: {e:#}");
        }
    }

    /// Opens the question for a controller that has none answered, when nothing else is up.
    pub(super) fn ask_pending_pad(&mut self) {
        let now = Instant::now();
        if self.active.is_some() || !self.config.enabled || self.offers.new_pad_after.is_some_and(|t| t > now) {
            return;
        }
        let Some((&by, dev)) = self.devices.iter().find(|(_, d)| self.config.controller(&d.id).is_some_and(|c| c.pending)) else { return };
        let (id, name) = (dev.id.clone(), dev.name.clone());
        self.offers.new_pad_after = Some(now + ASK_AGAIN);
        if !self.ensure_overlay_process() {
            return;
        }
        log!("asking how to set up the new controller {name}");
        let others: Vec<(String, String)> = self
            .config
            .controllers
            .iter()
            .filter(|c| c.id != id && !c.pending)
            .map(|c| (c.id.clone(), c.label().to_string()))
            .collect();
        self.release_mappings();
        self.driving.owner = Some(by);
        self.active = Some(Active::NewPad(NewPadSession::new(&name, &others, now), id));
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Offer, Some(sound::Feedback::Open));
    }

    /// Feeds an input to the question, and sounds its step or pick.
    pub(super) fn new_pad_input(&mut self, ev: InputEvent, now: Instant) -> Option<NewPadOutcome> {
        let Some(Active::NewPad(s, _)) = &mut self.active else { return None };
        let before = s.position();
        let outcome = s.handle(ev, now);
        let feedback = sound::Feedback::of(outcome.is_some(), s.position() != before);
        self.overlay_sound(sound::SoundOverlay::Offer, feedback);
        outcome
    }

    /// Advances the question's held direction. `None` when it timed out (and was put off), else
    /// whether the highlight moved.
    pub(super) fn tick_new_pad(&mut self, now: Instant) -> Option<bool> {
        let Some(Active::NewPad(s, _)) = &mut self.active else { return Some(false) };
        if s.expired(now) {
            self.answer_new_pad(NewPadOutcome::Later);
            return None;
        }
        Some(s.next_deadline().is_some_and(|d| d <= now) && s.tick(now))
    }

    /// Acts on the answer: sets the controller up, or leaves it passing through for later.
    pub(super) fn answer_new_pad(&mut self, outcome: NewPadOutcome) {
        let Some(Active::NewPad(_, id)) = &self.active else { return };
        let id = id.clone();
        self.close_overlay();
        match outcome {
            NewPadOutcome::Later => return,
            NewPadOutcome::Defaults => self.config.answer_new_controller(&id, None, false),
            NewPadOutcome::Copy(from) => self.config.answer_new_controller(&id, Some(&from), false),
            NewPadOutcome::PassThrough => self.config.answer_new_controller(&id, None, true),
        }
        self.resync_all();
        if let Err(e) = self.config.save_settings() {
            log!("saving config: {e:#}");
        }
    }
}
