//! The virtual devices the daemon writes to: uinput on Linux, ViGEmBus and SendInput on Windows.

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::{
    config::{Button, MouseButton},
    input::Axis,
    keycode::KeyCode,
};

#[cfg_attr(target_os = "linux", path = "output/uinput.rs")]
#[cfg_attr(windows, path = "output/windows.rs")]
mod device;
#[cfg(target_os = "linux")]
mod frames;

pub use device::{VirtualKbm, VirtualPad};
#[cfg(target_os = "linux")]
pub use device::{FfCaps, VIRTUAL_PREFIX};
#[cfg(windows)]
pub use device::{Target, is_virtual_slot, slots_known};

/// Abstract output produced by the mapping engine.
#[derive(Debug, Clone, PartialEq)]
pub enum OutEvent {
    PadButton(Button, bool),
    PadAxis(Axis, f32),
    Key(KeyCode, bool),
    MouseButton(MouseButton, bool),
    MouseMove(i32, i32),
    /// High-resolution wheel units (120 = one notch).
    Wheel { vertical: i32, horizontal: i32 },
}

fn stick_raw(v: f32) -> i32 {
    (v.clamp(-1.0, 1.0) * 32767.0).round() as i32
}

/// Which keys and buttons the daemon wants down, and which the kernel was last sent. They differ
/// only after a write has failed, and the difference is sent again on the next sync.
#[derive(Default)]
struct PressState {
    /// Presses per source, so two inputs mapped to one key don't release it early.
    held: HashMap<KeyCode, u32>,
    /// Keys and buttons the kernel has been sent a press for, and no release yet.
    sent: HashSet<KeyCode>,
}

impl PressState {
    fn press(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            *self.held.entry(code).or_insert(0) += 1;
        } else if let Some(count) = self.held.get_mut(&code) {
            *count -= 1;
            if *count == 0 {
                self.held.remove(&code);
            }
        }
    }

    /// Forgets every press; the next sync releases each key that was sent.
    fn release_all(&mut self) {
        self.held.clear();
    }

    /// The keys and buttons the kernel has been sent a press for.
    fn sent(&self) -> &HashSet<KeyCode> {
        &self.sent
    }

    /// Goes back to `sent` after writes that `sync` counted as made failed, so the next sync
    /// makes them again.
    fn restore_sent(&mut self, sent: HashSet<KeyCode>) {
        self.sent = sent;
    }

    /// Sends each key whose state differs from what was sent, in code order. Stops at the first
    /// failed write, and the rest stay pending for the next call.
    fn sync(&mut self, mut write: impl FnMut(KeyCode, bool) -> Result<()>) -> Result<()> {
        let mut codes: Vec<KeyCode> = self.held.keys().chain(&self.sent).copied().collect();
        codes.sort_by_key(|c| c.0);
        codes.dedup();
        for code in codes {
            let want = self.held.contains_key(&code);
            if want != self.sent.contains(&code) {
                write(code, want)?;
                if want {
                    self.sent.insert(code);
                } else {
                    self.sent.remove(&code);
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn mouse_code(b: MouseButton) -> KeyCode {
    match b {
        MouseButton::Left => KeyCode::BTN_LEFT,
        MouseButton::Right => KeyCode::BTN_RIGHT,
        MouseButton::Middle => KeyCode::BTN_MIDDLE,
        MouseButton::Back => KeyCode::BTN_SIDE,
        MouseButton::Forward => KeyCode::BTN_EXTRA,
    }
}

/// Moves the mouse `px` pixels to the right, smoothly over a second, after a short wait that
/// lets the user switch to a game. For checking how far that turns the game's camera. Runs on
/// its own thread with its own virtual mouse.
pub fn test_turn(px: i32) {
    use std::{thread, time::Duration};
    const WAIT: Duration = Duration::from_secs(3);
    const STEPS: i32 = 100;
    thread::spawn(move || {
        let Ok(mut kbm) = VirtualKbm::new() else { return };
        thread::sleep(WAIT);
        let mut sent = 0;
        for step in 1..=STEPS {
            let target = (i64::from(px) * i64::from(step) / i64::from(STEPS)) as i32;
            kbm.mouse_move(target - sent, 0);
            if kbm.flush().is_err() {
                return;
            }
            sent = target;
            thread::sleep(Duration::from_millis(10));
        }
    });
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::*;

    /// Sends what `state` wants down to a fake kernel (a set of codes), refusing every write
    /// when `refuse` is set.
    fn sync(state: &mut PressState, kernel: &mut HashSet<KeyCode>, refuse: bool) -> Result<()> {
        state.sync(|code, down| {
            if refuse {
                bail!("write refused");
            }
            if down {
                kernel.insert(code);
            } else {
                kernel.remove(&code);
            }
            Ok(())
        })
    }

    #[test]
    fn presses_are_counted_per_source() {
        let (mut state, mut kernel) = (PressState::default(), HashSet::new());
        state.press(KeyCode::KEY_A, true);
        state.press(KeyCode::KEY_A, true);
        sync(&mut state, &mut kernel, false).unwrap();
        state.press(KeyCode::KEY_A, false);
        sync(&mut state, &mut kernel, false).unwrap();
        assert!(kernel.contains(&KeyCode::KEY_A), "one source still holds the key");
        state.press(KeyCode::KEY_A, false);
        sync(&mut state, &mut kernel, false).unwrap();
        assert!(kernel.is_empty());
    }

    #[test]
    fn a_refused_release_is_sent_again_on_a_later_sync() {
        let (mut state, mut kernel) = (PressState::default(), HashSet::new());
        state.press(KeyCode::KEY_A, true);
        sync(&mut state, &mut kernel, false).unwrap();

        state.press(KeyCode::KEY_A, false);
        assert!(sync(&mut state, &mut kernel, true).is_err());
        assert!(kernel.contains(&KeyCode::KEY_A), "the kernel still has the key down");

        // Nothing else is pressed, and the next sync still releases it.
        sync(&mut state, &mut kernel, false).unwrap();
        assert!(kernel.is_empty());
    }

    #[test]
    fn a_refused_press_is_sent_before_the_next_key() {
        let (mut state, mut kernel) = (PressState::default(), HashSet::new());
        state.press(KeyCode::KEY_A, true);
        assert!(sync(&mut state, &mut kernel, true).is_err());
        state.press(KeyCode::KEY_B, true);
        sync(&mut state, &mut kernel, false).unwrap();
        assert_eq!(kernel, HashSet::from([KeyCode::KEY_A, KeyCode::KEY_B]));
    }
}
