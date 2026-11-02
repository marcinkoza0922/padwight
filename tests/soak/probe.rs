//! What the soak run reads from outside the daemon: its IPC socket, /proc, and the kernel's key
//! state on the daemon's virtual devices. Also the invariant check run every few minutes.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::Path,
    process::Child,
    thread,
    time::{Duration, Instant},
};

use evdev::Device;

use crate::steps::World;

/// How far above its start values the daemon may grow.
#[derive(Clone, Copy)]
pub struct Bounds {
    pub rss_kb: u64,
    pub fds: u64,
    pub threads: u64,
}

/// The daemon's counters at one moment.
#[derive(Clone, Copy)]
pub struct Sample {
    pub rss_kb: u64,
    pub fds: u64,
    pub threads: u64,
    /// `padwight overlay` processes it has running.
    pub overlays: usize,
}

impl Sample {
    /// None once the process has gone.
    pub fn take(pid: u32) -> Option<Self> {
        let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let field = |key: &str| status.lines().find_map(|l| l.strip_prefix(key)).and_then(|r| r.split_whitespace().next()?.parse::<u64>().ok());
        Some(Sample {
            rss_kb: field("VmRSS:")?,
            threads: field("Threads:")?,
            fds: fs::read_dir(format!("/proc/{pid}/fd")).ok()?.count() as u64,
            overlays: overlay_pids(pid).len(),
        })
    }
}

/// Sends one request line and returns the reply line, or None when the daemon doesn't answer.
pub fn request(runtime: &Path, line: &str) -> Option<String> {
    let mut stream = UnixStream::connect(runtime.join("padwight.sock")).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(4))).ok()?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).ok()?;
    stream.write_all(format!("{line}\n").as_bytes()).ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    (!reply.is_empty()).then_some(reply)
}

/// The `padwight overlay` processes that are children of the daemon `pid`.
pub fn overlay_pids(pid: u32) -> Vec<u32> {
    let Ok(entries) = fs::read_dir("/proc") else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&other| {
            let Ok(stat) = fs::read_to_string(format!("/proc/{other}/stat")) else { return false };
            let after = stat.rfind(')').map_or("", |i| &stat[i + 2..]);
            let mut fields = after.split_whitespace();
            let (state, ppid) = (fields.next(), fields.next().and_then(|p| p.parse::<u32>().ok()));
            let cmdline = fs::read(format!("/proc/{other}/cmdline")).unwrap_or_default();
            state != Some("Z") && ppid == Some(pid) && cmdline.split(|&b| b == 0).nth(1) == Some(b"overlay")
        })
        .collect()
}

/// Keys and buttons the kernel shows as held on the daemon's virtual devices, as `device: KEY`.
pub fn kernel_held() -> Vec<String> {
    let Ok(entries) = fs::read_dir("/dev/input") else { return Vec::new() };
    let mut held = Vec::new();
    for entry in entries.flatten() {
        let Ok(dev) = Device::open(entry.path()) else { continue };
        let Some(name) = dev.name().filter(|n| n.starts_with("Padwight Virtual")) else { continue };
        if let Ok(state) = dev.get_key_state() {
            held.extend(state.iter().map(|key| format!("{name}: {key:?}")));
        }
    }
    held
}

/// Releases everything, then checks the invariants. Returns the daemon's counters, or why it failed.
pub fn check_invariants(world: &mut World, daemon: &mut Child, start: Sample, bounds: Bounds) -> Result<Sample, String> {
    if let Some(status) = daemon.try_wait().map_err(|e| e.to_string())? {
        return Err(format!("the daemon exited ({status})"));
    }
    world.release_everything();
    // Held keys must come up within a few seconds of everything being let go.
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut answered = false;
    let held = loop {
        answered = answered || request(&world.runtime, "\"Status\"").is_some();
        let held = kernel_held();
        if (held.is_empty() && answered) || Instant::now() >= deadline {
            break held;
        }
        thread::sleep(Duration::from_millis(100));
    };
    if !answered {
        return Err("the daemon does not answer on its socket".into());
    }
    if !held.is_empty() {
        return Err(format!("still held in the kernel after releasing everything: {}", held.join(", ")));
    }
    let log = world.log();
    if let Some(line) = log.lines().find(|l| l.contains("panicked") || l.contains("output error")) {
        return Err(format!("the daemon log has a problem: {line}"));
    }
    let now = Sample::take(world.pid).ok_or("the daemon is gone")?;
    if now.rss_kb > start.rss_kb + bounds.rss_kb {
        return Err(format!("resident memory grew from {} kB to {} kB (bound +{})", start.rss_kb, now.rss_kb, bounds.rss_kb));
    }
    if now.fds > start.fds + bounds.fds {
        return Err(format!("open descriptors grew from {} to {} (bound +{})", start.fds, now.fds, bounds.fds));
    }
    if now.threads > start.threads + bounds.threads {
        return Err(format!("threads grew from {} to {} (bound +{})", start.threads, now.threads, bounds.threads));
    }
    if now.overlays > 1 {
        return Err(format!("{} overlay processes are running", now.overlays));
    }
    Ok(now)
}
