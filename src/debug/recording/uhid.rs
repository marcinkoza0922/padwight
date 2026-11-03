//! Replaying a gamepad through the kernel's uhid, so the daemon sees a HID device (it ignores
//! uinput ones, which count as its own). The recorded buttons and axes become a HID report
//! descriptor, and each frame becomes a report of the whole state.
//!
//! Covered: face, shoulder and trigger-happy buttons the HID gamepad mapping can express, the
//! axes X, Y, Z, RX, RY, RZ, GAS and BRAKE, and the D-pad as one hat switch. Motion sensors,
//! touchpads and the back paddles cannot be made this way (a note says so when one is skipped).

use std::{collections::HashMap, fs::File, io::Write};

use anyhow::{Context, Result, bail};

use super::{
    model::{AbsSpec, DeviceSpec, EV_ABS, EV_KEY, Event},
    replay::Sink,
};

const BTN_SOUTH: u16 = 0x130;
const BTN_TRIGGER_HAPPY: u16 = 0x2c0;
const ABS_GAS: u16 = 9;
const ABS_BRAKE: u16 = 10;
const ABS_HAT0X: u16 = 16;
const ABS_HAT0Y: u16 = 17;

/// The HID usage (on the Button page) the kernel turns into `code` inside a gamepad collection.
fn button_usage(code: u16) -> Option<u16> {
    match code {
        BTN_SOUTH..=0x13f => Some(code - BTN_SOUTH + 1),
        c if (BTN_TRIGGER_HAPPY..BTN_TRIGGER_HAPPY + 0x40).contains(&c) => Some(c - BTN_TRIGGER_HAPPY + 17),
        _ => None,
    }
}

