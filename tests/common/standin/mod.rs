//! Stand-in compositors: small servers that speak just enough of a compositor's focus protocol for
//! the daemon's tracker to connect and follow them. A test starts one under its runtime dir,
//! starts the daemon with that dir, and calls [`Compositor::focus`] to move focus.
//!
//! They need no display, session bus or `/dev/input`. The soak runner can use the same types to
//! make random focus changes.

use std::{
    io,
    os::fd::AsRawFd,
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

pub mod hyprland;
pub mod niri;
pub mod sway;

/// A window a stand-in compositor shows. The pid should belong to a process that exists (see
/// [`super::focus::GameProcess`]): the daemon identifies the game from it.
#[derive(Clone, Debug)]
pub struct Window {
    pub id: u64,
    pub pid: u32,
    pub class: String,
    pub title: String,
}

impl Window {
    pub fn new(id: u64, pid: u32, class: &str, title: &str) -> Self {
        Self { id, pid, class: class.into(), title: title.into() }
    }
}

/// What every stand-in does, so a runner can drive any of them the same way.
pub trait Compositor {
    /// Gives `window` focus, opening it first if the compositor has not seen it.
    fn focus(&self, window: &Window);
    /// Nothing has focus (an empty workspace, the desktop).
    fn unfocus(&self);
    /// Closes `window`; if it had focus, nothing has.
    fn close(&self, window: &Window);
}

/// A Unix socket server: accepts connections and runs `handler` for each on its own thread.
/// Stops accepting and removes the socket file when dropped.
pub struct Server {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    waker: UnixListener,
    accept: Option<JoinHandle<()>>,
}

impl Server {
    pub fn start(path: &Path, handler: impl Fn(UnixStream) + Send + Sync + 'static) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        let stop = Arc::new(AtomicBool::new(false));
        let handler = Arc::new(handler);
        let waker = listener.try_clone()?;
        let flag = stop.clone();
        let accept = thread::spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::Relaxed) {
                    return;
                }
                let Ok(stream) = stream else { continue };
                let handler = handler.clone();
                thread::spawn(move || handler(stream));
            }
        });
        Ok(Self { path: path.to_path_buf(), stop, waker, accept: Some(accept) })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Shutting the listening socket down wakes the accept loop, even if the socket file is gone.
        // SAFETY: the descriptor is owned by `self.waker`, which is open.
        unsafe { libc::shutdown(self.waker.as_raw_fd(), libc::SHUT_RDWR) };
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}
