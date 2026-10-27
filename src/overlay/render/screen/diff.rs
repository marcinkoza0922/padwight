//! What a change to a panel's list touches: the ops of the old list and the new one that aren't
//! the same op in the same place, and where those can put ink.
//!
//! The lists are matched op by op (Myers' diff, on each op's hash and then the op itself). An op
//! left unmatched on either side is changed. A clip or layer changed is changed whole, its group
//! with it, and an op matched to a changed one is changed too, until nothing more changes. What
//! is left unchanged is then the same sequence of ops in both lists, each in the same clips and
//! layers, as every group taken out was taken out whole.
//!
//! An op's reach is where it can put ink on the picture: its bounds cut by the clips round it
//! and moved by the layers round it, and for a clip or layer the bounds of its whole group. A
//! tile of the picture that no changed op of either list reaches draws the same from both lists
//! (`tiles.rs` draws each tile from the ops that reach it), so only the tiles they reach are
//! drawn again.

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::overlay::render::{Layer, Op, Rect, bounds};

/// Past this many ops added or taken away, the lists are taken to differ throughout.
const MOST: usize = 160;

/// A list with what diffing and drawing it in tiles needs: each op's hash, where each group
/// closes, and each op's reach.
pub(super) struct Marked {
    pub ops: Vec<Op>,
    pub hashes: Vec<u64>,
    /// For a clip or layer, the index of the op that closes it (or the list's length).
    pub closes: Vec<usize>,
    /// Where each op can put ink, in logical pixels; `None` for a close or an op that draws
    /// nothing.
    pub reach: Vec<Option<Rect>>,
    /// Whether a layer round the op moves it.
    pub moved: Vec<bool>,
}

/// Where a changed op can put ink, in logical pixels, and the op when what it covers can say
/// more than that box (`raster::reaches`): one no layer moves.
pub(super) struct Reach<'a> {
    pub rect: Rect,
    pub op: Option<&'a Op>,
}

/// A group around an op: a clip cuts what it reaches, a layer moves it.
#[derive(Clone, Copy)]
enum Around {
    Clip(Rect),
    Layer(Layer),
}

impl Marked {
    pub fn new(ops: &[Op]) -> Marked {
        let hashes = ops
            .iter()
            .map(|op| {
                let mut h = DefaultHasher::new();
                op.hash(&mut h);
                h.finish()
            })
            .collect();
        let closes = closes(ops);
        let mut reach = vec![None; ops.len()];
        let mut moved = vec![false; ops.len()];
        let mut around: Vec<Around> = Vec::new();
        for (i, op) in ops.iter().enumerate() {
            match op {
                Op::Clip { rect, .. } => {
                    reach[i] = within(&around, bounds(&ops[i..closes[i].min(ops.len() - 1) + 1]));
                    around.push(Around::Clip(*rect));
                }
                Op::Layer(layer) => {
                    reach[i] = within(&around, bounds(&ops[i..closes[i].min(ops.len() - 1) + 1]));
                    around.push(Around::Layer(*layer));
                }
                // A close with nothing open is passed over, as the rasterizer does.
                Op::Unclip | Op::Unlayer => {
                    around.pop();
                }
                op => {
                    reach[i] = within(&around, op.bounds());
                    moved[i] = around.iter().any(|group| matches!(group, Around::Layer(_)));
                }
            }
        }
        Marked { ops: ops.to_vec(), hashes, closes, reach, moved }
    }

    fn same(&self, i: usize, other: &Marked, j: usize) -> bool {
        self.hashes[i] == other.hashes[j] && self.ops[i] == other.ops[j]
    }
}

/// `rect` cut by the clips and moved by the layers of `around`, innermost first.
fn within(around: &[Around], rect: Option<Rect>) -> Option<Rect> {
    let mut rect = rect?;
    for group in around.iter().rev() {
        rect = match group {
            Around::Clip(clip) => rect.intersection(*clip)?,
            Around::Layer(layer) => layer.map(rect),
        };
    }
    Some(rect)
}

/// For each clip or layer, the index of the op closing it, as the rasterizer pairs them (any
/// close closes the innermost group), or the list's length when it is left open.
fn closes(ops: &[Op]) -> Vec<usize> {
    let mut out = vec![0; ops.len()];
    let mut open = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Clip { .. } | Op::Layer(_) => open.push(i),
            Op::Unclip | Op::Unlayer => {
                if let Some(at) = open.pop() {
                    out[at] = i;
                }
            }
            _ => {}
        }
    }
    for at in open {
        out[at] = ops.len();
    }
    out
}

