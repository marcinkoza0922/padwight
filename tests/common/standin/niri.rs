//! niri's IPC socket: `"EventStream"` answers `{"Ok":"Handled"}`, then the window list, then
//! window and focus events, one JSON object per line.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::Path,
    sync::{Arc, Mutex, PoisonError},
};

use serde_json::{Value, json};

use super::{Compositor, Server, Window};

#[derive(Default)]
struct State {
    windows: Vec<Window>,
    focused: Option<u64>,
    subscribers: Vec<UnixStream>,
}

pub struct Niri {
    state: Arc<Mutex<State>>,
    _server: Server,
}

fn window_json(w: &Window, focused: bool) -> Value {
    json!({"id": w.id, "pid": w.pid, "app_id": w.class, "title": w.title, "is_focused": focused})
}

impl State {
    fn send(&mut self, event: &Value) {
        let line = format!("{event}\n");
        self.subscribers.retain_mut(|s| s.write_all(line.as_bytes()).is_ok());
    }

    fn all(&self) -> Value {
        json!({"WindowsChanged": {"windows": self.windows.iter().map(|w| window_json(w, self.focused == Some(w.id))).collect::<Vec<_>>()}})
    }
}

impl Niri {
    /// Listens on `socket`; point the daemon's `NIRI_SOCKET` at it.
    pub fn start(socket: &Path) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        let server = Server::start(socket, move |mut stream| {
            let Ok(clone) = stream.try_clone() else { return };
            let mut line = String::new();
            if BufReader::new(clone).read_line(&mut line).is_err() || line.trim() != "\"EventStream\"" {
                let _ = writeln!(stream, "{}", json!({"Err": "unsupported request"}));
                return;
            }
            let mut state = shared.lock().unwrap_or_else(PoisonError::into_inner);
            let hello = format!("{}\n{}\n", json!({"Ok": "Handled"}), state.all());
            if stream.write_all(hello.as_bytes()).is_ok() {
                state.subscribers.push(stream);
            }
        })
        .expect("binding the niri socket");
        Self { state, _server: server }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Compositor for Niri {
    fn focus(&self, window: &Window) {
        let mut state = self.lock();
        if !state.windows.iter().any(|w| w.id == window.id) {
            state.windows.push(window.clone());
            state.send(&json!({"WindowOpenedOrChanged": {"window": window_json(window, false)}}));
        }
        state.focused = Some(window.id);
        state.send(&json!({"WindowFocusChanged": {"id": window.id}}));
    }

    fn unfocus(&self) {
        let mut state = self.lock();
        state.focused = None;
        state.send(&json!({"WindowFocusChanged": {"id": null}}));
    }

    fn close(&self, window: &Window) {
        let mut state = self.lock();
        state.windows.retain(|w| w.id != window.id);
        if state.focused == Some(window.id) {
            state.focused = None;
        }
        state.send(&json!({"WindowClosed": {"id": window.id}}));
    }
}
