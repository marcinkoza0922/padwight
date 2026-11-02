//! ENV-1 without the drivers: how a controller on each Linux driver looks to Padwight.
//!
//! What a real `xpad`, `xone`, `xpadneo`, `hid-nintendo` (alone or combined by `joycond`),
//! `hid-steam` or `hid-playstation` pad puts in sysfs and on its evdev node is written out here
//! from the capability sets those drivers declare, then run through the code that decides the
//! face-button label order, the motion-sensor frame, the trigger kind, and what `doctor` reports.
//! Nothing here proves the real driver behaves so (the driver names and capability lists are the
//! drivers' documented ones, written from memory of their sources); it pins what Padwight does
//! with each, so a change shows up and a person only has to confirm the shapes on real hardware.
//!
//! The `kernel` tests at the bottom make the same pads as uinput devices and need /dev/uinput:
//! `scripts/kernel-test-docker.sh variants_kernel`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::*;
use crate::doctor::devices::scan;

/// What one driver's pad declares.
struct Variant {
    /// A label for failures.
    name: &'static str,
    /// The input device's name, as the driver sets it.
    device_name: &'static str,
    /// The driver bound to the HID or USB device behind the node, as udev's `DRIVER` sees it.
    driver: &'static str,
    keys: &'static [u16],
    abs: &'static [u16],
    /// INPUT_PROP_* bits.
    props: &'static [u16],
}

const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_TL2: u16 = 0x138;
const BTN_TR2: u16 = 0x139;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;
const BTN_DPAD_UP: u16 = 0x220;
const ABS_X: u16 = 0;
const ABS_Y: u16 = 1;
const ABS_Z: u16 = 2;
const ABS_RX: u16 = 3;
const ABS_RY: u16 = 4;
const ABS_RZ: u16 = 5;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const PROP_ACCELEROMETER: u16 = 6;

const XBOX_KEYS: &[u16] = &[BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR, BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR];
const STICKS_TRIGGERS_HAT: &[u16] = &[ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ, ABS_HAT0X, ABS_HAT0Y];

/// The pads. `driver` is the kernel (or out-of-tree) driver's registered name.
const VARIANTS: &[Variant] = &[
    Variant { name: "xpad (wired Xbox)", device_name: "Microsoft X-Box 360 pad", driver: "xpad", keys: XBOX_KEYS, abs: STICKS_TRIGGERS_HAT, props: &[] },
    Variant { name: "xone (USB or dongle)", device_name: "Xbox One Wireless Controller", driver: "xone-gip-gamepad", keys: XBOX_KEYS, abs: STICKS_TRIGGERS_HAT, props: &[] },
    Variant { name: "xpadneo (Bluetooth)", device_name: "Xbox Wireless Controller", driver: "xpadneo", keys: XBOX_KEYS, abs: STICKS_TRIGGERS_HAT, props: &[] },
    // hid-nintendo: positional buttons, triggers are buttons, a dpad of buttons, no hat.
    Variant {
        name: "hid-nintendo Pro Controller",
        device_name: "Nintendo Switch Pro Controller",
        driver: "nintendo",
        keys: &[BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR, BTN_TL2, BTN_TR2, BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR, BTN_DPAD_UP],
        abs: &[ABS_X, ABS_Y, ABS_RX, ABS_RY],
        props: &[],
    },
    // hid-steam with lizard mode off: the Deck's pad node, with paddles and a hat.
    Variant {
        name: "hid-steam Deck, lizard mode off",
        device_name: "Steam Deck",
        driver: "hid-steam",
        keys: &[BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR, BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR, 0x224],
        abs: &[ABS_X, ABS_Y, ABS_RX, ABS_RY, ABS_HAT0X, ABS_HAT0Y],
        props: &[],
    },
    // hid-playstation: positional buttons, analog triggers on Z and RZ, a hat.
    Variant {
        name: "hid-playstation DualSense",
        device_name: "Sony Interactive Entertainment DualSense Wireless Controller",
        driver: "playstation",
        keys: &[BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR, BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR],
        abs: STICKS_TRIGGERS_HAT,
        props: &[],
    },
    // A D-input pad with no driver of its own: the right stick on Z and RZ.
    Variant {
        name: "8BitDo in D-input mode (hid-generic)",
        device_name: "8BitDo Pro 2",
        driver: "hid-generic",
        keys: &[BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR, BTN_TL2, BTN_TR2, BTN_SELECT, BTN_START, BTN_MODE],
        abs: &[ABS_X, ABS_Y, ABS_Z, ABS_RZ, ABS_HAT0X, ABS_HAT0Y],
        props: &[],
    },
];

/// Whether the variant has triggers that report as analog axes (the Normalizer's rule: an axis
/// for them exists, and the right stick isn't using Z/RZ).
fn analog_triggers(v: &Variant) -> bool {
    let right_on_z = !v.abs.contains(&ABS_RX) && v.abs.contains(&ABS_RZ);
    !right_on_z && (v.abs.contains(&ABS_Z) || v.abs.contains(&ABS_RZ))
}

