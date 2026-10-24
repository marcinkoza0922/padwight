//! The procedural pictures that texture a panel or an item: paper, brushed metal, concrete,
//! cloth, denim, silk, leather, steel tread plate and wood, scanlines, grain, pixel blocks and
//! camo. The materials, grain and blocks are translucent white and black, so they lighten and
//! darken whatever they sit on.

use std::sync::OnceLock;

use iced::Element;

use crate::config::{Camo, CamoScheme, Grain, Paint, Pixels, Scanlines, Texture};

mod anchored;
mod materials;
mod patterns;

pub use anchored::anchor;
use anchored::sized;
use materials::{brushed_metal, cloth, concrete, denim, leather, paper, silk, steel, wood};
pub use patterns::{argyle, plaid, stripes};

/// What a texture looks like.
#[derive(Debug, Clone, Copy)]
pub enum Material {
    /// Fine grain, soft blotches and a few fibres.
    Paper,
    /// Fine streaks along the panel, with a soft sheen.
    BrushedMetal,
    /// Coarse mottling, shuttering seams, pits and the odd hairline crack.
    Concrete,
    /// Uneven threads woven over and under each other, like linen.
    Cloth,
    /// Twill: threads woven in diagonal ribs.
    Denim,
    /// Smooth drapery: flowing folds with a soft sheen, and next to no grain.
    Silk,
    /// Pebbled grain with creases between the pebbles and the odd wrinkle.
    Leather,
    /// Tread plate: raised lugs in a herringbone, on a mottled steel.
    Steel,
    /// Planks with wavy grain, fibres and the odd knot.
    Wood,
}

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

/// The material as a layer the size of what it covers: the relief, repeated if that's bigger.
pub fn layer<'a, M: 'a>(kind: Material, t: &Texture, round: f32, fade: f32) -> Element<'a, M> {
    sized([kind as u32, 0, 0], round, (t.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let source = reliefs(kind);
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            let row = (y % HEIGHT) * WIDTH * 4;
            for x in x0..x0 + w {
                let at = row + (x % WIDTH) * 4;
                pixels.extend_from_slice(&source[at..at + 4]);
            }
        }
        pixels
    })
}

/// Noise in blocks `block` pixels across. Given a clock, it picks one of a few patterns by it,
/// a dozen times a second.
pub fn grain<'a, M: 'a>(g: &Grain, s: f32, clock: Option<f32>, round: f32, fade: f32) -> Element<'a, M> {
    let block = (g.size.clamp(1.0, 6.0) * s).round().max(1.0) as usize;
    let frame = clock.map_or(0, |clock| (clock * 12.0) as u32 % 4);
    sized([10, block as u32, frame], round, (g.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
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
pub fn scanlines<'a, M: 'a>(sc: &Scanlines, s: f32, clock: Option<f32>, round: f32, fade: f32) -> Element<'a, M> {
    let gap = (sc.spacing.max(2.0) * s).round() as usize;
    let thickness = (gap as f32 * 0.35).round().max(1.0) as usize;
    let [r, g, b] = Paint::new(&sc.color, 1.0).rgb().unwrap_or([0, 0, 0]);
    let shift = clock.map_or(0, |clock| (clock * DRIFT_SPEED * s) as usize % gap);
    let color = u32::from_be_bytes([0, r, g, b]);
    sized([16 | (shift as u32) << 8, gap as u32, color], round, (sc.opacity * fade).clamp(0.0, 1.0), move |_, y0, w, h| {
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

/// Blocks `block` pixels across, each lightened or darkened at random, with a dark seam along
/// two of its edges.
pub fn pixels<'a, M: 'a>(p: &Pixels, s: f32, round: f32, fade: f32) -> Element<'a, M> {
    let block = (p.size.clamp(2.0, 24.0) * s).round().max(2.0) as usize;
    sized([11, block as u32, 0], round, (p.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
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
pub fn camo<'a, M: 'a>(c: &Camo, s: f32, round: f32, fade: f32) -> Element<'a, M> {
    let cell = (c.size.clamp(16.0, 200.0) * s).max(8.0);
    let scheme = c.scheme;
    let [base, a, b, dark] = match scheme {
        CamoScheme::Woodland => [[0x5a, 0x6b, 0x3a], [0x3e, 0x4c, 0x2a], [0x7a, 0x62, 0x3c], [0x22, 0x2a, 0x1a]],
        CamoScheme::Desert => [[0xc2, 0xa8, 0x78], [0xa8, 0x8c, 0x5e], [0xd8, 0xc4, 0x98], [0x7a, 0x62, 0x40]],
        CamoScheme::Urban => [[0x80, 0x84, 0x88], [0x5c, 0x60, 0x66], [0xa4, 0xa8, 0xac], [0x2e, 0x30, 0x34]],
        CamoScheme::Arctic => [[0xe8, 0xec, 0xf0], [0xc4, 0xd0, 0xdc], [0xf8, 0xf8, 0xfa], [0x98, 0xa8, 0xb8]],
    };
    sized([12, cell as u32, scheme as u32], round, (c.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
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

/// The relief of each kind, as RGBA pixels, made on first use.
fn reliefs(kind: Material) -> &'static [u8] {
    static PAPER: OnceLock<Vec<u8>> = OnceLock::new();
    static METAL: OnceLock<Vec<u8>> = OnceLock::new();
    static CONCRETE: OnceLock<Vec<u8>> = OnceLock::new();
    static CLOTH: OnceLock<Vec<u8>> = OnceLock::new();
    static DENIM: OnceLock<Vec<u8>> = OnceLock::new();
    static SILK: OnceLock<Vec<u8>> = OnceLock::new();
    static LEATHER: OnceLock<Vec<u8>> = OnceLock::new();
    static STEEL: OnceLock<Vec<u8>> = OnceLock::new();
    static WOOD: OnceLock<Vec<u8>> = OnceLock::new();
    match kind {
        Material::Paper => PAPER.get_or_init(|| relief(paper)),
        Material::BrushedMetal => METAL.get_or_init(|| relief(brushed_metal)),
        Material::Concrete => CONCRETE.get_or_init(|| relief(concrete)),
        Material::Cloth => CLOTH.get_or_init(|| relief(cloth)),
        Material::Denim => DENIM.get_or_init(|| relief(denim)),
        Material::Silk => SILK.get_or_init(|| relief(silk)),
        Material::Leather => LEATHER.get_or_init(|| relief(leather)),
        Material::Steel => STEEL.get_or_init(|| relief(steel)),
        Material::Wood => WOOD.get_or_init(|| relief(wood)),
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
    smooth(x as f32 / cell, y as f32 / cell, seed)
}

/// Smooth noise at a point, its features one unit across.
fn smooth(fx: f32, fy: f32, seed: u32) -> f32 {
    let (ix, iy) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - ix, fy - iy);
    let ease = |t: f32| t * t * (3.0 - 2.0 * t);
    let at = |dx: i64, dy: i64| hash(ix as i64 + dx, iy as i64 + dy, seed);
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * ease(tx);
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * ease(tx);
    top + (bottom - top) * ease(ty)
}
