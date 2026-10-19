//! Where a panel dragged or resized on the stage ends up, said the way its style says it: the
//! spot of the nine it is nearest, how far it sits from there, and its size, all as percentages
//! of the display, with the edges it moved snapped to the grid.

use iced::{Rectangle, Size};

use crate::config::{OverlayStyle, ScreenPosition};
use crate::overlay::draw::EDGE_MARGIN;

/// Which part of a panel the pointer holds: the middle moves it, an edge or a corner resizes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grip {
    Move,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Grip {
    /// The edges it moves: left, right, top, bottom.
    fn edges(self) -> [bool; 4] {
        match self {
            Grip::Move => [false; 4],
            Grip::N => [false, false, true, false],
            Grip::S => [false, false, false, true],
            Grip::E => [false, true, false, false],
            Grip::W => [true, false, false, false],
            Grip::NE => [false, true, true, false],
            Grip::NW => [true, false, true, false],
            Grip::SE => [false, true, false, true],
            Grip::SW => [true, false, false, true],
        }
    }
}

/// A drag on a panel: what it holds, where the panel was when it started, where the pointer has
/// taken it since, and on what display. `free` leaves the grid out (Alt held, or a key nudge).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gesture {
    pub grip: Grip,
    pub from: Rectangle,
    pub to: Rectangle,
    pub display: Size,
    pub free: bool,
}

/// The smallest a panel can be resized to, in pixels.
const MIN_SIDE: f32 = 48.0;

