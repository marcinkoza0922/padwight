//! The patterned color filters: plaid, argyle and stripes, each solid colors over the panel.

use std::hash::{DefaultHasher, Hash, Hasher};

use iced::Element;

use super::sized;
use crate::config::{Argyle, Plaid, StripeDirection, Stripes, default_argyle_colors, default_plaid_colors, default_stripes_colors, parse_hex};

/// A tartan: each thread is the ground, a band or a line by where it lies in the sett, and the
/// twill shows the thread running down at some points and the one running across at the others,
/// so where a band crosses the ground the two hatch together.
pub fn plaid<'a, M: 'a>(p: &Plaid, s: f32, round: f32, fade: f32) -> Element<'a, M> {
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
    sized([13, sett as u32, key(&p.colors, 0)], round, (p.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let color = if (x + y) / 2 % 2 == 0 { thread(x) } else { thread(y) };
                pixels.extend_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        pixels
    })
}

/// Diamonds a third taller than they're wide, the two colors alternating like a chequerboard,
/// with a thin line along each diagonal through their middles.
pub fn argyle<'a, M: 'a>(a: &Argyle, s: f32, round: f32, fade: f32) -> Element<'a, M> {
    let width = (a.size.clamp(16.0, 160.0) * s).max(8.0);
    let height = width * 1.4;
    let [first, second, line] = rgb(&a.colors, default_argyle_colors);
    // How far a line reaches either side of its middle, in the diagonals' units.
    let reach = 0.8 * (1.0 / (width * width) + 1.0 / (height * height)).sqrt();
    sized([14, width as u32, key(&a.colors, 0)], round, (a.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
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
pub fn stripes<'a, M: 'a>(st: &Stripes, s: f32, round: f32, fade: f32) -> Element<'a, M> {
    let band = (st.size.clamp(4.0, 120.0) * s).max(2.0);
    let direction = st.direction;
    let [first, second, pin] = rgb(&st.colors, default_stripes_colors);
    let half_pin = 0.8 * s.max(1.0);
    sized([15, band as u32, key(&st.colors, direction as u32)], round, (st.opacity * fade).clamp(0.0, 1.0), move |w, h| {
        let mut pixels = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
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

/// The colors as RGB, each one that isn't a valid `#rrggbb` its default.
fn rgb(colors: &[String; 3], defaults: fn() -> [String; 3]) -> [[u8; 3]; 3] {
    let defaults = defaults();
    std::array::from_fn(|i| parse_hex(&colors[i]).or_else(|| parse_hex(&defaults[i])).unwrap_or_default())
}

/// A number that names the colors (and `extra`), for the picture's cache key.
fn key(colors: &[String; 3], extra: u32) -> u32 {
    let mut hasher = DefaultHasher::new();
    (colors, extra).hash(&mut hasher);
    hasher.finish() as u32
}
