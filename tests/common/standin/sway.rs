//! Sway's i3-IPC socket: answers `subscribe` and `get_tree`, and sends `window` events.

use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    sync::{Arc, Mutex, PoisonError},
};

use serde_json::{Value, json};

use super::{Compositor, Server, Window};

const MAGIC: &[u8; 6] = b"i3-ipc";
const SUBSCRIBE: u32 = 2;
const GET_TREE: u32 = 4;
const EVENT_WINDOW: u32 = 0x8000_0003;

#[derive(Default)]
struct State {
    windows: Vec<Window>,
    focused: Option<u64>,
    subscribers: Vec<UnixStream>,
}

pub struct Sway {
    state: Arc<Mutex<State>>,
    _server: Server,
}

fn frame(kind: u32, json: &Value) -> Vec<u8> {
    let body = json.to_string();
    let mut msg = Vec::new();
    msg.extend_from_slice(MAGIC);
    msg.extend_from_slice(&u32::try_from(body.len()).unwrap().to_ne_bytes());
    msg.extend_from_slice(&kind.to_ne_bytes());
    msg.extend_from_slice(body.as_bytes());
    msg
}

fn node(w: &Window, focused: bool) -> Value {
    json!({"id": w.id, "pid": w.pid, "app_id": w.class, "name": w.title, "focused": focused, "nodes": [], "floating_nodes": []})
}

/// A tree like Sway's: root, output, workspace with the windows; an empty workspace beside it.
fn tree(state: &State) -> Value {
    let windows: Vec<Value> = state.windows.iter().map(|w| node(w, state.focused == Some(w.id))).collect();
    let empty_focused = state.focused.is_none();
    json!({"id": 1, "focused": false, "nodes": [{"id": 2, "focused": false, "nodes": [
        {"id": 3, "name": "1", "focused": false, "nodes": windows, "floating_nodes": []},
        {"id": 4, "name": "2", "focused": empty_focused, "nodes": [], "floating_nodes": []},
    ], "floating_nodes": []}], "floating_nodes": []})
}

fn handle(mut stream: UnixStream, state: &Mutex<State>) {
    loop {
        let mut head = [0u8; 14];
        if stream.read_exact(&mut head).is_err() || &head[..6] != MAGIC {
            return;
        }
        let len = u32::from_ne_bytes([head[6], head[7], head[8], head[9]]) as usize;
        let kind = u32::from_ne_bytes([head[10], head[11], head[12], head[13]]);
        let mut payload = vec![0; len];
        if stream.read_exact(&mut payload).is_err() {
            return;
        }
        let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
        let ok = match kind {
            SUBSCRIBE => {
                let ok = stream.write_all(&frame(SUBSCRIBE, &json!({"success": true}))).is_ok();
                if let Ok(clone) = stream.try_clone() {
                    state.subscribers.push(clone);
                }
                ok
            }
            GET_TREE => stream.write_all(&frame(GET_TREE, &tree(&state))).is_ok(),
            _ => stream.write_all(&frame(kind, &json!({"success": false}))).is_ok(),
        };
        if !ok {
            return;
        }
    }
}

impl State {
    fn send(&mut self, kind: u32, event: &Value) {
        let msg = frame(kind, event);
        self.subscribers.retain_mut(|s| s.write_all(&msg).is_ok());
    }
}

impl Sway {
    /// Listens on `socket`; point the daemon's `SWAYSOCK` at it.
    pub fn start(socket: &Path) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        let server = Server::start(socket, move |stream| handle(stream, &shared)).expect("binding the Sway socket");
        Self { state, _server: server }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Compositor for Sway {
    fn focus(&self, window: &Window) {
        let mut state = self.lock();
        if !state.windows.iter().any(|w| w.id == window.id) {
            state.windows.push(window.clone());
        }
        state.focused = Some(window.id);
        state.send(EVENT_WINDOW, &json!({"change": "focus", "container": node(window, true)}));
    }

    fn unfocus(&self) {
        let mut state = self.lock();
        state.focused = None;
        // Focus moving to an empty workspace: a container with no pid.
        state.send(EVENT_WINDOW, &json!({"change": "focus", "container": {"id": 4, "name": "2", "nodes": []}}));
    }

    fn close(&self, window: &Window) {
        let mut state = self.lock();
        state.windows.retain(|w| w.id != window.id);
        if state.focused == Some(window.id) {
            state.focused = None;
        }
        // Closing the focused window sends only `close`; the daemon asks for the tree again.
        state.send(EVENT_WINDOW, &json!({"change": "close", "container": node(window, false)}));
    }
}
