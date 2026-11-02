//! The recording file: plain text, one fact per line, written and read by [`write`] and [`parse`].
//!
//! ```text
//! padwight-recording 1
//! device 0 "Xbox Controller" bus=3 vendor=0x045e product=0x028e version=0x0110 uniq="" phys=""
//!   keys 304 305 307
//!   abs 0 min=-32768 max=32767 fuzz=16 flat=128 res=0
//! events
//! 0.012345 0 3 0 1500   # ABS_X
//! ```
//!
//! An event line is the time in seconds, the device number, then the kernel's type, code and
//! value. Everything after `#` is a comment, so the names are for the reader only. A device's
//! capability lines are `props`, `keys`, `rel`, `msc`, `sw`, `led` and `ff` (a list of codes
//! each) and `abs` (one per axis).

use std::fmt::Write;

use anyhow::{Context, Result, bail};

use super::model::{AbsSpec, DeviceSpec, EV_ABS, EV_KEY, EV_MSC, EV_REL, EV_SYN, Event, Recording};

const MAGIC: &str = "padwight-recording 1";

pub fn write(rec: &Recording) -> String {
    let mut out = format!("{MAGIC}\n");
    for (n, d) in rec.devices.iter().enumerate() {
        let _ = writeln!(
            out,
            "device {n} {} bus={} vendor={:#06x} product={:#06x} version={:#06x} uniq={} phys={}",
            quote(&d.name),
            d.bus,
            d.vendor,
            d.product,
            d.version,
            quote(&d.uniq),
            quote(&d.phys)
        );
        for (word, codes) in lists(d) {
            if !codes.is_empty() {
                let _ = writeln!(out, "  {word} {}", codes.iter().map(u16::to_string).collect::<Vec<_>>().join(" "));
            }
        }
        for a in &d.abs {
            let _ = writeln!(out, "  abs {} min={} max={} fuzz={} flat={} res={}", a.code, a.min, a.max, a.fuzz, a.flat, a.res);
        }
    }
    out.push_str("events\n");
    for e in &rec.events {
        let _ = writeln!(out, "{}.{:06} {} {} {} {}   # {}", e.micros / 1_000_000, e.micros % 1_000_000, e.device, e.kind, e.code, e.value, name(e));
    }
    out
}

fn lists(d: &DeviceSpec) -> [(&'static str, &Vec<u16>); 7] {
    [("props", &d.props), ("keys", &d.keys), ("rel", &d.rel), ("msc", &d.msc), ("sw", &d.sw), ("led", &d.led), ("ff", &d.ff)]
}

fn lists_mut(d: &mut DeviceSpec) -> [(&'static str, &mut Vec<u16>); 7] {
    [("props", &mut d.props), ("keys", &mut d.keys), ("rel", &mut d.rel), ("msc", &mut d.msc), ("sw", &mut d.sw), ("led", &mut d.led), ("ff", &mut d.ff)]
}

/// The kernel's name for an event's code, for the comment on its line.
fn name(e: &Event) -> String {
    match e.kind {
        EV_SYN => format!("{:?}", evdev::SynchronizationCode(e.code)),
        EV_KEY => format!("{:?}", evdev::KeyCode(e.code)),
        EV_REL => format!("{:?}", evdev::RelativeAxisCode(e.code)),
        EV_ABS => format!("{:?}", evdev::AbsoluteAxisCode(e.code)),
        EV_MSC => format!("{:?}", evdev::MiscCode(e.code)),
        _ => format!("type {}", e.kind),
    }
}

fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars().filter(|c| !c.is_control()) {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Splits a line into words; a word may be `"quoted text"` or `key="quoted text"`.
fn words(line: &str) -> Result<Vec<String>> {
    let (mut out, mut cur, mut quoted, mut escaped, mut open) = (Vec::new(), String::new(), false, false, false);
    for c in line.chars() {
        match c {
            _ if escaped => {
                cur.push(c);
                escaped = false;
            }
            '\\' if quoted => escaped = true,
            '"' => {
                quoted = !quoted;
                open = true;
            }
            c if c.is_whitespace() && !quoted => {
                if open || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    open = false;
                }
            }
            c => cur.push(c),
        }
    }
    if quoted {
        bail!("a quote is not closed");
    }
    if open || !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

/// A decimal or `0x` hexadecimal number.
pub fn number(text: &str) -> Result<i64> {
    let (negative, digits) = text.strip_prefix('-').map_or((false, text), |rest| (true, rest));
    let value = match digits.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16),
        None => digits.parse(),
    }
    .with_context(|| format!("{text:?} is not a number"))?;
    Ok(if negative { -value } else { value })
}

pub fn small(text: &str) -> Result<u16> {
    u16::try_from(number(text)?).with_context(|| format!("{text} does not fit 16 bits"))
}

/// The value of a `key=value` word.
fn field<'a>(words: &'a [String], key: &str) -> Result<&'a str> {
    words.iter().find_map(|w| w.strip_prefix(key)?.strip_prefix('=')).with_context(|| format!("missing {key}="))
}

