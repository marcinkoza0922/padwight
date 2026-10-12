//! Linux's half of input: raw evdev events from a physical gamepad, its motion sensors and its
//! touchpad, turned into normalized [`InputEvent`]s and [`MotionSample`]s.

use std::{collections::HashMap, path::Path};

use evdev::{AbsoluteAxisCode as Abs, Device, EventSummary, KeyCode, MiscCode, PropType};

use super::{Axis, InputEvent, MotionFrame, MotionSample};
use crate::config::Button;

mod touchpad;

pub use touchpad::{TouchNormalizer, is_touchpad};

#[derive(Clone, Copy)]
struct AxisRange {
    axis: Axis,
    min: f32,
    max: f32,
}

impl AxisRange {
    fn normalize(&self, raw: i32) -> f32 {
        let span = (self.max - self.min).max(1.0);
        let unit = (raw as f32 - self.min) / span;
        match self.axis {
            Axis::LeftTrigger | Axis::RightTrigger => unit.clamp(0.0, 1.0),
            _ => (unit * 2.0 - 1.0).clamp(-1.0, 1.0),
        }
    }
}

pub struct Normalizer {
    axes: HashMap<Abs, AxisRange>,
    analog_triggers: bool,
    hat: (i32, i32),
    /// See [`uses_xbox_labels`].
    xbox_labels: bool,
}

/// The kernel defines BTN_X == BTN_NORTH and BTN_Y == BTN_WEST, so drivers that emit
/// buttons by their Xbox *label* report X (physically west) as "north" and Y as "west".
/// Positional drivers (hid-playstation, hid-nintendo, ...) follow the gamepad spec instead.
pub fn uses_xbox_labels(dev_path: &Path) -> bool {
    let Some(node) = dev_path.file_name() else { return false };
    let driver = Path::new("/sys/class/input").join(node).join("device/device/driver");
    std::fs::canonicalize(driver)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .is_some_and(|name| name == "xpad")
}

/// Returns true if the device looks like a gamepad per the kernel gamepad spec
/// (BTN_SOUTH is the same code as BTN_GAMEPAD).
pub fn is_gamepad(dev: &Device) -> bool {
    let has_buttons = dev
        .supported_keys()
        .is_some_and(|k| k.contains(KeyCode::BTN_SOUTH));
    let has_stick = dev
        .supported_absolute_axes()
        .is_some_and(|a| a.contains(Abs::ABS_X) && a.contains(Abs::ABS_Y));
    has_buttons && has_stick
}

/// Back paddles (Xbox Elite, DualSense Edge, Steam Deck). The kernel's header defines these
/// codes, but the evdev crate doesn't, so they're named here.
const BTN_GRIPL: KeyCode = KeyCode(0x224);
const BTN_GRIPR: KeyCode = KeyCode(0x225);
const BTN_GRIPL2: KeyCode = KeyCode(0x226);
const BTN_GRIPR2: KeyCode = KeyCode(0x227);

/// True if the device reports any back paddle button.
pub fn has_paddles(dev: &Device) -> bool {
    dev.supported_keys().is_some_and(|k| {
        [BTN_GRIPL, BTN_GRIPR, BTN_GRIPL2, BTN_GRIPR2].into_iter().any(|code| k.contains(code))
    })
}

/// False for pads whose triggers are only on/off buttons (Switch Pro Controller, Joy-Cons,
/// many retro-style pads): they report BTN_TL2/BTN_TR2 but no trigger axis.
pub fn has_analog_triggers(dev: &Device) -> bool {
    Normalizer::new(dev, false).analog_triggers
}

impl Normalizer {
    pub fn new(dev: &Device, xbox_labels: bool) -> Self {
        let info: HashMap<Abs, (i32, i32)> = dev
            .get_absinfo()
            .map(|it| it.map(|(code, i)| (code, (i.minimum(), i.maximum()))).collect())
            .unwrap_or_default();

        // Most pads report the right stick on RX/RY and triggers on Z/RZ. Some generic HID
        // pads have no RX/RY and use Z/RZ for the right stick, with triggers on BRAKE/GAS.
        let right_on_z = !info.contains_key(&Abs::ABS_RX) && info.contains_key(&Abs::ABS_RZ);
        let mut layout = vec![
            (Abs::ABS_X, Axis::LeftX),
            (Abs::ABS_Y, Axis::LeftY),
            (Abs::ABS_BRAKE, Axis::LeftTrigger),
            (Abs::ABS_GAS, Axis::RightTrigger),
        ];
        if right_on_z {
            layout.extend([(Abs::ABS_Z, Axis::RightX), (Abs::ABS_RZ, Axis::RightY)]);
        } else {
            layout.extend([
                (Abs::ABS_RX, Axis::RightX),
                (Abs::ABS_RY, Axis::RightY),
                (Abs::ABS_Z, Axis::LeftTrigger),
                (Abs::ABS_RZ, Axis::RightTrigger),
            ]);
        }

        let mut axes = HashMap::new();
        for (code, axis) in layout {
            if let Some(&(min, max)) = info.get(&code) {
                axes.insert(code, AxisRange { axis, min: min as f32, max: max as f32 });
            }
        }
        let analog_triggers = axes
            .values()
            .any(|r| matches!(r.axis, Axis::LeftTrigger | Axis::RightTrigger));

        Normalizer { axes, analog_triggers, hat: (0, 0), xbox_labels }
    }

