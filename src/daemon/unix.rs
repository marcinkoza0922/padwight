//! The service's process on Linux: SIGTERM and SIGINT, a control socket only this user can
//! reach, and a session of its own when started in the background.

use std::{
    os::unix::process::CommandExt,
    path::Path,
    process::Command,
    sync::mpsc::Sender,
    thread,
};

use anyhow::{Context, Result, bail};

use super::Msg;
use crate::ipc::{UnixListener, UnixStream};

/// A directory named `name` in the temp directory, for this user only: the temp directory is
/// shared, so the name carries the user's ID.
pub fn user_temp_dir(name: &str) -> std::path::PathBuf {
    // SAFETY: getuid cannot fail.
    std::env::temp_dir().join(format!("{name}-{}", unsafe { libc::getuid() }))
}

/// A window the daemon starts (the overlay, the settings) as it is: only Windows would give it a
/// console window too.
pub fn windowless(command: &mut Command) -> &mut Command {
    command
}

/// Puts the daemon `command` starts in a session of its own, which keeps it alive when the
/// terminal that started the window goes away.
pub fn detach(command: &mut Command) {
    // SAFETY: only calls `setsid`, which is async-signal-safe.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

/// The write end of the pipe that SIGTERM and SIGINT wake the daemon through; -1 until set up.
static WAKE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// Makes SIGTERM and SIGINT send `Msg::Stop`, so the daemon can release what it holds before it
/// exits. The handler only writes a byte to a pipe, the one thing a signal handler can do safely;
/// a thread reads the byte and passes the message on, so the loop wakes straight away.
pub fn stop_on_signals(tx: Sender<Msg>) -> Result<()> {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: pipe2 writes two descriptors into the array it is given.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        bail!("creating the signal pipe: {}", std::io::Error::last_os_error());
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    WAKE.store(write_fd, std::sync::atomic::Ordering::Relaxed);
    for signal in [libc::SIGTERM, libc::SIGINT] {
        // SAFETY: a zeroed sigaction is a valid starting point; the fields set below are the ones used.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = on_stop_signal as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        // SAFETY: the handler stays valid for the life of the process.
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 || unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) } != 0 {
            bail!("installing the handler for signal {signal}: {}", std::io::Error::last_os_error());
        }
    }
    thread::spawn(move || {
        let mut byte = [0u8; 1];
        loop {
            // SAFETY: reads at most one byte into a local buffer, from the pipe read end we own.
            let n = unsafe { libc::read(read_fd, byte.as_mut_ptr().cast(), 1) };
            if n == 1 {
                if tx.send(Msg::Stop).is_err() {
                    return;
                }
            } else if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return;
            }
        }
    });
    Ok(())
}

/// Signal handler: writes one byte to the wake pipe. Only async-signal-safe calls are made.
extern "C" fn on_stop_signal(_signal: libc::c_int) {
    let fd = WAKE.load(std::sync::atomic::Ordering::Relaxed);
    if fd >= 0 {
        let byte = 1u8;
        // SAFETY: write is async-signal-safe. Its result is ignored: a full pipe already wakes the reader.
        unsafe { libc::write(fd, (&byte as *const u8).cast(), 1) };
    }
}

/// Binds the control socket so that only this user can connect: its directory must belong to
/// this user and be private, and the socket itself is `0600`. Anyone who can send a request can
/// drive the daemon, so this matters most for the `/tmp` fallback, which other users can see.
pub fn bind_socket_at(path: &Path) -> Result<UnixListener> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(dir) = path.parent() {
        prepare_private_dir(dir)?;
    }
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            bail!("another instance is already running ({})", path.display());
        }
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path).with_context(|| format!("binding {}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("restricting {}", path.display()))?;
    Ok(listener)
}

/// Creates `dir` (and any missing parents) as a directory only this user can use. An existing
/// directory must be owned by this user, or the daemon refuses to put its socket there.
pub fn prepare_private_dir(dir: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let meta = std::fs::symlink_metadata(dir)?;
    // SAFETY: getuid cannot fail.
    if meta.uid() != unsafe { libc::getuid() } || !meta.is_dir() {
        bail!("{} is not a directory owned by this user", dir.display());
    }
    if meta.mode() & 0o077 != 0 {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_socket_is_private_to_this_user() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("padwight-test-{}", std::process::id())).join("run");
        let path = dir.join("padwight.sock");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());

        let listener = bind_socket_at(&path).unwrap();
        let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        let sock_mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!((dir_mode, sock_mode), (0o700, 0o600));
        assert!(bind_socket_at(&path).is_err(), "a running daemon's socket is not taken over");

        drop(listener);
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
