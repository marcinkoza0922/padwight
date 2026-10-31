//! The overlay on GNOME, which has no layer-shell: the frames are drawn here, as everywhere
//! on Linux, but handed to the Shell extension (`gnome-extension/…/overlay.js`), which shows
//! them as a click-through actor above everything, fullscreen games included.
//!
//! The picture is one memfd, mapped here and in the extension (`SetBuffer`, again whenever the
//! size changes). Each frame is painted by the overlay's own renderer straight into it, kept
//! between frames so only what changed is repainted, and `Present` tells the extension to show
//! it. Nothing is sent per frame: Clutter takes a whole texture whatever was damaged. The
//! extension reads the picture only while `Present` runs, which we wait out before painting again.
//! Without frame callbacks it animates on a 16 ms timer.
//!
//! Run on GNOME Shell 51 in the test VM; over a fullscreen window, on two monitors and at a fractional scale.

use std::{
    fs::File,
    os::fd::{AsFd, AsRawFd, FromRawFd},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tiny_skia::PixmapMut;
use zbus::{blocking::Connection, zvariant::Fd};

use super::{
    render::{Point, Screen},
    timing::{timing_log, unix_ns},
    ui::Overlay,
    wayland::feed::Feed,
};

const BUS_NAME: &str = "io.github.marcinkoza0922.Padwight.Shell";
const OBJECT_PATH: &str = "/Overlay";
const INTERFACE: &str = "io.github.marcinkoza0922.Padwight.Overlay";
/// About a refresh at 60 Hz.
const STEP: Duration = Duration::from_millis(16);

/// Whether this session is GNOME's, where the overlay can't be a window of its own. The
/// desktop's name isn't always in the daemon's environment, so GNOME Shell on the bus counts too.
pub(super) fn wanted() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|desktop| desktop.split(':').any(|part| part.eq_ignore_ascii_case("gnome")))
        || Connection::session().is_ok_and(|conn| {
            let reply = conn.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "NameHasOwner", &("org.gnome.Shell",));
            reply.is_ok_and(|reply| reply.body().deserialize::<bool>().unwrap_or(false))
        })
}

/// The monitor the extension covers (the focused window's): logical size and scale, which may
/// be fractional.
struct Monitor {
    size: (u32, u32),
    scale: f64,
}

struct Shell {
    conn: Connection,
}

impl Shell {
    fn call<B>(&self, method: &str, body: &B) -> Result<zbus::message::Message>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        Ok(self.conn.call_method(Some(BUS_NAME), OBJECT_PATH, Some(INTERFACE), method, body)?)
    }

    fn monitor(&self) -> Result<Monitor> {
        let (width, height, scale): (i32, i32, f64) = self.call("Geometry", &())?.body().deserialize()?;
        Ok(Monitor { size: (width.max(1) as u32, height.max(1) as u32), scale: scale.max(1.0) })
    }

    fn set_buffer(&self, picture: &Picture) -> Result<()> {
        let (width, height) = picture.size;
        self.call("SetBuffer", &(Fd::from(picture.file.as_fd()), width as i32, height as i32))?;
        Ok(())
    }

    /// Returns once the extension has taken the picture, so it may be painted on again.
    fn present(&self) -> Result<()> {
        self.call("Present", &())?;
        Ok(())
    }

    fn hide(&self) -> Result<()> {
        self.call("Hide", &())?;
        Ok(())
    }
}

pub(super) fn run() -> Result<()> {
    super::render::text::warm();
    let shell = Shell { conn: Connection::session().context("connecting to the session bus")? };
    let feed = Feed::connect()?;
    let mut overlay = Overlay::new();
    let mut view = View::default();
    // When the next frame is due while something moves: a step after the last one began, not
    // after it was done, so the work doesn't stretch the step.
    let mut due: Option<Instant> = None;
    loop {
        let arrived = feed.drain();
        for frame in arrived.frames {
            view.percent = frame.lite.resolution.percent();
            overlay.receive(frame);
        }
        if arrived.gone {
            // The daemon went away: nothing left to draw for.
            let _ = shell.hide();
            return Ok(());
        }
        let now = Instant::now();
        overlay.prune(now);
        if let Err(e) = view.show(&shell, &mut overlay, now) {
            crate::monitor::log!("overlay: {e:#}");
            // The extension isn't there (yet): look again at the next frame.
            view.hidden();
        }
        let now = Instant::now();
        due = if overlay.shown() && overlay.busy(now) { Some(due.map_or(now + STEP, |at| (at + STEP).max(now))) } else { None };
        wait(&feed, due.map(|at| at.saturating_duration_since(now)))?;
    }
}

/// The picture: one memfd of `size` device pixels, mapped.
struct Picture {
    file: File,
    map: *mut u8,
    len: usize,
    size: (u32, u32),
}

