//! Puts text on the desktop's clipboard through `wl-copy` (wl-clipboard). The daemon has no
//! window of its own to do it with, so it asks the tool. Xwayland games read the same clipboard,
//! since the compositor keeps the two in step.

use std::{
    io::Write,
    process::{Command, Stdio},
};

/// The tool that reads text on standard input and makes it the clipboard.
pub const TOOL: &str = "wl-copy";

/// Copies `text`. Errs with the reason when the tool is missing or fails.
pub fn copy(text: &str) -> Result<(), String> {
    let mut child = Command::new(TOOL)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("can't run {TOOL} (install wl-clipboard): {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        // The tool reads to the end, so it's closed before waiting: a tool that forks to keep
        // the clipboard keeps none of our pipes.
        let _ = stdin.write_all(text.as_bytes());
    }
    match child.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("{TOOL} failed ({status})")),
        Err(e) => Err(format!("{TOOL} failed: {e}")),
    }
}
