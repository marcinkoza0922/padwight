//! On press and On release: taps beside an input's own action, as it goes down and comes up.

use std::time::{Duration, Instant};

use super::*;
use crate::{
    config::{Gestures, MouseButton},
    input::{Axis, InputEvent},
};

fn step(e: &mut Engine, p: &Profile, ev: InputEvent, at: Instant) -> Vec<OutEvent> {
    let mut out = Vec::new();
    e.handle(p, ev, at, &mut out);
    out
}

fn press(e: &mut Engine, p: &Profile, b: Button, down: bool, at: Instant) -> Vec<OutEvent> {
    step(e, p, InputEvent::Button(b, down), at)
}

fn right_click() -> Vec<OutEvent> {
    vec![OutEvent::MouseButton(MouseButton::Right, true), OutEvent::MouseButton(MouseButton::Right, false)]
}

/// The game's aim is a toggle on the right mouse button; tapping it on press and on release
/// makes it a hold, and the input's own action holds a finer-aim layer meanwhile.
fn held_toggle() -> Gestures {
    let click = ButtonAction::Mouse(MouseButton::Right);
    Gestures { on_press: Some(click.clone()), on_release: Some(click), ..Default::default() }
}

#[test]
fn a_toggle_becomes_a_hold_beside_the_buttons_layer() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::LeftBumper, ButtonAction::Layer("Aim".into()));
    p.gestures.insert(Button::LeftBumper, held_toggle());
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert_eq!(press(&mut e, &p, Button::LeftBumper, true, t0), right_click(), "fires at once, with no tap window");
    assert_eq!(e.layers(), vec!["Aim".to_string()]);
    assert_eq!(e.next_deadline(), None);
    assert_eq!(press(&mut e, &p, Button::LeftBumper, false, t0 + Duration::from_secs(2)), right_click());
    assert!(e.layers().is_empty());
}

#[test]
fn on_press_alone_does_nothing_more_while_held_or_on_release() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, ButtonAction::Disabled);
    p.gestures.insert(Button::North, Gestures { on_press: Some(ButtonAction::Keys(vec!["KEY_E".into()])), ..Default::default() });
    let mut e = Engine::default();
    let t0 = Instant::now();
    let tap = vec![OutEvent::Key(KeyCode::KEY_E, true), OutEvent::Key(KeyCode::KEY_E, false)];
    assert_eq!(press(&mut e, &p, Button::North, true, t0), tap);
    let mut out = Vec::new();
    e.timers(&p, t0 + Duration::from_secs(1), &mut out);
    assert!(out.is_empty());
    assert!(press(&mut e, &p, Button::North, false, t0 + Duration::from_secs(2)).is_empty());
}

#[test]
fn on_release_alone_fires_only_when_let_go() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, ButtonAction::Disabled);
    p.gestures.insert(Button::North, Gestures { on_release: Some(ButtonAction::Keys(vec!["KEY_G".into()])), ..Default::default() });
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert!(press(&mut e, &p, Button::North, true, t0).is_empty());
    let tap = vec![OutEvent::Key(KeyCode::KEY_G, true), OutEvent::Key(KeyCode::KEY_G, false)];
    assert_eq!(press(&mut e, &p, Button::North, false, t0 + Duration::from_millis(500)), tap);
}

#[test]
fn a_trigger_taps_on_its_pull_and_its_letting_go() {
    let mut p = Profile::passthrough("p");
    p.left_trigger = TriggerAction::Button { action: ButtonAction::Layer("Aim".into()), threshold: 0.5 }.into();
    p.left_trigger.gestures = held_toggle();
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert!(step(&mut e, &p, InputEvent::Axis(Axis::LeftTrigger, 0.3), t0).is_empty());
    assert_eq!(step(&mut e, &p, InputEvent::Axis(Axis::LeftTrigger, 0.9), t0), right_click());
    assert!(step(&mut e, &p, InputEvent::Axis(Axis::LeftTrigger, 1.0), t0).is_empty(), "once per pull");
    assert_eq!(e.layers(), vec!["Aim".to_string()]);
    assert_eq!(step(&mut e, &p, InputEvent::Axis(Axis::LeftTrigger, 0.0), t0), right_click());
    assert!(e.layers().is_empty());
}

#[test]
fn a_trigger_passed_to_the_pad_has_no_edges() {
    let mut p = Profile::passthrough("p");
    p.left_trigger.gestures = held_toggle();
    let mut e = Engine::default();
    let out = step(&mut e, &p, InputEvent::Axis(Axis::LeftTrigger, 1.0), Instant::now());
    assert!(!out.iter().any(|ev| matches!(ev, OutEvent::MouseButton(..))));
}

#[test]
fn on_press_does_not_wait_on_a_double_tap() {
    let mut p = Profile::passthrough("p");
    let mut g = held_toggle();
    g.double_tap = Some(ButtonAction::Keys(vec!["KEY_H".into()]));
    p.gestures.insert(Button::North, g);
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert_eq!(press(&mut e, &p, Button::North, true, t0), right_click());
    assert_eq!(press(&mut e, &p, Button::North, false, t0 + Duration::from_millis(50)), right_click());
    let mut second = press(&mut e, &p, Button::North, true, t0 + Duration::from_millis(100));
    assert_eq!(second.drain(..2).collect::<Vec<_>>(), right_click());
    assert_eq!(second, vec![OutEvent::Key(KeyCode::KEY_H, true)], "the double tap still fires");
}

#[test]
fn releasing_everything_lands_the_on_release_once() {
    let mut p = Profile::passthrough("p");
    p.gestures.insert(Button::LeftBumper, held_toggle());
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &p, Button::LeftBumper, true, t0);
    let mut out = Vec::new();
    e.release_all(false, &mut out);
    assert!(out.windows(2).any(|w| w == right_click().as_slice()), "the game's toggle goes back off");
    let out = press(&mut e, &p, Button::LeftBumper, false, t0 + Duration::from_secs(1));
    assert!(!out.iter().any(|ev| matches!(ev, OutEvent::MouseButton(..))));
}

#[test]
fn the_on_release_is_the_one_set_when_pressed() {
    let mut p = Profile::passthrough("p");
    p.gestures.insert(Button::LeftBumper, held_toggle());
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &p, Button::LeftBumper, true, t0);
    // A layer that came on meanwhile drops the gestures; the toggle still has to go back off.
    p.gestures.clear();
    assert_eq!(press(&mut e, &p, Button::LeftBumper, false, t0 + Duration::from_secs(1))[..2], right_click()[..]);
}
