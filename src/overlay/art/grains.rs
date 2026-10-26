//! The translucent and noisy filters: grain, scanlines, pixel blocks and camo, and a flat wash.

use super::{Pattern, blotch, hash};
use crate::config::{Camo, CamoScheme, Grain, Paint, Pixels, Scanlines, Tint};

/// Noise in blocks `block` pixels across. Given a clock, it picks one of a few patterns by it,
/// a dozen times a second.
pub fn grain(g: &Grain, s: f32, clock: Option<f32>) -> Pattern {
    let block = (g.size.clamp(1.0, 6.0) * s).round().max(1.0) as usize;
    let frame = clock.map_or(0, |clock| (clock * 12.0) as u32 % 4);
    Pattern::new([10, block as u32, frame], g.opacity, move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let v = (hash((x / block) as i64, (y / block) as i64, 20 + frame) - 0.5) * 2.0;
                let shade = if v > 0.0 { 255 } else { 0 };
                pixels.extend_from_slice(&[shade, shade, shade, (v.abs() * 255.0) as u8]);
            }
        }
        pixels
    })
}

/// How far a scanline drifts per second, in pixels at scale 1.
const DRIFT_SPEED: f32 = 14.0;

/// Thin lines across, `spacing` apart. Given a clock, they drift down by it, a pixel at a time.
pub fn scanlines(sc: &Scanlines, s: f32, clock: Option<f32>) -> Pattern {
    let gap = (sc.spacing.max(2.0) * s).round() as usize;
    let thickness = (gap as f32 * 0.35).round().max(1.0) as usize;
    let [r, g, b] = Paint::new(&sc.color, 1.0).rgb().unwrap_or([0, 0, 0]);
    let shift = clock.map_or(0, |clock| (clock * DRIFT_SPEED * s) as usize % gap);
    let color = u32::from_be_bytes([0, r, g, b]);
    Pattern::new([16 | (shift as u32) << 8, gap as u32, color], sc.opacity, move |_, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            let on = (y + gap - shift) % gap < thickness;
            for _ in 0..w {
                pixels.extend_from_slice(&[r, g, b, if on { 255 } else { 0 }]);
            }
        }
        pixels
    })
}

/// How many steps a shimmer takes a second, and how many it takes to come round again: each
/// block goes up and down a whole number of times in that loop, so it repeats without a jump.
const SHIMMER_RATE: f32 = 12.0;
const SHIMMER_LOOP: u32 = 96;

/// Blocks `block` pixels across, each lightened or darkened at random, with a dark seam along
/// two of its edges. Given a clock, each block brightens and dims by it, at its own pace and
/// out of step with its neighbours.
pub fn pixels(p: &Pixels, s: f32, clock: Option<f32>) -> Pattern {
    let block = (p.size.clamp(2.0, 24.0) * s).round().max(2.0) as usize;
    let frame = clock.map(|clock| (clock * SHIMMER_RATE) as u32 % SHIMMER_LOOP);
    Pattern::new([11, block as u32, frame.map_or(0, |f| f + 1)], p.opacity, move |x0, y0, w, h| {
        let shade_of = |bx: usize, by: usize| {
            let (bx, by) = (bx as i64, by as i64);
            let mut v = (hash(bx, by, 30) - 0.5) * 1.2;
            if let Some(frame) = frame {
                let turns = 1.0 + (hash(bx, by, 31) * 3.0).floor().min(2.0);
                let phase = hash(bx, by, 32) + turns * frame as f32 / SHIMMER_LOOP as f32;
                v = (v + 0.45 * (phase * std::f32::consts::TAU).sin()).clamp(-1.0, 1.0);
            }
            let shade = if v > 0.0 { 255 } else { 0 };
            [shade, shade, shade, (v.abs() * 255.0) as u8]
        };
        let mut pixels = Vec::with_capacity(w * h * 4);
        let mut shades = Vec::new();
        for y in y0..y0 + h {
            // Each block's shade once a row of blocks, not once a pixel.
            if y == y0 || y % block == 0 {
                shades = (x0 / block..=(x0 + w) / block).map(|bx| shade_of(bx, y / block)).collect();
            }
            for x in x0..x0 + w {
                if x % block == block - 1 || y % block == block - 1 {
                    pixels.extend_from_slice(&[0, 0, 0, 200]);
                } else {
                    pixels.extend_from_slice(&shades[x / block - x0 / block]);
                }
            }
        }
        pixels
    })
}

/// Blotches of three colors on a base, `size` pixels across: each color shows where its own
/// noise runs high, the later ones over the earlier.
pub fn camo(c: &Camo, s: f32) -> Pattern {
    let cell = (c.size.clamp(16.0, 200.0) * s).max(8.0);
    let scheme = c.scheme;
    let [base, a, b, dark] = match scheme {
        CamoScheme::Woodland => [[0x5a, 0x6b, 0x3a], [0x3e, 0x4c, 0x2a], [0x7a, 0x62, 0x3c], [0x22, 0x2a, 0x1a]],
        CamoScheme::Desert => [[0xc2, 0xa8, 0x78], [0xa8, 0x8c, 0x5e], [0xd8, 0xc4, 0x98], [0x7a, 0x62, 0x40]],
        CamoScheme::Urban => [[0x80, 0x84, 0x88], [0x5c, 0x60, 0x66], [0xa4, 0xa8, 0xac], [0x2e, 0x30, 0x34]],
        CamoScheme::Arctic => [[0xe8, 0xec, 0xf0], [0xc4, 0xd0, 0xdc], [0xf8, 0xf8, 0xfa], [0x98, 0xa8, 0xb8]],
    };
    Pattern::new([12, cell as u32, scheme as u32], c.opacity, move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                // Two octaves, so the blotches have ragged edges.
                let noise = |seed: u32| blotch(x, y, cell, seed) * 0.75 + blotch(x, y, cell / 3.0, seed + 100) * 0.25;
                let color = if noise(41) > 0.62 {
                    dark
                } else if noise(42) > 0.56 {
                    b
                } else if noise(43) > 0.5 {
                    a
                } else {
                    base
                };
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// A flat wash of the tint's color as a picture, for a shape a box's rounded corners can't draw.
pub fn wash(t: &Tint) -> Pattern {
    let [r, g, b] = Paint::new(&t.color, 1.0).rgb().unwrap_or([0xff, 0xa0, 0x40]);
    Pattern::new([19, 0, u32::from_be_bytes([0, r, g, b])], t.opacity, move |_, _, w, h| [r, g, b, 255].repeat(w * h))
}
