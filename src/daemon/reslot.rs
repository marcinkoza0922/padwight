//! Putting Padwight's pad ahead of the real one in Steam's slots. Steam numbers controllers as
//! they arrive, giving each the lowest free slot, and the real pad always arrived first: games
//! reading Steam's controller 0 got the grabbed, silent real pad. So while Steam runs, the first
//! time Padwight takes a USB pad it lets go of the pad's kernel driver for a moment. Steam drops
//! the pad, Padwight's virtual pad is built into the free slot, and the driver binds again, so the
//! real pad comes back behind it. See docs/development/steam-input.md.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
    time::{Duration, Instant},
};

use evdev::Device;

use super::{Daemon, Msg, pads::FfSpec, virtual_pad_for};
use crate::{monitor::log, output::VirtualPad, pad_identity::PadIdentity};

/// How long Steam gets to drop the real pad before the virtual pad is built.
const STEAM_DROPS: Duration = Duration::from_millis(1000);
/// How long Steam gets to list the virtual pad before the real one comes back.
const STEAM_LISTS: Duration = Duration::from_millis(500);
/// How long scans pass over a pad being reordered before taking it anyway.
const PENDING_LIMIT: Duration = Duration::from_secs(6);
/// Marks a driver let go of, in the runtime directory, so a daemon killed meanwhile can bind it
/// again when it next starts.
const RECORD_PREFIX: &str = "padwight-detached.";

const USBDEVFS_RELEASEINTERFACE: libc::Ioctl = libc::_IOR::<libc::c_uint>(b'U' as u32, 16);
const USBDEVFS_IOCTL: libc::Ioctl = libc::_IOWR::<UsbIoctl>(b'U' as u32, 18);
const USBDEVFS_CONNECT: libc::c_int = libc::_IO(b'U' as u32, 23) as libc::c_int;
const USBDEVFS_DISCONNECT_CLAIM: libc::Ioctl = libc::_IOR::<DisconnectClaim>(b'U' as u32, 27);

#[repr(C)]
struct DisconnectClaim {
    interface: libc::c_uint,
    flags: libc::c_uint,
    driver: [libc::c_char; 256],
}

#[repr(C)]
struct UsbIoctl {
    ifno: libc::c_int,
    ioctl_code: libc::c_int,
    data: *mut libc::c_void,
}

/// The USB interface an input node reads from, as usbfs addresses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UsbInterface {
    /// The USB device in sysfs, e.g. `/sys/devices/…/1-8.4`.
    device: PathBuf,
    /// Its usbfs node, `/dev/bus/usb/BBB/DDD`. Each plug-in gets a new one.
    node: PathBuf,
    number: u32,
}

/// The USB interface behind input node `event`, if it is on USB.
pub(super) fn usb_interface(event: &Path) -> Option<UsbInterface> {
    let name = event.file_name()?;
    let mut dir = fs::canonicalize(Path::new("/sys/class/input").join(name).join("device")).ok()?;
    loop {
        if let Ok(number) = fs::read_to_string(dir.join("bInterfaceNumber")) {
            let number = u32::from_str_radix(number.trim(), 16).ok()?;
            let device = dir.parent()?.to_path_buf();
            let read = |file: &str| -> Option<u32> { fs::read_to_string(device.join(file)).ok()?.trim().parse().ok() };
            let node = PathBuf::from(format!("/dev/bus/usb/{:03}/{:03}", read("busnum")?, read("devnum")?));
            return Some(UsbInterface { device, node, number });
        }
        if !dir.pop() || dir == Path::new("/sys/devices") {
            return None;
        }
    }
}

/// The reorders done and under way.
#[derive(Default)]
pub(super) struct Reslots {
    /// Pads already reordered, by usbfs node.
    done: HashSet<PathBuf>,
    /// Pads being reordered, by USB device, and since when.
    pending: HashMap<PathBuf, Instant>,
    /// Virtual pads built while their real pad was away, by USB device.
    parked: HashMap<PathBuf, VirtualPad>,
}

impl Daemon {
    /// Whether the scan should leave the gamepad at `path` for now: it is being reordered, or a
    /// reorder for it just started.
    pub(super) fn hold_for_reorder(&mut self, path: &Path, dev: &Device) -> bool {
        let Some(usb) = usb_interface(path) else { return false };
        if let Some(since) = self.reslots.pending.get(&usb.device) {
            if since.elapsed() < PENDING_LIMIT {
                return true;
            }
            self.reslots.pending.remove(&usb.device);
        }
        if self.reslots.done.contains(&usb.node) || !crate::guide_menu::steam_running() {
            return false;
        }
        self.reslots.done.insert(usb.node.clone());
        // Without write access to the node (no udev rule for it), the pad is grabbed as before.
        let file = match OpenOptions::new().read(true).write(true).open(&usb.node) {
            Ok(file) => file,
            Err(e) => {
                log!("cannot put Padwight's pad ahead of {} in Steam: {}: {e}", dev.name().unwrap_or("?"), usb.node.display());
                return false;
            }
        };
        log!("letting go of {}'s driver for a moment, so Steam lists Padwight's pad first", dev.name().unwrap_or("?"));
        self.reslots.pending.insert(usb.device.clone(), Instant::now());
        let (ff, identity, tx) = (FfSpec::of(dev), self.pad_identity_for(dev), self.tx.clone());
        thread::spawn(move || reorder(&file, &usb, ff.as_ref(), identity, &tx));
        true
    }

