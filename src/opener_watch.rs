//! Notices the input that opens the Guide overlay in a stream of controller events, for the
//! panels the daemon runs itself (keyboard, media controls, menus, the offer). The engine has
//! its own handling while the controller is playing; these panels take the events instead, and
//! the same input should put them away.

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use crate::{
    config::{Button, GestureKind, ShortcutInput},
    input::InputEvent,
};

/// How long gestures take, from the active profile.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// The longest gap between the taps of a double or triple tap.
    pub tap: Duration,
    /// How long a long press is held.
    pub long: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { tap: Duration::from_millis(250), long: Duration::from_millis(500) }
    }
}

#[derive(Debug, Default)]
pub struct Watch {
    input: Option<ShortcutInput>,
    held: HashSet<Button>,
    /// A combo that went off stays quiet until one of its buttons is let go.
    fired: bool,
    taps: Vec<Instant>,
    down_since: Option<Instant>,
}

impl Watch {
    /// Watches for `input` from now on; nothing for an unset opener.
    pub fn set(&mut self, input: Option<ShortcutInput>) {
        if self.input != input {
            self.input = input;
            self.fired = false;
            self.taps.clear();
            self.down_since = None;
        }
    }

    /// Takes the next event; true when it completes the input: a button going down, the last
    /// button of a combo, the last tap of a gesture, or a long press being let go.
    pub fn feed(&mut self, ev: &InputEvent, now: Instant, timing: Timing) -> bool {
        let InputEvent::Button(b, down) = *ev else { return false };
        if down {
            self.held.insert(b);
        } else {
            self.held.remove(&b);
        }
        let Some(input) = self.input.clone() else { return false };
        match (input.buttons.as_slice(), input.gesture) {
            ([only], None) => down && b == *only,
            ([only], Some(kind)) => self.gesture((*only, kind), (b, down), now, timing),
            (members, _) => {
                if !members.contains(&b) {
                    return false;
                }
                if !down {
                    self.fired = false;
                    return false;
                }
                let all = members.iter().all(|m| self.held.contains(m));
                let was = self.fired;
                self.fired = was || all;
                !was && all
            }
        }
    }

    fn gesture(&mut self, (button, kind): (Button, GestureKind), (b, down): (Button, bool), now: Instant, timing: Timing) -> bool {
        if b != button {
            if down {
                self.taps.clear();
                self.down_since = None;
            }
            return false;
        }
        match kind {
            GestureKind::LongPress if down => {
                self.down_since = Some(now);
                false
            }
            GestureKind::LongPress => self.down_since.take().is_some_and(|t| now.duration_since(t) >= timing.long),
            GestureKind::DoubleTap | GestureKind::TripleTap if down => {
                self.taps.retain(|t| now.duration_since(*t) <= timing.tap);
                self.taps.push(now);
                let needed = if kind == GestureKind::DoubleTap { 2 } else { 3 };
                let done = self.taps.len() >= needed;
                if done {
                    self.taps.clear();
                }
                done
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(b: Button, down: bool) -> InputEvent {
        InputEvent::Button(b, down)
    }

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    fn watching(input: ShortcutInput) -> Watch {
        let mut w = Watch::default();
        w.set(Some(input));
        w
    }

    #[test]
    fn a_button_fires_as_it_goes_down() {
        let (mut w, t0, timing) = (watching(ShortcutInput::button(Button::Guide)), Instant::now(), Timing::default());
        assert!(w.feed(&press(Button::Guide, true), t0, timing));
        assert!(!w.feed(&press(Button::Guide, false), at(t0, 50), timing));
        assert!(!w.feed(&press(Button::South, true), at(t0, 60), timing));
    }

    #[test]
    fn a_combo_fires_once_when_its_last_button_goes_down() {
        let (mut w, t0, timing) = (watching(ShortcutInput::combo(&[Button::Start, Button::Select])), Instant::now(), Timing::default());
        assert!(!w.feed(&press(Button::Start, true), t0, timing));
        assert!(w.feed(&press(Button::Select, true), at(t0, 10), timing));
        assert!(!w.feed(&press(Button::Select, false), at(t0, 20), timing));
        assert!(w.feed(&press(Button::Select, true), at(t0, 30), timing), "Start is still down, so Select again is the combo again");
        // Pressing in the other order works too.
        w.feed(&press(Button::Start, false), at(t0, 40), timing);
        w.feed(&press(Button::Select, false), at(t0, 41), timing);
        assert!(!w.feed(&press(Button::Select, true), at(t0, 50), timing));
        assert!(w.feed(&press(Button::Start, true), at(t0, 60), timing));
    }

    #[test]
    fn taps_must_come_quickly_and_alone() {
        let (mut w, t0, timing) = (watching(ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap)), Instant::now(), Timing::default());
        assert!(!w.feed(&press(Button::Select, true), t0, timing));
        assert!(w.feed(&press(Button::Select, true), at(t0, 150), timing));
        assert!(!w.feed(&press(Button::Select, true), at(t0, 1000), timing));
        assert!(!w.feed(&press(Button::Select, true), at(t0, 1400), timing), "too slow");
        w.feed(&press(Button::Select, true), at(t0, 1500), timing);
        w.feed(&press(Button::South, true), at(t0, 1510), timing);
        assert!(!w.feed(&press(Button::Select, true), at(t0, 1520), timing), "another button between starts over");
    }

    #[test]
    fn a_long_press_fires_when_let_go_after_long_enough() {
        let (mut w, t0, timing) = (watching(ShortcutInput::gesture(Button::Select, GestureKind::LongPress)), Instant::now(), Timing::default());
        w.feed(&press(Button::Select, true), t0, timing);
        assert!(!w.feed(&press(Button::Select, false), at(t0, 200), timing));
        w.feed(&press(Button::Select, true), at(t0, 300), timing);
        assert!(w.feed(&press(Button::Select, false), at(t0, 900), timing));
    }

    #[test]
    fn an_unset_opener_never_fires() {
        let mut w = Watch::default();
        assert!(!w.feed(&press(Button::Guide, true), Instant::now(), Timing::default()));
    }
}
