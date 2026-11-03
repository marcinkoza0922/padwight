//! The overlay object the GNOME Shell and Cinnamon extensions export at
//! `io.github.marcinkoza0922.Padwight.Shell` `/Overlay`: `SetBuffer`, `Present`, `Geometry` and
//! `Hide`. It records the calls, so a test can check what the overlay process handed over.

use std::{
    fs::File,
    sync::{Arc, Mutex, PoisonError},
};

use zbus::zvariant::OwnedFd;

/// One call the overlay process made.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// The picture's size in device pixels, and the length of the memfd it is in.
    SetBuffer { width: i32, height: i32, bytes: u64 },
    Present,
    Geometry,
    Hide,
}

#[derive(Clone, Default)]
pub struct OverlayLog {
    calls: Arc<Mutex<Vec<Call>>>,
    /// What `Geometry` answers: the focused window's monitor, logical size and scale.
    monitor: Arc<Mutex<(i32, i32, f64)>>,
}

impl OverlayLog {
    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn set_monitor(&self, width: i32, height: i32, scale: f64) {
        *self.monitor.lock().unwrap_or_else(PoisonError::into_inner) = (width, height, scale);
    }

    fn push(&self, call: Call) {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push(call);
    }
}

pub struct Overlay(OverlayLog);

impl Overlay {
    pub fn new(log: OverlayLog) -> Self {
        if *log.monitor.lock().unwrap_or_else(PoisonError::into_inner) == (0, 0, 0.0) {
            log.set_monitor(1920, 1080, 1.0);
        }
        Self(log)
    }
}

#[zbus::interface(name = "io.github.marcinkoza0922.Padwight.Overlay")]
impl Overlay {
    fn set_buffer(&self, fd: OwnedFd, width: i32, height: i32) {
        let file = File::from(std::os::fd::OwnedFd::from(fd));
        let bytes = file.metadata().map_or(0, |m| m.len());
        self.0.push(Call::SetBuffer { width, height, bytes });
    }

    fn present(&self) {
        self.0.push(Call::Present);
    }

    fn geometry(&self) -> (i32, i32, f64) {
        self.0.push(Call::Geometry);
        *self.0.monitor.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn hide(&self) {
        self.0.push(Call::Hide);
    }
}
