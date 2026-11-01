use std::time::Instant;

use super::*;
use crate::{
    config::Button,
    input::{Axis, InputEvent},
    keyboard::Layout,
    keycode::KeyCode,
};

#[test]
fn frames_from_an_older_daemon_still_load() {
    let frame: OverlayFrame = serde_json::from_str(r#"{"info":[],"active":null}"#).unwrap();
    assert!(frame.logs.is_empty() && frame.info.is_empty() && frame.active.is_none());
    assert!(frame.motion.is_default(), "older frames have no motion, so the default");
}

fn at(c: &OverlayController) -> &'static str {
    keyboard::key_at(Layout::Keyboard, c.cursor()).code
}

fn ms(t: Instant, n: u64) -> Instant {
    t + Duration::from_millis(n)
}

fn start() -> OverlayController {
    OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_Q").unwrap())
}

#[test]
fn dpad_moves_once_then_repeats_while_held() {
    let mut c = start();
    let t0 = Instant::now();
    c.handle(InputEvent::Button(Button::DpadRight, true), t0);
    assert_eq!(at(&c), "KEY_W");
    let (delay, every) = (crate::nav::REPEAT_DELAY, crate::nav::REPEAT_EVERY);
    assert!(!c.tick(t0 + delay / 2).1, "no repeat before the delay");
    c.tick(t0 + delay);
    assert_eq!(at(&c), "KEY_E");
    c.tick(t0 + delay + every);
    c.tick(t0 + delay + every * 2);
    assert_eq!(at(&c), "KEY_T");
    c.handle(InputEvent::Button(Button::DpadRight, false), ms(t0, 600));
    c.tick(ms(t0, 2000));
    assert_eq!(at(&c), "KEY_T", "stops when released");
}

#[test]
fn stick_moves_like_a_dpad_with_hysteresis() {
    let mut c = start();
    let t0 = Instant::now();
    c.handle(InputEvent::Axis(Axis::LeftY, 0.3), t0);
    assert_eq!(at(&c), "KEY_Q", "a light push does nothing");
    c.handle(InputEvent::Axis(Axis::LeftY, 0.9), t0);
    assert_eq!(at(&c), "KEY_A");
    c.handle(InputEvent::Axis(Axis::LeftY, 0.5), t0);
    assert_eq!(at(&c), "KEY_A", "still held inside the release margin");
    c.handle(InputEvent::Axis(Axis::LeftY, 0.0), t0);
    c.handle(InputEvent::Axis(Axis::LeftY, 0.9), t0);
    assert_eq!(at(&c), "KEY_Z", "a fresh push moves again");
}

#[test]
fn a_holds_the_selected_key_and_modifiers_latch_for_one_key() {
    let mut c = OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_LEFTSHIFT").unwrap());
    let t0 = Instant::now();
    let shift = KeyCode::KEY_LEFTSHIFT;
    assert_eq!(c.handle(InputEvent::Button(Button::South, true), t0), vec![OverlayAction::Key(shift, true)]);
    assert!(c.handle(InputEvent::Button(Button::South, false), t0).is_empty(), "shift stays latched");
    assert_eq!(c.view(t0).latched, ["KEY_LEFTSHIFT"]);

    c.handle(InputEvent::Button(Button::DpadRight, true), t0);
    c.handle(InputEvent::Button(Button::DpadRight, false), t0);
    assert_eq!(at(&c), "KEY_Z");
    assert_eq!(c.handle(InputEvent::Button(Button::South, true), t0), vec![OverlayAction::Key(KeyCode::KEY_Z, true)]);
    assert_eq!(
        c.handle(InputEvent::Button(Button::South, false), t0),
        vec![OverlayAction::Key(KeyCode::KEY_Z, false), OverlayAction::Key(shift, false)],
        "capital Z, then shift lets go"
    );
    assert!(c.view(t0).latched.is_empty());
}

