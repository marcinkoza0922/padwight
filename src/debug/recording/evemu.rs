//! Reading the files `evemu-record` writes (version 1.x), so recordings made elsewhere replay too.
//!
//! The lines used: `N:` the name, `I:` bus, vendor, product and version (hex), `P:` the
//! properties and `B:` a capability bitmap per event type (hex bytes, the lowest bit first), `A:` an
//! axis (code, min, max, fuzz, flat, resolution), and `E:` an event (`sec.usec type code value`,
//! type and code in hex). Other lines and `#` comments are ignored. The file holds one device.

use anyhow::{Context, Result, bail};

use super::model::{AbsSpec, DeviceSpec, EV_ABS, EV_KEY, EV_MSC, EV_REL, Event, Recording};

pub fn looks_like_evemu(text: &str) -> bool {
    text.lines().take(5).any(|l| l.starts_with("# EVEMU") || l.starts_with("N: ") || l.starts_with("I: "))
}

/// The positions of the set bits in `B:`/`P:` bytes written as hex.
fn bits(bytes: &[&str]) -> Result<Vec<u16>> {
    let mut out = Vec::new();
    for (i, b) in bytes.iter().enumerate() {
        let byte = u8::from_str_radix(b, 16).with_context(|| format!("{b:?} is not a hex byte"))?;
        out.extend((0..8).filter(|bit| byte >> bit & 1 == 1).map(|bit| (i * 8 + bit) as u16));
    }
    Ok(out)
}

pub fn parse(text: &str) -> Result<Recording> {
    let mut dev = DeviceSpec::default();
    let mut raw: Vec<(u64, u16, u16, i32)> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = || format!("line {}", n + 1);
        let Some((tag, rest)) = line.split_once(": ").or_else(|| line.split_once(':')) else { continue };
        let w: Vec<&str> = rest.split_whitespace().collect();
        match tag {
            "N" => dev.name = rest.trim().to_string(),
            "I" => {
                let [bus, vendor, product, version] = w[..] else { bail!("{}: I: needs four numbers", at()) };
                let hex = |t: &str| u16::from_str_radix(t, 16).with_context(at);
                (dev.bus, dev.vendor, dev.product, dev.version) = (hex(bus)?, hex(vendor)?, hex(product)?, hex(version)?);
            }
            "P" => dev.props = bits(&w).with_context(at)?,
            "B" => {
                let (kind, bytes) = w.split_first().with_context(|| format!("{}: B: needs a type", at()))?;
                let list = bits(bytes).with_context(at)?;
                match u16::from_str_radix(kind, 16).with_context(at)? {
                    EV_KEY => dev.keys = list,
                    EV_REL => dev.rel = list,
                    EV_MSC => dev.msc = list,
                    5 => dev.sw = list,
                    17 => dev.led = list,
                    21 => dev.ff = list,
                    _ => {}
                }
            }
            "A" => {
                let [code, min, max, fuzz, flat, res] = w[..] else { bail!("{}: A: needs six numbers", at()) };
                let n = |t: &str| t.parse::<i32>().with_context(at);
                let code = u16::from_str_radix(code, 16).with_context(at)?;
                dev.abs.push(AbsSpec { code, min: n(min)?, max: n(max)?, fuzz: n(fuzz)?, flat: n(flat)?, res: n(res)? });
            }
            "E" => {
                let [time, kind, code, value] = w[..] else { bail!("{}: E: needs time, type, code, value", at()) };
                let (sec, usec) = time.split_once('.').context("a time like 12.345678").with_context(at)?;
                let micros = sec.parse::<u64>().with_context(at)? * 1_000_000 + usec.parse::<u64>().with_context(at)?;
                let kind = u16::from_str_radix(kind, 16).with_context(at)?;
                raw.push((micros, kind, u16::from_str_radix(code, 16).with_context(at)?, value.parse().with_context(at)?));
            }
            _ => {}
        }
    }
    if dev.name.is_empty() && raw.is_empty() {
        bail!("no evemu device or events found");
    }
    if dev.abs.iter().all(|a| a.code != 0) && raw.iter().any(|e| e.1 == EV_ABS) {
        // A recording of events alone (no device header) can still be replayed; the axes get the usual stick range.
        let mut seen: Vec<u16> = raw.iter().filter(|e| e.1 == EV_ABS).map(|e| e.2).collect();
        seen.sort_unstable();
        seen.dedup();
        let known: Vec<u16> = dev.abs.iter().map(|a| a.code).collect();
        dev.abs.extend(seen.into_iter().filter(|c| !known.contains(c)).map(|code| AbsSpec { code, min: -32768, max: 32767, fuzz: 0, flat: 0, res: 0 }));
    }
    let start = raw.first().map_or(0, |e| e.0);
    let events = raw.into_iter().map(|(t, kind, code, value)| Event { micros: t - start, device: 0, kind, code, value }).collect();
    if dev.name.is_empty() {
        dev.name = "evemu device".into();
    }
    Ok(Recording { devices: vec![dev], events })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        let keys = format!("{}03", "00 ".repeat(38));
        "# EVEMU 1.3\n# Kernel: 6.1\nN: Test Pad\nI: 0003 045e 028e 0110\nP: 00 00 00 00 00 00 00 00\nB: 00 0b 00 00 00 00 00 00 00\nB: 01 {KEYS}\nA: 00 -32768 32767 16 128 0\nA: 01 -32768 32767 16 128 0\nE: 100.000000 0001 0130 1\nE: 100.000000 0000 0000 0\nE: 100.250000 0003 0000 -500\n".replace("{KEYS}", &keys)
    }

    #[test]
    fn a_device_header_and_events_are_read() {
        assert!(looks_like_evemu(&sample()));
        let rec = parse(&sample()).unwrap();
        let d = &rec.devices[0];
        assert_eq!((d.name.as_str(), d.bus, d.vendor, d.product, d.version), ("Test Pad", 3, 0x45e, 0x28e, 0x110));
        assert_eq!(d.keys, vec![0x130, 0x131], "BTN_SOUTH and BTN_EAST");
        assert_eq!(d.abs.len(), 2);
        assert_eq!(d.abs[0], AbsSpec { code: 0, min: -32768, max: 32767, fuzz: 16, flat: 128, res: 0 });
        assert_eq!(rec.events.len(), 3);
        assert_eq!(rec.events[2], Event { micros: 250_000, device: 0, kind: 3, code: 0, value: -500 });
    }

    #[test]
    fn a_bitmap_lists_the_set_bits_lowest_first() {
        assert_eq!(bits(&["05", "80"]).unwrap(), vec![0, 2, 15]);
    }

    #[test]
    fn rubbish_is_refused() {
        assert!(parse("hello\nworld\n").is_err());
        assert!(parse("N: X\nE: 1.0 0001 zz 1\n").is_err());
    }
}
