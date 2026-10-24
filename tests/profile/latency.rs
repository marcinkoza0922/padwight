//! Input latency: from a South press on the controller to the daemon's virtual pad reporting it,
//! with the controller idle, then under a 1 kHz stick flood, as a busy controller would be, then
//! with every CPU kept busy by threads of normal priority, as a demanding game would.
//!
//! The test runs several sessions, each with a fresh daemon, so run-to-run variation shows up in
//! the figures. The samples of all sessions are pooled for the distribution. Each suite run also
//! appends its summary to `history.jsonl`, which the plots use for the trend.

use std::{io::Write, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

use evdev::InputEvent;
use serde_json::{Value, json};

use super::support::{Output, Pad, Rng, Running, latency_stats, now_ns, output_dir, padwight, report, south, wait_managed};
use crate::common::{default_config, install_config, serial, temp_root, TIMEOUT};

/// Presses per condition, per session. At 20 to 60 ms between them, a session takes about 30 s.
const SAMPLES: usize = 300;
/// The gap before each press, in milliseconds: random, so presses don't line up with daemon timers.
const GAP_MS: (u64, u64) = (20, 60);
/// How long to wait for the daemon's output before a press counts as missed.
const WAIT: Duration = Duration::from_secs(2);
/// Sessions per run, overridable with `PADWIGHT_LATENCY_REPEATS`.
const REPEATS: usize = 3;
const NAME: &str = "Profile Controller";

/// The conditions each session measures, in order, by their names in the results.
const CONDITIONS: [&str; 3] = ["idle", "flood_1khz", "cpu_load"];

/// The samples and counts of one condition in one session.
#[derive(Default)]
struct Condition {
    samples_ns: Vec<u64>,
    missed: u32,
    seconds: f64,
    reports_per_second: f64,
    output_events_per_second: f64,
    dropped_total: u64,
    /// The daemon's own stages for every batch it took meanwhile (see [`Stages`]).
    stages: Stages,
}

/// How long each batch of input spent at each stage inside the daemon, in nanoseconds, from its
/// input timing log (`PADWIGHT_INPUT_TIMING`): the reader waking after the kernel stamped the
/// event, the hand-over to the main loop, mapping and writing the output, and the bookkeeping
/// after it.
#[derive(Default)]
struct Stages {
    wake: Vec<u64>,
    hand_over: Vec<u64>,
    map_and_send: Vec<u64>,
    after: Vec<u64>,
}

impl Stages {
    /// The batches of the log at `path` whose kernel time stamp falls in `from..to`.
    fn read(path: &std::path::Path, from: u64, to: u64) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let numbers = |line: &str| line.split(' ').skip(1).filter_map(|n| n.parse::<u64>().ok()).collect::<Vec<_>>();
        let mut read_at = std::collections::HashMap::new();
        let mut stages = Stages::default();
        for line in text.lines() {
            let n = numbers(line);
            match (line.split(' ').next(), n.as_slice()) {
                (Some("read"), [kernel, read]) if (from..to).contains(kernel) => {
                    stages.wake.push(read.saturating_sub(*kernel));
                    read_at.insert(*kernel, *read);
                }
                (Some("input"), [kernel, received, sent, done]) if (from..to).contains(kernel) => {
                    if let Some(read) = read_at.get(kernel) {
                        stages.hand_over.push(received.saturating_sub(*read));
                    }
                    stages.map_and_send.push(sent.saturating_sub(*received));
                    stages.after.push(done.saturating_sub(*sent));
                }
                _ => {}
            }
        }
        stages
    }

    fn extend(&mut self, other: Stages) {
        self.wake.extend(other.wake);
        self.hand_over.extend(other.hand_over);
        self.map_and_send.extend(other.map_and_send);
        self.after.extend(other.after);
    }

    fn json(&self) -> Value {
        json!({
            "wake": latency_stats(self.wake.clone()),
            "hand_over": latency_stats(self.hand_over.clone()),
            "map_and_send": latency_stats(self.map_and_send.clone()),
            "after": latency_stats(self.after.clone()),
        })
    }
}

