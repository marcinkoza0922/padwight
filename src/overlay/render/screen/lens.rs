//! The magnifying glass: a round window over the screen that shows the saved picture of the
//! screen, enlarged about the lens's own center. It is drawn straight, on top of every panel, so
//! the panels' pictures stay as they are underneath.

use super::super::{Corners, Fit, Op, Outline, Point, Rect, Rgba, Shadow};
use crate::overlay::Lens;

/// The lens's radius, as a share of the screen's shorter side.
const RADIUS: f32 = 0.17;
const RIM: f32 = 3.0;

/// Puts the lens on top of `ops`, on a `display` of logical pixels.
pub(super) fn put(ops: &mut Vec<Op>, lens: &Lens, display: Point) {
    let radius = (display.x.min(display.y) * RADIUS).max(40.0);
    let center = Point::new(lens.x.clamp(0.0, 1.0) * display.x, lens.y.clamp(0.0, 1.0) * display.y);
    // The lens stays on the screen; what it shows is what lies under its center.
    let at = Point::new(center.x.clamp(radius, (display.x - radius).max(radius)), center.y.clamp(radius, (display.y - radius).max(radius)));
    let rect = Rect::new(at.x - radius, at.y - radius, 2.0 * radius, 2.0 * radius);
    let round = Corners::all(radius);
    let rim = Rgba::rgb(0.95, 0.95, 0.95);
    ops.push(Op::Shadow { rect, radii: round, shadow: Shadow { color: Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.55 }, offset: (0.0, 4.0), blur: 18.0 }, spread: 0.0 });
    ops.push(Op::Rect { rect, radii: round, fill: Some(Rgba::rgb(0.08, 0.08, 0.08).into()), border: None });
    if let Some(file) = &lens.shot {
        let z = lens.zoom.max(1.0);
        // The picture, enlarged about the point under the lens's center.
        let whole = Rect::new(at.x - at.x * z, at.y - at.y * z, display.x * z, display.y * z);
        ops.push(Op::Clip { rect, radii: round });
        ops.push(Op::Picture { file: file.as_path().into(), rect: whole, fit: Fit::Stretch, opacity: 1.0 });
        ops.push(Op::Unclip);
    }
    ops.push(Op::Rect { rect, radii: round, fill: None, border: Some(Outline { width: RIM, color: rim }) });
}
