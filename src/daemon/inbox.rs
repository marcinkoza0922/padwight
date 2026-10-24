//! The main loop's inbox: what the daemon's other threads (the control socket, focus tracking, the
//! tray, background tasks) send it. On Linux the loop waits on the controllers' nodes as well as on
//! this, so each message also rings a wake-up the loop can wait on beside them (an eventfd).

use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};

use super::Msg;

/// A sender to the main loop. Like `mpsc::Sender`, and it wakes the loop.
#[derive(Clone)]
pub(super) struct Tx {
    tx: Sender<Msg>,
    pub(super) wake: Arc<Wake>,
}

/// The main loop has ended, so nothing it is sent is read.
#[derive(Debug)]
pub(super) struct Closed;

impl std::fmt::Display for Closed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the daemon's main loop has ended")
    }
}

impl std::error::Error for Closed {}

impl Tx {
    pub(super) fn send(&self, msg: Msg) -> Result<(), Closed> {
        self.tx.send(msg).map_err(|_| Closed)?;
        self.wake.ring();
        Ok(())
    }
}

pub(super) fn channel() -> std::io::Result<(Tx, Receiver<Msg>)> {
    let (tx, rx) = mpsc::channel();
    Ok((Tx { tx, wake: Arc::new(Wake::new()?) }, rx))
}

/// An eventfd the main loop polls beside the controllers' nodes.
#[cfg(target_os = "linux")]
pub(super) struct Wake(std::os::fd::OwnedFd);

#[cfg(target_os = "linux")]
impl Wake {
    fn new() -> std::io::Result<Self> {
        use std::os::fd::FromRawFd;
        // SAFETY: eventfd takes no pointers; a non-negative result is a new descriptor we own.
        let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: as above, `fd` is open and nothing else owns it.
        Ok(Wake(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) }))
    }

    fn ring(&self) {
        use std::os::fd::AsRawFd;
        let one = 1u64;
        // SAFETY: writes the 8 bytes of `one`. It can only fail when the counter is about to
        // overflow, and then the loop is woken already.
        unsafe { libc::write(self.0.as_raw_fd(), (&raw const one).cast(), 8) };
    }

    /// Clears the wake-up, once the loop has woken.
    pub(super) fn clear(&self) {
        use std::os::fd::AsRawFd;
        let mut count = 0u64;
        // SAFETY: reads at most 8 bytes into `count`; with nothing to read it fails at once.
        unsafe { libc::read(self.0.as_raw_fd(), (&raw mut count).cast(), 8) };
    }

    pub(super) fn fd(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.0.as_raw_fd()
    }
}

/// Windows: the loop waits on the channel alone, which wakes it itself.
#[cfg(windows)]
pub(super) struct Wake;

#[cfg(windows)]
impl Wake {
    fn new() -> std::io::Result<Self> {
        Ok(Wake)
    }

    fn ring(&self) {}
}