/// Motion-sensor nodes next to a pad.
const MOTION: &[Variant] = &[
    Variant { name: "hid-playstation motion", device_name: "DualSense Motion Sensors", driver: "playstation", keys: &[], abs: &[ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ], props: &[PROP_ACCELEROMETER] },
    Variant { name: "hid-nintendo IMU", device_name: "Nintendo Switch Pro Controller IMU", driver: "nintendo", keys: &[], abs: &[ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ], props: &[PROP_ACCELEROMETER] },
];

/// A sysfs bitmap of `bits`: hex words as wide as a C long, the highest first.
fn bitmap(bits: &[u16]) -> String {
    let width = usize::BITS as usize;
    let words = bits.iter().map(|b| *b as usize / width + 1).max().unwrap_or(1);
    let mut map = vec![0u64; words];
    for b in bits {
        map[*b as usize / width] |= 1 << (*b as usize % width);
    }
    map.iter().rev().map(|w| format!("{w:x}")).collect::<Vec<_>>().join(" ")
}

/// A made-up /sys/class/input with the layout the kernel has: `eventN` is a link to the node
/// inside its input device, whose `device` link goes to the HID device that has the driver.
struct Sysfs(PathBuf);

impl Sysfs {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("padwight-variants-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("class")).unwrap();
        fs::create_dir_all(root.join("dev")).unwrap();
        Sysfs(root)
    }

    fn class(&self) -> PathBuf {
        self.0.join("class")
    }

    fn dev(&self) -> PathBuf {
        self.0.join("dev")
    }

    fn add(&self, event: &str, v: &Variant) {
        let hid = self.0.join("devices").join(format!("hid-{event}"));
        let input = hid.join("input");
        let node = input.join(event);
        fs::create_dir_all(input.join("capabilities")).unwrap();
        fs::create_dir_all(&node).unwrap();
        fs::create_dir_all(self.0.join("drivers").join(v.driver)).unwrap();
        fs::write(input.join("name"), format!("{}\n", v.device_name)).unwrap();
        fs::write(input.join("capabilities/key"), format!("{}\n", bitmap(v.keys))).unwrap();
        fs::write(input.join("capabilities/abs"), format!("{}\n", bitmap(v.abs))).unwrap();
        fs::write(input.join("properties"), format!("{}\n", bitmap(v.props))).unwrap();
        fs::write(self.dev().join(event), "").unwrap();
        // The node's `device` is its input device; the input device's `device` is the HID device.
        std::os::unix::fs::symlink("..", node.join("device")).unwrap();
        std::os::unix::fs::symlink("..", input.join("device")).unwrap();
        std::os::unix::fs::symlink(format!("../../drivers/{}", v.driver), hid.join("driver")).unwrap();
        std::os::unix::fs::symlink(&node, self.class().join(event)).unwrap();
    }

    fn path(&self, event: &str) -> PathBuf {
        Path::new("/dev/input").join(event)
    }
}

impl Drop for Sysfs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn only_the_xpad_driver_gets_the_xbox_label_swap() {
    // Characterisation. xpad sends BTN_X for the pad's X button, which sits on the west, and
    // Padwight undoes that for `xpad` only. `xone` and `xpadneo` are modelled on xpad and may
    // label their buttons the same way; whether they do is a question for a real pad (ENV-1),
    // and this is the test to change when the answer is yes.
    for v in VARIANTS {
        let sys = Sysfs::new("labels");
        sys.add("event7", v);
        let swapped = uses_xbox_labels_in(&sys.class(), &sys.path("event7"));
        assert_eq!(swapped, v.driver == "xpad", "{}", v.name);
    }
}

#[test]
fn a_node_with_no_sysfs_entry_or_no_driver_is_positional() {
    let sys = Sysfs::new("missing");
    assert!(!uses_xbox_labels_in(&sys.class(), &sys.path("event99")));
    assert!(!uses_xbox_labels_in(&sys.class(), Path::new("/")));
    assert_eq!(MotionFrame::of_in(&sys.class(), &sys.path("event99")), MotionFrame::PlayStation);
}

#[test]
fn motion_sensors_use_their_drivers_axis_frame() {
    for (v, expected) in MOTION.iter().zip([MotionFrame::PlayStation, MotionFrame::Nintendo]) {
        let sys = Sysfs::new("motion");
        sys.add("event8", v);
        assert_eq!(MotionFrame::of_in(&sys.class(), &sys.path("event8")), expected, "{}", v.name);
    }
}

