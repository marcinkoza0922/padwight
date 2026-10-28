//! The doodles over the game: the canvas's strokes and text boxes, the cursor of the Doodle screen,
//! and its HUD. They go under the magnifying glass and every panel, so a menu is never drawn under
//! a doodle. Positions and sizes are shares of the screen (see `tools::doodle`), and are turned
//! into logical pixels here.

mod hud;
mod marks;

use std::{cell::RefCell, f32::consts::TAU};

use super::{
    Context,
    super::{Cap, Node, Op, Path, Point, Rect, Rgba, Rule, Stroke, text::Spec},
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

/// The ops the canvas's finished items were last drawn as, with what they were drawn from: they
/// change only when an item is added or removed, and building them each frame for every stroke
/// on the screen cost more than painting what the stroke being drawn changed.
struct Drawn {
    display: Point,
    font: Option<&'static str>,
    items: Vec<Item>,
    ops: Vec<Op>,
}

thread_local! {
    static DRAWN: RefCell<Option<Drawn>> = const { RefCell::new(None) };
}

/// Puts the doodles of `frame` on top of `ops`, on the display in `cx`.
pub(super) fn put(ops: &mut Vec<Op>, frame: &OverlayFrame, cx: &Context) {
    let items = frame.doodles.items();
    DRAWN.with_borrow_mut(|drawn| {
        let fresh = drawn.as_ref().is_some_and(|d| (d.display, d.font) == (cx.display, cx.font) && d.items == items);
        if !fresh {
            let mut built = Vec::new();
            for item in items {
                match item {
                    Item::Stroke(mark) => stroke(&mut built, mark, cx),
                    Item::Text(text) => text_box(&mut built, text, cx),
                }
            }
            *drawn = Some(Drawn { display: cx.display, font: cx.font, items: items.to_vec(), ops: built });
        }
        ops.extend(drawn.iter().flat_map(|d| d.ops.iter().cloned()));
    });
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
        Kind::Solid => marks::line(ops, &px, (width, color, cap(pen.tip)), |_, _| {}),
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

    fn ops_of(canvas: &Canvas) -> Vec<Op> {
        let frame = OverlayFrame { doodles: canvas.clone(), ..OverlayFrame::default() };
        let cx = Context::of(&frame, Point::new(1920.0, 1080.0));
        let mut ops = Vec::new();
        put(&mut ops, &frame, &cx);
        ops
    }

    /// A stroke is drawn in runs, so as it grows only the ops at its end change: the screen
    /// repaints only there, rather than the whole stroke's box each frame. Yet it is a run of
    /// segments to an op, not one op per dot or streak, which made long strokes slow to draw.
    #[test]
    fn a_growing_stroke_changes_only_its_last_ops() {
        for kind in [Kind::Solid, Kind::Brush, Kind::Spray] {
            for tip in [Tip::Round, Tip::Square] {
                let mut canvas = Canvas::default();
                canvas.begin_stroke(Pen { kind, tip, size: 0.02, ..Pen::default() }, Share::new(0.1, 0.5));
                for i in 0..400 {
                    canvas.extend_stroke(Share::new(0.1 + 0.002 * i as f32, 0.5 + 0.1 * (i as f32 / 20.0).sin()));
                }
                let points = canvas.pending().unwrap().points.len();
                let before = ops_of(&canvas);
                assert!(before.len() <= 3 * points / 16 + 6, "{kind:?} draws {} ops for {points} points", before.len());
                canvas.extend_stroke(Share::new(0.95, 0.55));
                let after = ops_of(&canvas);
                let same = before.iter().zip(&after).take_while(|(a, b)| a == b).count();
                assert!(before.len() - same <= 6, "{kind:?} {tip:?}: {} of {} ops changed", before.len() - same, before.len());
            }
        }
    }

    /// A wavy stroke across the screen, `n` points long, from height `y`.
    fn wave(canvas: &mut Canvas, pen: Pen, y: f32, n: usize) {
        canvas.begin_stroke(pen, Share::new(0.05, y));
        for i in 0..n {
            canvas.extend_stroke(Share::new(0.05 + 0.9 * i as f32 / n as f32, y + 0.08 * (i as f32 / 15.0).sin()));
        }
    }

    /// Times drawing a spray or brush stroke over a screen with a few finished ones, a frame
    /// for each tick the stick moves the cursor (run optimized: `--cargo-profile release`):
    /// building the ops, painting what changed, and sending the frame as JSON.
    #[test]
    #[ignore]
    #[expect(clippy::print_stdout, reason = "a benchmark reports its timing")]
    fn bench_drawing_over_doodles() {
        use std::time::{Duration, Instant};
        for kind in [Kind::Solid, Kind::Brush, Kind::Spray] {
            let mut canvas = Canvas::default();
            for (i, k) in [Kind::Solid, Kind::Brush, Kind::Spray, Kind::Brush, Kind::Spray, Kind::Solid].into_iter().enumerate() {
                wave(&mut canvas, Pen { kind: k, size: 0.02, ..Pen::default() }, 0.1 + 0.13 * i as f32, 1200);
                canvas.end_stroke();
            }
            let pen = Pen { kind, size: 0.02, ..Pen::default() };
            canvas.begin_stroke(pen, Share::new(0.05, 0.5));
            let mut screen = super::super::Screen::new();
            let mut buffer = tiny_skia::Pixmap::new(1920, 1080).unwrap();
            let (mut built, mut painted, mut sent) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
            let frames = 1500;
            let (mut area, mut count) = (0u64, 0usize);
            let (mut sent_items, mut kept_items) = (Vec::new(), Vec::new());
            for f in 0..frames {
                let at = Share::new(0.05 + 0.9 * f as f32 / frames as f32, 0.5 + 0.1 * (f as f32 / 40.0).sin());
                canvas.extend_stroke(at);
                let mut frame = OverlayFrame {
                    doodles: canvas.clone(),
                    doodle_cursor: Some(DoodleCursor { at, radius: 0.01, color: [255, 0, 0] }),
                    ..OverlayFrame::default()
                };
                let start = Instant::now();
                frame.leave_out_doodles_sent(&mut sent_items);
                let json = serde_json::to_string(&frame).unwrap();
                let mut frame: OverlayFrame = serde_json::from_str(&json).unwrap();
                frame.put_back_doodles(&mut kept_items);
                sent += start.elapsed();
                let start = Instant::now();
                let ops = super::super::screen_ops(&frame, &super::super::Shown::settled(&frame), Point::new(1920.0, 1080.0));
                built += start.elapsed();
                let start = Instant::now();
                let damage = screen.paint(&ops, 1.0, &mut buffer.as_mut(), 1);
                painted += start.elapsed();
                area += damage.patches().iter().map(|p| (p.width() * p.height()) as u64).sum::<u64>();
                count += ops.len();
            }
            let per = |d: Duration| d.as_secs_f64() * 1000.0 / frames as f64;
            println!("{kind:?}: build {:.3} ms, paint {:.3} ms, json {:.3} ms a frame; {} px repainted, {} ops", per(built), per(painted), per(sent), area / frames as u64, count / frames);
        }
    }
}
