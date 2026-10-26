//! The reliefs of the materials a panel can lie on: heights from 0 to 1, 0.5 flat.

use super::{blotch, hash, smooth};

/// Fine grain, soft blotches, and short pale or dark fibres lying at slants.
pub(super) fn paper(x: usize, y: usize) -> f32 {
    let grain = hash(x as i64, y as i64, 1) - 0.5;
    let soft = blotch(x, y, 48.0, 2) - 0.5;
    let mottle = blotch(x, y, 7.0, 3) - 0.5;
    // Fibres: each 16-pixel cell may hold one, a short line across it at its own slant.
    let (cx, cy) = ((x / 16) as i64, (y / 16) as i64);
    let fibre = if hash(cx, cy, 4) > 0.55 {
        let slant = hash(cx, cy, 5) * 2.0 - 1.0;
        let (lx, ly) = ((x % 16) as f32 - 8.0, (y % 16) as f32 - 8.0);
        let off = (ly - lx * slant).abs();
        if off < 0.7 && lx.abs() < 7.0 { (hash(cx, cy, 6) - 0.5) * 0.9 } else { 0.0 }
    } else {
        0.0
    };
    0.5 + grain * 0.28 + soft * 0.22 + mottle * 0.18 + fibre
}

/// Streaks along the panel (noise smeared sideways, each row its own strength) under a soft
/// diagonal sheen.
pub(super) fn brushed_metal(x: usize, y: usize) -> f32 {
    const SMEAR: i64 = 18;
    let mut sum = 0.0;
    for dx in -SMEAR..=SMEAR {
        sum += hash(x as i64 + dx, y as i64, 7) - 0.5;
    }
    let streak = sum / (SMEAR as f32 * 0.7);
    let row = hash(0, y as i64, 8) - 0.5;
    let fine = hash(x as i64, y as i64, 9) - 0.5;
    let sheen = ((x as f32 / 260.0 + y as f32 / 900.0) * std::f32::consts::PI).sin() * 0.5;
    0.5 + streak * 0.30 + row * 0.12 + fine * 0.05 + sheen * 0.22
}

/// Poured concrete: broad stains, a sandy grain, scattered pits (small dark pockets with a
/// pale lip) and, every 320 by 180 pixels, a seam where the shuttering met, with a hairline
/// crack wandering down some of the vertical ones.
pub(super) fn concrete(x: usize, y: usize) -> f32 {
    let stain = blotch(x, y, 140.0, 10) - 0.5;
    let cloud = blotch(x, y, 34.0, 11) - 0.5;
    let sand = hash(x as i64, y as i64, 12) - 0.5;
    let fine = blotch(x, y, 3.0, 13) - 0.5;
    // Pits: a pocket may sit in each 9-pixel cell, dark in the middle with a lighter rim.
    let (cx, cy) = ((x / 9) as i64, (y / 9) as i64);
    let pit = if hash(cx, cy, 14) > 0.93 {
        let (dx, dy) = ((x % 9) as f32 - 3.0 - hash(cx, cy, 15) * 3.0, (y % 9) as f32 - 3.0 - hash(cx, cy, 16) * 3.0);
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1.8 { -0.55 } else if d < 2.8 { 0.2 } else { 0.0 }
    } else {
        0.0
    };
    // Seams: a dark line with a pale edge beside it.
    let seam = |at: usize, size: usize| -> f32 {
        match at % size {
            0 => -0.45,
            1 => 0.18,
            r if r == size - 1 => 0.12,
            _ => 0.0,
        }
    };
    let panel_cols = x / 320;
    let crack = if hash(panel_cols as i64, 17, 17) > 0.45 {
        // A crack down the seam's left, its x wandering with y.
        let wander = ((blotch(0, y, 40.0, 18) - 0.5) * 22.0) as i64;
        let line = (panel_cols * 320) as i64 + 160 + wander;
        if (x as i64 - line).abs() == 0 && hash(y as i64 / 90, panel_cols as i64, 19) > 0.4 { -0.4 } else { 0.0 }
    } else {
        0.0
    };
    0.5 + stain * 0.30 + cloud * 0.22 + sand * 0.26 + fine * 0.14 + pit + seam(x, 320) + seam(y, 180) + crack
}

/// Plain-weave cloth, like linen: threads 3 pixels wide, each going over one and under the next.
/// Each thread is its own shade and thickens and thins along its length, so the threads show as
/// uneven lines rather than a grid of dots.
pub(super) fn cloth(x: usize, y: usize) -> f32 {
    const PITCH: usize = 3;
    let (i, j) = (x / PITCH, y / PITCH);
    let (u, v) = (((x % PITCH) as f32 + 0.5) / PITCH as f32, ((y % PITCH) as f32 + 0.5) / PITCH as f32);
    let (across, along, thread, slub) = if (i + j) % 2 == 0 {
        (u, v, hash(i as i64, 0, 20), smooth(i as f32, y as f32 / 18.0, 22))
    } else {
        (v, u, hash(0, j as i64, 21), smooth(x as f32 / 18.0, j as f32, 23))
    };
    weave(x, y, &Thread { across, along, shade: thread, slub }, 0.3)
}

