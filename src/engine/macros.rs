//! Macros and turbo: playing a macro's steps over time, and flipping a turbo on and off.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum MacroOp {
    Press(ButtonAction),
    Release(ButtonAction),
    /// Seconds.
    Wait(f32),
    Stick(Stick, (f32, f32)),
}

pub(super) struct MacroRun {
    ops: Arc<Vec<MacroOp>>,
    next: usize,
    /// Seconds left in the current wait; may go negative to carry over into the next step.
    wait: f32,
    repeat: bool,
    /// Actions the macro has pressed and not yet released.
    held: Vec<ButtonAction>,
    /// Sticks the macro has moved, recentered when a pass ends.
    sticks: Vec<Stick>,
}

impl MacroRun {
    pub(super) fn new(ops: Arc<Vec<MacroOp>>, repeat: bool) -> Self {
        MacroRun { ops, next: 0, wait: 0.0, repeat, held: Vec::new(), sticks: Vec::new() }
    }
}

pub(super) fn compile_macro(m: &Macro) -> Vec<MacroOp> {
    let mut ops = Vec::new();
    for step in &m.steps {
        match step {
            MacroStep::Tap { action, hold_ms } => ops.extend([
                MacroOp::Press(action.clone()),
                MacroOp::Wait(*hold_ms as f32 / 1000.0),
                MacroOp::Release(action.clone()),
            ]),
            MacroStep::Press(action) => ops.push(MacroOp::Press(action.clone())),
            MacroStep::Release(action) => ops.push(MacroOp::Release(action.clone())),
            MacroStep::Wait(ms) => ops.push(MacroOp::Wait(*ms as f32 / 1000.0)),
            MacroStep::Stick { stick, x, y } => {
                ops.push(MacroOp::Stick(*stick, (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0))))
            }
        }
    }
    ops
}

/// The macros that play themselves, directly or through other macros.
pub(super) fn looping_macros(macros: &[Macro]) -> HashSet<&str> {
    let plays: HashMap<&str, Vec<String>> = macros
        .iter()
        .map(|m| {
            let mut names = Vec::new();
            for action in m.steps.iter().filter_map(MacroStep::action) {
                action.walk(&mut |a| {
                    if let ButtonAction::Macro { name, .. } = a {
                        names.push(name.clone());
                    }
                });
            }
            (m.name.as_str(), names)
        })
        .collect();
    let reaches_itself = |start: &str| {
        let mut seen = HashSet::new();
        let mut next: Vec<&str> = plays.get(start).into_iter().flatten().map(String::as_str).collect();
        while let Some(name) = next.pop() {
            if name == start {
                return true;
            }
            if seen.insert(name) {
                next.extend(plays.get(name).into_iter().flatten().map(String::as_str));
            }
        }
        false
    };
    macros.iter().map(|m| m.name.as_str()).filter(|n| reaches_itself(n)).collect()
}

pub(super) struct TurboState {
    pub(super) action: ButtonAction,
    /// Seconds between press and release (half a turbo cycle).
    pub(super) half_period: f32,
    pub(super) elapsed: f32,
    pub(super) down: bool,
}

impl Engine {
    /// Replaces the macro definitions `ButtonAction::Macro` refers to (from the config). A macro
    /// that plays itself, directly or through others, would play forever, so it does nothing.
    pub fn set_macros(&mut self, macros: &[Macro]) {
        let looping = looping_macros(macros);
        self.macro_defs = macros
            .iter()
            .map(|m| {
                let ops = if looping.contains(m.name.as_str()) {
                    crate::monitor::log!("macro {:?} plays itself, so it does nothing", m.name);
                    Vec::new()
                } else {
                    compile_macro(m)
                };
                (m.name.clone(), Arc::new(ops))
            })
            .collect();
        self.macro_ms = macros.iter().map(|m| (m.name.clone(), m.duration_ms())).collect();
    }

