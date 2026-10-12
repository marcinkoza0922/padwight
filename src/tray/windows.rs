//! Tray icon in the Windows notification area: a hidden window on a thread of its own owns the
//! icon and gets its clicks. Left click opens the settings, right click the menu.

use std::{
    cell::RefCell,
    sync::mpsc::{self, Sender},
    thread,
};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW},
            WindowsAndMessaging::{
                AppendMenuW, CreateIcon, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
                DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics, HICON, MF_CHECKED, MF_SEPARATOR, MF_STRING,
                MF_UNCHECKED, MSG, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SM_CXSMICON,
                SetForegroundWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WINDOW_EX_STYLE, WM_APP, WM_CLOSE,
                WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
            },
        },
    },
    core::{PCWSTR, w},
};

use crate::icon;

/// A tray menu choice, for the daemon to carry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    ToggleEnabled,
    OpenSettings,
    Quit,
}

/// What the tray displays, kept current by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub enabled: bool,
    pub profile: String,
}

/// Whether the daemon should show a tray icon: always, on Windows.
pub fn wanted() -> bool {
    true
}

/// The icon's clicks arrive as this message.
const CALLBACK: u32 = WM_APP + 1;
/// `Shown` changed; the new one is waiting in the channel.
const UPDATE: u32 = WM_APP + 2;

const ITEM_REMAPPING: usize = 1;
const ITEM_SETTINGS: usize = 2;
const ITEM_QUIT: usize = 3;

/// A running tray icon. Dropping it removes it.
pub struct TrayIcon {
    hwnd: isize,
    updates: Sender<Shown>,
    shown: Shown,
}

