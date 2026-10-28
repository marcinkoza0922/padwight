//! The marks a pen makes: a solid line, a bristle brush, and an airbrush's spray of dots.
//!
//! A stroke is drawn in runs of `RUN` segments, an op each. As a stroke grows only its last run
//! or two change, so the screen repaints only where they lie, and every other run is laid from
//! the coverage the rasterizer kept for it. One op for the whole stroke changed with each point
//! the stick added: its whole box was repainted, and its whole shape rasterized again, each frame.

use std::{f32::consts::TAU, ops::Range};

use crate::overlay::render::{Cap, Join, Op, Path, Point, Rgba, Rule, Stroke, blend};

/// How many segments of a stroke each of its ops draws.
const RUN: usize = 32;

/// The brush: how far apart its bristles are and how many it has at least and at most, each one's
/// width as a share of its spacing (more than 1, so they overlap), its shades (how far toward white,
/// or toward black where it's below 0, and how wide their bristles are), how narrow the stroke
/// starts (as a share of its width) and over how many widths it spreads out, and how often its edge
/// bristles run dry, in spans of `DRY_SPAN` segments.
const BRISTLE_SPACING: f32 = 1.3;
const BRISTLES: (usize, usize) = (8, 24);
const BRISTLE_WIDTH: f32 = 2.0;
const SHADES: [(f32, f32); 4] = [(0.0, 1.0), (0.1, 0.85), (-0.14, 1.1), (0.22, 0.7)];
const PRESS: f32 = 0.45;
const PRESS_LENGTH: f32 = 1.5;
const DRY: f32 = 0.5;
/// How far out from the middle, as a share of half the width, the bristles never run dry.
const WET: f32 = 0.4;
const DRY_SPAN: usize = 10;

/// The spray: how much of the area it passes over its dots cover, how many it puts down in its
/// first burst for each of its radius's pixels, how strongly they gather to its middle (1 is
/// evenly along the radius, which is already thickest at the middle; more is thicker still), and
/// a dot's size: fine at any width, a little larger the wider the spray. A spray has many more
/// shapes to a segment than a line, so its ops are a run of `SPRAY_RUN` segments: a tile drawn
/// again rasterizes only the dots near it.
const SPRAY_COVER: f32 = 0.25;
const SPRAY_RUN: usize = 8;
const BURST: f32 = 4.0;
const SPRAY_FALLOFF: f32 = 1.3;
const DOT: (f32, f32) = (1.3, 0.02);

/// The points of each run of a line of `n` points: `RUN` segments, and one more at each end
/// where there is one, so the join where two runs meet is drawn whole by both.
fn runs(n: usize) -> impl Iterator<Item = Range<usize>> {
    (0..n.saturating_sub(1).div_ceil(RUN).max(1)).map(move |k| (k * RUN).saturating_sub(1)..((k + 1) * RUN + 2).min(n))
}

/// Draws the line through `px` (logical pixels) `width` wide in `color`, its ends capped by `cap`.
///
/// Runs meet where the line goes on, so their own ends mustn't show: a round cap is inside the
/// round join the next run makes there, so round runs are capped round; any other line's runs
/// are cut square at their ends (inside the run they overlap), and its two real ends are capped
/// on their own. `along` adds what goes with the `k`th run after it, so a stroke's ops run from
/// its start to its end, and those that change as it grows are the last.
pub fn line(ops: &mut Vec<Op>, px: &[Point], (width, color, cap): (f32, Rgba, Cap), mut along: impl FnMut(&mut Vec<Op>, usize)) {
    let single = px.len() <= RUN + 1;
    let runs_cap = if single { cap } else if cap == Cap::Round { Cap::Round } else { Cap::Butt };
    for (k, run) in runs(px.len()).enumerate() {
        let mut path = Path::new();
        trace(&mut path, &px[run]);
        let stroke = Stroke { cap: runs_cap, join: Join::Round, ..Stroke::new(width, color) };
        ops.push(Op::Path { path, fill: None, rule: Rule::NonZero, stroke: Some(stroke) });
        along(ops, k);
    }
    if cap == Cap::Square && !single {
        square_cap(ops, px.iter().rev(), width, color);
        square_cap(ops, px.iter(), width, color);
    }
}

