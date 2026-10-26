//! The procedural pictures behind a panel's filters, made without any drawing library: each
//! generator returns a `Pattern`, which any renderer can turn into pixels.

use std::sync::OnceLock;

use crate::config::Texture;

mod grains;
mod materials;
mod patterns;
mod shape;

pub use grains::{camo, grain, pixels, scanlines, wash};
use materials::{brushed_metal, cloth, concrete, denim, leather, paper, silk, steel, wood};
pub use patterns::{argyle, checkerboard, plaid, polka_dots, stripes};
pub use shape::Shape;

/// Makes the RGBA pixels of the `w` by `h` part of a picture whose top left corner is `x`, `y`
/// pixels in: `build(x, y, w, h)`.
pub type Build = Box<dyn Fn(usize, usize, usize, usize) -> Vec<u8> + Send + Sync>;

/// A filter's picture, which spans the panel it lies on.
pub struct Pattern {
    /// Names the picture, so a caller can keep what it built.
    pub key: [u32; 3],
    /// How strongly it shows, before any fade.
    pub opacity: f32,
    pub build: Build,
}

impl Pattern {
    fn new(key: [u32; 3], opacity: f32, build: impl Fn(usize, usize, usize, usize) -> Vec<u8> + Send + Sync + 'static) -> Self {
        Self { key, opacity, build: Box::new(build) }
    }
}

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

/// The material as a picture: the relief, repeated if what it covers is bigger.
pub fn material(kind: Material, t: &Texture) -> Pattern {
    Pattern::new([kind as u32, 0, 0], t.opacity, move |x0, y0, w, h| {
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