#[test]
fn left_trigger_holds_shift() {
    let mut c = start();
    let t0 = Instant::now();
    let shift = KeyCode::KEY_LEFTSHIFT;
    assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.8), t0), vec![OverlayAction::Key(shift, true)]);
    assert!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.9), t0).is_empty());
    assert_eq!(c.view(t0).latched, ["KEY_LEFTSHIFT"]);
    // Typing keeps it held: capitals until the trigger lets go.
    c.handle(InputEvent::Button(Button::South, true), t0);
    assert_eq!(c.handle(InputEvent::Button(Button::South, false), t0), vec![OverlayAction::Key(KeyCode::KEY_Q, false)]);
    assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.1), t0), vec![OverlayAction::Key(shift, false)]);
    assert!(c.view(t0).latched.is_empty());
    // Closing while held lets go of it.
    c.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0);
    assert_eq!(c.release_all(), vec![OverlayAction::Key(shift, false)]);
    // Not on the numpad.
    let mut n = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
    assert!(n.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0).is_empty());
}

#[test]
fn right_trigger_holds_ctrl() {
    let mut c = start();
    let t0 = Instant::now();
    let ctrl = KeyCode::KEY_LEFTCTRL;
    assert_eq!(c.handle(InputEvent::Axis(Axis::RightTrigger, 0.8), t0), vec![OverlayAction::Key(ctrl, true)]);
    assert_eq!(c.view(t0).latched, ["KEY_LEFTCTRL"]);
    c.handle(InputEvent::Button(Button::South, true), t0);
    assert_eq!(c.handle(InputEvent::Button(Button::South, false), t0), vec![OverlayAction::Key(KeyCode::KEY_Q, false)]);
    assert_eq!(c.handle(InputEvent::Axis(Axis::RightTrigger, 0.1), t0), vec![OverlayAction::Key(ctrl, false)]);
    c.handle(InputEvent::Axis(Axis::RightTrigger, 1.0), t0);
    assert_eq!(c.release_all(), vec![OverlayAction::Key(ctrl, false)]);
}

#[test]
fn a_latched_shift_coming_off_leaves_the_triggers_shift_down() {
    let mut c = OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_LEFTSHIFT").unwrap());
    let t0 = Instant::now();
    let shift = KeyCode::KEY_LEFTSHIFT;
    assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0), vec![OverlayAction::Key(shift, true)]);
    assert!(c.handle(InputEvent::Button(Button::South, true), t0).is_empty(), "already down");
    assert!(c.handle(InputEvent::Button(Button::South, true), t0).is_empty(), "unlatching keeps it down");
    assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.0), t0), vec![OverlayAction::Key(shift, false)]);
}

#[test]
fn a_late_tick_moves_one_key() {
    let mut c = start();
    let t0 = Instant::now();
    c.handle(InputEvent::Button(Button::DpadRight, true), t0);
    c.tick(ms(t0, 2000));
    assert_eq!(at(&c), "KEY_E");
}

#[test]
fn holding_x_keeps_deleting_and_the_bumpers_move_the_text_cursor() {
    let mut c = start();
    let t0 = Instant::now();
    let tap = |k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)];
    assert_eq!(c.handle(InputEvent::Button(Button::West, true), t0), tap(KeyCode::KEY_BACKSPACE));
    assert!(c.tick(t0 + crate::nav::REPEAT_DELAY / 2).0.is_empty());
    assert_eq!(c.tick(t0 + crate::nav::REPEAT_DELAY).0, tap(KeyCode::KEY_BACKSPACE));
    c.handle(InputEvent::Button(Button::West, false), t0 + crate::nav::REPEAT_DELAY);
    assert_eq!(c.next_deadline(), None);
    assert_eq!(c.handle(InputEvent::Button(Button::LeftBumper, true), t0), tap(KeyCode::KEY_LEFT));
    assert_eq!(c.handle(InputEvent::Button(Button::RightBumper, true), t0), tap(KeyCode::KEY_RIGHT));
    let mut n = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
    assert!(n.handle(InputEvent::Button(Button::LeftBumper, true), t0).is_empty(), "no text cursor keys on the numpad");
}

#[test]
fn the_keyboard_counts_its_caps_lock_presses_and_the_full_board_has_more_keys() {
    let mut c = OverlayController::new(Board::Basic, keyboard::find(Board::Basic, "KEY_CAPSLOCK").unwrap());
    let t0 = Instant::now();
    assert!(!c.view(t0).caps);
    c.handle(InputEvent::Button(Button::South, true), t0);
    c.handle(InputEvent::Button(Button::South, false), t0);
    assert!(c.view(t0).caps);
    assert!(keyboard::find(Board::Basic, "KEY_F1").is_none() && keyboard::find(Board::Full, "KEY_F1").is_some());
    assert!(keyboard::find(Board::Basic, "KEY_LEFT").is_some(), "the basic board has the arrows");
    assert!(keyboard::find(Board::Numpad, "KEY_KPMINUS").is_some(), "and the numpad a minus");
}

