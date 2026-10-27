//! Screen damage: which device pixels a frame has to clear and redraw.
//!
//! A frame's own damage is what changed since the frame before: the union of what each panel
//! covered on the buffer before and what it covers now, for every panel whose picture, place,
//! zoom or opacity changed, came or went (`Screen` in `pictures.rs` works that out). It is kept
//! as a few disjoint patches: overlapping or nearby ones are merged into their bounding box, and
//! past `MOST` of them all become one box.
//!
//! The window reuses its buffers, each holding a frame some frames old (its age: 1 is the last
//! frame committed, 2 the one before; 0 is unknown contents). Bringing a buffer of age `n` up to
//! date takes this frame's damage and the damage of the `n - 1` frames since it was drawn:
//! `History` keeps the last few frames' damage for that, and asks for everything when the
//! buffer is older than what it keeps.

use std::collections::VecDeque;

/// Patches nearer each other than this (device pixels, either way) are merged.
const GAP: i32 = 16;
/// Beyond this many patches, one box around them all is painted instead.
const MOST: usize = 8;
/// How many frames' damage is kept: a buffer older than this is redrawn whole. The window has
/// three buffers, so ages past 3 are rare.
const FRAMES: usize = 4;

/// A rectangle of device pixels: columns `x0..x1` and rows `y0..y1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Patch {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Patch {
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Patch {
        Patch { x0, y0, x1, y1 }
    }

    pub fn width(&self) -> i32 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> i32 {
        self.y1 - self.y0
    }

    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    /// Whether the pixel at column `x`, row `y` lies in it.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }

    /// Whether `other` lies wholly inside it.
    pub fn holds(&self, other: &Patch) -> bool {
        self.x0 <= other.x0 && self.y0 <= other.y0 && other.x1 <= self.x1 && other.y1 <= self.y1
    }

    /// Where it and `other` overlap, if they do.
    pub fn intersection(&self, other: &Patch) -> Option<Patch> {
        let p = Patch::new(self.x0.max(other.x0), self.y0.max(other.y0), self.x1.min(other.x1), self.y1.min(other.y1));
        (!p.is_empty()).then_some(p)
    }

    /// The box round it and `other`.
    fn around(&self, other: &Patch) -> Patch {
        Patch::new(self.x0.min(other.x0), self.y0.min(other.y0), self.x1.max(other.x1), self.y1.max(other.y1))
    }

    /// Whether it and `other` overlap or are less than `GAP` apart.
    fn near(&self, other: &Patch) -> bool {
        self.x0 - GAP < other.x1 && other.x0 - GAP < self.x1 && self.y0 - GAP < other.y1 && other.y0 - GAP < self.y1
    }
}

/// The pixels a frame repainted: a few disjoint patches within the buffer. Each pixel lies in
/// one patch at most, so a pass over the patches (swapping red and blue) touches it once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Damage {
    patches: Vec<Patch>,
}

impl Damage {
    /// The patches, disjoint.
    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    /// Nothing repainted: the buffer is as it was, and there is nothing to commit.
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    /// Whether the pixel at column `x`, row `y` was repainted.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.patches.iter().any(|p| p.contains(x, y))
    }

    /// The whole of a `width` × `height` buffer.
    pub fn full(width: u32, height: u32) -> Damage {
        Damage::of([Patch::new(0, 0, width as i32, height as i32)], width, height)
    }

    /// `patches` cut to a `width` × `height` buffer and merged till none are near each other.
    pub(super) fn of(patches: impl IntoIterator<Item = Patch>, width: u32, height: u32) -> Damage {
        let screen = Patch::new(0, 0, width as i32, height as i32);
        let mut patches: Vec<Patch> = patches.into_iter().filter_map(|p| p.intersection(&screen)).collect();
        while let Some((i, j)) = near_pair(&patches) {
            let other = patches.swap_remove(j);
            patches[i] = patches[i].around(&other);
        }
        if patches.len() > MOST {
            let around = patches.iter().fold(patches[0], |all, p| all.around(p));
            patches = vec![around];
        }
        Damage { patches }
    }
}

/// Two patches near each other, the first before the second.
fn near_pair(patches: &[Patch]) -> Option<(usize, usize)> {
    (0..patches.len()).find_map(|i| (i + 1..patches.len()).find(|&j| patches[i].near(&patches[j])).map(|j| (i, j)))
}

/// The last few frames' own damage, newest first.
#[derive(Debug, Default)]
pub(super) struct History {
    frames: VecDeque<Vec<Patch>>,
}

