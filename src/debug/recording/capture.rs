//! `padwight debug record`: reading a controller's evdev nodes without grabbing them.

use std::{
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use evdev::Device;

use super::model::{AbsSpec, DeviceSpec, Event, Recording, SYN_DROPPED};
use crate::{daemon, input, output::VIRTUAL_PREFIX};

/// What `record` was asked for.
pub struct Options {
    /// An event node (`/dev/input/event7`) or part of a controller's name; none picks the only
    /// controller there is.
    pub device: Option<String>,
    /// Stop after this long; none records until Ctrl+C.
    pub seconds: Option<f64>,
}

/// Everything the kernel says a device can do.
pub fn spec_of(dev: &Device) -> DeviceSpec {
    let id = dev.input_id();
    let abs_state = dev.get_abs_state().ok();
    DeviceSpec {
        name: dev.name().unwrap_or_default().to_string(),
        bus: id.bus_type().0,
        vendor: id.vendor(),
        product: id.product(),
        version: id.version(),
        uniq: dev.unique_name().unwrap_or_default().to_string(),
        phys: dev.physical_path().unwrap_or_default().to_string(),
        props: dev.properties().iter().map(|p| p.0).collect(),
        keys: dev.supported_keys().map(|s| s.iter().map(|c| c.0).collect()).unwrap_or_default(),
        rel: dev.supported_relative_axes().map(|s| s.iter().map(|c| c.0).collect()).unwrap_or_default(),
        abs: dev
            .supported_absolute_axes()
            .map(|s| {
                s.iter()
                    .filter_map(|axis| {
                        let info = abs_state.as_ref()?.get(usize::from(axis.0))?;
                        Some(AbsSpec { code: axis.0, min: info.minimum, max: info.maximum, fuzz: info.fuzz, flat: info.flat, res: info.resolution })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        msc: dev.misc_properties().map(|s| s.iter().map(|c| c.0).collect()).unwrap_or_default(),
        sw: dev.supported_switches().map(|s| s.iter().map(|c| c.0).collect()).unwrap_or_default(),
        led: dev.supported_leds().map(|s| s.iter().map(|c| c.0).collect()).unwrap_or_default(),
        ff: dev.supported_ff().map(|s| s.iter().map(|c| c.0).collect()).unwrap_or_default(),
    }
}

/// Whether the node is a device of a uinput program (Steam Input, another remapper, this one).
fn is_virtual(path: &Path) -> bool {
    let Some(node) = path.file_name() else { return false };
    std::fs::canonicalize(Path::new("/sys/class/input").join(node).join("device")).is_ok_and(|p| p.to_string_lossy().contains("/devices/virtual/input/"))
}

fn event_nodes() -> Vec<PathBuf> {
    let mut nodes: Vec<PathBuf> = std::fs::read_dir("/dev/input").into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("event"))).collect();
    nodes.sort_by_key(|p| p.file_name().and_then(|n| n.to_str()).and_then(|n| n[5..].parse::<u32>().ok()));
    nodes
}

/// The controller to record: the gamepad node, and the motion-sensor and touchpad nodes the daemon
/// would pair with it.
fn pick(device: Option<&str>) -> Result<Vec<(PathBuf, Device)>> {
    let mut opened: Vec<(PathBuf, Device)> = event_nodes().into_iter().filter_map(|p| Device::open(&p).ok().map(|d| (p, d))).collect();
    let pads: Vec<usize> = (0..opened.len())
        .filter(|&i| {
            let (path, dev) = &opened[i];
            let name = dev.name().unwrap_or_default();
            let wanted = match device {
                Some(want) => path.to_string_lossy() == want || name.to_lowercase().contains(&want.to_lowercase()),
                None => !is_virtual(path) && !name.starts_with(VIRTUAL_PREFIX),
            };
            wanted && input::is_gamepad(dev)
        })
        .collect();
    let list = || pads.iter().map(|&i| format!("  {}  {}", opened[i].0.display(), opened[i].1.name().unwrap_or_default())).collect::<Vec<_>>().join("\n");
    let pad = match pads[..] {
        [] if opened.is_empty() => bail!("no input devices could be opened (is /dev/input readable? the `input` group)"),
        [] => bail!("no matching controller found"),
        [one] => one,
        _ => bail!("several controllers found; pick one with --device <node or name>:\n{}", list()),
    };
    let (pad_path, pad_dev) = &opened[pad];
    let (pad_name, pad_uniq) = (pad_dev.name().unwrap_or_default().to_string(), pad_dev.unique_name().map(str::to_string));
    let mut chosen = vec![pad];
    for (i, (path, dev)) in opened.iter().enumerate() {
        let sibling = input::is_touchpad(dev) || input::is_motion_sensor(dev);
        if i != pad && sibling && daemon::same_controller((path, dev.name().unwrap_or_default(), dev.unique_name()), (pad_path, &pad_name, pad_uniq.as_deref())) {
            chosen.push(i);
        }
    }
    // Take the chosen ones out of the list, keeping the gamepad first.
    let mut picked = Vec::new();
    for (i, entry) in opened.drain(..).enumerate() {
        if let Some(pos) = chosen.iter().position(|&c| c == i) {
            picked.push((pos, entry));
        }
    }
    picked.sort_by_key(|(pos, _)| *pos);
    Ok(picked.into_iter().map(|(_, entry)| entry).collect())
}

fn micros_of(ev: &evdev::InputEvent) -> u64 {
    ev.timestamp().duration_since(UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

/// Reads the controller until told to stop, and returns what it said.
#[expect(clippy::print_stderr, reason = "CLI progress")]
pub fn record(opts: &Options) -> Result<Recording> {
    let mut devs = pick(opts.device.as_deref())?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    daemon::stop_on_signals(move || {
        flag.store(true, Ordering::Relaxed);
        true
    })?;
    let mut rec = Recording::default();
    for (path, dev) in &devs {
        eprintln!("recording {} ({}); Ctrl+C ends it", dev.name().unwrap_or_default(), path.display());
        rec.devices.push(spec_of(dev));
        dev.set_nonblocking(true).context("making a device non-blocking")?;
    }
    let deadline = opts.seconds.map(|s| Instant::now() + Duration::from_secs_f64(s));
    let mut raw: Vec<(u64, Event)> = Vec::new();
    while !stop.load(Ordering::Relaxed) && deadline.is_none_or(|d| Instant::now() < d) {
        let mut fds: Vec<libc::pollfd> = devs.iter().map(|(_, d)| libc::pollfd { fd: d.as_raw_fd(), events: libc::POLLIN, revents: 0 }).collect();
        // SAFETY: `fds` is a valid array of the length given.
        unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 100) };
        for (n, (_, dev)) in devs.iter_mut().enumerate() {
            loop {
                let events = match dev.fetch_events() {
                    Ok(events) => events.collect::<Vec<_>>(),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => bail!("reading {}: {e}", rec.devices[n].name),
                };
                raw.extend(events.iter().filter(|e| !(e.event_type().0 == 0 && e.code() == SYN_DROPPED)).map(|e| (micros_of(e), Event { micros: 0, device: n, kind: e.event_type().0, code: e.code(), value: e.value() })));
            }
        }
    }
    // Devices were read one after another, so put the events in the order they happened.
    raw.sort_by_key(|(t, _)| *t);
    let start = raw.first().map_or(0, |(t, _)| *t);
    rec.events = raw.into_iter().map(|(t, e)| Event { micros: t - start, ..e }).collect();
    eprintln!("recorded {} events from {} devices", rec.events.len(), rec.devices.len());
    Ok(rec)
}
