//! Profiling only: the input path's timing log. When `PADWIGHT_INPUT_TIMING` names a file, each
//! batch of controller input is written to it as it passes each stage, so the daemon's share of
//! the latency can be split up (tests/profile reads it):
//!
//! - `read <kernel ns> <read ns>`: the reader thread has the batch (its newest event's kernel
//!   time stamp, and when it was read).
//! - `input <kernel ns> <received ns> <sent ns> <done ns>`: the main loop took the batch, wrote
//!   what it mapped to the virtual devices, and finished the bookkeeping after it.
//! - `scan <at ns> <step>=<ns> ...`: how long each step of the periodic scan took; input waits
//!   while it runs.
//!
//! Times are nanoseconds since the Unix epoch, the clock evdev stamps events with. Unset, this
//! costs one check of a flag.

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

fn log() -> Option<&'static Mutex<BufWriter<File>>> {
    static LOG: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();
    LOG.get_or_init(|| {
        let path = std::env::var_os("PADWIGHT_INPUT_TIMING")?;
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path).ok()?;
        Some(Mutex::new(BufWriter::new(file)))
    })
    .as_ref()
}

/// Whether the log is on.
pub(super) fn enabled() -> bool {
    log().is_some()
}

/// Now, in nanoseconds since the Unix epoch.
pub(super) fn now_ns() -> u64 {
    ns(SystemTime::now())
}

pub(super) fn ns(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64)
}

/// Appends one line. Take the times before calling: the write itself takes a while.
pub(super) fn write(line: std::fmt::Arguments) {
    let Some(log) = log() else { return };
    let mut log = log.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ = log.write_fmt(line);
    let _ = log.write_all(b"\n");
    let _ = log.flush();
}

/// Times the steps of a piece of work for one log line, when the log is on.
pub(super) struct Steps {
    what: &'static str,
    started: Option<(u64, std::time::Instant)>,
    line: String,
}

impl Steps {
    pub(super) fn new(what: &'static str) -> Self {
        Steps { what, started: enabled().then(|| (now_ns(), std::time::Instant::now())), line: String::new() }
    }

    /// The step `name` ended now.
    pub(super) fn done(&mut self, name: &str) {
        if let Some((_, last)) = &mut self.started {
            let now = std::time::Instant::now();
            self.line.push_str(&format!(" {name}={}", now.duration_since(*last).as_nanos()));
            *last = now;
        }
    }
}

impl Drop for Steps {
    fn drop(&mut self) {
        if let Some((at, _)) = self.started {
            write(format_args!("{} {at}{}", self.what, self.line));
        }
    }
}
