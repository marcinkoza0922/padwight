//! Where a radial menu's parts go, in pixels at a scale: shared by both overlay renderers, so
//! they place the arcs, the hub and the boxes alike.

use std::f32::consts::TAU;

/// The hub's width at scale 1.
pub const HUB: f32 = 48.0;
/// The width and height of an item's label cell at scale 1.
pub const CELL: (f32, f32) = (120.0, 44.0);

/// Where a radial menu's arcs go: their angles, the radii between which they're drawn, and the
/// hub's size, at scale `s`.
#[derive(Debug, Clone, PartialEq)]
pub struct Ring {
    pub arcs: Vec<(f32, f32)>,
    pub inner: f32,
    pub outer: f32,
    pub hub: f32,
    pub s: f32,
}

impl Ring {
    /// The ring for items of these `weights`, at scale `s`.
    pub fn new(weights: &[f32], s: f32) -> Ring {
        let arcs = super::arcs(weights);
        let hub = HUB * s;
        let inner = hub / 2.0 + 6.0 * s;
        Ring { inner, outer: outer_radius(&arcs, inner, s), hub, s, arcs }
    }

    /// The width and height of the square it's drawn in.
    pub fn size(&self) -> f32 {
        2.0 * self.outer + 16.0 * self.s
    }

    /// The point `r` from the middle of the square, `degrees` clockwise from up.
    pub fn point(&self, degrees: f32, r: f32) -> (f32, f32) {
        let center = self.size() / 2.0;
        let (sin, cos) = degrees.to_radians().sin_cos();
        (center + sin * r, center - cos * r)
    }

    /// The whole pixels around arc `i`, as `(x, y, width, height)` in the square: the box round
    /// its corners and every quarter turn it passes (a first arc that spans more than half the
    /// circle starts in the negatives), at both radii.
    pub fn arc_box(&self, i: usize) -> (f32, f32, f32, f32) {
        let (start, end) = self.arcs[i];
        let size = self.size();
        let quarters = (-4..=4).map(|q| q as f32 * 90.0).filter(|&q| q > start && q < end);
        let (mut x0, mut y0, mut x1, mut y1) = (size, size, 0.0_f32, 0.0_f32);
        for degrees in [start, end].into_iter().chain(quarters) {
            for r in [self.inner, self.outer] {
                let (x, y) = self.point(degrees, r);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
        let (x0, y0) = (x0.floor().max(0.0), y0.floor().max(0.0));
        (x0, y0, x1.ceil().min(size) - x0, y1.ceil().min(size) - y0)
    }
}

/// How far the arcs reach: far enough that the middle of each arc has room for its label,
/// up to a limit for very many items.
pub fn outer_radius(arcs: &[(f32, f32)], inner: f32, s: f32) -> f32 {
    let label = CELL.0 * s;
    let fits = |outer: f32| {
        let mid = (inner + outer) / 2.0;
        arcs.len() < 2 || arcs.iter().all(|&(a, b)| 2.0 * mid * ((b - a).to_radians() / 2.0).sin() >= label)
    };
    let mut outer = 170.0 * s;
    while !fits(outer) && outer < 420.0 * s {
        outer += 4.0 * s;
    }
    outer
}

/// The smallest ring (at least 150) on which neighbouring `w`×`h` cells don't touch.
pub fn box_radius(n: usize, w: f32, h: f32) -> f32 {
    let gap = 8.0;
    let step = TAU / n.max(1) as f32;
    let overlaps = |r: f32| {
        (0..n).any(|i| {
            let (a, b) = (i as f32 * step, (i + 1) as f32 * step);
            let dx = (r * a.sin() - r * b.sin()).abs();
            let dy = (r * a.cos() - r * b.cos()).abs();
            n > 1 && dx < w + gap && dy < h + gap
        })
    };
    let mut r = 150.0;
    while overlaps(r) && r < 600.0 {
        r += 5.0;
    }
    r
}

/// The side of the square that holds boxes for `n` items at scale `s`, and the ring's radius.
pub fn box_square(n: usize, s: f32) -> (f32, f32) {
    let radius = box_radius(n, CELL.0, CELL.1) * s;
    (2.0 * radius + CELL.0 * s, radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arcs_reach_until_each_label_fits() {
        let few = Ring::new(&[1.0; 3], 1.0);
        assert_eq!(few.outer, 170.0);
        let many = Ring::new(&[1.0; 16], 1.0);
        assert!(many.outer > 170.0 && many.outer < 430.0);
        assert_eq!(Ring::new(&[1.0; 3], 2.0).outer, 340.0);
        assert_eq!(few.size(), 2.0 * 170.0 + 16.0);
    }

    #[test]
    fn the_box_around_an_arc_holds_its_corners_and_quarter_turns() {
        let ring = Ring::new(&[1.0; 4], 1.0);
        let center = ring.size() / 2.0;
        // The first arc is centered on up, from -45 to 45 degrees.
        let (x, y, w, h) = ring.arc_box(0);
        assert!(x < center && x + w > center && y < center - ring.outer + 1.0 && y + h <= center);
        assert_eq!((x.fract(), y.fract(), w.fract(), h.fract()), (0.0, 0.0, 0.0, 0.0));
        // A lone arc is the whole circle.
        let (x, _, w, _) = Ring::new(&[1.0], 1.0).arc_box(0);
        assert!(x <= 8.0 && w >= 2.0 * 170.0);
    }

    #[test]
    fn boxes_stay_apart() {
        assert_eq!(box_radius(1, 120.0, 44.0), 150.0);
        assert!(box_radius(12, 120.0, 44.0) > 150.0);
        let (size, radius) = box_square(4, 2.0);
        assert_eq!(size, 2.0 * radius + 240.0);
    }
}
