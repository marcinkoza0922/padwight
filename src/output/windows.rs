//! The virtual devices on Windows: an Xbox 360 pad through the ViGEmBus driver, and the keyboard
//! and mouse through `SendInput`. Keys go out as hardware scancodes, so games that read raw input
//! or DirectInput see them like a real keyboard's.

use std::sync::{
    Arc, Mutex, OnceLock, PoisonError,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use anyhow::{Result, bail};
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};
use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_MOVE, MOUSEEVENTF_WHEEL};

use super::{PressState, mouse_code, stick_raw};

#[path = "windows/sendinput.rs"]
mod sendinput;
use sendinput::{key_input, mouse_button_input, mouse_input, send};
use crate::{
    config::{Button, MouseButton},
    input::Axis,
    keycode::KeyCode,
    monitor::log,
    pad_identity::PadIdentity,
};

/// The ViGEmBus connection every virtual pad shares, or why there is none.
fn client() -> Result<Arc<Client>> {
    static CLIENT: OnceLock<Result<Arc<Client>, String>> = OnceLock::new();
    match CLIENT.get_or_init(|| Client::connect().map(Arc::new).map_err(|e| e.to_string())) {
        Ok(client) => Ok(client.clone()),
        Err(e) => bail!("ViGEmBus is not available ({e}); install it to get a virtual pad"),
    }
}

/// The XInput slots our own virtual pads took, so the input scanner never reads them back.
static VIRTUAL_SLOTS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// How many of our virtual pads took a slot XInput hasn't told us yet.
static UNPLACED: AtomicUsize = AtomicUsize::new(0);

/// Whether XInput slot `slot` is one of our virtual pads. While any pad's slot is unknown, every
/// slot might be, so new controllers wait (see [`slots_known`]).
pub fn is_virtual_slot(slot: u32) -> bool {
    VIRTUAL_SLOTS.lock().unwrap_or_else(PoisonError::into_inner).contains(&slot)
}

/// Whether every virtual pad's slot is known, so [`is_virtual_slot`] can be trusted.
pub fn slots_known() -> bool {
    UNPLACED.load(Ordering::Relaxed) == 0
}

pub type Target = Xbox360Wired<Arc<Client>>;

pub struct VirtualPad {
    /// Shared with the rumble thread, which asks the driver for the game's rumble. `None` when
    /// ViGEmBus isn't installed: keyboard and mouse output still work, the pad's is dropped.
    target: Option<Arc<Mutex<Target>>>,
    /// The XInput slot the virtual pad took.
    slot: Option<u32>,
    report: XGamepad,
    /// Whether `report` changed since it was last sent (see `flush`).
    dirty: bool,
    /// What the pad was asked to present as. ViGEmBus pads are always Xbox 360 pads here.
    identity: PadIdentity,
}

impl VirtualPad {
    /// An Xbox 360 pad.
    pub fn new() -> Result<Self> {
        Self::with_identity(PadIdentity::Xbox360)
    }

    /// A pad for a game that asked for `identity`. On Windows it is always an Xbox 360 pad, the
    /// one XInput games read; the identity is kept so it isn't rebuilt each time it's asked for.
    pub fn with_identity(identity: PadIdentity) -> Result<Self> {
        static NO_BUS: AtomicBool = AtomicBool::new(false);
        let client = match client() {
            Ok(client) => client,
            Err(e) => {
                if !NO_BUS.swap(true, Ordering::Relaxed) {
                    log!("{e:#}");
                }
                return Ok(VirtualPad { target: None, slot: None, report: XGamepad::default(), dirty: false, identity });
            }
        };
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
        target.plugin()?;
        target.wait_ready()?;
        let slot = user_index(&mut target);
        match slot {
            Some(slot) => VIRTUAL_SLOTS.lock().unwrap_or_else(PoisonError::into_inner).push(slot),
            None => {
                log!("XInput hasn't placed the virtual pad; new controllers wait until it goes");
                UNPLACED.fetch_add(1, Ordering::Relaxed);
            }
        }
        if identity != PadIdentity::Xbox360 {
            log!("the game asked for a {identity:?} pad; Windows presents an Xbox 360 pad");
        }
        Ok(VirtualPad { target: Some(Arc::new(Mutex::new(target))), slot, report: XGamepad::default(), dirty: false, identity })
    }

