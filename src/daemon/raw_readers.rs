//! Other programs reading a managed controller through its raw HID node (/dev/hidrawN). The grab
//! takes only the input node, so Steam or an SDL game reading the raw node gets every press as
//! well as the virtual pad. The daemon looks for them now and then, logs each one it finds, and
//! reports them to the settings window, whose fix is `dist/72-padwight-hidraw.rules`.

use std::time::Instant;

use super::Daemon;
use crate::ipc::RawReaders;

/// How often the scan looks. Programs open the raw node as the controller connects, so a few
/// seconds late is soon enough, and a scan reads every open file of every process.
#[cfg(target_os = "linux")]
const CHECK_EVERY: std::time::Duration = std::time::Duration::from_secs(6);

/// How long the toast asking to open the settings stays up.
#[cfg(target_os = "linux")]
const PROMPT_TIME: std::time::Duration = std::time::Duration::from_secs(8);

#[derive(Default)]
#[cfg_attr(not(target_os = "linux"), expect(dead_code, reason = "only Linux has raw HID nodes to watch"))]
pub(super) struct Watch {
    next: Option<Instant>,
    found: Vec<RawReaders>,
    /// The controllers and programs already prompted about, by name.
    announced: std::collections::HashSet<(String, String)>,
}

impl Watch {
    pub(super) fn found(&self) -> Vec<RawReaders> {
        self.found.clone()
    }
}

impl Daemon {
    /// Looks for programs reading a managed controller's raw node, when it's time to.
    #[cfg(target_os = "linux")]
    pub(super) fn check_raw_readers(&mut self, now: Instant) {
        if self.raw_readers.next.is_some_and(|t| t > now) {
            return;
        }
        self.raw_readers.next = Some(now + CHECK_EVERY);
        let pads: Vec<(String, Vec<std::path::PathBuf>)> = self
            .devices
            .values()
            .filter_map(|d| Some((d.name.clone(), linux::open_nodes(d.parent.as_deref()?))))
            .filter(|(_, nodes)| !nodes.is_empty())
            .collect();
        let found: Vec<RawReaders> = if pads.is_empty() {
            Vec::new()
        } else {
            let open = linux::openers(std::path::Path::new("/proc"), std::process::id());
            pads.into_iter()
                .filter_map(|(controller, nodes)| {
                    let mut programs: Vec<String> = nodes.iter().filter_map(|n| open.get(n)).flatten().cloned().collect();
                    programs.sort();
                    programs.dedup();
                    (!programs.is_empty()).then_some(RawReaders { controller, programs })
                })
                .collect()
        };
        for r in found.iter().filter(|r| !self.raw_readers.found.contains(r)) {
            crate::monitor::log!("{} (through its raw HID node); install dist/72-padwight-hidraw.rules", r.warning());
        }
        // Each program is prompted about once a run: a Bluetooth pad that drops and comes back
        // would otherwise prompt again every time.
        let new: Vec<RawReaders> = found
            .iter()
            .filter_map(|r| {
                let programs: Vec<String> = r.programs.iter().filter(|p| self.raw_readers.announced.insert((r.controller.clone(), (*p).clone()))).cloned().collect();
                (!programs.is_empty()).then(|| RawReaders { controller: r.controller.clone(), programs })
            })
            .collect();
        self.raw_readers.found = found;
        if !new.is_empty() {
            self.prompt_raw_readers(&new);
        }
    }

    /// Asks the user to open the settings window, whose banner says how to fix it: in a desktop
    /// notification with a button for it, and in a toast for whoever is in a fullscreen game.
    #[cfg(target_os = "linux")]
    fn prompt_raw_readers(&mut self, new: &[RawReaders]) {
        let first = &new[0];
        let summary = format!("{} reads {} past Padwight", first.who(), first.controller);
        let body = new.iter().map(RawReaders::warning).chain(["Open Padwight's settings to fix it.".to_owned()]).collect::<Vec<_>>().join("\n");
        let tx = self.tx.clone();
        linux::notify(summary.clone(), body, move || {
            let _ = tx.send(super::Msg::Tray(crate::tray::Command::OpenSettings));
        });
        self.toast = Some(crate::info::Toast::lasting(vec![summary, "Open Padwight's settings to fix it".into()], Instant::now(), PROMPT_TIME));
        self.broadcast_overlay();
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::{
        collections::HashMap,
        fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
    };

    /// The raw nodes of the HID device at `parent` (in /sys) that some program may open. The rule
    /// leaves them mode 0000, which only root gets past.
    pub(super) fn open_nodes(parent: &Path) -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(parent.join("hidraw")) else { return Vec::new() };
        entries
            .filter_map(|e| Some(Path::new("/dev").join(e.ok()?.file_name())))
            .filter(|node| fs::metadata(node).is_ok_and(|m| m.permissions().mode() & 0o777 != 0))
            .collect()
    }

