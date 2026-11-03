//! A DualSense (Sony 054c:0ce6) over USB, as `hid-playstation` sees it.
//!
//! The report layouts and sizes follow the kernel's `drivers/hid/hid-playstation.c` (GPL, read as
//! a reference; nothing is copied). The report descriptor is a reduced description of our own: a
//! vendor collection that declares the same report ids and sizes. The driver reads every value
//! from the raw reports, so the real descriptor's usages add nothing.

use super::{BUS_USB, HidPad, Responder, Spec, replica_mac};

pub const NAME: &str = "Sony Interactive Entertainment DualSense Wireless Controller";
pub const VENDOR: u32 = 0x054c;
pub const PRODUCT: u32 = 0x0ce6;

const INPUT_ID: u8 = 0x01;
const CALIBRATION: u8 = 0x05;
const PAIRING: u8 = 0x09;
const FIRMWARE: u8 = 0x20;

/// A vendor-defined application collection with the DualSense's USB reports: input 1 (63 bytes
/// after the id), output 2 (62), and the feature reports the driver reads while probing.
const DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xff, // Usage Page (Vendor 0xff00)
    0x09, 0x01, // Usage (1)
    0xa1, 0x01, // Collection (Application)
    0x85, INPUT_ID, 0x09, 0x20, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 63, 0x81, 0x02, // Input 1
    0x85, 0x02, 0x09, 0x21, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 62, 0x91, 0x02, // Output 2
    0x85, CALIBRATION, 0x09, 0x22, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 40, 0xb1, 0x02, // Feature 5
    0x85, PAIRING, 0x09, 0x23, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 19, 0xb1, 0x02, // Feature 9
    0x85, FIRMWARE, 0x09, 0x24, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 63, 0xb1, 0x02, // Feature 0x20
    0xc0, // End Collection
];

struct Answers {
    mac: [u8; 6],
}

impl Responder for Answers {
    fn get_report(&mut self, number: u8) -> Option<Vec<u8>> {
        let mut data;
        match number {
            CALIBRATION => {
                data = vec![0u8; 41];
                let words: [i16; 17] = [
                    0, 0, 0, // gyro bias: pitch, yaw, roll
                    8192, -8192, 8192, -8192, 8192, -8192, // gyro plus and minus per axis
                    540, 540, // gyro speed plus, minus
                    8192, -8192, 8192, -8192, 8192, -8192, // accelerometer plus and minus per axis
                ];
                for (i, word) in words.iter().enumerate() {
                    data[1 + 2 * i..3 + 2 * i].copy_from_slice(&word.to_le_bytes());
                }
            }
            PAIRING => {
                data = vec![0u8; 20];
                data[1..7].copy_from_slice(&self.mac);
            }
            FIRMWARE => {
                data = vec![0u8; 64];
                data[24..28].copy_from_slice(&0x0000_0a00u32.to_le_bytes()); // hardware version
                data[28..32].copy_from_slice(&0x0100_0900u32.to_le_bytes()); // firmware version
                data[44..46].copy_from_slice(&0x0215u16.to_le_bytes()); // update version 2.21: rumble v2
            }
            _ => return None,
        }
        data[0] = number;
        Some(data)
    }
}

/// What a DualSense reports. The default is a pad at rest: sticks centred, nothing pressed, no
/// finger on the touchpad.
#[derive(Clone)]
pub struct DualSenseState {
    pub cross: bool,
    pub circle: bool,
    pub ps: bool,
    /// Square and triangle (the other two face buttons).
    pub square: bool,
    pub triangle: bool,
    /// The d-pad: 0 is up, then clockwise in steps of 45 degrees; 8 is centred.
    pub hat: u8,
    /// The byte of shoulder buttons: bit 0 L1, 1 R1, 2 L2, 3 R2, 4 create, 5 options, 6 L3, 7 R3.
    pub shoulders: u8,
    /// Left stick x and y, right stick x and y (128 is centred).
    pub sticks: [u8; 4],
    /// The analog triggers, left then right.
    pub triggers: [u8; 2],
    /// The touchpad pressed down.
    pub touch_click: bool,
    /// Raw gyro readings (pitch, yaw, roll); the driver scales them with the calibration above.
    pub gyro: [i16; 3],
    pub accel: [i16; 3],
    /// Two fingers: position on the 1920 by 1080 pad, or none.
    pub touch: [Option<(u16, u16)>; 2],
    /// The sensor's clock in 0.33 microsecond units. Advance it between reports.
    pub timestamp: u32,
}

impl Default for DualSenseState {
    fn default() -> Self {
        // Lying flat: gravity on the accelerometer's z axis.
        DualSenseState { cross: false, circle: false, ps: false, square: false, triangle: false, hat: 8, shoulders: 0, sticks: [128; 4], triggers: [0; 2], touch_click: false, gyro: [0; 3], accel: [0, 0, 8192], touch: [None; 2], timestamp: 0 }
    }
}

impl DualSenseState {
    /// The USB input report (id 1, 64 bytes).
    pub fn report(&self) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = INPUT_ID;
        r[1..5].copy_from_slice(&self.sticks);
        r[5..7].copy_from_slice(&self.triggers);
        r[8] = self.hat & 0x0f
            | if self.square { 0x10 } else { 0 }
            | if self.cross { 0x20 } else { 0 }
            | if self.circle { 0x40 } else { 0 }
            | if self.triangle { 0x80 } else { 0 };
        r[9] = self.shoulders;
        r[10] = u8::from(self.ps) | if self.touch_click { 0x02 } else { 0 };
        for (i, v) in self.gyro.iter().chain(&self.accel).enumerate() {
            r[16 + 2 * i..18 + 2 * i].copy_from_slice(&v.to_le_bytes());
        }
        r[28..32].copy_from_slice(&self.timestamp.to_le_bytes());
        for (i, finger) in self.touch.iter().enumerate() {
            let p = &mut r[33 + 4 * i..37 + 4 * i];
            match finger {
                Some((x, y)) => {
                    p[0] = i as u8 + 1; // the contact id; bit 7 clear means touching
                    p[1] = (x & 0xff) as u8;
                    p[2] = ((x >> 8) & 0xf) as u8 | ((y & 0xf) << 4) as u8;
                    p[3] = (y >> 4) as u8;
                }
                None => p[0] = 0x80,
            }
        }
        r[53] = 0x08; // battery 80%, discharging
        r
    }
}

impl HidPad {
    /// A DualSense on USB. `n` (below 200) picks its address, which must differ between pads that
    /// exist at the same time. The kernel makes its gamepad, "... Motion Sensors" and "...
    /// Touchpad" nodes.
    pub fn dualsense(n: u8) -> Self {
        let uniq = "";
        let spec = Spec { uniq, bus: BUS_USB, vendor: VENDOR, product: PRODUCT, descriptor: DESCRIPTOR };
        Self::create(&spec, NAME, Some(Box::new(Answers { mac: replica_mac(n) })))
    }
}
