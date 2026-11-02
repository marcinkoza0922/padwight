//! cosmic-comp's window list: a Wayland server with `ext-foreign-toplevel-list` (titles and app
//! ids) and `cosmic-toplevel-info` (each window's state, which says which one is activated), the
//! two protocols the daemon's COSMIC tracker reads. Like the wlroots one it carries no pid, so a
//! game's rule has to be a class rule.

use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use wayland_protocols::ext::foreign_toplevel_list::v1::server::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, ListeningSocket, New, Resource};

use self::protocol::{
    zcosmic_toplevel_handle_v1::{self, ZcosmicToplevelHandleV1},
    zcosmic_toplevel_info_v1::{self, ZcosmicToplevelInfoV1},
};
use super::{Compositor, Window};

/// The protocol's `cosmic-toplevel-info`, generated from `protocols/` for the server side.
mod protocol {
    // The generated code names untyped objects `wayland_server::backend::ObjectId`.
    mod wayland_server {
        pub use ::wayland_server::*;
    }
    use self::wayland_server::protocol::*;
    use wayland_protocols::ext::foreign_toplevel_list::v1::server::*;

    pub mod __interfaces {
        use wayland_protocols::ext::foreign_toplevel_list::v1::server::__interfaces::*;
        use wayland_server::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("tests/fixtures/cosmic-toplevel-info-server.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_server_code!("tests/fixtures/cosmic-toplevel-info-server.xml");
}

/// `activated` in the protocol's state enum.
const ACTIVATED: u32 = 2;

enum Command {
    Focus(Window),
    Unfocus,
    Close(u64),
}

struct Win {
    window: Window,
    /// One per client that bound the list.
    foreign: Vec<ExtForeignToplevelHandleV1>,
    cosmic: Vec<ZcosmicToplevelHandleV1>,
}

#[derive(Default)]
struct State {
    lists: Vec<ExtForeignToplevelListV1>,
    infos: Vec<ZcosmicToplevelInfoV1>,
    windows: Vec<Win>,
    active: Option<u64>,
}

pub struct Cosmic {
    commands: Sender<Command>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Cosmic {
    /// Listens on `socket` (named `wayland-…` in the daemon's runtime dir, with `WAYLAND_DISPLAY`
    /// set to its name).
    pub fn start(socket: &Path) -> Self {
        let listener = ListeningSocket::bind_absolute(socket.to_path_buf()).expect("binding the Wayland socket");
        let (commands, receiver) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = thread::spawn(move || serve(&listener, &receiver, &flag));
        Self { commands, stop, thread: Some(thread) }
    }
}

impl Compositor for Cosmic {
    fn focus(&self, window: &Window) {
        let _ = self.commands.send(Command::Focus(window.clone()));
    }

    fn unfocus(&self) {
        let _ = self.commands.send(Command::Unfocus);
    }

