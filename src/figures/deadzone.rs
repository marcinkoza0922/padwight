//! The deadzone in two pictures: where the stick is on its travel, with the dead middle shaded,
//! and the response along one push, flat until the deadzone and then rising to full. A live stick
//! is drawn in both: hollow where it is, filled where the deadzone puts it.

use std::fmt::Write;

use crate::config::{Deadzone, DeadzoneShape};
use crate::engine::apply_deadzone;

/// Side of each picture, and the margin kept clear inside it, in logical pixels.
pub const SIDE: f32 = 120.0;
const PAD: f32 = 10.0;
const PANEL: &str = "#1d2026";
const DEAD: &str = "#3d434e";
const LINEAR: &str = "#5d6470";
const STEADY: &str = "#4ea1ff";
const MARK: &str = "#ffffff";

/// The live stick as `(x, y)` in -1..1, pulled back inside the travel circle.
fn inside_travel(live: (f32, f32)) -> (f32, f32) {
    let m = live.0.hypot(live.1);
    if m > 1.0 { (live.0 / m, live.1 / m) } else { live }
}

fn circle(c: f32, r: f32, fill: &str, stroke: &str, dash: &str) -> String {
    format!(r#"<circle cx="{c}" cy="{c}" r="{r:.1}" fill="{fill}" stroke="{stroke}" stroke-width="1" stroke-dasharray="{dash}"/>"#)
}

fn dot(at: (f32, f32), fill: &str) -> String {
    format!(r##"<circle cx="{:.1}" cy="{:.1}" r="4.5" fill="{fill}" stroke="#000" stroke-width="1"/>"##, at.0, at.1)
}

fn ring_dot(at: (f32, f32)) -> String {
    format!(r##"<circle cx="{:.1}" cy="{:.1}" r="4.5" fill="none" stroke="{MARK}" stroke-width="1.5"/>"##, at.0, at.1)
}

fn frame(extra: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{SIDE}" height="{SIDE}" viewBox="0 0 {SIDE} {SIDE}"><rect width="{SIDE}" height="{SIDE}" rx="6" fill="{PANEL}"/>{extra}"#)
}

fn rect((x, y, w, h): (f32, f32, f32, f32), fill: &str, stroke: &str, dash: &str) -> String {
    format!(r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{fill}" stroke="{stroke}" stroke-width="1" stroke-dasharray="{dash}"/>"#)
}

/// The dead middle's outline and shading, in the shape the deadzone has.
fn dead_shape(dz: Deadzone, c: f32, r: f32) -> String {
    let d = dz.size * r;
    let both = |r: (f32, f32, f32, f32)| rect(r, DEAD, "none", "none") + &rect(r, "none", LINEAR, "2 2");
    match dz.shape {
        DeadzoneShape::Circle => circle(c, d, DEAD, "none", "none") + &circle(c, d, "none", LINEAR, "2 2"),
        DeadzoneShape::Square => both((c - d, c - d, 2.0 * d, 2.0 * d)),
        DeadzoneShape::Cross => both((c - d, c - r, 2.0 * d, 2.0 * r)) + &both((c - r, c - d, 2.0 * r, 2.0 * d)),
    }
}

/// SVG of the stick's travel: the full circle, its dead middle shaded, and the live stick.
pub fn travel_svg(dz: Deadzone, live: Option<(f32, f32)>) -> String {
    let c = SIDE / 2.0;
    let r = c - PAD;
    let mut out = frame(&circle(c, r, "none", LINEAR, "3 3"));
    out.push_str(&dead_shape(dz, c, r));
    if let Some(at) = live {
        let (x, y) = inside_travel(at);
        // Screen y points down, as the stick's does.
        let raw = (c + x * r, c + y * r);
        let (mx, my) = apply_deadzone(x, y, dz);
        out.push_str(&ring_dot(raw));
        out.push_str(&dot((c + mx * r, c + my * r), STEADY));
    }
    out.push_str("</svg>");
    out
}

/// SVG of the response: the push going in (across) against the output (up). The shaded band is
/// the deadzone, and the dotted line is what a stick with no deadzone would give.
pub fn response_svg(dz: Deadzone, live: Option<(f32, f32)>) -> String {
    let span = SIDE - 2.0 * PAD;
    let x = |m: f32| PAD + m * span;
    let y = |v: f32| SIDE - PAD - v * span;
    let start = (dz.size + dz.buffer).min(1.0);
    let mut out = frame("");
    out.push_str(&format!(r#"<rect x="{PAD}" y="{PAD}" width="{:.1}" height="{span}" fill="{DEAD}"/>"#, dz.size * span));
    out.push_str(&format!(r#"<path d="M{:.1} {:.1} L{:.1} {:.1}" stroke="{LINEAR}" stroke-width="1.5" stroke-dasharray="3 3" fill="none"/>"#, x(0.0), y(0.0), x(1.0), y(1.0)));
    let mut points = vec![(0.0, 0.0), (dz.size, 0.0)];
    if dz.anti > 0.0 {
        points.push((dz.size, dz.anti));
        points.push((start, dz.anti));
    }
    points.push((1.0, 1.0));
    let path: Vec<String> = points.iter().enumerate().map(|(i, (m, v))| format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x(*m), y(*v))).collect();
    out.push_str(&format!(r#"<path d="{}" stroke="{STEADY}" stroke-width="2" stroke-linejoin="round" fill="none"/>"#, path.join(" ")));
    if let Some(at) = live {
        let (x_in, y_in) = inside_travel(at);
        let mag = x_in.hypot(y_in);
        // The output magnitude the deadzone gives this push: the x of a push along one axis.
        let (out_mag, _) = apply_deadzone(mag, 0.0, dz);
        let _ = write!(out, r#"<path d="M{0:.1} {1:.1} L{0:.1} {2:.1}" stroke="{MARK}" stroke-width="1" stroke-dasharray="2 2"/>"#, x(mag), y(0.0), y(out_mag));
        out.push_str(&dot((x(mag), y(out_mag)), STEADY));
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dead_middle_is_as_wide_as_the_deadzone() {
        let svg = travel_svg(0.2.into(), None);
        assert!(svg.contains(&format!(r#"r="{:.1}""#, 0.2 * (SIDE / 2.0 - PAD))));
        assert_eq!(svg.matches("<circle").count(), 3);
    }

    #[test]
    fn a_live_stick_inside_the_deadzone_maps_to_the_middle() {
        let svg = travel_svg(0.3.into(), Some((0.1, 0.0)));
        // The filled dot sits on the center, the hollow one where the stick is.
        assert!(svg.contains(&format!(r#"cx="{:.1}""#, SIDE / 2.0)));
        assert_eq!(svg.matches("<circle").count(), 5);
    }

    #[test]
    fn the_response_is_flat_until_the_deadzone() {
        let svg = response_svg(0.5.into(), Some((0.25, 0.0)));
        // The live push is in the flat band, so its output marker sits on the baseline.
        assert!(svg.contains(&format!(r#"cy="{:.1}""#, SIDE - PAD)));
    }
}
