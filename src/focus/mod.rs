//! Which game is active: a tracker for the running desktop reports the focused window, windows
//! and processes are identified from /proc (see [`identify`]),
//! and per-game rules pick the profile. Desktops without a tracker fall back to matching running
//! processes.

#[cfg(target_os = "linux")]
mod dbus;
#[cfg(target_os = "linux")]
mod gnome;
#[cfg(target_os = "linux")]
mod hyprland;
#[cfg(target_os = "linux")]
mod identify;
#[cfg(target_os = "linux")]
mod kwin;
#[cfg(target_os = "linux")]
mod sway;
#[cfg(target_os = "linux")]
mod wlroots;

use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::{thread, time::Duration};

#[cfg(target_os = "linux")]
use zbus::blocking::Connection;

use crate::{
    config::{Config, ProfileRef, Rule, RuleKind},
    ipc::{FocusBackend, WindowInfo},
    monitor::log,
};
#[cfg(target_os = "linux")]
pub use identify::{identify, is_own_window, running_processes};

/// How often we check that the desktop's tracker is still there (a compositor restart, a late
/// login, an extension switched on).
#[cfg(target_os = "linux")]
const WATCHDOG: Duration = Duration::from_secs(10);

/// Events the focus tracker sends to the daemon.
pub enum FocusEvent {
    Focused(WindowInfo),
    /// Nothing has focus, such as the desktop or an empty workspace.
    Unfocused,
    Backend(FocusBackend),
}

type Notify = Arc<dyn Fn(FocusEvent) + Send + Sync>;

/// What a compositor tells us about the focused window, before we look at its process.
struct Reported {
    class: String,
    title: String,
    pid: u32,
}

fn report(notify: &Notify, window: Reported) {
    notify(FocusEvent::Focused(identify(window.class, window.title, window.pid)));
}

/// Tells the daemon about changes of backend, and logs them once.
struct Reporter {
    notify: Notify,
    backend: Option<FocusBackend>,
}

impl Reporter {
    fn backend(&mut self, now: FocusBackend) {
        if self.backend != Some(now) {
            log!("focus tracking: {}", now.label());
            (self.notify)(FocusEvent::Backend(now));
            self.backend = Some(now);
        }
    }
}

/// Starts focus tracking: reports which tracker is working through `Backend`, and keeps
/// retrying so it recovers when the desktop's shell appears or restarts.
#[cfg(target_os = "linux")]
pub fn spawn(notify: impl Fn(FocusEvent) + Send + Sync + 'static) {
    let notify: Notify = Arc::new(notify);
    thread::spawn(move || {
        let conn = match dbus::serve(notify.clone()) {
            Ok(conn) => Some(conn),
            Err(e) => {
                log!("focus tracking: no session bus ({e:#})");
                None
            }
        };
        // A script left by an earlier run would not report the current window until focus
        // changes; reloading it reports immediately.
        if let Some(conn) = &conn {
            let _ = kwin::unload_script(conn);
        }
        let mut reporter = Reporter { notify: notify.clone(), backend: None };
        loop {
            track(&mut reporter, conn.as_ref());
            thread::sleep(WATCHDOG);
        }
    });
}

/// One round of tracking. Sway and Hyprland push events, so this returns only when that
/// connection ends; the rest are checked once.
#[cfg(target_os = "linux")]
fn track(reporter: &mut Reporter, conn: Option<&Connection>) {
    let notify = reporter.notify.clone();
    if let Ok(sway) = sway::connect() {
        reporter.backend(FocusBackend::Sway);
        if let Err(e) = sway.follow(&notify) {
            log!("lost Sway: {e:#}");
        }
    } else if let Ok(hypr) = hyprland::connect() {
        reporter.backend(FocusBackend::Hyprland);
        if let Err(e) = hypr.follow(&notify) {
            log!("lost Hyprland: {e:#}");
        }
    } else if let Some(backend) = conn.and_then(shell_backend) {
        reporter.backend(backend);
    } else if let Ok(wl) = wlroots::connect() {
        reporter.backend(FocusBackend::Wlroots);
        if let Err(e) = wl.follow(&notify) {
            log!("lost the Wayland compositor: {e:#}");
        }
    } else {
        reporter.backend(FocusBackend::ProcessScan);
    }
}

