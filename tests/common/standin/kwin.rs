//! KWin's scripting interface: `isScriptLoaded`, `loadScript` and `unloadScript` on
//! `/Scripting`, and `run` on the loaded script. The daemon's JavaScript is not executed; the
//! stand-in checks that the file is there and hooks `windowActivated`, then does what the script
//! does: reports the active window when run, and again on every focus change.

use std::{
    sync::{Arc, Mutex, PoisonError},
    thread,
};

use zbus::{blocking::Connection, fdo};

use super::{
    Compositor, Window,
    bus::{SessionBus, report_focus},
};

#[derive(Default)]
struct State {
    /// The script's name, once loaded.
    loaded: Option<String>,
    running: bool,
    focused: Option<Window>,
    /// How many times a script has been loaded, for tests that expect exactly one.
    loads: u32,
}

type Shared = Arc<Mutex<State>>;

fn lock(state: &Shared) -> std::sync::MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Scripting {
    state: Shared,
}

#[zbus::interface(name = "org.kde.kwin.Scripting")]
impl Scripting {
    #[zbus(name = "isScriptLoaded")]
    fn is_script_loaded(&self, name: &str) -> bool {
        lock(&self.state).loaded.as_deref() == Some(name)
    }

    #[zbus(name = "loadScript")]
    fn load_script(&self, path: &str, name: &str) -> fdo::Result<i32> {
        let script = std::fs::read_to_string(path).map_err(|e| fdo::Error::FileNotFound(format!("{path}: {e}")))?;
        if !script.contains("workspace.windowActivated.connect") || !script.contains("WindowActivated") {
            return Err(fdo::Error::InvalidArgs("not a focus script".into()));
        }
        let mut state = lock(&self.state);
        state.loaded = Some(name.to_string());
        state.running = false;
        state.loads += 1;
        Ok(1)
    }

    #[zbus(name = "unloadScript")]
    fn unload_script(&self, name: &str) -> bool {
        let mut state = lock(&self.state);
        let was = state.loaded.as_deref() == Some(name);
        if was {
            state.loaded = None;
            state.running = false;
        }
        was
    }
}

struct Script {
    state: Shared,
    out: Connection,
}

#[zbus::interface(name = "org.kde.kwin.Script")]
impl Script {
    #[zbus(name = "run")]
    fn run(&self) {
        let (state, out) = (self.state.clone(), self.out.clone());
        // The script reports from its own thread; the daemon is waiting for this reply.
        thread::spawn(move || {
            let focused = {
                let mut s = lock(&state);
                if s.loaded.is_none() {
                    return;
                }
                s.running = true;
                s.focused.clone()
            };
            report_focus(&out, focused.as_ref());
        });
    }
}

pub struct Kwin {
    state: Shared,
    out: Connection,
    _serve: Connection,
}

impl Kwin {
    pub fn start(bus: &SessionBus) -> Self {
        let state = Shared::default();
        let out = bus.connect(None);
        let serve = zbus::blocking::connection::Builder::address(bus.address.as_str())
            .unwrap()
            .name("org.kde.KWin")
            .unwrap()
            .serve_at("/Scripting", Scripting { state: state.clone() })
            .unwrap()
            .serve_at("/Scripting/Script1", Script { state: state.clone(), out: out.clone() })
            .unwrap()
            .build()
            .expect("serving the KWin stand-in");
        Self { state, out, _serve: serve }
    }

    /// How many times the daemon has loaded its script.
    pub fn loads(&self) -> u32 {
        lock(&self.state).loads
    }

    pub fn script_running(&self) -> bool {
        lock(&self.state).running
    }

    fn set(&self, window: Option<&Window>) {
        let running = {
            let mut s = lock(&self.state);
            s.focused = window.cloned();
            s.running
        };
        if running {
            report_focus(&self.out, window);
        }
    }
}

impl Compositor for Kwin {
    fn focus(&self, window: &Window) {
        self.set(Some(window));
    }

    fn unfocus(&self) {
        self.set(None);
    }

    fn close(&self, _window: &Window) {
        self.set(None);
    }
}
