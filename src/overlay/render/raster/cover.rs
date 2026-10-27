//! Paths filled, stroked and clipped through their coverage: each shape is rasterized whole by
//! tiny-skia into a mask of its own, kept, and laid through that mask wherever it is drawn.
//!
//! tiny-skia cuts a shape that crosses the pixmap's edge at the edge before it turns curves into
//! steps, so the same curve drawn into two pixmaps (two tiles of a panel's picture,
//! `screen/tiles.rs`) comes out as different steps, up to half a pixel apart where the tiles
//! meet. A shape's mask is drawn whole, never cut, from the shape and the fraction of a pixel it
//! lies off the pixmap's grid, so every tile takes the same coverage from it, which is also
//! what drawing the shape straight onto a pixmap gives. A shape seen again (a radial menu's
//! arcs as the highlight glides over them, a key's box in every tile it crosses) is laid from
//! its kept mask without being rasterized again.
//!
//! Laying a solid color is done here (`through.rs`), only over the pixels a shape covers; a
//! gradient goes through tiny-skia, with the coverage as its mask.

mod through;

use std::{
    cell::RefCell,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
};

use tiny_skia::{FillRule, Mask, MaskType, Paint, Pixmap, Stroke, Transform};

use super::{Pen, kept::Kept};

/// How a shape is inked: filled by a rule, or stroked.
#[derive(Clone, Copy)]
enum Ink<'a> {
    Fill(FillRule),
    Stroke(&'a Stroke),
}

/// A shape's coverage: a mask just big enough, where its corner lies from the device pixel its
/// pixmap's translation rounds down to, and for each of its rows the columns it covers.
struct Shape {
    mask: Mask,
    corner: (i32, i32),
    rows: Vec<(u32, u32)>,
}

/// The key a coverage is kept under: the shape and ink's hash, and the transform's scale and
/// fraction of a pixel.
type Key = (u64, [u32; 4]);

thread_local! {
    static KEPT: RefCell<Kept<Key, Shape>> = RefCell::new(Kept::new(24 << 20));
}

/// Drops every coverage kept, as when the overlay hides.
pub(super) fn forget() {
    KEPT.with_borrow_mut(Kept::clear);
}

/// Fills `path` by `rule` with `paint`, with the pen.
pub(super) fn fill(pen: &mut Pen, path: &tiny_skia::Path, paint: &Paint, rule: FillRule) {
    if let Some((shape, at)) = shape(path, Ink::Fill(rule), pen.transform) {
        through::lay(pen, &shape, at, paint);
    }
}

/// Strokes `path` with `paint`, with the pen.
pub(super) fn stroke(pen: &mut Pen, path: &tiny_skia::Path, paint: &Paint, stroke: &Stroke) {
    if let Some((shape, at)) = shape(path, Ink::Stroke(stroke), pen.transform) {
        through::lay(pen, &shape, at, paint);
    }
}

/// Fills `mask` (clear) with `path`'s coverage by `rule`, or (with `cut`) cuts what it holds to
/// it.
pub(super) fn mask(mask: &mut Mask, path: &tiny_skia::Path, rule: FillRule, transform: Transform, cut: bool) {
    match shape(path, Ink::Fill(rule), transform) {
        Some((shape, at)) => through::mask(mask, &shape, at, cut),
        None if cut => mask.data_mut().fill(0),
        None => {}
    }
}

/// The coverage of `path` inked as `ink` through `transform` (a scale and a translation), and
/// where its corner lies on the pixmap; made the first time and kept.
fn shape(path: &tiny_skia::Path, ink: Ink, transform: Transform) -> Option<(Rc<Shape>, (i32, i32))> {
    let (sx, sy) = (transform.sx, transform.sy);
    let whole = (transform.tx.floor(), transform.ty.floor());
    let (fx, fy) = (transform.tx - whole.0, transform.ty - whole.1);
    let key = (key(path, ink), [sx, sy, fx, fy].map(f32::to_bits));
    let local = Transform::from_row(sx, 0.0, 0.0, sy, fx, fy);
    let kept = KEPT.with_borrow_mut(|kept| kept.get(&key, || made(path, ink, local)))?;
    let at = (kept.corner.0 + whole.0 as i32, kept.corner.1 + whole.1 as i32);
    Some((kept, at))
}

/// `path` inked as `ink` through `local` into a mask just big enough, with its size in bytes.
fn made(path: &tiny_skia::Path, ink: Ink, local: Transform) -> Option<(Shape, usize)> {
    let outline;
    let (shape, rule, hairline) = match ink {
        Ink::Fill(rule) => (path, rule, None),
        // As tiny-skia strokes: a hairline as it is, anything wider as its outline filled.
        Ink::Stroke(stroke) if stroke.width * local.sx <= 1.0 => (path, FillRule::Winding, Some(stroke)),
        Ink::Stroke(stroke) => {
            outline = path.stroke(stroke, local.sx)?;
            (&outline, FillRule::Winding, None)
        }
    };
    let b = shape.bounds().transform(local)?;
    // A hairline reaches a pixel past its path, at its caps and joins.
    let spill = if hairline.is_some() { 2.0 } else { 1.0 };
    let (x0, y0) = ((b.left() - spill).floor() as i32, (b.top() - spill).floor() as i32);
    let (x1, y1) = ((b.right() + spill).ceil() as i32, (b.bottom() + spill).ceil() as i32);
    if x1 - x0 > 16_384 || y1 - y0 > 16_384 {
        return None;
    }
    let to = local.post_translate(-x0 as f32, -y0 as f32);
    let mask = match hairline {
        None => {
            let mut mask = Mask::new((x1 - x0) as u32, (y1 - y0) as u32)?;
            mask.fill_path(shape, rule, true, to);
            mask
        }
        Some(stroke) => {
            let mut pixmap = Pixmap::new((x1 - x0) as u32, (y1 - y0) as u32)?;
            let mut white = Paint { anti_alias: true, ..Paint::default() };
            white.set_color_rgba8(255, 255, 255, 255);
            pixmap.stroke_path(shape, &white, stroke, to, None);
            Mask::from_pixmap(pixmap.as_ref(), MaskType::Alpha)
        }
    };
    let w = mask.width() as usize;
    let rows: Vec<(u32, u32)> = mask
        .data()
        .chunks(w)
        .map(|row| match (row.iter().position(|c| *c > 0), row.iter().rposition(|c| *c > 0)) {
            (Some(a), Some(b)) => (a as u32, b as u32 + 1),
            _ => (0, 0),
        })
        .collect();
    let bytes = mask.data().len() + 8 * rows.len();
    Some((Shape { mask, corner: (x0, y0), rows }, bytes))
}

/// A hash of the shape and how it is inked.
fn key(path: &tiny_skia::Path, ink: Ink) -> u64 {
    let mut h = DefaultHasher::new();
    for verb in path.verbs() {
        (*verb as u8).hash(&mut h);
    }
    for p in path.points() {
        (p.x.to_bits(), p.y.to_bits()).hash(&mut h);
    }
    match ink {
        Ink::Fill(rule) => (0u8, rule as u8).hash(&mut h),
        Ink::Stroke(s) => (1u8, s.width.to_bits(), s.miter_limit.to_bits(), s.line_cap as u8, s.line_join as u8, s.dash.is_some()).hash(&mut h),
    }
    h.finish()
}
