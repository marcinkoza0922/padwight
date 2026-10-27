//! Each panel drawn once into a picture of its own, and laid onto the screen as it fades, slides
//! and zooms.
//!
//! A panel is a layer at the top level of the screen's list (see `screen_ops`). Its ops are
//! drawn at full strength, at the device scale, into a picture as big as their ink (shadow and
//! all) plus a clear pixel all round, in the screen's own device pixels: the picture's corner
//! is on a whole pixel and the panel lies in it where it lies on the screen at rest, so a
//! picture laid down unmoved is the same as drawing the ops straight onto the screen. The
//! picture is kept, keyed by the list's `digest` and the scale and checked against the list
//! itself, while the list stays the same; a few are kept, the least recently used dropped.
//!
//! Laying it down (`lay.rs`): pixmaps hold premultiplied color, so a picture laid at opacity
//! `a` has all four channels multiplied by `a` and goes over the screen source-over: every part
//! of the panel fades by the same amount, so an outline never shows ahead of the label inside
//! it. Unzoomed, its offset is rounded to whole device pixels and the picture copied pixel for
//! pixel, so text doesn't shimmer as it slides. Zoomed, it is sampled bilinearly; sampling
//! premultiplied pixels, and the clear pixel round the edge, keep its rim free of dark fringes
//! and hard cut edges.
//!
//! What moves inside a panel (a highlight gliding, a pick flashing, an item swelling or fading
//! in) changes its list. The picture is drawn in tiles (`tiles.rs`), so a changed list redraws
//! only the tiles its changed ops reach (`diff.rs`), in the picture laid last in the same place,
//! each from the list drawn as far as its changes began (`under.rs`); the frame's damage is
//! then those tiles alone. When most of the picture would be redrawn, a new one is drawn
//! instead, and the old one kept.
//!
//! The picture's corner is on a whole pixel, and the rasterizer draws a list the same whatever
//! pixel a pixmap's corner is on (`tiles.rs`), so the picture is the ops drawn straight onto the
//! screen within a level.

use std::time::{Duration, Instant};

use tiny_skia::{Pixmap, PixmapMut};

use super::{
    Shown,
    damage::{Damage, History, Patch},
    diff::{self, Marked},
    items::{self, Item, What},
    screen_ops,
    tiles::{self, Grid},
    under::{self, Under},
};
use crate::overlay::{
    OverlayFrame,
    render::{Layer, Op, Point, bounds, digest, raster},
};

/// How many panel pictures are kept besides those on screen: enough for the panels open at
/// once and the few they change between (a list's highlight moving back and forth).
const KEPT: usize = 8;
/// Far past any screen, as a guard against a runaway size.
const LIMIT: i32 = 16_384;

/// A panel's list drawn into a picture.
struct Kept {
    /// Its number, never given to another picture.
    id: u64,
    digest: u64,
    /// The device scale's bits.
    scale: u32,
    list: Marked,
    pixmap: Pixmap,
    /// Where the picture's top left corner lies on the screen at rest, in device pixels, and
    /// its size.
    grid: Grid,
    /// How many times its tiles have been drawn again from a changed list.
    version: u64,
    /// The pixels of the picture its last redraw changed.
    redrawn: Vec<Patch>,
    /// Its list drawn as far as its changes began, for redrawing tiles from.
    under: Option<Under>,
    /// The frame it was last laid down in.
    used: u64,
}

/// The screen drawn on: its size in logical pixels, and device pixels per logical one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    pub size: Point,
    pub scale: f32,
}

/// The last frame painted: what it laid down, on a buffer of what size, at what scale.
struct Last {
    items: Vec<Item>,
    size: (u32, u32),
    scale: u32,
}

/// Draws whole screens, keeping each panel's picture from one frame to the next, and what the
/// last frames laid down, to repaint only what changed (`paint`).
#[derive(Default)]
pub struct Screen {
    kept: Vec<Kept>,
    frame: u64,
    drawn: u64,
    tiles: u64,
    pictures: u64,
    last: Option<Last>,
    history: History,
    /// Time spent since `spent` was last asked: marking and diffing lists, and drawing tiles.
    spent: [Duration; 2],
}

impl Screen {
    pub fn new() -> Screen {
        Screen::default()
    }

    /// Draws `shown` on `display`, over what `target` (as big as the display in device pixels)
    /// holds: clear it first for a fresh screen. The next `paint` repaints everything.
    pub fn draw(&mut self, frame: &OverlayFrame, shown: &Shown, display: Display, target: &mut PixmapMut) {
        self.lay(&screen_ops(frame, shown, display.size), display.scale, target);
    }

    /// Draws a screen's list over what `target` holds: each layer at its top level is a panel,
    /// laid down from its picture; anything else there is drawn as it is.
    pub fn lay(&mut self, ops: &[Op], scale: f32, target: &mut PixmapMut) {
        self.frame += 1;
        let items = self.items(ops, scale);
        let whole = Patch::new(0, 0, target.width() as i32, target.height() as i32);
        for item in &items {
            self.lay_item(item, scale, (whole, false), target);
        }
        self.reset();
        self.prune();
    }

