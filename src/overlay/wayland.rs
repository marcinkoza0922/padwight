//! The overlay window on Linux without iced: a wlr-layer-shell surface of our own, drawn on the
//! CPU. Chosen with `PADWIGHT_OVERLAY=native` while it is being built; `layershell.rs` stays
//! the default until it can replace it.
//!
//! Frames are drawn on the compositor's frame callbacks while anything moves, and not at all
//! while everything is still (`pacing.rs`), into a pool of up to three reused shared-memory
//! buffers (`shm.rs`, `slots.rs`). A thread reads the daemon's frames and wakes the loop through an
//! eventfd (`feed.rs`), so a still or hidden overlay sleeps in `poll` until something comes.
//!
//! In the harness: `PADWIGHT_HARNESS_RELEASE=1 PADWIGHT_OVERLAY=native scripts/overlay-harness.sh
//! scripts/harness/pacing.sh <out>`. Besides the usual lines, the timing log has `surface <ns> ask
//! <target> <w>x<h>` for each request, `configure <ns> <w>x<h> <target> <commit>` for each configure
//! acked, and `commit <ns> <w>x<h> <why> <target>` for each buffer committed, so it shows that
//! every buffer goes on at the size last configured; `present <ns>` is stamped at each frame's
//! commit, and `callback <ns> <compositor ms> current|stale` as each frame callback is done.

use std::{
    os::fd::{AsRawFd, BorrowedFd},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use wayland_client::{Connection, EventQueue, globals::registry_queue_init};

use super::ui::Overlay;

mod events;
mod feed;
mod pacing;
mod picture;
mod scale;
mod shm;
mod sizing;
mod slots;
mod window;

use feed::Feed;
use picture::{Screen, TestPicture};
use sizing::{Commit, Target};
use window::Window;

pub(super) fn run() -> Result<()> {
    let conn = Connection::connect_to_env().context("the overlay window needs a Wayland session")?;
    let (globals, mut queue) = registry_queue_init::<Window>(&conn).context("reading the compositor's globals")?;
    let qh = queue.handle();
    let mut window = Window::new(&globals, &qh)?;
    let feed = Feed::connect()?;
    let mut overlay = Overlay::new();
    let mut screen = TestPicture;
    loop {
        // Looked at on every wake: a frame callback, a release, a configure, a frame from the
        // daemon or the fallback. The daemon's frames are all taken, those a later one stands in
        // for left out.
        let arrived = feed.drain();
        for frame in arrived.frames {
            overlay.receive(frame);
            window.pacer.changed();
        }
        if arrived.gone {
            // The daemon went away: nothing left to draw for.
            return Ok(());
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
        wait(&conn, &mut queue, &mut window, &feed, timeout)?;
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
    // The first thing after a change of scale; a configure or draw that follows draws again.
    window.rescale(screen, overlay, now, qh)?;
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

/// Sends what's queued, waits up to `timeout` (for ever when none) for the compositor or the
/// daemon's frames, and handles what the compositor sent. The frames are taken at the top of the
/// next round.
fn wait(conn: &Connection, queue: &mut EventQueue<Window>, window: &mut Window, feed: &Feed, timeout: Option<Duration>) -> Result<()> {
    queue.dispatch_pending(window)?;
    conn.flush()?;
    if let Some(guard) = queue.prepare_read()
        && readable(guard.connection_fd(), feed.fd(), timeout)?
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

/// Waits up to `timeout` (for ever when none) for the compositor's socket or the daemon's wake
/// to have something to read: returns whether the compositor's has.
fn readable(compositor: BorrowedFd<'_>, daemon: BorrowedFd<'_>, timeout: Option<Duration>) -> Result<bool> {
    let mut fds = [libc::pollfd { fd: compositor.as_raw_fd(), events: libc::POLLIN, revents: 0 }, libc::pollfd { fd: daemon.as_raw_fd(), events: libc::POLLIN, revents: 0 }];
    let ms = timeout.map_or(-1, |t| i32::try_from(t.as_millis()).unwrap_or(i32::MAX));
    // SAFETY: two pollfds, valid for the call.
    let n = unsafe { libc::poll(fds.as_mut_ptr(), 2, ms) };
    if n < 0 {
        let e = std::io::Error::last_os_error();
        return if e.kind() == std::io::ErrorKind::Interrupted { Ok(false) } else { Err(e).context("poll") };
    }
    // A hang-up or error on the socket is read too, so the read reports it.
    Ok(fds[0].revents != 0)
}
