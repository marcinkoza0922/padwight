//! Input logging: what the player pressed, one entry per press, and what each press did.
//! The daemon feeds a controller's raw input in (before any mapping, so it reads the same
//! whether a profile, a menu or the on-screen keyboard has the controller) and attaches the
//! actions that input fired. [`view`] groups entries into sequences for `{current_input}` and
//! log overlays.

mod view;

#[cfg(test)]
mod tests;

use std::time::Instant;

pub use view::{Inputs, LogCell, LogView, line_life, log_view, merged, next_change, shifts_next_change, shifts_view};
#[cfg(test)]
pub use view::LogLine;

use crate::{
    config::{Button, ButtonAction, Deadzone, MouseButton, Stick, StickConfig, Trigger, TriggerAction, WheelDirection},
    input::{Axis, InputEvent},
};

/// The engine's hysteresis for stick directions and triggers, so the log agrees with it.
const HYSTERESIS: f32 = 0.05;
/// Entries kept per controller; far more than any overlay shows.
const CAPACITY: usize = 256;
/// How far back an action looks for the press that fired it.
const MATCH_WINDOW: usize = 32;

/// One of eight directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

impl Dir {
    /// The direction of held `[up, down, left, right]`; opposites cancel out.
    fn from_held([up, down, left, right]: [bool; 4]) -> Option<Dir> {
        let v = i8::from(down) - i8::from(up);
        let h = i8::from(right) - i8::from(left);
        Some(match (h, v) {
            (0, -1) => Dir::Up,
            (1, -1) => Dir::UpRight,
            (1, 0) => Dir::Right,
            (1, 1) => Dir::DownRight,
            (0, 1) => Dir::Down,
            (-1, 1) => Dir::DownLeft,
            (-1, 0) => Dir::Left,
            (-1, -1) => Dir::UpLeft,
            _ => return None,
        })
    }

    /// The D-pad arms that make this direction, `[up, down, left, right]`.
    pub fn arms(self) -> [bool; 4] {
        let (up, down) = (matches!(self, Dir::Up | Dir::UpRight | Dir::UpLeft), matches!(self, Dir::Down | Dir::DownRight | Dir::DownLeft));
        let (left, right) = (matches!(self, Dir::Left | Dir::UpLeft | Dir::DownLeft), matches!(self, Dir::Right | Dir::UpRight | Dir::DownRight));
        [up, down, left, right]
    }

    pub fn arrow(self) -> &'static str {
        match self {
            Dir::Up => "↑",
            Dir::UpRight => "↗",
            Dir::Right => "→",
            Dir::DownRight => "↘",
            Dir::Down => "↓",
            Dir::DownLeft => "↙",
            Dir::Left => "←",
            Dir::UpLeft => "↖",
        }
    }

    /// Whether it points toward this one of `[up, down, left, right]`.
    fn has(self, cardinal: usize) -> bool {
        let [up, down, left, right] = match self {
            Dir::Up => [true, false, false, false],
            Dir::UpRight => [true, false, false, true],
            Dir::Right => [false, false, false, true],
            Dir::DownRight => [false, true, false, true],
            Dir::Down => [false, true, false, false],
            Dir::DownLeft => [false, true, true, false],
            Dir::Left => [false, false, true, false],
            Dir::UpLeft => [true, false, true, false],
        };
        [up, down, left, right][cardinal]
    }
}

/// What an entry records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Button(Button),
    /// A d-pad (`None`) or stick direction, diagonals included.
    Direction(Option<Stick>, Dir),
    Trigger(Trigger),
    /// Buttons that fired a combo together, in the order they were pressed.
    Combo(Vec<Button>),
    /// What a controller sent as a keyboard and mouse: a key (by its evdev code), a mouse
    /// button, or a turn of the wheel.
    Key(String),
    Mouse(MouseButton),
    Wheel(WheelDirection),
    /// A mode the controller was in; held for as long as it was.
    Mode(String),
    /// A layer held down.
    Layer(String),
}

/// One press: what, when, for how long, and what it did.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub input: Input,
    pub pressed: Instant,
    /// `None` while still held.
    pub released: Option<Instant>,
    /// What it did, once known: `None` for nothing, or a button passed straight through.
    pub label: Option<String>,
    /// An action has been matched to it (even one that gets no label).
    resolved: bool,
    id: u64,
}

/// Wheel turns closer together than this are one entry.
const WHEEL_MERGE: std::time::Duration = std::time::Duration::from_millis(300);