#[test]
fn doctor_finds_every_pad_variant_and_names_its_driver() {
    for v in VARIANTS {
        let sys = Sysfs::new("doctor");
        sys.add("event7", v);
        let nodes = scan(&sys.class(), &sys.dev());
        assert_eq!(nodes.len(), 1, "{}: a pad is found", v.name);
        let n = &nodes[0];
        assert!(n.gamepad && !n.motion, "{}", v.name);
        assert!(n.drivers.iter().any(|d| d == v.driver), "{}: names {:?}", v.name, n.drivers);
        assert_eq!(n.name, v.device_name);
        // Only the drivers the shipped udev rule lists get the hidraw and motion-sensor rule.
        assert_eq!(n.is_hidraw_pad(), ["nintendo", "playstation"].contains(&v.driver), "{}", v.name);
    }
    for v in MOTION {
        let sys = Sysfs::new("doctor-motion");
        sys.add("event8", v);
        let nodes = scan(&sys.class(), &sys.dev());
        assert_eq!(nodes.len(), 1, "{}", v.name);
        assert!(nodes[0].motion && nodes[0].is_pad_motion(), "{}", v.name);
    }
}

#[test]
fn a_deck_in_lizard_mode_shows_no_gamepad_node() {
    // In lizard mode hid-steam presents a keyboard and mouse, not a pad: no BTN_SOUTH, no sticks.
    let lizard = Variant { name: "Deck lizard mode", device_name: "Steam Deck", driver: "hid-steam", keys: &[0x1e, 0x110], abs: &[], props: &[] };
    let sys = Sysfs::new("lizard");
    sys.add("event7", &lizard);
    assert!(scan(&sys.class(), &sys.dev()).is_empty());
}

mod kernel {
    //! The same pads as real uinput devices, read back through the gamepad checks and [`Normalizer`].

    use std::{thread, time::Duration};

    use evdev::{AbsInfo, AttributeSet, BusType, EventType, InputId, UinputAbsSetup, uinput::VirtualDevice};

    use super::*;

    fn build(v: &Variant) -> (VirtualDevice, Device) {
        let mut keys = AttributeSet::<KeyCode>::new();
        v.keys.iter().for_each(|k| keys.insert(KeyCode(*k)));
        let name = format!("padwight-variant-{}", v.driver);
        let mut builder = VirtualDevice::builder()
            .unwrap()
            .name(&name)
            .input_id(InputId::new(BusType::BUS_USB, 0x1209, 0x0001, 1))
            .with_keys(&keys)
            .unwrap();
        for code in v.abs {
            let info = match *code {
                ABS_Z | ABS_RZ if v.driver == "xpad" || v.driver == "playstation" => AbsInfo::new(0, 0, 255, 0, 0, 0),
                ABS_Z | ABS_RZ if v.driver == "xpadneo" || v.driver == "xone-gip-gamepad" => AbsInfo::new(0, 0, 1023, 0, 0, 0),
                ABS_HAT0X | ABS_HAT0Y => AbsInfo::new(0, -1, 1, 0, 0, 0),
                _ => AbsInfo::new(0, -32768, 32767, 16, 128, 0),
            };
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(Abs(*code), info)).unwrap();
        }
        let mut dev = builder.build().unwrap();
        let nodes: Vec<PathBuf> = dev.enumerate_dev_nodes_blocking().unwrap().flatten().collect();
        let node = nodes.first().expect("a node");
        thread::sleep(Duration::from_millis(100));
        let read = Device::open(node).unwrap();
        (dev, read)
    }

    #[test]
    #[ignore = "needs /dev/uinput"]
    fn variants_kernel_the_declared_pads_are_recognised_and_normalized() {
        for v in VARIANTS {
            let (_keep, dev) = build(v);
            assert!(is_gamepad(&dev), "{}: a gamepad", v.name);
            assert_eq!(has_paddles(&dev), v.keys.contains(&0x224), "{}: paddles", v.name);
            let analog = has_analog_triggers(&dev);
            assert_eq!(analog, analog_triggers(v), "{}: triggers", v.name);
            assert!(!is_motion_sensor(&dev), "{}", v.name);
            // Full stick travel reads as -1..1 on every variant, and a trigger as 0..1.
            let mut norm = Normalizer::new(&dev, false);
            let mut out = Vec::new();
            let abs = |code: u16, value: i32| evdev::InputEvent::new(EventType::ABSOLUTE.0, code, value);
            norm.translate(abs(ABS_X, 32767), &mut out);
            norm.translate(abs(ABS_X, -32768), &mut out);
            assert_eq!(out, [InputEvent::Axis(Axis::LeftX, 1.0), InputEvent::Axis(Axis::LeftX, -1.0)], "{}", v.name);
            out.clear();
            if analog {
                let max = if matches!(v.driver, "xpad" | "playstation") { 255 } else { 1023 };
                norm.translate(abs(ABS_Z, max), &mut out);
                assert_eq!(out, [InputEvent::Axis(Axis::LeftTrigger, 1.0)], "{}", v.name);
            } else if v.keys.contains(&BTN_TL2) {
                norm.translate(evdev::InputEvent::new(EventType::KEY.0, BTN_TL2, 1), &mut out);
                assert_eq!(out, [InputEvent::Axis(Axis::LeftTrigger, 1.0)], "{}: a trigger button reads as a full pull", v.name);
            }
        }
    }
}