    pub fn translate(&mut self, ev: evdev::InputEvent, out: &mut Vec<InputEvent>) {
        match ev.destructure() {
            EventSummary::Key(_, code, value) => {
                // value 2 is autorepeat; ignore it.
                if value > 1 {
                    return;
                }
                let pressed = value == 1;
                if let Some(b) = key_to_button(code, self.xbox_labels) {
                    out.push(InputEvent::Button(b, pressed));
                } else if !self.analog_triggers {
                    let level = if pressed { 1.0 } else { 0.0 };
                    match code {
                        KeyCode::BTN_TL2 => out.push(InputEvent::Axis(Axis::LeftTrigger, level)),
                        KeyCode::BTN_TR2 => out.push(InputEvent::Axis(Axis::RightTrigger, level)),
                        _ => {}
                    }
                }
            }
            EventSummary::AbsoluteAxis(_, Abs::ABS_HAT0X, v) => {
                let old = self.hat.0;
                self.hat.0 = v.signum();
                hat_change(old, self.hat.0, Button::DpadLeft, Button::DpadRight, out);
            }
            EventSummary::AbsoluteAxis(_, Abs::ABS_HAT0Y, v) => {
                let old = self.hat.1;
                self.hat.1 = v.signum();
                hat_change(old, self.hat.1, Button::DpadUp, Button::DpadDown, out);
            }
            EventSummary::AbsoluteAxis(_, code, v) => {
                if let Some(range) = self.axes.get(&code) {
                    out.push(InputEvent::Axis(range.axis, range.normalize(v)));
                }
            }
            _ => {}
        }
    }
}

/// True for a controller's motion-sensor device (hid-playstation's "... Motion Sensors",
/// hid-nintendo's "... IMU"): accelerometer on ABS_X/Y/Z and gyro on ABS_RX/RY/RZ.
pub fn is_motion_sensor(dev: &Device) -> bool {
    dev.properties().contains(PropType::ACCELEROMETER)
        && dev.supported_absolute_axes().is_some_and(|a| a.contains(Abs::ABS_RX) && a.contains(Abs::ABS_RY))
}

/// Name of a game controller motion sensor at `dev_path` that we are not allowed to open,
/// read from sysfs (which needs no access to the device itself).
pub fn inaccessible_motion_sensor(dev_path: &Path) -> Option<String> {
    let node = dev_path.file_name()?;
    let input = Path::new("/sys/class/input").join(node).join("device");
    // INPUT_PROP_ACCELEROMETER is property bit 6.
    let props = u64::from_str_radix(std::fs::read_to_string(input.join("properties")).ok()?.trim(), 16).ok()?;
    let driver = std::fs::canonicalize(input.join("device/driver")).ok()?;
    let controller = matches!(driver.file_name()?.to_str()?, "playstation" | "sony" | "nintendo");
    if props & (1 << 6) == 0 || !controller || Device::open(dev_path).is_ok() {
        return None;
    }
    Some(std::fs::read_to_string(input.join("name")).ok()?.trim().to_string())
}

impl MotionFrame {
    /// Frame of the motion-sensor device at `dev_path`, from its driver.
    pub fn of(dev_path: &Path) -> Self {
        let driver = dev_path
            .file_name()
            .and_then(|node| std::fs::canonicalize(Path::new("/sys/class/input").join(node).join("device/device/driver")).ok());
        match driver.as_deref().and_then(|d| d.file_name()).and_then(|n| n.to_str()) {
            Some("nintendo") => MotionFrame::Nintendo,
            _ => MotionFrame::PlayStation,
        }
    }
}

