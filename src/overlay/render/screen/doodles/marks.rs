//! The marks a pen's style makes beyond a plain stroke: a brush is solid through its middle with
//! streaks along both edges, and a spray is dithered over the area it covers.

use std::collections::BTreeSet;

use crate::overlay::render::{Cap, Join, Op, Path, Point, Rgba, Rule, Stroke};

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

/// Draws `px` (logical pixels) `width` wide in `color`: a solid core, with streaks along both edges.
pub fn brush(ops: &mut Vec<Op>, px: &[Point], width: f32, color: Rgba, cap: Cap) {
    let mut core = Path::new();
    trace(&mut core, px);
    let stroke = Stroke { cap, join: Join::Round, ..Stroke::new(width * CORE, color) };
    ops.push(Op::Path { path: core, fill: None, rule: Rule::NonZero, stroke: Some(stroke) });
    let normals = normals(px);
    for (side, phase) in [(-EDGE, 0), (EDGE, 1)] {
        let streak = Streak { offset: side * width, width: width * STREAK, color, phase };
        streak_along(ops, px, &normals, &streak);
    }
}

/// A streak along one edge: how far it stands from the line, how wide it is, its color, and which
/// segment its breaks start at.
struct Streak {
    offset: f32,
    width: f32,
    color: Rgba,
    phase: usize,
}

/// Draws `streak` along the line through `px`, broken into runs. The runs are one path, so a
/// long stroke is one op to draw rather than one for each run.
fn streak_along(ops: &mut Vec<Op>, px: &[Point], normals: &[Point], streak: &Streak) {
    let shifted = |i: usize| Point::new(px[i].x + normals[i].x * streak.offset, px[i].y + normals[i].y * streak.offset);
    let mut path = Path::new();
    let (mut running, mut any) = (false, false);
    for i in 0..px.len().saturating_sub(1) {
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
pub fn trace(path: &mut Path, px: &[Point]) {
    let Some(&first) = px.first() else { return };
    path.move_to(first);
    for &point in px {
        path.line_to(point);
    }
}

/// Dithers a spray `width` wide over `px`: each cell of a grid the width's fifth across gets a dot
/// where the spray is dense enough there, in a fixed pattern, so the marks read as grain.
pub fn spray(ops: &mut Vec<Op>, px: &[Point], width: f32, color: Rgba) {
    let radius = width / 2.0;
    let cell = (width / 5.0).max(2.0);
    // Sorted, so the same doodle draws the same ops each frame.
    let mut dots = BTreeSet::new();
    for p in px {
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
                if density > threshold {
                    dots.insert((gx, gy));
                }
            }
        }
    }
    if dots.is_empty() {
        return;
    }
    // The dots are one path, so a spray is one op to draw however many dots it has.
    let mut path = Path::new();
    for (gx, gy) in dots {
        let (x, y, side) = (gx as f32 * cell, gy as f32 * cell, cell * DOT);
        path.move_to(Point::new(x, y));
        path.line_to(Point::new(x + side, y));
        path.line_to(Point::new(x + side, y + side));
        path.line_to(Point::new(x, y + side));
        path.close();
    }
    ops.push(Op::Path { path, fill: Some(color.into()), rule: Rule::NonZero, stroke: None });
}