    /// The controller this pad was asked to present itself as.
    pub fn identity(&self) -> PadIdentity {
        self.identity
    }

    /// The driver target, for rumble forwarding; `None` without ViGEmBus.
    pub fn shared(&self) -> Option<Arc<Mutex<Target>>> {
        self.target.clone()
    }

    /// Sends the report, if `button` or `axis` changed it, in one update to the driver.
    pub fn flush(&mut self) -> Result<()> {
        if !std::mem::take(&mut self.dirty) {
            return Ok(());
        }
        if let Some(target) = &self.target {
            let sent = target.lock().unwrap_or_else(PoisonError::into_inner).update(&self.report);
            // Sent again next time, whatever else changes.
            self.dirty = sent.is_err();
            sent?;
        }
        Ok(())
    }

    /// Changes a button in the report, sent by the next `flush`.
    pub fn button(&mut self, b: Button, pressed: bool) {
        let bit = match b {
            Button::South => XButtons::A,
            Button::East => XButtons::B,
            Button::North => XButtons::Y,
            Button::West => XButtons::X,
            Button::LeftBumper => XButtons::LB,
            Button::RightBumper => XButtons::RB,
            Button::Select => XButtons::BACK,
            Button::Start => XButtons::START,
            Button::Guide => XButtons::GUIDE,
            Button::LeftStick => XButtons::LTHUMB,
            Button::RightStick => XButtons::RTHUMB,
            Button::DpadUp => XButtons::UP,
            Button::DpadDown => XButtons::DOWN,
            Button::DpadLeft => XButtons::LEFT,
            Button::DpadRight => XButtons::RIGHT,
            // The engine turns stick directions into stick deflection; nothing to press.
            // Paddles aren't on the Xbox pad, so they have no output.
            Button::LeftStickUp
            | Button::LeftStickDown
            | Button::LeftStickLeft
            | Button::LeftStickRight
            | Button::RightStickUp
            | Button::RightStickDown
            | Button::RightStickLeft
            | Button::RightStickRight
            | Button::LeftPaddle
            | Button::RightPaddle
            | Button::LeftPaddle2
            | Button::RightPaddle2 => return,
        };
        if pressed {
            self.report.buttons.raw |= bit;
        } else {
            self.report.buttons.raw &= !bit;
        }
        self.dirty = true;
    }

    /// Changes an axis in the report, sent by the next `flush`.
    pub fn axis(&mut self, axis: Axis, value: f32) {
        // XInput's Y axes point up; the engine's point down.
        let stick = |v: f32| stick_raw(v) as i16;
        let trigger = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        match axis {
            Axis::LeftX => self.report.thumb_lx = stick(value),
            Axis::LeftY => self.report.thumb_ly = stick(-value),
            Axis::RightX => self.report.thumb_rx = stick(value),
            Axis::RightY => self.report.thumb_ry = stick(-value),
            Axis::LeftTrigger => self.report.left_trigger = trigger(value),
            Axis::RightTrigger => self.report.right_trigger = trigger(value),
        }
        self.dirty = true;
    }
}

