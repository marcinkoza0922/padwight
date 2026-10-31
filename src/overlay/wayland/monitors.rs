//! Which monitor the focused window is on, so the overlay's surface can ask for it. A layer
//! surface made without an output lands wherever the compositor likes: labwc puts it on the
//! first monitor whatever has focus. `wlr-foreign-toplevel-management` says which outputs each
//! window is on and which one is activated; compositors without it keep choosing for us.

use std::collections::HashMap;

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
    globals::GlobalList,
    protocol::{
        wl_output::WlOutput,
        wl_registry::{self, WlRegistry},
    },
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

use super::window::Window;

/// `activated` in the protocol's state enum.
const ACTIVATED: u32 = 2;

#[derive(Default)]
struct Toplevel {
    /// From the `state` event, applied at `done` as the protocol asks.
    activated_pending: bool,
    activated: bool,
    /// The outputs the window is on, in the order it entered them.
    outputs: Vec<WlOutput>,
}

#[derive(Default)]
pub(super) struct Monitors {
    /// Kept alive: it is what sends the windows.
    _manager: Option<ZwlrForeignToplevelManagerV1>,
    /// Every output bound, by its global's name. The protocol only names outputs the client has.
    outputs: Vec<(u32, WlOutput)>,
    windows: HashMap<u32, Toplevel>,
}

impl Monitors {
    /// Binds every output, and the window list when the compositor offers one.
    pub(super) fn new(globals: &GlobalList, qh: &QueueHandle<Window>) -> Self {
        let outputs = globals.contents().with_list(|list| {
            list.iter()
                .filter(|g| g.interface == "wl_output")
                .map(|g| (g.name, globals.registry().bind(g.name, g.version.min(4), qh, ())))
                .collect()
        });
        Self { _manager: globals.bind(qh, 1..=3, ()).ok(), outputs, windows: HashMap::new() }
    }

    /// The output the activated window is on, if there is one and the compositor says.
    pub(super) fn focused(&self) -> Option<&WlOutput> {
        let window = self.windows.values().find(|w| w.activated)?;
        window.outputs.iter().rev().find(|o| self.outputs.iter().any(|(_, known)| known.id() == o.id()))
    }
}

impl Dispatch<WlRegistry, wayland_client::globals::GlobalListContents> for Window {
    fn event(
        window: &mut Window,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &wayland_client::globals::GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Window>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } if interface == "wl_output" => {
                window.monitors.outputs.push((name, registry.bind(name, version.min(4), qh, ())));
            }
            wl_registry::Event::GlobalRemove { name } => window.monitors.outputs.retain(|(n, _)| *n != name),
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for Window {
    fn event(
        window: &mut Window,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Window>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            window.monitors.windows.insert(toplevel.id().protocol_id(), Toplevel::default());
        }
    }

    event_created_child!(Window, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for Window {
    fn event(
        window: &mut Window,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Window>,
    ) {
        let id = handle.id().protocol_id();
        let windows = &mut window.monitors.windows;
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::State { state } => {
                if let Some(w) = windows.get_mut(&id) {
                    w.activated_pending = state.as_chunks::<4>().0.iter().any(|b| u32::from_ne_bytes(*b) == ACTIVATED);
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                if let Some(w) = windows.get_mut(&id) {
                    w.outputs.push(output);
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { output } => {
                if let Some(w) = windows.get_mut(&id) {
                    w.outputs.retain(|o| o.id() != output.id());
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                if let Some(w) = windows.get_mut(&id) {
                    w.activated = w.activated_pending;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                // The protocol asks for the handle to be destroyed once closed.
                handle.destroy();
                windows.remove(&id);
            }
            _ => {}
        }
    }
}

delegate_noop!(Window: ignore WlOutput);