#[test]
#[ignore]
fn profile_input_latency() {
    let _serial = serial();
    let repeats = repeats();
    let mut pooled: Vec<Condition> = CONDITIONS.iter().map(|_| Condition::default()).collect();
    let mut sessions = Vec::new();
    for repeat in 0..repeats {
        let measured = session(repeat);
        let mut one = serde_json::Map::new();
        one.insert("repeat".into(), json!(repeat));
        for ((name, all), c) in CONDITIONS.iter().zip(&mut pooled).zip(measured) {
            one.insert((*name).into(), summary(&c));
            all.samples_ns.extend(c.samples_ns);
            all.missed += c.missed;
            all.stages.extend(c.stages);
        }
        sessions.push(Value::Object(one));
    }
    let mut results = serde_json::Map::new();
    results.insert("what".into(), json!("South press written to the uhid pad, to the kernel time stamp of BTN_SOUTH on the daemon's virtual pad"));
    results.insert("samples_per_condition_per_session".into(), json!(SAMPLES));
    results.insert("sessions".into(), json!(repeats));
    for (name, c) in CONDITIONS.iter().zip(&pooled) {
        results.insert((*name).into(), condition_json(c));
    }
    results.insert("per_session".into(), Value::Array(sessions));
    append_history(repeats, &results);
    report("input_latency", Value::Object(results));
}

/// Sessions per run: `PADWIGHT_LATENCY_REPEATS` if it is a positive number, else [`REPEATS`].
fn repeats() -> usize {
    std::env::var("PADWIGHT_LATENCY_REPEATS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(REPEATS)
}

/// One session: a fresh daemon, then each of [`CONDITIONS`].
fn session(repeat: usize) -> Vec<Condition> {
    let root = temp_root();
    install_config(&root.join("config/padwight"), &default_config());
    // The pad exists before the daemon starts, as a controller plugged in first would.
    let mut pad = Pad::new(NAME);
    let timing = root.join("input-timing.log");
    let daemon = Running(padwight(&root, &["daemon"]).env("PADWIGHT_INPUT_TIMING", &timing).spawn().expect("starting the daemon"));
    wait_managed(Instant::now(), &root.join("run"), NAME, TIMEOUT).expect("the daemon grabbed the pad");
    let mut out = Output::open();
    warm_up(&mut pad, &mut out);

    let mut measured = Vec::new();
    for (i, name) in CONDITIONS.iter().enumerate() {
        let load = (*name == "cpu_load").then(CpuLoad::start);
        let from = now_ns();
        let mut c = measure(&mut pad, &mut out, *name == "flood_1khz", seed(repeat, i as u64));
        c.stages = Stages::read(&timing, from, now_ns());
        drop(load);
        measured.push(c);
    }

    drop(daemon);
    // Kept beside the results: it says, among other things, how the input threads are scheduled.
    let _ = std::fs::copy(root.join("daemon.log"), output_dir().join(format!("latency-daemon-{repeat}.log")));
    let _ = std::fs::copy(&timing, output_dir().join(format!("latency-timing-{repeat}.log")));
    let _ = std::fs::remove_dir_all(&root);
    measured
}

/// A thread per CPU spinning at normal priority, until dropped.
struct CpuLoad {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl CpuLoad {
    fn start() -> Self {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        let stop = Arc::new(AtomicBool::new(false));
        let cpus = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
        let threads = (0..cpus)
            .map(|_| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    let mut x = 0u64;
                    while !stop.load(Ordering::Relaxed) {
                        x = std::hint::black_box(x.wrapping_mul(6364136223846793005).wrapping_add(1));
                    }
                })
            })
            .collect();
        CpuLoad { stop, threads }
    }
}

