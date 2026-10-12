//! Focus tracking on Windows: a WinEvent hook reports each change of foreground window, and
//! windows and processes are identified from their executables. Steam games are recognized by
//! where they're installed (`steamapps\common\<folder>`), matched against Steam's manifests.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND},
        System::{
            Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS},
            Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
        },
        UI::{
            Accessibility::{HWINEVENTHOOK, SetWinEventHook},
            WindowsAndMessaging::{
                DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GetClassNameW, GetForegroundWindow, GetMessageW, GetWindowTextW,
                GetWindowThreadProcessId, MSG, OBJID_WINDOW, WINEVENT_OUTOFCONTEXT,
            },
        },
    },
    core::PWSTR,
};

use super::{FocusEvent, Notify, Reported, Reporter, report};
use crate::{ipc::FocusBackend, ipc::WindowInfo, monitor::log};

thread_local! {
    /// Where the hook, which runs on the tracking thread, sends what it sees.
    static NOTIFY: RefCell<Option<Notify>> = const { RefCell::new(None) };
}

/// Starts focus tracking on a thread of its own, which the hook's events are delivered to.
pub fn spawn(notify: impl Fn(FocusEvent) + Send + Sync + 'static) {
    let notify: Notify = Arc::new(notify);
    thread::spawn(move || {
        let mut reporter = Reporter { notify: notify.clone(), backend: None };
        NOTIFY.with(|n| *n.borrow_mut() = Some(notify));
        // SAFETY: the callback is a plain function that stays valid for the life of the thread,
        // which is the life of the process.
        let hook = unsafe {
            SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_foreground), 0, 0, WINEVENT_OUTOFCONTEXT)
        };
        if hook.is_invalid() {
            log!("focus tracking: cannot hook foreground changes: {}", std::io::Error::last_os_error());
            reporter.backend(FocusBackend::ProcessScan);
            return;
        }
        reporter.backend(FocusBackend::Windows);
        // SAFETY: GetForegroundWindow has no preconditions.
        report_window(unsafe { GetForegroundWindow() });
        let mut msg = MSG::default();
        // SAFETY: a plain message loop on this thread; out-of-context hooks are delivered by it.
        while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
            unsafe { DispatchMessageW(&msg) };
        }
    });
}

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if object == OBJID_WINDOW.0 {
        report_window(hwnd);
    }
}

/// Tells the daemon `hwnd` has focus, or that nothing does.
fn report_window(hwnd: HWND) {
    NOTIFY.with(|notify| {
        let notify = notify.borrow();
        let Some(notify) = notify.as_ref() else { return };
        if hwnd.is_invalid() {
            notify(FocusEvent::Unfocused);
            return;
        }
        let mut pid = 0;
        let mut class = [0u16; 256];
        let mut title = [0u16; 512];
        // SAFETY: each call writes into the buffer it is given, up to its length.
        let (class, title) = unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let class_len = GetClassNameW(hwnd, &mut class).max(0) as usize;
            let title_len = GetWindowTextW(hwnd, &mut title).max(0) as usize;
            (String::from_utf16_lossy(&class[..class_len]), String::from_utf16_lossy(&title[..title_len]))
        };
        report(notify, Reported { class, title, pid });
    });
}

/// Builds a [`WindowInfo`] for a window owned by `pid`, from its process's executable.
pub fn identify(class: String, title: String, pid: u32) -> WindowInfo {
    let info = process_path(pid).map(|path| process_info(pid, &path)).unwrap_or_default();
    WindowInfo { class, title, pid, ..info }
}

fn process_info(pid: u32, path: &Path) -> WindowInfo {
    WindowInfo {
        pid,
        exe: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        steam_app_id: steam_app_id(path),
        ..WindowInfo::default()
    }
}

/// The full path of the program process `pid` runs, if we may look.
fn process_path(pid: u32) -> Option<PathBuf> {
    if pid == 0 {
        return None;
    }
    // SAFETY: the handle is closed below; the buffer is as long as the size passed.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(process);
        ok.then(|| PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
    }
}

/// The Steam App ID of a game installed at `exe`: Steam installs each game in
/// `steamapps\common\<folder>`, and the library's `appmanifest_<id>.acf` names that folder.
fn steam_app_id(exe: &Path) -> Option<String> {
    let parts: Vec<_> = exe.components().collect();
    let at = parts.windows(2).position(|w| {
        w[0].as_os_str().eq_ignore_ascii_case("steamapps") && w[1].as_os_str().eq_ignore_ascii_case("common")
    })?;
    let folder = parts.get(at + 2)?.as_os_str().to_string_lossy();
    let steamapps: PathBuf = parts[..=at].iter().collect();
    std::fs::read_dir(&steamapps).ok()?.flatten().find_map(|entry| {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
            return None;
        }
        let text = std::fs::read_to_string(entry.path()).ok()?;
        match crate::launchers::parse_manifest(&text) {
            (Some(id), Some(dir)) if dir.eq_ignore_ascii_case(&folder) => Some(id),
            _ => None,
        }
    })
}

/// True if this window belongs to padwight itself (the settings window).
pub fn is_own_window(info: &WindowInfo) -> bool {
    let Ok(own) = std::env::current_exe() else { return false };
    process_path(info.pid).is_some_and(|exe| exe == own)
}

/// The running processes we may look at: other users' and the system's are left out.
pub fn running_processes() -> Vec<WindowInfo> {
    // SAFETY: the snapshot handle is closed below; each entry is a valid PROCESSENTRY32W with
    // its size set, as the API asks.
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return Vec::new() };
        let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut pids = Vec::new();
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            pids.push(entry.th32ProcessID);
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        pids.into_iter().filter_map(|pid| process_path(pid).map(|path| process_info(pid, &path))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_this_test_process() {
        let me = std::process::id();
        let info = identify("test".into(), String::new(), me);
        assert!(info.exe.to_ascii_lowercase().ends_with(".exe"), "{}", info.exe);
        assert!(is_own_window(&info));
        assert!(running_processes().iter().any(|p| p.pid == me));
    }

    #[test]
    fn steam_games_are_found_by_their_install_folder() {
        let root = std::env::temp_dir().join(format!("padwight-steam-{}", std::process::id()));
        let apps = root.join("steamapps");
        std::fs::create_dir_all(apps.join("common").join("ELDEN RING").join("Game")).unwrap();
        let manifest = "\"AppState\"\n{\n\t\"appid\"\t\t\"1245620\"\n\t\"installdir\"\t\t\"ELDEN RING\"\n}\n";
        std::fs::write(apps.join("appmanifest_1245620.acf"), manifest).unwrap();
        let exe = apps.join("common").join("ELDEN RING").join("Game").join("eldenring.exe");
        assert_eq!(steam_app_id(&exe).as_deref(), Some("1245620"));
        assert_eq!(steam_app_id(&root.join("other").join("game.exe")), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