/// Where an input is held; each holds at most one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Button(Button),
    Dpad,
    Stick(Stick),
    Trigger(Trigger),
    Key(u16),
    Mouse(MouseButton),
    /// Never held: a turn of the wheel is let go as it's recorded.
    Wheel,
}

/// How analog input becomes presses: the profile's own settings, so the log agrees with
/// the engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// Per stick (left, right): deadzone and direction press threshold.
    pub sticks: [(Deadzone, f32); 2],
    /// Per trigger (left, right): press threshold.
    pub triggers: [f32; 2],
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds { sticks: [(Deadzone::from(0.1), 0.5); 2], triggers: [0.5; 2] }
    }
}

impl Thresholds {
    /// A trigger mapped to a button presses where that button does; any other at halfway.
    pub fn new(sticks: [&StickConfig; 2], triggers: [&TriggerAction; 2]) -> Self {
        let trigger = |a: &TriggerAction| match a {
            TriggerAction::Button { threshold, .. } => *threshold,
            _ => 0.5,
        };
        Thresholds {
            sticks: sticks.map(|s| (s.dead(), s.key_threshold)),
            triggers: triggers.map(trigger),
        }
    }
}

/// The input an action came from, as the log matches it to an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FiredFrom {
    Button(Button),
    Combo(Vec<Button>),
    Trigger(Trigger),
    /// A stick zone.
    Stick(Stick),
    /// The touchpad's click.
    Touchpad,
}

/// The label for an action fired from `from`: nothing for a button passed straight through
/// to the same button on the virtual pad, or for no action.
pub fn label_for(from: &FiredFrom, action: &ButtonAction) -> Option<String> {
    match (from, action) {
        (_, ButtonAction::Disabled) => None,
        (FiredFrom::Button(b), ButtonAction::Gamepad(out)) if b == out => None,
        _ => Some(action.summary()),
    }
}

/// One controller's log.
#[derive(Debug, Default)]
pub struct InputLog {
    entries: Vec<Entry>,
    next_id: u64,
    /// The entry each held slot is holding.
    open: Vec<(Slot, u64)>,
    /// Held d-pad and stick directions, `[up, down, left, right]`.
    dpad: [bool; 4],
    sticks: [[bool; 4]; 2],
    /// Raw stick axes: left x, y, right x, y.
    axes: [f32; 4],
    triggers: [bool; 2],
}

impl InputLog {
    /// Oldest first.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Records one raw input. Returns whether the log changed.
    pub fn event(&mut self, ev: InputEvent, t: Thresholds, now: Instant) -> bool {
        match ev {
            InputEvent::Button(b, pressed) => match cardinal(b) {
                Some(i) => {
                    self.dpad[i] = pressed;
                    self.direction(Slot::Dpad, None, Dir::from_held(self.dpad), now)
                }
                None if pressed => self.press(Slot::Button(b), Input::Button(b), now),
                None => self.release(Slot::Button(b), now),
            },
            InputEvent::Axis(axis, v) => {
                let (stick, i) = match axis {
                    Axis::LeftX => (Stick::Left, 0),
                    Axis::LeftY => (Stick::Left, 1),
                    Axis::RightX => (Stick::Right, 2),
                    Axis::RightY => (Stick::Right, 3),
                    Axis::LeftTrigger => return self.trigger(Trigger::Left, v, t.triggers[0], now),
                    Axis::RightTrigger => return self.trigger(Trigger::Right, v, t.triggers[1], now),
                };
                self.axes[i] = v;
                self.stick(stick, t.sticks[stick_index(stick)], now)
            }
            // Touchpad clicks don't go in the log yet.
            InputEvent::Touchpad(_) => false,
        }
    }

