//! What a bug report says about the machine: system, desktop, graphics, audio, init, and the other
//! programs that handle controllers. Each function reads what it can and says so when it can't.

use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

/// How long a helper program gets before it is given up on.
const HELPER_TIME: Duration = Duration::from_secs(3);

/// The first line of what `program args` prints, or `None` when it's missing, fails or hangs.
pub fn run(program: &str, args: &[&str]) -> Option<String> {
    run_all(program, args).and_then(|out| out.lines().next().map(|l| l.trim().to_owned())).filter(|l| !l.is_empty())
}

/// Everything `program args` prints, or `None` when it's missing, fails or hangs.
pub fn run_all(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stdout.read_to_string(&mut text);
        let _ = tx.send(text);
    });
    match rx.recv_timeout(HELPER_TIME) {
        Ok(text) => {
            let ok = child.wait().is_ok_and(|s| s.success());
            ok.then_some(text)
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    }
}

fn read(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn os_release_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('=')).map(|v| v.trim_matches('"').to_owned())
}

/// Lines of `key: value` for the operating system, kernel and session.
pub fn system() -> String {
    let release = read("/etc/os-release").unwrap_or_default();
    let mut out = String::new();
    let os = os_release_value(&release, "PRETTY_NAME").unwrap_or_else(|| "unknown".into());
    out.push_str(&format!("OS: {os}\n"));
    if let Some(variant) = os_release_value(&release, "VARIANT_ID") {
        out.push_str(&format!("Variant: {variant}\n"));
    }
    out.push_str(&format!("Kernel: {}\n", read("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".into())));
    out.push_str(&format!("Architecture: {}\n", std::env::consts::ARCH));
    let init = read("/proc/1/comm").unwrap_or_else(|| "unknown".into());
    out.push_str(&format!("Init system: {init}\n"));
    let sandbox = if Path::new("/.flatpak-info").exists() {
        "Flatpak"
    } else if var("SNAP").is_some() {
        "Snap"
    } else if Path::new("/run/.containerenv").exists() || Path::new("/.dockerenv").exists() {
        "container"
    } else {
        "none"
    };
    out.push_str(&format!("Sandbox: {sandbox}\n"));
    out
}

/// The desktop, its version and the kind of session.
pub fn desktop() -> String {
    let current = var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let mut out = String::new();
    out.push_str(&format!("Desktop: {}\n", if current.is_empty() { "unknown" } else { &current }));
    out.push_str(&format!("Session type: {}\n", var("XDG_SESSION_TYPE").unwrap_or_else(|| "unknown".into())));
    out.push_str(&format!("Display: {}\n", display_kind()));
    let lower = current.to_lowercase();
    let probes: [(&str, &str, &[&str]); 8] = [
        ("kde", "plasmashell", &["--version"]),
        ("gnome", "gnome-shell", &["--version"]),
        ("x-cinnamon", "cinnamon", &["--version"]),
        ("cosmic", "cosmic-comp", &["--version"]),
        ("sway", "sway", &["--version"]),
        ("hyprland", "Hyprland", &["--version"]),
        ("niri", "niri", &["--version"]),
        ("budgie", "budgie-desktop", &["--version"]),
    ];
    let version = probes.iter().find(|(name, ..)| lower.contains(name)).and_then(|(_, program, args)| run(program, args));
    out.push_str(&format!("Desktop version: {}\n", version.unwrap_or_else(|| "not found".into())));
    out
}

fn display_kind() -> String {
    let wayland = var("WAYLAND_DISPLAY").is_some();
    let x11 = var("DISPLAY").is_some();
    match (wayland, x11) {
        (true, true) => "Wayland (with XWayland)".into(),
        (true, false) => "Wayland".into(),
        (false, true) => "X11".into(),
        (false, false) => "none (no graphical session in this shell)".into(),
    }
}