#[test]
fn the_keyboard_follows_what_it_types() {
    let mut c = OverlayController::new(Board::Basic, keyboard::find(Board::Basic, "KEY_H").unwrap());
    let t0 = Instant::now();
    let press = |c: &mut OverlayController, b| {
        c.handle(InputEvent::Button(b, true), t0);
        c.handle(InputEvent::Button(b, false), t0);
    };
    press(&mut c, Button::South);
    c.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0);
    press(&mut c, Button::South);
    c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.0), t0);
    press(&mut c, Button::North);
    let typed = c.view(t0).typed;
    // The labels come from the desktop's layout; H is h in every Latin one.
    assert!(typed.eq_ignore_ascii_case("hh ") && typed.starts_with('h'), "{typed:?}");
    press(&mut c, Button::West);
    assert_eq!(c.view(t0).typed.chars().count(), 2);
    press(&mut c, Button::Start);
    assert!(c.view(t0).typed.is_empty(), "Enter starts afresh");
}

#[test]
fn face_button_shortcuts() {
    let mut c = start();
    let t0 = Instant::now();
    let tap = |k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)];
    assert_eq!(c.handle(InputEvent::Button(Button::West, true), t0), tap(KeyCode::KEY_BACKSPACE));
    assert_eq!(c.handle(InputEvent::Button(Button::North, true), t0), tap(KeyCode::KEY_SPACE));
    assert_eq!(c.handle(InputEvent::Button(Button::Start, true), t0), [tap(KeyCode::KEY_ENTER), vec![OverlayAction::Close]].concat());
}

#[test]
fn enter_closes_the_keyboard_but_not_the_numpad() {
    let t0 = Instant::now();
    let close = |mut v: Vec<OverlayAction>| v.pop() == Some(OverlayAction::Close);
    let mut c = start();
    assert!(close(c.handle(InputEvent::Button(Button::Start, true), t0)), "Start sends Enter and closes");
    let mut n = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
    assert!(!close(n.handle(InputEvent::Button(Button::Start, true), t0)), "the numpad stays up");
}

#[test]
fn guide_with_x_closes_the_keyboard_and_with_y_the_numpad() {
    let t0 = Instant::now();
    let mut keyboard = OverlayController::new(Layout::Keyboard, Layout::Keyboard.home());
    keyboard.handle(InputEvent::Button(Button::Guide, true), t0);
    assert_eq!(keyboard.handle(InputEvent::Button(Button::West, true), t0), vec![OverlayAction::Close]);
    let mut keyboard = OverlayController::new(Layout::Keyboard, Layout::Keyboard.home());
    assert_eq!(keyboard.handle(InputEvent::Button(Button::West, true), t0).len(), 2, "X alone is backspace");

    let mut numpad = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
    numpad.set_guide(true, Some(Button::Guide), Some(Button::North));
    assert!(!numpad.handle(InputEvent::Button(Button::West, true), t0).contains(&OverlayAction::Close), "X doesn't close the numpad");
    assert_eq!(numpad.handle(InputEvent::Button(Button::North, true), t0), vec![OverlayAction::Close]);
    numpad.handle(InputEvent::Button(Button::Guide, false), t0);
    assert!(numpad.handle(InputEvent::Button(Button::North, true), t0).is_empty());
}

#[test]
fn paste_is_ctrl_v_once_and_types_no_text() {
    let mut c = OverlayController::new(Board::Basic, keyboard::find(Board::Basic, keyboard::PASTE).unwrap());
    let t0 = Instant::now();
    let (ctrl, v) = (KeyCode::KEY_LEFTCTRL, KeyCode::KEY_V);
    assert_eq!(
        c.handle(InputEvent::Button(Button::South, true), t0),
        vec![OverlayAction::Key(ctrl, true), OverlayAction::Key(v, true), OverlayAction::Key(v, false), OverlayAction::Key(ctrl, false)]
    );
    assert!(c.handle(InputEvent::Button(Button::South, false), t0).is_empty(), "nothing is held down");
    assert_eq!(c.typed(), "", "pasting is not typing");
}