/// `style` with the panel where `g` leaves it. Moving picks the nearest spot (so the panel grows
/// from the corner or edge it is near) and its offset from there; resizing gives it a fixed size
/// and keeps its spot. With a `grid` (its step, in pixels), the edges that move land on its lines,
/// and a moved panel can also land back on its spot with no offset.
pub fn apply(style: &OverlayStyle, g: &Gesture, grid: Option<f32>) -> OverlayStyle {
    let step = grid.filter(|s| !g.free && *s >= 2.0);
    let display = g.display;
    let mut out = style.clone();
    let mut r = g.to;
    let [left, right, top, bottom] = g.grip.edges();
    if g.grip == Grip::Move {
        out.position = nearest_spot(r, display);
        if let Some(step) = step {
            let (col, row) = out.position.cell();
            let margin = margin(out.position);
            r.x += snap_shift(anchor(r.x, r.width, col), base(col, margin, display.width), step);
            r.y += snap_shift(anchor(r.y, r.height, row), base(row, margin, display.height), step);
        }
    } else {
        let snap = |v: f32| step.map_or(v, |s| (v / s).round() * s);
        let (mut x0, mut x1, mut y0, mut y1) = (r.x, r.x + r.width, r.y, r.y + r.height);
        if left {
            x0 = snap(x0).min(x1 - MIN_SIDE);
        }
        if right {
            x1 = snap(x1).max(x0 + MIN_SIDE);
        }
        if top {
            y0 = snap(y0).min(y1 - MIN_SIDE);
        }
        if bottom {
            y1 = snap(y1).max(y0 + MIN_SIDE);
        }
        r = Rectangle { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
        if left || right {
            let width = percent(r.width, display.width).min(100.0);
            out.width = Some(width);
            out.max_width = out.max_width.max(width);
        }
        if top || bottom {
            let height = percent(r.height, display.height).min(100.0);
            out.height = Some(height);
            out.max_height = out.max_height.max(height);
        }
    }
    let (col, row) = out.position.cell();
    let margin = margin(out.position);
    out.x_offset = percent(anchor(r.x, r.width, col) - base(col, margin, display.width), display.width).clamp(-100.0, 100.0);
    out.y_offset = percent(anchor(r.y, r.height, row) - base(row, margin, display.height), display.height).clamp(-100.0, 100.0);
    out
}

/// The spot of the nine whose third of the display the panel's middle is in.
fn nearest_spot(r: Rectangle, display: Size) -> ScreenPosition {
    let third = |at: f32, of: f32| ((at / of * 3.0).floor().max(0.0) as usize).min(2);
    let center = r.center();
    ScreenPosition::GRID[third(center.y, display.height) * 3 + third(center.x, display.width)]
}

/// How far from the display's edges a panel at `spot` sits with no offset (see `draw::place`).
fn margin(spot: ScreenPosition) -> f32 {
    if spot == ScreenPosition::Center { 0.0 } else { EDGE_MARGIN }
}

/// The side of the panel its spot holds it by, along one axis: its start, middle or end, for
/// `slot` 0, 1 or 2.
fn anchor(start: f32, length: f32, slot: usize) -> f32 {
    match slot {
        0 => start,
        2 => start + length,
        _ => start + length / 2.0,
    }
}

/// Where that side is with no offset, along an axis `extent` long.
fn base(slot: usize, margin: f32, extent: f32) -> f32 {
    match slot {
        0 => margin,
        2 => extent - margin,
        _ => extent / 2.0,
    }
}

/// How far to move a panel so the side its spot holds it by, at `at`, lands on the nearest grid
/// line, or on `home` (no offset) if that is nearer.
fn snap_shift(at: f32, home: f32, step: f32) -> f32 {
    let line = (at / step).round() * step;
    let to = if (home - at).abs() <= (line - at).abs() { home } else { line };
    to - at
}

/// `pixels` as a percentage of `of`, to two decimals.
fn percent(pixels: f32, of: f32) -> f32 {
    (pixels / of * 10_000.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISPLAY: Size = Size::new(2000.0, 1000.0);

    fn gesture(grip: Grip, from: Rectangle, to: Rectangle) -> Gesture {
        Gesture { grip, from, to, display: DISPLAY, free: false }
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle { x, y, width, height }
    }

    #[test]
    fn a_moved_panel_takes_the_spot_it_is_nearest_and_its_offset_from_there() {
        let from = rect(950.0, 450.0, 100.0, 100.0);
        let style = apply(&OverlayStyle::default(), &gesture(Grip::Move, from, rect(1700.0, 800.0, 100.0, 100.0)), None);
        assert_eq!(style.position, ScreenPosition::BottomRight);
        // Its right edge is 160 px in from the margin, its bottom 60 px.
        assert_eq!((style.x_offset, style.y_offset), (-8.0, -6.0));
        assert_eq!((style.width, style.height), (None, None), "moving keeps it fitting its contents");
    }

    #[test]
    fn with_a_grid_the_held_side_lands_on_a_line_or_back_home() {
        let from = rect(950.0, 450.0, 100.0, 100.0);
        let style = apply(&OverlayStyle::default(), &gesture(Grip::Move, from, rect(54.0, 45.0, 100.0, 100.0)), Some(32.0));
        assert_eq!(style.position, ScreenPosition::TopLeft);
        // 54 is nearer 64 than the 40 px margin, and 45 nearer the margin than 32 or 64.
        assert_eq!((style.x_offset, style.y_offset), (1.2, 0.0));
        let centered = apply(&OverlayStyle::default(), &gesture(Grip::Move, from, rect(953.0, 447.0, 100.0, 100.0)), Some(32.0));
        assert_eq!((centered.position, centered.x_offset, centered.y_offset), (ScreenPosition::Center, 0.0, 0.0));
    }

    #[test]
    fn alt_leaves_the_grid_out() {
        let from = rect(950.0, 450.0, 100.0, 100.0);
        let g = Gesture { free: true, ..gesture(Grip::Move, from, rect(52.0, 45.0, 100.0, 100.0)) };
        let style = apply(&OverlayStyle::default(), &g, Some(32.0));
        assert_eq!((style.x_offset, style.y_offset), (0.6, 0.5));
    }

    #[test]
    fn resizing_fixes_the_size_and_keeps_the_spot() {
        let style = OverlayStyle { position: ScreenPosition::TopLeft, ..OverlayStyle::default() };
        let from = rect(40.0, 40.0, 300.0, 200.0);
        let out = apply(&style, &gesture(Grip::SE, from, rect(40.0, 40.0, 410.0, 290.0)), Some(32.0));
        assert_eq!(out.position, ScreenPosition::TopLeft);
        // The right edge snaps from 450 to 448, the bottom from 330 to 320.
        assert_eq!((out.width, out.height), (Some(20.4), Some(28.0)));
        assert_eq!((out.x_offset, out.y_offset), (0.0, 0.0), "the held corner stays put");
    }

    #[test]
    fn resizing_a_centered_panel_from_one_side_moves_its_middle() {
        let from = rect(900.0, 450.0, 200.0, 100.0);
        let out = apply(&OverlayStyle::default(), &gesture(Grip::E, from, rect(900.0, 450.0, 400.0, 100.0)), None);
        assert_eq!(out.width, Some(20.0));
        assert_eq!(out.height, None, "only the edge held changes");
        assert_eq!(out.x_offset, 5.0, "the middle moved 100 px right");
    }

    #[test]
    fn a_panel_never_shrinks_past_the_smallest_size_and_its_max_grows_to_fit() {
        let from = rect(900.0, 450.0, 200.0, 100.0);
        let tiny = apply(&OverlayStyle::default(), &gesture(Grip::W, from, rect(1150.0, 450.0, -50.0, 100.0)), None);
        assert_eq!(tiny.width, Some(percent(MIN_SIDE, DISPLAY.width)));
        let wide = apply(&OverlayStyle::default(), &gesture(Grip::E, from, rect(0.0, 450.0, 1960.0, 100.0)), None);
        assert_eq!((wide.width, wide.max_width), (Some(98.0), 98.0));
    }
}