/// The tracker for a desktop reached over D-Bus: KWin, or GNOME Shell with our extension. `None`
/// when the desktop isn't one of those.
#[cfg(target_os = "linux")]
fn shell_backend(conn: &Connection) -> Option<FocusBackend> {
    if kwin::ensure_script(conn).is_ok() {
        Some(FocusBackend::Kwin)
    } else if gnome::is_running(conn) && gnome::ensure_extension(conn).is_ok() {
        Some(FocusBackend::GnomeShell)
    } else {
        None
    }
}

/// Last component of a Unix or Windows path.
pub fn base_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn rule_matches(kind: RuleKind, value: &str, info: &WindowInfo) -> bool {
    let value = value.trim();
    !value.is_empty()
        && match kind {
            RuleKind::Executable if names_folder(value) => path_ends_with(&info.path, value),
            RuleKind::Executable => info.exe.eq_ignore_ascii_case(value),
            RuleKind::SteamAppId => info.steam_app_id.as_deref() == Some(value),
            RuleKind::WindowClass => info.class.eq_ignore_ascii_case(value),
        }
}

/// Whether an Executable rule's value names folders as well as the file
/// (`Prince of Persia - The Sands of Time/gpp.exe`), to tell apart games whose programs share
/// a name.
pub(crate) fn names_folder(value: &str) -> bool {
    value.contains(['/', '\\'])
}

/// Whether `path` ends with the folders and file of `tail`, whole names only, ignoring case and
/// whether either uses `/` or `\`.
pub(crate) fn path_ends_with(path: &str, tail: &str) -> bool {
    let norm = |s: &str| s.replace('\\', "/").to_ascii_lowercase();
    let (path, tail) = (norm(path), norm(tail));
    let tail = tail.trim_matches('/');
    !tail.is_empty() && (path == tail || path.ends_with(&format!("/{tail}")))
}

/// How closely a rule picks out a program: one naming folders beats one naming only the file.
fn specificity(rule: &Rule) -> u8 {
    u8::from(rule.kind == RuleKind::Executable && names_folder(&rule.value))
}

/// The highest [`specificity`]: a match this close ends the search.
const MOST_SPECIFIC: u8 = 1;

/// The most specific of the rules that match, the first of those in rule order. Stops at the
/// first match nothing can beat, so the process scan doesn't try every rule.
fn best<'a, T>(found: impl Iterator<Item = (&'a str, &'a Rule, T)>) -> Option<(&'a str, &'a Rule, T)> {
    let mut best: Option<(&str, &Rule, T)> = None;
    for next in found {
        let closeness = specificity(next.1);
        if best.as_ref().is_none_or(|b| closeness > specificity(b.1)) {
            best = Some(next);
            if closeness == MOST_SPECIFIC {
                break;
            }
        }
    }
    best
}

/// Every game's switched-on rules, in order (games in order, then each game's rules).
pub fn rules(config: &Config) -> impl Iterator<Item = (&str, &Rule)> {
    config.games.iter().flat_map(|g| g.rules.iter().filter(|r| r.enabled).map(move |r| (g.name.as_str(), r)))
}

/// The rule that matches a focused window, with its game's name: the most specific one, and
/// the first of those.
pub fn matching_rule<'a>(config: &'a Config, info: &WindowInfo) -> Option<(&'a str, &'a Rule)> {
    best(rules(config).filter(|(_, r)| rule_matches(r.kind, &r.value, info)).map(|(g, r)| (g, r, ()))).map(|(g, r, ())| (g, r))
}