/// A square cap at the line's end `points` starts from (the end first): half the width past the
/// end, along the line's last stretch there. It reaches a pixel back over the line, so no seam
/// shows where the two meet.
fn square_cap<'a>(ops: &mut Vec<Op>, mut points: impl Iterator<Item = &'a Point>, width: f32, color: Rgba) {
    let Some(&end) = points.next() else { return };
    let Some(&from) = points.find(|p| **p != end) else { return };
    let (dx, dy) = (end.x - from.x, end.y - from.y);
    let length = dx.hypot(dy);
    let h = width / 2.0;
    let (ax, ay) = (dx / length * h, dy / length * h);
    let (nx, ny) = (-ay, ax);
    let back = Point::new(end.x - dx / length, end.y - dy / length);
    let mut path = Path::new();
    path.move_to(Point::new(back.x + nx, back.y + ny));
    path.line_to(Point::new(end.x + nx + ax, end.y + ny + ay));
    path.line_to(Point::new(end.x - nx + ax, end.y - ny + ay));
    path.line_to(Point::new(back.x - nx, back.y - ny));
    path.close();
    ops.push(Op::Path { path, fill: Some(color.into()), rule: Rule::NonZero, stroke: None });
}

/// Paints `px` (logical pixels) `width` wide in `color` with a bristle brush, the way a paint
/// program's oil or bristle brush does: bristles across the width, each a little lighter or darker
/// than the paint, pressed in from a narrower start, and the outer ones running dry in streaks.
/// Each run of `RUN` segments is an op for each shade, its bristles' paths in it.
pub fn brush(ops: &mut Vec<Op>, px: &[Point], width: f32, color: Rgba) {
    let course = Course { px, normals: normals(px), along: lengths(px), width };
    let segments = px.len().saturating_sub(1);
    let count = ((width / BRISTLE_SPACING).round() as usize).clamp(BRISTLES.0, BRISTLES.1);
    let bristles: Vec<Bristle> = (0..count)
        .map(|j| {
            let even = (j as f32 + 0.5) / count as f32 - 0.5;
            let jitter = (noise(j as u64, 0) - 0.5) / count as f32 / 2.0;
            let shade = (noise(j as u64, 1) * SHADES.len() as f32) as usize % SHADES.len();
            Bristle { offset: (even + jitter) * width, shade, seed: j as u64 }
        })
        .collect();
    let thick = width / count as f32 * BRISTLE_WIDTH;
    for k in 0..segments.div_ceil(RUN).max(1) {
        let run = (k * RUN).min(segments)..((k + 1) * RUN).min(segments);
        for (shade, &(tone, thickness)) in SHADES.iter().enumerate() {
            let mut path = Path::new();
            for bristle in bristles.iter().filter(|b| b.shade == shade) {
                bristle_along(&mut path, &course, bristle, run.clone());
            }
            if !path.segs().is_empty() {
                // The shades' bristles differ in width too, so their streaks don't line up.
                let ink = if tone < 0.0 { blend(color, Rgba::BLACK, -tone) } else { blend(color, Rgba::WHITE, tone) };
                let stroke = Stroke { cap: Cap::Round, join: Join::Round, ..Stroke::new(thick * thickness, ink) };
                ops.push(Op::Path { path, fill: None, rule: Rule::NonZero, stroke: Some(stroke) });
            }
        }
    }
}

/// The line a brush paints along: its points (logical pixels), the unit normal at each and how far
/// along it each is, and how wide the brush is.
struct Course<'a> {
    px: &'a [Point],
    normals: Vec<Point>,
    along: Vec<f32>,
    width: f32,
}

/// One bristle of a brush: how far it stands from the line's middle, which of the shades it
/// paints in, and what its streaks are drawn from.
struct Bristle {
    offset: f32,
    shade: usize,
    seed: u64,
}

/// Adds `bristle`'s trail along the `segments` of `course` to `path`, with gaps where it runs dry:
/// never at the middle, more often the nearer the edge, a few segments at a time. A single point
/// is a dot. All the bristles of a shade go in one path, so a shade is one op.
fn bristle_along(path: &mut Path, course: &Course, bristle: &Bristle, segments: Range<usize>) {
    let Course { px, normals, along, width } = course;
    let width = *width;
    // Pressed in from the start: the bristles spread from `PRESS` of the width to all of it.
    let spread = |i: usize| PRESS + (1.0 - PRESS) * (along[i] / (width * PRESS_LENGTH)).min(1.0);
    let shifted = |i: usize| {
        let offset = bristle.offset * spread(i);
        Point::new(px[i].x + normals[i].x * offset, px[i].y + normals[i].y * offset)
    };
    if px.len() == 1 {
        trace(path, &[shifted(0)]);
        return;
    }
    let edge = (bristle.offset.abs() / (width / 2.0)).min(1.0);
    let dry = DRY * ((edge - WET) / (1.0 - WET)).max(0.0);
    let mut running = false;
    for i in segments {
        if noise(bristle.seed, (i / DRY_SPAN) as u64 + 2) < dry {
            running = false;
            continue;
        }
        if !running {
            path.move_to(shifted(i));
            running = true;
        }
        path.line_to(shifted(i + 1));
    }
}

