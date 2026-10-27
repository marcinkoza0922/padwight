//! A picture laid over whole device pixels: filter patterns and SVGs are placed on whole device
//! pixels, so they go down this way. At its own size it is copied pixel for pixel; stretched,
//! each device pixel takes the four picture pixels round its center, weighted by how near they
//! are (bilinear, clamped at the picture's edge).
//!
//! Done here rather than as a tiny-skia pattern, whose sampling can land differently in
//! pixmaps whose corners lie elsewhere: here a device pixel's color depends only on where it
//! lies from the picture's corner, so a picture comes out the same in every tile of a panel's
//! picture (`screen/tiles.rs`).

use tiny_skia::Pixmap;

use super::Pen;
use crate::overlay::render::Rect;

/// Where `place` (logical pixels) lies on the pen's pixmap, when its edges are on whole device
/// pixels: its top left corner and size.
pub(super) fn whole(pen: &Pen, place: Rect) -> Option<((i32, i32), (i32, i32))> {
    let edges = pen.edges(place);
    let near = |a: f32| (a - a.round()).abs() < 1e-3;
    let [x0, y0, x1, y1] = edges.map(|e| e.round() as i32);
    (edges.iter().all(|e| near(*e)) && x1 > x0 && y1 > y0).then_some(((x0, y0), (x1 - x0, y1 - y0)))
}

/// One axis of a stretch: for each device pixel along it, the first of the two picture pixels
/// it is drawn from, the second, and the second's weight out of 256.
fn taps(from: i32, to: i32) -> Vec<(usize, usize, u32)> {
    let last = (from - 1).max(0);
    (0..to)
        .map(|i| {
            if from == to {
                return (i as usize, i as usize, 0);
            }
            let u = ((i as f32 + 0.5) * from as f32 / to as f32 - 0.5).max(0.0);
            let a = (u.floor() as i32).min(last);
            let w = ((u - a as f32) * 256.0).round() as u32;
            (a as usize, (a + 1).min(last) as usize, w.min(256))
        })
        .collect()
}

/// Lays `image` over the device pixels at `at`, `size` big, at `opacity`, source-over, cut by
/// the pen's mask.
pub(super) fn blit(pen: &mut Pen, image: &Pixmap, (at, size): ((i32, i32), (i32, i32)), opacity: f32) {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    let (pw, ph) = (pen.pixmap.width() as i32, pen.pixmap.height() as i32);
    let (x0, x1) = (at.0.max(0), (at.0 + size.0).min(pw));
    let (y0, y1) = (at.1.max(0), (at.1 + size.1).min(ph));
    if alpha == 0 || x0 >= x1 || y0 >= y1 {
        return;
    }
    let (iw, ih) = (image.width() as i32, image.height() as i32);
    let cut = pen.mask.map(tiny_skia::Mask::data);
    let from = image.data();
    let to = pen.pixmap.data_mut();
    if (iw, ih) == size {
        let rows = |y: i32| ((y - at.1) * iw + (x0 - at.0)) as usize..((y - at.1) * iw + (x1 - at.0)) as usize;
        for y in y0..y1 {
            let (s, d) = (rows(y), (y * pw + x0) as usize..(y * pw + x1) as usize);
            copied(&from[4 * s.start..4 * s.end], &mut to[4 * d.start..4 * d.end], cut.map(|m| &m[d]), alpha as u8);
        }
        return;
    }
    let (across, down) = (taps(iw, size.0), taps(ih, size.1));
    let pixel = |x: usize, y: usize| &from[4 * (y * iw as usize + x)..4 * (y * iw as usize + x) + 4];
    for y in y0..y1 {
        let (r0, r1, wy) = down[(y - at.1) as usize];
        for x in x0..x1 {
            let d = (y * pw + x) as usize;
            let a = cut.map_or(alpha, |m| mul(alpha, u32::from(m[d])));
            if a == 0 {
                continue;
            }
            let (c0, c1, wx) = across[(x - at.0) as usize];
            let (p00, p01, p10, p11) = (pixel(c0, r0), pixel(c1, r0), pixel(c0, r1), pixel(c1, r1));
            let src = [0, 1, 2, 3].map(|c| {
                let top = u32::from(p00[c]) * (256 - wx) + u32::from(p01[c]) * wx;
                let bottom = u32::from(p10[c]) * (256 - wx) + u32::from(p11[c]) * wx;
                let v = (top * (256 - wy) + bottom * wy + (1 << 15)) >> 16;
                mul(v, a)
            });
            if src[3] == 0 {
                continue;
            }
            let keep = 255 - src[3];
            let px = &mut to[4 * d..4 * d + 4];
            for (c, v) in px.iter_mut().enumerate() {
                *v = (src[c] + mul(u32::from(*v), keep)) as u8;
            }
        }
    }
}

/// One row of a picture at its own size, `from`, at `alpha`, cut by `cut`, over the row `to`.
fn copied(from: &[u8], to: &mut [u8], cut: Option<&[u8]>, alpha: u8) {
    let (src, dst) = (from.as_chunks::<4>().0, to.as_chunks_mut::<4>().0);
    match cut {
        Some(cut) => {
            for ((px, s), &m) in dst.iter_mut().zip(src).zip(cut) {
                *px = over(*s, mul8(alpha, m), *px);
            }
        }
        None => {
            for (px, s) in dst.iter_mut().zip(src) {
                *px = over(*s, alpha, *px);
            }
        }
    }
}

/// `a × b / 255`, rounded, for bytes.
pub(in crate::overlay::render) fn mul(a: u32, b: u32) -> u32 {
    let t = a * b + 128;
    (t + (t >> 8)) >> 8
}

/// `mul` for two bytes, in 16 bits (the sums fit), so a loop of them runs several at a time.
#[inline(always)]
pub(in crate::overlay::render) fn mul8(a: u8, b: u8) -> u8 {
    let t = u16::from(a) * u16::from(b) + 128;
    ((t + (t >> 8)) >> 8) as u8
}

/// `a × b / 255` for two bytes as tiny-skia's pipeline takes it (`(v + 255) >> 8`).
#[inline(always)]
fn skia_mul(a: u8, b: u8) -> u8 {
    ((u16::from(a) * u16::from(b) + 255) >> 8) as u8
}

/// `over` with tiny-skia's sums, so shapes laid through their coverage (`cover.rs`) blend as
/// iced's do.
#[inline(always)]
pub(in crate::overlay::render) fn skia_over(src: [u8; 4], alpha: u8, dst: [u8; 4]) -> [u8; 4] {
    let s = src.map(|c| skia_mul(c, alpha));
    let keep = 255 - s[3];
    [0, 1, 2, 3].map(|k| s[k].wrapping_add(skia_mul(dst[k], keep)))
}

/// The premultiplied pixel `src` at `alpha` out of 255, over `dst`: at alpha 0 `dst` as it is,
/// at 255 with an opaque `src`, `src`.
#[inline(always)]
pub(in crate::overlay::render) fn over(src: [u8; 4], alpha: u8, dst: [u8; 4]) -> [u8; 4] {
    let s = src.map(|c| mul8(c, alpha));
    let keep = 255 - s[3];
    [0, 1, 2, 3].map(|k| s[k].wrapping_add(mul8(dst[k], keep)))
}
