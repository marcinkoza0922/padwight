//! `padwight doctor --report <file>`: one text file with what a bug report needs, scrubbed of what is
//! personal (see [`super::scrub`]). Every section is gathered on its own, so a section that fails says so
//! and the rest are still there. It works with the daemon off, and says that it was.

use std::{
    fmt::Write as _,
    fs,
    io::{Read, Seek, SeekFrom},
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::{Roots, devices, render, scrub::Scrubber, system};
use crate::ipc::{self, Request, Response, Status};

/// How many lines of the daemon's log, and of the journal, go in.
const LOG_LINES: usize = 1000;
/// The most of the log file that is read to find those lines.
const LOG_BYTES: u64 = 1 << 20;
/// The most of the setup's files that go in, in bytes.
const SETUP_BYTES: usize = 400_000;

/// What the sections share: whether the daemon answered, and what it said.
struct Gathered {
    status: Result<Status, String>,
    version: Option<String>,
}

type Section = (&'static str, fn(&Gathered) -> Result<String>);

const SECTIONS: [Section; 13] = [
    ("Padwight", padwight),
    ("System", |_| Ok(system::system())),
    ("Desktop", |_| Ok(system::desktop())),
    ("Graphics", |_| Ok(system::gpu())),
    ("Audio", |_| Ok(system::audio())),
    ("Doctor checks", checks),
    ("Daemon", daemon),
    ("Controllers (kernel)", controllers),
    ("Raw HID nodes and who reads them", hidraw),
    ("Steam and other remappers", |_| Ok(system::remappers())),
    ("Daemon log", log),
    ("Journal of the service", journal),
    ("Active setup", setup),
];

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(file: &str) -> Result<()> {
    let text = build();
    fs::write(file, &text).with_context(|| format!("writing {file}"))?;
    println!("Wrote {file} ({} lines).", text.lines().count());
    println!("Personal details (your user and host name, device serial numbers and addresses, window titles) were taken out.");
    println!("Read it before you post it, and cut what you don't want to share.");
    Ok(())
}

/// The whole report, scrubbed.
pub fn build() -> String {
    let (status, version) = ask_daemon();
    let ctx = Gathered { status, version };
    let mut out = String::from(
        "Padwight bug report\n\
         Made by `padwight doctor --report`. Personal details were scrubbed; check it before posting.\n\
         Not collected: display scale and refresh rate, controller firmware, keyboard layout. Add them by hand.\n",
    );
    for (title, gather) in SECTIONS {
        let body = match catch_unwind(AssertUnwindSafe(|| gather(&ctx))) {
            Ok(Ok(body)) => body,
            Ok(Err(e)) => format!("(unavailable: {e:#})\n"),
            Err(_) => "(unavailable: this section crashed)\n".to_owned(),
        };
        let _ = write!(out, "\n== {title} ==\n{}", body.trim_end_matches('\n').to_owned() + "\n");
    }
    scrubber().scrub(&out)
}

/// Asks the daemon for its status; the report goes on without it.
fn ask_daemon() -> (Result<Status, String>, Option<String>) {
    let status = match ipc::request(&Request::Status) {
        Ok(Response::Status(s)) => Ok(s),
        Ok(_) => Err("the daemon sent an unexpected reply".to_owned()),
        Err(e) => Err(format!("{e:#}")),
    };
    let version = match ipc::request(&Request::Version) {
        Ok(Response::Version(v)) => Some(v.app),
        _ => None,
    };
    (status, version)
}

fn scrubber() -> Scrubber {
    let host = fs::read_to_string("/proc/sys/kernel/hostname").or_else(|_| fs::read_to_string("/etc/hostname")).ok();
    let user = std::env::var("USER").or_else(|_| std::env::var("LOGNAME")).ok();
    let home = dirs::home_dir();
    let mut scrubber = Scrubber::new(home.as_deref().and_then(Path::to_str), user.as_deref(), host.as_deref());
    for value in serials() {
        scrubber.secret(&value);
    }
    scrubber
}

/// Serial numbers, unique strings and ids of this machine's input and HID devices.
fn serials() -> Vec<String> {
    let mut found = Vec::new();
    for dir in entries("/sys/class/input") {
        found.extend(read(dir.join("device/uniq")));
    }
    for dir in entries("/sys/class/hidraw") {
        found.extend(uevent(&dir).into_iter().find_map(|(k, v)| (k == "HID_UNIQ").then_some(v)));
    }
    for dir in entries("/sys/class/power_supply") {
        found.extend(read(dir.join("serial_number")));
    }
    found.extend(read("/etc/machine-id"));
    found
}

fn entries(dir: &str) -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    all.sort();
    all
}

fn read(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// The `KEY=value` lines of a sysfs `uevent` file under `dir/device`.
fn uevent(dir: &Path) -> Vec<(String, String)> {
    read(dir.join("device/uevent"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
        .collect()
}

fn padwight(ctx: &Gathered) -> Result<String> {
    let mut out = format!("Version: {}\nCommit: {}\n", env!("CARGO_PKG_VERSION"), env!("PADWIGHT_COMMIT"));
    out.push_str(&format!("Executable: {}\n", std::env::current_exe().map_or_else(|_| "unknown".into(), |p| p.display().to_string())));
    if let Some(app) = &ctx.version {
        out.push_str(&format!("Running daemon's version: {app}\n"));
    }
    out.push_str(&system::service());
    Ok(out)
}

fn checks(_: &Gathered) -> Result<String> {
    Ok(render(&super::check_all()))
}

fn daemon(ctx: &Gathered) -> Result<String> {
    let s = match &ctx.status {
        Ok(s) => s,
        Err(e) => return Ok(format!("NOT RUNNING (or not reachable): {e}\nOnly what can be read from the system is below.\n")),
    };
    let mut out = String::from("Running: yes\n");
    out.push_str(&format!("Remapping enabled: {}\n", s.enabled));
    out.push_str(&format!("Active profile: {}\n", s.active_profile));
    out.push_str(&format!("Active setup: {}\n", s.active_game.as_deref().unwrap_or("General")));
    out.push_str(&format!("Layers: {}\nMode: {}\n", s.active_layers.join(", "), s.active_mode.as_deref().unwrap_or("default")));
    out.push_str(&format!("Profile locked: {}\n", s.profile_locked));
    out.push_str(&format!("Focus backend: {:?} ({})\n", s.focus_backend, s.focus_backend.label()));
    out.push_str("Focused window and recent windows: left out\n");
    for d in &s.devices {
        out.push_str(&format!(
            "Device: {} id={} managed={} ignored={} profile={:?} analog_triggers={} rumble={} gyro={} paddles={} touchpad={} family={:?} model={:?}\n",
            d.name, d.id, d.managed, d.ignored, d.profile, d.analog_triggers, d.rumble, d.gyro, d.paddles, d.touchpad, d.family, d.model
        ));
    }
    for c in &s.controllers {
        out.push_str(&format!("Remembered controller: id={} name={} connected={} pass_through={} pending={}\n", c.id, c.name, c.connected, c.pass_through, c.pending));
    }
    for name in &s.motion_access_denied {
        out.push_str(&format!("Motion sensor not readable: {name}\n"));
    }
    for r in &s.raw_readers {
        out.push_str(&format!("Other programs read {} directly: {}\n", r.controller, r.programs.join(", ")));
    }
    Ok(out)
}

fn bus(code: &str) -> &'static str {
    match code.trim_start_matches('0') {
        "3" => "USB",
        "5" => "Bluetooth",
        "6" => "virtual (HIL)",
        "19" => "host",
        _ => "other",
    }
}

fn controllers(_: &Gathered) -> Result<String> {
    let roots = Roots::system();
    let nodes = devices::scan(&roots.sysfs_input, &roots.input_dev);
    if nodes.is_empty() {
        return Ok("No gamepad or motion-sensor input nodes found.\n".into());
    }
    let mut out = String::new();
    for node in nodes {
        let id = |file: &str| read(roots.sysfs_input.join(&node.event).join("device/id").join(file)).unwrap_or_else(|| "?".into());
        let (vendor, product, version, bustype) = (id("vendor"), id("product"), id("version"), id("bustype"));
        let kind = match (node.gamepad, node.motion) {
            (true, _) => "gamepad",
            (_, true) => "motion sensor",
            _ => "other",
        };
        let has_uniq = read(roots.sysfs_input.join(&node.event).join("device/uniq")).is_some();
        let access = match node.access {
            Ok(()) => "readable".to_owned(),
            Err(kind) => format!("not readable ({kind})"),
        };
        out.push_str(&format!(
            "{}: {} [{kind}]\n    id {vendor}:{product} version {version}, connection {} (bus {bustype}), serial/address {}\n    drivers {}\n    node {access}\n",
            node.event,
            node.name,
            bus(&bustype),
            if has_uniq { "present (scrubbed)" } else { "none" },
            node.drivers.join(", "),
        ));
    }
    Ok(out)
}

fn hidraw(_: &Gathered) -> Result<String> {
    let roots = Roots::system();
    let pads: Vec<String> = devices::scan(&roots.sysfs_input, &roots.input_dev)
        .iter()
        .filter(|n| n.gamepad)
        .filter_map(|n| {
            let id = |file: &str| read(roots.sysfs_input.join(&n.event).join("device/id").join(file));
            Some(format!("{}:{}", id("vendor")?.to_lowercase(), id("product")?.to_lowercase()))
        })
        .collect();
    let openers = crate::daemon::raw_readers::hidraw_openers();
    let mut out = String::new();
    for dir in entries("/sys/class/hidraw") {
        let info = uevent(&dir);
        let get = |key: &str| info.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        // HID_ID is bus:vendor:product in 8 hex digits each.
        let ids = get("HID_ID").map(|v| v.to_lowercase()).unwrap_or_default();
        let parts: Vec<&str> = ids.split(':').collect();
        let key = match parts.as_slice() {
            [_, vendor, product] => normalize(&format!("{vendor}:{product}")),
            _ => String::new(),
        };
        if !pads.iter().any(|p| normalize(p) == key) {
            continue;
        }
        let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let node = Path::new("/dev").join(&name);
        let mode = fs::metadata(&node).map(|m| format!("{:04o}", std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o7777)).unwrap_or_else(|_| "?".into());
        let readers = openers.get(&node).map(|r| r.join(", ")).unwrap_or_else(|| "nobody".into());
        out.push_str(&format!("{name}: {} (HID_ID {ids}) mode {mode}, open by: {readers}\n", get("HID_NAME").unwrap_or_default()));
    }
    Ok(if out.is_empty() { "No raw HID node belongs to a connected gamepad.\n".into() } else { out })
}

/// `054c:0ce6` and `54c:ce6` are the same pair.
fn normalize(pair: &str) -> String {
    pair.split(':').map(|p| p.trim_start_matches('0')).collect::<Vec<_>>().join(":")
}

/// The last `n` lines of `text`.
fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n") + "\n"
}

fn log(_: &Gathered) -> Result<String> {
    let path = crate::daemon::log_path();
    let mut file = fs::File::open(&path).with_context(|| format!("no log at {}", path.display()))?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(LOG_BYTES)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(format!("{} (last {LOG_LINES} lines)\n{}", path.display(), last_lines(&text, LOG_LINES)))
}

fn journal(_: &Gathered) -> Result<String> {
    let lines = LOG_LINES.to_string();
    let text = system::run_all("journalctl", &["--user", "-u", "padwight.service", "-n", &lines, "--no-pager", "-o", "short-iso"])
        .context("journalctl has nothing for padwight.service, or isn't there")?;
    if text.lines().filter(|l| !l.starts_with("-- ")).count() == 0 {
        return Ok("The service has logged nothing to the journal.\n".into());
    }
    Ok(format!("last {LOG_LINES} lines\n{text}"))
}

/// The folder of the setup that is active: the daemon's answer, else `config.toml`'s.
fn active_game(ctx: &Gathered, config: &Path) -> Option<String> {
    if let Ok(s) = &ctx.status {
        return s.active_game.clone();
    }
    let table: toml::Table = fs::read_to_string(config).ok()?.parse().ok()?;
    table.get("active")?.get("setup")?.as_str().map(str::to_owned)
}

fn setup_folder(root: &Path, game: Option<&str>) -> Option<PathBuf> {
    let Some(game) = game else { return Some(root.join("general")) };
    entries(root.join("setups").to_str()?).into_iter().find(|dir| {
        read(dir.join("setup.toml"))
            .and_then(|t| t.parse::<toml::Table>().ok())
            .and_then(|t| t.get("name").and_then(|n| n.as_str().map(str::to_owned)))
            .is_some_and(|name| name == game)
    })
}

fn files_under(dir: &Path, found: &mut Vec<PathBuf>) {
    for path in entries(dir.to_str().unwrap_or_default()) {
        if path.is_dir() {
            files_under(&path, found);
        } else if path.extension().is_some_and(|e| e == "toml") {
            found.push(path);
        }
    }
}

fn setup(ctx: &Gathered) -> Result<String> {
    let config = crate::config::Config::path();
    let root = config.parent().context("the config path has no folder")?;
    let game = active_game(ctx, &config);
    let folder = setup_folder(root, game.as_deref()).with_context(|| format!("no folder for the setup {game:?}"))?;
    let mut files = vec![config.clone()];
    files_under(&folder, &mut files);
    let mut out = String::new();
    let mut used = 0;
    for path in files {
        let shown = path.strip_prefix(root).unwrap_or(&path).display().to_string();
        let Ok(text) = fs::read_to_string(&path) else { continue };
        used += text.len();
        if used > SETUP_BYTES {
            out.push_str("(more files left out: the report is full)\n");
            break;
        }
        let _ = write!(out, "--- {shown} ---\n{}\n", text.trim_end());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_last_lines() {
        assert_eq!(last_lines("a\nb\nc\n", 2), "b\nc\n");
        assert_eq!(last_lines("a\n", 5), "a\n");
    }

    #[test]
    fn pairs_compare_without_leading_zeros() {
        assert_eq!(normalize("054c:0ce6"), normalize("54c:ce6"));
    }

    #[test]
    fn names_the_connection() {
        assert_eq!(bus("0003"), "USB");
        assert_eq!(bus("0005"), "Bluetooth");
    }

    #[test]
    fn finds_the_folder_of_a_setup_by_its_name() {
        let root = std::env::temp_dir().join(format!("padwight-report-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("setups/001-Doom")).unwrap();
        fs::create_dir_all(root.join("setups/002-Quake")).unwrap();
        fs::write(root.join("setups/001-Doom/setup.toml"), "name = \"Doom\"\n").unwrap();
        fs::write(root.join("setups/002-Quake/setup.toml"), "name = \"Quake\"\n").unwrap();
        assert_eq!(setup_folder(&root, Some("Quake")), Some(root.join("setups/002-Quake")));
        assert_eq!(setup_folder(&root, None), Some(root.join("general")));
        assert_eq!(setup_folder(&root, Some("Nope")), None);
        let mut found = Vec::new();
        files_under(&root.join("setups/001-Doom"), &mut found);
        assert_eq!(found, [root.join("setups/001-Doom/setup.toml")]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_report_is_made_without_a_daemon_and_says_so() {
        let ctx = Gathered { status: Err("daemon not reachable".into()), version: None };
        assert!(daemon(&ctx).unwrap().starts_with("NOT RUNNING"));
    }

    #[test]
    fn a_failing_section_does_not_stop_the_report() {
        let broken: Section = ("Broken", |_| anyhow::bail!("boom"));
        let ctx = Gathered { status: Err(String::new()), version: None };
        let body = match (broken.1)(&ctx) {
            Ok(b) => b,
            Err(e) => format!("(unavailable: {e:#})\n"),
        };
        assert!(body.contains("unavailable: boom"));
    }
}
