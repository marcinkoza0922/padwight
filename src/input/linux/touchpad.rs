//! A controller's touchpad node (DualShock 4, DualSense): a click button and multitouch
//! positions. Each finger is reported on its own, by its contact slot.

use evdev::{AbsoluteAxisCode as Abs, Device, EventSummary, KeyCode};

use crate::input::{InputEvent, TouchpadEvent};

/// Pad size assumed when the device doesn't report its range (a DualSense's).
const FALLBACK_SIZE: (f32, f32) = (1920.0, 1080.0);

/// True for a controller's touchpad node: a click button and multitouch positions, and no face
/// buttons (which would make it a gamepad). Only used to pair it with a managed controller.
pub fn is_touchpad(dev: &Device) -> bool {
    let click = dev.supported_keys().is_some_and(|k| k.contains(KeyCode::BTN_LEFT));
    let multitouch = dev
        .supported_absolute_axes()
        .is_some_and(|a| a.contains(Abs::ABS_MT_POSITION_X) && a.contains(Abs::ABS_MT_POSITION_Y));
    click && multitouch && !super::is_gamepad(dev)
}

/// The latest state of one contact slot. `tracking` is -1 when the slot has no finger.
#[derive(Clone, Copy, PartialEq)]
struct Contact {
    tracking: i32,
    x: i32,
    y: i32,
}

const NO_CONTACT: Contact = Contact { tracking: -1, x: 0, y: 0 };

/// Turns a touchpad's raw events into [`InputEvent::Touchpad`]s, one per finger that changed in
/// each report.
pub struct TouchNormalizer {
    /// Raw units at the pad's top left, and across and down it.
    origin: (i32, i32),
    size: (f32, f32),
    contacts: Vec<Contact>,
    /// Each slot as last reported.
    reported: Vec<Contact>,
    /// Slot that the next position events apply to (ABS_MT_SLOT).
    slot: usize,
}

impl TouchNormalizer {
    pub fn new(dev: &Device) -> Self {
        let range = |axis: Abs| {
            dev.get_absinfo()
                .ok()
                .and_then(|mut it| it.find(|(code, _)| *code == axis))
                .map(|(_, info)| (info.minimum(), (info.maximum() - info.minimum()) as f32))
                .filter(|(_, size)| *size > 0.0)
        };
        let (x0, w) = range(Abs::ABS_MT_POSITION_X).unwrap_or((0, FALLBACK_SIZE.0));
        let (y0, h) = range(Abs::ABS_MT_POSITION_Y).unwrap_or((0, FALLBACK_SIZE.1));
        Self::with_size((x0, y0), (w, h))
    }

    fn with_size(origin: (i32, i32), size: (f32, f32)) -> Self {
        TouchNormalizer { origin, size, contacts: Vec::new(), reported: Vec::new(), slot: 0 }
    }

    pub fn translate(&mut self, ev: evdev::InputEvent, out: &mut Vec<InputEvent>) {
        match ev.destructure() {
            EventSummary::Key(_, KeyCode::BTN_LEFT, value) if value <= 1 => {
                out.push(InputEvent::Touchpad(TouchpadEvent::Click(value == 1)));
            }
            EventSummary::AbsoluteAxis(_, Abs::ABS_MT_SLOT, value) => self.slot = value.max(0) as usize,
            EventSummary::AbsoluteAxis(_, Abs::ABS_MT_TRACKING_ID, value) => self.contact().tracking = value,
            EventSummary::AbsoluteAxis(_, Abs::ABS_MT_POSITION_X, value) => self.contact().x = value,
            EventSummary::AbsoluteAxis(_, Abs::ABS_MT_POSITION_Y, value) => self.contact().y = value,
            EventSummary::Synchronization(..) => self.finish_report(out),
            _ => {}
        }
    }

    /// The contact slot the next position or tracking event applies to.
    fn contact(&mut self) -> &mut Contact {
        if self.contacts.len() <= self.slot {
            self.contacts.resize(self.slot + 1, NO_CONTACT);
        }
        &mut self.contacts[self.slot]
    }

