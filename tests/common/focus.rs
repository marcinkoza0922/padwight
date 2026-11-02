//! What every focus test needs: a config with one game, a process named like it, and a way to
//! read the profile the daemon has chosen.

use std::{
    fs,
    path::Path,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use super::{TIMEOUT, daemon_log, default_config};

/// The stand-in game's file name, which the rule matches.
pub const GAME_EXE: &str = "padwight-fake-game";
pub const GAME: &str = "Fake Game";
pub const GAME_PROFILE: &str = "Fake Pad";
/// What `padwight status` prints for the profile when the game has focus, and when nothing does.
pub const GAME_LINE: &str = "profile: Fake Pad (Fake Game)";
pub const DEFAULT_LINE: &str = "profile: Gamepad";

/// The default config with one game added: a rule that matches [`GAME_EXE`], and a profile copied
/// from the General Gamepad profile, so it maps the same buttons under another name.
pub fn game_config() -> String {
    // The default config has an empty `setups` list, which the game's `[[setups]]` would repeat.
    let text = default_config();
    assert!(text.contains("\nsetups = []\n"), "the default config has an empty setups list");
    let text = text.replace("\nsetups = []\n", "\n");
    let mut general = Vec::new();
    let mut in_profile = false;
    for line in text.lines() {
        if line.starts_with("[[general.profiles]]") {
            in_profile = true;
        } else if line.starts_with('[') && !line.starts_with("[general.profiles.") && !line.starts_with("[[general.profiles.") {
            in_profile = false;
        }
        if in_profile {
            general.push(line);
        }
    }
    let profile = general.join("\n").replace("general.profiles", "setups.profiles").replace("name = \"Gamepad\"", &format!("name = \"{GAME_PROFILE}\""));
    assert!(profile.contains(&format!("name = \"{GAME_PROFILE}\"")), "the default config has a Gamepad profile");
    format!(
        "{text}\n[[setups]]\nname = \"{GAME}\"\n\n[[setups.rules]]\nkind = \"executable\"\nvalue = \"{GAME_EXE}\"\nprofile = \"{GAME_PROFILE}\"\n\n{profile}\n"
    )
}

/// A process that runs as [`GAME_EXE`]: a copy of `sleep` under that name, so the daemon sees the
/// name in the process's executable. Killed when dropped.
pub struct GameProcess(pub Child);

impl GameProcess {
    pub fn start(root: &Path) -> Self {
        let sleep = ["/bin/sleep", "/usr/bin/sleep"].into_iter().find(|p| Path::new(p).exists()).expect("a sleep binary");
        let game = root.join(GAME_EXE);
        if !game.exists() {
            fs::copy(sleep, &game).unwrap();
        }
        Self(Command::new(&game).arg("600").spawn().unwrap())
    }

    pub fn pid(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for GameProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Waits until the daemon's log has `line`. On a timeout the failure shows the whole log.
pub fn wait_for_log(root: &Path, line: &str) {
    let deadline = Instant::now() + TIMEOUT;
    while !daemon_log(root).contains(line) {
        assert!(Instant::now() < deadline, "timed out waiting for the log to say {line:?}: {}", daemon_log(root));
        thread::sleep(Duration::from_millis(20));
    }
}

/// The output of `padwight status` for the daemon whose runtime dir is `runtime`.
pub fn status(runtime: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_padwight")).arg("status").env("XDG_RUNTIME_DIR", runtime).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Waits until `padwight status` has `line`; on a timeout the failure shows the last status and the log.
pub fn wait_for_status(root: &Path, runtime: &Path, line: &str) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let now = status(runtime);
        if now.lines().any(|l| l == line) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for status to say {line:?}; it says:\n{now}\nlog:\n{}", daemon_log(root));
        thread::sleep(Duration::from_millis(50));
    }
}
