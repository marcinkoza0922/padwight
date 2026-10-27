//! Tests for the canvas and the tools' settings. The Doodle screen's controls are tested in `screen.rs`.

use super::*;

fn pen() -> Pen {
    Pen::default()
}

/// Draws a straight stroke from `from` to `to`, and lifts the pen.
fn line(canvas: &mut Canvas, from: Point, to: Point) {
    canvas.begin_stroke(pen(), from);
    canvas.extend_stroke(to);
    canvas.end_stroke();
}

fn text(at: Point, words: &str) -> TextBox {
    TextBox { at, text: words.into(), style: TextStyle::default() }
}

#[test]
fn tools_step_through_pen_eraser_and_text_and_wrap() {
    assert_eq!(Tool::Pen.next(), Tool::Eraser);
    assert_eq!(Tool::Text.next(), Tool::Pen);
    assert_eq!(Tool::Pen.previous(), Tool::Text);
    assert_eq!(Tool::Eraser.previous(), Tool::Pen);
}

#[test]
fn the_palette_has_twelve_colors_and_the_text_tool_eight_fonts() {
    assert_eq!(PALETTE.len(), 12);
    assert_eq!(FONTS.len(), 8);
}

#[test]
fn tip_and_kind_cycle_back_to_the_first() {
    assert_eq!(Tip::Diagonal.next(), Tip::Round);
    assert_eq!(Kind::Spray.next(), Kind::Solid);
}

#[test]
fn sizes_step_and_stay_within_their_limits() {
    assert!(resize(PEN_SIZE, 1, PEN_SIZES, SIZE_STEP) > PEN_SIZE);
    assert_eq!(resize(PEN_SIZE, -100, PEN_SIZES, SIZE_STEP), PEN_SIZES.0);
    assert_eq!(resize(PEN_SIZES.1, 5, PEN_SIZES, SIZE_STEP), PEN_SIZES.1);
    assert_eq!(resize(TEXT_SIZE, 1000, TEXT_SIZES, TEXT_STEP), TEXT_SIZES.1);
}

#[test]
fn a_stroke_is_an_item_only_once_the_pen_lifts() {
    let mut canvas = Canvas::default();
    canvas.begin_stroke(pen(), Point::new(0.0, 0.0));
    canvas.extend_stroke(Point::new(0.1, 0.0));
    assert!(canvas.items().is_empty());
    assert_eq!(canvas.pending().map(|s| s.points.len()), Some(2));
    canvas.end_stroke();
    assert_eq!(canvas.items().len(), 1);
    assert!(canvas.pending().is_none());
}

#[test]
fn erasing_removes_the_topmost_item_touched() {
    let mut canvas = Canvas::default();
    line(&mut canvas, Point::new(0.0, 0.5), Point::new(0.5, 0.5));
    line(&mut canvas, Point::new(0.0, 0.5), Point::new(0.5, 0.5));
    assert!(canvas.erase_at(Point::new(0.25, 0.502), 0.0));
    assert_eq!(canvas.items().len(), 1);
    assert!(!canvas.erase_at(Point::new(0.25, 0.9), 0.0), "nothing is that far from the line");
}

#[test]
fn the_eraser_reaches_as_far_as_its_radius() {
    let mut canvas = Canvas::default();
    line(&mut canvas, Point::new(0.0, 0.5), Point::new(0.5, 0.5));
    // The pen is 0.006 wide, so its edge is 0.003 off the line; a radius of 0.02 reaches 0.015.
    assert!(canvas.erase_at(Point::new(0.25, 0.515), 0.02));
}

#[test]
fn a_text_box_is_found_under_the_pointer_and_erased_like_a_stroke() {
    let mut canvas = Canvas::default();
    let index = canvas.add_text(text(Point::new(0.1, 0.1), "hello"));
    assert_eq!(canvas.text_at(Point::new(0.12, 0.115)), Some(index));
    assert_eq!(canvas.text_at(Point::new(0.9, 0.115)), None);
    assert!(canvas.erase_at(Point::new(0.12, 0.115), 0.0));
    assert!(canvas.items().is_empty());
}

#[test]
fn text_at_skips_strokes_so_south_on_a_stroke_makes_a_box() {
    let mut canvas = Canvas::default();
    line(&mut canvas, Point::new(0.0, 0.5), Point::new(0.5, 0.5));
    assert_eq!(canvas.text_at(Point::new(0.25, 0.5)), None);
}

#[test]
fn text_can_be_edited_in_place() {
    let mut canvas = Canvas::default();
    let index = canvas.add_text(text(Point::new(0.0, 0.0), "hi"));
    canvas.text_mut(index).unwrap().text.push('!');
    assert_eq!(canvas.items()[index], Item::Text(text(Point::new(0.0, 0.0), "hi!")));
}

#[test]
fn undo_takes_back_the_last_item_added() {
    let mut canvas = Canvas::default();
    line(&mut canvas, Point::new(0.0, 0.1), Point::new(0.1, 0.1));
    canvas.add_text(text(Point::new(0.0, 0.2), "x"));
    assert!(canvas.undo());
    assert_eq!(canvas.items().len(), 1);
    assert!(matches!(canvas.items()[0], Item::Stroke(_)));
    assert!(canvas.undo());
    assert!(!canvas.undo());
}

#[test]
fn wipe_clears_items_and_a_stroke_in_progress() {
    let mut canvas = Canvas::default();
    line(&mut canvas, Point::new(0.0, 0.1), Point::new(0.1, 0.1));
    canvas.begin_stroke(pen(), Point::new(0.5, 0.5));
    canvas.wipe();
    assert!(canvas.is_empty());
}
