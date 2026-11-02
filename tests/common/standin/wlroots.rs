//! A small Wayland server with two `wl_output`s and `wlr-foreign-toplevel-management`, as a
//! wlroots compositor (labwc, Wayfire, river) offers them. It tells each client about every
//! window, which one is activated, and which output it is on: what the daemon's tracker and the
//! overlay's choice of monitor (`overlay/wayland/monitors.rs`) read. It runs on its own thread
//! and takes its orders over a channel, so a test calls it like any other [`Compositor`].
//!
//! The protocol carries no pid, so the daemon matches class rules only: a window here needs its
//! `class` set to what the game's rule says.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        Mutex, PoisonError,
        mpsc::{Receiver, Sender, channel},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use wayland_protocols_wlr::layer_shell::v1::server::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols_wlr::foreign_toplevel::v1::server::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};
use wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, ListeningSocket, New, Resource,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_shm::WlShm,
    },
};

mod layer;

pub use layer::LayerSurface;

use super::{Compositor, Window};

/// `activated` in the protocol's state enum.
const ACTIVATED: u32 = 2;

/// An output's logical size and scale, as `wl_output` will say it.
#[derive(Clone, Copy, Debug)]
pub struct OutputSpec {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: i32,
}

enum Command {
    Focus(Window, usize),
    Unfocus,
    Close(u64),
}

struct Win {
    window: Window,
    output: usize,
    handles: Vec<ZwlrForeignToplevelHandleV1>,
}

struct State {
    outputs: Vec<OutputSpec>,
    /// Each `wl_output` a client has bound, with the output it stands for.
    bound: Vec<(WlOutput, usize)>,
    managers: Vec<ZwlrForeignToplevelManagerV1>,
    windows: Vec<Win>,
    active: Option<u64>,
    /// Frame callbacks waiting for the next refresh.
    frame_callbacks: Vec<wayland_server::protocol::wl_callback::WlCallback>,
    /// The layer surfaces clients have made, in order.
    layer_surfaces: Arc<Mutex<Vec<LayerSurface>>>,
}

pub struct Wlroots {
    layers: Arc<Mutex<Vec<LayerSurface>>>,
    commands: Sender<Command>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    socket: PathBuf,
}

impl Wlroots {
    /// Listens on `socket` (its name should start with `wayland-` so the daemon finds it in the
    /// runtime dir; also give the daemon `WAYLAND_DISPLAY` with that name).
    pub fn start(socket: &Path, outputs: &[OutputSpec]) -> Self {
        let listener = ListeningSocket::bind_absolute(socket.to_path_buf()).expect("binding the Wayland socket");
        let (commands, receiver) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let outputs = outputs.to_vec();
        let flag = stop.clone();
        let layers = Arc::new(Mutex::new(Vec::new()));
        let shared = layers.clone();
        let thread = thread::spawn(move || serve(&listener, &outputs, &receiver, &flag, shared));
        Self { layers, commands, stop, thread: Some(thread), socket: socket.to_path_buf() }
    }

    /// Focuses `window`, which is on output `output` (an index into the specs given to `start`).
    pub fn focus_on(&self, window: &Window, output: usize) {
        let _ = self.commands.send(Command::Focus(window.clone(), output));
    }

