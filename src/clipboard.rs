//! Puts text on the desktop's clipboard, through whichever clipboard tool the system has. The
//! daemon has no window of its own to do it with, so it asks the tool that the desktop ships.

use std::{
    io::Write,
    process::{Command, Stdio},
};

/// The tools that read text on standard input and make it the clipboard, in the order they're tried.
#[cfg(target_os = "linux")]
const TOOLS: &[(&str, &[&str])] = &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"]), ("xsel", &["--clipboard", "--input"])];

#[cfg(windows)]
const TOOLS: &[(&str, &[&str])] = &[("clip", &[])];

/// Copies `text`. Errs with the reason when no tool takes it.
pub fn copy(text: &str) -> Result<(), String> {
    for (program, args) in TOOLS {
        let Ok(mut child) = Command::new(program).args(*args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else {
            continue;
        };
        if let Some(mut stdin) = child.stdin.take() {
            // The tool reads to the end, so it's closed before waiting: a tool that forks to keep
            // the clipboard keeps none of our pipes.
            let _ = stdin.write_all(text.as_bytes());
        }
        if child.wait().is_ok_and(|status| status.success()) {
            return Ok(());
        }
    }
    Err(format!("no clipboard tool took it ({})", TOOLS.iter().map(|(program, _)| *program).collect::<Vec<_>>().join(", ")))
}
