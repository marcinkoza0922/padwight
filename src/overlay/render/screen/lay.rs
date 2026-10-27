//! Laying a panel's picture onto the screen, source-over, at its opacity.
//!
//! Unzoomed, the picture sits on whole device pixels, so each of its pixels goes over one of
//! the screen's: that is done here byte by byte, in premultiplied color as tiny-skia does it
//! (a pixel's channels scaled by the opacity, then over what is there). A pixel the picture
//! leaves clear is skipped, and one it covers fully at full strength is copied. Zoomed, the
//! picture is sampled bilinearly through a tiny-skia pattern.

use tiny_skia::{FilterQuality, Paint, Pattern, Pixmap, PixmapMut, SpreadMode, Transform};

use super::{damage::Patch, items::Laid};
use crate::overlay::render::raster::{mul8, over as blend};

/// Lays `pixmap` down as `laid` says over `target`, source-over, only within `clip`; when
/// `onto_clear`, `clip` is clear and isn't read. A picture laid down in pieces is the same,
/// pixel for pixel, as laid down whole.
pub(in crate::overlay::render::screen) fn lay_picture(pixmap: &Pixmap, laid: &Laid, clip: Patch, target: &mut PixmapMut, onto_clear: bool) {
    if laid.smooth {
        return smooth(pixmap, laid, clip, target);
    }
    let (x, y) = (laid.transform.tx as i32, laid.transform.ty as i32);
    let (w, h) = (target.width() as i32, target.height() as i32);
    let picture = Patch::new(x, y, x + pixmap.width() as i32, y + pixmap.height() as i32);
    let Some(p) = picture.intersection(&clip).and_then(|p| p.intersection(&Patch::new(0, 0, w, h))) else { return };
    let alpha = (laid.opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    if alpha == 0 {
        return;
    }
    let (from_stride, to_stride) = (4 * pixmap.width() as usize, 4 * w as usize);
    let span = 4 * p.width() as usize;
    let (from, to) = (pixmap.data(), target.data_mut());
    for row in p.y0..p.y1 {
        let a = (row - y) as usize * from_stride + 4 * (p.x0 - x) as usize;
        let b = row as usize * to_stride + 4 * p.x0 as usize;
        let (from, to) = (&from[a..a + span], &mut to[b..b + span]);
        match (onto_clear, alpha) {
            (true, 255) => to.copy_from_slice(from),
            (true, _) => {
                for (d, s) in to.iter_mut().zip(from) {
                    *d = mul8(*s, alpha as u8);
                }
            }
            _ => over(from, to, alpha),
        }
    }
}

/// One row of premultiplied pixels `from`, at `alpha` out of 255, over the row `to`.
fn over(from: &[u8], to: &mut [u8], alpha: u32) {
    let (from, to) = (from.as_chunks::<4>().0, to.as_chunks_mut::<4>().0);
    for (s, d) in from.iter().zip(to) {
        *d = blend(*s, alpha as u8, *d);
    }
}

/// `a × b / 255`, rounded, for bytes.
/// A zoomed picture: a pattern over the patch's whole pixels, sampled bilinearly.
fn smooth(pixmap: &Pixmap, laid: &Laid, clip: Patch, target: &mut PixmapMut) {
    let Some(rect) = tiny_skia::Rect::from_ltrb(clip.x0 as f32, clip.y0 as f32, clip.x1 as f32, clip.y1 as f32) else {
        return;
    };
    let shader = Pattern::new(pixmap.as_ref(), SpreadMode::Pad, FilterQuality::Bilinear, laid.opacity, laid.transform);
    let paint = Paint { shader, anti_alias: false, ..Paint::default() };
    target.fill_rect(rect, &paint, Transform::identity(), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_matches_premultiplied_source_over() {
        for (s, d, alpha) in [([100, 50, 20, 128], [10, 200, 30, 255], 255), ([255, 255, 255, 255], [0, 0, 0, 255], 100), ([40, 40, 40, 80], [0, 0, 0, 0], 51)] {
            let mut to = d;
            over(&s, &mut to, alpha);
            let a = alpha as f32 / 255.0;
            for c in 0..4 {
                let want = s[c] as f32 * a + d[c] as f32 * (1.0 - s[3] as f32 * a / 255.0);
                assert!((to[c] as f32 - want).abs() <= 1.0, "{s:?} over {d:?} at {alpha}: {to:?}");
            }
        }
    }

    #[test]
    fn a_picture_laid_in_pieces_is_laid_whole() {
        let mut picture = Pixmap::new(20, 10).unwrap();
        for (i, b) in picture.data_mut().iter_mut().enumerate() {
            *b = (i % 4 * 60) as u8;
        }
        let laid = Laid { opacity: 0.6, transform: Transform::from_translate(3.0, -2.0), smooth: false };
        let mut whole = Pixmap::new(30, 30).unwrap();
        lay_picture(&picture, &laid, Patch::new(0, 0, 30, 30), &mut whole.as_mut(), true);
        let mut pieces = Pixmap::new(30, 30).unwrap();
        for clip in [Patch::new(0, 0, 10, 30), Patch::new(10, 0, 30, 5), Patch::new(10, 5, 30, 30)] {
            lay_picture(&picture, &laid, clip, &mut pieces.as_mut(), false);
        }
        assert_eq!(whole.data(), pieces.data());
        assert_eq!(whole.pixel(2, 0).unwrap().alpha(), 0, "left of the picture");
        assert!(whole.pixel(3, 0).unwrap().alpha() > 0);
    }
}