/// Profile for a focused window: the first matching rule's, `None` if it isn't a game's.
pub fn profile_for(config: &Config, info: &WindowInfo) -> Option<ProfileRef> {
    matching_rule(config, info).map(|(game, r)| ProfileRef::new(Some(game), &r.profile))
}

/// The rule that matches a running process, with that process: the most specific one, and the
/// first of those in rule order.
pub fn matching_process<'a>(config: &'a Config, processes: &'a [WindowInfo]) -> Option<(&'a str, &'a Rule, &'a WindowInfo)> {
    best(rules(config).filter_map(|(game, r)| processes.iter().find(|p| rule_matches(r.kind, &r.value, p)).map(|p| (game, r, p))))
}

/// Profile from running processes, for desktops without focus information.
pub fn profile_for_processes(config: &Config, processes: &[WindowInfo]) -> Option<ProfileRef> {
    matching_process(config, processes).map(|(game, r, _)| ProfileRef::new(Some(game), &r.profile))
}

/// A rule as people read it, for the reason a profile was switched: `Executable "doom.exe"`.
pub fn rule_text(rule: &Rule) -> String {
    format!("{} “{}”", rule.kind, rule.value)
}

/// Whether rules can pick `game`, so it has focus only while one matches.
pub fn has_rules(config: &Config, game: &str) -> bool {
    rules(config).any(|(g, _)| g == game)
}

/// The game a rule matches this window to, and the window's process: a launch is the first
/// focus of a game with a process it hasn't had. (The default profile isn't a game launch.)
pub fn game_launch(config: &Config, info: &WindowInfo) -> Option<(String, u32)> {
    matching_rule(config, info).map(|(game, _)| (game.to_string(), info.pid))
}

/// Like [`game_launch`], for the first rule that matches a running process.
pub fn game_launch_in(config: &Config, processes: &[WindowInfo]) -> Option<(String, u32)> {
    matching_process(config, processes).map(|(game, _, p)| (game.to_string(), p.pid))
}

/// Rule for a window using its most specific identifier: Steam App ID, then a Windows `.exe`
/// name, then the window class, then the native executable name. An `.exe` name that a rule
/// in `config` already uses gets its folder too (`Sands of Time/gpp.exe`), so the two games
/// stay apart.
pub fn rule_for_window(config: &Config, w: &WindowInfo, profile: String) -> Rule {
    let (kind, value) = if let Some(id) = &w.steam_app_id {
        (RuleKind::SteamAppId, id.clone())
    } else if w.exe.to_ascii_lowercase().ends_with(".exe") {
        (RuleKind::Executable, exe_with_folder_if_taken(config, w))
    } else if !w.class.is_empty() {
        (RuleKind::WindowClass, w.class.clone())
    } else {
        (RuleKind::Executable, w.exe.clone())
    };
    Rule::new(kind, value, profile)
}

/// `w`'s exe name, or its folder and name when a rule in `config` already names that exe.
fn exe_with_folder_if_taken(config: &Config, w: &WindowInfo) -> String {
    let taken = config.games.iter().flat_map(|g| &g.rules).any(|r| r.kind == RuleKind::Executable && r.value.trim().eq_ignore_ascii_case(&w.exe));
    let folder = w.path.rsplit(['/', '\\']).nth(1).filter(|f| !f.is_empty() && !f.ends_with(':'));
    match folder {
        Some(folder) if taken => format!("{folder}/{}", w.exe),
        _ => w.exe.clone(),
    }
}

/// Whether a rule matches any of these windows or processes.
pub fn rule_matches_any(rule: &Rule, seen: &[WindowInfo]) -> bool {
    seen.iter().any(|w| rule_matches(rule.kind, &rule.value, w))
}

/// How closely the best of `rules` that matches one of `seen` picks it out (higher is closer),
/// or `None` when none matches.
pub fn match_specificity(rules: &[Rule], seen: &[WindowInfo]) -> Option<u8> {
    rules.iter().filter(|r| rule_matches_any(r, seen)).map(specificity).max()
}

#[cfg(test)]
mod tests;
