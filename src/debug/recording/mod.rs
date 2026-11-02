//! Recording a real controller's input and playing it back through virtual devices
//! (`padwight debug record` and `padwight debug replay`), so a bug seen on hardware becomes a
//! test that needs none.

mod capture;
mod evemu;
mod format;
mod model;
mod replay;
mod uhid;

use std::time::Duration;

use anyhow::{Context, Result, bail};

pub use model::Recording;

pub const USAGE: &str = "
Recording real input (no daemon or session needed; reads /dev/input, does not grab):
  record <file> [--device <node|name>] [--seconds <s>]
                                 write a controller's events, with their times, and its devices'
                                 names, IDs and capabilities. The gamepad and its motion-sensor
                                 and touchpad nodes are taken together. Ctrl+C ends it
  replay <file> [--uhid] [--speed <x>] [--lead-in <ms>] [--hold <ms>] [--uniq <id>]
                                 make devices like the recorded ones and play the events at their
                                 times. uinput by default; --uhid makes a HID gamepad instead, the
                                 kind the daemon grabs (a daemon ignores uinput devices). Reads
                                 the files `evemu-record` writes too
";

pub fn run_record(args: &[&str]) -> Result<()> {
    let mut it = args.iter();
    let file = it.next().filter(|a| !a.starts_with("--")).context("record needs a file to write")?;
    let mut opts = capture::Options { device: None, seconds: None };
    while let Some(arg) = it.next() {
        match *arg {
            "--device" => opts.device = Some(it.next().context("--device needs a node or a name")?.to_string()),
            "--seconds" => opts.seconds = Some(it.next().context("--seconds needs a number")?.parse().context("--seconds takes a number")?),
            other => bail!("unknown option {other}"),
        }
    }
    let rec = capture::record(&opts)?;
    std::fs::write(file, format::write(&rec)).with_context(|| format!("writing {file}"))
}

/// Reads a recording in either file format.
pub fn load(path: &str) -> Result<Recording> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    if text.starts_with("padwight-recording") {
        format::parse(&text)
    } else if evemu::looks_like_evemu(&text) {
        evemu::parse(&text).with_context(|| format!("reading {path} as an evemu recording"))
    } else {
        bail!("{path} is neither a padwight recording nor an evemu-record file")
    }
}

#[expect(clippy::print_stderr, reason = "CLI progress")]
pub fn run_replay(args: &[&str]) -> Result<()> {
    let mut it = args.iter();
    let file = it.next().filter(|a| !a.starts_with("--")).context("replay needs a recording")?;
    let (mut uhid_pad, mut speed, mut lead_in, mut hold, mut uniq) = (false, 1.0f64, 3000u64, 500u64, None);
    while let Some(arg) = it.next() {
        let mut value = |what: &str| it.next().map(ToString::to_string).with_context(|| format!("{arg} needs {what}"));
        match *arg {
            "--uhid" => uhid_pad = true,
            "--speed" => speed = value("a number")?.parse().context("--speed takes a number")?,
            "--lead-in" => lead_in = value("milliseconds")?.parse().context("--lead-in takes milliseconds")?,
            "--hold" => hold = value("milliseconds")?.parse().context("--hold takes milliseconds")?,
            "--uniq" => uniq = Some(value("an id")?),
            other => bail!("unknown option {other}"),
        }
    }
    if speed <= 0.0 {
        bail!("--speed must be above zero");
    }
    let rec = load(file)?;
    let mut sink: Box<dyn replay::Sink> = if uhid_pad { Box::new(uhid::UhidSink::new(&rec.devices, uniq.as_deref())?) } else { Box::new(replay::UinputSink::new(&rec)?) };
    eprintln!("replaying {} events from {} devices; first frame in {lead_in} ms", rec.events.len(), rec.devices.len());
    // The lead-in gives whoever watches the devices (a daemon) time to find and open them.
    std::thread::sleep(Duration::from_millis(lead_in));
    let frames = replay::play(&rec, &mut replay::RealClock::new(speed), &mut *sink)?;
    std::thread::sleep(Duration::from_millis(hold));
    eprintln!("replayed {frames} frames");
    Ok(())
}
