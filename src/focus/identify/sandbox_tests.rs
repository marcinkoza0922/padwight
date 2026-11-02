//! ENV-4 without the launchers: Flatpak and Snap process trees and their /proc entries, written
//! out as a made-up /proc and run through the window-to-process lookup that `padwight debug focus`
//! and the daemon use. Covers what a game's window reports (a pid from inside the sandbox, a
//! `steam_app_` class, a Wine class), what the lookup then names, whether a rule matches it, and
//! which processes Force Quit would end.
//!
//! The trees are modelled on what the launchers do (bwrap with its own pid namespace for Flatpak,
//! the same namespace for Snap); they were not read from a machine with those launchers installed,
//! so the real Flatpak and Snap Steam, Lutris, Bottles and Heroic runs stay manual (ENV-4).

use std::{fs, path::PathBuf};

use super::*;
use crate::config::{Rule, RuleKind};

/// A made-up /proc under the temp directory, deleted on drop.
struct Proc(PathBuf);

impl Proc {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("padwight-proc-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Proc(root)
    }

    /// A process: its `exe` link, command line and environment, and its pids in each pid namespace
    /// from the host's down (`NSpid`).
    fn add(&self, nspid: &[u32], exe: &str, args: &[&str], env: &[&str]) -> u32 {
        let pid = nspid[0];
        let dir = self.0.join(pid.to_string());
        fs::create_dir_all(&dir).unwrap();
        let nul = |parts: &[&str]| parts.iter().flat_map(|p| p.bytes().chain([0])).collect::<Vec<u8>>();
        fs::write(dir.join("cmdline"), nul(args)).unwrap();
        fs::write(dir.join("environ"), nul(env)).unwrap();
        let ids: Vec<String> = nspid.iter().map(u32::to_string).collect();
        fs::write(dir.join("status"), format!("Name:\t{}\nNSpid:\t{}\n", base_name(exe), ids.join("\t"))).unwrap();
        std::os::unix::fs::symlink(exe, dir.join("exe")).unwrap();
        pid
    }