#[test]
fn typing_presses_the_keys_that_type_each_character() {
    assert!(typing("").is_empty());
    assert!(typing("\u{1F600}").is_empty(), "no key types it");
    // One key, or Shift and a key, each pressed and let go.
    assert!([2, 4].contains(&typing("5").len()));
    assert!(typing("42").len() >= 4);
}

#[test]
fn numpad_types_digits_and_enter_and_closes_with_b() {
    let mut c = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
    let t0 = Instant::now();
    let tap = |k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)];
    c.handle(InputEvent::Button(Button::DpadUp, true), t0);
    c.handle(InputEvent::Button(Button::DpadUp, false), t0);
    assert_eq!(c.handle(InputEvent::Button(Button::South, true), t0), vec![OverlayAction::Key(KeyCode::KEY_8, true)]);
    assert_eq!(c.handle(InputEvent::Button(Button::South, false), t0), vec![OverlayAction::Key(KeyCode::KEY_8, false)]);
    assert_eq!(c.handle(InputEvent::Button(Button::Start, true), t0), tap(KeyCode::KEY_ENTER));
    assert!(c.handle(InputEvent::Button(Button::North, true), t0).is_empty(), "no space bar on a numpad");
    assert_eq!(c.view(t0).board, Board::Numpad);
    c.handle(InputEvent::Button(Button::East, true), t0);
    assert_eq!(c.tick(ms(t0, 700)).0, vec![OverlayAction::Close]);
}

#[test]
fn holding_east_closes_but_a_tap_does_not() {
    let mut c = start();
    let t0 = Instant::now();
    c.handle(InputEvent::Button(Button::East, true), t0);
    assert!(c.view(ms(t0, 350)).closing > 0.4);
    c.handle(InputEvent::Button(Button::East, false), ms(t0, 400));
    assert_eq!(c.tick(ms(t0, 2000)).0, vec![]);
    assert_eq!(c.view(ms(t0, 2000)).closing, 0.0);

    c.handle(InputEvent::Button(Button::East, true), ms(t0, 3000));
    assert_eq!(c.next_deadline().map(|d| d <= ms(t0, 3700)), Some(true));
    assert_eq!(c.tick(ms(t0, 3700)).0, vec![OverlayAction::Close]);
}

#[test]
fn closing_releases_everything_held() {
    let mut c = OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_LEFTCTRL").unwrap());
    let t0 = Instant::now();
    c.handle(InputEvent::Button(Button::South, true), t0);
    c.handle(InputEvent::Button(Button::South, false), t0);
    c.handle(InputEvent::Button(Button::DpadUp, true), t0);
    assert!(at(&c) != "KEY_LEFTCTRL");
    c.handle(InputEvent::Button(Button::South, true), t0);
    let out = c.release_all();
    assert_eq!(out.len(), 2, "{out:?}");
    assert!(out.contains(&OverlayAction::Key(KeyCode::KEY_LEFTCTRL, false)));
    assert_eq!(c.next_deadline(), None);
}

#[test]
fn the_bumpers_and_the_arrow_keys_move_the_caret_in_the_typed_line() {
    let t0 = Instant::now();
    let mut c = OverlayController::for_field("Name", "abc");
    assert_eq!(c.view(t0).caret, 3);
    c.handle(InputEvent::Button(Button::LeftBumper, true), t0);
    c.handle(InputEvent::Button(Button::LeftBumper, true), t0);
    assert_eq!(c.view(t0).caret, 1);
    // Typing and deleting happen at the caret.
    c.handle(InputEvent::Button(Button::West, true), t0);
    assert_eq!((c.typed(), c.view(t0).caret), ("bc", 0));
    c.handle(InputEvent::Button(Button::RightBumper, true), t0);
    c.handle(InputEvent::Button(Button::RightBumper, true), t0);
    c.handle(InputEvent::Button(Button::RightBumper, true), t0);
    assert_eq!(c.view(t0).caret, 2, "stops at the end");
    c.handle(InputEvent::Button(Button::North, true), t0);
    assert_eq!((c.typed(), c.view(t0).caret), ("bc ", 3));
}
