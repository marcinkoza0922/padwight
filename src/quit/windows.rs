//! Force-quitting on Windows: windows are asked to close, as their close button would, and
//! whatever is still running after the grace period is terminated.

use std::{collections::HashMap, thread};

use windows::Win32::{
    Foundation::{CloseHandle, HWND, LPARAM, STILL_ACTIVE, WPARAM},
    System::{
        Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS},
        Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess,
        },
    },
    UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE},
};
use windows::core::BOOL;

use super::{GRACE, tree};

/// Every process's parent.
pub(super) fn parents() -> HashMap<u32, u32> {
    let mut parents = HashMap::new();
    // SAFETY: the snapshot handle is closed below; the entry has its size set, as the API asks.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return parents };
        let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    parents
}

/// Whether the process `pid` is still running.
pub fn is_running(pid: u32) -> bool {
    // SAFETY: the handle is closed below; the exit code is written into a local.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut code = 0;
        let running = GetExitCodeProcess(process, &mut code).is_ok() && code == STILL_ACTIVE.0 as u32;
        let _ = CloseHandle(process);
        running
    }
}

pub(super) fn alive(pid: u32) -> bool {
    is_running(pid)
}

/// Asks every top-level window of these processes to close.
fn close_windows(pids: &[u32]) {
    unsafe extern "system" fn ask(hwnd: HWND, pids: LPARAM) -> BOOL {
        // SAFETY: `pids` is the slice `close_windows` passed, alive for the whole enumeration.
        let pids = unsafe { &*(pids.0 as *const Vec<u32>) };
        let mut pid = 0;
        // SAFETY: plain calls on a window handle Windows gave us.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pids.contains(&pid) {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        true.into()
    }
    let pids = pids.to_vec();
    // SAFETY: `pids` outlives the call, which is synchronous.
    let _ = unsafe { EnumWindows(Some(ask), LPARAM(&pids as *const Vec<u32> as isize)) };
}

fn terminate(pid: u32) {
    // SAFETY: the handle is closed below.
    unsafe {
        if let Ok(process) = OpenProcess(PROCESS_TERMINATE, false, pid) {
            let _ = TerminateProcess(process, 1);
            let _ = CloseHandle(process);
        }
    }
}

/// Asks the game's windows, and those of the processes it started, to close, and terminates
/// what is still there after a few seconds. Returns at once; the rest follows on a thread. Does
/// nothing for the system's pids 0 and 4, or this program.
pub fn force_quit(pid: u32) {
    if pid == 0 || pid == 4 || pid == std::process::id() {
        return;
    }
    let pids = tree(pid, &parents());
    close_windows(&pids);
    thread::spawn(move || {
        thread::sleep(GRACE);
        for pid in pids.into_iter().filter(|p| alive(*p)) {
            terminate(pid);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn force_quit_ends_a_process_and_its_children() {
        // A console program with no window: it is terminated after the grace period.
        let mut shell = std::process::Command::new("cmd").args(["/C", "ping -n 60 127.0.0.1 > NUL"]).spawn().unwrap();
        thread::sleep(Duration::from_millis(300));
        force_quit(shell.id());
        let status = shell.wait().unwrap();
        assert!(!status.success());
    }

    #[test]
    fn running_processes_are_told_from_gone_ones() {
        assert!(is_running(std::process::id()));
        let mut child = std::process::Command::new("cmd").args(["/C", "exit 0"]).spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        drop(child);
        assert!(!is_running(pid), "exited");
        assert!(!is_running(u32::MAX - 1));
    }
}
