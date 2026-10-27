//! Laying a paint through a shape's coverage (`cover.rs`), and making clip masks of it.

use tiny_skia::{BlendMode, Mask, Paint, Shader, Transform};

use super::{super::Pen, Shape};
use crate::overlay::render::raster::blit::{mul, mul8, skia_over as over};

/// The pixels of a pixmap `size` big that `shape`, its corner at `at`, covers in each of its
/// rows: the row on the pixmap, the shape's own row, and the columns on the pixmap.
fn spans(shape: &Shape, at: (i32, i32), size: (i32, i32)) -> impl Iterator<Item = (i32, usize, (i32, i32))> + '_ {
    let h = shape.rows.len() as i32;
    (at.1.max(0)..(at.1 + h).min(size.1)).filter_map(move |y| {
        let row = (y - at.1) as usize;
        let (a, b) = shape.rows[row];
        let (x0, x1) = ((at.0 + a as i32).max(0), (at.0 + b as i32).min(size.0));
        (x0 < x1).then_some((y, row, (x0, x1)))
    })
}

/// Lays `paint` (mapped by the pen's transform) through `shape`, its corner at `at`, and the
/// pen's mask.
pub(super) fn lay(pen: &mut Pen, shape: &Shape, at: (i32, i32), paint: &Paint) {
    let (pixmap, transform, mask) = (&mut *pen.pixmap, pen.transform, pen.mask);
    let size = (pixmap.width() as i32, pixmap.height() as i32);
    if let (Shader::SolidColor(color), BlendMode::SourceOver) = (&paint.shader, paint.blend_mode) {
        let c = color.premultiply().to_color_u8();
        let ink = [c.red(), c.green(), c.blue(), c.alpha()];
        let (cover, w) = (shape.mask.data(), shape.mask.width() as usize);
        let outer = mask.map(Mask::data);
        let data = pixmap.data_mut();
        for (y, row, (x0, x1)) in spans(shape, at, size) {
            let (d0, n) = ((y * size.0 + x0) as usize, (x1 - x0) as usize);
            let c0 = row * w + (x0 - at.0) as usize;
            let coverage = &cover[c0..c0 + n];
            let pixels = data[4 * d0..4 * (d0 + n)].as_chunks_mut::<4>().0;
            // Without branches, so the loops run several pixels at a time.
            match outer {
                Some(outer) => {
                    for ((px, &c), &m) in pixels.iter_mut().zip(coverage).zip(&outer[d0..d0 + n]) {
                        *px = over(ink, mul8(c, m), *px);
                    }
                }
                None => {
                    for (px, &c) in pixels.iter_mut().zip(coverage) {
                        *px = over(ink, c, *px);
                    }
                }
            }
        }
        return;
    }
    // Anything else through tiny-skia, the coverage (cut by `mask`) as its mask.
    let Some(mut through) = Mask::new(size.0 as u32, size.1 as u32) else { return };
    let (cover, w) = (shape.mask.data(), shape.mask.width() as usize);
    let outer = mask.map(Mask::data);
    let to = through.data_mut();
    let (mut left, mut top, mut right, mut bottom) = (size.0, size.1, 0, 0);
    for (y, row, (x0, x1)) in spans(shape, at, size) {
        let (a, b) = ((y * size.0 + x0) as usize, row * w + (x0 - at.0) as usize);
        let n = (x1 - x0) as usize;
        to[a..a + n].copy_from_slice(&cover[b..b + n]);
        if let Some(outer) = outer {
            for (v, m) in to[a..a + n].iter_mut().zip(&outer[a..a + n]) {
                *v = mul(u32::from(*v), u32::from(*m)) as u8;
            }
        }
        (left, right, top, bottom) = (left.min(x0), right.max(x1), top.min(y), bottom.max(y + 1));
    }
    let Some(rect) = tiny_skia::Rect::from_ltrb(left as f32, top as f32, right as f32, bottom as f32) else { return };
    let mut paint = paint.clone();
    paint.shader.transform(transform);
    paint.anti_alias = false;
    pixmap.fill_rect(rect, &paint, Transform::identity(), Some(&through));
}

/// Fills `mask` (clear) with `shape`'s coverage, its corner at `at`, or (with `cut`) cuts what
/// `mask` holds to it.
pub(super) fn mask(mask: &mut Mask, shape: &Shape, at: (i32, i32), cut: bool) {
    let size = (mask.width() as i32, mask.height() as i32);
    let (cover, w) = (shape.mask.data(), shape.mask.width() as usize);
    let data = mask.data_mut();
    if !cut {
        for (y, row, (x0, x1)) in spans(shape, at, size) {
            let (a, b) = ((y * size.0 + x0) as usize, row * w + (x0 - at.0) as usize);
            let n = (x1 - x0) as usize;
            data[a..a + n].copy_from_slice(&cover[b..b + n]);
        }
        return;
    }
    for y in 0..size.1 {
        let line = &mut data[(y * size.0) as usize..((y + 1) * size.0) as usize];
        let row = y - at.1;
        let (a, b) = shape.rows.get(row as usize).filter(|_| row >= 0).map_or((0, 0), |&(a, b)| (at.0 + a as i32, at.0 + b as i32));
        for (x, v) in line.iter_mut().enumerate() {
            let x = x as i32;
            let c = if (a..b).contains(&x) { cover[row as usize * w + (x - at.0) as usize] } else { 0 };
            *v = mul(u32::from(*v), u32::from(c)) as u8;
        }
    }
}
