//! The protocol's events, dispatched to the `Window`.

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop,
    globals::GlobalListContents,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_callback::{self, WlCallback},
        wl_compositor::WlCompositor,
        wl_region::WlRegion,
        wl_registry::{self, WlRegistry},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::wp::{
    fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{self, WpFractionalScaleV1},
    },
    viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

use super::{shm::BufferId, window::Window};
use crate::overlay::timing::{timing_log, unix_ns};

impl Dispatch<ZwlrLayerSurfaceV1, ()> for Window {
    fn event(window: &mut Window, layer: &ZwlrLayerSurfaceV1, event: zwlr_layer_surface_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Window>) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                // Acked before anything is committed for it; a later configure replaces what
                // this one asked for.
                layer.ack_configure(serial);
                window.todo = window.sizing.configure(width, height);
                timing_log(&format!("configure {} {width}x{height} {:?} {:?}", unix_ns(), window.sizing.target(), window.todo));
            }
            zwlr_layer_surface_v1::Event::Closed => window.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<WlBuffer, BufferId> for Window {
    fn event(window: &mut Window, _: &WlBuffer, event: wl_buffer::Event, id: &BufferId, _: &Connection, _: &QueueHandle<Window>) {
        if let wl_buffer::Event::Release = event
            && let Some(pool) = window.pool.as_mut().filter(|p| p.id == id.pool)
        {
            pool.slots.released(id.slot);
        }
    }
}

impl Dispatch<WlCallback, u64> for Window {
    fn event(window: &mut Window, _: &WlCallback, event: wl_callback::Event, id: &u64, _: &Connection, _: &QueueHandle<Window>) {
        if let wl_callback::Event::Done { callback_data } = event {
            // The compositor's time is in milliseconds, from a clock of its own choosing.
            let current = window.pacer.done(*id);
            timing_log(&format!("callback {} {callback_data} {}", unix_ns(), if current { "current" } else { "stale" }));
        }
    }
}

impl Dispatch<WlShm, ()> for Window {
    fn event(window: &mut Window, _: &WlShm, event: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Window>) {
        if let wl_shm::Event::Format { format } = event {
            timing_log(&format!("shm-format {} {format:?}", unix_ns()));
            if let WEnum::Value(format) = format {
                window.formats.offered(format);
            }
        }
    }
}

impl Dispatch<WpFractionalScaleV1, ()> for Window {
    fn event(window: &mut Window, _: &WpFractionalScaleV1, event: wp_fractional_scale_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Window>) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            window.scale120 = Some(scale);
            timing_log(&format!("scale {} {scale}/120", unix_ns()));
        }
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for Window {
    fn event(_: &mut Window, _: &WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Window>) {}
}

// Enter, leave and the preferred buffer scale: step 5.4 reads them.
delegate_noop!(Window: ignore WlSurface);
delegate_noop!(Window: WlCompositor);
delegate_noop!(Window: WlShmPool);
delegate_noop!(Window: WlRegion);
delegate_noop!(Window: ZwlrLayerShellV1);
delegate_noop!(Window: WpFractionalScaleManagerV1);
delegate_noop!(Window: WpViewporter);
delegate_noop!(Window: WpViewport);
