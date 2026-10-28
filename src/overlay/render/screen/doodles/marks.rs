//! The marks a pen makes: a solid line, a brush (solid through its middle with streaks along both
//! edges), and a spray dithered over the area it covers.
//!
//! A stroke is drawn in runs of `RUN` segments, an op each. As a stroke grows only its last run
//! or two change, so the screen repaints only where they lie, and every other run is laid from
//! the coverage the rasterizer kept for it. One op for the whole stroke changed with each point
//! the stick added: its whole box was repainted, and its whole shape rasterized again, each frame.

use std::{
    collections::{BTreeSet, HashSet},
    ops::Range,
};

use crate::overlay::render::{Cap, Join, Op, Path, Point, Rgba, Rule, Stroke};

/// How many segments of a stroke each of its ops draws.
const RUN: usize = 32;

/// The brush's solid core, as a share of the stroke's width, and where its streaks run (the
/// streaks' centers, as shares of the width from the middle), how wide they are, and how often
/// they break. A break is every `GAP_EVERY` segments, so the streaks read as bristle marks.
const CORE: f32 = 0.6;
const EDGE: f32 = 0.52;
const STREAK: f32 = 0.2;
const GAP_EVERY: usize = 4;

/// The dither: a 4 by 4 ordered matrix of thresholds, and how dense a spray is at its middle.
const BAYER: [[f32; 4]; 4] = [[0.0, 8.0, 2.0, 10.0], [12.0, 4.0, 14.0, 6.0], [3.0, 11.0, 1.0, 9.0], [15.0, 7.0, 13.0, 5.0]];
const DENSITY: f32 = 0.9;
/// Each dot's side as a share of the grid cell it sits in, so the grain shows.
const DOT: f32 = 0.75;

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

/// Draws `px` (logical pixels) `width` wide in `color`: a solid core, with streaks along both edges.
pub fn brush(ops: &mut Vec<Op>, px: &[Point], width: f32, color: Rgba, cap: Cap) {
    let normals = normals(px);
    let segments = px.len().saturating_sub(1);
    let streaks = [(-EDGE, 0), (EDGE, 1)].map(|(side, phase)| Streak { offset: side * width, width: width * STREAK, color, phase });
    line(ops, px, (width * CORE, color, cap), |ops, k| {
        for streak in &streaks {
            streak_along(ops, px, &normals, streak, (k * RUN).min(segments)..((k + 1) * RUN).min(segments));
        }
    });
}

/// A streak along one edge: how far it stands from the line, how wide it is, its color, and which
/// segment its breaks start at.
struct Streak {
    offset: f32,
    width: f32,
    color: Rgba,
    phase: usize,
}

/// Draws `streak` along the `segments` of the line through `px`, broken into runs, as one path.
/// A run crossing from one op's segments to the next is two lines meeting at a round cap each,
/// which is the line it would be whole.
fn streak_along(ops: &mut Vec<Op>, px: &[Point], normals: &[Point], streak: &Streak, segments: Range<usize>) {
    let shifted = |i: usize| Point::new(px[i].x + normals[i].x * streak.offset, px[i].y + normals[i].y * streak.offset);
    let mut path = Path::new();
    let (mut running, mut any) = (false, false);
    for i in segments {
        if (i + streak.phase).is_multiple_of(GAP_EVERY) {
            running = false;
            continue;
        }
        if !running {
            path.move_to(shifted(i));
            (running, any) = (true, true);
        }
        path.line_to(shifted(i + 1));
    }
    if !any {
        return;
    }
    let stroke = Stroke { cap: Cap::Round, join: Join::Round, ..Stroke::new(streak.width, streak.color) };
    ops.push(Op::Path { path, fill: None, rule: Rule::NonZero, stroke: Some(stroke) });
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

/// Dithers a spray `width` wide over `px`: each cell of a grid the width's fifth across gets a dot
/// where the spray is dense enough there, in a fixed pattern, so the marks read as grain. Each run
/// of `RUN` points is an op of the dots it adds, leaving out those an earlier run has.
pub fn spray(ops: &mut Vec<Op>, px: &[Point], width: f32, color: Rgba) {
    let radius = width / 2.0;
    let cell = (width / 5.0).max(2.0);
    let mut seen = HashSet::new();
    for run in px.chunks(RUN) {
        // Sorted, so the same doodle draws the same ops each frame.
        let mut dots = BTreeSet::new();
        for p in run {
            let (x0, x1) = (((p.x - radius) / cell).floor() as i32, ((p.x + radius) / cell).floor() as i32);
            let (y0, y1) = (((p.y - radius) / cell).floor() as i32, ((p.y + radius) / cell).floor() as i32);
            for gy in y0..=y1 {
                for gx in x0..=x1 {
                    let centre = Point::new((gx as f32 + 0.5) * cell, (gy as f32 + 0.5) * cell);
                    let distance = (centre.x - p.x).hypot(centre.y - p.y);
                    if distance > radius {
                        continue;
                    }
                    let density = (1.0 - distance / radius) * DENSITY;
                    let threshold = (BAYER[gy.rem_euclid(4) as usize][gx.rem_euclid(4) as usize] + 0.5) / 16.0;
                    if density > threshold && !seen.contains(&(gx, gy)) {
                        dots.insert((gx, gy));
                    }
                }
            }
        }
        if dots.is_empty() {
            continue;
        }
        let mut path = Path::new();
        for &(gx, gy) in &dots {
            let (x, y, side) = (gx as f32 * cell, gy as f32 * cell, cell * DOT);
            path.move_to(Point::new(x, y));
            path.line_to(Point::new(x + side, y));
            path.line_to(Point::new(x + side, y + side));
            path.line_to(Point::new(x, y + side));
            path.close();
        }
        seen.extend(dots);
        ops.push(Op::Path { path, fill: Some(color.into()), rule: Rule::NonZero, stroke: None });
    }
}
