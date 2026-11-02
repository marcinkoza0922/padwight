use super::*;
use crate::debug::recording::model::{EV_ABS, EV_KEY, SYN_REPORT};

fn sample() -> Recording {
    let pad = DeviceSpec {
        name: "Pad \"One\" \\ test".into(),
        bus: 3,
        vendor: 0x045e,
        product: 0x028e,
        version: 0x110,
        uniq: "aa:bb".into(),
        keys: vec![304, 305],
        abs: vec![AbsSpec { code: 0, min: -32768, max: 32767, fuzz: 16, flat: 128, res: 0 }, AbsSpec { code: 2, min: 0, max: 255, fuzz: 0, flat: 0, res: 0 }],
        ff: vec![80],
        ..DeviceSpec::default()
    };
    let motion = DeviceSpec { name: "Pad Motion Sensors".into(), bus: 3, vendor: 0x045e, product: 0x028e, msc: vec![5], ..DeviceSpec::default() };
    let ev = |micros, device, kind, code, value| Event { micros, device, kind, code, value };
    Recording {
        devices: vec![pad, motion],
        events: vec![ev(0, 0, EV_KEY, 304, 1), ev(0, 0, 0, SYN_REPORT, 0), ev(12_345, 0, EV_ABS, 0, -1500), ev(1_000_001, 1, 4, 5, 77)],
    }
}

#[test]
fn a_recording_survives_a_write_and_a_parse() {
    let rec = sample();
    let text = write(&rec);
    assert_eq!(parse(&text).unwrap(), rec, "{text}");
}

#[test]
fn event_lines_carry_a_readable_name() {
    let text = write(&sample());
    assert!(text.contains("0.000000 0 1 304 1   # BTN_SOUTH"), "{text}");
    assert!(text.contains("0.012345 0 3 0 -1500   # ABS_X"), "{text}");
}

#[test]
fn a_file_by_hand_with_short_times_and_hex_is_read() {
    let text = "padwight-recording 1\n# a comment\ndevice 0 \"X\" bus=0x3 vendor=0x1 product=2 version=0\n  keys 0x130\nevents\n1.5 0 1 0x130 1\n";
    let rec = parse(text).unwrap();
    assert_eq!(rec.devices[0].keys, vec![0x130]);
    assert_eq!(rec.events[0].micros, 1_500_000);
}

#[test]
fn mistakes_name_their_line() {
    assert!(parse("hello").unwrap_err().to_string().contains("not a padwight recording"));
    let err = parse("padwight-recording 1\nevents\n0.0 3 1 1 1\n").unwrap_err();
    assert!(format!("{err:#}").contains("line 3") && format!("{err:#}").contains("no device 3"), "{err:#}");
    let err = parse("padwight-recording 1\ndevice 0 \"X\" bus=1 vendor=1 product=1 version=1\n  bogus 1\nevents\n").unwrap_err();
    assert!(format!("{err:#}").contains("line 3"), "{err:#}");
    assert!(parse("padwight-recording 1\n").unwrap_err().to_string().contains("events"));
}