/// Graphics cards and their kernel drivers. PCI ids only; nothing about the machine's owner.
pub fn gpu() -> String {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return "not readable\n".into() };
    let mut cards: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.strip_prefix("card").is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit())))
        .collect();
    cards.sort();
    let mut out = String::new();
    for card in cards {
        let dir = Path::new("/sys/class/drm").join(&card).join("device");
        let id = |file: &str| read(dir.join(file)).unwrap_or_else(|| "?".into());
        let driver = fs::read_link(dir.join("driver")).ok().and_then(|l| l.file_name().map(|n| n.to_string_lossy().into_owned()));
        out.push_str(&format!("{card}: vendor {} device {} driver {}\n", id("vendor"), id("device"), driver.as_deref().unwrap_or("none")));
    }
    if let Some(version) = read("/proc/driver/nvidia/version").and_then(|v| v.lines().next().map(str::to_owned)) {
        out.push_str(&format!("NVIDIA: {version}\n"));
    }
    if out.is_empty() { "none found\n".into() } else { out }
}

/// The sound server, as `pactl` names it, else the server program that runs.
pub fn audio() -> String {
    let server = run_all("pactl", &["info"])
        .and_then(|info| info.lines().find_map(|l| l.strip_prefix("Server Name:").map(|v| v.trim().to_owned())));
    match server {
        Some(name) => format!("Sound server: {name}\n"),
        None => {
            let found: Vec<&str> = ["pipewire", "pipewire-pulse", "pulseaudio", "jackd"].into_iter().filter(|n| process_running(n)).collect();
            format!("Sound server: {}\n", if found.is_empty() { "not found".into() } else { found.join(", ") })
        }
    }
}

/// Process names (`comm`) of the processes this user can see, with their command lines.
fn processes() -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir("/proc") else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit())))
        .filter_map(|e| {
            let comm = read(e.path().join("comm"))?;
            let cmdline = fs::read(e.path().join("cmdline")).map(|b| String::from_utf8_lossy(&b).replace('\0', " ")).unwrap_or_default();
            Some((comm, cmdline))
        })
        .collect()
}

fn process_running(name: &str) -> bool {
    processes().iter().any(|(comm, _)| comm == name)
}

/// Steam (and its package kind) and other programs that remap or read controllers.
pub fn remappers() -> String {
    const NAMES: [&str; 9] =
        ["steam", "steamwebhelper", "antimicrox", "inputplumber", "opengamepadui", "sc-controller", "joyshockmapper", "input-remapper-service", "keyd"];
    let all = processes();
    let mut out = String::new();
    for name in NAMES {
        let Some((_, cmdline)) = all.iter().find(|(comm, _)| comm == name) else { continue };
        let package = if name == "steam" || name == "steamwebhelper" {
            if cmdline.contains("com.valvesoftware.Steam") || cmdline.contains("/app/") {
                " (Flatpak)"
            } else if cmdline.contains("/snap/") {
                " (Snap)"
            } else {
                " (native or unknown package)"
            }
        } else {
            ""
        };
        out.push_str(&format!("{name}: running{package}\n"));
    }
    if out.is_empty() { "none of steam, antimicrox, inputplumber, opengamepadui, sc-controller, joyshockmapper, input-remapper, keyd running\n".into() } else { out }
}

/// Whether the systemd user service is installed, enabled and running.
pub fn service() -> String {
    let state = |verb: &str| run("systemctl", &["--user", verb, "padwight.service"]).unwrap_or_else(|| "unknown".into());
    format!("padwight.service: {} / {}\n", state("is-enabled"), state("is-active"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_os_release_values_without_quotes() {
        let text = "NAME=\"Nobara Linux\"\nPRETTY_NAME=\"Nobara Linux 44\"\nVARIANT_ID=kde\n";
        assert_eq!(os_release_value(text, "PRETTY_NAME").as_deref(), Some("Nobara Linux 44"));
        assert_eq!(os_release_value(text, "VARIANT_ID").as_deref(), Some("kde"));
        assert_eq!(os_release_value(text, "ID"), None);
    }

    #[test]
    fn a_missing_program_gives_none() {
        assert_eq!(run("padwight-no-such-program", &[]), None);
    }

    #[test]
    fn a_hanging_program_is_given_up_on() {
        let start = std::time::Instant::now();
        assert_eq!(run_all("sleep", &["30"]), None);
        assert!(start.elapsed() < Duration::from_secs(10));
    }
}
