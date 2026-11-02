//! WCAG contrast, the colors a text really drew in, and the color-blindness simulation.

use std::collections::HashMap;

use iced::Color;

use crate::snapshot::Image;

/// WCAG relative luminance of an sRGB color.
pub fn luminance(c: Color) -> f32 {
    let lin = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio, 1 to 21.
pub fn ratio(a: Color, b: Color) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `c` over `under`, as it is drawn.
pub fn over(c: Color, under: Color) -> Color {
    let mix = |a: f32, b: f32| a * c.a + b * (1.0 - c.a);
    Color { r: mix(c.r, under.r), g: mix(c.g, under.g), b: mix(c.b, under.b), a: 1.0 }
}

fn rgb(p: [u8; 4]) -> Color {
    Color::from_rgb8(p[0], p[1], p[2])
}

/// The background and text colors drawn inside `area` (pixels), or `None` when nothing but one
/// color is there. The background is the commonest color; the text is the one furthest from it
/// in luminance (glyph edges blend, so the middle of a stroke is the truest sample).
pub fn drawn_colors(image: &Image, area: [u32; 4]) -> Option<(Color, Color)> {
    let [x0, y0, x1, y1] = area;
    let mut counts: HashMap<[u8; 3], u32> = HashMap::new();
    for y in y0..y1.min(image.height) {
        for x in x0..x1.min(image.width) {
            let p = image.pixel(x, y);
            *counts.entry([p[0], p[1], p[2]]).or_default() += 1;
        }
    }
    let (&bg, _) = counts.iter().max_by_key(|(c, n)| (**n, **c))?;
    let bg = rgb([bg[0], bg[1], bg[2], 255]);
    // A real stroke has a few pixels of its color; one stray pixel is a border or an edge.
    let fg = counts.iter().filter(|(_, n)| **n >= 2).map(|(c, _)| rgb([c[0], c[1], c[2], 255])).max_by(|a, b| ratio(*a, bg).total_cmp(&ratio(*b, bg)))?;
    (ratio(fg, bg) > 1.05).then_some((bg, fg))
}

/// Linear-light deuteranopia (Machado, Oliveira and Fernandes 2009, full severity).
pub fn deuteranope(c: Color) -> Color {
    let lin = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    let enc = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
    };
    let (r, g, b) = (lin(c.r), lin(c.g), lin(c.b));
    Color {
        r: enc(0.367322 * r + 0.860646 * g - 0.227968 * b),
        g: enc(0.280085 * r + 0.672501 * g + 0.047413 * b),
        b: enc(-0.011820 * r + 0.042940 * g + 0.968881 * b),
        a: c.a,
    }
}

/// CIE L*a*b* of an sRGB color (D65).
fn lab(c: Color) -> [f32; 3] {
    let lin = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    let (r, g, b) = (lin(c.r), lin(c.g), lin(c.b));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f32| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// How different two colors look, as CIE76 ΔE (about 2 is the least a person notices; 10 or more
/// reads as two colors at a glance).
pub fn delta_e(a: Color, b: Color) -> f32 {
    let (la, lb) = (lab(a), lab(b));
    ((la[0] - lb[0]).powi(2) + (la[1] - lb[1]).powi(2) + (la[2] - lb[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21_to_1() {
        assert!((ratio(Color::BLACK, Color::WHITE) - 21.0).abs() < 0.01);
        assert!((ratio(Color::WHITE, Color::WHITE) - 1.0).abs() < 0.001);
    }

    #[test]
    fn deuteranopia_merges_red_and_green_not_blue_and_orange() {
        let red = Color::from_rgb8(0xd0, 0x64, 0x64);
        let green = Color::from_rgb8(0x5a, 0x9a, 0x1a);
        let blue = Color::from_rgb8(0x4a, 0x8f, 0xe0);
        let orange = Color::from_rgb8(0xe8, 0x96, 0x2e);
        assert!(delta_e(deuteranope(blue), deuteranope(orange)) > delta_e(deuteranope(red), deuteranope(green)));
    }
}
