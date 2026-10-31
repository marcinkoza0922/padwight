//! The daemon's frames, read on a thread of their own and handed to the loop through a channel
//! plus an eventfd, so the loop can sleep in one `poll` on both the compositor's socket and the
//! daemon's frames, and wakes for neither more than it has to.
//!
//! The thread reads the blocking socket, sends each frame, and writes the eventfd. When the
//! daemon goes away (end of stream, or an error) it drops the sender and writes the eventfd one
//! last time, so the loop finds the channel closed and ends.

use std::{
    io::{BufRead, BufReader, Write},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use anyhow::{Context, Result};

use crate::{
    ipc::{self, Request, UnixStream},
    overlay::OverlayFrame,
};

pub(in crate::overlay) struct Feed {
    frames: Receiver<OverlayFrame>,
    wake: Arc<OwnedFd>,
}

/// What a wake brought.
pub(in crate::overlay) struct Arrived {
    /// The frames to show, in order: those a newer one stands in for are left out.
    pub frames: Vec<OverlayFrame>,
    /// The daemon went away.
    pub gone: bool,
}

impl Feed {
    /// Connects to the daemon, asks to watch the overlay, and starts reading.
    pub fn connect() -> Result<Self> {
        let stream = UnixStream::connect(ipc::socket_path()).context("the daemon isn't running")?;
        let mut request = serde_json::to_string(&Request::WatchOverlay)?;
        request.push('\n');
        (&stream).write_all(request.as_bytes())?;
        let wake = new_eventfd()?;
        let (tx, frames) = mpsc::channel();
        let signal = Arc::clone(&wake);
        thread::spawn(move || {
            let mut doodles = Vec::new();
            for line in BufReader::new(&stream).lines() {
                let Ok(line) = line else { break };
                if let Ok(mut frame) = serde_json::from_str::<OverlayFrame>(&line) {
                    frame.put_back_doodles(&mut doodles);
                    if tx.send(frame).is_err() {
                        return;
                    }
                    signal_wake(&signal);
                }
            }
            drop(tx);
            signal_wake(&signal);
        });
        Ok(Self { frames, wake })
    }

    /// The descriptor that is readable while something has arrived.
    pub fn fd(&self) -> BorrowedFd<'_> {
        self.wake.as_fd()
    }

    /// Clears the wake and takes everything that has arrived, so no frame is left behind a
    /// cleared wake: the channel is emptied after the counter is read.
    pub fn drain(&self) -> Arrived {
        let mut counter = [0u8; 8];
        // SAFETY: reads at most 8 bytes into a buffer of 8; a non-blocking eventfd either gives
        // the counter or fails with EAGAIN when it is already zero.
        unsafe { libc::read(self.wake.as_raw_fd(), counter.as_mut_ptr().cast(), counter.len()) };
        let mut burst = Vec::new();
        let mut gone = false;
        loop {
            match self.frames.try_recv() {
                Ok(frame) => burst.push(frame),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    gone = true;
                    break;
                }
            }
        }
        Arrived { frames: coalesce(burst), gone }
    }
}

/// Leaves out each frame the one after it stands in for, so a burst shows its last state but
/// every opening, closing and pick is still seen.
pub(super) fn coalesce(mut burst: Vec<OverlayFrame>) -> Vec<OverlayFrame> {
    let mut keep: Vec<bool> = (0..burst.len()).map(|i| !burst.get(i + 1).is_some_and(|newer| newer.stands_in_for(&burst[i]))).collect();
    let mut keep = keep.drain(..);
    burst.retain(|_| keep.next().unwrap_or(true));
    burst
}

fn new_eventfd() -> Result<Arc<OwnedFd>> {
    // SAFETY: a plain syscall; the descriptor it returns is owned by nothing else.
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("eventfd");
    }
    // SAFETY: `fd` is a fresh, valid descriptor.
    Ok(Arc::new(unsafe { OwnedFd::from_raw_fd(fd) }))
}

fn signal_wake(fd: &OwnedFd) {
    let one = 1u64.to_ne_bytes();
    // SAFETY: writes 8 bytes from a buffer of 8. A full counter (EAGAIN) means the loop is
    // already due to wake, so the result needs no handling.
    unsafe { libc::write(fd.as_raw_fd(), one.as_ptr().cast(), one.len()) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::MenuKind,
        menu::MenuView,
        overlay::OverlayView,
    };

    fn menu(selected: usize, picks: u32) -> OverlayFrame {
        let view = MenuView {
            title: "Belt".into(),
            kind: MenuKind::List,
            items: Vec::new(),
            selected: Some(selected),
            picks,
            crumbs: Vec::new(),
            hint: String::new(),
            about: String::new(),
            style: Default::default(),
            touch: false,
            aim: None,
            prompt: None,
            one_column: false,
            figure: None,
        };
        OverlayFrame { active: Some(OverlayView::Menu(view)), ..Default::default() }
    }

    fn selection(frame: &OverlayFrame) -> Option<usize> {
        match &frame.active {
            Some(OverlayView::Menu(m)) => m.selected,
            _ => None,
        }
    }

    #[test]
    fn a_burst_of_cursor_moves_keeps_the_last() {
        let kept = coalesce(vec![menu(0, 0), menu(1, 0), menu(2, 0), menu(3, 0)]);
        assert_eq!(kept.iter().map(selection).collect::<Vec<_>>(), [Some(3)]);
    }

    #[test]
    fn openings_closings_and_picks_are_kept() {
        let burst = vec![OverlayFrame::default(), menu(0, 0), menu(1, 0), menu(1, 1), OverlayFrame::default()];
        let kept = coalesce(burst);
        // The empty frame before the opening, the last move, the pick and the close.
        assert_eq!(kept.len(), 4);
        assert!(kept[0].active.is_none());
        assert_eq!(selection(&kept[1]), Some(1));
        assert!(kept[3].active.is_none());
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(coalesce(Vec::new()).is_empty());
        assert_eq!(coalesce(vec![menu(0, 0)]).len(), 1);
    }
}
