//! The virtual devices on Windows: an Xbox 360 pad through the ViGEmBus driver, and the keyboard
//! and mouse through `SendInput`. Keys go out as hardware scancodes, so games that read raw input
//! or DirectInput see them like a real keyboard's.

use std::sync::{
    Arc, Mutex, OnceLock, PoisonError,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use anyhow::{Result, bail};
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, SendInput, VIRTUAL_KEY,
};

use super::{PressState, mouse_code, stick_raw};
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
                return Ok(VirtualPad { target: None, slot: None, report: XGamepad::default(), identity });
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
        Ok(VirtualPad { target: Some(Arc::new(Mutex::new(target))), slot, report: XGamepad::default(), identity })
    }

    /// The controller this pad was asked to present itself as.
    pub fn identity(&self) -> PadIdentity {
        self.identity
    }

    /// The driver target, for rumble forwarding; `None` without ViGEmBus.
    pub fn shared(&self) -> Option<Arc<Mutex<Target>>> {
        self.target.clone()
    }

    fn send(&self) -> Result<()> {
        if let Some(target) = &self.target {
            target.lock().unwrap_or_else(PoisonError::into_inner).update(&self.report)?;
        }
        Ok(())
    }

    pub fn button(&mut self, b: Button, pressed: bool) -> Result<()> {
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
            | Button::RightPaddle2 => return Ok(()),
        };
        if pressed {
            self.report.buttons.raw |= bit;
        } else {
            self.report.buttons.raw &= !bit;
        }
        self.send()
    }

    pub fn axis(&mut self, axis: Axis, value: f32) -> Result<()> {
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
        self.send()
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
/// press or release Windows refused is sent again by `sync`.
pub struct VirtualKbm {
    press: PressState,
}

impl VirtualKbm {
    pub fn new() -> Result<Self> {
        Ok(VirtualKbm { press: PressState::default() })
    }

    pub fn key(&mut self, code: KeyCode, pressed: bool) -> Result<()> {
        self.press.press(code, pressed);
        self.sync()
    }

    pub fn mouse_button(&mut self, b: MouseButton, pressed: bool) -> Result<()> {
        self.press.press(mouse_code(b), pressed);
        self.sync()
    }

    /// Releases every key and button, including ones the on-screen keyboard holds.
    pub fn release_all(&mut self) -> Result<()> {
        self.press.release_all();
        self.sync()
    }

    /// Brings Windows in line with what is held. Called after every key or button, and by the
    /// daemon's periodic scan, so a release that failed is sent again even if nothing else is
    /// pressed.
    pub fn sync(&mut self) -> Result<()> {
        self.press.sync(|code, down| match MouseButton::ALL.iter().find(|b| mouse_code(**b) == code) {
            Some(b) => send(&[mouse_button_input(*b, down)]),
            None => match key_input(code, down) {
                Some(input) => send(&[input]),
                None => Ok(()),
            },
        })
    }

    pub fn mouse_move(&mut self, dx: i32, dy: i32) -> Result<()> {
        if dx == 0 && dy == 0 {
            return Ok(());
        }
        send(&[mouse_input(dx, dy, 0, MOUSEEVENTF_MOVE)])
    }

    /// Takes hi-res units (120 per notch), which is what Windows counts in too.
    pub fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<()> {
        let mut inputs = Vec::with_capacity(2);
        if vertical != 0 {
            inputs.push(mouse_input(0, 0, vertical, MOUSEEVENTF_WHEEL));
        }
        if horizontal != 0 {
            inputs.push(mouse_input(0, 0, horizontal, MOUSEEVENTF_HWHEEL));
        }
        if inputs.is_empty() {
            return Ok(());
        }
        send(&inputs)
    }
}

fn send(inputs: &[INPUT]) -> Result<()> {
    // SAFETY: `inputs` is a valid slice of initialized INPUT structures.
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        bail!("SendInput refused the input: {}", std::io::Error::last_os_error());
    }
    Ok(())
}

fn mouse_input(dx: i32, dy: i32, data: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

fn mouse_button_input(b: MouseButton, down: bool) -> INPUT {
    const XBUTTON1: i32 = 1;
    const XBUTTON2: i32 = 2;
    let pick = |on, off| if down { on } else { off };
    let (flags, data) = match b {
        MouseButton::Left => (pick(MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP), 0),
        MouseButton::Right => (pick(MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP), 0),
        MouseButton::Middle => (pick(MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP), 0),
        MouseButton::Back => (pick(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP), XBUTTON1),
        MouseButton::Forward => (pick(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP), XBUTTON2),
    };
    mouse_input(0, 0, data, flags)
}

/// How a key reaches Windows: by its scancode (with the E0 prefix, if `extended`), or by virtual
/// key for the media and browser keys, which have no scancode every layout agrees on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WinKey {
    Scan { code: u16, extended: bool },
    Virtual(u16),
}

