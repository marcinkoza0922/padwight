//! Where a panel sits on the full-screen surface, without iced: the margins its position and
//! offset leave around it, and the corner it lands on. Both renderers place panels with these.

use super::Point;
use crate::config::ScreenPosition;

/// How far a panel off center sits from the screen's edge.
pub const EDGE_MARGIN: f32 = 40.0;

/// The space left on each side of a panel's box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// The margins for a panel at `position`, `shift` pixels (x, y) off where that puts it. A
/// panel at the edge keeps `EDGE_MARGIN` from it; one pushed outward loses the margin, never
/// less than none.
pub fn margins(position: ScreenPosition, shift: (f32, f32)) -> Margins {
    let (col, row) = position.cell();
    let margin = if position == ScreenPosition::Center { 0.0 } else { EDGE_MARGIN };
    let (left, right) = sides(margin, shift.0, col);
    let (top, bottom) = sides(margin, shift.1, row);
    Margins { top, right, bottom, left }
}

/// The padding on the two sides of a panel, for a panel `shift` pixels off where it sits in
/// `slot` (0 start, 1 center, 2 end) of a `margin` from the edge.
fn sides(margin: f32, shift: f32, slot: usize) -> (f32, f32) {
    match slot {
        0 => ((margin + shift).max(0.0), margin),
        2 => (margin, (margin - shift).max(0.0)),
        // A centered panel sits halfway between its two paddings.
        _ => (margin + (2.0 * shift).max(0.0), margin + (-2.0 * shift).max(0.0)),
    }
}

/// The most room a panel has on a `display`, once its margins are taken.
pub fn room(display: Point, margins: Margins) -> Point {
    Point::new((display.x - margins.left - margins.right).max(0.0), (display.y - margins.top - margins.bottom).max(0.0))
}

/// The top left corner of a panel `size` big at `position` on a `display`, `shift` pixels off
/// where the position puts it.
pub fn origin(display: Point, size: Point, position: ScreenPosition, shift: (f32, f32)) -> Point {
    let m = margins(position, shift);
    let room = room(display, m);
    let (col, row) = position.cell();
    let along = |slot: usize, start: f32, room: f32, size: f32| start + (room - size).max(0.0) * [0.0, 0.5, 1.0][slot];
    Point::new(along(col, m.left, room.x, size.x), along(row, m.top, room.y, size.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISPLAY: Point = Point::new(1000.0, 600.0);
    const SIZE: Point = Point::new(200.0, 100.0);

    fn at(position: ScreenPosition, shift: (f32, f32)) -> (f32, f32) {
        let p = origin(DISPLAY, SIZE, position, shift);
        (p.x, p.y)
    }

    #[test]
    fn the_nine_spots_sit_in_their_corners_a_margin_off() {
        assert_eq!(at(ScreenPosition::TopLeft, (0.0, 0.0)), (40.0, 40.0));
        assert_eq!(at(ScreenPosition::TopRight, (0.0, 0.0)), (760.0, 40.0));
        assert_eq!(at(ScreenPosition::BottomLeft, (0.0, 0.0)), (40.0, 460.0));
        assert_eq!(at(ScreenPosition::BottomRight, (0.0, 0.0)), (760.0, 460.0));
        assert_eq!(at(ScreenPosition::TopCenter, (0.0, 0.0)), (400.0, 40.0));
        assert_eq!(at(ScreenPosition::CenterLeft, (0.0, 0.0)), (40.0, 250.0));
        assert_eq!(at(ScreenPosition::CenterRight, (0.0, 0.0)), (760.0, 250.0));
    }

    #[test]
    fn the_center_has_no_margin() {
        assert_eq!(at(ScreenPosition::Center, (0.0, 0.0)), (400.0, 250.0));
        assert_eq!(margins(ScreenPosition::Center, (0.0, 0.0)), Margins { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 });
    }

    #[test]
    fn an_offset_moves_the_panel_but_never_past_the_edge() {
        assert_eq!(at(ScreenPosition::TopLeft, (30.0, -10.0)), (70.0, 30.0));
        assert_eq!(at(ScreenPosition::TopLeft, (-100.0, -100.0)), (0.0, 0.0), "held at the edge");
        assert_eq!(at(ScreenPosition::BottomRight, (-30.0, 20.0)), (730.0, 480.0));
        assert_eq!(at(ScreenPosition::BottomRight, (100.0, 100.0)), (800.0, 500.0), "held at the edge");
    }

    #[test]
    fn a_centered_panel_moves_by_its_shift() {
        assert_eq!(at(ScreenPosition::Center, (50.0, -20.0)), (450.0, 230.0));
        assert_eq!(at(ScreenPosition::Center, (-50.0, 20.0)), (350.0, 270.0));
    }

    #[test]
    fn a_panel_bigger_than_the_room_sits_at_the_start() {
        let p = origin(Point::new(100.0, 100.0), SIZE, ScreenPosition::Center, (0.0, 0.0));
        assert_eq!((p.x, p.y), (0.0, 0.0));
    }
}
