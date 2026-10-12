//! The overlay window on Windows: a layered window over the monitor the game is on, drawn with
//! the CPU renderer and handed to Windows with its transparency. It never takes focus or clicks
//! (`WS_EX_NOACTIVATE`, `WS_EX_TRANSPARENT`), so the game keeps both, and it hides while there's
//! nothing to draw.

use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Instant,
};

use anyhow::{Context, Result};
use iced::{Color, Length, Size, widget::stack};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
            MonitorFromWindow, ReleaseDC, SelectObject,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow, GetMessageW, KillTimer,
                PostMessageW, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SetTimer,
                SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WM_APP, WM_CLOSE, WM_DESTROY, WM_TIMER, WNDCLASSW,
                WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, HWND_TOPMOST,
            },
        },
    },
    core::w,
};

use super::{
    OverlayFrame,
    timing::{timing_log, unix_ns},
    ui::{Overlay, layers},
};
use crate::ipc::{self, Request, UnixStream};

/// A frame from the daemon is waiting in the channel.
const FRAME: u32 = WM_APP + 1;
/// The animation timer's ID, and how often it fires while something moves.
const ANIMATION: usize = 1;
const ANIMATION_MS: u32 = 16;

/// The window's side of things, on the thread that owns it.
struct Window {
    hwnd: HWND,
    overlay: Overlay,
    frames: Receiver<OverlayFrame>,
    /// Where the window is, in pixels, and its monitor's scale.
    at: POINT,
    size: SIZE,
    scale: f32,
    visible: bool,
    animating: bool,
}

thread_local! {
    static WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
}

pub fn run() -> Result<()> {
    // SAFETY: called before any window exists; sizes are then in real pixels.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    let hwnd = create_window()?;
    let (tx, frames) = mpsc::channel();
    let stream = UnixStream::connect(ipc::socket_path()).context("the daemon isn't running")?;
    let mut request = serde_json::to_string(&Request::WatchOverlay)?;
    request.push('\n');
    (&stream).write_all(request.as_bytes())?;
    let target = hwnd.0 as isize;
    thread::spawn(move || read_frames(&stream, &tx, target));
    WINDOW.with(|w| {
        *w.borrow_mut() = Some(Window {
            hwnd,
            overlay: Overlay::new(),
            frames,
            at: POINT::default(),
            size: SIZE::default(),
            scale: 1.0,
            visible: false,
            animating: false,
        });
    });
    let mut msg = Default::default();
    // SAFETY: a plain message loop for the window this thread made.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        unsafe { DispatchMessageW(&msg) };
    }
    Ok(())
}

/// Passes the daemon's frames to the window until the daemon goes away, then closes it.
fn read_frames(stream: &UnixStream, tx: &Sender<OverlayFrame>, hwnd: isize) {
    let hwnd = HWND(hwnd as *mut _);
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        if let Ok(frame) = serde_json::from_str::<OverlayFrame>(&line) {
            if tx.send(frame).is_err() {
                return;
            }
            // SAFETY: posting to the overlay window; a gone window just fails.
            let _ = unsafe { PostMessageW(Some(hwnd), FRAME, WPARAM(0), LPARAM(0)) };
        }
    }
    // SAFETY: as above.
    let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

