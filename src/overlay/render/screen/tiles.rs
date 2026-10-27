//! A panel's picture drawn in tiles: squares of `TILE` device pixels from the picture's corner,
//! each drawn on its own from only the ops that reach it (`diff::Marked::reach`).
//!
//! A tile's pixels depend only on the ops that reach it and on where the tile is, never on the
//! tiles round it, so a picture is the same whether its tiles were drawn at once or some of them
//! again later from a changed list. When a panel's list changes, only the tiles that the changed
//! ops reach (`diff::changed`) are drawn again: a highlight gliding from one row to the next
//! redraws those two rows, not the panel.
//!
//! The rasterizer draws the same pixels into a tile as into the whole picture: shapes go down
//! through their coverage, made whole (`raster/cover.rs`), pictures are copied or stretched
//! from their corner on whole pixels (`raster/blit.rs`), and glyphs are placed on the same
//! sub-pixel bins whichever whole pixel the tile starts on (`text/paint.rs`).

use tiny_skia::{Pixmap, Transform};

use super::{damage::Patch, diff::Marked};
use crate::overlay::render::{Op, Rect, raster};

/// A tile's side, in device pixels.
pub(super) const TILE: i32 = 64;
/// How far past a tile (device pixels) an op's reach still counts as reaching it: edges drawn
/// smooth and boxes snapped to whole pixels can spill a little past an op's bounds.
const SPILL: f32 = 2.0;

/// A picture's tiles: its size in device pixels, where its corner lies, and the device scale it
/// is drawn at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Grid {
    pub origin: (i32, i32),
    pub size: (i32, i32),
    pub scale: f32,
}

impl Grid {
    fn columns(&self) -> i32 {
        (self.size.0 + TILE - 1) / TILE
    }

    fn rows(&self) -> i32 {
        (self.size.1 + TILE - 1) / TILE
    }

    /// How many tiles it has.
    pub fn count(&self) -> usize {
        (self.columns() * self.rows()) as usize
    }

    /// Tile `n`'s pixels in the picture, row by row from the top left.
    pub fn tile(&self, n: usize) -> Patch {
        let (c, r) = (n as i32 % self.columns(), n as i32 / self.columns());
        let (x0, y0) = (c * TILE, r * TILE);
        Patch::new(x0, y0, (x0 + TILE).min(self.size.0), (y0 + TILE).min(self.size.1))
    }

    /// Tile `n` in logical pixels, grown by `SPILL`.
    fn reach(&self, n: usize) -> Rect {
        let scale = self.scale;
        let t = self.tile(n);
        let (x0, y0) = ((self.origin.0 + t.x0) as f32 - SPILL, (self.origin.1 + t.y0) as f32 - SPILL);
        let (w, h) = (t.width() as f32 + 2.0 * SPILL, t.height() as f32 + 2.0 * SPILL);
        Rect::new(x0 / scale, y0 / scale, w / scale, h / scale)
    }
}

/// Whether `reach` lies over `tile`.
fn over(reach: Rect, tile: Rect) -> bool {
    reach.x < tile.right() && tile.x < reach.right() && reach.y < tile.bottom() && tile.y < reach.bottom()
}

/// Which of `grid`'s tiles any of `rects` (logical pixels) lies over.
pub(super) fn reached(grid: Grid, rects: &[Rect]) -> Vec<bool> {
    (0..grid.count())
        .map(|n| {
            let tile = grid.reach(n);
            rects.iter().any(|r| over(*r, tile))
        })
        .collect()
}

