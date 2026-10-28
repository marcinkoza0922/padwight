use super::*;
use crate::tools::doodle::Canvas;

/// Presses `b` on the menu.
fn press(session: &mut ToolsSession, b: Button) -> Option<Outcome> {
    session.handle(b, true, &mut Canvas::default(), Instant::now())
}

/// Moves the cursor to the keypad cell `index`, and presses it.
fn tap(session: &mut ToolsSession, index: usize) -> Option<Outcome> {
    session.cursor = index;
    press(session, Button::South)
}

fn title(session: &ToolsSession) -> String {
    session.view(OverlayStyle::default()).expect("the menu is up").title
}

/// The keypad cell at `row`, `column`.
fn at(row: usize, column: usize) -> usize {
    row * COLUMNS + column
}

#[test]
fn east_closes_the_list_and_south_opens_the_calculator() {
    let mut session = ToolsSession::new();
    assert_eq!(press(&mut session, Button::South), None);
    assert_eq!(title(&session), "Calculator");
    assert_eq!(press(&mut session, Button::East), None);
    assert_eq!(title(&session), "Tools");
    assert_eq!(press(&mut session, Button::East), Some(Outcome::Close));
}

#[test]
fn the_keypad_wraps_at_its_edges() {
    assert_eq!(pad_step(at(0, 0), (-1, 0)), at(0, COLUMNS - 1));
    assert_eq!(pad_step(at(0, 0), (0, -1)), at(PAD.len() / COLUMNS - 1, 0));
    assert_eq!(pad_step(at(1, 1), (1, 1)), at(2, 2));
}

#[test]
fn the_d_pad_moves_across_the_keypad() {
    let mut session = ToolsSession::new();
    press(&mut session, Button::South);
    press(&mut session, Button::DpadRight);
    assert_eq!(session.cursor(), 1);
    press(&mut session, Button::DpadDown);
    assert_eq!(session.cursor(), COLUMNS + 1);
}

#[test]
fn a_sum_is_keyed_on_the_pad_and_sent_to_the_game() {
    let mut session = ToolsSession::new();
    press(&mut session, Button::South);
    // 7 × 6 =, then Send
    for index in [0, 7, 6, 17] {
        assert_eq!(tap(&mut session, index), None);
    }
    assert_eq!(session.calc.line(), "42");
    assert_eq!(tap(&mut session, 18), Some(Outcome::Type("42".into())));
}

#[test]
fn copy_sends_the_answer_to_the_clipboard_and_an_empty_sum_says_so() {
    let mut session = ToolsSession::new();
    press(&mut session, Button::South);
    assert_eq!(tap(&mut session, 19), None);
    assert_eq!(session.calc.line(), "Nothing to send yet");
    tap(&mut session, 5);
    assert_eq!(tap(&mut session, 19), Some(Outcome::Copy("5".into())));
}

#[test]
fn a_sum_left_unfinished_is_sent_as_the_number_before_its_operator() {
    let mut session = ToolsSession::new();
    press(&mut session, Button::South);
    tap(&mut session, 5);
    tap(&mut session, 7);
    assert_eq!(tap(&mut session, 18), Some(Outcome::Type("5".into())));
}

#[test]
fn doodle_draws_over_the_game_until_guide_leaves_it() {
    let mut session = ToolsSession::new();
    let (mut canvas, now) = (Canvas::default(), Instant::now());
    session.cursor = TOOLS.iter().position(|&t| t == Tool::Doodle).unwrap();
    assert_eq!(session.handle(Button::South, true, &mut canvas, now), None);
    assert!(session.in_doodle());
    assert!(session.view(OverlayStyle::default()).is_none(), "the Doodle screen draws no menu");
    assert!(session.doodle_cursor().is_some());
    assert_eq!(session.handle(Button::Guide, true, &mut canvas, now), Some(Outcome::Close));
    assert!(!session.in_doodle());
}

#[test]
fn a_held_east_clears_doodle_on_a_tick_without_leaving() {
    let mut session = ToolsSession::new();
    let (mut canvas, start) = (Canvas::default(), Instant::now());
    session.cursor = TOOLS.iter().position(|&t| t == Tool::Doodle).unwrap();
    session.handle(Button::South, true, &mut canvas, start);
    session.handle(Button::South, true, &mut canvas, start);
    session.handle(Button::South, false, &mut canvas, start);
    assert!(session.in_doodle(), "a pen mark is not a leave");
    session.handle(Button::East, true, &mut canvas, start);
    assert!(!canvas.is_empty(), "the mark was drawn");
    session.handle(Button::East, true, &mut canvas, start);
    assert_eq!(session.tick(&mut canvas, 0.0, start + std::time::Duration::from_secs(5)), None);
    assert!(canvas.is_empty());
    assert!(session.in_doodle());
}

#[test]
fn the_calculator_keeps_its_sum_when_the_list_is_opened_again() {
    let mut session = ToolsSession::new();
    press(&mut session, Button::South);
    tap(&mut session, 2);
    press(&mut session, Button::East);
    press(&mut session, Button::South);
    assert_eq!(session.calc.line(), "9");
}