    /// Records what the mappings sent as a keyboard and mouse: a key or a mouse button going
    /// down or up, or a turn of the wheel (a tap). Returns whether the log changed.
    pub fn output(&mut self, ev: &crate::output::OutEvent, now: Instant) -> bool {
        use crate::output::OutEvent;
        match *ev {
            OutEvent::Key(k, true) => self.press(Slot::Key(k.code()), Input::Key(format!("{k:?}")), now),
            OutEvent::Key(k, false) => self.release(Slot::Key(k.code()), now),
            OutEvent::MouseButton(m, true) => self.press(Slot::Mouse(m), Input::Mouse(m), now),
            OutEvent::MouseButton(m, false) => self.release(Slot::Mouse(m), now),
            OutEvent::Wheel { vertical, horizontal } => {
                let dir = match (vertical.signum(), horizontal.signum()) {
                    (1, _) => WheelDirection::Up,
                    (-1, _) => WheelDirection::Down,
                    (_, 1) => WheelDirection::Right,
                    (_, -1) => WheelDirection::Left,
                    _ => return false,
                };
                // A turn has no length; one already showing for this direction isn't repeated
                // while the wheel keeps turning.
                if self.entries.last().is_some_and(|e| e.input == Input::Wheel(dir) && now.duration_since(e.pressed) < WHEEL_MERGE) {
                    return false;
                }
                let id = self.next_id;
                self.press(Slot::Wheel, Input::Wheel(dir), now);
                self.open.retain(|(_, open)| *open != id);
                if let Some(e) = self.entries.last_mut() {
                    e.released = Some(now);
                }
                true
            }
            OutEvent::PadButton(b, pressed) => self.event(InputEvent::Button(b, pressed), Thresholds::default(), now),
            OutEvent::PadAxis(..) | OutEvent::MouseMove(..) | OutEvent::MouseTo(..) => false,
        }
    }

    /// Records the controller's mode and held layers as they are now: what stopped being on is
    /// let go, and what started is a new entry. Returns whether the log changed.
    pub fn shifts(&mut self, mode: Option<&str>, layers: &[String], now: Instant) -> bool {
        let mut changed = false;
        for e in self.entries.iter_mut().filter(|e| e.released.is_none()) {
            let on = match &e.input {
                Input::Mode(m) => mode == Some(m.as_str()),
                Input::Layer(l) => layers.contains(l),
                _ => continue,
            };
            if !on {
                e.released = Some(now);
                changed = true;
            }
        }
        let wanted = mode.map(|m| Input::Mode(m.to_string())).into_iter().chain(layers.iter().map(|l| Input::Layer(l.clone())));
        for input in wanted {
            if !self.entries.iter().any(|e| e.released.is_none() && e.input == input) {
                self.record(input, now);
                changed = true;
            }
        }
        changed
    }

    /// Attaches an action to the press that fired it. A combo's presses become one entry.
    pub fn attach(&mut self, from: &FiredFrom, label: Option<String>) {
        if let FiredFrom::Combo(members) = from
            && self.combine(members)
        {
            return self.label_newest(|i| matches!(i, Input::Combo(m) if same_buttons(m, members)), label);
        }
        let first = match from {
            FiredFrom::Combo(members) => members.first().copied().map(FiredFrom::Button),
            other => Some(other.clone()),
        };
        if let Some(from) = first {
            self.label_newest(|i| fired_by(&from, i), label);
        }
    }

    fn label_newest(&mut self, pick: impl Fn(&Input) -> bool, label: Option<String>) {
        let start = self.entries.len().saturating_sub(MATCH_WINDOW);
        let Some(e) = self.entries[start..].iter_mut().rev().find(|e| pick(&e.input)) else { return };
        e.resolved = true;
        let Some(label) = label else { return };
        e.label = match e.label.take() {
            Some(prev) if prev == label => Some(prev),
            Some(prev) => Some(format!("{prev} & {label}")),
            None => Some(label),
        };
    }

    /// Merges the newest unresolved press of each member into one combo entry where the
    /// first of them was. False when some member has no such press (a d-pad member, say).
    fn combine(&mut self, members: &[Button]) -> bool {
        let start = self.entries.len().saturating_sub(MATCH_WINDOW);
        let mut found = Vec::new();
        for b in members {
            let newest = (start..self.entries.len()).rev().find(|&i| self.entries[i].input == Input::Button(*b));
            match newest {
                Some(i) if !self.entries[i].resolved => found.push(i),
                _ => return false,
            }
        }
        found.sort_unstable();
        let pressed: Vec<Button> = found
            .iter()
            .filter_map(|&i| match self.entries[i].input {
                Input::Button(b) => Some(b),
                _ => None,
            })
            .collect();
        let keep = found[0];
        let id = self.entries[keep].id;
        for &i in found[1..].iter().rev() {
            let gone = self.entries.remove(i);
            for (_, open) in &mut self.open {
                if *open == gone.id {
                    *open = id;
                }
            }
        }
        self.entries[keep].input = Input::Combo(pressed);
        // Released only once every member is.
        if self.open.iter().any(|(_, open)| *open == id) {
            self.entries[keep].released = None;
        }
        true
    }

