//! What a frame lays down at the top level of its list, one item per panel, each with the patch
//! of device pixels it can touch; which of those changed from one frame to the next; and laying
//! them down, cut to a patch, over a buffer cleared there.
//!
//! An item's patch holds every pixel it can change. Outside the patches of the items that
//! changed, the items over any pixel are the same ones, in the same order, laid the same way,
//! so the pixel comes out the same and needn't be redrawn. A picture laid the same way as
//! before, whose tiles were drawn again since (`tiles.rs`), changed only where those tiles lie.

use tiny_skia::{Pixmap, PixmapMut, Transform};

pub(super) use super::lay::lay_picture;

use super::damage::Patch;
use crate::overlay::render::{Layer, Op, bounds, raster};

/// One thing laid at the top level of a frame's list.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Item {
    pub what: What,
    /// The device pixels it can change.
    pub at: Patch,
    /// For a picture: the device pixels its last redraw changed, where it lies now.
    pub redrawn: Vec<Patch>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum What {
    /// A kept picture (by its number, never reused), as it was after its `version`th redraw,
    /// laid as `Laid` says.
    Picture { id: u64, version: u64, laid: Laid },
    /// Ops outside any panel's layer, drawn straight.
    Straight(Vec<Op>),
}

/// How a picture goes down: faded by `opacity`, its pixels taken to the screen's by
/// `transform`, sampled smoothly when zoomed and pixel for pixel when not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Laid {
    pub opacity: f32,
    pub transform: Transform,
    pub smooth: bool,
}

/// Where a picture as big as `size`, whose corner lies at `origin` (device pixels) at rest,
/// goes as `layer` moves it, and the patch it covers there.
///
/// Unzoomed, its offset is rounded to whole device pixels and the picture copied pixel for
/// pixel, so text doesn't shimmer as it slides. Zoomed about the layer's point, it is sampled
/// bilinearly: picture pixel u is device point origin + u at rest, logical (origin + u) / scale,
/// which the layer takes to about + shift + (p - about) * z.
pub(super) fn laid(size: (u32, u32), origin: (i32, i32), layer: &Layer, scale: f32) -> (Laid, Patch) {
    let opacity = layer.opacity.clamp(0.0, 1.0);
    let z = layer.scale;
    let (w, h) = (size.0 as f32, size.1 as f32);
    if (z - 1.0).abs() < 1e-4 {
        let x = origin.0 + (layer.shift.x * scale).round() as i32;
        let y = origin.1 + (layer.shift.y * scale).round() as i32;
        let transform = Transform::from_translate(x as f32, y as f32);
        return (Laid { opacity, transform, smooth: false }, Patch::new(x, y, x + size.0 as i32, y + size.1 as i32));
    }
    let (a, s) = (layer.about, layer.shift);
    let tx = z * origin.0 as f32 + scale * (a.x * (1.0 - z) + s.x);
    let ty = z * origin.1 as f32 + scale * (a.y * (1.0 - z) + s.y);
    let transform = Transform::from_row(z, 0.0, 0.0, z, tx, ty);
    // Past the picture's edge the clear pixel round it is sampled on: nothing is drawn there.
    let at = Patch::new(tx.floor() as i32, ty.floor() as i32, (tx + z * w).ceil() as i32, (ty + z * h).ceil() as i32);
    (Laid { opacity, transform, smooth: true }, at)
}

/// Where `patches` of a picture lie on the screen when it is laid as `laid` covering `at`:
/// moved with it unzoomed, and all of `at` zoomed.
pub(super) fn moved(patches: &[Patch], laid: &Laid, at: Patch) -> Vec<Patch> {
    if patches.is_empty() {
        return Vec::new();
    }
    if laid.smooth {
        return vec![at];
    }
    let (x, y) = (laid.transform.tx as i32, laid.transform.ty as i32);
    patches.iter().map(|p| Patch::new(p.x0 + x, p.y0 + y, p.x1 + x, p.y1 + y)).collect()
}

/// Whether `now` is `before` laid the same way, its picture perhaps redrawn since.
fn same(before: &Item, now: &Item) -> bool {
    before.at == now.at
        && match (&before.what, &now.what) {
            (What::Picture { id: a, laid: la, .. }, What::Picture { id: b, laid: lb, .. }) => a == b && la == lb,
            (a, b) => a == b,
        }
}

/// What changed of an item laid the same way as last frame: nothing for the same ops drawn
/// straight, or for a picture not redrawn since; the patches its picture's one redraw since
/// changed, or all of it after more than one.
fn redrawn(before: &Item, now: &Item, out: &mut Vec<Patch>) {
    match (&before.what, &now.what) {
        (What::Picture { version: a, .. }, What::Picture { version: b, .. }) if a == b => {}
        (What::Picture { version: a, .. }, What::Picture { version: b, .. }) if a + 1 == *b => out.extend(&now.redrawn),
        (What::Straight(_), What::Straight(_)) => {}
        _ => out.push(now.at),
    }
}

/// The patch `ops`, drawn straight at `scale`, can touch: their ink and a pixel more all round.
pub(super) fn straight_patch(ops: &[Op], scale: f32) -> Option<Patch> {
    let ink = bounds(ops)?;
    let edge = |v: f32| v * scale;
    let (x0, y0) = (edge(ink.x).floor() as i32 - 1, edge(ink.y).floor() as i32 - 1);
    Some(Patch::new(x0, y0, edge(ink.right()).ceil() as i32 + 1, edge(ink.bottom()).ceil() as i32 + 1))
}

