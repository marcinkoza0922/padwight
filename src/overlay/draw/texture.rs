//! The procedural pictures that texture a panel: paper, brushed metal and concrete behind its contents,
//! and grain and pixel blocks over them. Each is translucent white and black, so it lightens
//! and darkens whatever it sits on.

use std::sync::{Mutex, OnceLock};

use iced::{ContentFit, Element, Length, widget::{image, responsive}};

use crate::config::{Camo, CamoScheme, Grain, Pixels, Texture};

/// What a texture looks like.
#[derive(Debug, Clone, Copy)]
pub enum Material {
    /// Fine grain, soft blotches and a few fibres.
    Paper,
    /// Fine streaks along the panel, with a soft sheen.
    BrushedMetal,
    /// Coarse mottling, shuttering seams, pits and the odd hairline crack.
    Concrete,
}

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

/// A layer the size of the panel, cut at layout from pixels `build` makes for that size.
/// `key` names the picture, so the last few are kept and a panel that holds its size doesn't
/// rebuild its picture every frame.
fn sized<'a, M: 'a>(key: [u32; 3], opacity: f32, build: impl Fn(usize, usize) -> Vec<u8> + 'a) -> Element<'a, M> {
    responsive(move |size| {
        let (w, h) = (size.width.max(1.0) as usize, size.height.max(1.0) as usize);
        let handle = cached((key, w, h), || build(w, h));
        image(handle).content_fit(ContentFit::Fill).width(Length::Fill).height(Length::Fill).opacity(opacity).into()
    })
    .into()
}

type Key = ([u32; 3], usize, usize);

fn cached(key: Key, build: impl FnOnce() -> Vec<u8>) -> image::Handle {
    static RECENT: Mutex<Vec<(Key, image::Handle)>> = Mutex::new(Vec::new());
    let mut recent = RECENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, handle)) = recent.iter().find(|(k, _)| *k == key) {
        return handle.clone();
    }
    let handle = image::Handle::from_rgba(key.1 as u32, key.2 as u32, build());
    if recent.len() >= 16 {
        recent.remove(0);
    }
    recent.push((key, handle.clone()));
    handle
}

/// The material as a layer the size of the panel: the relief, repeated if the panel is bigger.
pub fn layer<'a, M: 'a>(kind: Material, t: &Texture, fade: f32) -> Element<'a, M> {
    sized([kind as u32, 0, 0], (t.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let source = reliefs(kind);
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            let row = (y % HEIGHT) * WIDTH * 4;
            for x in 0..w {
                let at = row + (x % WIDTH) * 4;
                pixels.extend_from_slice(&source[at..at + 4]);
            }
        }
        pixels
    })
}

/// Noise in blocks `block` pixels across. Animated, it picks one of a few patterns by the
/// clock, a dozen times a second.
pub fn grain<'a, M: 'a>(g: &Grain, s: f32, clock: f32, animated: bool, fade: f32) -> Element<'a, M> {
    let block = (g.size.clamp(1.0, 6.0) * s).round().max(1.0) as usize;
    let frame = if animated { (clock * 12.0) as u32 % 4 } else { 0 };
    sized([10, block as u32, frame], (g.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let v = (hash((x / block) as i64, (y / block) as i64, 20 + frame) - 0.5) * 2.0;
                let shade = if v > 0.0 { 255 } else { 0 };
                pixels.extend_from_slice(&[shade, shade, shade, (v.abs() * 255.0) as u8]);
            }
        }
        pixels
    })
}

/// Blocks `block` pixels across, each lightened or darkened at random, with a dark seam along
/// two of its edges.
pub fn pixels<'a, M: 'a>(p: &Pixels, s: f32, fade: f32) -> Element<'a, M> {
    let block = (p.size.clamp(2.0, 24.0) * s).round().max(2.0) as usize;
    sized([11, block as u32, 0], (p.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                if x % block == block - 1 || y % block == block - 1 {
                    pixels.extend_from_slice(&[0, 0, 0, 200]);
                    continue;
                }
                let v = (hash((x / block) as i64, (y / block) as i64, 30) - 0.5) * 1.2;
                let shade = if v > 0.0 { 255 } else { 0 };
                pixels.extend_from_slice(&[shade, shade, shade, (v.abs() * 255.0) as u8]);
            }
        }
        pixels
    })
}

/// Blotches of three colors on a base, `size` pixels across: each color shows where its own
/// noise runs high, the later ones over the earlier.
pub fn camo<'a, M: 'a>(c: &Camo, s: f32, fade: f32) -> Element<'a, M> {
    let cell = (c.size.clamp(16.0, 200.0) * s).max(8.0);
    let scheme = c.scheme;
    let [base, a, b, dark] = match scheme {
        CamoScheme::Woodland => [[0x5a, 0x6b, 0x3a], [0x3e, 0x4c, 0x2a], [0x7a, 0x62, 0x3c], [0x22, 0x2a, 0x1a]],
        CamoScheme::Desert => [[0xc2, 0xa8, 0x78], [0xa8, 0x8c, 0x5e], [0xd8, 0xc4, 0x98], [0x7a, 0x62, 0x40]],
        CamoScheme::Urban => [[0x80, 0x84, 0x88], [0x5c, 0x60, 0x66], [0xa4, 0xa8, 0xac], [0x2e, 0x30, 0x34]],
        CamoScheme::Arctic => [[0xe8, 0xec, 0xf0], [0xc4, 0xd0, 0xdc], [0xf8, 0xf8, 0xfa], [0x98, 0xa8, 0xb8]],
    };
    sized([12, cell as u32, scheme as u32], (c.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
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

/// The relief of each kind, as RGBA pixels, made on first use.
fn reliefs(kind: Material) -> &'static [u8] {
    static PAPER: OnceLock<Vec<u8>> = OnceLock::new();
    static METAL: OnceLock<Vec<u8>> = OnceLock::new();
    static CONCRETE: OnceLock<Vec<u8>> = OnceLock::new();
    match kind {
        Material::Paper => PAPER.get_or_init(|| relief(paper)),
        Material::BrushedMetal => METAL.get_or_init(|| relief(brushed_metal)),
        Material::Concrete => CONCRETE.get_or_init(|| relief(concrete)),
    }
}

/// Pixels from a relief: 0.5 is flat, above it lightens and below it darkens.
fn relief(height: fn(usize, usize) -> f32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let v = (height(x, y) - 0.5) * 2.0;
            let (shade, alpha) = if v > 0.0 { (255, v) } else { (0, -v) };
            pixels.extend_from_slice(&[shade, shade, shade, (alpha.clamp(0.0, 1.0) * 255.0) as u8]);
        }
    }
    pixels
}

/// A hash of a lattice point and a seed, 0 to 1.
fn hash(x: i64, y: i64, seed: u32) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ u64::from(seed).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Smooth noise whose features are `cell` pixels across.
fn blotch(x: usize, y: usize, cell: f32, seed: u32) -> f32 {
    let (fx, fy) = (x as f32 / cell, y as f32 / cell);
    let (ix, iy) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - ix, fy - iy);
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    let at = |dx: i64, dy: i64| hash(ix as i64 + dx, iy as i64 + dy, seed);
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * ease(tx);
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * ease(tx);
    top + (bottom - top) * ease(ty)
}

/// Fine grain, soft blotches, and short pale or dark fibres lying at slants.
fn paper(x: usize, y: usize) -> f32 {
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
fn brushed_metal(x: usize, y: usize) -> f32 {
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
fn concrete(x: usize, y: usize) -> f32 {
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
