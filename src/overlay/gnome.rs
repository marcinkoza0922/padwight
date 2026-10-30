//! The overlay on GNOME, which has no layer-shell: the frames are drawn here, as everywhere
//! on Linux, but handed to the Shell extension (`gnome-extension/…/overlay.js`), which shows
//! them as a click-through actor above everything, fullscreen games included.
//!
//! Each frame is painted by the overlay's own renderer into a picture kept between frames, so
//! only what changed is repainted, then written to a memfd and sent over D-Bus with its size.
//! The extension maps the memfd, so the memfd is not written again once sent; two alternate.
//! Without frame callbacks it animates on a 16 ms timer, as the Windows overlay does.
//!
//! Run on GNOME Shell 51 in the test VM; not yet over a fullscreen game or with several monitors.

use std::{
    fs::File,
    os::{
        fd::{AsFd, AsRawFd, FromRawFd},
        unix::fs::FileExt,
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tiny_skia::PixmapMut;
use zbus::{blocking::Connection, zvariant::Fd};

use super::{
    render::{Point, Screen},
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
            conn.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "NameHasOwner", &("org.gnome.Shell",))
                .and_then(|reply| Ok(reply.body().deserialize::<bool>()?))
                .unwrap_or(false)
        })
}

/// The monitor the extension covers: logical size and scale.
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

    fn present(&self, memfd: &File, (width, height): (u32, u32)) -> Result<()> {
        let stride = width * 4;
        self.call("Present", &(Fd::from(memfd.as_fd()), width as i32, height as i32, stride as i32))?;
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
        let timeout = if overlay.shown() && overlay.busy(Instant::now()) { Some(STEP) } else { None };
        wait(&feed, timeout)?;
    }
}

/// What the loop keeps between looks.
#[derive(Default)]
struct View {
    screen: Screen,
    /// The share of the monitor's scale drawn at, in hundredths (`Resolution::percent`).
    percent: u32,
    visible: bool,
    /// The picture, which always holds the last frame painted (age 1), and its size.
    picture: Vec<u8>,
    size: (u32, u32),
    /// The two memfds frames go out in: the extension may still hold the last one.
    memfds: [Option<File>; 2],
    sent: usize,
}

impl View {
    fn hidden(&mut self) {
        self.visible = false;
        self.picture = Vec::new();
        self.size = (0, 0);
        self.memfds = [None, None];
        self.screen.forget();
        super::render::raster::shrink();
    }

    fn show(&mut self, shell: &Shell, overlay: &mut Overlay, now: Instant) -> Result<()> {
        if !overlay.shown() {
            if self.visible {
                shell.hide()?;
                self.hidden();
            }
            return Ok(());
        }
        let monitor = shell.monitor()?;
        let scale = (monitor.scale as f32 * self.percent.clamp(1, 100) as f32 / 100.0).max(0.1);
        let device = ((monitor.size.0 as f32 * scale).round() as u32, (monitor.size.1 as f32 * scale).round() as u32);
        let ops = overlay.screen_ops(now, Point::new(monitor.size.0 as f32, monitor.size.1 as f32));
        // A new picture is empty; one kept holds the last frame, which is age 1.
        let age = if self.size == device {
            1
        } else {
            self.picture = vec![0; device.0 as usize * device.1 as usize * 4];
            self.size = device;
            self.memfds = [None, None];
            0
        };
        let mut canvas = PixmapMut::from_bytes(&mut self.picture, device.0, device.1).context("the overlay's picture has no size")?;
        let damage = self.screen.paint(&ops, scale, &mut canvas, age);
        if damage.is_empty() && self.visible {
            return Ok(());
        }
        self.sent = 1 - self.sent;
        let memfd = match self.memfds[self.sent].take() {
            Some(file) => file,
            None => memfd(self.picture.len())?,
        };
        // Whole rows are written: the extension uploads the whole picture anyway.
        memfd.write_all_at(&self.picture, 0).context("writing the frame")?;
        shell.present(&memfd, device)?;
        self.memfds[self.sent] = Some(memfd);
        self.visible = true;
        overlay.presented(Instant::now());
        Ok(())
    }
}

fn memfd(len: usize) -> Result<File> {
    // SAFETY: memfd_create takes a NUL-terminated name and flags, and returns a new fd or -1.
    let fd = unsafe { libc::memfd_create(c"padwight-overlay".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("memfd_create");
    }
    // SAFETY: `fd` is a new descriptor nothing else owns.
    let file = unsafe { File::from_raw_fd(fd) };
    file.set_len(len as u64).context("sizing the frame's memfd")?;
    Ok(file)
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