/// The usage page and usage that become the absolute axis `code`.
fn axis_usage(code: u16) -> Option<(u8, u16)> {
    match code {
        0..=5 => Some((0x01, 0x30 + code)),
        ABS_GAS => Some((0x02, 0xc4)),
        ABS_BRAKE => Some((0x02, 0xc5)),
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Field {
    axis: AbsSpec,
    /// Bits in the report: 8, 16 or 32.
    bits: u8,
}

/// The report descriptor made for a recorded device, and how a report is laid out under it.
#[derive(Debug)]
pub struct Layout {
    pub descriptor: Vec<u8>,
    buttons: Vec<u16>,
    axes: Vec<Field>,
    hat: bool,
    /// Recorded things the descriptor could not express.
    pub skipped: Vec<String>,
}

fn item(out: &mut Vec<u8>, tag: u8, value: i32) {
    out.push(tag | 3);
    out.extend(value.to_le_bytes());
}

fn usage_item(out: &mut Vec<u8>, usage: u16) {
    out.extend([0x0a, usage as u8, (usage >> 8) as u8]);
}

fn padding(out: &mut Vec<u8>, bits: usize) {
    if !bits.is_multiple_of(8) {
        out.extend([0x75, (8 - bits % 8) as u8, 0x95, 0x01, 0x81, 0x03]);
    }
}

pub fn layout(spec: &DeviceSpec) -> Result<Layout> {
    if !spec.keys.contains(&BTN_SOUTH) || !spec.abs.iter().any(|a| a.code == 0) {
        bail!("{} is not a gamepad; uhid replay makes gamepads only", spec.name);
    }
    let mut skipped = Vec::new();
    let mut buttons: Vec<(u16, u16)> = Vec::new();
    for &k in &spec.keys {
        match button_usage(k) {
            Some(u) => buttons.push((u, k)),
            None => skipped.push(format!("key {k}")),
        }
    }
    buttons.sort_unstable();
    let mut axes = Vec::new();
    for a in &spec.abs {
        if axis_usage(a.code).is_some() {
            let signed_fits = |lo: i64, hi: i64| i64::from(a.min) >= lo && i64::from(a.max) <= hi;
            let bits = if a.min >= 0 && signed_fits(0, 0xff) || signed_fits(-128, 127) {
                8
            } else if a.min >= 0 && signed_fits(0, 0xffff) || signed_fits(-32768, 32767) {
                16
            } else {
                32
            };
            axes.push(Field { axis: *a, bits });
        } else if a.code != ABS_HAT0X && a.code != ABS_HAT0Y {
            skipped.push(format!("axis {}", a.code));
        }
    }
    let hat = spec.abs.iter().any(|a| a.code == ABS_HAT0X) && spec.abs.iter().any(|a| a.code == ABS_HAT0Y);
    if !hat && spec.abs.iter().any(|a| a.code == ABS_HAT0X || a.code == ABS_HAT0Y) {
        skipped.push("half a D-pad".into());
    }

    let mut d = vec![0x05, 0x01, 0x09, 0x05, 0xa1, 0x01];
    if !buttons.is_empty() {
        d.extend([0x05, 0x09]);
        buttons.iter().for_each(|(u, _)| usage_item(&mut d, *u));
        item(&mut d, 0x14, 0);
        item(&mut d, 0x24, 1);
        d.extend([0x75, 0x01, 0x95, buttons.len() as u8, 0x81, 0x02]);
        padding(&mut d, buttons.len());
    }
    for f in &axes {
        let (page, usage) = axis_usage(f.axis.code).context("axis")?;
        d.extend([0x05, page]);
        usage_item(&mut d, usage);
        item(&mut d, 0x14, f.axis.min);
        item(&mut d, 0x24, f.axis.max);
        d.extend([0x75, f.bits, 0x95, 0x01, 0x81, 0x02]);
    }
    if hat {
        d.extend([0x05, 0x01, 0x09, 0x39, 0x15, 0x01, 0x25, 0x08, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42]);
        padding(&mut d, 4);
    }
    d.push(0xc0);
    Ok(Layout { descriptor: d, buttons: buttons.into_iter().map(|(_, k)| k).collect(), axes, hat, skipped })
}

/// The hat switch value for a D-pad position: 1 is up, then clockwise; 0 is centred.
fn hat_value(x: i32, y: i32) -> u8 {
    match (x.signum(), y.signum()) {
        (0, -1) => 1,
        (1, -1) => 2,
        (1, 0) => 3,
        (1, 1) => 4,
        (0, 1) => 5,
        (-1, 1) => 6,
        (-1, 0) => 7,
        (-1, -1) => 8,
        _ => 0,
    }
}

impl Layout {
    /// One input report for the state `keys`/`abs` (code to value; missing means 0).
    pub fn report(&self, keys: &HashMap<u16, i32>, abs: &HashMap<u16, i32>) -> Vec<u8> {
        let mut out = vec![0u8; self.buttons.len().div_ceil(8)];
        for (i, k) in self.buttons.iter().enumerate() {
            if keys.get(k).is_some_and(|v| *v != 0) {
                out[i / 8] |= 1 << (i % 8);
            }
        }
        for f in &self.axes {
            // An axis that has not moved rests in the middle of its range, as a stick does.
            let rest = if f.axis.min < 0 { 0 } else { f.axis.min };
            let v = abs.get(&f.axis.code).copied().unwrap_or(rest).clamp(f.axis.min, f.axis.max);
            out.extend(&v.to_le_bytes()[..usize::from(f.bits / 8)]);
        }
        if self.hat {
            let v = |code| abs.get(&code).copied().unwrap_or(0);
            out.push(hat_value(v(ABS_HAT0X), v(ABS_HAT0Y)));
        }
        out
    }
}

/// Size of the kernel's `struct uhid_event` (packed): a type, then the largest payload.
const UHID_EVENT_SIZE: usize = 4376;
const UHID_CREATE2: u32 = 11;
const UHID_INPUT2: u32 = 12;

struct UhidPad {
    file: File,
    layout: Layout,
    keys: HashMap<u16, i32>,
    abs: HashMap<u16, i32>,
}

impl UhidPad {
    fn create(spec: &DeviceSpec, uniq: Option<&str>) -> Result<Self> {
        let layout = layout(spec)?;
        if layout.descriptor.len() > 4096 {
            bail!("the report descriptor for {} is too big", spec.name);
        }
        let mut file = std::fs::OpenOptions::new().read(true).write(true).open("/dev/uhid").context("opening /dev/uhid (needs root or the uhid device)")?;
        let mut event = vec![0u8; UHID_EVENT_SIZE];
        event[0..4].copy_from_slice(&UHID_CREATE2.to_le_bytes());
        // The packed `uhid_create2_req` follows the type: name[128], phys[64], uniq[64], then the numbers.
        let put = |event: &mut Vec<u8>, at: usize, max: usize, text: &str| {
            let n = text.len().min(max - 1);
            event[at..at + n].copy_from_slice(&text.as_bytes()[..n]);
        };
        put(&mut event, 4, 128, &spec.name);
        put(&mut event, 132, 64, &spec.phys);
        put(&mut event, 196, 64, uniq.unwrap_or(&spec.uniq));
        event[260..262].copy_from_slice(&(layout.descriptor.len() as u16).to_le_bytes());
        event[262..264].copy_from_slice(&spec.bus.to_le_bytes());
        event[264..268].copy_from_slice(&u32::from(spec.vendor).to_le_bytes());
        event[268..272].copy_from_slice(&u32::from(spec.product).to_le_bytes());
        event[272..276].copy_from_slice(&u32::from(spec.version).to_le_bytes());
        event[280..280 + layout.descriptor.len()].copy_from_slice(&layout.descriptor);
        file.write_all(&event).context("creating the uhid device")?;
        Ok(Self { file, layout, keys: HashMap::new(), abs: HashMap::new() })
    }

    fn send(&mut self, frame: &[Event]) -> Result<()> {
        for e in frame {
            match e.kind {
                EV_KEY => drop(self.keys.insert(e.code, e.value)),
                EV_ABS => drop(self.abs.insert(e.code, e.value)),
                _ => {}
            }
        }
        let report = self.layout.report(&self.keys, &self.abs);
        let mut event = vec![0u8; UHID_EVENT_SIZE];
        event[0..4].copy_from_slice(&UHID_INPUT2.to_le_bytes());
        event[4..6].copy_from_slice(&(report.len() as u16).to_le_bytes());
        event[6..6 + report.len()].copy_from_slice(&report);
        self.file.write_all(&event).context("writing a uhid report")
    }
}

/// The recording's gamepads as uhid devices. Other devices are skipped, with a note on stderr.
pub struct UhidSink {
    pads: Vec<Option<UhidPad>>,
}

impl UhidSink {
    #[expect(clippy::print_stderr, reason = "CLI notes")]
    pub fn new(devices: &[DeviceSpec], uniq: Option<&str>) -> Result<Self> {
        let mut pads = Vec::new();
        for spec in devices {
            match UhidPad::create(spec, uniq) {
                Ok(pad) => {
                    if !pad.layout.skipped.is_empty() {
                        eprintln!("{}: not replayed through uhid: {}", spec.name, pad.layout.skipped.join(", "));
                    }
                    pads.push(Some(pad));
                }
                Err(e) if spec.keys.contains(&BTN_SOUTH) => return Err(e),
                Err(e) => {
                    eprintln!("skipping {}: {e}", spec.name);
                    pads.push(None);
                }
            }
        }
        if pads.iter().all(Option::is_none) {
            bail!("no gamepad in the recording to replay through uhid");
        }
        Ok(Self { pads })
    }
}

impl Sink for UhidSink {
    fn emit(&mut self, device: usize, frame: &[Event]) -> Result<()> {
        match &mut self.pads[device] {
            Some(pad) => pad.send(frame),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad() -> DeviceSpec {
        let axis = |code, min, max| AbsSpec { code, min, max, fuzz: 0, flat: 0, res: 0 };
        DeviceSpec {
            name: "Pad".into(),
            keys: vec![0x130, 0x131, 0x136, 0x224],
            abs: vec![axis(0, -32768, 32767), axis(1, -32768, 32767), axis(2, 0, 255), axis(16, -1, 1), axis(17, -1, 1), axis(40, 0, 3)],
            ..DeviceSpec::default()
        }
    }

    #[test]
    fn the_layout_covers_what_hid_can_express_and_names_the_rest() {
        let l = layout(&pad()).unwrap();
        assert_eq!(l.buttons, vec![0x130, 0x131, 0x136]);
        assert_eq!(l.axes.iter().map(|f| (f.axis.code, f.bits)).collect::<Vec<_>>(), vec![(0, 16), (1, 16), (2, 8)]);
        assert!(l.hat);
        assert_eq!(l.skipped, vec!["key 548", "axis 40"]);
        assert_eq!(&l.descriptor[..6], [0x05, 0x01, 0x09, 0x05, 0xa1, 0x01]);
        assert_eq!(l.descriptor.last(), Some(&0xc0));
    }

    #[test]
    fn a_report_packs_buttons_axes_and_the_hat() {
        let l = layout(&pad()).unwrap();
        let keys = HashMap::from([(0x131, 1), (0x136, 1), (0x130, 0)]);
        let abs = HashMap::from([(0, -2), (2, 200), (16, 1), (17, -1)]);
        // Buttons 2 and 3 (bits 1 and 2), X = -2, Y rests at 0, the trigger 200, the hat up-right.
        assert_eq!(l.report(&keys, &abs), vec![0b110, 0xfe, 0xff, 0, 0, 200, 2]);
        assert_eq!(l.report(&HashMap::new(), &HashMap::new()), vec![0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn something_that_is_not_a_gamepad_is_refused() {
        assert!(layout(&DeviceSpec { name: "Motion".into(), ..DeviceSpec::default() }).is_err());
    }

    #[test]
    fn the_hat_goes_clockwise_from_up() {
        let all: Vec<u8> = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, 0)].iter().map(|&(x, y)| hat_value(x, y)).collect();
        assert_eq!(all, vec![1, 2, 3, 4, 5, 6, 7, 8, 0]);
    }
}
