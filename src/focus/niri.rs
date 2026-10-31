//! niri: asks its IPC socket for the event stream, which lists the windows (with their pids)
//! and says which has focus.

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::{FocusEvent, Notify, Reported, report};

pub struct Niri(UnixStream);

/// `$NIRI_SOCKET`, then any `niri.*.sock` in the runtime dir (a systemd user service doesn't
/// necessarily inherit the compositor's environment).
fn candidates() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::env::var_os("NIRI_SOCKET").map(PathBuf::from).into_iter().collect();
    if let Some(entries) = dirs::runtime_dir().and_then(|dir| fs::read_dir(dir).ok()) {
        paths.extend(entries.flatten().map(|e| e.path()).filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("niri.") && n.ends_with(".sock"))
        }));
    }
    paths
}

pub fn connect() -> Result<Niri> {
    candidates()
        .iter()
        .find_map(|path| UnixStream::connect(path).ok())
        .map(Niri)
        .context("no niri IPC socket")
}

/// The windows niri has told us about, by id, and which one has focus.
#[derive(Default)]
struct Windows {
    all: HashMap<u64, Value>,
    focused: Option<u64>,
}

impl Windows {
    /// Applies one event; true when the focused window may have changed.
    fn apply(&mut self, event: &Value) -> bool {
        if let Some(list) = event["WindowsChanged"]["windows"].as_array() {
            self.all = list.iter().filter_map(|w| Some((w["id"].as_u64()?, w.clone()))).collect();
            self.focused = list.iter().find(|w| w["is_focused"] == true).and_then(|w| w["id"].as_u64());
            true
        } else if let window @ Value::Object(_) = &event["WindowOpenedOrChanged"]["window"] {
            let Some(id) = window["id"].as_u64() else { return false };
            self.all.insert(id, window.clone());
            if window["is_focused"] == true {
                self.focused = Some(id);
            }
            // A title change reports the window again, which is not news unless it is the one in front.
            self.focused == Some(id)
        } else if let Some(id) = event["WindowClosed"]["id"].as_u64() {
            self.all.remove(&id);
            self.focused == Some(id) && self.focused.take().is_some()
        } else if event.get("WindowFocusChanged").is_some() {
            self.focused = event["WindowFocusChanged"]["id"].as_u64();
            true
        } else {
            false
        }
    }

    fn focused(&self) -> Option<Reported> {
        reported(self.all.get(&self.focused?)?)
    }
}

impl Niri {
    /// Reports the focused window now and on every change, until the connection ends.
    pub fn follow(mut self, notify: &Notify) -> Result<()> {
        self.0.write_all(b"\"EventStream\"\n")?;
        let mut lines = BufReader::new(self.0).lines();
        let reply: Value = serde_json::from_str(&lines.next().context("niri closed the socket")??)?;
        if reply.get("Ok").is_none() {
            bail!("niri refused the event stream: {reply}");
        }
        let mut windows = Windows::default();
        for line in lines {
            if windows.apply(&serde_json::from_str(&line?)?) {
                match windows.focused() {
                    Some(window) => report(notify, window),
                    None => notify(FocusEvent::Unfocused),
                }
            }
        }
        bail!("niri closed the event stream")
    }
}

/// A window's class is its `app_id`. A window without a pid is skipped.
fn reported(window: &Value) -> Option<Reported> {
    let pid = u32::try_from(window["pid"].as_u64()?).ok()?;
    Some(Reported {
        class: window["app_id"].as_str().unwrap_or_default().into(),
        title: window["title"].as_str().unwrap_or_default().into(),
        pid,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::json;

    use super::*;

    fn window(id: u64, app_id: &str, focused: bool) -> Value {
        json!({"id": id, "pid": id * 10, "app_id": app_id, "title": "t", "is_focused": focused})
    }

    #[test]
    fn tracks_focus_through_opens_changes_and_closes() {
        let mut w = Windows::default();
        assert!(w.apply(&json!({"WindowsChanged": {"windows": [window(1, "foot", true), window(2, "steam_app_7", false)]}})));
        assert_eq!(w.focused().unwrap().class, "foot");
        assert!(w.apply(&json!({"WindowFocusChanged": {"id": 2}})));
        assert_eq!((w.focused().unwrap().class.as_str(), w.focused().unwrap().pid), ("steam_app_7", 20));
        assert!(!w.apply(&json!({"WindowOpenedOrChanged": {"window": window(1, "foot", false)}})), "a background change is not news");
        assert!(w.apply(&json!({"WindowOpenedOrChanged": {"window": window(3, "kitty", true)}})));
        assert_eq!(w.focused().unwrap().class, "kitty");
        assert!(w.apply(&json!({"WindowClosed": {"id": 3}})));
        assert!(w.focused().is_none());
        assert!(!w.apply(&json!({"WindowClosed": {"id": 1}})), "closing a background window is not news");
        assert!(w.apply(&json!({"WindowFocusChanged": {"id": null}})));
        assert!(!w.apply(&json!({"WorkspacesChanged": {"workspaces": []}})));
    }

    #[test]
    fn follow_reports_the_stream() {
        let (client, mut server) = UnixStream::pair().unwrap();
        let lines = [
            json!({"Ok": "Handled"}),
            json!({"WindowsChanged": {"windows": [window(1, "foot", true)]}}),
            json!({"WindowFocusChanged": {"id": null}}),
        ];
        for line in lines {
            writeln!(server, "{line}").unwrap();
        }
        server.shutdown(std::net::Shutdown::Write).unwrap();

        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let notify: Notify = Arc::new(move |ev| {
            sink.lock().unwrap().push(match ev {
                FocusEvent::Focused(info) => info.class,
                _ => "none".into(),
            });
        });
        assert!(Niri(client).follow(&notify).is_err(), "ends when the socket closes");
        assert_eq!(*seen.lock().unwrap(), ["foot", "none"]);
    }
}
