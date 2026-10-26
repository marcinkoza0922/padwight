//! Mouse-region sticks: the stick's position puts the pointer in a place on the desktop.

use super::super::*;
use crate::config::{MouseRegion, RegionShape, StickConfig};

impl Engine {
    /// Puts the pointer where a mouse-region stick points. Back at rest, the stick puts it in the
    /// region's middle once and then leaves it alone, so the mouse can still move it.
    pub(in crate::engine) fn update_region(&mut self, cfg: &StickConfig, s: Stick, pos: (f32, f32), out: &mut Vec<OutEvent>) {
        let StickAction::Region { region } = &cfg.action else {
            self.stick_region.remove(&s);
            return;
        };
        if pos == (0.0, 0.0) {
            if self.stick_region.remove(&s).is_some() {
                let (x, y) = region.point((0.0, 0.0));
                out.push(OutEvent::MouseTo(x, y));
            }
            return;
        }
        let at = stick_point(region, pos);
        if self.stick_region.insert(s, at) != Some(at) {
            out.push(OutEvent::MouseTo(at.0, at.1));
        }
    }
}

/// Where a stick at `pos` puts the pointer in `region`, as shares of the desktop. A circle takes
/// the stick's position as it is; a rectangle stretches it out to the square around the stick's
/// circle, so that pushing all the way into a diagonal reaches a corner.
pub(crate) fn stick_point(region: &MouseRegion, pos: (f32, f32)) -> (f32, f32) {
    match region.shape {
        RegionShape::Circle => region.point(pos),
        RegionShape::Rectangle => region.point(circle_to_square(pos)),
    }
}

fn circle_to_square((x, y): (f32, f32)) -> (f32, f32) {
    let longer = x.abs().max(y.abs());
    if longer == 0.0 {
        return (0.0, 0.0);
    }
    let stretch = x.hypot(y) / longer;
    (x * stretch, y * stretch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push(e: &mut Engine, p: &Profile, x: f32, y: f32) -> Vec<OutEvent> {
        let mut out = Vec::new();
        e.handle(p, InputEvent::Axis(Axis::RightX, x), Instant::now(), &mut out);
        e.handle(p, InputEvent::Axis(Axis::RightY, y), Instant::now(), &mut out);
        out
    }

    fn placed(out: &[OutEvent]) -> Option<(f32, f32)> {
        out.iter().rev().find_map(|ev| if let OutEvent::MouseTo(x, y) = ev { Some((*x, *y)) } else { None })
    }

    #[test]
    fn the_stick_places_the_pointer_and_letting_go_puts_it_in_the_middle_once() {
        let mut p = Profile::passthrough("p");
        let region = MouseRegion { shape: RegionShape::Rectangle, ..MouseRegion::default() };
        p.right_stick = StickConfig::new(StickAction::Region { region }, 0.0, 1.0);
        let mut e = Engine::default();
        assert_eq!(placed(&push(&mut e, &p, 0.0, 0.0)), None, "a stick at rest leaves the pointer alone");
        assert_eq!(placed(&push(&mut e, &p, 1.0, 0.0)), Some((0.75, 0.5)));
        let diagonal = placed(&push(&mut e, &p, std::f32::consts::FRAC_1_SQRT_2, -std::f32::consts::FRAC_1_SQRT_2)).unwrap();
        assert!((diagonal.0 - 0.75).abs() < 1e-4 && (diagonal.1 - 0.25).abs() < 1e-4, "a full diagonal reaches the corner: {diagonal:?}");
        assert_eq!(placed(&push(&mut e, &p, 0.0, 0.0)), Some((0.5, 0.5)));
        assert_eq!(placed(&push(&mut e, &p, 0.0, 0.0)), None, "only once");
    }

    #[test]
    fn a_circle_follows_the_stick_and_a_diagonal_stays_on_its_edge() {
        let mut p = Profile::passthrough("p");
        p.right_stick = StickConfig::new(StickAction::Region { region: MouseRegion::default() }, 0.0, 1.0);
        let mut e = Engine::default();
        assert_eq!(placed(&push(&mut e, &p, 1.0, 0.0)), Some((0.75, 0.5)));
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let diagonal = placed(&push(&mut e, &p, d, -d)).unwrap();
        assert!((diagonal.0 - (0.5 + 0.25 * d)).abs() < 1e-4 && (diagonal.1 - (0.5 - 0.25 * d)).abs() < 1e-4, "{diagonal:?}");
    }

    #[test]
    fn circle_to_square_keeps_the_axes_and_stretches_the_diagonals() {
        assert_eq!(circle_to_square((0.5, 0.0)), (0.5, 0.0));
        assert_eq!(circle_to_square((0.0, 0.0)), (0.0, 0.0));
        let (x, y) = circle_to_square((0.6, 0.8));
        assert!((x - 0.75).abs() < 1e-5 && (y - 1.0).abs() < 1e-5, "{x} {y}");
    }
}
