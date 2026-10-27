//! Drawing a shaped run as iced tiny-skia draws it: each glyph at the device scale, at its
//! sub-pixel bin across and on a whole pixel down, over the underline.

use cosmic_text::SwashContent;

use super::{Canvas, engine::GlyphSource, set::Run};
use crate::overlay::render::{Point, Rgba};

pub(super) fn run(run: &Run, glyphs: &mut GlyphSource, at: Point, color: Rgba, canvas: &mut Canvas) {
    let alpha = color.a.clamp(0.0, 1.0);
    if alpha <= 0.0 || run.lines.is_empty() {
        return;
    }
    let t = canvas.transform;
    let scale = t.sx;
    // The translation's whole pixels are added after the glyphs are placed, so a run lands on
    // the same sub-pixel bins whichever whole pixel the pixmap's corner is on: a picture drawn
    // in tiles places it the same in each.
    let whole = (t.tx.floor(), t.ty.floor());
    let origin = (at.x.mul_add(t.sx, t.tx - whole.0), at.y.mul_add(t.sy, t.ty - whole.1));
    let shift = (whole.0 as i32, whole.1 as i32);
    let [r, g, b, _] = color.to_u8();
    let ink = Ink { rgb: [r, g, b], alpha: (alpha * 255.0).round() as u32 };
    // Under the glyphs, as iced draws its quads before its text, and filled as a path as iced
    // fills them: `fill_rect`'s coverage at the edges differs.
    for u in run.lines.iter().filter_map(|l| l.underline) {
        let Some(rect) = tiny_skia::Rect::from_xywh(at.x + u.x, at.y + u.y, u.w, u.h) else { continue };
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(r, g, b, ink.alpha as u8);
        let path = tiny_skia::PathBuilder::from_rect(rect);
        canvas.pixmap.fill_path(&path, &paint, tiny_skia::FillRule::EvenOdd, t, canvas.mask);
    }
    for line in &run.lines {
        let baseline = (line.baseline * scale).round() as i32;
        for glyph in &line.glyphs {
            let physical = glyph.physical(origin, scale);
            // cosmic-text puts a glyph on the whole pixel down by truncating, which is the floor
            // only on the positive side; the floor everywhere keeps it the same either side.
            let y = (glyph.y - glyph.font_size * glyph.y_offset).mul_add(scale, origin.1).floor() as i32;
            if let Some(image) = glyphs.get(physical.cache_key) {
                let p = image.placement;
                blit(canvas, image, (physical.x + shift.0 + p.left, y + shift.1 - p.top + baseline), ink);
            }
        }
    }
}

/// The text's color as bytes, and its alpha out of 255.
#[derive(Clone, Copy)]
struct Ink {
    rgb: [u8; 3],
    alpha: u32,
}

/// Lays a glyph's picture over the pixmap with its top-left at `at` (device pixels): a mask in
/// the ink's color, a color glyph in its own, both at the ink's alpha and cut by the canvas mask.
fn blit(canvas: &mut Canvas, image: &cosmic_text::SwashImage, at: (i32, i32), ink: Ink) {
    let (pw, ph) = (canvas.pixmap.width() as i32, canvas.pixmap.height() as i32);
    let (gw, gh) = (image.placement.width as i32, image.placement.height as i32);
    let (x0, x1) = (at.0.max(0), (at.0 + gw).min(pw));
    let (y0, y1) = (at.1.max(0), (at.1 + gh).min(ph));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let cut = canvas.mask.map(tiny_skia::Mask::data);
    let data = canvas.pixmap.data_mut();
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y - at.1) * gw + (x - at.0)) as usize;
            let (rgb, a) = match image.content {
                SwashContent::Mask => (ink.rgb, u32::from(image.data[i])),
                SwashContent::Color => {
                    let c = &image.data[4 * i..4 * i + 4];
                    ([c[0], c[1], c[2]], u32::from(c[3]))
                }
                // Never asked for: glyphs are drawn as alpha masks.
                SwashContent::SubpixelMask => continue,
            };
            let at = (y * pw + x) as usize;
            let a = mul(a, ink.alpha);
            let a = cut.map_or(a, |m| mul(a, u32::from(m[at])));
            if a == 0 {
                continue;
            }
            let px = &mut data[4 * at..4 * at + 4];
            let keep = 255 - a;
            for (d, s) in px.iter_mut().zip(rgb.iter().map(|&c| mul(u32::from(c), a)).chain([a])) {
                *d = (s + mul(u32::from(*d), keep)) as u8;
            }
        }
    }
}

/// `a × b / 255`, rounded, for bytes.
fn mul(a: u32, b: u32) -> u32 {
    let t = a * b + 128;
    (t + (t >> 8)) >> 8
}
