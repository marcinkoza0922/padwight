//! Colors: straight (not premultiplied) RGBA in 0 to 1, as iced's `Color` holds them, so the
//! same maths gives the same values.

use std::hash::{Hash, Hasher};

use super::geom::hash_floats;
use crate::config::Paint;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const WHITE: Rgba = Rgba::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Rgba = Rgba::rgb(0.0, 0.0, 0.0);
    pub const TRANSPARENT: Rgba = Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub const fn rgb(r: f32, g: f32, b: f32) -> Rgba {
        Rgba { r, g, b, a: 1.0 }
    }

    /// Bytes for the channels, an alpha already in 0 to 1.
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: f32) -> Rgba {
        Rgba { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a }
    }

    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Rgba {
        Rgba::from_rgba8(r, g, b, 1.0)
    }

    /// This color with its alpha multiplied by `factor`.
    pub fn scale_alpha(self, factor: f32) -> Rgba {
        Rgba { a: self.a * factor, ..self }
    }

    /// The channels as bytes, rounded: red, green, blue, alpha.
    pub fn to_u8(self) -> [u8; 4] {
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [byte(self.r), byte(self.g), byte(self.b), byte(self.a)]
    }
}

impl Hash for Rgba {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.r, self.g, self.b, self.a]);
    }
}

/// `a` to `b`, `t` of the way (clamped to 0 to 1), alpha included.
pub fn blend(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Rgba { r: mix(a.r, b.r), g: mix(a.g, b.g), b: mix(a.b, b.b), a: mix(a.a, b.a) }
}

/// A setting's color and opacity; `fallback` stands in for a color that doesn't parse.
pub fn paint(p: &Paint, fallback: [u8; 3]) -> Rgba {
    let [r, g, b] = p.rgb().unwrap_or(fallback);
    Rgba::from_rgba8(r, g, b, p.opacity.clamp(0.0, 1.0))
}

/// White on dark colors, black on light ones.
pub fn text_on(c: Rgba) -> Rgba {
    let luminance = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    if luminance > 0.6 { Rgba::rgb(0.08, 0.08, 0.1) } else { Rgba::WHITE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_clamps_opacity_and_falls_back() {
        let p = paint(&Paint::new("#102030", 1.7), [1, 2, 3]);
        assert_eq!(p, Rgba::from_rgba8(0x10, 0x20, 0x30, 1.0));
        assert_eq!(paint(&Paint::new("nope", 0.5), [255, 0, 0]), Rgba::from_rgba8(255, 0, 0, 0.5));
        assert_eq!(Rgba::from_rgba8(255, 128, 0, 0.5).to_u8(), [255, 128, 0, 128]);
    }
}