impl TrayIcon {
    /// Puts the icon on the tray, reporting menu choices through `on_command`. `None` if the
    /// window that owns it can't be made.
    pub fn start(shown: Shown, on_command: impl Fn(Command) + Send + 'static) -> Option<Self> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (updates, updates_rx) = mpsc::channel();
        let first = shown.clone();
        thread::spawn(move || {
            let Some(hwnd) = create_window() else {
                let _ = ready_tx.send(None);
                return;
            };
            STATE.with(|s| {
                *s.borrow_mut() = Some(State { hwnd, shown: first, on_command: Box::new(on_command), updates: updates_rx })
            });
            add_icon(hwnd);
            let _ = ready_tx.send(Some(hwnd.0 as isize));
            let mut msg = MSG::default();
            // SAFETY: a plain message loop for the window this thread made.
            while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
                unsafe { DispatchMessageW(&msg) };
            }
        });
        let hwnd = ready_rx.recv().ok().flatten()?;
        Some(Self { hwnd, updates, shown })
    }

    /// Updates the icon's tooltip and menu if `shown` differs from what it shows.
    pub fn show(&mut self, shown: &Shown) {
        if &self.shown == shown {
            return;
        }
        self.shown = shown.clone();
        if self.updates.send(shown.clone()).is_ok() {
            // SAFETY: posting to the window the tray thread owns; a gone window just fails.
            let _ = unsafe { PostMessageW(Some(HWND(self.hwnd as *mut _)), UPDATE, WPARAM(0), LPARAM(0)) };
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        // The window removes its icon as it closes.
        // SAFETY: posting to the window the tray thread owns.
        let _ = unsafe { PostMessageW(Some(HWND(self.hwnd as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        // Give the tray thread a moment, since the daemon exits right after.
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// The tray thread's side.
struct State {
    hwnd: HWND,
    shown: Shown,
    on_command: Box<dyn Fn(Command)>,
    updates: mpsc::Receiver<Shown>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn create_window() -> Option<HWND> {
    // SAFETY: registering a class whose procedure is a plain function, and making a window of it
    // that is never shown.
    unsafe {
        let instance = GetModuleHandleW(None).ok()?;
        let class = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance.into(), lpszClassName: w!("PadwightTray"), ..Default::default() };
        RegisterClassW(&class);
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("PadwightTray"),
            w!("Padwight"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .ok()
    }
}

/// The icon's data: its window, message, picture and tooltip.
fn icon_data(hwnd: HWND, shown: &Shown) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: CALLBACK,
        hIcon: picture(),
        ..Default::default()
    };
    let state = if shown.enabled { "Remapping on" } else { "Remapping off" };
    let tip = if shown.profile.is_empty() { format!("Padwight: {state}") } else { format!("Padwight: {state}, {}", shown.profile) };
    for (dst, src) in data.szTip.iter_mut().zip(tip.encode_utf16().take(127)) {
        *dst = src;
    }
    data
}

fn add_icon(hwnd: HWND) {
    STATE.with(|s| {
        if let Some(state) = &*s.borrow() {
            // SAFETY: the data is valid for the call.
            let _ = unsafe { Shell_NotifyIconW(NIM_ADD, &icon_data(hwnd, &state.shown)) };
        }
    });
}

/// The app icon at the tray's size, with its transparency.
fn picture() -> HICON {
    // SAFETY: GetSystemMetrics has no preconditions.
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
    let rgba = icon::rgba(size);
    let bgra: Vec<u8> = rgba.as_chunks::<4>().0.iter().flat_map(|px| [px[2], px[1], px[0], px[3]]).collect();
    let mask = vec![0u8; (size * size / 8) as usize];
    // SAFETY: the bits are a size x size picture at 32 bits per pixel, and a mask of 1 bit.
    unsafe { CreateIcon(None, size as i32, size as i32, 1, 32, mask.as_ptr(), bgra.as_ptr()) }.unwrap_or_default()
}

/// Shows the menu at the pointer and returns the chosen command.
fn menu(hwnd: HWND, enabled: bool) -> Option<Command> {
    // SAFETY: the menu is made, shown and destroyed here; the window is ours.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let check = if enabled { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(menu, MF_STRING | check, ITEM_REMAPPING, w!("Remapping"));
        let _ = AppendMenuW(menu, MF_STRING, ITEM_SETTINGS, w!("Open settings"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ITEM_QUIT, w!("Quit"));
        let mut at = POINT::default();
        let _ = GetCursorPos(&mut at);
        // The menu closes when it loses focus only if its window is in front.
        let _ = SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, at.x, at.y, None, hwnd, None).0 as usize;
        let _ = DestroyMenu(menu);
        match chosen {
            ITEM_REMAPPING => Some(Command::ToggleEnabled),
            ITEM_SETTINGS => Some(Command::OpenSettings),
            ITEM_QUIT => Some(Command::Quit),
            _ => None,
        }
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Explorer announces itself when it restarts; the icon has to be added again.
    // SAFETY: registering a well-known message name.
    let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    match msg {
        CALLBACK => {
            let enabled = STATE.with(|s| s.borrow().as_ref().is_some_and(|state| state.shown.enabled));
            let command = match (lparam.0 & 0xffff) as u32 {
                WM_LBUTTONUP => Some(Command::OpenSettings),
                WM_RBUTTONUP | WM_CONTEXTMENU => menu(hwnd, enabled),
                _ => None,
            };
            if let Some(command) = command {
                STATE.with(|s| {
                    if let Some(state) = &*s.borrow() {
                        (state.on_command)(command);
                    }
                });
            }
            LRESULT(0)
        }
        UPDATE => {
            STATE.with(|s| {
                if let Some(state) = &mut *s.borrow_mut() {
                    while let Ok(shown) = state.updates.try_recv() {
                        state.shown = shown;
                    }
                    // SAFETY: the data is valid for the call.
                    let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &icon_data(state.hwnd, &state.shown)) };
                }
            });
            LRESULT(0)
        }
        WM_CLOSE => {
            let data = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
            // SAFETY: removing our own icon and destroying our own window.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ if msg == taskbar_created => {
            add_icon(hwnd);
            LRESULT(0)
        }
        // SAFETY: the default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
