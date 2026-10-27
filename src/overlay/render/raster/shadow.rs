//! A box's shadow by iced tiny-skia's formula (`draw_quad`), worked out whole once and kept.
//!
//! Each device pixel of the shadow's box, sampled at its top left corner, takes the shadow's
//! color times one less the smoothstep, over the blur either side, of its distance outside the
//! rounded box. The shadow is worked out whole, in the fraction of a pixel its box lies off the
//! pixmap's grid, and kept: every tile of a panel's picture (`screen/tiles.rs`) lays the same
//! pixels from it, and a shadow under a box that stays put is worked out once. Its strength at
//! each pixel is kept apart from its color, so a glow that pulses or flashes is only colored
//! again.

use std::{
    cell::RefCell,
    hash::{DefaultHasher, Hash, Hasher},
};

use tiny_skia::{IntSize, Pixmap};

use super::{Pen, blit, kept::Kept, paint, quad};
use crate::overlay::render::{Corners, Rect, Shadow, geom::hash_floats};

/// A shadow's strength at each pixel, 0 to 1, row by row, its size, and where its corner lies.
type Strengths = (Vec<f32>, IntSize, (i32, i32));
/// A shadow in its color, and where its corner lies.
type Colored = (Pixmap, (i32, i32));

thread_local! {
    static STRENGTHS: RefCell<Kept<u64, Strengths>> = RefCell::new(Kept::new(16 << 20));
    static KEPT: RefCell<Kept<u64, Colored>> = RefCell::new(Kept::new(16 << 20));
}

/// Drops every shadow kept, as when the overlay hides.
pub(super) fn forget() {
    STRENGTHS.with_borrow_mut(Kept::clear);
    KEPT.with_borrow_mut(Kept::clear);
}

/// Lays the shadow of a box `rect` with `corners`, grown by `spread`. Inside the box the
/// distance is 0, so a soft shadow is half strength there, as in iced. With no blur the shadow
/// is the box itself, unsoftened (iced's sum gives 0/0 there, which tiny-skia reads as full
/// strength).
pub(super) fn shadow(pen: &mut Pen, rect: Rect, corners: Corners, shadow: Shadow, spread: f32) {
    if shadow.color.a.is_nan() || shadow.color.a <= 0.0 {
        return;
    }
    let t = pen.transform;
    let whole = (t.tx.floor(), t.ty.floor());
    let local = (t.sx, t.tx - whole.0, t.ty - whole.1);
    let mut h = DefaultHasher::new();
    (rect, corners, shadow.offset.0.to_bits(), shadow.offset.1.to_bits(), shadow.blur.to_bits()).hash(&mut h);
    hash_floats(&mut h, &[spread, local.0, local.1, local.2]);
    let shape = h.finish();
    shadow.color.hash(&mut h);
    let key = h.finish();
    let colored = || {
        let strengths = STRENGTHS.with_borrow_mut(|kept| kept.get(&shape, || strengths(rect, corners, shadow, spread, local)))?;
        colored(&strengths, shadow)
    };
    let Some(kept) = KEPT.with_borrow_mut(|kept| kept.get(&key, colored)) else { return };
    let (picture, (x, y)) = &*kept;
    let at = (x + whole.0 as i32, y + whole.1 as i32);
    blit::blit(pen, picture, (at, (picture.width() as i32, picture.height() as i32)), 1.0);
}

/// `strengths` in the shadow's color, premultiplied.
fn colored((strengths, size, corner): &Strengths, shadow: Shadow) -> Option<(Colored, usize)> {
    let mut pixels = Vec::with_capacity(strengths.len() * 4);
    for &strength in strengths {
        let mut c = paint::color(shadow.color);
        c.apply_opacity(strength);
        let p = c.to_color_u8().premultiply();
        pixels.extend_from_slice(&[p.red(), p.green(), p.blue(), p.alpha()]);
    }
    let bytes = pixels.len();
    Some(((Pixmap::from_vec(pixels, *size)?, *corner), bytes))
}

/// The whole shadow's strengths at scale `s`, `fx`, `fy` off the grid.
fn strengths(rect: Rect, corners: Corners, shadow: Shadow, spread: f32, (s, fx, fy): (f32, f32, f32)) -> Option<(Strengths, usize)> {
    let base = quad::radii(rect, corners);
    let rect = rect.outset(spread);
    if rect.is_empty() {
        return None;
    }
    let radii = base.map(|r| if r > 0.0 { (r + spread).max(0.0).min(rect.w / 2.0).min(rect.h / 2.0) } else { 0.0 });
    let (blur, (dx, dy)) = (shadow.blur.max(0.0), shadow.offset);
    let device = |r: Rect| Rect::new(r.x * s + fx, r.y * s + fy, r.w * s, r.h * s);
    let reach = device(Rect::new(rect.x + dx - blur, rect.y + dy - blur, rect.w + 2.0 * blur, rect.h + 2.0 * blur));
    // Whole pixels as iced takes them (`as u32`, the floor for a shadow on the screen).
    let (x0, y0) = (reach.x.floor() as i32, reach.y.floor() as i32);
    let size = IntSize::from_wh(reach.w as u32, reach.h as u32)?;
    if size.width() > 16_384 || size.height() > 16_384 {
        return None;
    }
    let body = device(rect);
    let half = (body.w / 2.0, body.h / 2.0);
    let center = (body.x + dx * s + half.0, body.y + dy * s + half.1);
    let radii = radii.map(|r| r * s);
    let mut out = Vec::with_capacity(size.width() as usize * size.height() as usize);
    for y in y0..y0 + size.height() as i32 {
        for x in x0..x0 + size.width() as i32 {
            let d = distance((x as f32 - center.0, y as f32 - center.1), half, radii).max(0.0);
            out.push(if blur > 0.0 { 1.0 - smoothstep(-blur * s, blur * s, d) } else if d <= 0.0 { 1.0 } else { 0.0 });
        }
    }
    let bytes = out.len() * 4;
    Some(((out, size, (x0, y0)), bytes))
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let x = ((x - a) / (b - a)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How far `to_center` (from the box's middle) lies outside a box of `half` its size with
/// corners `radii` round, negative inside (iced's `rounded_box_sdf`).
fn distance(to_center: (f32, f32), half: (f32, f32), radii: [f32; 4]) -> f32 {
    let radius = match (to_center.0 > 0.0, to_center.1 > 0.0) {
        (true, true) => radii[2],
        (true, false) => radii[1],
        (false, true) => radii[3],
        (false, false) => radii[0],
    };
    let x = (to_center.0.abs() - half.0 + radius).max(0.0);
    let y = (to_center.1.abs() - half.1 + radius).max(0.0);
    (x.powf(2.0) + y.powf(2.0)).sqrt() - radius
}
