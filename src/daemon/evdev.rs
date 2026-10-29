//! Controllers on Linux: finds gamepads in /dev/input, grabs them so nothing else sees them, and
//! pairs them with their motion-sensor and touchpad nodes. The main loop reads them all
//! (`readers.rs`).

use std::{
    collections::{HashMap, HashSet},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use evdev::Device;

pub(super) use super::pads::{FfSpec, virtual_pad_for};
use super::{Daemon, Managed, SeenGamepad, readers::Reader};
use crate::controller_id::controller_id;
use crate::{
    input::{self, MotionFrame},
    monitor::log,
    output::VIRTUAL_PREFIX,
    pad_identity::PadIdentity,
    rumble,
};

/// A device node identity; the inode changes when a node is recreated for a new device.
pub(super) type NodeKey = (PathBuf, u64);

/// The input nodes a scan found that aren't gamepads of their own.
#[derive(Default)]
pub(super) struct Nodes {
    /// Nodes already checked that we never manage (not a gamepad, or a virtual device).
    skipped: HashSet<NodeKey>,
    /// Controller motion sensors, waiting to be paired with their gamepad.
    motion: HashMap<NodeKey, PairedNode>,
    /// Controller touchpads, waiting to be paired with their gamepad.
    pub(super) touch: HashMap<NodeKey, PairedNode>,
    /// Controller motion sensors we lack permission to open (names), for the GUI to explain.
    denied: HashMap<NodeKey, String>,
}

/// A plain Xbox 360 pad, for an injected controller whose output goes out for real.
pub(super) fn live_pad() -> anyhow::Result<crate::output::VirtualPad> {
    crate::output::VirtualPad::new(None)
}

impl Nodes {
    /// The motion sensors we may not open, by name.
    pub(super) fn denied_sensors(&self) -> Vec<String> {
        self.denied.values().cloned().collect()
    }
}

/// A controller's extra input device (motion sensors, touchpad), waiting to be paired with its
/// gamepad.
pub(super) struct PairedNode {
    pub(super) path: PathBuf,
    pub(super) name: String,
    parent: Option<PathBuf>,
    uniq: Option<String>,
}

impl PairedNode {
    /// Same physical controller: same parent HID device or same unique ID (e.g. Bluetooth
    /// MAC). Only when the parent can't be compared does the driver's naming ("<pad> Motion
    /// Sensors", "<pad> IMU") count, since two identical controllers share a name.
    pub(super) fn belongs_to(&self, pad: &Managed) -> bool {
        self.matches(&pad.name, &pad.parent, &pad.uniq)
    }

    fn matches(&self, name: &str, parent: &Option<PathBuf>, uniq: &Option<String>) -> bool {
        let same_parent = self.parent.is_some() && &self.parent == parent;
        let same_uniq = self.uniq.as_deref().is_some_and(|u| !u.is_empty()) && &self.uniq == uniq;
        let comparable = self.parent.is_some() && parent.is_some();
        let named_after = !comparable && self.name.len() > name.len() && self.name.starts_with(name);
        same_parent || same_uniq || named_after
    }
}

impl Daemon {
    /// Notes a controller's motion sensor we may not open, so the GUI can say how to fix it.
    fn note_denied_sensor(&mut self, key: NodeKey, path: &Path) {
        if !self.nodes.denied.contains_key(&key)
            && let Some(name) = input::inaccessible_motion_sensor(path)
        {
            log!("gyro: no permission to read {name}; install dist/70-padwight-motion.rules");
            self.nodes.denied.insert(key, name);
        }
    }

    /// Keeps a controller's motion-sensor or touchpad node to pair with its gamepad later.
    /// False for any other node.
    fn remember_sibling(&mut self, key: NodeKey, path: &Path, dev: &Device, name: &str) -> bool {
        if is_uinput(path) {
            return false;
        }
        let node = || PairedNode {
            path: path.to_path_buf(),
            name: name.to_string(),
            parent: hid_parent(path),
            uniq: dev.unique_name().map(str::to_string),
        };
        if input::is_touchpad(dev) {
            self.nodes.touch.insert(key, node());
        } else if input::is_motion_sensor(dev) {
            self.nodes.motion.insert(key, node());
        } else {
            return false;
        }
        true
    }

    /// Looks for new gamepads in /dev/input and grabs the ones we should manage, and pairs them
    /// with their motion sensors and touchpads.
    pub(super) fn scan_devices(&mut self) {
        let Ok(entries) = std::fs::read_dir("/dev/input") else { return };
        let mut present = HashSet::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.file_name().is_some_and(|n| n.to_string_lossy().starts_with("event")) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let key = (path.clone(), meta.ino());
            present.insert(key.clone());

            if self.nodes.skipped.contains(&key)
                || self.nodes.motion.contains_key(&key)
                || self.nodes.touch.contains_key(&key)
                || self.devices.values().any(|d| d.path == path)
            {
                continue;
            }
            if let Some(pad) = self.gamepads.get(&key)
                && !self.should_manage(&pad.id)
            {
                continue;
            }
            if !worth_opening(&path) {
                self.nodes.skipped.insert(key);
                continue;
            }
            // Permission errors are not cached: udev may grant access a moment later.
            let Ok(dev) = Device::open(&path) else {
                self.note_denied_sensor(key, &path);
                continue;
            };
            self.nodes.denied.remove(&key);
            let name = dev.name().unwrap_or("Unknown").to_string();
            if self.remember_sibling(key.clone(), &path, &dev, &name) {
                continue;
            }
            if name.starts_with(VIRTUAL_PREFIX) || is_uinput(&path) || !input::is_gamepad(&dev) {
                self.nodes.skipped.insert(key);
                continue;
            }
            let analog_triggers = input::has_analog_triggers(&dev);
            let rumble = rumble::supports_rumble(&dev);
            let paddles = input::has_paddles(&dev);
            let id = controller_id(dev.input_id().vendor(), dev.input_id().product(), dev.unique_name());
            self.gamepads.insert(key, SeenGamepad { name: name.clone(), id: id.clone(), analog_triggers, rumble, paddles });
            if self.should_manage(&id) {
                self.manage(path, name, id, dev);
            }
        }
        self.nodes.skipped.retain(|k| present.contains(k));
        self.nodes.motion.retain(|k, _| present.contains(k));
        self.nodes.touch.retain(|k, _| present.contains(k));
        self.nodes.denied.retain(|k, _| present.contains(k));
        self.attach_motion();
        self.attach_touchpad();
        self.gamepads.retain(|k, _| present.contains(k));
    }

    /// Pairs managed controllers with their motion-sensor devices and starts reading them.
    fn attach_motion(&mut self) {
        let pairs: Vec<(u64, PathBuf, String)> = self
            .devices
            .iter()
            .filter(|(_, d)| d.motion.is_none())
            .filter_map(|(id, d)| {
                let taken = |n: &PairedNode| self.devices.values().any(|o| o.motion.as_ref() == Some(&n.path));
                let node = self.nodes.motion.values().find(|n| !taken(n) && n.belongs_to(d))?;
                Some((*id, node.path.clone(), node.name.clone()))
            })
            .collect();
        for (id, path, motion_name) in pairs {
            let Ok(motion_dev) = Device::open(&path) else { continue };
            let Some(dev) = self.devices.get_mut(&id) else { continue };
            let reader = match Reader::motion(motion_dev) {
                Ok(reader) => reader,
                Err(e) => {
                    log!("gyro: cannot read {motion_name}: {e}");
                    continue;
                }
            };
            log!("gyro: using {motion_name} for {}", dev.name);
            dev.motion_frame = MotionFrame::of(&path);
            dev.motion = Some(path);
            dev.gyro_bias = self.config.gyro_calibration.get(&dev.id).copied().unwrap_or_default();
            dev.readers.push(reader);
        }
    }

    fn manage(&mut self, path: PathBuf, name: String, pad_id: String, mut dev: Device) {
        let xbox_labels = input::uses_xbox_labels(&path);
        let parent = hid_parent(&path);
        let family = crate::info::PadFamily::detect(dev.input_id().vendor(), &name);
        let model = crate::info::PadModel::detect(dev.input_id().vendor(), dev.input_id().product(), &name);
        let uniq = dev.unique_name().map(str::to_string);
        let usb = (dev.input_id().vendor(), dev.input_id().product());
        let paddles = input::has_paddles(&dev);
        if let Err(e) = dev.grab() {
            log!("cannot grab {name} ({}): {e}", path.display());
            return;
        }
        let ff = FfSpec::of(&dev);
        let identity = PadIdentity::resolve(self.identity_override, model, self.active_support());
        let Some(new_pad) = self.pad_for_new(ff.as_ref(), identity, self.config.controller(&pad_id).and_then(|c| c.player)) else { return };
        let pad = new_pad.pad;
        let reader = match Reader::pad(dev, xbox_labels) {
            Ok(reader) => reader,
            Err(e) => {
                log!("cannot read {name} ({}): {e}", path.display());
                return;
            }
        };
        log!("managing {name} ({})", path.display());
        let id = self.next_id;
        self.next_id += 1;
        let stop = Arc::new(AtomicBool::new(false));
        let rumble_stop = Arc::new(AtomicBool::new(false));
        // The kept pad advertises no rumble until it's rebuilt to fit (see `first_pad.rs`).
        if ff.is_some() && new_pad.refit_at.is_none() {
            rumble::spawn(pad.shared(), path.clone(), rumble_stop.clone());
        }
        let mut managed = self.new_managed(path, name, Some(pad), stop);
        managed.first = new_pad.first;
        managed.refit_at = new_pad.refit_at;
        managed.ff = ff;
        managed.rumble_stop = rumble_stop;
        managed.family = family;
        managed.model = model;
        managed.parent = parent;
        managed.uniq = uniq;
        managed.usb = usb;
        managed.id = pad_id;
        managed.paddles = paddles;
        managed.readers.push(reader);
        self.add_managed(id, managed);
    }
}

