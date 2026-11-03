//! What `padwight debug focus` names for games the way launchers start them (FOC-4): a plain binary,
//! and a Proton-like Wine process whose own binary is the loader and whose command line holds the
//! Windows program. The processes are stand-ins (copies of `sh` under the launcher's names, with
//! Steam's environment variables), so no Steam, Heroic or Lutris is needed; what the real launchers
//! put in those command lines and environments still has to be looked at once by hand.
//!
//! No devices or session are needed, so these run with the normal tests. The scan test uses rule
//! names of its own, since the tests run side by side and a scan sees every stand-in.

#![cfg(target_os = "linux")]

mod common;

use std::{
    fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

use common::{
    focus::{GAME, GAME_EXE, GAME_PROFILE, game_config},
    install_config, temp_root,
};

/// A copy of `sh` under `name` that runs `script` with `args` after it, so `/proc/<pid>/exe` and the
/// command line look as a launcher's child would. Killed when dropped.
struct Fake(Child);

impl Fake {
    fn start(root: &Path, name: &str, args: &[&str], env: &[(&str, &str)]) -> Self {
        let sh = ["/bin/sh", "/usr/bin/sh"].into_iter().find(|p| Path::new(p).exists()).expect("a shell");
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let dir = root.join("bin").join(COUNT.fetch_add(1, Ordering::Relaxed).to_string());
        fs::create_dir_all(&dir).unwrap();
        let copy = dir.join(name);
        fs::copy(fs::canonicalize(sh).unwrap(), &copy).unwrap();
        // A loop, not a single command, so the shell stays the process (some shells exec the last one).
        let child = Command::new(&copy)
            .args(["-c", "while :; do sleep 1; done"])
            .args(args)
            .envs(env.iter().copied())
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Self(child)
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        // The whole group: the shell's `sleep` would otherwise outlive it and show in a later scan.
        // SAFETY: `kill` only sends a signal, to a process group this struct started.
        unsafe { libc::kill(-(self.0.id() as libc::pid_t), libc::SIGKILL) };
        let _ = self.0.wait();
    }
}

/// The game config with two more rules for the same setup: a Windows executable and a Steam app.
fn config_with_wine_rules() -> String {
    let first = format!("[[setups.rules]]\nkind = \"executable\"\nvalue = \"{GAME_EXE}\"\nprofile = \"{GAME_PROFILE}\"\n");
    let text = game_config();
    assert!(text.contains(&first), "the game config has the executable rule");
    let more = format!(
        "{first}\n[[setups.rules]]\nkind = \"executable\"\nvalue = \"Foo.exe\"\nprofile = \"{GAME_PROFILE}\"\n\n[[setups.rules]]\nkind = \"steam_app_id\"\nvalue = \"1245620\"\nprofile = \"{GAME_PROFILE}\"\n"
    );
    text.replace(&first, &more)
}

const SCAN_EXE: &str = "padwight-scan-game";

/// The game config with rules only the scan test's stand-ins match.
fn scan_config() -> String {
    game_config().replace(&format!("value = \"{GAME_EXE}\""), &format!("value = \"{SCAN_EXE}\"")).replace(
        &format!("profile = \"{GAME_PROFILE}\"\n\n[["),
        &format!("profile = \"{GAME_PROFILE}\"\n\n[[setups.rules]]\nkind = \"executable\"\nvalue = \"Scan.exe\"\nprofile = \"{GAME_PROFILE}\"\n\n[["),
    )
}

fn root_with_config() -> PathBuf {
    let root = temp_root();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &config_with_wine_rules());
    root
}

/// `padwight debug <args>` against the config under `root`.
fn debug(root: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_padwight"))
        .arg("debug")
        .args(args)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env_remove("XDG_RUNTIME_DIR")
        .output()
        .unwrap();
    assert!(out.status.success(), "debug {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_plain_binary_is_named_by_its_file() {
    let root = root_with_config();
    let game = Fake::start(&root, GAME_EXE, &[], &[]);
    let text = debug(&root, &["focus", &game.pid().to_string(), "padwight_game"]);
    assert!(text.contains(&format!("program  {GAME_EXE}  path ")), "{text}");
    assert!(text.contains(&format!("rule     {GAME}: Executable “{GAME_EXE}” → profile {GAME_PROFILE}")), "{text}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_wine_process_is_named_by_the_windows_program_in_its_command_line() {
    let root = root_with_config();
    // As Proton starts it: the loader is the binary, the program is an argument with a Z: path.
    let game = Fake::start(&root, "wine64-preloader", &["Z:\\home\\me\\Games\\Foo\\Foo.exe"], &[]);
    let text = debug(&root, &["focus", &game.pid().to_string(), "foo.exe"]);
    assert!(text.contains("program  Foo.exe  path \"Z:\\\\home\\\\me\\\\Games\\\\Foo\\\\Foo.exe\""), "{text}");
    assert!(text.contains(&format!("rule     {GAME}: Executable “Foo.exe” → profile {GAME_PROFILE}")), "{text}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_proton_game_is_found_by_its_steam_app_id_and_class() {
    let root = root_with_config();
    // A Proton game whose command line names a program no rule knows: Steam's environment decides.
    let game = Fake::start(&root, "wine64-preloader", &["C:\\windows\\system32\\other.exe"], &[("SteamAppId", "1245620")]);
    let text = debug(&root, &["focus", &game.pid().to_string(), "steam_app_1245620"]);
    assert!(text.contains("program  other.exe"), "{text}");
    assert!(text.contains("steam    app 1245620"), "{text}");
    assert!(text.contains(&format!("rule     {GAME}: Steam App ID “1245620”")), "{text}");

    // With the environment unreadable (a window from another namespace), the class still gives the id.
    let plain = Fake::start(&root, "wine64-preloader", &["C:\\windows\\system32\\other.exe"], &[]);
    let text = debug(&root, &["focus", &plain.pid().to_string(), "steam_app_1245620"]);
    assert!(text.contains("steam    app 1245620"), "{text}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_process_scan_picks_launcher_started_games() {
    let root = temp_root();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &scan_config());
    let text = debug(&root, &["focus", "processes"]);
    assert!(text.contains("picked   nothing"), "no game runs yet: {text}");

    let plain = Fake::start(&root, SCAN_EXE, &[], &[]);
    let text = debug(&root, &["focus", "processes"]);
    assert!(text.contains(&format!("picked   {GAME}: Executable “{SCAN_EXE}” for {SCAN_EXE} ({})", plain.pid())), "{text}");
    drop(plain);

    let wine = Fake::start(&root, "wine64-preloader", &["Z:\\games\\Scan.exe"], &[]);
    let text = debug(&root, &["focus", "processes"]);
    assert!(text.contains(&format!("{GAME}: Executable “Scan.exe” matches Scan.exe ({})", wine.pid())), "{text}");
    let _ = fs::remove_dir_all(&root);
}