/// Turns raw motion-sensor events into [`MotionSample`]s, one per SYN_REPORT.
pub struct MotionNormalizer {
    /// Units per g (accel) and per degree/second (gyro), from the axes' resolution.
    scale: HashMap<Abs, f32>,
    current: MotionSample,
    last_timestamp: Option<u32>,
    timestamp: Option<u32>,
    last_report: Option<std::time::Instant>,
}

/// Longest gap treated as continuous motion (anything longer is a pause, not rotation).
const MAX_MOTION_DT: f32 = 0.05;

impl MotionNormalizer {
    pub fn new(dev: &Device) -> Self {
        let scale = dev
            .get_absinfo()
            .map(|it| {
                it.filter(|(code, _)| {
                    matches!(*code, Abs::ABS_X | Abs::ABS_Y | Abs::ABS_Z | Abs::ABS_RX | Abs::ABS_RY | Abs::ABS_RZ)
                })
                .map(|(code, info)| (code, info.resolution().max(1) as f32))
                .collect()
            })
            .unwrap_or_default();
        Self::with_scale(scale)
    }

    /// `scale` is the resolution per axis: units per g (accel) or per degree/second (gyro).
    fn with_scale(scale: HashMap<Abs, f32>) -> Self {
        MotionNormalizer {
            scale,
            current: MotionSample::default(),
            last_timestamp: None,
            timestamp: None,
            last_report: None,
        }
    }

    /// Feeds one raw event; returns a sample when a report completes.
    pub fn translate(&mut self, ev: evdev::InputEvent) -> Option<MotionSample> {
        match ev.destructure() {
            EventSummary::AbsoluteAxis(_, code, value) => {
                let v = value as f32 / self.scale.get(&code).copied().unwrap_or(1.0);
                match code {
                    Abs::ABS_X => self.current.accel[0] = v,
                    Abs::ABS_Y => self.current.accel[1] = v,
                    Abs::ABS_Z => self.current.accel[2] = v,
                    Abs::ABS_RX => self.current.gyro[0] = v,
                    Abs::ABS_RY => self.current.gyro[1] = v,
                    Abs::ABS_RZ => self.current.gyro[2] = v,
                    _ => {}
                }
                None
            }
            EventSummary::Misc(_, MiscCode::MSC_TIMESTAMP, value) => {
                self.timestamp = Some(value as u32);
                None
            }
            EventSummary::Synchronization(..) => Some(self.finish_report()),
            _ => None,
        }
    }

    fn finish_report(&mut self) -> MotionSample {
        let now = std::time::Instant::now();
        // Prefer the sensor's own microsecond clock (it wraps around at u32::MAX).
        let dt = match (self.last_timestamp, self.timestamp) {
            (Some(prev), Some(ts)) => ts.wrapping_sub(prev) as f32 / 1_000_000.0,
            _ => self.last_report.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.0),
        };
        self.last_timestamp = self.timestamp.or(self.last_timestamp);
        self.last_report = Some(now);
        MotionSample { dt: if dt > MAX_MOTION_DT { 0.0 } else { dt }, ..self.current }
    }
}

fn hat_change(old: i32, new: i32, neg: Button, pos: Button, out: &mut Vec<InputEvent>) {
    if old == new {
        return;
    }
    match old {
        -1 => out.push(InputEvent::Button(neg, false)),
        1 => out.push(InputEvent::Button(pos, false)),
        _ => {}
    }
    match new {
        -1 => out.push(InputEvent::Button(neg, true)),
        1 => out.push(InputEvent::Button(pos, true)),
        _ => {}
    }
}

