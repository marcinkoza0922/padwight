use super::*;
use crate::tools::doodle::screen::CURSOR_MIN;
use crate::{
    config::OverlayStyle,
    keycode::KeyCode,
    overlay::DoodleCell,
    tools::doodle::{Kind, TextBox, Tip, text::Entry},
};

fn words() -> TextBox {
    TextBox { at: Point::new(0.1, 0.1), text: "hi".into(), style: TextStyle::default() }
}

fn press(doodle: &mut Doodle, canvas: &mut Canvas, b: Button) {
    let now = Instant::now();
    doodle.handle(canvas, b, true, now);
    doodle.handle(canvas, b, false, now);
}

fn enter() -> Vec<OverlayAction> {
    vec![OverlayAction::Key(KeyCode::KEY_ENTER, true)]
}

#[test]
fn a_stick_moves_the_cursor_only_while_it_is_pushed() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    doodle.axis(&mut canvas, Axis::LeftX, 1.0, now);
    doodle.tick(&mut canvas, 0.1, now);
    assert!((doodle.cursor().x - 0.62).abs() < 1e-4);
    doodle.axis(&mut canvas, Axis::LeftX, 0.05, now);
    assert!(!doodle.needs_tick());
}

#[test]
fn the_pen_draws_while_south_is_held_and_lifts_on_release() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.1, 0.5)), Canvas::default(), Instant::now());
    doodle.handle(&mut canvas, Button::South, true, now);
    doodle.axis(&mut canvas, Axis::LeftX, 1.0, now);
    doodle.tick(&mut canvas, 0.1, now);
    doodle.handle(&mut canvas, Button::South, false, now);
    assert_eq!(canvas.items().len(), 1);
    assert!(canvas.pending().is_none());
}

#[test]
fn the_eraser_removes_the_item_under_the_cursor() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    doodle.handle(&mut canvas, Button::South, true, now);
    doodle.handle(&mut canvas, Button::South, false, now);
    assert_eq!(canvas.items().len(), 1);
    doodle.handle(&mut canvas, Button::LeftBumper, true, now);
    doodle.handle(&mut canvas, Button::RightBumper, true, now);
    doodle.handle(&mut canvas, Button::RightBumper, true, now);
    assert_eq!(doodle.tool(), Tool::Eraser);
    doodle.handle(&mut canvas, Button::South, true, now);
    assert!(canvas.is_empty());
}

#[test]
fn a_tap_of_east_undoes_and_a_hold_leaves() {
    let (mut doodle, mut canvas, start) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    canvas.add_text(words());
    doodle.handle(&mut canvas, Button::East, true, start);
    assert_eq!(doodle.handle(&mut canvas, Button::East, false, start), Step::Stay);
    assert!(canvas.items().is_empty());

    doodle.handle(&mut canvas, Button::East, true, start);
    let held = start + EXIT_HOLD;
    assert_eq!(doodle.tick(&mut canvas, 0.0, held), Step::Leave);
}

#[test]
fn guide_leaves_the_screen() {
    let (mut doodle, mut canvas) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default());
    assert_eq!(doodle.handle(&mut canvas, Button::Guide, true, Instant::now()), Step::Leave);
}

#[test]
fn l3_and_r3_together_wipe_the_canvas() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    canvas.add_text(words());
    doodle.handle(&mut canvas, Button::LeftStick, true, now);
    doodle.handle(&mut canvas, Button::LeftStick, false, now);
    doodle.handle(&mut canvas, Button::RightStick, true, now);
    assert_eq!(canvas.items().len(), 1, "one click alone wipes nothing");
    doodle.handle(&mut canvas, Button::LeftStick, true, now);
    assert!(canvas.is_empty());
}

#[test]
fn the_d_pad_sets_the_pen_and_the_eraser_reach() {
    let (mut doodle, mut canvas) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default());
    let size = doodle.pen().size;
    press(&mut doodle, &mut canvas, Button::DpadUp);
    assert!(doodle.pen().size > size);
    press(&mut doodle, &mut canvas, Button::DpadLeft);
    assert_eq!(doodle.pen().color, 1, "left of the palette's third color");
    press(&mut doodle, &mut canvas, Button::North);
    assert_eq!(doodle.pen().tip, Tip::Square);
    press(&mut doodle, &mut canvas, Button::West);
    assert_eq!(doodle.pen().kind, Kind::Brush);
}

#[test]
fn north_steps_the_text_through_its_faces() {
    let (mut doodle, mut canvas) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default());
    doodle.handle(&mut canvas, Button::RightBumper, true, Instant::now());
    doodle.handle(&mut canvas, Button::RightBumper, true, Instant::now());
    assert_eq!(doodle.tool(), Tool::Text);
    press(&mut doodle, &mut canvas, Button::North);
    assert!(doodle.text().italic && !doodle.text().bold);
    press(&mut doodle, &mut canvas, Button::North);
    assert!(doodle.text().bold && !doodle.text().underline);
    press(&mut doodle, &mut canvas, Button::North);
    assert!(doodle.text().underline);
    press(&mut doodle, &mut canvas, Button::North);
    assert_eq!(doodle.text(), TextStyle::default());
}

/// Switches the Doodle screen to the text tool.
fn text_tool(doodle: &mut Doodle, canvas: &mut Canvas) {
    press(doodle, canvas, Button::RightBumper);
    press(doodle, canvas, Button::RightBumper);
    assert_eq!(doodle.tool(), Tool::Text);
}

