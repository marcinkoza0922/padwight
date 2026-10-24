//! What a virtual device is sent, gathered into reports and written all at once.
//!
//! The engine outputs one change at a time (a stick's X, then its Y). Sent one by one, every change
//! is a report of its own (a write, then a SYN_REPORT write), so a diagonal reaches the game as
//! two moves and costs four system calls. Gathered here, changes to axes and relative motion merge
//! into one report, and everything waiting goes out in a single write on `flush`.

use std::{io, os::fd::AsRawFd};

use evdev::{AbsoluteAxisCode as Abs, EventType, InputEvent, KeyCode, RelativeAxisCode as Rel, SynchronizationCode};

/// The number of absolute axis codes (`ABS_CNT`).
const ABS_COUNT: usize = 0x40;

pub(super) struct Frames {
    events: Vec<InputEvent>,
    /// Where the report not yet ended by a SYN_REPORT starts.
    open: usize,
    /// The value each absolute axis has once what is waiting is written, when known, so an
    /// unchanged one isn't sent.
    abs_last: [Option<i32>; ABS_COUNT],
}

impl Default for Frames {
    fn default() -> Self {
        Frames { events: Vec::new(), open: 0, abs_last: [None; ABS_COUNT] }
    }
}

impl Frames {
    /// A key or button goes in a report of its own, as a real device sends a change, so a press and
    /// release of one key in the same batch both arrive.
    pub(super) fn key(&mut self, code: KeyCode, down: bool) {
        self.end();
        self.events.push(InputEvent::new(EventType::KEY.0, code.0, down as i32));
        self.end();
    }

    /// An axis's new value. It replaces a value the open report already has for the axis, and
    /// is dropped when the axis has it already.
    pub(super) fn abs(&mut self, code: Abs, value: i32) {
        if let Some(last) = self.abs_last.get_mut(code.0 as usize) {
            if *last == Some(value) {
                return;
            }
            *last = Some(value);
        }
        let event = InputEvent::new(EventType::ABSOLUTE.0, code.0, value);
        match self.pending(EventType::ABSOLUTE, code.0) {
            Some(i) => self.events[i] = event,
            None => self.events.push(event),
        }
    }

    /// Relative motion, added to what the open report already moves on that axis.
    pub(super) fn rel(&mut self, code: Rel, delta: i32) {
        if delta == 0 {
            return;
        }
        match self.pending(EventType::RELATIVE, code.0) {
            Some(i) => {
                let sum = self.events[i].value().saturating_add(delta);
                self.events[i] = InputEvent::new(EventType::RELATIVE.0, code.0, sum);
            }
            None => self.events.push(InputEvent::new(EventType::RELATIVE.0, code.0, delta)),
        }
    }

    /// Writes every report waiting, in one system call. On failure they are dropped, and the axes
    /// are sent in full next time, as what the kernel has is no longer known.
    pub(super) fn flush(&mut self, dev: &impl AsRawFd) -> io::Result<()> {
        self.end();
        if self.events.is_empty() {
            return Ok(());
        }
        let written = write_all(dev.as_raw_fd(), &self.events);
        if written.is_err() {
            self.abs_last = [None; ABS_COUNT];
        }
        self.events.clear();
        self.open = 0;
        written
    }

    /// Ends the open report, unless it is empty (zero motion merged away included).
    fn end(&mut self) {
        let open = &self.events[self.open..];
        if open.iter().all(|ev| ev.event_type() == EventType::RELATIVE && ev.value() == 0) {
            self.events.truncate(self.open);
            return;
        }
        self.events.push(InputEvent::new(EventType::SYNCHRONIZATION.0, SynchronizationCode::SYN_REPORT.0, 0));
        self.open = self.events.len();
    }

    /// Where the open report has an event of this type and code.
    fn pending(&self, kind: EventType, code: u16) -> Option<usize> {
        self.events[self.open..].iter().position(|ev| ev.event_type() == kind && ev.code() == code).map(|i| self.open + i)
    }

