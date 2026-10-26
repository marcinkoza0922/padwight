//! Mouse regions: a circle (or ellipse) or a rectangle of the desktop that a stick's or a finger's
//! position maps onto, so where the stick points (or where the finger is on the pad) is where the
//! pointer is.

use serde::{Deserialize, Serialize};

/// A part of the desktop, each number a share (0..1) of the whole desktop, every monitor
/// together: its middle, its size and its shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub shape: RegionShape,
}

/// A mouse region's shape. Either fills the region's width and height, so a circle whose width and
/// height differ (on the desktop, in pixels) is an ellipse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionShape {
    /// A stick's own shape: pushing the stick all the way in any direction reaches the edge.
    #[default]
    Circle,
    /// Pushing the stick all the way into a diagonal reaches a corner.
    Rectangle,
}

impl RegionShape {
    pub const ALL: [RegionShape; 2] = [RegionShape::Circle, RegionShape::Rectangle];
}

impl std::fmt::Display for RegionShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RegionShape::Circle => "Circle",
            RegionShape::Rectangle => "Rectangle",
        })
    }
}

impl Default for MouseRegion {
    /// A circle over the middle half of the desktop.
    fn default() -> Self {
        MouseRegion { x: 0.5, y: 0.5, width: 0.5, height: 0.5, shape: RegionShape::Circle }
    }
}

impl MouseRegion {
    /// The whole desktop.
    pub const FULL: MouseRegion = MouseRegion { x: 0.5, y: 0.5, width: 1.0, height: 1.0, shape: RegionShape::Rectangle };

    /// Where `(u, v)` (each -1..1 from the region's middle, positive right and down) is on the
    /// desktop, as shares of it. A point outside the region's shape is brought in to its edge,
    /// and the region is kept on the desktop.
    pub fn point(&self, (u, v): (f32, f32)) -> (f32, f32) {
        let (u, v) = match self.shape {
            RegionShape::Rectangle => (u.clamp(-1.0, 1.0), v.clamp(-1.0, 1.0)),
            RegionShape::Circle => {
                let r = u.hypot(v);
                if r > 1.0 { (u / r, v / r) } else { (u, v) }
            }
        };
        let x = self.x + u * self.width / 2.0;
        let y = self.y + v * self.height / 2.0;
        (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
    }

    /// The region with `preset`'s place and size, keeping its shape.
    pub fn placed_as(self, preset: MouseRegion) -> MouseRegion {
        MouseRegion { shape: self.shape, ..preset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_maps_its_corners_and_stays_on_the_desktop() {
        let r = MouseRegion { x: 0.25, y: 0.5, width: 0.5, height: 0.2, shape: RegionShape::Rectangle };
        assert_eq!(r.point((0.0, 0.0)), (0.25, 0.5));
        assert_eq!(r.point((1.0, -1.0)), (0.5, 0.4));
        assert_eq!(r.point((-2.0, 0.0)), (0.0, 0.5), "past the edge stops at it");
        let off = MouseRegion { x: 0.9, ..r };
        assert_eq!(off.point((1.0, 0.0)).0, 1.0, "a region hanging off the desktop is cut at its edge");
    }

    #[test]
    fn a_circle_brings_points_past_its_edge_in_and_stretches_to_an_ellipse() {
        let r = MouseRegion { x: 0.5, y: 0.5, width: 0.4, height: 0.8, shape: RegionShape::Circle };
        assert_eq!(r.point((1.0, 0.0)), (0.7, 0.5));
        assert!((r.point((0.0, -1.0)).1 - 0.1).abs() < 1e-6);
        let (x, y) = r.point((1.0, 1.0));
        let d = std::f32::consts::FRAC_1_SQRT_2;
        assert!((x - (0.5 + 0.2 * d)).abs() < 1e-5 && (y - (0.5 + 0.4 * d)).abs() < 1e-5, "a corner lands on the ellipse: {x} {y}");
    }

    #[test]
    fn missing_fields_take_the_default() {
        let r: MouseRegion = toml::from_str("width = 1.0").unwrap();
        assert_eq!(r, MouseRegion { width: 1.0, ..MouseRegion::default() });
    }
}