#[test]
fn south_on_empty_space_opens_a_keyboard_for_a_new_box() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    text_tool(&mut doodle, &mut canvas);
    doodle.handle(&mut canvas, Button::South, true, now);
    doodle.handle(&mut canvas, Button::South, false, now);
    assert!(doodle.keyboard(now).is_some());
    doodle.finish_entry(&mut canvas, &enter());
    assert!(canvas.is_empty(), "an empty box is not kept");
}

#[test]
fn the_keyboard_takes_the_controller_and_the_stick_no_longer_moves_the_cursor() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    text_tool(&mut doodle, &mut canvas);
    doodle.handle(&mut canvas, Button::South, true, now);
    doodle.axis(&mut canvas, Axis::LeftX, 1.0, now);
    doodle.tick(&mut canvas, 0.1, now);
    assert_eq!(doodle.cursor(), Point::new(0.5, 0.5));
    assert!(canvas.is_empty());
}

#[test]
fn enter_sets_an_edited_box_and_empties_it_away() {
    let (mut doodle, mut canvas) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default());
    canvas.add_text(words());
    doodle.entry = Some(Entry::editing(0, Point::new(0.1, 0.1), "hey"));
    doodle.finish_entry(&mut canvas, &enter());
    assert_eq!(canvas.items().len(), 1);
    assert!(matches!(&canvas.items()[0], crate::tools::doodle::Item::Text(t) if t.text == "hey"));

    doodle.entry = Some(Entry::editing(0, Point::new(0.1, 0.1), ""));
    doodle.finish_entry(&mut canvas, &enter());
    assert!(canvas.is_empty());
}

#[test]
fn closing_the_keyboard_keeps_the_text_as_it_was() {
    let (mut doodle, mut canvas) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default());
    canvas.add_text(words());
    doodle.entry = Some(Entry::editing(0, Point::new(0.1, 0.1), "bye"));
    doodle.finish_entry(&mut canvas, &[OverlayAction::Close]);
    assert!(doodle.keyboard(Instant::now()).is_none());
    assert!(matches!(&canvas.items()[0], crate::tools::doodle::Item::Text(t) if t.text == "hi"));
}

#[test]
fn south_on_a_box_and_moving_it_carries_the_box() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.11, 0.11)), Canvas::default(), Instant::now());
    canvas.add_text(words());
    text_tool(&mut doodle, &mut canvas);
    doodle.handle(&mut canvas, Button::South, true, now);
    doodle.axis(&mut canvas, Axis::LeftX, 1.0, now);
    doodle.tick(&mut canvas, 0.1, now);
    doodle.handle(&mut canvas, Button::South, false, now);
    assert!(doodle.keyboard(now).is_none(), "a moved box isn't opened for editing");
    let crate::tools::doodle::Item::Text(text) = &canvas.items()[0] else { panic!("the box is still there") };
    assert!((text.at.x - 0.22).abs() < 1e-4);
}

#[test]
fn south_on_a_box_without_moving_it_opens_it_for_editing() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.11, 0.11)), Canvas::default(), Instant::now());
    canvas.add_text(words());
    text_tool(&mut doodle, &mut canvas);
    doodle.handle(&mut canvas, Button::South, true, now);
    doodle.handle(&mut canvas, Button::South, false, now);
    assert!(doodle.keyboard(now).is_some());
}

#[test]
fn the_hud_highlights_the_tool_and_hints_its_controls() {
    let (mut doodle, mut canvas) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default());
    let hud = doodle.hud(OverlayStyle::default());
    assert_eq!((hud.tool, hud.tray), (0, true));
    assert!(matches!(&hud.cells[0], DoodleCell::Palette { current: 2, .. }));
    assert!(matches!(&hud.cells[2], DoodleCell::Tip { tip: Tip::Round, .. }));
    press(&mut doodle, &mut canvas, Button::RightBumper);
    let hud = doodle.hud(OverlayStyle::default());
    assert_eq!(hud.tool, 1);
    assert_eq!(hud.cells.len(), 1);
    assert!(matches!(&hud.cells[0], DoodleCell::Text(text) if text.contains("{updown}")));
}

#[test]
fn the_tray_is_left_off_while_the_keyboard_is_up() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    text_tool(&mut doodle, &mut canvas);
    doodle.handle(&mut canvas, Button::South, true, now);
    assert!(!doodle.hud(OverlayStyle::default()).tray);
}

#[test]
fn the_cursor_is_there_even_while_the_keyboard_is_up() {
    let (mut doodle, mut canvas, now) = (Doodle::new(Point::new(0.5, 0.5)), Canvas::default(), Instant::now());
    text_tool(&mut doodle, &mut canvas);
    doodle.handle(&mut canvas, Button::South, true, now);
    assert_eq!(doodle.cursor_ring().at, Point::new(0.5, 0.5));
    assert!(doodle.cursor_ring().radius >= CURSOR_MIN, "the ring is never too small to see");
}

impl Doodle {
    fn cursor(&self) -> Point {
        self.cursor
    }

    fn tool(&self) -> Tool {
        self.tool
    }

    fn pen(&self) -> Pen {
        self.pen
    }

    fn text(&self) -> TextStyle {
        self.text
    }
}