    fn identify(&self, class: &str, pid: u32) -> WindowInfo {
        identify_in(&self.0, class.into(), "title".into(), pid)
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn matches(kind: RuleKind, value: &str, info: &WindowInfo) -> bool {
    crate::focus::rule_matches_any(&Rule::new(kind, value, "P"), std::slice::from_ref(info))
}

const FLATPAK_STEAM: &str = "/home/me/.var/app/com.valvesoftware.Steam/.local/share/Steam";

#[test]
fn a_flatpak_steam_games_window_with_the_sandboxs_pid_finds_the_game() {
    let proc = Proc::new("flatpak-steam");
    // Steam itself, the sandbox's bwrap, and the game two levels down; inside the sandbox the game is pid 52.
    proc.add(&[3000], "/usr/bin/bwrap", &["bwrap", "--args", "40", "/app/bin/steam"], &[]);
    proc.add(&[3010, 30], "/usr/lib/x86_64-linux-gnu/bwrap", &["bwrap", "--args", "41"], &[]);
    let game = proc.add(&[3100, 52], &format!("{FLATPAK_STEAM}/steamapps/common/Factorio/bin/x64/factorio"), &["factorio"], &["SteamAppId=427520"]);
    // Something unrelated on the host that happens to have pid 52, as small pids nearly always do.
    proc.add(&[52], "/usr/lib/systemd/systemd-journald", &["systemd-journald"], &[]);

    let info = proc.identify("factorio", 52);
    assert_eq!(info.pid, game, "the window's inner pid is mapped to the game on the host");
    assert_eq!(info.exe, "factorio");
    assert_eq!(info.steam_app_id.as_deref(), Some("427520"));
    assert!(matches(RuleKind::Executable, "factorio", &info) && matches(RuleKind::SteamAppId, "427520", &info));
}

#[test]
fn a_proton_game_in_flatpak_steam_is_named_by_its_windows_exe_and_app_id() {
    let proc = Proc::new("flatpak-proton");
    let cmd = "Z:\\home\\me\\.var\\app\\com.valvesoftware.Steam\\.local\\share\\Steam\\steamapps\\common\\ELDEN RING\\Game\\eldenring.exe";
    let game = proc.add(&[4200, 77], "/app/lib/proton/files/bin/wine64-preloader", &[cmd], &["SteamAppId=1245620", "STEAM_COMPAT_APP_ID=1245620"]);
    proc.add(&[77], "/usr/bin/python3", &["python3"], &[]);

    let info = proc.identify("steam_app_1245620", 77);
    assert_eq!(info.pid, game);
    assert_eq!(info.exe, "eldenring.exe");
    assert!(matches(RuleKind::Executable, "eldenring.exe", &info) && matches(RuleKind::SteamAppId, "1245620", &info));
    // And the class alone is enough when the environment can't be read.
    fs::remove_file(proc.0.join("4200/environ")).unwrap();
    assert_eq!(proc.identify("steam_app_1245620", 77).steam_app_id.as_deref(), Some("1245620"));
}

#[test]
fn heroic_lutris_and_bottles_flatpaks_find_the_wine_game_not_their_helper() {
    for (launcher, class, exe) in [("heroic", "gpp.exe", "gpp.exe"), ("lutris", "eldenring.exe", "eldenring.exe"), ("bottles", "witcher3.exe", "witcher3.exe")] {
        let proc = Proc::new("flatpak-wine");
        // Another sandbox's helper has the same inner pid: the one that looks like the window wins,
        // whichever comes first.
        proc.add(&[6001, 120], "/usr/bin/bwrap", &["bwrap", "--args", "9"], &[]);
        let game = proc.add(&[6002, 120], "/opt/wine/bin/wine64-preloader", &[&format!("C:\\Games\\{exe}")], &[]);
        proc.add(&[6003, 120], "/usr/bin/sleep", &["sleep", "9999"], &[]);
        proc.add(&[120], &format!("/usr/bin/{launcher}"), &[launcher], &[]);

        let info = proc.identify(class, 120);
        assert_eq!(info.pid, game, "{launcher}");
        assert_eq!(info.exe, exe, "{launcher}");
        assert!(matches(RuleKind::Executable, exe, &info), "{launcher}");
    }
}

#[test]
fn a_window_that_looks_like_none_of_several_sandboxed_candidates_is_not_guessed() {
    let proc = Proc::new("ambiguous");
    proc.add(&[7001, 33], "/usr/bin/bwrap", &["bwrap"], &[]);
    proc.add(&[7002, 33], "/usr/bin/sleep", &["sleep"], &[]);
    // No process here has pid 33, two sandboxed ones do and neither fits "mystery": the pid stays,
    // so nothing is force quit by mistake.
    assert_eq!(proc.identify("mystery", 33).pid, 33);
}

#[test]
fn snap_steam_shares_the_host_pid_namespace_and_needs_no_mapping() {
    let proc = Proc::new("snap-steam");
    let root = "/home/me/snap/steam/common/.local/share/Steam";
    proc.add(&[9000], "/snap/steam/current/usr/lib/steam/steam", &["steam"], &[]);
    let game = proc.add(&[9100], &format!("{root}/steamapps/common/Hades/Hades"), &["Hades"], &["SteamAppId=1145360"]);
    let info = proc.identify("Hades", game);
    assert_eq!((info.pid, info.exe.as_str()), (game, "Hades"));
    assert_eq!(info.steam_app_id.as_deref(), Some("1145360"));
    assert!(info.path.contains("/snap/steam/common/"), "the path keeps the snap's home: {}", info.path);
    assert!(matches(RuleKind::Executable, "Hades", &info));
}

#[test]
fn debug_focus_processes_lists_every_visible_game_process_with_its_executable() {
    let proc = Proc::new("listing");
    proc.add(&[100], "/usr/bin/bwrap", &["bwrap"], &[]);
    proc.add(&[200, 52], &format!("{FLATPAK_STEAM}/steamapps/common/Factorio/bin/x64/factorio"), &["factorio"], &["SteamAppId=427520"]);
    proc.add(&[300], "/snap/firefox/current/usr/lib/firefox/firefox", &["firefox"], &[]);
    // A directory that is not a process is skipped.
    fs::create_dir_all(proc.0.join("sys")).unwrap();
    let mut exes: Vec<String> = running_processes_in(&proc.0).into_iter().map(|p| format!("{}={}", p.pid, p.exe)).collect();
    exes.sort();
    assert_eq!(exes, ["100=bwrap", "200=factorio", "300=firefox"]);
}

#[test]
fn force_quit_of_a_sandboxed_game_ends_the_game_and_its_children_not_the_launcher() {
    // The tree: Steam (3000) started the sandbox helper (3010), which started the game (3100) and its child (3101).
    // The window's pid is mapped to 3100 first (see above); then Force Quit takes that and below.
    let parents = std::collections::HashMap::from([(3010, 3000), (3100, 3010), (3101, 3100), (3200, 3000)]);
    assert_eq!(crate::quit::tree(3100, &parents, 1), [3101, 3100]);
    // Steam, the helper and a sibling are never in it.
    for other in [3000, 3010, 3200] {
        assert!(!crate::quit::tree(3100, &parents, 1).contains(&other));
    }
    // And a window that resolved to the launcher itself is refused outright.
    for exe in ["steam", "heroic", "lutris", "bottles", "steamwebhelper"] {
        assert!(crate::quit::protected(exe), "{exe}");
    }
}
