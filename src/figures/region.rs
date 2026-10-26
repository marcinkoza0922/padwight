//! A mouse region on the desktop: the desktop as a wide panel with the region drawn on it, and
//! where the pointer goes for a live stick or finger.

use std::fmt::Write;

use crate::config::MouseRegion;

/// The picture's size in logical pixels: a 16:9 desktop.
pub const WIDTH: f32 = 192.0;
pub const HEIGHT: f32 = 108.0;
const PANEL: &str = "#1d2026";
const EDGE: &str = "#3a3f48";
const REGION: &str = "#4ea1ff";
const MARK: &str = "#ffffff";

/// The desktop with `region` on it, and a mark at `pointer` (shares of the desktop) when given.
pub fn region_svg(region: &MouseRegion, pointer: Option<(f32, f32)>) -> String {
    let (x0, y0) = region.point((-1.0, -1.0));
    let (x1, y1) = region.point((1.0, 1.0));
    let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}">"#);
    let _ = write!(s, r#"<rect x="0.5" y="0.5" width="{}" height="{}" rx="4" fill="{PANEL}" stroke="{EDGE}"/>"#, WIDTH - 1.0, HEIGHT - 1.0);
    let _ = write!(
        s,
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{REGION}" fill-opacity="0.18" stroke="{REGION}" stroke-width="1.5"/>"#,
        x0 * WIDTH,
        y0 * HEIGHT,
        ((x1 - x0) * WIDTH).max(1.0),
        ((y1 - y0) * HEIGHT).max(1.0)
    );
    let (cx, cy) = region.point((0.0, 0.0));
    let (cx, cy) = (cx * WIDTH, cy * HEIGHT);
    let _ = write!(s, r#"<path d="M {} {cy:.1} H {} M {cx:.1} {} V {}" stroke="{REGION}"/>"#, cx - 5.0, cx + 5.0, cy - 5.0, cy + 5.0);
    if let Some((px, py)) = pointer {
        let _ = write!(s, r##"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{MARK}" stroke="#000" stroke-width="1"/>"##, px * WIDTH, py * HEIGHT);
    }
    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_region_and_the_pointer_are_drawn() {
        let svg = region_svg(&MouseRegion::default(), Some((0.5, 0.5)));
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(svg.contains(r#"x="48.0" y="27.0" width="96.0" height="54.0""#), "{svg}");
        assert!(svg.contains("<circle"));
    }
}
