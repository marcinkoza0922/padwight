//! The radial picture of a button ring: its sectors around a stick, the inner radius, and the
//! live stick position with the sector it points into highlighted.

use std::fmt::Write;

/// Side of the square preview, in logical pixels.
pub const SIZE: f32 = 120.0;
const RADIUS: f32 = 56.0;
/// The panel behind the ring, as the deadzone graphs beside it have, and the wedges a step lighter
/// so they show on it.
const PANEL: &str = "#1d2026";
const IDLE: &str = "#262b33";
const EDGE: &str = "#5d6470";
const ACTIVE: &str = "#4ea1ff";
const DEAD: &str = "#30343c";

/// The point `fraction` of the way out from the center at `degrees` clockwise from up.
fn polar(degrees: f32, fraction: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (SIZE / 2.0 + sin * fraction * RADIUS, SIZE / 2.0 - cos * fraction * RADIUS)
}

/// SVG of a ring of `sectors` equal wedges, the first centered `start_angle` degrees clockwise
/// from up, with `active` filled and `stick` (x right, y down, -1..1) drawn as a dot.
pub fn render(sectors: u8, start_angle: f32, inner_radius: f32, active: Option<usize>, stick: Option<(f32, f32)>) -> String {
    let n = usize::from(sectors.max(1));
    let width = 360.0 / n as f32;
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}"><rect width="{SIZE}" height="{SIZE}" rx="6" fill="{PANEL}"/>"#
    );
    // One sector is the whole circle, which an arc from a point back to itself can't draw.
    if n == 1 {
        let fill = if active == Some(0) { ACTIVE } else { IDLE };
        let _ = write!(out, r#"<circle cx="{c}" cy="{c}" r="{RADIUS}" fill="{fill}" stroke="{EDGE}" stroke-width="1.5"/>"#, c = SIZE / 2.0);
    }
    for i in (0..n).filter(|_| n > 1) {
        let center = start_angle + i as f32 * width;
        let ((x0, y0), (x1, y1)) = (polar(center - width / 2.0, 1.0), polar(center + width / 2.0, 1.0));
        let fill = if active == Some(i) { ACTIVE } else { IDLE };
        let large = u8::from(width > 180.0);
        let c = SIZE / 2.0;
        let _ = write!(
            out,
            r#"<path d="M{c} {c} L{x0:.1} {y0:.1} A{RADIUS} {RADIUS} 0 {large} 1 {x1:.1} {y1:.1} Z" fill="{fill}" stroke="{EDGE}" stroke-width="1.5"/>"#
        );
    }
    let _ = write!(
        out,
        r#"<circle cx="{c}" cy="{c}" r="{r:.1}" fill="{DEAD}" stroke="{EDGE}" stroke-dasharray="3 3"/>"#,
        c = SIZE / 2.0,
        r = inner_radius * RADIUS
    );
    if let Some((x, y)) = stick {
        let scale = RADIUS / x.hypot(y).max(1.0);
        let _ = write!(
            out,
            r##"<circle cx="{:.1}" cy="{:.1}" r="5" fill="#fff" stroke="#000" stroke-width="1"/>"##,
            SIZE / 2.0 + x * scale,
            SIZE / 2.0 + y * scale
        );
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_one_wedge_per_sector_and_highlights_the_active_one() {
        let svg = render(8, 0.0, 0.5, Some(2), None);
        assert_eq!(svg.matches("<path").count(), 8);
        assert_eq!(svg.matches(ACTIVE).count(), 1);
        assert!(!render(4, 0.0, 0.5, None, None).contains(ACTIVE));
    }

    #[test]
    fn a_ring_of_one_sector_is_drawn_whole_and_lights_up() {
        use resvg::{tiny_skia, usvg};
        let size = SIZE as u32;
        let tree = usvg::Tree::from_str(&render(1, 0.0, 0.3, Some(0), None), &usvg::Options::default()).unwrap();
        let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
        // Between the dead zone and the rim, all the way round, the active sector's blue shows.
        for (x, y) in [(60, 15), (105, 60), (60, 105), (15, 60)] {
            let p = pixmap.pixel(x, y).unwrap();
            assert!(p.blue() > p.red() + 50, "({x}, {y}) is {p:?}");
        }
    }

    #[test]
    fn the_stick_dot_stays_inside_the_ring() {
        let svg = render(4, 0.0, 0.5, None, Some((3.0, 0.0)));
        assert!(svg.contains(&format!(r#"cx="{:.1}""#, SIZE / 2.0 + RADIUS)));
    }
}