/// Draws the tiles of `grid` that `which` marks into `pixmap` (as big as the grid), each from
/// only the ops of `list` that reach it, over the same tile of `under` (`under.rs`), or
/// cleared. Returns how many it drew.
pub(super) fn draw(list: &Marked, grid: Grid, which: &[bool], pixmap: &mut Pixmap, under: Option<&Pixmap>) -> usize {
    let scale = grid.scale;
    let mut drawn = 0;
    let mut ops = Vec::new();
    let Some(mut scratch) = Pixmap::new(TILE as u32, TILE as u32) else { return 0 };
    for n in (0..grid.count()).filter(|&n| which[n]) {
        let tile = grid.tile(n);
        ops.clear();
        reaching(list, grid.reach(n), &mut ops);
        if ops.iter().all(|op| matches!(op, Op::Clip { .. } | Op::Unclip | Op::Layer(_) | Op::Unlayer)) {
            match under {
                Some(under) => copy(under, tile, pixmap),
                None => clear(tile, pixmap),
            }
            drawn += 1;
            continue;
        }
        match under {
            Some(under) => fetch(under, tile, &mut scratch),
            None => scratch.fill(tiny_skia::Color::TRANSPARENT),
        }
        let (x, y) = (grid.origin.0 + tile.x0, grid.origin.1 + tile.y0);
        raster::draw(&ops, &mut scratch.as_mut(), Transform::from_row(scale, 0.0, 0.0, scale, -x as f32, -y as f32));
        put(&scratch, tile, pixmap);
        drawn += 1;
    }
    drawn
}

/// The ops of `list` that reach `tile`, in order: a clip or layer that doesn't is left out with
/// its whole group.
fn reaching(list: &Marked, tile: Rect, out: &mut Vec<Op>) {
    let mut at = 0;
    while let Some(op) = list.ops.get(at) {
        let hits = list.reach[at].is_some_and(|r| over(r, tile));
        match op {
            Op::Clip { .. } | Op::Layer(_) if !hits => at = list.closes[at] + 1,
            Op::Clip { .. } | Op::Layer(_) | Op::Unclip | Op::Unlayer => {
                out.push(op.clone());
                at += 1;
            }
            op => {
                if hits {
                    out.push(op.clone());
                }
                at += 1;
            }
        }
    }
}

/// Copies a tile's drawing into its place in `pixmap`.
fn put(canvas: &Pixmap, tile: Patch, pixmap: &mut Pixmap) {
    let (w, from_stride, stride) = (4 * tile.width() as usize, 4 * canvas.width() as usize, 4 * pixmap.width() as usize);
    let from = canvas.data();
    let to = pixmap.data_mut();
    for (row, y) in (tile.y0..tile.y1).enumerate() {
        let at = y as usize * stride + 4 * tile.x0 as usize;
        to[at..at + w].copy_from_slice(&from[row * from_stride..row * from_stride + w]);
    }
}

/// Copies `tile` of `under` into the top left corner of `scratch`.
fn fetch(under: &Pixmap, tile: Patch, scratch: &mut Pixmap) {
    let (w, from_stride, to_stride) = (4 * tile.width() as usize, 4 * under.width() as usize, 4 * scratch.width() as usize);
    let from = under.data();
    let to = scratch.data_mut();
    for (row, y) in (tile.y0..tile.y1).enumerate() {
        let at = y as usize * from_stride + 4 * tile.x0 as usize;
        to[row * to_stride..row * to_stride + w].copy_from_slice(&from[at..at + w]);
    }
}

/// Copies `tile` of `under` into the same tile of `pixmap`, as big.
fn copy(under: &Pixmap, tile: Patch, pixmap: &mut Pixmap) {
    let (w, stride) = (4 * tile.width() as usize, 4 * pixmap.width() as usize);
    let (from, to) = (under.data(), pixmap.data_mut());
    for y in tile.y0..tile.y1 {
        let at = y as usize * stride + 4 * tile.x0 as usize;
        to[at..at + w].copy_from_slice(&from[at..at + w]);
    }
}

/// Clears a tile of `pixmap`.
fn clear(tile: Patch, pixmap: &mut Pixmap) {
    let stride = 4 * pixmap.width() as usize;
    let to = pixmap.data_mut();
    for y in tile.y0..tile.y1 {
        let at = y as usize * stride + 4 * tile.x0 as usize;
        to[at..at + 4 * tile.width() as usize].fill(0);
    }
}

#[cfg(test)]
mod tests;