/// Twill, like denim: threads 4 pixels wide, each warp thread going over two weft threads and
/// under the next, a step further along on each row, so the floats run in diagonal ribs.
pub(super) fn denim(x: usize, y: usize) -> f32 {
    const PITCH: usize = 4;
    let (i, j) = (x / PITCH, y / PITCH);
    let (u, v) = (((x % PITCH) as f32 + 0.5) / PITCH as f32, ((y % PITCH) as f32 + 0.5) / PITCH as f32);
    let step = (i + j) % 3;
    let (across, along, thread, slub) = if step == 0 {
        (v, u, hash(0, j as i64, 26), smooth(x as f32 / 30.0, j as f32, 27))
    } else {
        // Which of its float's two cells this is.
        let along = ((step - 1) as f32 + v) / 2.0;
        (u, along, hash(i as i64, 0, 28), smooth(i as f32, y as f32 / 30.0, 29))
    };
    weave(x, y, &Thread { across, along, shade: thread, slub }, 0.45)
}

/// Silk: smooth drapery with almost no grain, its folds lit from the top left, dim on the side
/// turned away and with a narrow sheen along the side turned toward the light.
pub(super) fn silk(x: usize, y: usize) -> f32 {
    let (fx, fy) = (x as f32, y as f32);
    // How steeply the cloth faces the light, -1 to 1.
    let facing = ((drape(fx - 3.0, fy - 3.0) - drape(fx + 3.0, fy + 3.0)) * 7.0).clamp(-1.0, 1.0);
    let sheen = (-((facing - 0.55) * 3.0).powi(2)).exp();
    let fine = hash(x as i64, y as i64, 50) - 0.5;
    0.5 + facing * 0.2 + sheen * 0.22 + fine * 0.02
}

/// How high draped silk stands at a point: two sets of folds, one hanging mostly downward and
/// a broader one across it, both bent by slow noise so they flow rather than run straight.
fn drape(x: f32, y: f32) -> f32 {
    let warp = (smooth(x / 480.0, y / 480.0, 51) - 0.5) * 1.5 + (smooth(x / 200.0, y / 200.0, 52) - 0.5) * 0.25;
    let hanging = ((x * 0.9 + y * 0.3) / 280.0 + warp) * std::f32::consts::TAU;
    let across = ((y * 0.85 - x * 0.35) / 560.0 + warp * 0.6) * std::f32::consts::TAU;
    hanging.sin() * 0.7 + across.sin() * 0.3
}

/// Where a point lies on the thread on top of a weave there, each 0 to 1.
struct Thread {
    /// Across the thread.
    across: f32,
    /// Along its float, from where it comes up to where it goes under.
    along: f32,
    /// The thread's own shade.
    shade: f32,
    /// How thick the thread is here.
    slub: f32,
}

/// The top thread at a point of a weave: rounded across it and dipping at each end of its
/// float, `lift` strong, in its own shade and thickness, with a fuzz over it all and soft folds
/// under it.
fn weave(x: usize, y: usize, t: &Thread, lift: f32) -> f32 {
    let round = (t.across * std::f32::consts::PI).sin() * (t.along * std::f32::consts::PI).sin().sqrt();
    let fuzz = hash(x as i64, y as i64, 24) - 0.5;
    let fold = blotch(x, y, 120.0, 25) - 0.5;
    0.5 + (round - 0.6) * lift + (t.shade - 0.5) * 0.22 + (t.slub - 0.5) * 0.3 + fuzz * 0.12 + fold * 0.2
}

/// Pebbled leather: a pebble round each of a jittered grid of points 5 pixels apart, lit from the
/// top left, with dark creases where pebbles meet, a few long wrinkles and a soft mottle.
pub(super) fn leather(x: usize, y: usize) -> f32 {
    const CELL: f32 = 5.0;
    let (fx, fy) = (x as f32, y as f32);
    let (ci, cj) = ((fx / CELL).floor() as i64, (fy / CELL).floor() as i64);
    // The nearest two points, and which way the nearest lies.
    let (mut first, mut second, mut toward) = (f32::MAX, f32::MAX, 0.0);
    for (di, dj) in (-1..=1).flat_map(|di| (-1..=1).map(move |dj| (di, dj))) {
        let (i, j) = (ci + di, cj + dj);
        let (px, py) = ((i as f32 + hash(i, j, 40)) * CELL, (j as f32 + hash(i, j, 41)) * CELL);
        let d = ((px - fx).powi(2) + (py - fy).powi(2)).sqrt();
        if d < first {
            (second, first, toward) = (first, d, (px - fx) + (py - fy));
        } else if d < second {
            second = d;
        }
    }
    let crease = (1.0 - (second - first) / 1.2).max(0.0).powi(2);
    let lit = (toward / CELL).clamp(-1.0, 1.0);
    let wrinkle = 1.0 - ((smooth(fx / 160.0, fy / 50.0, 42) - 0.5).abs() * 60.0).min(1.0);
    let mottle = blotch(x, y, 80.0, 43) - 0.5;
    0.5 + lit * 0.1 - crease * 0.22 - wrinkle * 0.2 + mottle * 0.25
}

