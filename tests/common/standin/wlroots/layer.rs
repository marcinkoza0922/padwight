//! Just enough surface handling for the overlay process to run against the stand-in: a
//! `wl_compositor`, `wl_shm` and `zwlr_layer_shell_v1` that answer every request, send the
//! configure a compositor would, and keep a list of the layer surfaces made and the output each
//! asked for. The overlay's choice of monitor is that output.

use std::sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicU32, Ordering},
};

use wayland_protocols_wlr::layer_shell::v1::server::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};
use wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_callback::WlCallback,
        wl_compositor::{self, WlCompositor},
        wl_region::{self, WlRegion},
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
        wl_surface::{self, WlSurface},
    },
};

use super::State;

/// A layer surface the client made.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerSurface {
    /// The output it asked for (an index into the specs the server was started with); `None`
    /// leaves the choice to the compositor.
    pub output: Option<usize>,
    pub namespace: String,
}

/// What the stand-in remembers of a `wl_surface`.
#[derive(Default)]
pub struct SurfaceData {
    callbacks: Mutex<Vec<WlCallback>>,
    buffer: Mutex<Option<WlBuffer>>,
    layer: Mutex<Option<(ZwlrLayerSurfaceV1, Arc<LayerData>)>>,
}

/// A layer surface's requested size, and the output its configure is sized for.
pub struct LayerData {
    requested: Mutex<(u32, u32)>,
    configured: Mutex<Option<(u32, u32)>>,
    output: Option<usize>,
}

static SERIAL: AtomicU32 = AtomicU32::new(1);

impl GlobalDispatch<WlCompositor, ()> for State {
    fn bind(_: &mut Self, _: &DisplayHandle, _: &Client, resource: New<WlCompositor>, _: &(), init: &mut DataInit<'_, Self>) {
        init.init(resource, ());
    }
}

impl Dispatch<WlCompositor, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &WlCompositor, request: wl_compositor::Request, _: &(), _: &DisplayHandle, init: &mut DataInit<'_, Self>) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                init.init(id, Arc::new(SurfaceData::default()));
            }
            wl_compositor::Request::CreateRegion { id } => {
                init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlRegion, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &WlRegion, _: wl_region::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl Dispatch<WlCallback, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &WlCallback, _: wayland_server::protocol::wl_callback::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl GlobalDispatch<WlShm, ()> for State {
    fn bind(_: &mut Self, _: &DisplayHandle, _: &Client, resource: New<WlShm>, _: &(), init: &mut DataInit<'_, Self>) {
        let shm = init.init(resource, ());
        shm.format(wl_shm::Format::Argb8888);
        shm.format(wl_shm::Format::Xrgb8888);
    }
}

impl Dispatch<WlShm, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &WlShm, request: wl_shm::Request, _: &(), _: &DisplayHandle, init: &mut DataInit<'_, Self>) {
        if let wl_shm::Request::CreatePool { id, .. } = request {
            init.init(id, ());
        }
    }
}

impl Dispatch<WlShmPool, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &WlShmPool, request: wl_shm_pool::Request, _: &(), _: &DisplayHandle, init: &mut DataInit<'_, Self>) {
        if let wl_shm_pool::Request::CreateBuffer { id, .. } = request {
            init.init(id, ());
        }
    }
}

impl Dispatch<WlBuffer, ()> for State {
    fn request(_: &mut Self, _: &Client, _: &WlBuffer, _: wl_buffer::Request, _: &(), _: &DisplayHandle, _: &mut DataInit<'_, Self>) {}
}

impl Dispatch<WlSurface, Arc<SurfaceData>> for State {
    fn request(state: &mut Self, _: &Client, _: &WlSurface, request: wl_surface::Request, data: &Arc<SurfaceData>, _: &DisplayHandle, init: &mut DataInit<'_, Self>) {
        match request {
            wl_surface::Request::Attach { buffer, .. } => *data.buffer.lock().unwrap_or_else(PoisonError::into_inner) = buffer,
            wl_surface::Request::Frame { callback } => {
                let callback = init.init(callback, ());
                data.callbacks.lock().unwrap_or_else(PoisonError::into_inner).push(callback);
            }
            wl_surface::Request::Commit => state.commit(data),
            _ => {}
        }
    }
}

impl State {
    /// Fires the frame callbacks committed since the last tick: the next "refresh".
    pub(super) fn tick(&mut self) {
        for callback in self.frame_callbacks.drain(..) {
            callback.done(0);
        }
    }

    /// What a compositor does at a commit: configure a layer surface whose size is not what it
    /// last configured, release the buffer (it is "shown" at once), and queue the frame
    /// callbacks for the next tick.
    fn commit(&mut self, data: &SurfaceData) {
        if let Some((layer, info)) = &*data.layer.lock().unwrap_or_else(PoisonError::into_inner) {
            let (w, h) = *info.requested.lock().unwrap_or_else(PoisonError::into_inner);
            let screen = self.outputs.get(info.output.unwrap_or(0)).map_or((0, 0), |o| (o.width as u32, o.height as u32));
            let size = (if w == 0 { screen.0 } else { w }, if h == 0 { screen.1 } else { h });
            let mut configured = info.configured.lock().unwrap_or_else(PoisonError::into_inner);
            if *configured != Some(size) {
                *configured = Some(size);
                layer.configure(SERIAL.fetch_add(1, Ordering::Relaxed), size.0, size.1);
            }
        }
        if let Some(buffer) = data.buffer.lock().unwrap_or_else(PoisonError::into_inner).take() {
            buffer.release();
        }
        for callback in data.callbacks.lock().unwrap_or_else(PoisonError::into_inner).drain(..) {
            self.frame_callbacks.push(callback);
        }
    }
}

impl GlobalDispatch<ZwlrLayerShellV1, ()> for State {
    fn bind(_: &mut Self, _: &DisplayHandle, _: &Client, resource: New<ZwlrLayerShellV1>, _: &(), init: &mut DataInit<'_, Self>) {
        init.init(resource, ());
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for State {
    fn request(state: &mut Self, _: &Client, _: &ZwlrLayerShellV1, request: zwlr_layer_shell_v1::Request, _: &(), _: &DisplayHandle, init: &mut DataInit<'_, Self>) {
        if let zwlr_layer_shell_v1::Request::GetLayerSurface { id, surface, output, namespace, .. } = request {
            let output = output.and_then(|o| o.data::<usize>().copied());
            state.layer_surfaces.lock().unwrap_or_else(PoisonError::into_inner).push(LayerSurface { output, namespace });
            let info = Arc::new(LayerData { requested: Mutex::new((0, 0)), configured: Mutex::new(None), output });
            let layer = init.init(id, info.clone());
            if let Some(data) = surface.data::<Arc<SurfaceData>>() {
                *data.layer.lock().unwrap_or_else(PoisonError::into_inner) = Some((layer, info));
            }
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, Arc<LayerData>> for State {
    fn request(_: &mut Self, _: &Client, _: &ZwlrLayerSurfaceV1, request: zwlr_layer_surface_v1::Request, data: &Arc<LayerData>, _: &DisplayHandle, _: &mut DataInit<'_, Self>) {
        if let zwlr_layer_surface_v1::Request::SetSize { width, height } = request {
            *data.requested.lock().unwrap_or_else(PoisonError::into_inner) = (width, height);
        }
    }
}
