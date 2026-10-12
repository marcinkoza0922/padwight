//! Windows' half of input: XInput's four controller slots, read by polling, and their rumble.
//!
//! XInput leaves out the Guide button and the controller's USB IDs. Both come from functions
//! `xinput1_4.dll` exports by number only (`XInputGetStateEx`, `XInputGetCapabilitiesEx`), which
//! Steam, SDL and others rely on; without them the Guide button is never pressed and the model
//! goes unrecognized.

use std::sync::OnceLock;

use windows::{
    Win32::{
        Foundation::{ERROR_SUCCESS, HMODULE},
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
        UI::Input::XboxController::{
            XINPUT_CAPABILITIES, XINPUT_FLAG_GAMEPAD, XINPUT_GAMEPAD, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK,
            XINPUT_GAMEPAD_BUTTON_FLAGS, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT,
            XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_SHOULDER,
            XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y, XINPUT_STATE, XINPUT_VIBRATION,
            XInputGetCapabilities, XInputGetState, XInputSetState,
        },
    },
    core::{PCSTR, w},
};

use super::{Axis, InputEvent};
use crate::config::Button;

/// XInput has four slots, 0 to 3.
pub const SLOTS: u32 = 4;

/// The path a controller in `slot` goes by, where Linux would give its device node.
pub fn slot_path(slot: u32) -> std::path::PathBuf {
    format!("xinput:{slot}").into()
}

/// The slot of a controller path from [`slot_path`].
pub fn slot_of(path: &std::path::Path) -> Option<u32> {
    path.to_str()?.strip_prefix("xinput:")?.parse().ok().filter(|slot| *slot < SLOTS)
}

/// The Guide button's bit, which only `XInputGetStateEx` reports.
const GUIDE: u16 = 0x0400;

/// Each button bit and the button it is. Xbox pads label their west button X and the north one
/// Y, so these are positions, as the engine's buttons are.
const BUTTONS: [(XINPUT_GAMEPAD_BUTTON_FLAGS, Button); 15] = [
    (XINPUT_GAMEPAD_A, Button::South),
    (XINPUT_GAMEPAD_B, Button::East),
    (XINPUT_GAMEPAD_X, Button::West),
    (XINPUT_GAMEPAD_Y, Button::North),
    (XINPUT_GAMEPAD_LEFT_SHOULDER, Button::LeftBumper),
    (XINPUT_GAMEPAD_RIGHT_SHOULDER, Button::RightBumper),
    (XINPUT_GAMEPAD_BACK, Button::Select),
    (XINPUT_GAMEPAD_START, Button::Start),
    (XINPUT_GAMEPAD_BUTTON_FLAGS(GUIDE), Button::Guide),
    (XINPUT_GAMEPAD_LEFT_THUMB, Button::LeftStick),
    (XINPUT_GAMEPAD_RIGHT_THUMB, Button::RightStick),
    (XINPUT_GAMEPAD_DPAD_UP, Button::DpadUp),
    (XINPUT_GAMEPAD_DPAD_DOWN, Button::DpadDown),
    (XINPUT_GAMEPAD_DPAD_LEFT, Button::DpadLeft),
    (XINPUT_GAMEPAD_DPAD_RIGHT, Button::DpadRight),
];

type GetStateEx = unsafe extern "system" fn(u32, *mut XINPUT_STATE) -> u32;
type GetCapabilitiesEx = unsafe extern "system" fn(u32, u32, u32, *mut CapabilitiesEx) -> u32;

/// `XINPUT_CAPABILITIES_EX`, which no header declares.
#[repr(C)]
#[derive(Default)]
struct CapabilitiesEx {
    caps: XINPUT_CAPABILITIES,
    vendor: u16,
    product: u16,
    version: u16,
    _unknown1: u16,
    _unknown2: u32,
}

/// The functions `xinput1_4.dll` exports by number, if it has them.
struct Hidden {
    get_state_ex: Option<GetStateEx>,
    get_capabilities_ex: Option<GetCapabilitiesEx>,
}

fn hidden() -> &'static Hidden {
    static HIDDEN: OnceLock<Hidden> = OnceLock::new();
    HIDDEN.get_or_init(|| {
        // SAFETY: loading a system DLL by name; the handle is kept for the life of the process.
        let module = unsafe { LoadLibraryW(w!("xinput1_4.dll")) }.unwrap_or(HMODULE::default());
        if module.is_invalid() {
            return Hidden { get_state_ex: None, get_capabilities_ex: None };
        }
        // SAFETY: GetProcAddress with an ordinal; the signatures are the ones these exports have.
        unsafe {
            let ordinal = |n: usize| GetProcAddress(module, PCSTR(n as *const u8));
            Hidden {
                get_state_ex: ordinal(100).map(|f| std::mem::transmute::<_, GetStateEx>(f)),
                get_capabilities_ex: ordinal(108).map(|f| std::mem::transmute::<_, GetCapabilitiesEx>(f)),
            }
        }
    })
}

/// The controller in `slot` right now, with the Guide button when XInput will say; `None` when
/// the slot is empty.
pub fn state(slot: u32) -> Option<XINPUT_STATE> {
    let mut state = XINPUT_STATE::default();
    // SAFETY: `state` is a valid XINPUT_STATE for the call to fill.
    let result = unsafe {
        match hidden().get_state_ex {
            Some(get_state_ex) => get_state_ex(slot, &mut state),
            None => XInputGetState(slot, &mut state),
        }
    };
    (result == ERROR_SUCCESS.0).then_some(state)
}

