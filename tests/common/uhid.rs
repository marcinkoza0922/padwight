//! A gamepad made through the kernel's uhid, so the daemon sees a HID device like a real controller.
//! The daemon ignores uinput devices (they count as its own), so a uinput stand-in won't be grabbed.
//!
//! [`HidPad::new`] makes a plain generic pad. [`HidPad::dualsense`], [`HidPad::dualshock4`] and
//! [`HidPad::switch_pro`] make replicas of real controllers: same vendor and product ids, and
//! the feature and output reports their kernel drivers (`hid-playstation`, `hid-nintendo`) ask
//! for while probing are answered, so the kernel binds the real driver. That driver then makes
//! the gamepad, motion-sensor and touchpad nodes a real controller has. It needs the driver's
//! module on the host (the kernel loads it when the device appears).

mod dualsense;
mod responder;

use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::fd::AsRawFd,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

#[allow(unused_imports, reason = "each test binary uses a different subset")]
pub use dualsense::DualSenseState;
pub use responder::Responder;

/// Report descriptor of a plain gamepad: four buttons (the first four are South, East, North and
/// West once the kernel maps them) and an X/Y stick, with one button byte then two 16-bit axes.
const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x05, // Usage (Gamepad)
    0xa1, 0x01, // Collection (Application)
    0x05, 0x09, // Usage Page (Button)
    0x19, 0x01, // Usage Minimum (1)
    0x29, 0x04, // Usage Maximum (4)
    0x15, 0x00, // Logical Minimum (0)
    0x25, 0x01, // Logical Maximum (1)
    0x75, 0x01, // Report Size (1)
    0x95, 0x04, // Report Count (4)
    0x81, 0x02, // Input (Data, Variable, Absolute)
    0x75, 0x04, // Report Size (4)
    0x95, 0x01, // Report Count (1)
    0x81, 0x03, // Input (Constant, Variable, Absolute): padding
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x30, // Usage (X)
    0x09, 0x31, // Usage (Y)
    0x16, 0x00, 0x80, // Logical Minimum (-32768)
    0x26, 0xff, 0x7f, // Logical Maximum (32767)
    0x75, 0x10, // Report Size (16)
    0x95, 0x02, // Report Count (2)
    0x81, 0x02, // Input (Data, Variable, Absolute)
    0xc0, // End Collection
];

pub const BUTTON_SOUTH: u8 = 0b0001;
pub const BUTTON_EAST: u8 = 0b0010;

/// Size of the kernel's `struct uhid_event` (packed): a type, then the largest payload.
const UHID_EVENT_SIZE: usize = 4376;
const UHID_DESTROY: u32 = 1;
const UHID_OUTPUT: u32 = 6;
const UHID_GET_REPORT: u32 = 9;
const UHID_GET_REPORT_REPLY: u32 = 10;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;
const UHID_SET_REPORT: u32 = 13;
const UHID_SET_REPORT_REPLY: u32 = 14;

const BUS_USB: u16 = 0x0003;

/// The unique id every test pad reports, which its controller id becomes.
pub const TEST_UNIQ: &str = "padwight-test-pad";

/// What every replica's unique id (the MAC address its driver reads from the pad) starts with. A
/// locally administered address, so it never equals a real controller's.
const REPLICA_MAC_PREFIX: &str = "02:70:61:64:";

/// True for the unique id of a pad this module made, whichever kind.
pub fn is_test_uniq(uniq: &str) -> bool {
    uniq == TEST_UNIQ || uniq.starts_with(REPLICA_MAC_PREFIX)
}

/// The MAC address (least significant byte first, as a controller reports it) of the `n`th replica.
/// The drivers refuse two pads with one address, and show it reversed as the pad's unique id:
/// `02:70:61:64:00:nn`.
fn replica_mac(n: u8) -> [u8; 6] {
    [n, 0x00, 0x64, 0x61, 0x70, 0x02]
}

/// What the kernel needs to make one device.
struct Spec<'a> {
    uniq: &'a str,
    bus: u16,
    vendor: u32,
    product: u32,
    descriptor: &'static [u8],
}