fn time_micros(text: &str) -> Result<u64> {
    let (secs, frac) = text.split_once('.').unwrap_or((text, ""));
    let frac = format!("{frac:0<6}");
    let micros = frac.get(..6).context("bad time")?;
    Ok(secs.parse::<u64>().context("bad time")? * 1_000_000 + micros.parse::<u64>().context("bad time")?)
}

pub fn parse(text: &str) -> Result<Recording> {
    let mut lines = text.lines().enumerate();
    if lines.next().map(|(_, l)| l.trim()) != Some(MAGIC) {
        bail!("not a padwight recording (the first line should be {MAGIC:?})");
    }
    let mut rec = Recording::default();
    let mut in_events = false;
    for (n, line) in lines {
        let line = line.split('#').next().unwrap_or_default();
        if line.trim().is_empty() {
            continue;
        }
        let at = || format!("line {}", n + 1);
        if in_events {
            let w: Vec<&str> = line.split_whitespace().collect();
            let [t, dev, kind, code, value] = w[..] else { bail!("{}: an event is time, device, type, code, value", at()) };
            let event = Event {
                micros: time_micros(t).with_context(at)?,
                device: dev.parse().with_context(at)?,
                kind: small(kind).with_context(at)?,
                code: small(code).with_context(at)?,
                value: i32::try_from(number(value).with_context(at)?).with_context(at)?,
            };
            if event.device >= rec.devices.len() {
                bail!("{}: there is no device {}", at(), event.device);
            }
            rec.events.push(event);
        } else if line.trim() == "events" {
            in_events = true;
        } else {
            device_line(&mut rec, &words(line).with_context(at)?).with_context(at)?;
        }
    }
    if !in_events {
        bail!("no `events` line");
    }
    Ok(rec)
}

fn device_line(rec: &mut Recording, w: &[String]) -> Result<()> {
    match w.first().map(String::as_str) {
        Some("device") => {
            if w.get(1).map(|n| n.parse::<usize>()) != Some(Ok(rec.devices.len())) {
                bail!("devices are numbered 0, 1, ... in order");
            }
            rec.devices.push(DeviceSpec {
                name: w.get(2).context("the device needs a name")?.clone(),
                bus: small(field(w, "bus")?)?,
                vendor: small(field(w, "vendor")?)?,
                product: small(field(w, "product")?)?,
                version: small(field(w, "version")?)?,
                uniq: field(w, "uniq").unwrap_or_default().to_string(),
                phys: field(w, "phys").unwrap_or_default().to_string(),
                ..DeviceSpec::default()
            });
        }
        Some("abs") => {
            let dev = rec.devices.last_mut().context("`abs` before any device")?;
            let int = |key| field(w, key).and_then(number).and_then(|v| i32::try_from(v).map_err(Into::into));
            dev.abs.push(AbsSpec { code: small(w.get(1).context("abs needs a code")?)?, min: int("min")?, max: int("max")?, fuzz: int("fuzz")?, flat: int("flat")?, res: int("res")? });
        }
        Some(word) => {
            let dev = rec.devices.last_mut().context("a capability before any device")?;
            let Some((_, list)) = lists_mut(dev).into_iter().find(|(name, _)| *name == word) else { bail!("unknown line {word:?}") };
            *list = w[1..].iter().map(|c| small(c)).collect::<Result<_>>()?;
        }
        None => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