impl Drop for CpuLoad {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

/// A different, non-zero generator seed for each session and condition.
fn seed(repeat: usize, condition: u64) -> u64 {
    let base: u64 = 0x9E37_79B9_7F4A_7C15;
    (base ^ ((repeat as u64 + 1).wrapping_mul(0x1000_0000_01B3)) ^ (condition << 32)).max(1)
}

/// Presses until one comes out, so the daemon is converting the pad before anything is timed.
fn warm_up(pad: &mut Pad, out: &mut Output) {
    for _ in 0..10 {
        pad.set_south(true);
        let seen = out.wait(Duration::from_millis(300), |ev| south(ev, 1), || pad.tick());
        pad.set_south(false);
        out.wait(Duration::from_millis(300), |ev| south(ev, 0), || pad.tick());
        if seen.is_some() {
            return;
        }
    }
    panic!("the daemon never output a South press from the uhid pad");
}

/// One condition: press South [`SAMPLES`] times, timing each press from the write to the output.
/// With `flood`, the stick is swept at 1 kHz the whole time, including during each wait.
fn measure(pad: &mut Pad, out: &mut Output, flood: bool, seed: u64) -> Condition {
    pad.flood = flood;
    let mut rng = Rng(seed);
    let mut result = Condition::default();
    let sent_before = pad.sent;
    let events_before = out.events;
    let dropped_before = out.dropped;
    let started = Instant::now();
    for _ in 0..SAMPLES {
        let gap = Duration::from_millis(rng.range(GAP_MS.0, GAP_MS.1));
        out.wait(gap, |_: &InputEvent| false, || pad.tick());
        let sent = now_ns();
        pad.set_south(true);
        match out.wait(WAIT, |ev| south(ev, 1), || pad.tick()) {
            Some(seen) => result.samples_ns.push(seen.saturating_sub(sent)),
            None => result.missed += 1,
        }
        pad.set_south(false);
        out.wait(WAIT, |ev| south(ev, 0), || pad.tick());
    }
    pad.flood = false;
    result.seconds = started.elapsed().as_secs_f64();
    result.reports_per_second = (pad.sent - sent_before) as f64 / result.seconds;
    result.output_events_per_second = (out.events - events_before) as f64 / result.seconds;
    result.dropped_total = out.dropped - dropped_before;
    result
}

/// Latency summary and counts of one condition, with its raw samples in microseconds.
fn condition_json(c: &Condition) -> Value {
    let samples_us: Vec<f64> = c.samples_ns.iter().map(|ns| *ns as f64 / 1000.0).collect();
    json!({
        "latency": latency_stats(c.samples_ns.clone()),
        "samples_us": samples_us,
        "missed": c.missed,
        "seconds": c.seconds,
        "reports_per_second": c.reports_per_second,
        "output_events_per_second": c.output_events_per_second,
        "dropped_output_events": c.dropped_total,
        "daemon_stages": c.stages.json(),
    })
}

/// Per-session figures without the samples.
fn summary(c: &Condition) -> Value {
    let stats = latency_stats(c.samples_ns.clone());
    json!({ "p50_us": stats["p50_us"], "p90_us": stats["p90_us"], "p99_us": stats["p99_us"], "max_us": stats["max_us"], "missed": c.missed })
}

/// Appends one line to `history.jsonl` in the output directory: the pooled p50 and p99 of each
/// condition, with the time and commit, so the trend across suite runs can be plotted.
fn append_history(sessions: usize, results: &serde_json::Map<String, Value>) {
    let mut line = json!({
        "time": SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        "commit": super::support::commit(),
        "sessions": sessions,
    });
    for (name, short) in CONDITIONS.iter().zip(["idle", "flood", "load"]) {
        line[format!("{short}_p50_us")] = results[*name]["latency"]["p50_us"].clone();
        line[format!("{short}_p99_us")] = results[*name]["latency"]["p99_us"].clone();
    }
    let dir = output_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("history.jsonl")).unwrap();
    writeln!(file, "{line}").unwrap();
}
