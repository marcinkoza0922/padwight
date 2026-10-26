//! The overlay window on Linux without iced: a wlr-layer-shell surface of our own, drawn on the
//! CPU. Chosen with `PADWIGHT_OVERLAY=native` while it is being built; `layershell.rs` stays
//! the default until it can replace it.
//!
//! For now (step 5.1) the loop wakes on a short timer to take frames from the daemon and to
//! animate, and each frame goes out in a new buffer. Later steps bring a buffer pool (5.2),
//! drawing on frame callbacks (5.3), the scale (5.4) and an eventfd the reading thread wakes
//! the loop with (5.5).
//!
//! In the harness: `PADWIGHT_HARNESS_RELEASE=1 PADWIGHT_OVERLAY=native scripts/overlay-harness.sh
//! scripts/harness/pacing.sh <out>`. Besides the usual lines, the timing log has `surface <ns> ask
//! <target> <w>x<h>` for each request, `configure <ns> <w>x<h> <target> <commit>` for each configure
//! acked, and `commit <ns> <w>x<h> <why> <target>` for each buffer committed, so it shows that
//! every buffer goes on at the size last configured.

use std::{
    io::{BufRead, BufReader, Write},
    os::fd::{AsRawFd, BorrowedFd},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use wayland_client::{Connection, EventQueue, globals::registry_queue_init};

use super::{OverlayFrame, ui::Overlay};
use crate::ipc::{self, Request, UnixStream};

mod events;
mod picture;
mod shm;
mod sizing;
mod window;

use picture::{Screen, TestPicture};
use sizing::{Commit, Target};
use window::Window;

/// How often the loop wakes while something moves, as today's overlay does.
const ANIMATING: Duration = Duration::from_millis(16);
/// How often it looks for frames from the daemon otherwise (until step 5.5's eventfd).
const IDLE: Duration = Duration::from_millis(50);

pub(super) fn run() -> Result<()> {
    let conn = Connection::connect_to_env().context("the overlay window needs a Wayland session")?;
    let (globals, mut queue) = registry_queue_init::<Window>(&conn).context("reading the compositor's globals")?;
    let qh = queue.handle();
    let mut window = Window::new(&globals, &qh)?;
    let frames = daemon_frames()?;
    let mut overlay = Overlay::new();
    let mut screen = TestPicture;
    // When the next animation tick is due; Wayland events can wake the loop between ticks.
    let mut tick = Instant::now();
    loop {
        let mut dirty = false;
        loop {
            match frames.try_recv() {
                Ok(frame) => {
                    overlay.receive(frame);
                    dirty = true;
                }
                Err(TryRecvError::Empty) => break,
                // The daemon went away: nothing left to draw for.
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
        let now = Instant::now();
        let due = now >= tick;
        if due {
            tick = now + ANIMATING;
        }
        let busy = overlay.busy(now);
        overlay.prune(now);
        step(&mut window, &mut screen, &overlay, dirty || (busy && due), &qh)?;
        if window.closed {
            anyhow::bail!("the compositor closed the overlay's surface");
        }
        let awake = overlay.busy(now) || window.sizing.target() == Target::Screen;
        let timeout = if awake { tick.saturating_duration_since(Instant::now()) } else { IDLE };
        wait(&conn, &mut queue, &mut window, timeout)?;
    }
}

/// Brings the surface in line with the overlay: asks for the screen or the corner, puts on
/// what the last configure asked for, and draws the screen when it has changed.
fn step(window: &mut Window, screen: &mut dyn Screen, overlay: &Overlay, changed: bool, qh: &wayland_client::QueueHandle<Window>) -> Result<()> {
    let todo = std::mem::replace(&mut window.todo, Commit::Nothing);
    match window.sizing.want(overlay.shown()) {
        Commit::Grow => window.grow(),
        Commit::Shrink { clear } => {
            window.shrink(clear, qh)?;
            window.forget();
        }
        _ => {}
    }
    // A configure answers the request it came for; one asked since then waits for its own.
    match todo {
        Commit::Blank if window.sizing.target() == Target::Corner => window.blank(qh)?,
        Commit::Draw(_) => window.draw(screen, overlay, qh)?,
        _ if changed && overlay.shown() => window.draw(screen, overlay, qh)?,
        _ => {}
    }
    Ok(())
}

/// Sends what's queued, waits up to `timeout` for the compositor, and handles what it sent.
fn wait(conn: &Connection, queue: &mut EventQueue<Window>, window: &mut Window, timeout: Duration) -> Result<()> {
    queue.dispatch_pending(window)?;
    conn.flush()?;
    if let Some(guard) = queue.prepare_read()
        && readable(guard.connection_fd(), timeout)?
    {
        match guard.read() {
            Ok(_) => {}
            Err(wayland_client::backend::WaylandError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e).context("reading from the compositor"),
        }
    }
    queue.dispatch_pending(window)?;
    Ok(())
}

/// Waits up to `timeout` for `fd` to have something to read.
fn readable(fd: BorrowedFd<'_>, timeout: Duration) -> Result<bool> {
    let mut poll = libc::pollfd { fd: fd.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    let ms = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
    // SAFETY: one pollfd, valid for the call.
    let n = unsafe { libc::poll(&raw mut poll, 1, ms) };
    if n < 0 {
        let e = std::io::Error::last_os_error();
        return if e.kind() == std::io::ErrorKind::Interrupted { Ok(false) } else { Err(e).context("poll") };
    }
    Ok(n > 0)
}

/// Connects to the daemon and reads its frames on a thread of their own. The channel closes
/// when the daemon goes away.
fn daemon_frames() -> Result<Receiver<OverlayFrame>> {
    let stream = UnixStream::connect(ipc::socket_path()).context("the daemon isn't running")?;
    let mut request = serde_json::to_string(&Request::WatchOverlay)?;
    request.push('\n');
    (&stream).write_all(request.as_bytes())?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(&stream).lines() {
            let Ok(line) = line else { break };
            if let Ok(frame) = serde_json::from_str::<OverlayFrame>(&line)
                && tx.send(frame).is_err()
            {
                return;
            }
        }
    });
    Ok(rx)
}
