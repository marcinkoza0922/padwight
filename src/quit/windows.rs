//! Force-quitting on Windows: windows are asked to close, as their close button would, and
//! whatever is still running after the grace period is terminated.

use std::{collections::HashMap, thread};

use windows::Win32::{
    Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM, STILL_ACTIVE, WPARAM},
    System::{
        Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS},
        Threading::{
            GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
            TerminateProcess,
        },
    },
    UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE},
};
use windows::core::BOOL;

use super::{GRACE, started_after_parents, tree};

/// Every process's parent, where the link still holds (see [`started_after_parents`]).
pub(super) fn parents() -> HashMap<u32, u32> {
    let mut listed = Vec::new();
    // SAFETY: the snapshot handle is closed below; the entry has its size set, as the API asks.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return HashMap::new() };
        let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            listed.push((entry.th32ProcessID, entry.th32ParentProcessID));
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    started_after_parents(&listed, started)
}

/// When the process `pid` started, in 100 ns ticks; `None` when it can't be opened.
fn started(pid: u32) -> Option<u64> {
    // SAFETY: the handle is closed below; the times are written into locals.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let (mut created, mut exited, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
        let read = GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user).is_ok();
        let _ = CloseHandle(process);
        read.then(|| (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }
}

/// Whether the process `pid` is still running.
pub fn is_running(pid: u32) -> bool {
    // SAFETY: the handle is closed below.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let running = still_active(process);
        let _ = CloseHandle(process);
        running
    }
}

/// Whether an opened process has yet to exit.
fn still_active(process: HANDLE) -> bool {
    let mut code = 0;
    // SAFETY: the exit code is written into a local; a handle that doesn't allow it just fails.
    unsafe { GetExitCodeProcess(process, &mut code) }.is_ok() && code == STILL_ACTIVE.0 as u32
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

/// Asks the game's windows, and those of the processes it started, to close, and terminates
/// what is still there after a few seconds. Returns at once; the rest follows on a thread. Does
/// nothing for the system's pids 0 and 4, or this program.
pub fn force_quit(pid: u32) {
    if pid == 0 || pid == 4 {
        return;
    }
    thread::spawn(move || {
        let pids = tree(pid, &parents(), std::process::id());
        // Opened before anything is asked to close: while a handle is open its process's ID
        // can't go to a new one, so what is terminated later is what was asked.
        // SAFETY: each handle is closed below.
        let opened: Vec<HANDLE> = pids
            .iter()
            .filter_map(|&pid| unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok())
            .collect();
        close_windows(&pids);
        thread::sleep(GRACE);
        for process in opened {
            // SAFETY: handles opened above, closed once.
            unsafe {
                if still_active(process) {
                    let _ = TerminateProcess(process, 1);
                }
                let _ = CloseHandle(process);
            }
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
