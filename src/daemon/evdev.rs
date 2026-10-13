//! Controllers on Linux: finds gamepads in /dev/input, grabs them so nothing else sees them, pairs
//! them with their motion-sensor and touchpad nodes, and reads each on its own thread.

use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
};

use evdev::Device;

pub(super) use super::pads::{FfSpec, virtual_pad_for};
use super::{Daemon, Managed, Msg, SeenGamepad};
use crate::{
    input::{self, InputEvent, MotionFrame, MotionNormalizer, MotionSample, Normalizer},
    monitor::log,
    output::VIRTUAL_PREFIX,
    pad_identity::PadIdentity,
    rumble,
};

/// A device node identity; the inode changes when a node is recreated for a new device.
pub(super) type NodeKey = (PathBuf, u64);

/// How long a reader waits for input before it checks whether to stop.
const POLL_TIMEOUT_MS: i32 = 200;

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
                && (!self.takes_pads() || self.config.ignored_devices.contains(&pad.name))
            {
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
            self.gamepads.insert(key, SeenGamepad { name: name.clone(), analog_triggers, rumble, paddles });
            if self.config.enabled && !self.config.ignored_devices.contains(&name) {
                self.manage(path, name, dev);
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
            log!("gyro: using {motion_name} for {}", dev.name);
            dev.motion_frame = MotionFrame::of(&path);
            dev.motion = Some(path);
            dev.gyro_bias = self.config.gyro_calibration.get(&dev.name).copied().unwrap_or_default();
            let (stop, tx) = (dev.stop.clone(), self.tx.clone());
            thread::spawn(move || read_motion(id, motion_dev, &stop, &tx));
        }
    }

    fn manage(&mut self, path: PathBuf, name: String, mut dev: Device) {
        let parent = hid_parent(&path);
        let family = crate::info::PadFamily::detect(dev.input_id().vendor(), &name);
        let model = crate::info::PadModel::detect(dev.input_id().vendor(), dev.input_id().product(), &name);
        let uniq = dev.unique_name().map(str::to_string);
        let paddles = input::has_paddles(&dev);
        if let Err(e) = dev.grab() {
            log!("cannot grab {name} ({}): {e}", path.display());
            return;
        }
        let ff = FfSpec::of(&dev);
        let identity = PadIdentity::resolve(self.identity_override, model, self.active_support());
        let Some(pad) = virtual_pad_for(ff.as_ref(), identity) else { return };
        log!("managing {name} ({})", path.display());
        let id = self.next_id;
        self.next_id += 1;
        let stop = Arc::new(AtomicBool::new(false));
        {
            let stop = stop.clone();
            let tx = self.tx.clone();
            let xbox_labels = input::uses_xbox_labels(&path);
            thread::spawn(move || read_device(id, dev, xbox_labels, &stop, &tx));
        }
        let rumble_stop = Arc::new(AtomicBool::new(false));
        if ff.is_some() {
            rumble::spawn(pad.shared(), path.clone(), rumble_stop.clone());
        }
        let mut managed = self.new_managed(path, name, Some(pad), stop);
        managed.ff = ff;
        managed.rumble_stop = rumble_stop;
        managed.family = family;
        managed.model = model;
        managed.parent = parent;
        managed.uniq = uniq;
        managed.paddles = paddles;
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

/// The HID device an input node belongs to; a controller's gamepad and motion-sensor nodes
/// share it.
fn hid_parent(dev_path: &Path) -> Option<PathBuf> {
    let node = dev_path.file_name()?;
    std::fs::canonicalize(Path::new("/sys/class/input").join(node).join("device/device")).ok()
}

fn read_motion(id: u64, mut dev: Device, stop: &AtomicBool, tx: &Sender<Msg>) {
    let mut norm = MotionNormalizer::new(&dev);
    if dev.set_nonblocking(true).is_err() {
        let _ = tx.send(Msg::MotionGone { id });
        return;
    }
    let mut pfd = libc::pollfd { fd: dev.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    while !stop.load(Ordering::Relaxed) {
        // SAFETY: pfd points to a single valid pollfd for the duration of the call.
        let n = unsafe { libc::poll(&mut pfd, 1, POLL_TIMEOUT_MS) };
        if n < 0 && std::io::Error::last_os_error().kind() != ErrorKind::Interrupted {
            break;
        }
        if n <= 0 {
            continue;
        }
        if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            break;
        }
        let samples: Vec<MotionSample> = match dev.fetch_events() {
            Ok(evs) => evs.filter_map(|ev| norm.translate(ev)).collect(),
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(_) => break,
        };
        for sample in samples {
            if tx.send(Msg::Motion { id, sample }).is_err() {
                return;
            }
        }
    }
    let _ = tx.send(Msg::MotionGone { id });
}

fn read_device(id: u64, mut dev: Device, xbox_labels: bool, stop: &AtomicBool, tx: &Sender<Msg>) {
    let mut norm = Normalizer::new(&dev, xbox_labels);
    read_input(id, &mut dev, stop, tx, |ev, out| norm.translate(ev, out));
    // Dropping `dev` closes the fd, which also releases the grab.
    let _ = tx.send(Msg::Gone { id });
}

/// Reads raw events from `dev` until `stop` is set or the device goes away. `translate` turns
/// each event into inputs, which go to the controller `id` one batch at a time.
pub(super) fn read_input(
    id: u64,
    dev: &mut Device,
    stop: &AtomicBool,
    tx: &Sender<Msg>,
    mut translate: impl FnMut(evdev::InputEvent, &mut Vec<InputEvent>),
) {
    if let Err(e) = dev.set_nonblocking(true) {
        log!("set_nonblocking: {e}");
        return;
    }
    let mut pfd = libc::pollfd { fd: dev.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    let mut events = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        // SAFETY: pfd points to a single valid pollfd for the duration of the call.
        let n = unsafe { libc::poll(&mut pfd, 1, POLL_TIMEOUT_MS) };
        if n < 0 {
            if std::io::Error::last_os_error().kind() == ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        if n == 0 {
            continue;
        }
        if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            break;
        }
        match dev.fetch_events() {
            Ok(evs) => evs.for_each(|ev| translate(ev, &mut events)),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        if !events.is_empty() && tx.send(Msg::Input { id, events: std::mem::take(&mut events) }).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, parent: Option<&str>, uniq: Option<&str>) -> PairedNode {
        PairedNode {
            path: PathBuf::from("/dev/input/event99"),
            name: name.into(),
            parent: parent.map(PathBuf::from),
            uniq: uniq.map(Into::into),
        }
    }

    #[test]
    fn motion_sensors_pair_with_their_own_controller() {
        let hid = Some(PathBuf::from("/sys/devices/x/0005:054C:0CE6.0001"));
        let mac = Some("a0:5a:5c:00:00:01".to_string());
        let pad = "Sony Interactive Entertainment DualSense Wireless Controller";

        // Same HID parent (USB or Bluetooth), or same MAC.
        assert!(node("x", Some("/sys/devices/x/0005:054C:0CE6.0001"), None).matches(pad, &hid, &None));
        assert!(node("x", None, Some("a0:5a:5c:00:00:01")).matches(pad, &None, &mac));
        // Driver naming: "<pad> Motion Sensors" (hid-playstation), "<pad> IMU" (hid-nintendo).
        assert!(node(&format!("{pad} Motion Sensors"), None, None).matches(pad, &None, &None));
        assert!(node("Nintendo Switch Pro Controller IMU", None, None).matches("Nintendo Switch Pro Controller", &None, &None));

        // A second identical controller's sensors don't match by parent or MAC, even though
        // the name would.
        let other = node(&format!("{pad} Motion Sensors"), Some("/sys/devices/x/0005:054C:0CE6.0002"), Some("a0:5a:5c:00:00:02"));
        assert!(!other.matches(pad, &hid, &mac));
        // Empty unique IDs (common over USB) never count as a match.
        assert!(!node("x", None, Some("")).matches(pad, &None, &Some(String::new())));
    }
}
