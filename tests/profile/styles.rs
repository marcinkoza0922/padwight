//! What overlay styles cost to draw. A stand-in daemon streams frames straight to the overlay
//! process (`padwight overlay`): a menu, the on-screen keyboard and an info sheet, each in its
//! default look, with every decoration turned up, and at its barest (the performance settings'
//! plain look and motion off). The cursor moves every [`STEP`], about as fast as
//! someone flicking through a menu, so the window keeps redrawing as the highlight glides.
//!
//! For each case it measures the overlay process's CPU, the compositor's CPU (it blends the
//! full-screen surface each frame), how many times the widgets were built and how long a build
//! took, and the GPU's busy share where `nvidia-smi` can tell. Every case runs on the GPU renderer
//! (wgpu) and on the software one (tiny-skia), which iced falls back to without a working GPU.
//!
//! Needs a Wayland session with layer-shell (not GNOME), and shows each case on the real screen
//! for a few seconds. No controller or daemon is involved. `PADWIGHT_PROFILE_CASES=radial` runs
//! only the cases whose name contains that text.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::support::{Monitor, Running, latency_stats, padwight, report};
use crate::common::{serial, temp_root};

/// How often the cursor moves.
const STEP: Duration = Duration::from_millis(150);
/// Shown this long before measuring, so start-up and the first draws don't count.
const WARM_UP: Duration = Duration::from_millis(1500);
/// Measured this long per case.
const MEASURE: Duration = Duration::from_secs(6);
const BACKENDS: [&str; 2] = ["wgpu", "tiny-skia"];
/// Each case is measured this many times; the figures are the median.
const ROUNDS: usize = 3;

#[test]
#[ignore]
fn profile_overlay_styles() {
    let _serial = serial();
    let Some(wayland) = wayland_socket() else {
        report("overlay_styles", json!({"skipped": "no Wayland display, and the overlay is a layer-shell surface"}));
        return;
    };
    let only = std::env::var("PADWIGHT_PROFILE_CASES").unwrap_or_default();
    let cases = cases();
    let chosen: Vec<(String, &Case, &str)> = BACKENDS
        .iter()
        .flat_map(|backend| cases.iter().map(move |case| (format!("{backend} · {}", case.name), case, *backend)))
        .filter(|(name, ..)| name.contains(&only))
        .collect();
    // Rounds go through every case in turn, so something else busy on the desktop for a while
    // lands on one sample of many cases rather than on every sample of one.
    let mut samples: Vec<Vec<Value>> = vec![Vec::new(); chosen.len()];
    let mut idle = Vec::new();
    for round in 0..ROUNDS {
        // What the compositor uses with nothing of ours on screen, to set the cases against.
        let window = compositor_pid().map(CpuWindow::start);
        std::thread::sleep(MEASURE);
        idle.extend(window.and_then(CpuWindow::finish));
        for ((name, case, backend), samples) in chosen.iter().zip(&mut samples) {
            eprintln!("-- round {} · {name}", round + 1);
            samples.push(run(case, backend, &wayland));
        }
    }
    let mut results = serde_json::Map::new();
    results.insert("no overlay".into(), json!({"compositor_cpu_percent": median(idle)}));
    for ((name, ..), samples) in chosen.into_iter().zip(samples) {
        results.insert(name, summarize(&samples));
    }
    report(
        "overlay_styles",
        json!({
            "what": "a frame every cursor step, streamed to the overlay process by a stand-in daemon; CPU is a share of one core, the median of the rounds",
            "step_ms": STEP.as_millis() as u64,
            "measured_s": MEASURE.as_secs(),
            "rounds": ROUNDS,
            "cases": results,
        }),
    );
}

/// The median of each figure over a case's rounds, with every round's figures beside it.
fn summarize(samples: &[Value]) -> Value {
    let figure = |key: &str| median(samples.iter().filter_map(|s| s[key].as_f64()).collect());
    json!({
        "overlay_cpu_percent": figure("overlay_cpu_percent"),
        "compositor_cpu_percent": figure("compositor_cpu_percent"),
        "gpu_busy_percent": figure("gpu_busy_percent"),
        "builds_per_second": figure("builds_per_second"),
        "build_p50_us": median(samples.iter().filter_map(|s| s["build"]["p50_us"].as_f64()).collect()),
        "rounds": samples,
    })
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied()
}

/// One thing to draw: how a frame looks at cursor step `n`.
struct Case {
    name: &'static str,
    frame: Box<dyn Fn(usize) -> Value>,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (look, style, lite, motion) in looks() {
        let (s, l, m) = (style.clone(), lite.clone(), motion.clone());
        cases.push(Case { name: leak(format!("list menu, {look}")), frame: Box::new(move |n| menu_frame(n, json!("list"), &s, &l, &m)) });
        let (s, l, m) = (style.clone(), lite.clone(), motion.clone());
        cases.push(Case { name: leak(format!("radial menu, {look}")), frame: Box::new(move |n| menu_frame(n, json!({"radial": {"stick": "Right"}}), &s, &l, &m)) });
        let (s, l, m) = (style.clone(), lite.clone(), motion.clone());
        cases.push(Case { name: leak(format!("keyboard, {look}")), frame: Box::new(move |n| keyboard_frame(n, &s, &l, &m)) });
        cases.push(Case { name: leak(format!("info sheet, {look}")), frame: Box::new(move |n| info_frame(n, &style, &lite, &motion)) });
    }
    cases
}