fn key_to_button(code: KeyCode, xbox_labels: bool) -> Option<Button> {
    Some(match code {
        KeyCode::BTN_SOUTH => Button::South,
        KeyCode::BTN_EAST => Button::East,
        KeyCode::BTN_NORTH if xbox_labels => Button::West,
        KeyCode::BTN_WEST if xbox_labels => Button::North,
        KeyCode::BTN_NORTH => Button::North,
        KeyCode::BTN_WEST => Button::West,
        KeyCode::BTN_TL => Button::LeftBumper,
        KeyCode::BTN_TR => Button::RightBumper,
        KeyCode::BTN_SELECT => Button::Select,
        KeyCode::BTN_START => Button::Start,
        KeyCode::BTN_MODE => Button::Guide,
        KeyCode::BTN_THUMBL => Button::LeftStick,
        KeyCode::BTN_THUMBR => Button::RightStick,
        KeyCode::BTN_DPAD_UP => Button::DpadUp,
        KeyCode::BTN_DPAD_DOWN => Button::DpadDown,
        KeyCode::BTN_DPAD_LEFT => Button::DpadLeft,
        KeyCode::BTN_DPAD_RIGHT => Button::DpadRight,
        BTN_GRIPL => Button::LeftPaddle,
        BTN_GRIPR => Button::RightPaddle,
        BTN_GRIPL2 => Button::LeftPaddle2,
        BTN_GRIPR2 => Button::RightPaddle2,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_stick_and_trigger_ranges() {
        let stick = AxisRange { axis: Axis::LeftX, min: -32768.0, max: 32767.0 };
        assert!((stick.normalize(-32768) + 1.0).abs() < 1e-4);
        assert!(stick.normalize(0).abs() < 1e-3);
        assert!((stick.normalize(32767) - 1.0).abs() < 1e-4);

        let trig = AxisRange { axis: Axis::RightTrigger, min: 0.0, max: 1023.0 };
        assert_eq!(trig.normalize(0), 0.0);
        assert_eq!(trig.normalize(1023), 1.0);
    }

    #[test]
    fn xbox_label_drivers_swap_north_and_west() {
        // xpad sends BTN_X (== BTN_NORTH) for the X button, which sits on the west.
        assert_eq!(key_to_button(KeyCode::BTN_NORTH, true), Some(Button::West));
        assert_eq!(key_to_button(KeyCode::BTN_WEST, true), Some(Button::North));
        assert_eq!(key_to_button(KeyCode::BTN_NORTH, false), Some(Button::North));
        assert_eq!(key_to_button(KeyCode::BTN_SOUTH, true), Some(Button::South));
    }

    fn ev(type_: evdev::EventType, code: u16, value: i32) -> evdev::InputEvent {
        evdev::InputEvent::new(type_.0, code, value)
    }

    #[test]
    fn motion_reports_scale_to_degrees_and_g_with_sensor_timestamps() {
        use evdev::EventType;
        // DualSense resolutions: 1024 units per degree/second, 8192 per g.
        let scale = HashMap::from([
            (Abs::ABS_X, 8192.0),
            (Abs::ABS_Y, 8192.0),
            (Abs::ABS_Z, 8192.0),
            (Abs::ABS_RX, 1024.0),
            (Abs::ABS_RY, 1024.0),
            (Abs::ABS_RZ, 1024.0),
        ]);
        let mut norm = MotionNormalizer::with_scale(scale);
        let report = |norm: &mut MotionNormalizer, yaw: i32, timestamp: u32| {
            let events = [
                ev(EventType::ABSOLUTE, Abs::ABS_Y.0, 8192),
                ev(EventType::ABSOLUTE, Abs::ABS_RY.0, yaw),
                ev(EventType::MISC, MiscCode::MSC_TIMESTAMP.0, timestamp as i32),
                ev(EventType::SYNCHRONIZATION, 0, 0),
            ];
            events.into_iter().filter_map(|e| norm.translate(e)).last().unwrap()
        };
        let first = report(&mut norm, 90 * 1024, 1_000);
        assert_eq!(first.gyro, [0.0, 90.0, 0.0]);
        assert_eq!(first.accel, [0.0, 1.0, 0.0]);
        let second = report(&mut norm, -45 * 1024, 5_000);
        assert_eq!(second.gyro[1], -45.0);
        assert!((second.dt - 0.004).abs() < 1e-6, "{}", second.dt);
        // The microsecond clock wraps around.
        let mut wrap = MotionNormalizer::with_scale(HashMap::new());
        report(&mut wrap, 0, u32::MAX - 999);
        assert!((report(&mut wrap, 0, 3_000).dt - 0.004).abs() < 1e-6);
        // A long gap (e.g. a stall) is not treated as one huge rotation step.
        assert_eq!(report(&mut wrap, 0, 3_000 + 500_000).dt, 0.0);
    }

    #[test]
    fn paddle_codes_map_to_the_paddle_buttons() {
        assert_eq!(key_to_button(BTN_GRIPL, false), Some(Button::LeftPaddle));
        assert_eq!(key_to_button(BTN_GRIPR, false), Some(Button::RightPaddle));
        assert_eq!(key_to_button(BTN_GRIPL2, true), Some(Button::LeftPaddle2));
        assert_eq!(key_to_button(BTN_GRIPR2, true), Some(Button::RightPaddle2));
    }

    #[test]
    fn hat_emits_release_then_press() {
        let mut out = Vec::new();
        hat_change(-1, 1, Button::DpadLeft, Button::DpadRight, &mut out);
        assert_eq!(
            out,
            vec![
                InputEvent::Button(Button::DpadLeft, false),
                InputEvent::Button(Button::DpadRight, true)
            ]
        );
    }
}
