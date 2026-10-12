//! The virtual uinput devices: a pad, a keyboard and a mouse the kernel shows to every program.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use evdev::{
    AbsInfo, AbsoluteAxisCode as Abs, AttributeSet, AttributeSetRef, BusType, EventType,
    FFEffectCode, InputEvent, InputId, KeyCode, RelativeAxisCode as Rel, UinputAbsSetup,
    uinput::VirtualDevice,
};

use super::{PressState, mouse_code, stick_raw};
use crate::{
    config::{Button, MouseButton},
    input::Axis,
    pad_identity::PadIdentity,
};

/// Prefix of every device we create, so the daemon never grabs its own output.
pub const VIRTUAL_PREFIX: &str = "Padwight Virtual";

pub struct VirtualPad {
    /// Shared with the rumble thread, which reads force-feedback requests from it.
    dev: Arc<Mutex<VirtualDevice>>,
    dpad: HashSet<Button>,
    identity: PadIdentity,
}

/// Force-feedback support to advertise, copied from the physical controller.
pub struct FfCaps<'a> {
    pub effects: &'a AttributeSetRef<FFEffectCode>,
    pub max_effects: u32,
}

impl VirtualPad {
    /// An Xbox 360 pad, the identity every pad had before games could ask for another.
    pub fn new(ff: Option<FfCaps>) -> Result<Self> {
        Self::with_identity(ff, PadIdentity::Xbox360)
    }

    /// A pad that presents itself as `identity`: its USB IDs, name and button labels.
    pub fn with_identity(ff: Option<FfCaps>, identity: PadIdentity) -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        for k in [
            KeyCode::BTN_SOUTH,
            KeyCode::BTN_EAST,
            KeyCode::BTN_NORTH,
            KeyCode::BTN_WEST,
            KeyCode::BTN_TL,
            KeyCode::BTN_TR,
            KeyCode::BTN_SELECT,
            KeyCode::BTN_START,
            KeyCode::BTN_MODE,
            KeyCode::BTN_THUMBL,
            KeyCode::BTN_THUMBR,
        ] {
            keys.insert(k);
        }
        let stick = AbsInfo::new(0, -32768, 32767, 16, 128, 0);
        let trigger = AbsInfo::new(0, 0, 255, 0, 0, 0);
        let hat = AbsInfo::new(0, -1, 1, 0, 0, 0);

        // An Xbox 360 pad by default, so SDL/Steam/games pick the right mapping out of the box.
        let name = format!("{VIRTUAL_PREFIX} {}", identity.name());
        let (vendor, product, version) = identity.usb_id();
        let mut builder = VirtualDevice::builder()?
            .name(&name)
            .input_id(InputId::new(BusType::BUS_USB, vendor, product, version))
            .with_keys(&keys)?;
        for (code, info) in [
            (Abs::ABS_X, stick),
            (Abs::ABS_Y, stick),
            (Abs::ABS_RX, stick),
            (Abs::ABS_RY, stick),
            (Abs::ABS_Z, trigger),
            (Abs::ABS_RZ, trigger),
            (Abs::ABS_HAT0X, hat),
            (Abs::ABS_HAT0Y, hat),
        ] {
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(code, info))?;
        }
        if let Some(ff) = ff {
            builder = builder.with_ff(ff.effects)?.with_ff_effects_max(ff.max_effects);
        }
        Ok(VirtualPad { dev: Arc::new(Mutex::new(builder.build()?)), dpad: HashSet::new(), identity })
    }

    /// The controller this pad presents itself as.
    pub fn identity(&self) -> PadIdentity {
        self.identity
    }

    pub fn shared(&self) -> Arc<Mutex<VirtualDevice>> {
        self.dev.clone()
    }

    fn emit(&self, events: &[InputEvent]) -> Result<()> {
        let mut dev = self.dev.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        dev.emit(events)?;
        Ok(())
    }

    pub fn button(&mut self, b: Button, pressed: bool) -> Result<()> {
        let code = match b {
            Button::South => KeyCode::BTN_SOUTH,
            Button::East => KeyCode::BTN_EAST,
            // Labels depend on the identity (see `north_code`).
            Button::North => self.north_code(),
            Button::West => self.west_code(),
            Button::LeftBumper => KeyCode::BTN_TL,
            Button::RightBumper => KeyCode::BTN_TR,
            Button::Select => KeyCode::BTN_SELECT,
            Button::Start => KeyCode::BTN_START,
            Button::Guide => KeyCode::BTN_MODE,
            Button::LeftStick => KeyCode::BTN_THUMBL,
            Button::RightStick => KeyCode::BTN_THUMBR,
            Button::DpadUp | Button::DpadDown | Button::DpadLeft | Button::DpadRight => {
                return self.dpad(b, pressed);
            }
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
        self.emit(&[key_event(code, pressed)])
    }

    /// The xpad driver's labels are swapped (its BTN_X is the west button), and the other pads
    /// use the gamepad spec's positions.
    fn north_code(&self) -> KeyCode {
        if self.identity.xpad_labels() { KeyCode::BTN_WEST } else { KeyCode::BTN_NORTH }
    }

    fn west_code(&self) -> KeyCode {
        if self.identity.xpad_labels() { KeyCode::BTN_NORTH } else { KeyCode::BTN_WEST }
    }

    fn dpad(&mut self, b: Button, pressed: bool) -> Result<()> {
        if pressed {
            self.dpad.insert(b);
        } else {
            self.dpad.remove(&b);
        }
        let axis = |neg, pos| self.dpad.contains(&pos) as i32 - self.dpad.contains(&neg) as i32;
        let x = axis(Button::DpadLeft, Button::DpadRight);
        let y = axis(Button::DpadUp, Button::DpadDown);
        self.emit(&[abs_event(Abs::ABS_HAT0X, x), abs_event(Abs::ABS_HAT0Y, y)])
    }

    pub fn axis(&mut self, axis: Axis, value: f32) -> Result<()> {
        let (code, raw) = match axis {
            Axis::LeftX => (Abs::ABS_X, stick_raw(value)),
            Axis::LeftY => (Abs::ABS_Y, stick_raw(value)),
            Axis::RightX => (Abs::ABS_RX, stick_raw(value)),
            Axis::RightY => (Abs::ABS_RY, stick_raw(value)),
            Axis::LeftTrigger => (Abs::ABS_Z, (value.clamp(0.0, 1.0) * 255.0).round() as i32),
            Axis::RightTrigger => (Abs::ABS_RZ, (value.clamp(0.0, 1.0) * 255.0).round() as i32),
        };
        self.emit(&[abs_event(code, raw)])
    }
}