/// The looks each overlay is drawn in: its style, the performance settings, and its motion.
fn looks() -> Vec<(&'static str, Value, Value, Value)> {
    let decorated = json!({
        "border": {"width": 3, "color": "#c8a040"},
        "shadow": {"color": "#000000", "opacity": 0.8, "blur": 60, "y": 16},
        "gradient": {"color": "#3a1010", "angle": 135},
        "item_border": {"width": 2, "color": "#806030"},
        "glow": {"color": "#ffcc40", "opacity": 0.9, "blur": 30},
        "text": "#e8d8a8",
    });
    vec![
        ("default look", json!({}), json!({}), json!({})),
        ("decorated", decorated, json!({}), json!({})),
        ("plain", json!({}), json!({"plain": true}), json!({})),
        ("default look, motion off", json!({}), json!({}), json!({"menu": "off", "keyboard": "off", "info": "off"})),
        ("barest", json!({}), json!({"plain": true}), json!({"menu": "off", "keyboard": "off", "info": "off"})),
    ]
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// `extra` laid over a default overlay style at `position`.
fn style(position: &str, extra: &Value) -> Value {
    let mut style = json!({"position": position, "scale": 1.0});
    for (k, v) in extra.as_object().into_iter().flatten() {
        style[k] = v.clone();
    }
    style
}

fn frame(info: Value, active: Value, lite: &Value, motion: &Value) -> Value {
    let mut frame = json!({"lite": lite, "motion": motion});
    frame["info"] = info;
    frame["active"] = active;
    frame
}

/// Eight items with button glyphs, one a submenu, the cursor walking down them.
fn menu_frame(n: usize, kind: Value, extra: &Value, lite: &Value, motion: &Value) -> Value {
    let buttons = ["South", "East", "West", "North", "LeftBumper", "RightBumper", "DpadUp", "DpadDown"];
    let items: Vec<Value> = (0..8)
        .map(|i| json!({"label": format!("Item {}", i + 1), "button": null, "submenu": i == 3, "buttons": [buttons[i]], "weight": 1.0}))
        .collect();
    let mut menu = json!({
        "title": "Belt", "items": items, "selected": n % 8, "picks": 0, "crumbs": [],
        "hint": "A choose · B back", "style": style("center", extra),
    });
    menu["kind"] = kind;
    frame(json!([]), json!({"Menu": menu}), lite, motion)
}

/// The keyboard, the cursor walking along the letter rows.
fn keyboard_frame(n: usize, extra: &Value, lite: &Value, motion: &Value) -> Value {
    let keyboard = json!({
        "layout": "keyboard", "style": style("bottom_center", extra),
        "cursor": {"row": 1 + n / 10 % 3, "col": 1 + n % 10}, "latched": [], "pressed": null, "closing": 0.0,
    });
    frame(json!([]), json!({"Keyboard": keyboard}), lite, motion)
}

/// A twelve-row cheat sheet of glyphs, D-pads and icons, its D-pad arms lighting in turn as live
/// input would.
fn info_frame(n: usize, extra: &Value, lite: &Value, motion: &Value) -> Value {
    let glyph = |label: &str, fill: Value| json!({"Glyph": {"label": label, "fill": fill, "round": true}});
    let mut arms = [false; 4];
    arms[n % 4] = true;
    let rows: Vec<Value> = (0..12)
        .map(|i| {
            json!([
                [glyph("A", json!([60, 160, 60])), glyph("LB", Value::Null), {"Dpad": arms}],
                [{"Text": format!("Action {i}")}],
                [{"StickClick": {"right": i % 2 == 0}}, {"Icon": {"Battery": {"percent": 80, "estimated": false}}}],
            ])
        })
        .collect();
    let info = json!({"name": "sheet", "style": style("top_right", extra), "title": [{"Text": "Controls"}], "rows": rows, "opacity": 1.0});
    frame(json!([info]), Value::Null, lite, motion)
}

/// The compositor's socket, as a full path: `padwight()` gives the overlay a runtime directory of
/// its own, where the socket isn't.
fn wayland_socket() -> Option<PathBuf> {
    let display = std::env::var_os("WAYLAND_DISPLAY")?;
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(Path::new(&dir).join(display))
}

/// Shows `case` on `backend` and measures it.
fn run(case: &Case, backend: &str, wayland: &Path) -> Value {
    let root = temp_root();
    let runtime = root.join("run");
    let timing_log = root.join("overlay-timing.log");
    let mut command = padwight(&root, &["overlay"]);
    let listener = UnixListener::bind(runtime.join("padwight.sock")).expect("binding the stand-in daemon's socket");
    command.env("WAYLAND_DISPLAY", wayland).env("ICED_BACKEND", backend).env("PADWIGHT_OVERLAY_TIMING", &timing_log);
    let overlay = Running(command.spawn().expect("starting the overlay"));
    let mut stream = accept(&listener).expect("the overlay connected");

    let mut n = 0;
    stream_for(&mut stream, case, &mut n, WARM_UP);
    let _ = fs::write(&timing_log, "");
    let compositor = compositor_pid().map(CpuWindow::start);
    let gpu = GpuSampler::start();
    let usage = Monitor::start(overlay.pid());
    stream_for(&mut stream, case, &mut n, MEASURE);
    let usage = usage.finish();
    let compositor = compositor.and_then(CpuWindow::finish);
    let gpu = gpu.finish();
    let builds = build_stats(&fs::read_to_string(&timing_log).unwrap_or_default());
    drop(stream);
    drop(overlay);
    let _ = fs::remove_dir_all(&root);
    json!({
        "overlay_cpu_percent": usage["cpu_percent"],
        "overlay_rss_peak_kb": usage["rss_peak_kb"],
        "compositor_cpu_percent": compositor,
        "gpu_busy_percent": gpu,
        "builds_per_second": builds.0 as f64 / MEASURE.as_secs_f64(),
        "build": builds.1,
    })
}

/// Waits for the overlay to connect and ask for frames.
fn accept(listener: &UnixListener) -> Option<UnixStream> {
    listener.set_nonblocking(true).ok()?;
    let started = Instant::now();
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(_) if started.elapsed() < Duration::from_secs(10) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    };
    stream.set_nonblocking(false).ok()?;
    let mut request = String::new();
    BufReader::new(stream.try_clone().ok()?).read_line(&mut request).ok()?;
    assert!(request.contains("WatchOverlay"), "{request}");
    Some(stream)
}

