//! The service's process on Windows: Ctrl+C and the console closing, the control socket, and no
//! console window of its own when started in the background.

use std::{
    os::windows::process::CommandExt,
    path::Path,
    process::Command,
    sync::OnceLock,
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use windows::{
    Win32::{
        Foundation::{FALSE, TRUE},
        System::{
            Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler},
            Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW},
        },
    },
    core::BOOL,
};

use crate::ipc::{UnixListener, UnixStream};

/// A directory named `name` in the temp directory, which on Windows is already the user's own.
pub fn user_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(name)
}

/// Starts a window of ours (the overlay, the settings) without the console window Windows gives a
/// console program whose parent has none.
pub fn windowless(command: &mut Command) -> &mut Command {
    command.creation_flags(CREATE_NO_WINDOW.0)
}

/// Starts the daemon with no console window, in a process group of its own so a Ctrl+C in the
/// terminal that opened the settings window doesn't reach it.
pub fn detach(command: &mut Command) {
    command.creation_flags((CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP).0);
}

/// What the console handler calls.
type Stop = Box<dyn Fn() -> bool + Send + Sync>;
static STOP: OnceLock<Stop> = OnceLock::new();

/// Makes Ctrl+C, Ctrl+Break and the console closing call `stop`, so the daemon (or a debug
/// session) can release what it holds before it exits. `stop` returns false once nothing listens.
pub fn stop_on_signals(stop: impl Fn() -> bool + Send + Sync + 'static) -> Result<()> {
    if STOP.set(Box::new(stop)).is_err() {
        bail!("the console handler is already installed");
    }
    // SAFETY: the handler is a plain function that stays valid for the life of the process.
    unsafe { SetConsoleCtrlHandler(Some(on_console_event), true) }.context("installing the console handler")?;
    Ok(())
}

/// Runs on a thread Windows starts for the event. Windows ends the process once a close, log-off
/// or shutdown handler returns, so those give the daemon a moment to release what it holds.
unsafe extern "system" fn on_console_event(event: u32) -> BOOL {
    let Some(stop) = STOP.get() else { return FALSE };
    let sent = stop();
    match event {
        CTRL_C_EVENT | CTRL_BREAK_EVENT => {
            if sent {
                TRUE
            } else {
                FALSE
            }
        }
        CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT => {
            thread::sleep(Duration::from_millis(500));
            TRUE
        }
        _ => FALSE,
    }
}

/// Binds the control socket. It lives in the user's local app data, which only they can open, so
/// nothing more is needed to keep other users out.
pub fn bind_socket_at(path: &Path) -> Result<UnixListener> {
    if let Some(dir) = path.parent() {
        prepare_private_dir(dir)?;
    }
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            bail!("another instance is already running ({})", path.display());
        }
        std::fs::remove_file(path)?;
    }
    UnixListener::bind(path).with_context(|| format!("binding {}", path.display()))
}

/// Creates `dir` (and any missing parents). Directories under the user's profile already belong
/// to the user alone.
pub fn prepare_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_daemons_socket_is_not_taken_over() {
        let dir = std::env::temp_dir().join(format!("padwight-test-{}", std::process::id())).join("run");
        let path = dir.join("padwight.sock");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());

        let listener = bind_socket_at(&path).unwrap();
        assert!(bind_socket_at(&path).is_err(), "a running daemon's socket is not taken over");
        drop(listener);
        // A socket file left behind by a daemon that's gone is replaced.
        drop(bind_socket_at(&path).unwrap());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