/// Shared virtual keyboard + mouse. Presses are reference counted (see `PressState`), and a
/// press or release the kernel refused is sent again by `sync`.
pub struct VirtualKbm {
    keyboard: VirtualDevice,
    mouse: VirtualDevice,
    press: PressState,
    /// Hi-res wheel units not yet emitted as whole REL_WHEEL notches.
    wheel_acc: (i32, i32),
}

impl VirtualKbm {
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        // Every regular keyboard key (KEY_ESC..KEY_MICMUTE).
        for code in 1..=248u16 {
            keys.insert(KeyCode::new(code));
        }
        let keyboard = VirtualDevice::builder()?
            .name(&format!("{VIRTUAL_PREFIX} Keyboard"))
            .input_id(InputId::new(BusType::BUS_VIRTUAL, 0x1209, 0x0001, 1))
            .with_keys(&keys)?
            .build()?;

        let mut buttons = AttributeSet::<KeyCode>::new();
        for b in MouseButton::ALL {
            buttons.insert(mouse_code(b));
        }
        let mut rel = AttributeSet::<Rel>::new();
        for r in [Rel::REL_X, Rel::REL_Y, Rel::REL_WHEEL, Rel::REL_HWHEEL, Rel::REL_WHEEL_HI_RES, Rel::REL_HWHEEL_HI_RES] {
            rel.insert(r);
        }
        let mouse = VirtualDevice::builder()?
            .name(&format!("{VIRTUAL_PREFIX} Mouse"))
            .input_id(InputId::new(BusType::BUS_VIRTUAL, 0x1209, 0x0002, 1))
            .with_keys(&buttons)?
            .with_relative_axes(&rel)?
            .build()?;

        Ok(VirtualKbm { keyboard, mouse, press: PressState::default(), wheel_acc: (0, 0) })
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

    /// Brings the kernel in line with what is held. Called after every key or button, and by the
    /// daemon's periodic scan, so a release that failed is sent again even if nothing else is
    /// pressed.
    pub fn sync(&mut self) -> Result<()> {
        let Self { press, keyboard, mouse, .. } = self;
        press.sync(|code, down| {
            let dev = if MouseButton::ALL.iter().any(|b| mouse_code(*b) == code) { &mut *mouse } else { &mut *keyboard };
            Ok(dev.emit(&[key_event(code, down)])?)
        })
    }

    /// The event nodes of the keyboard and the mouse, for tests that read what the kernel got.
    #[cfg(test)]
    pub(crate) fn event_nodes(&mut self) -> (std::path::PathBuf, std::path::PathBuf) {
        let node = |dev: &mut VirtualDevice| dev.enumerate_dev_nodes_blocking().unwrap().flatten().next().unwrap();
        (node(&mut self.keyboard), node(&mut self.mouse))
    }

    pub fn mouse_move(&mut self, dx: i32, dy: i32) -> Result<()> {
        let mut events = Vec::with_capacity(2);
        if dx != 0 {
            events.push(rel_event(Rel::REL_X, dx));
        }
        if dy != 0 {
            events.push(rel_event(Rel::REL_Y, dy));
        }
        if !events.is_empty() {
            self.mouse.emit(&events)?;
        }
        Ok(())
    }

    /// Takes hi-res units (120 per notch) and also emits legacy notches for older clients.
    pub fn wheel(&mut self, vertical: i32, horizontal: i32) -> Result<()> {
        self.wheel_acc.0 += vertical;
        self.wheel_acc.1 += horizontal;
        let notches = (self.wheel_acc.0 / 120, self.wheel_acc.1 / 120);
        self.wheel_acc.0 -= notches.0 * 120;
        self.wheel_acc.1 -= notches.1 * 120;

        let mut events = Vec::with_capacity(4);
        if vertical != 0 {
            events.push(rel_event(Rel::REL_WHEEL_HI_RES, vertical));
        }
        if horizontal != 0 {
            events.push(rel_event(Rel::REL_HWHEEL_HI_RES, horizontal));
        }
        if notches.0 != 0 {
            events.push(rel_event(Rel::REL_WHEEL, notches.0));
        }
        if notches.1 != 0 {
            events.push(rel_event(Rel::REL_HWHEEL, notches.1));
        }
        if !events.is_empty() {
            self.mouse.emit(&events)?;
        }
        Ok(())
    }
}

fn key_event(code: KeyCode, pressed: bool) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.0, pressed as i32)
}

fn abs_event(code: Abs, value: i32) -> InputEvent {
    InputEvent::new(EventType::ABSOLUTE.0, code.0, value)
}

fn rel_event(code: Rel, value: i32) -> InputEvent {
    InputEvent::new(EventType::RELATIVE.0, code.0, value)
}