/// Steel tread plate: pointed lugs every 24 pixels, each at right angles to its neighbours, lit
/// from the top left, on steel with faint streaks and mottling.
pub(super) fn steel(x: usize, y: usize) -> f32 {
    let (fx, fy) = (x as f32, y as f32);
    let lit = lug(fx - 1.0, fy - 1.0) - lug(fx + 1.0, fy + 1.0);
    let top = lug(fx, fy);
    let streak = smooth(fx / 40.0, fy / 1.5, 26) - 0.5;
    let mottle = blotch(x, y, 60.0, 27) - 0.5;
    let fine = hash(x as i64, y as i64, 28) - 0.5;
    0.5 + lit * 0.4 + top * 0.06 + streak * 0.14 + mottle * 0.2 + fine * 0.08
}

/// How far a tread plate's lug stands up at a point, 0 to 1: the nearest lug, a lens 18 pixels
/// long lying at 45 degrees, one way or the other by its place in the grid.
fn lug(x: f32, y: f32) -> f32 {
    const SPACING: f32 = 24.0;
    const HALF_LENGTH: f32 = 9.0;
    const HALF_WIDTH: f32 = 3.0;
    let (i, j) = ((x / SPACING).round(), (y / SPACING).round());
    let (dx, dy) = ((x - i * SPACING) * std::f32::consts::FRAC_1_SQRT_2, (y - j * SPACING) * std::f32::consts::FRAC_1_SQRT_2);
    let (along, across) = if (i + j).rem_euclid(2.0) == 0.0 { (dx + dy, dx - dy) } else { (dx - dy, dx + dy) };
    let t = along / HALF_LENGTH;
    let width = HALF_WIDTH * (1.0 - t * t);
    if t.abs() < 1.0 && across.abs() < width { (1.0 - (across / width).powi(2)).sqrt() } else { 0.0 }
}

/// Planks 90 pixels high, laid end to end with staggered joints. Each is its own shade, its
/// grain lines waving along it and bending round the odd knot, over fine fibres and pores.
pub(super) fn wood(x: usize, y: usize) -> f32 {
    const PLANK: usize = 90;
    const LENGTH: usize = 420;
    let row = (y / PLANK) as i64;
    let stagger = (hash(row, 0, 30) * LENGTH as f32) as usize;
    let board = ((x + stagger) / LENGTH) as i64;
    let shade = hash(row, board, 31) - 0.5;
    let (fx, fy) = (x as f32, (y % PLANK) as f32);
    // A knot on about half the boards, somewhere along the middle of it.
    let start = (board * LENGTH as i64 - stagger as i64) as f32;
    let (kx, ky) = (start + (0.2 + hash(row, board, 32) * 0.6) * LENGTH as f32, 20.0 + hash(row, board, 33) * 50.0);
    let knotted = hash(row, board, 34) > 0.5;
    let reach = (((fx - kx) * 0.3).powi(2) + (fy - ky).powi(2)).sqrt();
    let bend = if knotted { 16.0 * (-(reach / 12.0).powi(2)).exp() * (fy - ky).signum() } else { 0.0 };
    let wave = (smooth(fx / 160.0, fy / 40.0 + shade * 10.0, 35) - 0.5) * 30.0;
    let grain = ((fy + wave + bend + shade * 100.0) / 7.0 * std::f32::consts::TAU).sin();
    let line = grain.max(0.0).powi(4);
    let knot = if knotted && reach < 6.0 { -0.35 + (reach * 1.6).sin() * 0.1 } else { 0.0 };
    let fibre = smooth(fx / 25.0, fy / 1.2, 36) - 0.5;
    let pore = hash(x as i64, y as i64, 37) - 0.5;
    // Seams: a dark line along each plank's top and at each joint, with a pale edge after it.
    let seam = |at: usize, size: usize| match at % size {
        0 => -0.5,
        1 => 0.12,
        _ => 0.0,
    };
    0.5 + shade * 0.25 - line * 0.28 + grain * 0.04 + fibre * 0.25 + pore * 0.08 + knot + seam(y, PLANK) + seam(x + stagger, LENGTH)
}
