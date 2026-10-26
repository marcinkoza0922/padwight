//! Cycles: each press of a Cycle action presses the next of its cycle's steps, holding it while
//! the input is held.

use super::*;

impl Engine {
    /// Replaces the cycle definitions `ButtonAction::Cycle` refers to (from the config). Steps that
    /// aren't inputs or macros do nothing. A cycle whose steps didn't change keeps its place.
    pub fn set_cycles(&mut self, cycles: &[Cycle]) {
        let defs: HashMap<String, Vec<ButtonAction>> = cycles
            .iter()
            .map(|c| {
                let steps = c.steps.iter().map(|s| if crate::config::fits_cycle(s) { s.clone() } else { ButtonAction::Disabled }).collect();
                (c.name.clone(), steps)
            })
            .collect();
        self.cycle_at.retain(|name, _| defs.get(name) == self.cycle_defs.get(name));
        self.cycle_defs = defs;
    }

    /// Puts every cycle back at its start, so the next press presses its first step. For when the
    /// game starts.
    pub fn reset_cycles(&mut self) {
        self.cycle_at.clear();
    }

    /// The step of cycle `name` a press would press now: the one after the last pressed, or before
    /// it if `back`, wrapping around. The first (or, going back, the last) when none was yet.
    fn next_step(&self, name: &str, back: bool) -> Option<usize> {
        let len = self.cycle_defs.get(name)?.len();
        if len == 0 {
            return None;
        }
        Some(match (self.cycle_at.get(name), back) {
            (None, false) => 0,
            (None, true) => len - 1,
            (Some(&at), false) => (at + 1) % len,
            (Some(&at), true) => (at + len - 1) % len,
        })
    }

    /// Presses the next step of cycle `name` (or the one before, if `back`) for the Cycle action
    /// `id`, or releases the one it pressed.
    pub(super) fn cycle(&mut self, id: StateId, (name, back): (&str, bool), pressed: bool, out: &mut Vec<OutEvent>) {
        let (src, slot) = (&id.0, id.1);
        if !pressed {
            if let Some(step) = self.cycle_held.remove(&id) {
                self.emit(src, &step, false, slot + 1, out);
            }
            return;
        }
        if self.cycle_held.contains_key(&id) {
            return;
        }
        let Some(at) = self.next_step(name, back) else {
            crate::monitor::log!("no cycle named {name:?}, or it has no steps");
            return;
        };
        self.cycle_at.insert(name.to_string(), at);
        let step = self.cycle_defs[name][at].clone();
        self.emit(src, &step, true, slot + 1, out);
        self.cycle_held.insert(id, step);
    }

    /// Lets go of every step a held Cycle pressed.
    pub(super) fn release_cycles(&mut self, out: &mut Vec<OutEvent>) {
        for ((src, slot), step) in self.cycle_held.drain().collect::<Vec<_>>() {
            self.emit(&src, &step, false, slot + 1, out);
        }
    }
}