    /// Paints a screen's list (`screen_ops`) into `target`, a reused buffer holding the frame
    /// painted `age` frames back (1: the last one; 0: unknown contents), and returns the
    /// patches it cleared and redrew: the rest of `target` was neither read nor written.
    ///
    /// This frame's own damage is where any panel changed (its picture, place, zoom or
    /// opacity), was or now is; with the damage of the `age - 1` frames before it, that is what
    /// `target` needs. Each of those patches is cleared to transparent and every panel over it
    /// laid down again, cut to it. Everything is repainted for an unknown buffer (age 0), one
    /// older than the few frames remembered, the first frame, a new buffer size or scale, and
    /// after `forget` or `lay`.
    ///
    /// Each call counts as a frame committed, unless it returns no damage: then nothing changed
    /// and the buffer is as it was, to be left uncommitted.
    pub fn paint(&mut self, ops: &[Op], scale: f32, target: &mut PixmapMut, age: u64) -> Damage {
        self.frame += 1;
        let (size, bits) = ((target.width(), target.height()), scale.to_bits());
        let items = self.items(ops, scale);
        let own = match &self.last {
            Some(last) if (last.size, last.scale) == (size, bits) => Damage::of(items::changed(&last.items, &items), size.0, size.1),
            _ => {
                self.history.clear();
                Damage::full(size.0, size.1)
            }
        };
        let repaint = match self.history.since(age, own.patches()) {
            Some(patches) => Damage::of(patches, size.0, size.1),
            None => Damage::full(size.0, size.1),
        };
        if !repaint.is_empty() {
            self.history.push(&own);
        }
        for &patch in repaint.patches() {
            items::clear(target, patch);
            // The first goes down onto the cleared patch, without reading it.
            for (n, item) in items.iter().filter(|i| i.at.intersection(&patch).is_some()).enumerate() {
                self.lay_item(item, scale, (patch, n == 0), target);
            }
        }
        self.last = Some(Last { items, size, scale: bits });
        self.prune();
        repaint
    }

    /// How many panel pictures have been drawn (rather than reused) so far.
    pub fn drawn(&self) -> u64 {
        self.drawn
    }

    /// How many tiles of kept pictures have been drawn again (rather than whole pictures) so
    /// far.
    pub fn tiles(&self) -> u64 {
        self.tiles
    }

    /// The time spent marking and diffing panels' lists, and drawing their tiles, since last
    /// asked.
    pub fn spent(&mut self) -> [Duration; 2] {
        std::mem::take(&mut self.spent)
    }

    /// How many pictures are kept.
    #[cfg(test)]
    pub(super) fn kept(&self) -> usize {
        self.kept.len()
    }

    /// Drops every picture, as when the overlay hides; the next `paint` repaints everything.
    pub fn forget(&mut self) {
        self.kept.clear();
        raster::forget_shapes();
        self.reset();
    }

    /// Forgets what the last frames laid down: the next `paint` repaints everything.
    fn reset(&mut self) {
        self.last = None;
        self.history.clear();
    }

    /// What `ops` lays at their top level, in order, drawing the pictures not kept.
    fn items(&mut self, ops: &[Op], scale: f32) -> Vec<Item> {
        let mut items = Vec::new();
        let mut at = 0;
        while let Some(op) = ops.get(at) {
            let next = match op {
                Op::Layer(layer) => {
                    let (end, next) = close(ops, at + 1);
                    items.extend(self.panel(layer, &ops[at + 1..end], scale));
                    next
                }
                // A close with nothing open is passed over, as the rasterizer does.
                Op::Unlayer | Op::Unclip => at + 1,
                op => {
                    let next = if let Op::Clip { .. } = op { close(ops, at + 1).1 } else { at + 1 };
                    let straight = &ops[at..next];
                    if let Some(patch) = items::straight_patch(straight, scale) {
                        items.push(Item { what: What::Straight(straight.to_vec()), at: patch, redrawn: Vec::new() });
                    }
                    next
                }
            };
            at = next;
        }
        items
    }

