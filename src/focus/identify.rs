//! Identifies windows and processes from /proc, including Wine/Proton games.

use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use super::base_name;
use crate::ipc::WindowInfo;

/// Builds a [`WindowInfo`] for a window owned by `pid`, reading its process from /proc.
pub fn identify(class: String, title: String, pid: u32) -> WindowInfo {
    let pid = host_pid(pid);
    let mut info = process_info(pid).unwrap_or_default();
    // Proton names its windows `steam_app_<id>` even when the environment is unreadable.
    if info.steam_app_id.is_none() {
        info.steam_app_id = class.strip_prefix("steam_app_").filter(|id| is_app_id(id)).map(str::to_string);
    }
    WindowInfo { class, title, pid, ..info }
}

/// The pid of `pid` as this daemon sees it. A program in a sandbox with its own pid namespace
/// (a game launched from Flatpak Heroic, say) gives its window the pid it has in there, which
/// names no process here or a different one, so look for the process that has that pid in a
/// namespace below ours. A pid that names a process here is taken as it is.
fn host_pid(pid: u32) -> u32 {
    if pid == 0 || Path::new(&format!("/proc/{pid}")).exists() {
        return pid;
    }
    let Ok(entries) = fs::read_dir("/proc") else { return pid };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .find(|&p| fs::read_to_string(format!("/proc/{p}/status")).is_ok_and(|s| nested_pid(&s) == Some(pid)))
        .unwrap_or(pid)
}

/// A process's pid in the innermost of its pid namespaces, from /proc/<pid>/status, when that
/// namespace is not the one we read it from.
fn nested_pid(status: &str) -> Option<u32> {
    let ids: Vec<u32> = status
        .lines()
        .find_map(|l| l.strip_prefix("NSpid:"))?
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    (ids.len() > 1).then(|| *ids.last().unwrap_or(&0))
}

/// Executable and Steam App ID of a process, or `None` if it is gone or not ours.
fn process_info(pid: u32) -> Option<WindowInfo> {
    if pid == 0 {
        return None;
    }
    let dir = PathBuf::from(format!("/proc/{pid}"));
    let cmdline = fs::read(dir.join("cmdline")).ok()?;
    let exe_path = fs::read_link(dir.join("exe")).ok();
    let environ = fs::read(dir.join("environ")).unwrap_or_default();
    let path = program_path(&cmdline, exe_path.as_deref(), || mapped_exe(&dir));
    Some(WindowInfo {
        pid,
        exe: base_name(&path),
        path,
        steam_app_id: steam_app_id(&environ),
        ..WindowInfo::default()
    })
}

/// Path of what a process is running. Wine/Proton processes run a loader binary but put
/// the Windows program (`C:\Games\Foo\foo.exe`) in their command line, so prefer that. A
/// Windows program started by another one can have only its arguments there (`-007`); then the
/// program's file mapped into the Wine loader (`mapped`) names it.
fn program_path(cmdline: &[u8], exe_path: Option<&Path>, mapped: impl FnOnce() -> Option<String>) -> String {
    let args: Vec<String> = cmdline
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let windows_exe = args.iter().find(|a| a.to_ascii_lowercase().ends_with(".exe"));
    if let Some(exe) = windows_exe {
        return exe.clone();
    }
    let loader = exe_path.and_then(|p| p.file_name()).is_some_and(|n| n.to_string_lossy().starts_with("wine"));
    if let Some(exe) = loader.then(mapped).flatten() {
        return exe;
    }
    exe_path
        .map(|p| p.to_string_lossy().trim_end_matches(" (deleted)").to_string())
        .or_else(|| args.first().cloned())
        .unwrap_or_default()
}

/// The first `.exe` file mapped into a process's memory: for a Wine loader, the Windows
/// program it runs.
fn mapped_exe(dir: &Path) -> Option<String> {
    let maps = fs::read_to_string(dir.join("maps")).ok()?;
    first_mapped_exe(&maps)
}

fn first_mapped_exe(maps: &str) -> Option<String> {
    // Each line ends with the file's path, which can hold spaces, after five other fields.
    maps.lines()
        .filter_map(|l| l.splitn(6, char::is_whitespace).nth(5).map(str::trim_start))
        .find(|path| path.to_ascii_lowercase().ends_with(".exe"))
        .map(str::to_string)
}

/// Steam sets these for games it launches (and Proton for Windows games).
fn steam_app_id(environ: &[u8]) -> Option<String> {
    let vars: Vec<(&[u8], &[u8])> = environ
        .split(|b| *b == 0)
        .filter_map(|kv| {
            let eq = kv.iter().position(|b| *b == b'=')?;
            Some((&kv[..eq], &kv[eq + 1..]))
        })
        .collect();
    ["SteamAppId", "SteamGameId", "STEAM_COMPAT_APP_ID"].iter().find_map(|key| {
        vars.iter()
            .find(|(k, _)| *k == key.as_bytes())
            .map(|(_, v)| String::from_utf8_lossy(v).into_owned())
            .filter(|id| is_app_id(id))
    })
}