    fn close(&self, window: &Window) {
        let _ = self.commands.send(Command::Close(window.id));
    }
}

impl Drop for Cosmic {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Nothing;

impl wayland_server::backend::ClientData for Nothing {}

fn serve(listener: &ListeningSocket, commands: &Receiver<Command>, stop: &AtomicBool) {
    let mut display = wayland_server::Display::<State>::new().expect("creating the Wayland display");
    let mut handle = display.handle();
    handle.create_global::<State, ExtForeignToplevelListV1, ()>(1, ());
    handle.create_global::<State, ZcosmicToplevelInfoV1, ()>(3, ());
    let mut state = State::default();
    while !stop.load(Ordering::Relaxed) {
        while let Ok(Some(stream)) = listener.accept() {
            let _ = handle.insert_client(stream, Arc::new(Nothing));
        }
        while let Ok(command) = commands.try_recv() {
            state.apply(&handle, command);
        }
        let _ = display.dispatch_clients(&mut state);
        let _ = display.flush_clients();
        thread::sleep(Duration::from_millis(5));
    }
}

impl State {
    fn apply(&mut self, dh: &DisplayHandle, command: Command) {
        match command {
            Command::Focus(window) => {
                if !self.windows.iter().any(|w| w.window.id == window.id) {
                    let foreign = self.lists.iter().filter_map(|list| new_foreign(dh, list, &window)).collect();
                    self.windows.push(Win { window: window.clone(), foreign, cosmic: Vec::new() });
                }
                self.active = Some(window.id);
            }
            Command::Unfocus => self.active = None,
            Command::Close(id) => {
                if let Some(at) = self.windows.iter().position(|w| w.window.id == id) {
                    for handle in &self.windows[at].foreign {
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

    /// Every window's state, then `done` on each info object.
    fn send_states(&self) {
        for w in &self.windows {
            let flags = if self.active == Some(w.window.id) { ACTIVATED.to_ne_bytes().to_vec() } else { Vec::new() };
            for cosmic in &w.cosmic {
                cosmic.state(flags.clone());
            }
        }
        for info in &self.infos {
            info.done();
        }
    }
}

/// An `ext_foreign_toplevel_handle_v1` for `window` in the client that owns `list`.
fn new_foreign(dh: &DisplayHandle, list: &ExtForeignToplevelListV1, window: &Window) -> Option<ExtForeignToplevelHandleV1> {
    let client = list.client()?;
    let handle = client.create_resource::<ExtForeignToplevelHandleV1, u64, State>(dh, list.version(), window.id).ok()?;
    list.toplevel(&handle);
    handle.title(window.title.clone());
    handle.app_id(window.class.clone());
    handle.done();
    Some(handle)
}

impl GlobalDispatch<ExtForeignToplevelListV1, ()> for State {
    fn bind(state: &mut Self, dh: &DisplayHandle, _: &Client, resource: New<ExtForeignToplevelListV1>, _: &(), init: &mut DataInit<'_, Self>) {
        let list = init.init(resource, ());
        for w in &mut state.windows {
            w.foreign.extend(new_foreign(dh, &list, &w.window));
        }
        state.lists.push(list);
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn request(state: &mut Self, _: &Client, list: &ExtForeignToplevelListV1, request: ext_foreign_toplevel_list_v1::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {
        if let ext_foreign_toplevel_list_v1::Request::Stop = request {
            list.finished();
            state.lists.retain(|l| l.id() != list.id());
        }
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, u64> for State {
    fn request(_: &mut Self, _: &Client, _: &ExtForeignToplevelHandleV1, _: ext_foreign_toplevel_handle_v1::Request, _: &u64, _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl GlobalDispatch<ZcosmicToplevelInfoV1, ()> for State {
    fn bind(state: &mut Self, _: &DisplayHandle, _: &Client, resource: New<ZcosmicToplevelInfoV1>, _: &(), init: &mut DataInit<'_, Self>) {
        state.infos.push(init.init(resource, ()));
    }
}

impl Dispatch<ZcosmicToplevelInfoV1, ()> for State {
    fn request(state: &mut Self, _: &Client, _: &ZcosmicToplevelInfoV1, request: zcosmic_toplevel_info_v1::Request, _: &(), _: &DisplayHandle, init: &mut DataInit<'_, Self>) {
        if let zcosmic_toplevel_info_v1::Request::GetCosmicToplevel { cosmic_toplevel, foreign_toplevel } = request {
            let id = foreign_toplevel.data::<u64>().copied().unwrap_or_default();
            let handle = init.init(cosmic_toplevel, id);
            let flags = if state.active == Some(id) { ACTIVATED.to_ne_bytes().to_vec() } else { Vec::new() };
            handle.state(flags);
            if let Some(w) = state.windows.iter_mut().find(|w| w.window.id == id) {
                w.cosmic.push(handle);
            }
            for info in &state.infos {
                info.done();
            }
        }
    }
}

impl Dispatch<ZcosmicToplevelHandleV1, u64> for State {
    fn request(_: &mut Self, _: &Client, _: &ZcosmicToplevelHandleV1, _: zcosmic_toplevel_handle_v1::Request, _: &u64, _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}
