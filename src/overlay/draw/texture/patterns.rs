//! The patterned color filters: plaid, argyle, stripes, a checkerboard and polka dots, each
//! solid colors over the panel.

use std::hash::{DefaultHasher, Hash, Hasher};

use iced::Element;

use super::{Shape, sized};
use crate::config::{
    Argyle, Checkerboard, Plaid, PolkaDots, StripeDirection, Stripes, default_argyle_colors, default_checkerboard_colors, default_plaid_colors, default_polka_dot_colors,
    default_stripes_colors, parse_hex,
};

/// A tartan: each thread is the ground, a band or a line by where it lies in the sett, and the
/// twill shows the thread running down at some points and the one running across at the others,
/// so where a band crosses the ground the two hatch together.
pub fn plaid<'a, M: 'a>(p: &Plaid, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    let sett = (p.size.clamp(24.0, 240.0) * s).max(12.0);
    let [ground, band, line] = rgb(&p.colors, default_plaid_colors);
    // A line is about two pixels wide, at its place in the sett.
    let thin = 1.0 / sett;
    let thread = move |at: usize| {
        let t = (at as f32 % sett) / sett;
        if (t - 0.15).abs() < thin || (t - 0.65).abs() < thin {
            line
        } else if t < 0.3 {
            band
        } else {
            ground
        }
    };
    sized([13, sett as u32, key(&p.colors, 0)], shape, (p.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let color = if (x + y) / 2 % 2 == 0 { thread(x) } else { thread(y) };
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// Diamonds a third taller than they're wide, the two colors alternating like a chequerboard,
/// with a thin line along each diagonal through their middles.
pub fn argyle<'a, M: 'a>(a: &Argyle, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    let width = (a.size.clamp(16.0, 160.0) * s).max(8.0);
    let height = width * 1.4;
    let [first, second, line] = rgb(&a.colors, default_argyle_colors);
    // How far a line reaches either side of its middle, in the diagonals' units.
    let reach = 0.8 * (1.0 / (width * width) + 1.0 / (height * height)).sqrt();
    sized([14, width as u32, key(&a.colors, 0)], shape, (a.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let (u, v) = (x as f32 / width, y as f32 / height);
                let (down, up) = (u + v, u - v);
                let on_line = |d: f32| (d.rem_euclid(1.0) - 0.5).abs() < reach;
                let color = if on_line(down) || on_line(up) {
                    line
                } else if (down.floor() + up.floor()).rem_euclid(2.0) == 0.0 {
                    first
                } else {
                    second
                };
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// Stripes in two alternating colors, a pinstripe at each edge between them.
pub fn stripes<'a, M: 'a>(st: &Stripes, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    let band = (st.size.clamp(4.0, 120.0) * s).max(2.0);
    let direction = st.direction;
    let [first, second, pin] = rgb(&st.colors, default_stripes_colors);
    let half_pin = 0.8 * s.max(1.0);
    sized([15, band as u32, key(&st.colors, direction as u32)], shape, (st.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let at = match direction {
                    StripeDirection::Horizontal => y as f32,
                    StripeDirection::Vertical => x as f32,
                    StripeDirection::Diagonal => (x + y) as f32 * std::f32::consts::FRAC_1_SQRT_2,
                } % (band * 2.0);
                let edge = at.min((at - band).abs()).min(band * 2.0 - at);
                let color = if edge < half_pin {
                    pin
                } else if at < band {
                    first
                } else {
                    second
                };
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// Squares in two alternating colors.
pub fn checkerboard<'a, M: 'a>(c: &Checkerboard, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    let side = (c.size.clamp(4.0, 160.0) * s).max(2.0);
    let [first, second] = rgb(&c.colors, default_checkerboard_colors);
    sized([17, side as u32, key(&c.colors, 0)], shape, (c.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let color = if ((x as f32 / side).floor() + (y as f32 / side).floor()) % 2.0 == 0.0 { first } else { second };
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// Round dots on a ground, each row of them offset by half a step from the one above, their
/// edges smoothed over a pixel.
pub fn polka_dots<'a, M: 'a>(d: &PolkaDots, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    let step = (d.size.clamp(8.0, 120.0) * s).max(4.0);
    let [ground, dot] = rgb(&d.colors, default_polka_dot_colors);
    let radius = step * 0.3;
    sized([18, step as u32, key(&d.colors, 0)], shape, (d.opacity * fade).clamp(0.0, 1.0), move |x0, y0, w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let row = (py / step).floor();
                let shift = if row % 2.0 == 0.0 { 0.0 } else { step / 2.0 };
                let dx = (px - shift).rem_euclid(step) - step / 2.0;
                let dy = py - (row + 0.5) * step;
                let cover = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                let color: [u8; 3] = std::array::from_fn(|i| (ground[i] as f32 + (dot[i] as f32 - ground[i] as f32) * cover).round() as u8);
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// The colors as RGB, each one that isn't a valid `#rrggbb` its default.
fn rgb<const N: usize>(colors: &[String; N], defaults: fn() -> [String; N]) -> [[u8; 3]; N] {
    let defaults = defaults();
    std::array::from_fn(|i| parse_hex(&colors[i]).or_else(|| parse_hex(&defaults[i])).unwrap_or_default())
}

/// A number that names the colors (and `extra`), for the picture's cache key.
fn key<const N: usize>(colors: &[String; N], extra: u32) -> u32 {
    let mut hasher = DefaultHasher::new();
    (colors, extra).hash(&mut hasher);
    hasher.finish() as u32
}