/// Real app IDs are non-zero numbers (Steam uses 0 for "no game").
fn is_app_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && !id.trim_start_matches('0').is_empty()
}

/// True if this window belongs to padwight itself (the settings GUI).
pub fn is_own_window(info: &WindowInfo) -> bool {
    let Ok(own) = std::env::current_exe() else { return false };
    fs::read_link(format!("/proc/{}/exe", info.pid)).is_ok_and(|exe| exe == own)
}

/// This user's running processes.
pub fn running_processes() -> Vec<WindowInfo> {
    let uid = unsafe { libc::getuid() };
    let Ok(entries) = fs::read_dir("/proc") else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|e| {
            let pid: u32 = e.file_name().to_str()?.parse().ok()?;
            (e.metadata().ok()?.uid() == uid).then_some(pid)
        })
        .filter_map(process_info)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nul_joined(parts: &[&str]) -> Vec<u8> {
        parts.iter().flat_map(|p| p.bytes().chain([0])).collect()
    }

    #[test]
    fn wine_processes_report_the_windows_exe() {
        let cmd = nul_joined(&["C:\\Program Files\\ELDEN RING\\Game\\eldenring.exe"]);
        assert_eq!(program_path(&cmd, Some(Path::new("/usr/bin/wine64-preloader")), || None), "C:\\Program Files\\ELDEN RING\\Game\\eldenring.exe");
        // Some launches keep the loader as argv[0] and pass the program as an argument.
        let cmd = nul_joined(&["/opt/proton/files/bin/wine64", "Z:\\home\\me\\Games\\Foo.EXE"]);
        assert_eq!(program_path(&cmd, None, || None), "Z:\\home\\me\\Games\\Foo.EXE");
    }

    #[test]
    fn wine_programs_without_their_path_report_the_mapped_exe() {
        // PrinceOfPersia_gog.EXE starts gpp.exe, whose command line is only "-007".
        let cmd = nul_joined(&["-007"]);
        let maps = "00400000-00401000 r--p 00000000 00:2a 123 /home/me/Games/Prince of Persia - The Sands of Time/gpp.exe\n\
                    7f000000-7f001000 r--p 00000000 00:2a 456 /usr/lib/libc.so.6\n";
        let loader = Some(Path::new("/opt/proton/files/lib/wine/i386-unix/wine-preloader"));
        assert_eq!(
            program_path(&cmd, loader, || first_mapped_exe(maps)),
            "/home/me/Games/Prince of Persia - The Sands of Time/gpp.exe"
        );
        // Only a Wine loader is asked: a native program keeps its own name.
        assert_eq!(program_path(&cmd, Some(Path::new("/usr/bin/game")), || first_mapped_exe(maps)), "/usr/bin/game");
    }

    #[test]
    fn nested_pid_reads_the_innermost_namespace() {
        assert_eq!(nested_pid("Name:\tgame\nNSpid:\t14173\t3467\n"), Some(3467));
        assert_eq!(nested_pid("Name:\tbash\nNSpid:\t4905\n"), None);
    }

    #[test]
    fn native_processes_report_their_binary() {
        let cmd = nul_joined(&["./factorio", "--fullscreen"]);
        let deleted = Path::new("/home/me/factorio/bin/x64/factorio (deleted)");
        assert_eq!(program_path(&cmd, Some(deleted), || None), "/home/me/factorio/bin/x64/factorio");
        assert_eq!(base_name(&program_path(&cmd, None, || None)), "factorio");
    }

    #[test]
    fn steam_app_id_from_environment_skips_zero() {
        let env = nul_joined(&["HOME=/home/me", "SteamAppId=0", "STEAM_COMPAT_APP_ID=1245620"]);
        assert_eq!(steam_app_id(&env).as_deref(), Some("1245620"));
        assert_eq!(steam_app_id(&nul_joined(&["SteamAppId=0"])), None);
    }

    #[test]
    fn proton_window_class_gives_app_id() {
        let info = identify("steam_app_1245620".into(), "ELDEN RING".into(), 0);
        assert_eq!(info.steam_app_id.as_deref(), Some("1245620"));
        assert_eq!(identify("steam_app_0".into(), String::new(), 0).steam_app_id, None);
    }

    #[test]
    fn identifies_this_test_process() {
        let me = std::process::id();
        let info = identify("test".into(), String::new(), me);
        assert!(!info.exe.is_empty());
        assert!(is_own_window(&info));
        assert!(running_processes().iter().any(|p| p.pid == me));
    }
}
