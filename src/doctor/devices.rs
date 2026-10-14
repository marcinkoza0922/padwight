//! What /sys/class/input says about the controllers, and whether this user can open their nodes.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// The kernel's BTN_SOUTH (the same code as BTN_GAMEPAD).
const BTN_SOUTH: usize = 0x130;
/// INPUT_PROP_ACCELEROMETER: how the kernel marks a motion sensor.
const PROP_ACCELEROMETER: usize = 6;
/// The drivers (as udev's `DRIVERS` match sees them) whose motion sensors the shipped rule covers.
const MOTION_DRIVERS: [&str; 3] = ["playstation", "sony", "nintendo"];

/// An input node that is a gamepad or a motion sensor.
pub struct Node {
    /// `event5`.
    pub event: String,
    pub name: String,
    pub gamepad: bool,
    pub motion: bool,
    /// Drivers of the device and its parents, such as `playstation` or `usbhid`.
    pub drivers: Vec<String>,
    /// Whether this user can open the node for reading, and if not, why.
    pub access: Result<(), ErrorKind>,
}

impl Node {
    /// A motion sensor of a PlayStation or Nintendo controller, which the motion-sensor rule covers.
    pub fn is_pad_motion(&self) -> bool {
        self.motion && self.drivers.iter().any(|d| MOTION_DRIVERS.contains(&d.as_str()))
    }

    pub fn denied(&self) -> bool {
        self.access == Err(ErrorKind::PermissionDenied)
    }
}

/// The gamepads and motion sensors under `sysfs` (/sys/class/input), opened through `dev` (/dev/input).
pub fn scan(sysfs: &Path, dev: &Path) -> Vec<Node> {
    let Ok(entries) = fs::read_dir(sysfs) else { return Vec::new() };
    let mut events: Vec<String> = entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.strip_prefix("event").is_some_and(|rest| rest.parse::<u32>().is_ok()))
        .collect();
    events.sort_by_key(|n| n[5..].parse::<u32>().unwrap_or(0));
    events.into_iter().filter_map(|event| node(sysfs, dev, event)).collect()
}

fn node(sysfs: &Path, dev: &Path, event: String) -> Option<Node> {
    let dir = sysfs.join(&event);
    let device = dir.join("device");
    let key = read(&device.join("capabilities/key"));
    let abs = read(&device.join("capabilities/abs"));
    let gamepad = bit_set(&key, BTN_SOUTH) && bit_set(&abs, 0) && bit_set(&abs, 1);
    let motion = bit_set(&read(&device.join("properties")), PROP_ACCELEROMETER);
    if !gamepad && !motion {
        return None;
    }
    let name = read(&device.join("name"));
    if name.starts_with(crate::output::VIRTUAL_PREFIX) {
        return None;
    }
    let access = fs::File::open(dev.join(&event)).map(drop).map_err(|e| e.kind());
    Some(Node { name, gamepad, motion, drivers: drivers(&dir), access, event })
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).map(|s| s.trim().to_owned()).unwrap_or_default()
}

/// Whether bit `n` is set in a sysfs bitmap: hex words, most significant first, each as wide as a
/// C long.
fn bit_set(map: &str, n: usize) -> bool {
    let width = usize::BITS as usize;
    let words: Vec<&str> = map.split_whitespace().collect();
    let Some(word) = words.len().checked_sub(1 + n / width).and_then(|i| words.get(i)) else {
        return false;
    };
    u64::from_str_radix(word, 16).is_ok_and(|w| (w >> (n % width)) & 1 == 1)
}

/// The names of the drivers bound to the node's device and each of its parents.
fn drivers(node_dir: &Path) -> Vec<String> {
    let Ok(real) = fs::canonicalize(node_dir) else { return Vec::new() };
    real.ancestors()
        .filter_map(|dir| fs::read_link(dir.join("driver")).ok())
        .filter_map(|link| link.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

/// The directories udev reads rules from, in the order it prefers them.
pub fn rule_dirs() -> Vec<PathBuf> {
    ["/etc/udev/rules.d", "/run/udev/rules.d", "/usr/local/lib/udev/rules.d", "/usr/lib/udev/rules.d", "/lib/udev/rules.d"]
        .into_iter()
        .map(PathBuf::from)
        .collect()
}