/// The patches that differ between frames laying `before` and `now`: the patches of the items
/// of either not matched, in order, by the same item in the other, and what was redrawn of
/// those matched. Matching keeps the order, as the stacking order matters where items overlap;
/// an item matched out of order counts as gone and come, which only paints more.
pub(super) fn changed(before: &[Item], now: &[Item]) -> Vec<Patch> {
    let mut out = Vec::new();
    let mut from = 0;
    for item in now {
        match before[from..].iter().position(|b| same(b, item)) {
            Some(k) => {
                out.extend(before[from..from + k].iter().map(|b| b.at));
                redrawn(&before[from + k], item, &mut out);
                from += k + 1;
            }
            None => out.push(item.at),
        }
    }
    out.extend(before[from..].iter().map(|b| b.at));
    out
}

/// Clears `patch` of `target` to transparent, without reading it.
pub(super) fn clear(target: &mut PixmapMut, patch: Patch) {
    let (w, h) = (target.width() as i32, target.height() as i32);
    let Some(p) = patch.intersection(&Patch::new(0, 0, w, h)) else { return };
    let stride = 4 * w as usize;
    let data = target.data_mut();
    for y in p.y0 as usize..p.y1 as usize {
        data[y * stride + 4 * p.x0 as usize..y * stride + 4 * p.x1 as usize].fill(0);
    }
}

/// Draws each of `lists` straight at `scale`, in order, over `target`, only within `clip`; when
/// `onto_clear`, `clip` is clear and isn't read. They are drawn into a pixmap as big as the clip,
/// which is then laid down: a clip mask would be as big as the screen, and making one for each
/// list in each patch cost far more than the drawing (a doodle is many lists). Shapes moved by
/// whole pixels take the same coverage, so this is the same as drawing them on the screen.
pub(super) fn draw_straight(lists: &[&[Op]], scale: f32, clip: Patch, target: &mut PixmapMut, onto_clear: bool) {
    let Some(clip) = clip.intersection(&Patch::new(0, 0, target.width() as i32, target.height() as i32)) else { return };
    let Some(mut pixmap) = Pixmap::new(clip.width() as u32, clip.height() as u32) else { return };
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, -clip.x0 as f32, -clip.y0 as f32);
    for ops in lists {
        raster::draw(ops, &mut pixmap.as_mut(), transform);
    }
    let laid = Laid { opacity: 1.0, transform: Transform::from_translate(clip.x0 as f32, clip.y0 as f32), smooth: false };
    lay_picture(&pixmap, &laid, clip, target, onto_clear);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u64, x: i32) -> Item {
        let laid = Laid { opacity: 1.0, transform: Transform::identity(), smooth: false };
        Item { what: What::Picture { id, version: 0, laid }, at: Patch::new(x, 0, x + 10, 10), redrawn: Vec::new() }
    }

    #[test]
    fn unchanged_items_add_nothing() {
        let items = [item(1, 0), item(2, 100)];
        assert!(changed(&items, &items).is_empty());
    }

    /// Ops drawn straight (doodles) that stay the same aren't repainted: each counted as
    /// changed every frame, so a frame repainted every doodle's box.
    #[test]
    fn unchanged_straight_items_add_nothing() {
        let straight = |x: f32| {
            let ops = vec![Op::Rect { rect: crate::overlay::render::Rect::new(x, 0.0, 10.0, 10.0), radii: Default::default(), fill: None, border: None }];
            Item { at: straight_patch(&ops, 1.0).unwrap(), what: What::Straight(ops), redrawn: Vec::new() }
        };
        let (a, b, moved) = (straight(0.0), straight(100.0), straight(300.0));
        assert!(changed(&[a.clone(), b.clone()], &[a.clone(), b.clone()]).is_empty());
        assert_eq!(changed(&[a.clone(), b.clone()], &[a.clone(), moved.clone()]), vec![moved.at, b.at]);
    }

    #[test]
    fn a_changed_item_adds_where_it_was_and_where_it_is() {
        let (a, b) = (item(1, 0), item(2, 100));
        let moved = item(2, 300);
        assert_eq!(changed(&[a.clone(), b.clone()], &[a.clone(), moved.clone()]), vec![moved.at, b.at]);
    }

    #[test]
    fn items_come_and_go() {
        let (a, b, c) = (item(1, 0), item(2, 100), item(3, 200));
        assert_eq!(changed(&[a.clone(), b.clone()], std::slice::from_ref(&a)), vec![b.at]);
        assert_eq!(changed(std::slice::from_ref(&a), &[a.clone(), c.clone()]), vec![c.at]);
    }

    #[test]
    fn a_redrawn_picture_adds_only_what_was_redrawn() {
        let a = item(1, 0);
        let patch = Patch::new(2, 2, 4, 4);
        let mut b = a.clone();
        if let What::Picture { version, .. } = &mut b.what {
            *version = 1;
        }
        b.redrawn = vec![patch];
        assert_eq!(changed(std::slice::from_ref(&a), std::slice::from_ref(&b)), vec![patch]);
        // Redrawn twice since: all of it.
        let mut c = b.clone();
        if let What::Picture { version, .. } = &mut c.what {
            *version = 2;
        }
        assert_eq!(changed(std::slice::from_ref(&a), std::slice::from_ref(&c)), vec![a.at]);
    }

    #[test]
    fn swapped_items_count_as_changed() {
        let (a, b) = (item(1, 0), item(2, 5));
        let out = changed(&[a.clone(), b.clone()], &[b.clone(), a.clone()]);
        assert!(!out.is_empty());
    }
}
