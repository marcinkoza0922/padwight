//! Mouse regions: a rectangle of the desktop that a stick's or a finger's position maps onto, so
//! where the stick points (or where the finger is on the pad) is where the pointer is.

use serde::{Deserialize, Serialize};

/// A rectangle of the desktop, each number a share (0..1) of the whole desktop, every monitor
/// together: its middle and its size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for MouseRegion {
    /// The middle half of the desktop.
    fn default() -> Self {
        MouseRegion { x: 0.5, y: 0.5, width: 0.5, height: 0.5 }
    }
}

impl MouseRegion {
    /// The whole desktop.
    pub const FULL: MouseRegion = MouseRegion { x: 0.5, y: 0.5, width: 1.0, height: 1.0 };

    /// Where `(u, v)` (each -1..1 from the region's middle, positive right and down) is on the
    /// desktop, as shares of it, kept on the desktop.
    pub fn point(&self, (u, v): (f32, f32)) -> (f32, f32) {
        let x = self.x + u.clamp(-1.0, 1.0) * self.width / 2.0;
        let y = self.y + v.clamp(-1.0, 1.0) * self.height / 2.0;
        (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_maps_its_corners_and_stays_on_the_desktop() {
        let r = MouseRegion { x: 0.25, y: 0.5, width: 0.5, height: 0.2 };
        assert_eq!(r.point((0.0, 0.0)), (0.25, 0.5));
        assert_eq!(r.point((1.0, -1.0)), (0.5, 0.4));
        assert_eq!(r.point((-2.0, 0.0)), (0.0, 0.5), "past the edge stops at it");
        let off = MouseRegion { x: 0.9, ..r };
        assert_eq!(off.point((1.0, 0.0)).0, 1.0, "a region hanging off the desktop is cut at its edge");
    }

    #[test]
    fn missing_fields_take_the_default() {
        let r: MouseRegion = toml::from_str("width = 1.0").unwrap();
        assert_eq!(r, MouseRegion { width: 1.0, ..MouseRegion::default() });
    }
}