    /// Runs a macro's operations until it has to wait or it ends. Each pass ends by releasing
    /// whatever it still holds; a repeating macro then starts over seamlessly, except that a
    /// macro with no waits at all runs one pass per tick instead of spinning in place.
    pub(super) fn advance_macro(&mut self, id: &StateId, dt: f32, out: &mut Vec<OutEvent>) {
        let Some(mut run) = self.macros_running.remove(id) else { return };
        run.wait -= dt;
        let has_wait = run.ops.iter().any(|op| matches!(op, MacroOp::Wait(s) if *s > 0.0));
        while run.wait <= 0.0 {
            if run.next >= run.ops.len() {
                self.end_pass(id, &mut run, out);
                if !run.repeat {
                    return;
                }
                run.next = 0;
                if !has_wait {
                    run.wait = 0.0;
                    break;
                }
                continue;
            }
            match run.ops[run.next].clone() {
                MacroOp::Press(action) => {
                    self.emit(&id.0, &action, true, id.1 + 1, out);
                    run.held.push(action);
                }
                MacroOp::Release(action) => {
                    if let Some(i) = run.held.iter().position(|a| *a == action) {
                        run.held.remove(i);
                        self.emit(&id.0, &action, false, id.1 + 1, out);
                    }
                }
                MacroOp::Wait(seconds) => run.wait += seconds,
                MacroOp::Stick(stick, position) => {
                    self.macro_sticks.insert(stick, position);
                    if !run.sticks.contains(&stick) {
                        run.sticks.push(stick);
                    }
                    self.emit_pad_stick(stick, out);
                }
            }
            run.next += 1;
        }
        self.macros_running.insert(id.clone(), run);
    }

    /// Ends a pass: releases what the macro still holds (most recent first) and recenters the
    /// sticks it moved.
    pub(super) fn end_pass(&mut self, id: &StateId, run: &mut MacroRun, out: &mut Vec<OutEvent>) {
        for action in std::mem::take(&mut run.held).into_iter().rev() {
            self.emit(&id.0, &action, false, id.1 + 1, out);
        }
        for stick in std::mem::take(&mut run.sticks) {
            self.macro_sticks.remove(&stick);
            self.emit_pad_stick(stick, out);
        }
    }

    pub(super) fn stop_macro(&mut self, id: &StateId, out: &mut Vec<OutEvent>) {
        if let Some(mut run) = self.macros_running.remove(id) {
            self.end_pass(id, &mut run, out);
        }
    }

    pub(super) fn tick_macros(&mut self, dt: f32, out: &mut Vec<OutEvent>) {
        let ids: Vec<StateId> = self.macros_running.keys().cloned().collect();
        for id in ids {
            self.advance_macro(&id, dt, out);
        }
    }

    /// What a turbo repeats, and the seconds between its flips (half a turbo cycle). A macro is
    /// played once, never looping, and flipping it on and off every half period starts it once
    /// per cycle: pressing starts it and releasing does nothing.
    pub(super) fn turbo_pace(&self, inner: &ButtonAction, rate: f32, every_ms: u64) -> (ButtonAction, f32) {
        match inner {
            ButtonAction::Macro { name, .. } => {
                let macro_ms = self.macro_ms.get(name).copied().unwrap_or(0);
                let cycle_ms = crate::config::macro_turbo_ms(every_ms, macro_ms);
                (ButtonAction::Macro { name: name.clone(), repeat: false }, cycle_ms as f32 / 2000.0)
            }
            _ => (inner.clone(), 0.5 / rate.clamp(0.5, 60.0)),
        }
    }

    /// Advances turbo actions: each flips between pressed and released every half period.
    pub(super) fn tick_turbo(&mut self, dt: f32, out: &mut Vec<OutEvent>) {
        let ids: Vec<StateId> = self.turbo.keys().cloned().collect();
        for id in ids {
            let Some(state) = self.turbo.get_mut(&id) else { continue };
            state.elapsed += dt;
            let mut flips = Vec::new();
            while state.elapsed >= state.half_period {
                state.elapsed -= state.half_period;
                state.down = !state.down;
                flips.push(state.down);
            }
            let action = state.action.clone();
            for down in flips {
                self.emit(&id.0, &action, down, id.1 + 1, out);
            }
        }
    }
}
