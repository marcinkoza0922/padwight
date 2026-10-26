//! The overlay window on Linux without iced: a wlr-layer-shell surface of our own, drawn on the
//! CPU. Chosen with `PADWIGHT_OVERLAY=native` while it is being built; `layershell.rs` stays
//! the default until it can replace it.
//!
//! Frames are drawn on the compositor's frame callbacks while anything moves, and not at all
//! while everything is still (`pacing.rs`), into a pool of up to three reused shared-memory
//! buffers (`shm.rs`, `slots.rs`). For now the loop also wakes on a short timer to look for
//! frames from the daemon; later steps bring the scale (5.4) and an eventfd the reading thread
//! wakes the loop with (5.5).
//!
//! In the harness: `PADWIGHT_HARNESS_RELEASE=1 PADWIGHT_OVERLAY=native scripts/overlay-harness.sh
//! scripts/harness/pacing.sh <out>`. Besides the usual lines, the timing log has `surface <ns> ask
//! <target> <w>x<h>` for each request, `configure <ns> <w>x<h> <target> <commit>` for each configure
//! acked, and `commit <ns> <w>x<h> <why> <target>` for each buffer committed, so it shows that
//! every buffer goes on at the size last configured; `present <ns>` is stamped at each frame's
//! commit, and `callback <ns> <compositor ms> current|stale` as each frame callback is done.

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
mod pacing;
mod picture;
mod shm;
mod sizing;
mod slots;
mod window;

use picture::{Screen, TestPicture};
use sizing::{Commit, Target};
use window::Window;

pub(super) fn run() -> Result<()> {
    let conn = Connection::connect_to_env().context("the overlay window needs a Wayland session")?;
    let (globals, mut queue) = registry_queue_init::<Window>(&conn).context("reading the compositor's globals")?;
    let qh = queue.handle();
    let mut window = Window::new(&globals, &qh)?;
    let frames = daemon_frames()?;
    let mut overlay = Overlay::new();
    let mut screen = TestPicture;
    loop {
        // Looked at on every wake: a frame callback, a release, a configure or the timeout.
        loop {
            match frames.try_recv() {
                Ok(frame) => {
                    overlay.receive(frame);
                    window.pacer.changed();
                }
                Err(TryRecvError::Empty) => break,
                // The daemon went away: nothing left to draw for.
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
        // The animation clock, read as the loop wakes: just after a frame callback while
        // anything moves, so frames step by the compositor's refresh.
        let now = Instant::now();
        overlay.prune(now);
        window.pacer.moving(overlay.busy(now));
        step(&mut window, &mut screen, &overlay, now, &qh)?;
        if window.closed {
            anyhow::bail!("the compositor closed the overlay's surface");
        }
        let timeout = window.pacer.timeout(Instant::now(), overlay.shown());
        wait(&conn, &mut queue, &mut window, timeout)?;
    }
}

/// Brings the surface in line with the overlay: asks for the screen or the corner, puts on
/// what the last configure asked for, and draws the screen at `now` when a frame is due.
fn step(window: &mut Window, screen: &mut dyn Screen, overlay: &Overlay, now: Instant, qh: &wayland_client::QueueHandle<Window>) -> Result<()> {
    let todo = std::mem::replace(&mut window.todo, Commit::Nothing);
    match window.sizing.want(overlay.shown()) {
        Commit::Grow => window.grow(),
        // The screen's buffers are let go of once the 1×1 one is on (`Window::blank`).
        Commit::Shrink { clear } => window.shrink(clear, qh)?,
        _ => {}
    }
    // A configure answers the request it came for; one asked since then waits for its own.
    match todo {
        Commit::Blank if window.sizing.target() == Target::Corner => window.blank(qh)?,
        // The first frame at a new size goes at once, callback or not: nothing of ours is on
        // at that size for it to wait behind.
        Commit::Draw(_) => window.draw(screen, overlay, now, qh)?,
        // A frame skipped while every buffer was held goes out once one is released.
        _ if overlay.shown() && window.can_draw() && window.pacer.due(now) => window.draw(screen, overlay, now, qh)?,
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
