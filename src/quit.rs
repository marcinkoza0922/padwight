//! Force-quitting the focused game: ends its process and everything it started.

use std::{collections::HashMap, time::Duration};
#[cfg(target_os = "linux")]
use std::{fs, thread};

/// How long a game gets to exit after it is asked to before it is killed.
const GRACE: Duration = Duration::from_secs(3);

/// Programs that must never be force quit: the desktop itself, so a stray hold with no
/// window in front (or the shell focused) can't take the session down.
const PROTECTED: &[&str] = &[
    "plasmashell", "kwin_wayland", "kwin_wayland_wrapper", "kwin_x11", "krunner", "xorg", "xwayland", "gnome-shell",
    "mutter", "sway", "hyprland", "weston", "labwc", "river", "niri", "cosmic-comp", "wayfire", "systemd", "sddm",
    "gdm", "lightdm", "login", "pipewire", "wireplumber", "padwight",
];

/// Whether a window of this executable is off limits to force quit.
pub fn protected(exe: &str) -> bool {
    let exe = exe.trim().to_ascii_lowercase();
    exe.is_empty() || PROTECTED.contains(&exe.as_str())
}

/// `pid` and all its descendants (children first, so a launcher can't respawn them), given
/// each process's parent. `me` (this program, should a game have started it) and what it started
/// are left out.
fn tree(pid: u32, parents: &HashMap<u32, u32>, me: u32) -> Vec<u32> {
    if pid == me {
        return Vec::new();
    }
    let mut found = vec![pid];
    let mut i = 0;
    while i < found.len() {
        let parent = found[i];
        let mut children: Vec<u32> = parents.iter().filter(|(_, p)| **p == parent).map(|(c, _)| *c).collect();
        children.sort_unstable();
        children.retain(|c| *c != me && !found.contains(c));
        found.extend(children);
        i += 1;
    }
    found.reverse();
    found
}

/// Every process's parent, from /proc.
#[cfg(target_os = "linux")]
fn parents() -> HashMap<u32, u32> {
    let Ok(entries) = fs::read_dir("/proc") else { return HashMap::new() };
    entries
        .flatten()
        .filter_map(|e| {
            let pid: u32 = e.file_name().to_str()?.parse().ok()?;
            let stat = fs::read_to_string(e.path().join("stat")).ok()?;
            // "pid (comm) S ppid ...": the name can hold spaces and parentheses, so count
            // from its closing one.
            let rest = &stat[stat.rfind(')')? + 1..];
            let ppid: u32 = rest.split_whitespace().nth(1)?.parse().ok()?;
            Some((pid, ppid))
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn signal(pids: &[u32], sig: libc::c_int) {
    for pid in pids {
        // SAFETY: `kill` only sends a signal.
        unsafe { libc::kill(*pid as libc::pid_t, sig) };
    }
}

/// Whether the process `pid` is still running (a zombie, waiting to be reaped, is not).
#[cfg(target_os = "linux")]
pub fn is_running(pid: u32) -> bool {
    let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else { return false };
    stat.rfind(')').and_then(|i| stat[i + 1..].split_whitespace().next()).is_some_and(|state| state != "Z" && state != "X")
}

#[cfg(target_os = "linux")]
fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the process exists.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Asks the game's process and its descendants to exit, and kills what is still there after a
/// few seconds. Returns at once: reading every process's parent and the kill follow on a
/// thread. Does nothing for pid 0, 1 or this program.
#[cfg(target_os = "linux")]
pub fn force_quit(pid: u32) {
    if pid <= 1 {
        return;
    }
    thread::spawn(move || {
        let pids = tree(pid, &parents(), std::process::id());
        signal(&pids, libc::SIGTERM);
        thread::sleep(GRACE);
        let left: Vec<u32> = pids.into_iter().filter(|p| alive(*p)).collect();
        signal(&left, libc::SIGKILL);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_lists_descendants_children_first() {
        // 10 → 11 → 13, 10 → 12; 20 is unrelated.
        let parents = HashMap::from([(11, 10), (12, 10), (13, 11), (20, 1)]);
        assert_eq!(tree(10, &parents, 1), [13, 12, 11, 10]);
        assert_eq!(tree(20, &parents, 1), [20]);
        assert_eq!(tree(99, &parents, 1), [99]);
    }

    #[test]
    fn a_tree_never_holds_this_program_or_what_it_started() {
        // 10 → 11 (this program) → 13, 10 → 12.
        let parents = HashMap::from([(11, 10), (12, 10), (13, 11)]);
        assert_eq!(tree(10, &parents, 11), [12, 10]);
        assert_eq!(tree(11, &parents, 11), [] as [u32; 0]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn force_quit_ends_a_process_and_its_children() {
        // A shell that starts a child, reports its ID, and waits for it. The shell's own exit
        // status isn't checked: once its child is gone its `wait` returns and it can exit 0
        // before the signal for the shell itself arrives.
        let mut shell = std::process::Command::new("sh")
            .args(["-c", "sleep 60 & echo $!; wait"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(shell.stdout.take().unwrap()), &mut line).unwrap();
        let child: u32 = line.trim().parse().unwrap();
        force_quit(shell.id());
        shell.wait().unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while is_running(child) && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(!is_running(child));
    }

    #[test]
    fn the_desktop_is_protected() {
        for exe in ["plasmashell", "KWin_Wayland", "Hyprland", "", "padwight"] {
            assert!(protected(exe), "{exe}");
        }
        assert!(!protected("eldenring.exe") && !protected("firefox"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn running_processes_are_told_from_gone_ones() {
        assert!(is_running(std::process::id()));
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert!(!is_running(pid), "reaped");
        assert!(!is_running(u32::MAX - 1));
    }

    #[test]
    fn this_process_is_found_under_its_parent() {
        let me = std::process::id();
        let parent = parents().get(&me).copied();
        assert!(parent.is_some_and(|p| p > 0));
        assert!(alive(me));
    }
}
