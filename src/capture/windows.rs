//! Screenshots on Windows: the whole desktop (every monitor), copied with GDI and saved as PNG.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use windows::Win32::{
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SRCCOPY, SelectObject,
    },
    System::SystemInformation::GetLocalTime,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
        WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN},
    },
};

use super::{media_base, media_path};

/// The local time as `2026-10-07_19-58-03`.
pub fn timestamp() -> String {
    // SAFETY: GetLocalTime has no preconditions.
    let t = unsafe { GetLocalTime() };
    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

/// Saves a screenshot for `game` and returns where it went.
pub fn screenshot(game: &str) -> Result<PathBuf> {
    let pictures = media_base(dirs::picture_dir(), "Pictures")?;
    let path = media_path(&pictures, "Screenshots", game, &timestamp(), "png");
    std::fs::create_dir_all(path.parent().context("no folder")?).with_context(|| format!("creating {}", path.display()))?;
    let (width, height, rgba) = grab_desktop()?;
    let file = std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&rgba)?;
    Ok(path)
}

/// The desktop's pixels, as width, height and RGBA rows from the top.
fn grab_desktop() -> Result<(u32, u32, Vec<u8>)> {
    // SAFETY: every GDI object made here is selected out and freed before returning, and the
    // buffer GetDIBits fills is as big as the bitmap it describes.
    unsafe {
        // Real pixels, not the scaled-down ones a DPI-unaware thread would be given.
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let (x, y) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
        let (w, h) = (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN));
        if w <= 0 || h <= 0 {
            bail!("no screen to capture");
        }
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(memory, bitmap.into());
        let copied = BitBlt(memory, 0, 0, w, h, Some(screen), x, y, SRCCOPY | CAPTUREBLT);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative: rows from the top down.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; w as usize * h as usize * 4];
        let rows = GetDIBits(memory, bitmap, 0, h as u32, Some(pixels.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        SelectObject(memory, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        copied.context("copying the screen")?;
        if rows != h {
            bail!("reading the screen's pixels failed");
        }
        // BGRX to RGBA.
        for px in pixels.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }
        Ok((w as u32, h as u32, pixels))
    }
}