    /// Which processes under `proc` (/proc) have each file open, by process name, leaving out
    /// `me`. Only this user's processes can be looked into, which is what a desktop's Steam is.
    pub(super) fn openers(proc: &Path, me: u32) -> HashMap<PathBuf, Vec<String>> {
        let mut open: HashMap<PathBuf, Vec<String>> = HashMap::new();
        let Ok(entries) = fs::read_dir(proc) else { return open };
        for entry in entries.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
            if pid == me {
                continue;
            }
            let Ok(fds) = fs::read_dir(entry.path().join("fd")) else { continue };
            let targets: Vec<PathBuf> = fds.flatten().filter_map(|fd| fs::read_link(fd.path()).ok()).filter(|t| t.to_str().is_some_and(|t| t.starts_with("/dev/hidraw"))).collect();
            if targets.is_empty() {
                continue;
            }
            let name = fs::read_to_string(entry.path().join("comm")).map(|s| s.trim().to_owned()).unwrap_or_else(|_| pid.to_string());
            for target in targets {
                open.entry(target).or_default().push(name.clone());
            }
        }
        open
    }

    /// Shows a desktop notification with an "Open settings" button, and runs `open` if it's
    /// pressed. A thread waits for the answer, so a desktop without notifications costs a log line.
    pub(super) fn notify(summary: String, body: String, open: impl FnOnce() + Send + 'static) {
        std::thread::spawn(move || {
            if let Err(e) = notify_and_wait(&summary, &body, open) {
                crate::monitor::log!("cannot show a desktop notification: {e:#}");
            }
        });
    }

    fn notify_and_wait(summary: &str, body: &str, open: impl FnOnce()) -> anyhow::Result<()> {
        use zbus::zvariant::Value;
        let conn = zbus::blocking::Connection::session()?;
        let proxy = zbus::blocking::Proxy::new(&conn, "org.freedesktop.Notifications", "/org/freedesktop/Notifications", "org.freedesktop.Notifications")?;
        // Subscribed before the notification exists, so a quick click isn't missed.
        let signals = proxy.receive_all_signals()?;
        // The desktop entry's name, which its icon has too.
        let app = "io.github.marcinkoza0922.Padwight";
        let hints = HashMap::from([("desktop-entry", Value::from(app)), ("urgency", Value::from(2u8))]);
        let actions = ["default", "Open settings", "open", "Open settings"];
        let id: u32 = proxy.call("Notify", &("Padwight", 0u32, app, summary, body, &actions[..], hints, 0i32))?;
        for signal in signals {
            let member = signal.header().member().map(ToString::to_string);
            match member.as_deref() {
                Some("ActionInvoked") => {
                    let (of, _action): (u32, String) = signal.body().deserialize()?;
                    if of == id {
                        open();
                        return Ok(());
                    }
                }
                Some("NotificationClosed") => {
                    let (of, _reason): (u32, u32) = signal.body().deserialize()?;
                    if of == id {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn finds_who_has_a_raw_node_open_but_not_itself() {
            let proc = std::env::temp_dir().join(format!("padwight-raw-readers-{}", std::process::id()));
            let _ = fs::remove_dir_all(&proc);
            for (pid, name, target) in [("10", "steam", "/dev/hidraw7"), ("11", "padwight", "/dev/hidraw7"), ("12", "brave", "/dev/null")] {
                let fd = proc.join(pid).join("fd");
                fs::create_dir_all(&fd).unwrap();
                std::os::unix::fs::symlink(target, fd.join("3")).unwrap();
                fs::write(proc.join(pid).join("comm"), format!("{name}\n")).unwrap();
            }
            fs::create_dir_all(proc.join("self")).unwrap();
            let open = openers(&proc, 11);
            fs::remove_dir_all(&proc).unwrap();
            assert_eq!(open.len(), 1);
            assert_eq!(open[Path::new("/dev/hidraw7")], ["steam"]);
        }
    }
}