    /// The virtual pad built for the USB pad at `path` while it was away, if there is one.
    pub(super) fn take_parked(&mut self, path: &Path) -> Option<VirtualPad> {
        self.reslots.parked.remove(&usb_interface(path)?.device)
    }

    /// A reorder finished with `pad` built, or failed.
    pub(super) fn parked(&mut self, device: PathBuf, pad: Option<VirtualPad>) {
        self.reslots.pending.remove(&device);
        if let Some(pad) = pad {
            self.reslots.parked.insert(device, pad);
        }
    }
}

/// Lets go of the driver, builds the virtual pad once Steam has dropped the real one, hands it
/// to the daemon, and binds the driver again.
fn reorder(file: &File, usb: &UsbInterface, ff: Option<&FfSpec>, identity: PadIdentity, tx: &Sender<Msg>) {
    let pad = match Detached::new(file, usb) {
        Ok(detached) => {
            thread::sleep(STEAM_DROPS);
            let pad = virtual_pad_for(ff, identity);
            thread::sleep(STEAM_LISTS);
            // Sent before the driver is back, so the scan that finds the pad again can use it.
            let _ = tx.send(Msg::Parked { device: usb.device.clone(), pad });
            drop(detached);
            return;
        }
        Err(e) => {
            log!("cannot let go of the driver of {}: {e}", usb.node.display());
            None
        }
    };
    let _ = tx.send(Msg::Parked { device: usb.device.clone(), pad });
}

/// A USB interface taken from its kernel driver. Dropping it lets the driver bind again.
struct Detached<'a> {
    file: &'a File,
    number: u32,
    record: Option<PathBuf>,
}

impl<'a> Detached<'a> {
    fn new(file: &'a File, usb: &UsbInterface) -> io::Result<Self> {
        let record = record_path(&usb.node, usb.number);
        if let Some(record) = &record {
            let _ = fs::write(record, format!("{}\n{}\n", usb.node.display(), usb.number));
        }
        let claim = DisconnectClaim { interface: usb.number, flags: 0, driver: [0; 256] };
        // SAFETY: the fd is open, and the ioctl reads one DisconnectClaim.
        if unsafe { libc::ioctl(file.as_raw_fd(), USBDEVFS_DISCONNECT_CLAIM, &claim) } < 0 {
            let e = io::Error::last_os_error();
            if let Some(record) = &record {
                let _ = fs::remove_file(record);
            }
            return Err(e);
        }
        Ok(Detached { file, number: usb.number, record })
    }
}

impl Drop for Detached<'_> {
    fn drop(&mut self) {
        if let Err(e) = reattach(self.file, self.number) {
            log!("cannot give a controller back to its driver ({e}); replugging it will");
        }
        if let Some(record) = &self.record {
            let _ = fs::remove_file(record);
        }
    }
}

/// Releases interface `number` of the USB device open as `file` and lets the kernel bind a driver to it.
fn reattach(file: &File, number: u32) -> io::Result<()> {
    let fd = file.as_raw_fd();
    // SAFETY: the fd is open; RELEASEINTERFACE reads one uint, and USBDEVFS_IOCTL reads one
    // UsbIoctl, whose CONNECT request takes no data. Releasing fails harmlessly when nothing holds
    // the interface (a daemon killed meanwhile).
    unsafe { libc::ioctl(fd, USBDEVFS_RELEASEINTERFACE, &number) };
    let request = UsbIoctl { ifno: number as libc::c_int, ioctl_code: USBDEVFS_CONNECT, data: std::ptr::null_mut() };
    // SAFETY: as above.
    if unsafe { libc::ioctl(fd, USBDEVFS_IOCTL, &request) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn record_path(node: &Path, number: u32) -> Option<PathBuf> {
    let mut parts = node.iter().rev().take(2).map(|p| p.to_string_lossy().into_owned());
    let (dev, bus) = (parts.next()?, parts.next()?);
    Some(crate::ipc::shared_runtime_dir()?.join(format!("{RECORD_PREFIX}{bus}.{dev}.{number}")))
}

/// Binds the drivers again that a daemon killed in the middle of a reorder left unbound.
pub(super) fn reattach_leftovers() {
    let Some(dir) = crate::ipc::shared_runtime_dir() else { return };
    let Ok(entries) = fs::read_dir(&dir) else { return };
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(RECORD_PREFIX) {
            continue;
        }
        let record = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut lines = record.lines();
        if let (Some(node), Some(Ok(number))) = (lines.next(), lines.next().map(str::parse)) {
            match OpenOptions::new().read(true).write(true).open(node).and_then(|f| reattach(&f, number)) {
                Ok(()) => log!("gave {node} back to its driver, which a stopped daemon had let go of"),
                // A replugged pad has a new node, and its driver bound on its own.
                Err(e) => log!("{node} left without its driver earlier: {e}"),
            }
        }
        let _ = fs::remove_file(entry.path());
    }
}