/// Where the ops changed between `old` and `new` reach, and how many ops they start with alike:
/// `None` when the lists differ too much to say.
pub(super) fn changed<'a>(old: &'a Marked, new: &'a Marked) -> Option<(Vec<Reach<'a>>, usize)> {
    let (n, m) = (old.ops.len(), new.ops.len());
    let mut start = 0;
    while start < n.min(m) && old.same(start, new, start) {
        start += 1;
    }
    let mut end = 0;
    while end < n.min(m) - start && old.same(n - 1 - end, new, m - 1 - end) {
        end += 1;
    }
    let pairs: Vec<(usize, usize)> =
        (0..start).map(|i| (i, i)).chain(matched(old, new, (start, n - end), (start, m - end))?).chain((0..end).map(|e| (n - end + e, m - end + e))).collect();
    let (mut was, mut now) = (vec![true; n], vec![true; m]);
    for &(i, j) in &pairs {
        (was[i], now[j]) = (false, false);
    }
    loop {
        let grew = whole_groups(old, &mut was) | whole_groups(new, &mut now);
        let mut spread = false;
        for &(i, j) in &pairs {
            if was[i] != now[j] {
                (was[i], now[j], spread) = (true, true, true);
            }
        }
        if !grew && !spread {
            break;
        }
    }
    let reached = |list: &'a Marked, changed: &[bool]| {
        (0..list.ops.len())
            .filter(|&i| changed[i])
            .filter_map(|i| list.reach[i].map(|rect| Reach { rect, op: (!list.moved[i]).then_some(&list.ops[i]) }))
            .collect::<Vec<_>>()
    };
    let mut out = reached(old, &was);
    out.extend(reached(new, &now));
    Some((out, start))
}

/// Marks the whole group of a changed clip or layer, or of a changed close, changed; returns
/// whether that marked any more.
fn whole_groups(list: &Marked, changed: &mut [bool]) -> bool {
    let mut grew = false;
    for (i, op) in list.ops.iter().enumerate() {
        if !matches!(op, Op::Clip { .. } | Op::Layer(_)) {
            continue;
        }
        let close = list.closes[i];
        let last = close.min(list.ops.len() - 1);
        if (changed[i] || changed[last]) && !changed[i..=last].iter().all(|c| *c) {
            changed[i..=last].fill(true);
            grew = true;
        }
    }
    grew
}

/// The pairs of ops matched between `old[a.0..a.1]` and `new[b.0..b.1]` by Myers' diff, in
/// order, as indices into the whole lists; `None` past `MOST` ops added or taken away.
fn matched(old: &Marked, new: &Marked, a: (usize, usize), b: (usize, usize)) -> Option<Vec<(usize, usize)>> {
    let (n, m) = (a.1 - a.0, b.1 - b.0);
    let same = |x: usize, y: usize| old.same(a.0 + x, new, b.0 + y);
    let most = (n + m).min(MOST);
    let off = most as isize + 1;
    let mut v = vec![0usize; 2 * most + 3];
    let mut trace: Vec<Vec<usize>> = Vec::new();
    for d in 0..=most as isize {
        trace.push(v.clone());
        for k in (-d..=d).step_by(2) {
            let at = (k + off) as usize;
            let mut x = if k == -d || (k != d && v[at - 1] < v[at + 1]) { v[at + 1] } else { v[at - 1] + 1 };
            let mut y = (x as isize - k) as usize;
            if x > n || y > m {
                v[at] = x;
                continue;
            }
            while x < n && y < m && same(x, y) {
                (x, y) = (x + 1, y + 1);
            }
            v[at] = x;
            if x == n && y == m {
                return Some(back(&trace, (n, m), d, off).into_iter().map(|(x, y)| (a.0 + x, b.0 + y)).collect());
            }
        }
    }
    None
}

/// The matched pairs on the path that reached `end` after `d` steps, walking `trace` back.
fn back(trace: &[Vec<usize>], end: (usize, usize), d: isize, off: isize) -> Vec<(usize, usize)> {
    let (mut x, mut y) = (end.0 as isize, end.1 as isize);
    let mut pairs = Vec::new();
    for d in (0..=d).rev() {
        let k = x - y;
        let (px, py) = if d == 0 {
            (0, 0)
        } else {
            let v = &trace[d as usize];
            let pk = if k == -d || (k != d && v[(k - 1 + off) as usize] < v[(k + 1 + off) as usize]) { k + 1 } else { k - 1 };
            let px = v[(pk + off) as usize] as isize;
            (px, px - pk)
        };
        // The step from (px, py) is down (one added) or right (one taken away), then the run of
        // matches along the diagonal to (x, y).
        let (sx, sy) = if d == 0 { (0, 0) } else if x - y == px - py + 1 { (px + 1, py) } else { (px, py + 1) };
        while x > sx && y > sy {
            (x, y) = (x - 1, y - 1);
            pairs.push((x as usize, y as usize));
        }
        (x, y) = (px, py);
    }
    pairs.reverse();
    pairs
}

#[cfg(test)]
mod tests;
