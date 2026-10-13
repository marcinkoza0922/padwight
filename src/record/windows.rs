//! Screen recording on Windows with ffmpeg, saved to `~/Videos/Recordings/<game>/`.
//!
//! Windows has no recorder that can be started and stopped from the command line, so this needs
//! ffmpeg on PATH. It grabs the desktop through GDI and is told to finish by a `q` on its
//! standard input, which it reads between frames.

use std::{
    ffi::OsString,
    io::Write,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Mutex, PoisonError},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

use crate::capture::{claim_media_path, on_path, timestamp};

const PROGRAM: &str = "ffmpeg";

/// How long a fresh recorder has to stay up before it counts as started.
const SETTLE: Duration = Duration::from_millis(1200);

/// The running recorders' inputs, by process ID, for [`stop`] to send `q` to.
static INPUTS: Mutex<Vec<(u32, ChildStdin)>> = Mutex::new(Vec::new());

fn args(path: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["-hide_banner", "-loglevel", "error", "-y"].map(Into::into).into();
    // The whole desktop at 60 fps, encoded fast enough to keep up while a game runs.
    args.extend(["-f", "gdigrab", "-framerate", "60", "-i", "desktop"].map(OsString::from));
    args.extend(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "23", "-pix_fmt", "yuv420p"].map(OsString::from));
    args.push(path.as_os_str().to_owned());
    args
}

/// Starts recording for `game` and returns the recorder and its file. Waits a moment to see
/// that it stays up, so run it off the daemon's main thread.
pub fn start(game: &str) -> Result<(Child, PathBuf)> {
    if !on_path(PROGRAM) {
        bail!("{PROGRAM} is not installed (or use Xbox Game Bar: Win+Alt+R)");
    }
    let videos = crate::capture::media_base(dirs::video_dir(), "Videos")?;
    // Held while the recorder starts, which is when it makes the file.
    let claim = claim_media_path(&videos, "Recordings", game, &timestamp(), "mp4");
    let path = claim.path().to_owned();
    std::fs::create_dir_all(path.parent().context("no folder")?).with_context(|| format!("creating {}", path.display()))?;
    let mut child = Command::new(PROGRAM)
        .args(args(&path))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW.0)
        .spawn()
        .with_context(|| format!("running {PROGRAM}"))?;
    thread::sleep(SETTLE);
    if child.try_wait()?.is_some() {
        bail!("{PROGRAM} couldn't start recording the screen");
    }
    if let Some(input) = child.stdin.take() {
        let mut inputs = INPUTS.lock().unwrap_or_else(PoisonError::into_inner);
        // Recorders that ended by themselves were never stopped: their pipes go now.
        inputs.retain(|(pid, _)| crate::quit::is_running(*pid));
        inputs.push((child.id(), input));
    }
    Ok((child, path))
}

/// Asks the recorder to finish its file and exit.
pub fn stop(pid: u32) {
    let mut inputs = INPUTS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(at) = inputs.iter().position(|(p, _)| *p == pid) {
        let (_, mut input) = inputs.swap_remove(at);
        let _ = input.write_all(b"q");
        // Dropping `input` closes it, which also ends ffmpeg's wait for more.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_the_desktop_to_the_file() {
        let a = args(Path::new(r"C:\v\x.mp4"));
        assert!(a.iter().any(|x| x == "gdigrab") && a.iter().any(|x| x == "desktop"));
        assert_eq!(a.last().unwrap(), r"C:\v\x.mp4");
    }
}