fn key_input(code: KeyCode, down: bool) -> Option<INPUT> {
    let up = if down { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP };
    let ki = match win_key(code)? {
        WinKey::Scan { code, extended } => {
            let ext = if extended { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) };
            KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: code, dwFlags: KEYEVENTF_SCANCODE | ext | up, time: 0, dwExtraInfo: 0 }
        }
        WinKey::Virtual(vk) => KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: up, time: 0, dwExtraInfo: 0 },
    };
    Some(INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki } })
}

/// The Windows key for a Linux key code. Linux numbers the main block (Esc to F12) by the PC's
/// set 1 scancodes, so those pass straight through; the rest are looked up.
fn win_key(code: KeyCode) -> Option<WinKey> {
    let scan = |code| Some(WinKey::Scan { code, extended: false });
    let ext = |code| Some(WinKey::Scan { code, extended: true });
    let vk = |vk| Some(WinKey::Virtual(vk));
    match code.code() {
        n @ 1..=88 if n != 84 => scan(n),
        96 => ext(0x1c),  // KEY_KPENTER
        97 => ext(0x1d),  // KEY_RIGHTCTRL
        98 => ext(0x35),  // KEY_KPSLASH
        99 => vk(0x2c),   // KEY_SYSRQ: VK_SNAPSHOT
        100 => ext(0x38), // KEY_RIGHTALT
        102 => ext(0x47), // KEY_HOME
        103 => ext(0x48), // KEY_UP
        104 => ext(0x49), // KEY_PAGEUP
        105 => ext(0x4b), // KEY_LEFT
        106 => ext(0x4d), // KEY_RIGHT
        107 => ext(0x4f), // KEY_END
        108 => ext(0x50), // KEY_DOWN
        109 => ext(0x51), // KEY_PAGEDOWN
        110 => ext(0x52), // KEY_INSERT
        111 => ext(0x53), // KEY_DELETE
        113 => vk(0xad),  // KEY_MUTE: VK_VOLUME_MUTE
        114 => vk(0xae),  // KEY_VOLUMEDOWN
        115 => vk(0xaf),  // KEY_VOLUMEUP
        117 => scan(0x59), // KEY_KPEQUAL
        119 => vk(0x13),  // KEY_PAUSE: VK_PAUSE
        121 => scan(0x7e), // KEY_KPCOMMA
        124 => scan(0x7d), // KEY_YEN
        125 => ext(0x5b), // KEY_LEFTMETA
        126 => ext(0x5c), // KEY_RIGHTMETA
        127 => ext(0x5d), // KEY_COMPOSE: the menu key
        140 => vk(0xb7),  // KEY_CALC: VK_LAUNCH_APP2
        142 => vk(0x5f),  // KEY_SLEEP
        155 => vk(0xb4),  // KEY_MAIL
        158 => vk(0xa6),  // KEY_BACK: VK_BROWSER_BACK
        159 => vk(0xa7),  // KEY_FORWARD
        163 => vk(0xb0),  // KEY_NEXTSONG
        164 => vk(0xb3),  // KEY_PLAYPAUSE
        165 => vk(0xb1),  // KEY_PREVIOUSSONG
        166 => vk(0xb2),  // KEY_STOPCD
        172 => vk(0xac),  // KEY_HOMEPAGE
        173 => vk(0xa8),  // KEY_REFRESH
        n @ 183..=193 => scan(0x64 + (n - 183)), // KEY_F13..KEY_F23
        194 => scan(0x76), // KEY_F24
        207 => vk(0xfa),  // KEY_PLAY
        217 => vk(0xaa),  // KEY_SEARCH: VK_BROWSER_SEARCH
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_block_keys_are_their_scancodes() {
        assert_eq!(win_key(KeyCode::KEY_A), Some(WinKey::Scan { code: 0x1e, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_ESC), Some(WinKey::Scan { code: 0x01, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_F12), Some(WinKey::Scan { code: 0x58, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_UP), Some(WinKey::Scan { code: 0x48, extended: true }));
        assert_eq!(win_key(KeyCode::KEY_RIGHTCTRL), Some(WinKey::Scan { code: 0x1d, extended: true }));
        assert_eq!(win_key(KeyCode::KEY_F13), Some(WinKey::Scan { code: 0x64, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_PLAYPAUSE), Some(WinKey::Virtual(0xb3)));
        assert_eq!(win_key(KeyCode::BTN_SOUTH), None);
    }

    /// Sends real input, so it is opt-in: `cargo test sendinput -- --ignored`. F24 is on no
    /// keyboard, so nothing on the desktop reacts to it.
    #[test]
    #[ignore]
    fn sendinput_presses_reach_windows() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F24};
        let down = || unsafe { GetAsyncKeyState(i32::from(VK_F24.0)) } as u16 & 0x8000 != 0;
        let mut kbm = VirtualKbm::new().unwrap();
        kbm.key(KeyCode::KEY_F24, true).unwrap();
        assert!(down(), "Windows has F24 down");
        kbm.release_all().unwrap();
        assert!(!down(), "and up again");
    }
}
