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
use tiny_skia::PixmapMut;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, HBITMAP, HDC, HGDIOBJ, MONITOR_DEFAULTTOPRIMARY,
            MONITORINFO, MonitorFromWindow, ReleaseDC, SelectObject,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow, GetMessageW, KillTimer,
                PostMessageW, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SetTimer,
                SetWindowPos, ShowWindow, ULW_ALPHA, UPDATELAYEREDWINDOWINFO, UpdateLayeredWindowIndirect, WM_APP, WM_CLOSE, WM_DESTROY, WM_TIMER, WNDCLASSW,
                WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, HWND_TOPMOST,
            },
        },
    },
    core::w,
};

use super::{
    OverlayFrame,
    timing::{timing_log, unix_ns},
    render::{Damage, Patch, Point, Screen, swap_red_blue},
    ui::Overlay,
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
    /// The renderer, which keeps its panels' pictures and what the last frames covered.
    screen: Screen,
    /// The window's picture, kept while it is shown: it always holds the last frame drawn.
    dib: Option<Dib>,
    /// Whether Windows has the picture in `dib` as it stands.
    presented: bool,
}

/// A 32-bit top-down DIB section selected into its own memory DC: premultiplied BGRA, as
/// `UpdateLayeredWindowIndirect` wants it.
struct Dib {
    memory: HDC,
    bitmap: HBITMAP,
    /// What the DC held before the bitmap went in.
    old: HGDIOBJ,
    bits: *mut u8,
    width: u32,
    height: u32,
}

impl Dib {
    fn new(width: u32, height: u32) -> Result<Dib> {
        // SAFETY: a DC made from the screen's, and a DIB section of `width` x `height` at 32
        // bits (zeroed by Windows) selected into it; `Drop` undoes both.
        unsafe {
            let screen = GetDC(None);
            let memory = CreateCompatibleDC(Some(screen));
            ReleaseDC(None, screen);
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
            match CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(bitmap) => {
                    let old = SelectObject(memory, bitmap.into());
                    Ok(Dib { memory, bitmap, old, bits: bits.cast(), width, height })
                }
                Err(e) => {
                    let _ = DeleteDC(memory);
                    Err(e).context("making the overlay's picture")
                }
            }
        }
    }

    fn pixels(&mut self) -> &mut [u8] {
        // SAFETY: the section is `width` x `height` four-byte pixels, alive until `drop`, and
        // `&mut self` is the only way to reach it.
        unsafe { std::slice::from_raw_parts_mut(self.bits, 4 * self.width as usize * self.height as usize) }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        // SAFETY: the objects `new` made, selected out before they are deleted.
        unsafe {
            SelectObject(self.memory, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.memory);
        }
    }
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
            screen: Screen::new(),
            dib: None,
            presented: false,
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
                // The next time it shows may be on another monitor: start from nothing.
                self.dib = None;
                self.screen.forget();
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

    /// Paints the overlay into the window's picture, redrawing only what changed since the last
    /// frame, and puts that on the window.
    fn draw(&mut self, now: Instant) -> Result<()> {
        let started = Instant::now();
        let (width, height) = (self.size.cx.max(0) as u32, self.size.cy.max(0) as u32);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let size = Point::new(width as f32 / self.scale, height as f32 / self.scale);
        let ops = self.overlay.screen_ops(now, size);
        // A new picture is empty; one kept holds the last frame, which is age 1.
        let age = match &self.dib {
            Some(dib) if (dib.width, dib.height) == (width, height) => 1,
            _ => {
                self.dib = Some(Dib::new(width, height)?);
                self.presented = false;
                0
            }
        };
        let Some(dib) = self.dib.as_mut() else { return Ok(()) };
        let mut pixmap = PixmapMut::from_bytes(dib.pixels(), width, height).context("the overlay's picture has no size")?;
        let damage = self.screen.paint(&ops, self.scale, &mut pixmap, age);
        // tiny-skia's RGBA to the DIB's BGRA, over the repainted patches only.
        swap_red_blue(dib.pixels(), width, &damage);
        timing_log(&format!("built {} {} {}x{}", unix_ns(), started.elapsed().as_nanos(), size.x, size.y));
        if damage.is_empty() && self.presented {
            return Ok(());
        }
        self.presented = false;
        self.present(&damage)?;
        self.presented = true;
        Ok(())
    }

    /// Hands the DIB to Windows as the window's contents. Only the box round `damage` is
    /// updated (`prcDirty`), unless Windows has no picture of ours yet.
    fn present(&self, damage: &Damage) -> Result<()> {
        let Some(dib) = &self.dib else { return Ok(()) };
        let dirty = damage.patches().iter().copied().reduce(|a, b| Patch::new(a.x0.min(b.x0), a.y0.min(b.y0), a.x1.max(b.x1), a.y1.max(b.y1)));
        let rect = dirty.map(|p| RECT { left: p.x0, top: p.y0, right: p.x1, bottom: p.y1 });
        let partial = self.presented.then_some(rect.as_ref()).flatten();
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        let size = SIZE { cx: dib.width as i32, cy: dib.height as i32 };
        let origin = POINT::default();
        // SAFETY: every pointer in the struct refers to a local that outlives the call, and the
        // DCs are alive; the screen DC is released before returning.
        unsafe {
            let screen = GetDC(None);
            let info = UPDATELAYEREDWINDOWINFO {
                cbSize: size_of::<UPDATELAYEREDWINDOWINFO>() as u32,
                hdcDst: screen,
                pptDst: &self.at,
                psize: &size,
                hdcSrc: dib.memory,
                pptSrc: &origin,
                crKey: Default::default(),
                pblend: &blend,
                dwFlags: ULW_ALPHA,
                prcDirty: partial.map_or(std::ptr::null(), std::ptr::from_ref),
            };
            let updated = UpdateLayeredWindowIndirect(self.hwnd, &info);
            ReleaseDC(None, screen);
            if updated.as_bool() { Ok(()) } else { Err(windows::core::Error::from_thread()).context("updating the overlay window") }
        }
    }
}