/// A gamepad made through the kernel's uhid, so it is a HID device like a real controller. The
/// daemon ignores uinput devices (they count as its own), so a uinput stand-in won't be grabbed.
pub struct HidPad {
    file: Arc<File>,
    /// Answers the driver while it probes; present for replicas.
    server: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    /// Output reports the driver (or the daemon, through the driver) has sent.
    outputs: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl HidPad {
    /// A controller named `name`. Dropping it removes the device, as an unplug does.
    pub fn new(name: &str) -> Self {
        Self::with_uniq(name, TEST_UNIQ)
    }

    /// A controller called `name` with its own unique id, for tests that run several at once (the
    /// daemon tells controllers apart by it).
    pub fn with_uniq(name: &str, uniq: &str) -> Self {
        let spec = Spec { uniq, bus: BUS_USB, vendor: 0x045e, product: 0x028e, descriptor: DESCRIPTOR };
        Self::create(&spec, name, None)
    }

    fn create(spec: &Spec<'_>, name: &str, responder: Option<Box<dyn Responder>>) -> Self {
        let file = OpenOptions::new().read(true).write(true).open("/dev/uhid").expect("opening /dev/uhid (needs root or the uhid device)");
        let file = Arc::new(file);
        let outputs = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        // The driver asks its questions while the kernel adds the device, so the answering thread
        // runs before the device is created.
        let server = responder.map(|responder| {
            let (file, outputs, stop) = (Arc::clone(&file), Arc::clone(&outputs), Arc::clone(&stop));
            thread::spawn(move || serve(&file, responder, &outputs, &stop))
        });
        let mut event = vec![0u8; UHID_EVENT_SIZE];
        event[0..4].copy_from_slice(&UHID_CREATE2.to_le_bytes());
        // Packed `uhid_create2_req`, starting after the type: name[128], phys[64], uniq[64], then:
        event[4..4 + name.len()].copy_from_slice(name.as_bytes());
        // A unique id of its own, so a real pad with the same vendor and product (ignored by
        // `default_config`) doesn't take this one's id. Replicas' drivers replace it with the MAC.
        event[196..196 + spec.uniq.len()].copy_from_slice(spec.uniq.as_bytes());
        event[260..262].copy_from_slice(&(spec.descriptor.len() as u16).to_le_bytes()); // rd_size
        event[262..264].copy_from_slice(&spec.bus.to_le_bytes());
        event[264..268].copy_from_slice(&spec.vendor.to_le_bytes());
        event[268..272].copy_from_slice(&spec.product.to_le_bytes());
        event[272..276].copy_from_slice(&1u32.to_le_bytes()); // version
        event[280..280 + spec.descriptor.len()].copy_from_slice(spec.descriptor); // rd_data
        (&*file).write_all(&event).unwrap();
        HidPad { file, server, stop, outputs }
    }

    /// Sends the controller's state. The kernel drops reports until the daemon has opened the
    /// device, so callers send the state again until it shows up.
    pub fn report(&mut self, buttons: u8, x: i16, y: i16) {
        let mut data = vec![buttons];
        data.extend(x.to_le_bytes());
        data.extend(y.to_le_bytes());
        self.send(&data);
    }

    /// Sends an input report as it is on the wire, report id first (if the device uses ids).
    pub fn send(&mut self, data: &[u8]) {
        write_input(&self.file, data);
    }

    /// The output reports (rumble, lightbar, subcommands) the driver has sent so far, oldest first.
    pub fn outputs(&self) -> Vec<Vec<u8>> {
        self.outputs.lock().unwrap().clone()
    }
}

impl Drop for HidPad {
    fn drop(&mut self) {
        // The answering thread holds the file too, so closing it isn't enough to remove the device.
        let Some(server) = self.server.take() else { return };
        self.stop.store(true, Ordering::Relaxed);
        let mut event = vec![0u8; UHID_EVENT_SIZE];
        event[0..4].copy_from_slice(&UHID_DESTROY.to_le_bytes());
        let _ = (&*self.file).write_all(&event);
        let _ = server.join();
    }
}

fn write_input(file: &File, data: &[u8]) {
    let mut event = vec![0u8; UHID_EVENT_SIZE];
    event[0..4].copy_from_slice(&UHID_INPUT2.to_le_bytes());
    event[4..6].copy_from_slice(&(data.len() as u16).to_le_bytes());
    event[6..6 + data.len()].copy_from_slice(data);
    let _ = (&*file).write_all(&event);
}

/// Reads what the kernel asks of the device until it is destroyed, and answers.
fn serve(file: &File, mut responder: Box<dyn Responder>, outputs: &Mutex<Vec<Vec<u8>>>, stop: &AtomicBool) {
    let mut event = vec![0u8; UHID_EVENT_SIZE];
    while !stop.load(Ordering::Relaxed) {
        // Waits a little, so the thread notices it is to stop; a read would wait for ever.
        let mut poll = libc::pollfd { fd: file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        // SAFETY: one valid pollfd, and the count says so.
        if unsafe { libc::poll(&raw mut poll, 1, 100) } <= 0 {
            continue;
        }
        if !matches!((&*file).read(&mut event), Ok(n) if n >= 4) {
            return;
        }
        let kind = u32::from_le_bytes(event[0..4].try_into().unwrap());
        match kind {
            UHID_GET_REPORT => {
                let id: [u8; 4] = event[4..8].try_into().unwrap();
                let number = event[8];
                let mut reply = vec![0u8; UHID_EVENT_SIZE];
                reply[0..4].copy_from_slice(&UHID_GET_REPORT_REPLY.to_le_bytes());
                reply[4..8].copy_from_slice(&id);
                match responder.get_report(number) {
                    Some(data) => {
                        reply[10..12].copy_from_slice(&(data.len() as u16).to_le_bytes());
                        reply[12..12 + data.len()].copy_from_slice(&data);
                    }
                    None => reply[8..10].copy_from_slice(&5u16.to_le_bytes()), // EIO
                }
                let _ = (&*file).write_all(&reply);
            }
            UHID_SET_REPORT => {
                let mut reply = vec![0u8; UHID_EVENT_SIZE];
                reply[0..4].copy_from_slice(&UHID_SET_REPORT_REPLY.to_le_bytes());
                reply[4..8].copy_from_slice(&event[4..8]);
                let _ = (&*file).write_all(&reply);
            }
            UHID_OUTPUT => {
                let size = u16::from_le_bytes(event[4100..4102].try_into().unwrap()) as usize;
                let data = event[4..4 + size.min(4096)].to_vec();
                for answer in responder.output(&data) {
                    write_input(file, &answer);
                }
                outputs.lock().unwrap().push(data);
            }
            _ => {}
        }
    }
}