/// What a connected controller is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controller {
    /// USB vendor and product, when XInput says (0 otherwise).
    pub vendor: u16,
    pub product: u16,
    pub rumble: bool,
}

impl Controller {
    /// A name for the controller in `slot`, since XInput gives none: what it is, and its slot
    /// (Windows' "player" number, counted from 1).
    pub fn name(&self, slot: u32) -> String {
        let kind = match crate::info::PadModel::detect(self.vendor, self.product, "") {
            Some(model) => model.label(),
            None => "Xbox Controller",
        };
        format!("{kind} (player {})", slot + 1)
    }
}

/// The controller in `slot`, if one is connected and it is a gamepad.
pub fn controller(slot: u32) -> Option<Controller> {
    let mut ex = CapabilitiesEx::default();
    // SAFETY: each call gets a valid structure of the type it fills.
    let (ok, caps) = unsafe {
        match hidden().get_capabilities_ex {
            Some(get_ex) if get_ex(1, slot, XINPUT_FLAG_GAMEPAD.0, &mut ex) == ERROR_SUCCESS.0 => (true, ex.caps),
            _ => {
                let mut caps = XINPUT_CAPABILITIES::default();
                (XInputGetCapabilities(slot, XINPUT_FLAG_GAMEPAD, &mut caps) == ERROR_SUCCESS.0, caps)
            }
        }
    };
    if !ok {
        return None;
    }
    let rumble = caps.Vibration.wLeftMotorSpeed != 0 || caps.Vibration.wRightMotorSpeed != 0;
    Some(Controller { vendor: ex.vendor, product: ex.product, rumble })
}

/// Sets the controller's motors: `strong` is the low-frequency (left) one, `weak` the
/// high-frequency (right) one. False if the slot is empty.
pub fn set_rumble(slot: u32, strong: u16, weak: u16) -> bool {
    let vibration = XINPUT_VIBRATION { wLeftMotorSpeed: strong, wRightMotorSpeed: weak };
    // SAFETY: `vibration` is valid for the duration of the call.
    unsafe { XInputSetState(slot, &vibration) == ERROR_SUCCESS.0 }
}

/// Turns successive XInput states into the changes between them.
#[derive(Default)]
pub struct XInputNormalizer {
    last: XINPUT_GAMEPAD,
    packet: Option<u32>,
}

impl XInputNormalizer {
    /// What changed since the last state.
    pub fn translate(&mut self, state: &XINPUT_STATE, out: &mut Vec<InputEvent>) {
        if self.packet == Some(state.dwPacketNumber) {
            return;
        }
        self.packet = Some(state.dwPacketNumber);
        let (old, new) = (self.last, state.Gamepad);
        let changed = old.wButtons.0 ^ new.wButtons.0;
        for (bit, button) in BUTTONS {
            if changed & bit.0 != 0 {
                out.push(InputEvent::Button(button, new.wButtons.0 & bit.0 != 0));
            }
        }
        // XInput's Y axes point up; the engine's point down.
        let stick = |v: i16| (f32::from(v) / 32767.0).clamp(-1.0, 1.0);
        let trigger = |v: u8| f32::from(v) / 255.0;
        let axes = [
            (Axis::LeftX, old.sThumbLX != new.sThumbLX, stick(new.sThumbLX)),
            (Axis::LeftY, old.sThumbLY != new.sThumbLY, -stick(new.sThumbLY)),
            (Axis::RightX, old.sThumbRX != new.sThumbRX, stick(new.sThumbRX)),
            (Axis::RightY, old.sThumbRY != new.sThumbRY, -stick(new.sThumbRY)),
            (Axis::LeftTrigger, old.bLeftTrigger != new.bLeftTrigger, trigger(new.bLeftTrigger)),
            (Axis::RightTrigger, old.bRightTrigger != new.bRightTrigger, trigger(new.bRightTrigger)),
        ];
        for (axis, moved, value) in axes {
            if moved {
                out.push(InputEvent::Axis(axis, value));
            }
        }
        self.last = new;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(packet: u32, buttons: u16, lx: i16, ly: i16, lt: u8) -> XINPUT_STATE {
        XINPUT_STATE {
            dwPacketNumber: packet,
            Gamepad: XINPUT_GAMEPAD {
                wButtons: XINPUT_GAMEPAD_BUTTON_FLAGS(buttons),
                bLeftTrigger: lt,
                sThumbLX: lx,
                sThumbLY: ly,
                ..Default::default()
            },
        }
    }

    #[test]
    fn changes_become_events() {
        let mut norm = XInputNormalizer::default();
        let mut out = Vec::new();
        norm.translate(&state(1, XINPUT_GAMEPAD_A.0 | GUIDE, 32767, 32767, 255), &mut out);
        assert_eq!(
            out,
            [
                InputEvent::Button(Button::South, true),
                InputEvent::Button(Button::Guide, true),
                InputEvent::Axis(Axis::LeftX, 1.0),
                InputEvent::Axis(Axis::LeftY, -1.0),
                InputEvent::Axis(Axis::LeftTrigger, 1.0),
            ],
            "stick up is negative, as on Linux"
        );
        out.clear();
        norm.translate(&state(1, 0, 0, 0, 0), &mut out);
        assert!(out.is_empty(), "the same packet is read once");
        norm.translate(&state(2, XINPUT_GAMEPAD_X.0, 32767, 32767, 255), &mut out);
        assert_eq!(
            out,
            [InputEvent::Button(Button::South, false), InputEvent::Button(Button::West, true), InputEvent::Button(Button::Guide, false)],
            "X is the west button"
        );
    }
}
