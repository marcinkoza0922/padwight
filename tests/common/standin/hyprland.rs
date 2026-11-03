//! Hyprland's two sockets in an instance dir: `.socket.sock` answers commands (`j/activewindow`),
//! `.socket2.sock` streams `name>>data` events.

use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use serde_json::json;

use super::{Compositor, Server, Window};

#[derive(Default)]
struct State {
    focused: Option<Window>,
    subscribers: Vec<UnixStream>,
}

pub struct Hyprland {
    state: Arc<Mutex<State>>,
    signature: String,
    _command: Server,
    _events: Server,
}

impl State {
    fn send(&mut self, line: &str) {
        self.subscribers.retain_mut(|s| s.write_all(line.as_bytes()).is_ok());
    }
}

impl Hyprland {
    /// Makes `<runtime>/hypr/<signature>/` with both sockets. The daemon finds it through its
    /// runtime dir; also give it `HYPRLAND_INSTANCE_SIGNATURE` ([`Self::signature`]).
    pub fn start(runtime: &Path, signature: &str) -> Self {
        let dir: PathBuf = runtime.join("hypr").join(signature);
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        let command = Server::start(&dir.join(".socket.sock"), move |mut stream| {
            let mut buf = [0u8; 4096];
            let Ok(n) = stream.read(&mut buf) else { return };
            let reply = match &buf[..n] {
                b"j/activewindow" => match &shared.lock().unwrap_or_else(PoisonError::into_inner).focused {
                    Some(w) => json!({"address": format!("0x{:x}", w.id), "mapped": true, "class": w.class, "title": w.title, "pid": w.pid}).to_string(),
                    None => "{}".into(),
                },
                _ => "unknown request".into(),
            };
            // Like Hyprland, answer and close.
            let _ = stream.write_all(reply.as_bytes());
        })
        .expect("binding the Hyprland command socket");
        let shared = state.clone();
        let events = Server::start(&dir.join(".socket2.sock"), move |stream| {
            shared.lock().unwrap_or_else(PoisonError::into_inner).subscribers.push(stream);
        })
        .expect("binding the Hyprland event socket");
        Self { state, signature: signature.into(), _command: command, _events: events }
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Compositor for Hyprland {
    fn focus(&self, window: &Window) {
        let mut state = self.lock();
        state.focused = Some(window.clone());
        state.send(&format!("activewindow>>{},{}\nactivewindowv2>>{:x}\n", window.class, window.title, window.id));
    }

    fn unfocus(&self) {
        let mut state = self.lock();
        state.focused = None;
        state.send("activewindow>>,\nactivewindowv2>>,\n");
    }

    fn close(&self, window: &Window) {
        let mut state = self.lock();
        if state.focused.as_ref().is_some_and(|w| w.id == window.id) {
            state.focused = None;
            state.send("closewindow>>0\nactivewindowv2>>,\n");
        } else {
            state.send("closewindow>>0\n");
        }
    }
}