fn create_window() -> Result<HWND> {
    // SAFETY: registering a class whose procedure is a plain function, and making a hidden
    // window of it.
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance.into(), lpszClassName: w!("PadwightOverlay"), ..Default::default() };
        RegisterClassW(&class);
        let style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        Ok(CreateWindowExW(style, w!("PadwightOverlay"), w!("Padwight overlay"), WS_POPUP, 0, 0, 1, 1, None, None, Some(instance.into()), None)?)
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        FRAME => {
            with_window(|w| {
                while let Ok(frame) = w.frames.try_recv() {
                    w.overlay.receive(frame);
                }
                w.refresh();
            });
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == ANIMATION => {
            with_window(|w| {
                w.overlay.prune(Instant::now());
                w.refresh();
            });
            LRESULT(0)
        }
        WM_CLOSE => {
            // SAFETY: destroying our own window.
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: the default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn with_window(f: impl FnOnce(&mut Window)) {
    WINDOW.with(|w| {
        if let Some(window) = &mut *w.borrow_mut() {
            f(window);
        }
    });
}

impl Window {
    /// Shows what the overlay has at this moment, hides the window when that's nothing, and
    /// keeps the animation timer running while something moves.
    fn refresh(&mut self) {
        let now = Instant::now();
        if !self.overlay.shown() {
            if self.visible {
                // SAFETY: hiding our own window.
                let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
                self.visible = false;
            }
            self.animate(false);
            return;
        }
        if !self.visible {
            self.cover_game_monitor();
        }
        if let Err(e) = self.draw(now) {
            crate::monitor::log!("overlay: {e:#}");
        }
        if !self.visible {
            // SAFETY: showing our own window without activating it.
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
            self.visible = true;
        }
        self.animate(self.overlay.busy(now));
    }

    fn animate(&mut self, on: bool) {
        if on == self.animating {
            return;
        }
        // SAFETY: a timer on our own window.
        unsafe {
            if on {
                SetTimer(Some(self.hwnd), ANIMATION, ANIMATION_MS, None);
            } else {
                let _ = KillTimer(Some(self.hwnd), ANIMATION);
            }
        }
        self.animating = on;
    }

    /// Moves the window over the whole monitor the focused window is on.
    fn cover_game_monitor(&mut self) {
        // SAFETY: plain queries, and moving our own window without activating it.
        unsafe {
            let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return;
            }
            let r = info.rcMonitor;
            self.at = POINT { x: r.left, y: r.top };
            self.size = SIZE { cx: r.right - r.left, cy: r.bottom - r.top };
            let (mut dpi, mut _dpi_y) = (96, 96);
            let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut _dpi_y);
            self.scale = dpi as f32 / 96.0;
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), r.left, r.top, self.size.cx, self.size.cy, SWP_NOACTIVATE);
        }
    }

    /// Draws the overlay's layers and puts them on the window.
    fn draw(&self, now: Instant) -> Result<()> {
        let started = Instant::now();
        let display = Size::new(self.size.cx as f32 / self.scale, self.size.cy as f32 / self.scale);
        let screen = stack(layers::<()>(&self.overlay, display, now)).width(Length::Fill).height(Length::Fill);
        let image = crate::snapshot::render(screen.into(), display, self.scale, Color::TRANSPARENT)?;
        timing_log(&format!("built {} {} {}x{}", unix_ns(), started.elapsed().as_nanos(), display.width, display.height));
        self.present(image.width, image.height, &image.rgba)
    }

    /// Hands a premultiplied RGBA picture to Windows as the window's contents.
    fn present(&self, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
        // SAFETY: the DIB section is `width` x `height` at 32 bits, as big as the copy into it;
        // every GDI object is selected out and freed before returning.
        unsafe {
            let screen = GetDC(None);
            let memory = CreateCompatibleDC(Some(screen));
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0);
            let result = match bitmap {
                Ok(bitmap) => {
                    let pixels = std::slice::from_raw_parts_mut(bits.cast::<u8>(), rgba.len());
                    for (dst, src) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(rgba.as_chunks::<4>().0) {
                        *dst = [src[2], src[1], src[0], src[3]];
                    }
                    let old = SelectObject(memory, bitmap.into());
                    let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
                    let size = SIZE { cx: width as i32, cy: height as i32 };
                    let updated = UpdateLayeredWindow(
                        self.hwnd,
                        Some(screen),
                        Some(&self.at),
                        Some(&size),
                        Some(memory),
                        Some(&POINT::default()),
                        Default::default(),
                        Some(&blend),
                        ULW_ALPHA,
                    );
                    SelectObject(memory, old);
                    let _ = DeleteObject(bitmap.into());
                    updated.context("updating the overlay window")
                }
                Err(e) => Err(e).context("making the overlay's picture"),
            };
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
            result
        }
    }
}