    /// Reports each finger that landed, moved or lifted since the last report. A slot whose
    /// contact changed without lifting in between is a new finger: the old one lifts first.
    fn finish_report(&mut self, out: &mut Vec<InputEvent>) {
        self.reported.resize(self.contacts.len().max(self.reported.len()), NO_CONTACT);
        for slot in 0..self.reported.len() {
            let now = self.contacts.get(slot).copied().unwrap_or(NO_CONTACT);
            let was = self.reported[slot];
            if now == was {
                continue;
            }
            let finger = slot.min(u8::MAX as usize) as u8;
            let new_contact = was.tracking != now.tracking;
            if was.tracking >= 0 && new_contact {
                out.push(InputEvent::Touchpad(TouchpadEvent::Lift { finger }));
            }
            if now.tracking >= 0 {
                let x = ((now.x - self.origin.0) as f32 / self.size.0).clamp(0.0, 1.0);
                let y = ((now.y - self.origin.1) as f32 / self.size.1).clamp(0.0, 1.0);
                out.push(InputEvent::Touchpad(TouchpadEvent::Touch { finger, x, y, height: self.size.1 / self.size.0 }));
            }
            self.reported[slot] = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use evdev::{EventType, InputEvent as Raw};

    use super::*;

    fn abs(code: Abs, value: i32) -> Raw {
        Raw::new(EventType::ABSOLUTE.0, code.0, value)
    }

    fn sync() -> Raw {
        Raw::new(EventType::SYNCHRONIZATION.0, 0, 0)
    }

    /// Feeds one report and returns what it produced.
    fn report(norm: &mut TouchNormalizer, events: &[Raw]) -> Vec<InputEvent> {
        let mut out = Vec::new();
        for ev in events.iter().copied().chain([sync()]) {
            norm.translate(ev, &mut out);
        }
        out
    }

    fn touch(norm: &mut TouchNormalizer, slot: i32, id: i32, x: i32, y: i32) -> Vec<InputEvent> {
        report(norm, &[abs(Abs::ABS_MT_SLOT, slot), abs(Abs::ABS_MT_TRACKING_ID, id), abs(Abs::ABS_MT_POSITION_X, x), abs(Abs::ABS_MT_POSITION_Y, y)])
    }

    fn norm() -> TouchNormalizer {
        TouchNormalizer::with_size((0, 0), (1000.0, 500.0))
    }

    fn at(finger: u8, x: f32, y: f32) -> InputEvent {
        InputEvent::Touchpad(TouchpadEvent::Touch { finger, x, y, height: 0.5 })
    }

    fn lift(finger: u8) -> InputEvent {
        InputEvent::Touchpad(TouchpadEvent::Lift { finger })
    }

    #[test]
    fn a_finger_is_reported_where_it_lands_and_as_it_moves() {
        let mut n = norm();
        assert_eq!(touch(&mut n, 0, 7, 100, 200), [at(0, 0.1, 0.4)]);
        assert_eq!(touch(&mut n, 0, 7, 300, 150), [at(0, 0.3, 0.3)]);
        // A report with no change says nothing.
        assert!(report(&mut n, &[]).is_empty());
        assert_eq!(report(&mut n, &[abs(Abs::ABS_MT_TRACKING_ID, -1)]), [lift(0)]);
    }

    #[test]
    fn positions_are_measured_from_the_pads_corner_and_kept_on_it() {
        let mut n = TouchNormalizer::with_size((100, 50), (1000.0, 500.0));
        assert_eq!(touch(&mut n, 0, 1, 600, 300), [at(0, 0.5, 0.5)]);
        assert_eq!(touch(&mut n, 0, 1, 2000, 0), [at(0, 1.0, 0.0)]);
    }

    #[test]
    fn each_finger_is_reported_by_its_slot() {
        let mut n = norm();
        touch(&mut n, 0, 1, 100, 100);
        assert_eq!(touch(&mut n, 1, 2, 500, 250), [at(1, 0.5, 0.5)]);
        // Both move in one report.
        let both = report(
            &mut n,
            &[abs(Abs::ABS_MT_SLOT, 0), abs(Abs::ABS_MT_POSITION_X, 200), abs(Abs::ABS_MT_SLOT, 1), abs(Abs::ABS_MT_POSITION_X, 600)],
        );
        assert_eq!(both, [at(0, 0.2, 0.2), at(1, 0.6, 0.5)]);
        assert_eq!(report(&mut n, &[abs(Abs::ABS_MT_SLOT, 0), abs(Abs::ABS_MT_TRACKING_ID, -1)]), [lift(0)]);
    }

    #[test]
    fn a_new_contact_in_the_same_slot_lifts_the_old_one_first() {
        let mut n = norm();
        touch(&mut n, 0, 7, 100, 100);
        assert_eq!(touch(&mut n, 0, 8, 900, 400), [lift(0), at(0, 0.9, 0.8)]);
    }

    #[test]
    fn click_reports_press_and_release_and_ignores_repeats() {
        let mut n = norm();
        let key = |value| Raw::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, value);
        assert_eq!(report(&mut n, &[key(1)]), vec![InputEvent::Touchpad(TouchpadEvent::Click(true))]);
        assert!(report(&mut n, &[key(2)]).is_empty());
        assert_eq!(report(&mut n, &[key(0)]), vec![InputEvent::Touchpad(TouchpadEvent::Click(false))]);
    }
}
