//! COSMIC (cosmic-comp): has no `wlr-foreign-toplevel-management`, so this follows the window
//! list of `ext-foreign-toplevel-list` (titles and app ids) and asks `cosmic-toplevel-info` for
//! each window's state, which says which one is activated. Like the wlroots tracker, it can't
//! tell which process owns a window, so only class rules match here.

use std::{collections::HashMap, os::unix::net::UnixStream};

use anyhow::{Context, Result, bail};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    backend::ObjectId,
    event_created_child,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_output, wl_registry},
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

use self::protocol::{
    zcosmic_toplevel_handle_v1::{self, ZcosmicToplevelHandleV1},
    zcosmic_toplevel_info_v1::{self, ZcosmicToplevelInfoV1},
};
use super::{
    Notify,
    wlroots::{Change, Windows, announce, socket_candidates},
};

/// The protocol's `cosmic-toplevel-info`, generated from `protocols/` at build time.
mod protocol {
    // The generated code names untyped objects `wayland_client::ObjectId`, which the crate only reaches through `backend`.
    mod wayland_client {
        pub use ::wayland_client::{backend::ObjectId, *};
    }
    use self::wayland_client::protocol::*;
    use wayland_protocols::ext::foreign_toplevel_list::v1::client::*;

    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        use wayland_protocols::ext::foreign_toplevel_list::v1::client::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/cosmic-toplevel-info-unstable-v1.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("protocols/cosmic-toplevel-info-unstable-v1.xml");
}

/// `activated` in the protocol's state enum.
const ACTIVATED: u32 = 2;

/// The state the Wayland event loop dispatches into.
struct State {
    info: ZcosmicToplevelInfoV1,
    windows: Windows<ObjectId>,
    /// The extension object of each window, kept to release it when the window closes.
    cosmic: HashMap<ObjectId, ZcosmicToplevelHandleV1>,
    /// Windows whose state changed since the info object's last `done`.
    dirty: Vec<ObjectId>,
    changes: Vec<Change>,
}

impl State {
    /// Where focus ended up after the events read so far, if it changed. Moving focus reports
    /// the old window leaving before the new one arriving, and reporting both would leave the
    /// game for a moment.
    fn settled(&mut self) -> Option<Change> {
        let last = self.changes.pop();
        self.changes.clear();
        last
    }
}

/// A connection to the compositor's toplevel list and info.
pub struct Cosmic {
    queue: wayland_client::EventQueue<State>,
    state: State,
}

/// Connects and checks that the compositor offers both protocols.
pub fn connect() -> Result<Cosmic> {
    let stream = socket_candidates()
        .iter()
        .find_map(|path| UnixStream::connect(path).ok())
        .context("no Wayland socket")?;
    let conn = Connection::from_socket(stream)?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let list: std::result::Result<ExtForeignToplevelListV1, _> = globals.bind(&qh, 1..=1, ());
    let info: std::result::Result<ZcosmicToplevelInfoV1, _> = globals.bind(&qh, 2..=3, ());
    let (Ok(_list), Ok(info)) = (list, info) else {
        bail!("the compositor doesn't offer cosmic-toplevel-info");
    };
    let mut state = State { info, windows: Windows::default(), cosmic: HashMap::new(), dirty: Vec::new(), changes: Vec::new() };
    queue.roundtrip(&mut state)?;
    Ok(Cosmic { queue, state })
}

impl Cosmic {
    /// Reports the focused window now and on every change, until the connection ends.
    pub fn follow(mut self, notify: &Notify) -> Result<()> {
        loop {
            announce(notify, self.state.settled());
            self.queue.blocking_dispatch(&mut self.state)?;
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(_: &mut Self, _: &wl_output::WlOutput, _: wl_output::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            let id = toplevel.id();
            state.windows.new_window(id.clone());
            let handle = state.info.get_cosmic_toplevel(&toplevel, qh, id.clone());
            state.cosmic.insert(id, handle);
        }
    }

    event_created_child!(State, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = handle.id();
        let change = match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                state.windows.title(&id, title);
                None
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                state.windows.app_id(&id, app_id);
                None
            }
            ext_foreign_toplevel_handle_v1::Event::Done => state.windows.done(&id),
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                handle.destroy();
                if let Some(cosmic) = state.cosmic.remove(&id) {
                    cosmic.destroy();
                }
                state.windows.closed(&id)
            }
            _ => None,
        };
        state.changes.extend(change);
    }
}

impl Dispatch<ZcosmicToplevelInfoV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZcosmicToplevelInfoV1,
        event: zcosmic_toplevel_info_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // A window's states arrive in a batch, closed by this event.
        if let zcosmic_toplevel_info_v1::Event::Done = event {
            for id in std::mem::take(&mut state.dirty) {
                let change = state.windows.done(&id);
                state.changes.extend(change);
            }
        }
    }
}

impl Dispatch<ZcosmicToplevelHandleV1, ObjectId> for State {
    fn event(
        state: &mut Self,
        _: &ZcosmicToplevelHandleV1,
        event: zcosmic_toplevel_handle_v1::Event,
        window: &ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zcosmic_toplevel_handle_v1::Event::State { state: flags } = event {
            let activated = flags.as_chunks::<4>().0.iter().any(|b| u32::from_ne_bytes(*b) == ACTIVATED);
            state.windows.state(window, activated);
            if !state.dirty.contains(window) {
                state.dirty.push(window.clone());
            }
        }
    }
}
