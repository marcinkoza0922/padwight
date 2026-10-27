//! The doodles over the game: the canvas's strokes and text boxes, the cursor of the Doodle screen,
//! and its HUD. They go under the magnifying glass and every panel, so a menu is never drawn under
//! a doodle. Positions and sizes are shares of the screen (see `tools::doodle`), and are turned
//! into logical pixels here.

mod hud;
mod marks;

use std::f32::consts::TAU;

use super::{
    Context,
    super::{Cap, Join, Node, Op, Path, Point, Rect, Rgba, Rule, Stroke, text::Spec},
};
use crate::{
    overlay::{DoodleCursor, OverlayFrame},
    tools::doodle::{Item, Kind, PALETTE, Point as Share, Stroke as Mark, TextBox, Tip},
};

/// The cursor's rim, and how many sides its circle has.
const RIM: f32 = 2.0;
const SIDES: usize = 32;
/// How much wider a spray is than its pen, so its dither covers the stroke's area.
const SPRAY_WIDTH: f32 = 1.6;

/// Puts the doodles of `frame` on top of `ops`, on the display in `cx`.
pub(super) fn put(ops: &mut Vec<Op>, frame: &OverlayFrame, cx: &Context) {
    for item in frame.doodles.items() {
        match item {
            Item::Stroke(mark) => stroke(ops, mark, cx),
            Item::Text(text) => text_box(ops, text, cx),
        }
    }
    if let Some(mark) = frame.doodles.pending() {
        stroke(ops, mark, cx);
    }
    if let Some(cursor) = &frame.doodle_cursor {
        cursor_ring(ops, cursor, cx);
    }
    if let Some(hud) = &frame.doodle_hud {
        hud::carousel(ops, hud, cx);
        if hud.tray {
            hud::tray(ops, hud, cx);
        }
        if let Some(checks) = &hud.checks {
            hud::checklist(ops, checks, hud, cx);
        }
    }
}

/// A logical pixel point for a share of the screen.
fn at(share: Share, cx: &Context) -> Point {
    Point::new(share.x * cx.display.x, share.y * cx.display.y)
}

/// The color of palette entry `index`, at `alpha`.
pub(super) fn ink(index: usize, alpha: f32) -> Rgba {
    let [r, g, b] = PALETTE[index % PALETTE.len()];
    Rgba::from_rgba8(r, g, b, alpha)
}

/// Draws a stroke in the look its pen's style gives it.
fn stroke(ops: &mut Vec<Op>, mark: &Mark, cx: &Context) {
    let px: Vec<Point> = mark.points.iter().map(|&p| at(p, cx)).collect();
    if px.is_empty() {
        return;
    }
    let pen = &mark.pen;
    let width = pen.size * cx.display.y;
    let color = ink(pen.color, 1.0);
    match pen.kind {
        Kind::Solid => {
            let mut path = Path::new();
            marks::trace(&mut path, &px);
            let stroke = Stroke { cap: cap(pen.tip), join: Join::Round, ..Stroke::new(width, color) };
            ops.push(Op::Path { path, fill: None, rule: Rule::NonZero, stroke: Some(stroke) });
        }
        Kind::Brush => marks::brush(ops, &px, width, color, cap(pen.tip)),
        Kind::Spray => marks::spray(ops, &px, width * SPRAY_WIDTH, color),
    }
}

/// The cap a pen's tip makes at the ends of a stroke.
fn cap(tip: Tip) -> Cap {
    match tip {
        Tip::Round => Cap::Round,
        Tip::Square => Cap::Square,
        Tip::Chisel | Tip::Diagonal => Cap::Butt,
    }
}

fn text_box(ops: &mut Vec<Op>, text: &TextBox, cx: &Context) {
    let style = text.style;
    let spec = Spec {
        family: cx.font,
        bold: style.bold,
        italic: style.italic,
        underline: style.underline,
        ..Spec::new(text.text.clone(), style.size * cx.display.y)
    };
    let node = Node::text(spec, ink(style.color, 1.0));
    let corner = at(text.at, cx);
    let size = node.measure(cx.display.x);
    node.place(Rect::new(corner.x, corner.y, size.x, size.y), ops);
}

/// The cursor: a ring the size of the pen (or the eraser's reach), in the pen's color, with a dark
/// rim so it shows on any picture.
fn cursor_ring(ops: &mut Vec<Op>, cursor: &DoodleCursor, cx: &Context) {
    let centre = at(cursor.at, cx);
    let radius = cursor.radius * cx.display.y;
    let mut path = Path::new();
    for side in 0..=SIDES {
        let angle = TAU * side as f32 / SIDES as f32;
        let edge = Point::new(centre.x + radius * angle.cos(), centre.y + radius * angle.sin());
        if side == 0 {
            path.move_to(edge);
        } else {
            path.line_to(edge);
        }
    }
    path.close();
    let [r, g, b] = cursor.color;
    let colour = Rgba::from_rgba8(r, g, b, 1.0);
    ops.push(Op::Path { path: path.clone(), fill: None, rule: Rule::NonZero, stroke: Some(Stroke::new(RIM + 1.0, Rgba::BLACK)) });
    ops.push(Op::Path { path, fill: None, rule: Rule::NonZero, stroke: Some(Stroke::new(RIM, colour)) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::doodle::{Canvas, Pen};

    /// A brush or a spray is a few ops however long it is: each op is drawn on its own, so one
    /// per dot or per streak made long strokes slow to draw.
    #[test]
    fn a_long_brush_or_spray_stroke_is_a_few_ops() {
        for kind in [Kind::Brush, Kind::Spray] {
            let mut canvas = Canvas::default();
            canvas.begin_stroke(Pen { kind, size: 0.02, ..Pen::default() }, Share::new(0.1, 0.5));
            for i in 0..400 {
                canvas.extend_stroke(Share::new(0.1 + 0.002 * i as f32, 0.5 + 0.1 * (i as f32 / 20.0).sin()));
            }
            let frame = OverlayFrame { doodles: canvas, ..OverlayFrame::default() };
            let cx = Context::of(&frame, Point::new(1920.0, 1080.0));
            let mut ops = Vec::new();
            put(&mut ops, &frame, &cx);
            assert!(ops.len() <= 3, "{kind:?} draws {} ops", ops.len());
        }
    }
}
