//! What lies under a panel's changes: its list drawn only as far as the first op that changed,
//! kept beside its picture, so a tile drawn again starts from that and draws only the ops from
//! there on.
//!
//! A highlight gliding down a menu changes the ops of two rows; everything before them (the
//! shadow, the panel's background and filters, the rows above) stays, and is drawn once into
//! this picture rather than again in every tile the glide touches. The cut is made where no
//! layer is open (a layer is laid down as a whole), with the clips open there opened again for
//! the ops after it: drawing the ops before the cut and then those after, each over what is
//! there, is drawing them all (`raster` lays op after op), tile by tile the same pixels.
//!
//! The picture holds the list up to its cut for as long as the list doesn't change before the
//! cut; when it does, it is drawn again to the new, earlier cut.

use tiny_skia::Pixmap;

use super::{
    diff::Marked,
    tiles::{self, Grid},
};
use crate::overlay::render::Op;

/// A panel's list drawn up to `upto`, the first op not drawn.
pub(super) struct Under {
    pub upto: usize,
    pub pixmap: Pixmap,
}

/// Where `ops` can be cut at or before `start`: before the outermost layer open there.
fn cut(ops: &[Op], start: usize) -> usize {
    let mut open: Vec<(usize, bool)> = Vec::new();
    for (i, op) in ops[..start.min(ops.len())].iter().enumerate() {
        match op {
            Op::Clip { .. } => open.push((i, false)),
            Op::Layer(_) => open.push((i, true)),
            Op::Unclip | Op::Unlayer => {
                open.pop();
            }
            _ => {}
        }
    }
    open.iter().find(|(_, layer)| *layer).map_or(start.min(ops.len()), |(i, _)| *i)
}

/// The ops of `ops` from `upto` on, after the clips open there, opened again.
pub(super) fn rest(ops: &[Op], upto: usize) -> Vec<Op> {
    let mut open = Vec::new();
    for op in &ops[..upto] {
        match op {
            Op::Clip { .. } => open.push(Some(op.clone())),
            Op::Layer(_) => open.push(None),
            Op::Unclip | Op::Unlayer => {
                open.pop();
            }
            _ => {}
        }
    }
    let mut out: Vec<Op> = open.into_iter().flatten().collect();
    out.extend_from_slice(&ops[upto..]);
    out
}

/// Brings `under` in step with `list`, which is the same as the list before it up to `start`:
/// kept when its cut lies at or before `start`, else drawn again to the cut there (none when
/// that is the very start). Returns how many tiles it drew.
pub(super) fn update(under: &mut Option<Under>, list: &Marked, start: usize, grid: Grid) -> usize {
    if under.as_ref().is_some_and(|u| u.upto <= start) {
        return 0;
    }
    *under = None;
    let upto = cut(&list.ops, start);
    if upto == 0 {
        return 0;
    }
    let Some(mut pixmap) = Pixmap::new(grid.size.0 as u32, grid.size.1 as u32) else { return 0 };
    let drawn = tiles::draw(&Marked::new(&list.ops[..upto]), grid, &vec![true; grid.count()], &mut pixmap, None);
    *under = Some(Under { upto, pixmap });
    drawn
}