    /// A panel's picture as it goes down this frame: a kept one with the same list, else the
    /// one laid last in the same place with the tiles its list changed drawn again (when that
    /// isn't most of them), else a new one.
    fn panel(&mut self, layer: &Layer, ops: &[Op], scale: f32) -> Option<Item> {
        if !(layer.opacity > 0.0 && layer.scale > 0.0) {
            return None;
        }
        let key = (digest(ops), scale.to_bits());
        let found = self.kept.iter().position(|k| (k.digest, k.scale) == key && k.list.ops == ops);
        let i = match found {
            Some(i) => i,
            None => {
                let start = Instant::now();
                let list = Marked::new(ops);
                self.spent[0] += start.elapsed();
                let grid = grid(ops, scale)?;
                match self.redraw(list, grid, key.0) {
                    Ok(i) => i,
                    Err(list) => {
                        let mut pixmap = Pixmap::new(grid.size.0 as u32, grid.size.1 as u32)?;
                        let start = Instant::now();
                        tiles::draw(&list, grid, &vec![true; grid.count()], &mut pixmap, None);
                        self.spent[1] += start.elapsed();
                        self.drawn += 1;
                        self.pictures += 1;
                        let (id, redrawn) = (self.pictures, Vec::new());
                        self.kept.push(Kept { id, digest: key.0, scale: key.1, list, pixmap, grid, version: 0, redrawn, under: None, used: 0 });
                        self.kept.len() - 1
                    }
                }
            }
        };
        let kept = &mut self.kept[i];
        kept.used = self.frame;
        let (laid, at) = items::laid((kept.pixmap.width(), kept.pixmap.height()), kept.grid.origin, layer, scale);
        let redrawn = items::moved(&kept.redrawn, &laid, at);
        Some(Item { what: What::Picture { id: kept.id, version: kept.version, laid }, at, redrawn })
    }

    /// Draws again the tiles that `list` changes of the picture laid most recently in the
    /// same place, at the same size and scale, and not yet this frame; hands `list` back when
    /// there is none, or it would redraw most of it.
    fn redraw(&mut self, list: Marked, grid: Grid, digest: u64) -> Result<usize, Marked> {
        let frame = self.frame;
        let Some(i) = (0..self.kept.len())
            .filter(|&i| self.kept[i].grid == grid && self.kept[i].used != frame)
            .max_by_key(|&i| self.kept[i].used)
        else {
            return Err(list);
        };
        let start = Instant::now();
        let rects = diff::changed(&self.kept[i].list, &list);
        self.spent[0] += start.elapsed();
        let kept = &mut self.kept[i];
        let Some((rects, alike)) = rects else { return Err(list) };
        let which = tiles::reached(grid, &rects);
        let count = which.iter().filter(|w| **w).count();
        if 4 * count > 3 * grid.count() {
            return Err(list);
        }
        let start = Instant::now();
        under::update(&mut kept.under, &list, alike, grid);
        match &kept.under {
            Some(u) => tiles::draw(&Marked::new(&under::rest(&list.ops, u.upto)), grid, &which, &mut kept.pixmap, Some(&u.pixmap)),
            None => tiles::draw(&list, grid, &which, &mut kept.pixmap, None),
        };
        self.spent[1] += start.elapsed();
        kept.redrawn = (0..grid.count()).filter(|&n| which[n]).map(|n| grid.tile(n)).collect();
        (kept.list, kept.digest, kept.version) = (list, digest, kept.version + 1);
        self.tiles += count as u64;
        Ok(i)
    }

    /// Lays `item` down over `target`, only within `clip`, which is clear when `onto_clear`.
    fn lay_item(&self, item: &Item, scale: f32, (clip, onto_clear): (Patch, bool), target: &mut PixmapMut) {
        let Some(clip) = item.at.intersection(&clip) else { return };
        match &item.what {
            What::Picture { id, laid, .. } => {
                if let Some(kept) = self.kept.iter().find(|k| k.id == *id) {
                    items::lay_picture(&kept.pixmap, laid, clip, target, onto_clear);
                }
            }
            What::Straight(ops) => items::draw_straight(ops, scale, clip, target),
        }
    }

    /// Keeps the pictures laid down this frame and the most recently used of the rest, up to
    /// `KEPT` of those.
    fn prune(&mut self) {
        let now = self.kept.iter().filter(|k| k.used == self.frame).count();
        self.kept.sort_by_key(|k| std::cmp::Reverse(k.used));
        self.kept.truncate(now + KEPT);
    }
}

/// Where the group opened just before `at` ends: the index of its close (or the end), and the
/// index after it.
fn close(ops: &[Op], mut at: usize) -> (usize, usize) {
    let mut depth = 0;
    while let Some(op) = ops.get(at) {
        match op {
            Op::Clip { .. } | Op::Layer(_) => depth += 1,
            Op::Unclip | Op::Unlayer if depth == 0 => return (at, at + 1),
            Op::Unclip | Op::Unlayer => depth -= 1,
            _ => {}
        }
        at += 1;
    }
    (at, at)
}

/// The picture `ops` are drawn into at `scale`: covering their ink and a pixel more all round,
/// its corner on a whole device pixel; `None` when they draw nothing.
fn grid(ops: &[Op], scale: f32) -> Option<Grid> {
    let ink = bounds(ops)?;
    let edge = |v: f32| (v * scale).clamp(-LIMIT as f32, LIMIT as f32);
    let (x0, y0) = (edge(ink.x).floor() as i32 - 1, edge(ink.y).floor() as i32 - 1);
    let (x1, y1) = (edge(ink.right()).ceil() as i32 + 1, edge(ink.bottom()).ceil() as i32 + 1);
    if x1 - x0 > LIMIT || y1 - y0 > LIMIT {
        return None;
    }
    Some(Grid { origin: (x0, y0), size: (x1 - x0, y1 - y0), scale })
}