/// Sends a frame every [`STEP`] for `how_long`, moving the cursor each time.
fn stream_for(stream: &mut UnixStream, case: &Case, n: &mut usize, how_long: Duration) {
    let until = Instant::now() + how_long;
    while Instant::now() < until {
        let line = format!("{}\n", (case.frame)(*n));
        stream.write_all(line.as_bytes()).expect("sending a frame");
        *n += 1;
        std::thread::sleep(STEP);
    }
}

/// The number of widget builds in the overlay's timing log, and how long one took.
fn build_stats(log: &str) -> (usize, Value) {
    let builds: Vec<u64> = log
        .lines()
        .filter_map(|line| line.strip_prefix("built ")?.split_whitespace().nth(1)?.parse().ok())
        .collect();
    (builds.len(), latency_stats(builds))
}

/// The compositor's process: the one drawing the screen the overlay is on.
fn compositor_pid() -> Option<u32> {
    const NAMES: [&str; 6] = ["kwin_wayland", "sway", "Hyprland", "labwc", "wayfire", "river"];
    fs::read_dir("/proc").ok()?.flatten().find_map(|entry| {
        let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
        let comm = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
        NAMES.contains(&comm.trim()).then_some(pid)
    })
}

/// A process's CPU over a window, from its stat line alone: a compositor with capabilities hides
/// the rest of /proc from other processes, which [`Monitor`] reads.
struct CpuWindow {
    pid: u32,
    started: Instant,
    ticks: u64,
}

impl CpuWindow {
    fn start(pid: u32) -> Self {
        CpuWindow { pid, started: Instant::now(), ticks: cpu_ticks(pid).unwrap_or(0) }
    }

    /// The share of one core it used, in percent.
    fn finish(self) -> Option<f64> {
        let used = cpu_ticks(self.pid)?.saturating_sub(self.ticks) as f64;
        // SAFETY: sysconf only reads a system setting.
        let per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        Some(used / per_second / self.started.elapsed().as_secs_f64() * 100.0)
    }
}

/// User plus system time of `pid`, in clock ticks.
fn cpu_ticks(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields: Vec<&str> = stat[stat.rfind(')')? + 2..].split_whitespace().collect();
    Some(fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?)
}

/// The GPU's busy share over a window, from `nvidia-smi`, where there is one.
struct GpuSampler(Option<Running>);

impl GpuSampler {
    fn start() -> Self {
        let child = Command::new("nvidia-smi")
            .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits", "-lms", "200"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        GpuSampler(child.ok().map(Running))
    }

    fn finish(self) -> Value {
        let Some(mut running) = self.0 else { return Value::Null };
        // SAFETY: kill only sends a signal, to a child we started and have not waited for.
        unsafe { libc::kill(running.pid() as libc::pid_t, libc::SIGTERM) };
        let mut out = String::new();
        if let Some(mut stdout) = running.0.stdout.take() {
            let _ = std::io::Read::read_to_string(&mut stdout, &mut out);
        }
        let samples: Vec<f64> = out.lines().filter_map(|l| l.trim().parse().ok()).collect();
        if samples.is_empty() {
            return Value::Null;
        }
        json!(samples.iter().sum::<f64>() / samples.len() as f64)
    }
}