    /// The layer surfaces made so far, each with the output it asked for: the overlay process
    /// makes a new one whenever it shows after being hidden, on the focused window's output.
    pub fn layer_surfaces(&self) -> Vec<LayerSurface> {
        self.layers.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

impl Compositor for Wlroots {
    fn focus(&self, window: &Window) {
        self.focus_on(window, 0);
    }

    fn unfocus(&self) {
        let _ = self.commands.send(Command::Unfocus);
    }

    fn close(&self, window: &Window) {
        let _ = self.commands.send(Command::Close(window.id));
    }
}

impl Drop for Wlroots {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(listener: &ListeningSocket, outputs: &[OutputSpec], commands: &Receiver<Command>, stop: &AtomicBool, layer_surfaces: Arc<Mutex<Vec<LayerSurface>>>) {
    let mut display = wayland_server::Display::<State>::new().expect("creating the Wayland display");
    let mut state = State { outputs: outputs.to_vec(), bound: Vec::new(), managers: Vec::new(), windows: Vec::new(), active: None, frame_callbacks: Vec::new(), layer_surfaces };
    let handle = display.handle();
    for index in 0..outputs.len() {
        handle.create_global::<State, WlOutput, usize>(4, index);
    }
    handle.create_global::<State, ZwlrForeignToplevelManagerV1, ()>(3, ());
    handle.create_global::<State, WlCompositor, ()>(6, ());
    handle.create_global::<State, WlShm, ()>(1, ());
    handle.create_global::<State, ZwlrLayerShellV1, ()>(4, ());
    while !stop.load(Ordering::Relaxed) {
        while let Ok(Some(stream)) = listener.accept() {
            let _ = display.handle().insert_client(stream, Arc::new(Nothing));
        }
        while let Ok(command) = commands.try_recv() {
            state.apply(&display.handle(), command);
        }
        let _ = display.dispatch_clients(&mut state);
        let _ = display.flush_clients();
        state.tick();
        thread::sleep(Duration::from_millis(5));
    }
}

struct Nothing;

impl wayland_server::backend::ClientData for Nothing {}

impl State {
    fn apply(&mut self, dh: &DisplayHandle, command: Command) {
        match command {
            Command::Focus(window, output) => {
                if !self.windows.iter().any(|w| w.window.id == window.id) {
                    let handles = self.managers.clone().iter().filter_map(|m| self.new_handle(dh, m, &window, output)).collect();
                    self.windows.push(Win { window: window.clone(), output, handles });
                } else if let Some(w) = self.windows.iter_mut().find(|w| w.window.id == window.id) {
                    w.output = output;
                }
                self.active = Some(window.id);
            }
            Command::Unfocus => self.active = None,
            Command::Close(id) => {
                if let Some(at) = self.windows.iter().position(|w| w.window.id == id) {
                    for handle in &self.windows[at].handles {
                        handle.closed();
                    }
                    self.windows.remove(at);
                }
                if self.active == Some(id) {
                    self.active = None;
                }
            }
        }
        self.send_states();
    }

    /// Tells every client which window is activated. The old one is deactivated first, as
    /// compositors do (the daemon must settle on the last).
    fn send_states(&self) {
        for w in &self.windows {
            if self.active != Some(w.window.id) {
                for handle in &w.handles {
                    handle.state(Vec::new());
                    handle.done();
                }
            }
        }
        for w in self.windows.iter().filter(|w| self.active == Some(w.window.id)) {
            for handle in &w.handles {
                handle.state(ACTIVATED.to_ne_bytes().to_vec());
                handle.done();
            }
        }
    }

    /// A toplevel handle for `window` in the client that owns `manager`, with its details.
    fn new_handle(&self, dh: &DisplayHandle, manager: &ZwlrForeignToplevelManagerV1, window: &Window, output: usize) -> Option<ZwlrForeignToplevelHandleV1> {
        let client = manager.client()?;
        let handle = client.create_resource::<ZwlrForeignToplevelHandleV1, (), State>(dh, manager.version(), ()).ok()?;
        manager.toplevel(&handle);
        handle.title(window.title.clone());
        handle.app_id(window.class.clone());
        if let Some((out, _)) = self.bound.iter().find(|(o, i)| *i == output && o.client().is_some_and(|c| c.id() == client.id())) {
            handle.output_enter(out);
        }
        Some(handle)
    }
}

impl GlobalDispatch<WlOutput, usize> for State {
    fn bind(state: &mut Self, _: &DisplayHandle, _: &Client, resource: New<WlOutput>, index: &usize, init: &mut DataInit<'_, Self>) {
        let output = init.init(resource, *index);
        let spec = state.outputs[*index];
        output.geometry(spec.x, spec.y, 0, 0, wl_output::Subpixel::Unknown, "Padwight".into(), format!("Stand-in {index}"), wl_output::Transform::Normal);
        output.mode(wl_output::Mode::Current | wl_output::Mode::Preferred, spec.width * spec.scale, spec.height * spec.scale, 60_000);
        if output.version() >= 2 {
            output.scale(spec.scale);
        }
        if output.version() >= 4 {
            output.name(format!("STANDIN-{index}"));
        }
        if output.version() >= 2 {
            output.done();
        }
        state.bound.push((output, *index));
    }
}

impl Dispatch<WlOutput, usize> for State {
    fn request(_: &mut Self, _: &Client, _: &WlOutput, _: wl_output::Request, _: &usize, _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl GlobalDispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn bind(state: &mut Self, dh: &DisplayHandle, _: &Client, resource: New<ZwlrForeignToplevelManagerV1>, _: &(), init: &mut DataInit<'_, Self>) {
        let manager = init.init(resource, ());
        // A client that binds late hears about the windows already open.
        let known: Vec<(Window, usize)> = state.windows.iter().map(|w| (w.window.clone(), w.output)).collect();
        let mut handles = Vec::new();
        for (window, output) in &known {
            handles.push(state.new_handle(dh, &manager, window, *output));
        }
        for (w, handle) in state.windows.iter_mut().zip(handles) {
            w.handles.extend(handle);
        }
        state.managers.push(manager);
        state.send_states();
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn request(state: &mut Self, _: &Client, manager: &ZwlrForeignToplevelManagerV1, request: zwlr_foreign_toplevel_manager_v1::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            manager.finished();
            state.managers.retain(|m| m.id() != manager.id());
        }
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &ZwlrForeignToplevelHandleV1, _: zwlr_foreign_toplevel_handle_v1::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}