impl History {
    /// What a buffer of `age` needs repainted besides `now` (this frame's own damage): the
    /// damage of the `age - 1` frames before it. `None` when that is everything: an unknown
    /// buffer, or one older than the frames kept.
    pub(super) fn since(&self, age: u64, now: &[Patch]) -> Option<Vec<Patch>> {
        let back = usize::try_from(age.checked_sub(1)?).ok().filter(|&n| n <= self.frames.len())?;
        Some(now.iter().chain(self.frames.iter().take(back).flatten()).copied().collect())
    }

    /// Notes a committed frame's own damage.
    pub(super) fn push(&mut self, own: &Damage) {
        self.frames.push_front(own.patches.clone());
        self.frames.truncate(FRAMES);
    }

    pub(super) fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Swaps the red and blue bytes of each pixel `damage` covers, in `pixels`: rows of `width`
/// four-byte pixels. That turns tiny-skia's RGBA into ARGB8888's little-endian B, G, R, A over
/// the repainted patches, and leaves the rest, already swapped in an earlier frame, alone.
pub fn swap_red_blue(pixels: &mut [u8], width: u32, damage: &Damage) {
    let stride = 4 * width as usize;
    for p in &damage.patches {
        let (x0, x1) = (4 * p.x0.max(0) as usize, 4 * p.x1.max(0) as usize);
        for y in p.y0.max(0) as usize..p.y1.max(0) as usize {
            let Some(row) = pixels.get_mut(y * stride + x0..y * stride + x1.min(stride)) else { break };
            for px in row.as_chunks_mut::<4>().0 {
                px.swap(0, 2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_patches_merge_and_far_ones_stay_apart() {
        let d = Damage::of([Patch::new(0, 0, 10, 10), Patch::new(20, 0, 30, 10), Patch::new(500, 500, 510, 510)], 1000, 1000);
        assert_eq!(d.patches(), &[Patch::new(0, 0, 30, 10), Patch::new(500, 500, 510, 510)]);
    }

    #[test]
    fn patches_are_cut_to_the_buffer() {
        let d = Damage::of([Patch::new(-5, -5, 10, 10), Patch::new(2000, 0, 2100, 10)], 100, 100);
        assert_eq!(d.patches(), &[Patch::new(0, 0, 10, 10)]);
    }

    #[test]
    fn many_patches_become_one_box() {
        let many = (0..20).map(|i| Patch::new(i * 100, i * 50, i * 100 + 10, i * 50 + 10));
        let d = Damage::of(many, 4000, 4000);
        assert_eq!(d.patches(), &[Patch::new(0, 0, 1910, 960)]);
    }

    #[test]
    fn merged_patches_never_overlap() {
        let ps = (0..40).map(|i| Patch::new((i * 37) % 300, (i * 53) % 200, (i * 37) % 300 + 40, (i * 53) % 200 + 25));
        let d = Damage::of(ps, 400, 300);
        for (i, a) in d.patches().iter().enumerate() {
            for b in &d.patches()[i + 1..] {
                assert!(a.intersection(b).is_none(), "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn the_swap_turns_only_the_damage() {
        let mut px: Vec<u8> = (0..4 * 4 * 3).map(|i| i as u8).collect();
        let before = px.clone();
        let d = Damage::of([Patch::new(1, 1, 3, 2)], 4, 3);
        swap_red_blue(&mut px, 4, &d);
        for (i, (a, b)) in px.as_chunks::<4>().0.iter().zip(before.as_chunks::<4>().0).enumerate() {
            let (x, y) = (i as i32 % 4, i as i32 / 4);
            let want = if d.contains(x, y) { [b[2], b[1], b[0], b[3]] } else { *b };
            assert_eq!(*a, want, "({x}, {y})");
        }
    }

    #[test]
    fn history_unions_the_frames_since_the_buffer() {
        let mut h = History::default();
        let p = |i| Patch::new(i, 0, i + 1, 1);
        h.push(&Damage { patches: vec![p(1)] });
        h.push(&Damage { patches: vec![p(2)] });
        assert_eq!(h.since(0, &[p(0)]), None, "unknown contents");
        assert_eq!(h.since(1, &[p(0)]), Some(vec![p(0)]));
        assert_eq!(h.since(2, &[p(0)]), Some(vec![p(0), p(2)]));
        assert_eq!(h.since(3, &[p(0)]), Some(vec![p(0), p(2), p(1)]));
        assert_eq!(h.since(4, &[p(0)]), None, "older than the history");
    }
}
