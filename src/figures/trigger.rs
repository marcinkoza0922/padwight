//! A trigger's travel as a bar: where it presses (for a trigger that acts as a button), its
//! zones, and how far the live trigger is pulled.

use std::fmt::Write;

pub const W: f32 = 220.0;
pub const H: f32 = 44.0;
const PAD: f32 = 10.0;
const PANEL: &str = "#1d2026";
const TRACK: &str = "#30343c";
const ZONE: &str = "#2e7d46";
const PRESS: &str = "#e8a33d";
const LIVE: &str = "#4ea1ff";

/// SVG of the travel from rest (left) to a full pull (right): the zones as bands, the press
/// point as a line, and the live pull filled in.
pub fn travel_svg(press_at: Option<f32>, zones: &[(f32, f32)], live: Option<f32>) -> String {
    let span = W - 2.0 * PAD;
    let x = |v: f32| PAD + v.clamp(0.0, 1.0) * span;
    let (top, height) = (PAD, H - 2.0 * PAD);
    let mut out = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}"><rect width="{W}" height="{H}" rx="6" fill="{PANEL}"/>"#);
    let _ = write!(out, r#"<rect x="{PAD}" y="{top}" width="{span}" height="{height}" rx="4" fill="{TRACK}"/>"#);
    for &(from, to) in zones {
        let (a, b) = (x(from.min(to)), x(from.max(to)));
        let _ = write!(out, r#"<rect x="{a:.1}" y="{top}" width="{:.1}" height="{height}" fill="{ZONE}" fill-opacity="0.6"/>"#, b - a);
    }
    if let Some(pull) = live {
        let _ = write!(out, r#"<rect x="{PAD}" y="{:.1}" width="{:.1}" height="{:.1}" rx="3" fill="{LIVE}"/>"#, top + height * 0.3, x(pull) - PAD, height * 0.4);
    }
    if let Some(at) = press_at {
        let _ = write!(out, r#"<path d="M{0:.1} {1} L{0:.1} {2}" stroke="{PRESS}" stroke-width="2"/>"#, x(at), top - 4.0, top + height + 4.0);
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_press_point_and_pull_are_drawn_when_there_are_any() {
        let bare = travel_svg(None, &[], None);
        assert_eq!(bare.matches("<rect").count(), 2);
        let full = travel_svg(Some(0.5), &[(0.2, 0.4), (0.8, 1.0)], Some(0.3));
        assert_eq!(full.matches("<rect").count(), 5);
        assert!(full.contains(PRESS));
    }
}