/// Devices created through uinput (Steam Input, other remappers, us) live under
/// /sys/devices/virtual/input. BLE pads via uhid are under /virtual/misc and are kept.
fn is_uinput(dev_path: &Path) -> bool {
    let Some(node) = dev_path.file_name() else { return false };
    let sys = Path::new("/sys/class/input").join(node).join("device");
    std::fs::canonicalize(sys)
        .map(|p| p.to_string_lossy().contains("/devices/virtual/input/"))
        .unwrap_or(false)
}

/// Whether node `path` could be a gamepad, motion sensors or a touchpad, and isn't one of our own
/// virtual devices, going by sysfs. Opening a node asks its driver for all it has, which takes
/// some drivers tens of milliseconds (RGB controllers, sound cards' jacks), and the main loop
/// reads no input meanwhile; sysfs answers without asking the driver. When it can't, the node is
/// opened and looked at.
fn worth_opening(path: &Path) -> bool {
    let Some(node) = path.file_name() else { return true };
    let device = Path::new("/sys/class/input").join(node).join("device");
    if std::fs::read_to_string(device.join("name")).is_ok_and(|name| name.starts_with(VIRTUAL_PREFIX)) {
        return false;
    }
    std::fs::read_to_string(device.join("capabilities/abs")).map_or(true, |abs| has_our_axes(&abs))
}

/// Whether sysfs's absolute-axis bitmap `abs` (hex words, the lowest last) has ABS_X to ABS_RZ
/// (sticks, triggers, accelerometers and gyros) or ABS_MT_POSITION_X (touchpads): every node we
/// use has one. True when it can't be read, so the node is looked at.
fn has_our_axes(abs: &str) -> bool {
    const AXES: u64 = 0x3f | 1 << 0x35;
    abs.split_whitespace().last().and_then(|word| u64::from_str_radix(word, 16).ok()).is_none_or(|low| low & AXES != 0)
}

/// The HID device an input node belongs to; a controller's gamepad and motion-sensor nodes
/// share it.
fn hid_parent(dev_path: &Path) -> Option<PathBuf> {
    let node = dev_path.file_name()?;
    std::fs::canonicalize(Path::new("/sys/class/input").join(node).join("device/device")).ok()
}

#[cfg(test)]
#[path = "evdev/tests.rs"]
mod tests;