    #[cfg(test)]
    pub(super) fn waiting(&mut self) -> Vec<(u16, u16, i32)> {
        self.end();
        self.events.iter().map(|ev| (ev.event_type().0, ev.code(), ev.value())).collect()
    }

    /// What `flush` does when the write succeeds.
    #[cfg(test)]
    fn written(&mut self) {
        self.end();
        self.events.clear();
        self.open = 0;
    }
}

/// Writes `events` to the device `fd` whole, retrying after a signal or a short write.
fn write_all(fd: i32, events: &[InputEvent]) -> io::Result<()> {
    // SAFETY: `InputEvent` is `repr(transparent)` over the kernel's `input_event`, so the slice is
    // that many plain structs, laid out as the uinput device reads them.
    let mut bytes = unsafe { std::slice::from_raw_parts(events.as_ptr().cast::<u8>(), std::mem::size_of_val(events)) };
    while !bytes.is_empty() {
        // SAFETY: the pointer and length describe the live slice above.
        let n = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        if n < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        if n == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        bytes = &bytes[n as usize..];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: u16 = EventType::KEY.0;
    const ABS: u16 = EventType::ABSOLUTE.0;
    const REL: u16 = EventType::RELATIVE.0;
    const SYN: (u16, u16, i32) = (EventType::SYNCHRONIZATION.0, 0, 0);

    #[test]
    fn a_stick_moving_on_both_axes_is_one_report() {
        let mut frames = Frames::default();
        // The engine sends both axes for each axis that changed.
        frames.abs(Abs::ABS_X, 100);
        frames.abs(Abs::ABS_Y, 0);
        frames.abs(Abs::ABS_X, 100);
        frames.abs(Abs::ABS_Y, -50);
        assert_eq!(frames.waiting(), vec![(ABS, 0, 100), (ABS, 1, -50), SYN]);
    }

    #[test]
    fn a_tap_of_one_key_keeps_both_halves_in_order() {
        let mut frames = Frames::default();
        frames.abs(Abs::ABS_X, 7);
        frames.key(KeyCode::KEY_A, true);
        frames.key(KeyCode::KEY_A, false);
        frames.abs(Abs::ABS_X, 8);
        let a = KeyCode::KEY_A.0;
        assert_eq!(frames.waiting(), vec![(ABS, 0, 7), SYN, (KEY, a, 1), SYN, (KEY, a, 0), SYN, (ABS, 0, 8), SYN]);
    }

    #[test]
    fn mouse_motion_adds_up_and_nothing_left_sends_nothing() {
        let mut frames = Frames::default();
        frames.rel(Rel::REL_X, 3);
        frames.rel(Rel::REL_X, 4);
        frames.rel(Rel::REL_Y, -2);
        assert_eq!(frames.waiting(), vec![(REL, 0, 7), (REL, 1, -2), SYN]);

        let mut frames = Frames::default();
        frames.rel(Rel::REL_X, 3);
        frames.rel(Rel::REL_X, -3);
        assert!(frames.waiting().is_empty());
    }

    #[test]
    fn an_axis_that_already_has_the_value_is_not_sent_again() {
        let mut frames = Frames::default();
        frames.abs(Abs::ABS_X, 5);
        frames.written();
        frames.abs(Abs::ABS_X, 5);
        frames.abs(Abs::ABS_Y, 5);
        assert_eq!(frames.waiting(), vec![(ABS, 1, 5), SYN]);
    }

    #[test]
    fn an_axis_going_back_within_a_batch_is_sent() {
        let mut frames = Frames::default();
        frames.abs(Abs::ABS_X, 5);
        frames.written();
        // Changed in a report the key ended, then back to what was written before.
        frames.abs(Abs::ABS_X, 7);
        frames.key(KeyCode::KEY_A, true);
        frames.abs(Abs::ABS_X, 5);
        let a = KeyCode::KEY_A.0;
        assert_eq!(frames.waiting(), vec![(ABS, 0, 7), SYN, (KEY, a, 1), SYN, (ABS, 0, 5), SYN]);
    }
}