impl Picture {
    fn new((width, height): (u32, u32)) -> Result<Picture> {
        let len = width as usize * height as usize * 4;
        // SAFETY: memfd_create takes a NUL-terminated name and flags, and returns a new fd or -1.
        let fd = unsafe { libc::memfd_create(c"padwight-overlay".as_ptr(), libc::MFD_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error()).context("memfd_create");
        }
        // SAFETY: `fd` is a new descriptor nothing else owns.
        let file = unsafe { File::from_raw_fd(fd) };
        file.set_len(len as u64).context("sizing the picture's memfd")?;
        // SAFETY: a shared mapping of the whole memfd, which is `len` bytes; unmapped on drop.
        let map = unsafe { libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, file.as_raw_fd(), 0) };
        if map == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error()).context("mapping the picture");
        }
        Ok(Picture { file, map: map.cast(), len, size: (width, height) })
    }

    /// The pixels, to paint into. The extension reads them only during `Shell::present`, so
    /// nothing else touches them while this borrow lives.
    fn pixels(&mut self) -> &mut [u8] {
        // SAFETY: the mapping is `len` bytes, valid until drop, and borrowed uniquely here.
        unsafe { std::slice::from_raw_parts_mut(self.map, self.len) }
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: unmapping the mapping made in `new`, which nothing points into any more.
        unsafe { libc::munmap(self.map.cast(), self.len) };
    }
}

/// What the loop keeps between looks.
#[derive(Default)]
struct View {
    screen: Screen,
    /// The share of the monitor's scale drawn at, in hundredths (`Resolution::percent`).
    percent: u32,
    /// The monitor, asked for each time the overlay appears.
    monitor: Option<Monitor>,
    /// The picture, which always holds the last frame painted (age 1) while shown.
    picture: Option<Picture>,
    visible: bool,
}

impl View {
    fn hidden(&mut self) {
        self.visible = false;
        self.monitor = None;
        self.picture = None;
        self.screen.forget();
        super::render::raster::shrink();
    }

    fn show(&mut self, shell: &Shell, overlay: &mut Overlay, now: Instant) -> Result<()> {
        let started = Instant::now();
        if !overlay.shown() {
            if self.visible {
                shell.hide()?;
                self.hidden();
            }
            return Ok(());
        }
        if self.monitor.is_none() {
            self.monitor = Some(shell.monitor()?);
        }
        let monitor = self.monitor.as_ref().expect("asked for above");
        let (logical, scale) = (monitor.size, (monitor.scale as f32 * self.percent.clamp(1, 100) as f32 / 100.0).max(0.1));
        let device = ((logical.0 as f32 * scale).round() as u32, (logical.1 as f32 * scale).round() as u32);
        let ops = overlay.screen_ops(now, Point::new(logical.0 as f32, logical.1 as f32));
        let asked = started.elapsed();
        // A new picture is clear; one kept holds the last frame, which is age 1.
        let age = match &self.picture {
            Some(picture) if picture.size == device => 1,
            _ => {
                let picture = Picture::new(device)?;
                shell.set_buffer(&picture)?;
                self.picture = Some(picture);
                0
            }
        };
        let picture = self.picture.as_mut().expect("made above");
        let mut canvas = PixmapMut::from_bytes(picture.pixels(), device.0, device.1).context("the overlay's picture has no size")?;
        let damage = self.screen.paint(&ops, scale, &mut canvas, age);
        let painted = started.elapsed();
        if damage.is_empty() && self.visible {
            timing_log(&format!("gnome-unchanged {} {}", unix_ns(), painted.as_nanos()));
            return Ok(());
        }
        shell.present()?;
        self.visible = true;
        let sent = started.elapsed();
        // `monitor` is the layout (and the Geometry call when the overlay has just appeared),
        // `paint` ends the painting, and `call` the Present call, which returns once the
        // extension has taken the frame.
        timing_log(&format!(
            "gnome {} {} {}x{} monitor {} paint {} write 0 call {} damage {} {}",
            unix_ns(),
            sent.as_nanos(),
            device.0,
            device.1,
            asked.as_nanos(),
            (painted - asked).as_nanos(),
            (sent - painted).as_nanos(),
            damage.patches().len(),
            damage.patches().iter().map(|p| i64::from(p.width()) * i64::from(p.height())).sum::<i64>(),
        ));
        overlay.presented(Instant::now());
        Ok(())
    }
}

/// Sleeps up to `timeout` (for ever when none) until the daemon sends something.
fn wait(feed: &Feed, timeout: Option<Duration>) -> Result<()> {
    let mut fd = libc::pollfd { fd: feed.fd().as_raw_fd(), events: libc::POLLIN, revents: 0 };
    let ms = timeout.map_or(-1, |t| i32::try_from(t.as_millis()).unwrap_or(i32::MAX));
    // SAFETY: one pollfd, valid for the call.
    let n = unsafe { libc::poll(&mut fd, 1, ms) };
    if n < 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::Interrupted {
            return Err(e).context("poll");
        }
    }
    Ok(())
}