/// The XInput slot a just-plugged pad took. XInput finds the pad a moment after it is ready, so
/// this asks for up to a second.
fn user_index(target: &mut Target) -> Option<u32> {
    const TRIES: u32 = 20;
    for _ in 0..TRIES {
        if let Ok(slot) = target.get_user_index() {
            return Some(slot);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    None
}

impl Drop for VirtualPad {
    fn drop(&mut self) {
        if self.target.is_some() && self.slot.is_none() {
            UNPLACED.fetch_sub(1, Ordering::Relaxed);
        }
        if let Some(slot) = self.slot {
            VIRTUAL_SLOTS.lock().unwrap_or_else(PoisonError::into_inner).retain(|s| *s != slot);
        }
    }
}

/// Shared virtual keyboard + mouse. Presses are reference counted (see `PressState`), and a
/// press or release Windows refused is sent again by `sync`. Keys, buttons and motion are queued
/// and sent by `flush`.
pub struct VirtualKbm {
    press: PressState,
    queued: Vec<INPUT>,
    /// What Windows had been sent before the presses queued now, to go back to if sending them
    /// fails, so `sync` sends them again.
    confirmed: Option<std::collections::HashSet<KeyCode>>,
}

impl VirtualKbm {
    pub fn new() -> Result<Self> {
        Ok(VirtualKbm { press: PressState::default(), queued: Vec::new(), confirmed: None })
    }

    /// Queues a press or release, for the next `flush`.
    pub fn key(&mut self, code: KeyCode, pressed: bool) {
        self.press.press(code, pressed);
        self.queue_changes();
    }

    /// Queues a press or release, for the next `flush`.
    pub fn mouse_button(&mut self, b: MouseButton, pressed: bool) {
        self.press.press(mouse_code(b), pressed);
        self.queue_changes();
    }

    /// Releases every key and button, including ones the on-screen keyboard holds.
    pub fn release_all(&mut self) -> Result<()> {
        self.press.release_all();
        self.sync()
    }

    /// Brings Windows in line with what is held, at once. The daemon's periodic scan calls it, so
    /// a release that failed is sent again even if nothing else is pressed.
    pub fn sync(&mut self) -> Result<()> {
        self.queue_changes();
        self.flush()
    }

    /// Sends everything queued in one `SendInput`, which Windows doesn't interleave with other
    /// input. If it fails, the presses in it count as not sent, so the next `sync` sends them again.
    pub fn flush(&mut self) -> Result<()> {
        let confirmed = self.confirmed.take();
        if self.queued.is_empty() {
            return Ok(());
        }
        let sent = send(&self.queued);
        self.queued.clear();
        if sent.is_err()
            && let Some(before) = confirmed
        {
            self.press.restore_sent(before);
        }
        sent
    }

    /// Queues each key whose state differs from what was sent (see `PressState::sync`).
    fn queue_changes(&mut self) {
        let Self { press, queued, confirmed } = self;
        if confirmed.is_none() {
            *confirmed = Some(press.sent().clone());
        }
        let done: Result<()> = press.sync(|code, down| {
            match MouseButton::ALL.iter().find(|b| mouse_code(**b) == code) {
                Some(b) => queued.push(mouse_button_input(*b, down)),
                None => queued.extend(key_input(code, down)),
            }
            Ok(())
        });
        debug_assert!(done.is_ok(), "queueing never fails");
    }

    /// Queues motion, for the next `flush`.
    pub fn mouse_move(&mut self, dx: i32, dy: i32) {
        if dx != 0 || dy != 0 {
            self.queued.push(mouse_input(dx, dy, 0, MOUSEEVENTF_MOVE));
        }
    }

    /// Takes hi-res units (120 per notch), which is what Windows counts in too. Queued, for the
    /// next `flush`.
    pub fn wheel(&mut self, vertical: i32, horizontal: i32) {
        if vertical != 0 {
            self.queued.push(mouse_input(0, 0, vertical, MOUSEEVENTF_WHEEL));
        }
        if horizontal != 0 {
            self.queued.push(mouse_input(0, 0, horizontal, MOUSEEVENTF_HWHEEL));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends real input, so it is opt-in: `cargo test sendinput -- --ignored`. F24 is on no
    /// keyboard, so nothing on the desktop reacts to it.
    #[test]
    #[ignore]
    fn sendinput_presses_reach_windows() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F24};
        let down = || unsafe { GetAsyncKeyState(i32::from(VK_F24.0)) } as u16 & 0x8000 != 0;
        let mut kbm = VirtualKbm::new().unwrap();
        kbm.key(KeyCode::KEY_F24, true);
        kbm.flush().unwrap();
        assert!(down(), "Windows has F24 down");
        kbm.release_all().unwrap();
        assert!(!down(), "and up again");
    }
}