    fn press(&mut self, slot: Slot, input: Input, now: Instant) -> bool {
        self.release(slot, now);
        let id = self.record(input, now);
        self.open.push((slot, id));
        true
    }

    /// Adds an entry, held until something lets it go; returns its id.
    fn record(&mut self, input: Input, now: Instant) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(Entry { input, pressed: now, released: None, label: None, resolved: false, id });
        if self.entries.len() > CAPACITY {
            self.entries.drain(..CAPACITY / 4);
        }
        id
    }

    fn release(&mut self, slot: Slot, now: Instant) -> bool {
        let Some(at) = self.open.iter().position(|(s, _)| *s == slot) else { return false };
        let (_, id) = self.open.remove(at);
        if self.open.iter().any(|(_, open)| *open == id) {
            return false;
        }
        match self.entries.iter_mut().rev().find(|e| e.id == id) {
            Some(e) => {
                e.released = Some(now);
                true
            }
            None => false,
        }
    }

    /// Moves a d-pad or stick to a direction (or center), ending the old direction's entry.
    fn direction(&mut self, slot: Slot, stick: Option<Stick>, dir: Option<Dir>, now: Instant) -> bool {
        let held = self.open.iter().find(|(s, _)| *s == slot).map(|(_, id)| *id);
        let current = held.and_then(|id| self.entries.iter().rev().find(|e| e.id == id)).map(|e| e.input.clone());
        if let Some(d) = dir
            && current == Some(Input::Direction(stick, d))
        {
            return false;
        }
        let released = self.release(slot, now);
        match dir {
            Some(d) => self.press(slot, Input::Direction(stick, d), now),
            None => released,
        }
    }

    fn stick(&mut self, stick: Stick, (deadzone, threshold): (Deadzone, f32), now: Instant) -> bool {
        let base = stick_index(stick) * 2;
        let (x, y) = crate::engine::apply_deadzone(self.axes[base], self.axes[base + 1], deadzone);
        let held = &mut self.sticks[stick_index(stick)];
        for (i, v) in [-y, y, -x, x].into_iter().enumerate() {
            if !held[i] && v >= threshold {
                held[i] = true;
            } else if held[i] && v < threshold - HYSTERESIS {
                held[i] = false;
            }
        }
        let dir = Dir::from_held(*held);
        self.direction(Slot::Stick(stick), Some(stick), dir, now)
    }

    fn trigger(&mut self, t: Trigger, value: f32, threshold: f32, now: Instant) -> bool {
        let i = usize::from(t == Trigger::Right);
        if !self.triggers[i] && value >= threshold {
            self.triggers[i] = true;
            self.press(Slot::Trigger(t), Input::Trigger(t), now)
        } else if self.triggers[i] && value < threshold - HYSTERESIS {
            self.triggers[i] = false;
            self.release(Slot::Trigger(t), now)
        } else {
            false
        }
    }
}

fn stick_index(s: Stick) -> usize {
    usize::from(s == Stick::Right)
}

/// A d-pad button's place in `[up, down, left, right]`.
fn cardinal(b: Button) -> Option<usize> {
    match b {
        Button::DpadUp => Some(0),
        Button::DpadDown => Some(1),
        Button::DpadLeft => Some(2),
        Button::DpadRight => Some(3),
        _ => None,
    }
}

/// Whether two combos have the same buttons, in any order.
fn same_buttons(a: &[Button], b: &[Button]) -> bool {
    a.len() == b.len() && a.iter().all(|x| b.contains(x))
}

/// Whether an action fired from `from` came from this entry's input.
fn fired_by(from: &FiredFrom, input: &Input) -> bool {
    match (from, input) {
        (FiredFrom::Button(b), Input::Direction(None, d)) => cardinal(*b).is_some_and(|c| d.has(c)),
        (FiredFrom::Button(b), Input::Direction(Some(s), d)) => b.stick_direction().is_some_and(|(stick, _)| stick == *s)
            && Button::stick_directions(*s).iter().position(|x| x == b).is_some_and(|c| d.has(c)),
        (FiredFrom::Button(b), Input::Button(x)) => b == x,
        (FiredFrom::Button(b), Input::Combo(members)) => members.contains(b),
        (FiredFrom::Stick(s), Input::Direction(Some(x), _)) => s == x,
        (FiredFrom::Trigger(t), Input::Trigger(x)) => t == x,
        _ => false,
    }
}