/// How far along the line through `px` each of its points is.
fn lengths(px: &[Point]) -> Vec<f32> {
    let mut total = 0.0;
    let mut out = Vec::with_capacity(px.len());
    for (i, p) in px.iter().enumerate() {
        if i > 0 {
            total += (p.x - px[i - 1].x).hypot(p.y - px[i - 1].y);
        }
        out.push(total);
    }
    out
}

/// The unit normal at each point of the line through `px`, pointing to its left.
fn normals(px: &[Point]) -> Vec<Point> {
    (0..px.len())
        .map(|i| {
            let (a, b) = (px[i.saturating_sub(1)], px[(i + 1).min(px.len() - 1)]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let length = dx.hypot(dy);
            if length == 0.0 { Point::ZERO } else { Point::new(-dy / length, dx / length) }
        })
        .collect()
}

/// Adds the line through `px` to `path`, with a point at its first place even when it's alone, so
/// a single point still makes a dot.
fn trace(path: &mut Path, px: &[Point]) {
    let Some(&first) = px.first() else { return };
    path.move_to(first);
    for &point in px {
        path.line_to(point);
    }
}

/// Sprays paint `width` wide along `px`, the way a paint program's airbrush does: fine round dots
/// scattered over a disc at each step, thickest at its middle and thinning to its rim, as many
/// for each stretch as its length. The dots are random but fixed by where they fall in the stroke,
/// so the same stroke sprays the same dots each frame. Each run of `SPRAY_RUN` segments is an op.
pub fn spray(ops: &mut Vec<Op>, px: &[Point], width: f32, color: Rgba) {
    let nozzle = Nozzle { radius: width / 2.0, dot: DOT.0 + width * DOT.1 };
    // Dots for each pixel it moves, so they cover `SPRAY_COVER` of the band it sweeps.
    let per_pixel = SPRAY_COVER * width / (nozzle.dot * nozzle.dot);
    let segments = px.len().saturating_sub(1);
    let mut owed = 0.0;
    for k in 0..segments.div_ceil(SPRAY_RUN).max(1) {
        let mut path = Path::new();
        // The first point gets a disc's worth, the burst of the nozzle opening.
        if k == 0 {
            let first = px[0];
            for n in 0..(BURST * nozzle.radius) as u64 {
                speck(&mut path, first, &nozzle, (u64::MAX, n));
            }
        }
        for i in (k * SPRAY_RUN).min(segments)..((k + 1) * SPRAY_RUN).min(segments) {
            let (a, b) = (px[i], px[i + 1]);
            owed += (b.x - a.x).hypot(b.y - a.y) * per_pixel;
            let count = owed.floor();
            owed -= count;
            for n in 0..count as u64 {
                // Spread along the segment, so a fast stroke doesn't spray in clumps.
                let t = (n as f32 + noise(i as u64, n * 4)) / count;
                speck(&mut path, Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), &nozzle, (i as u64, n));
            }
        }
        if !path.segs().is_empty() {
            ops.push(Op::Path { path, fill: Some(color.into()), rule: Rule::NonZero, stroke: None });
        }
    }
}

/// How a spray comes out: over a disc of `radius`, in dots about `dot` across.
struct Nozzle {
    radius: f32,
    dot: f32,
}

/// Adds the `n`th dot sprayed about `centre` from step `seed` to `path`: somewhere on the
/// nozzle's disc, nearer the middle more often, and about its dot's size.
fn speck(path: &mut Path, centre: Point, &Nozzle { radius, dot }: &Nozzle, (seed, n): (u64, u64)) {
    let angle = TAU * noise(seed, n * 4 + 1);
    let reach = radius * noise(seed, n * 4 + 2).powf(SPRAY_FALLOFF);
    // A dot is a square: at a pixel or two across, smoothing makes it look round, and it's the
    // fewest edges to rasterize, with thousands of dots in a stroke.
    let half = dot * (0.6 + 0.8 * noise(seed, n * 4 + 3)) / 2.0;
    let (x, y) = (centre.x + reach * angle.cos(), centre.y + reach * angle.sin());
    path.move_to(Point::new(x - half, y - half));
    path.line_to(Point::new(x + half, y - half));
    path.line_to(Point::new(x + half, y + half));
    path.line_to(Point::new(x - half, y + half));
    path.close();
}

/// A number from 0 to 1 that looks random but is always the same for `seed` and `n`.
fn noise(seed: u64, n: u64) -> f32 {
    let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ n.wrapping_add(0x632B_E59B_D9B4_E019).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}
