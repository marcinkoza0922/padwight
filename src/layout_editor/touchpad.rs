//! The touchpad page of Edit Controls: what fingers do on the pad (or on each half of a split
//! pad), each kind's settings, and what clicking it does.

use crate::config::{
    ButtonAction, Config, CurvePreset, DEFAULT_SWIPE_DISTANCE, DEFAULT_TOUCHPAD_SPEED, SpeedCurve, Stick, SwipeDirection, TOUCHPAD_CURVE_SCALE, TouchChoose, TouchRegion,
    TouchSide, TouchSlot, TouchpadConfig, TouchpadMotion,
};

use super::{
    action::{ActionAt, Slot},
    inputs::swipe_name,
    owner::{self, Owner, Part, touchpad_mut},
    pages::{Page, stick_name},
    rows::{Number, Row, cycle, set},
    triggers::{back_to_profile, override_rows},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Off,
    Mouse,
    Joystick,
    Menu,
    Swipe,
}

const KINDS: [Kind; 5] = [Kind::Off, Kind::Mouse, Kind::Joystick, Kind::Menu, Kind::Swipe];

impl Kind {
    fn of(motion: &TouchpadMotion) -> Kind {
        match motion {
            TouchpadMotion::Off => Kind::Off,
            TouchpadMotion::Mouse { .. } => Kind::Mouse,
            TouchpadMotion::Joystick { .. } => Kind::Joystick,
            TouchpadMotion::Menu { .. } => Kind::Menu,
            TouchpadMotion::Swipe { .. } => Kind::Swipe,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Off => "Off",
            Kind::Mouse => "Mouse pointer",
            Kind::Joystick => "Gamepad stick",
            Kind::Menu => "Menu zones",
            Kind::Swipe => "Swipes",
        }
    }

    /// What the pad does when switched to this kind, as the settings window starts it.
    fn motion(self, first_menu: Option<String>) -> TouchpadMotion {
        match self {
            Kind::Off => TouchpadMotion::Off,
            Kind::Mouse => TouchpadMotion::mouse(DEFAULT_TOUCHPAD_SPEED),
            Kind::Joystick => TouchpadMotion::Joystick { stick: Stick::Right, relative: true, radius: 0.2, deadzone: 0.1 },
            Kind::Menu => TouchpadMotion::Menu { menu: first_menu.unwrap_or_default(), choose: TouchChoose::Click, overlay: true },
            Kind::Swipe => TouchpadMotion::Swipe {
                up: ButtonAction::Disabled,
                down: ButtonAction::Disabled,
                left: ButtonAction::Disabled,
                right: ButtonAction::Disabled,
                distance: DEFAULT_SWIPE_DISTANCE,
            },
        }
    }
}

const SPEED: Number = Number::new(50.0, (200.0, 4000.0), 0, " px across the pad");
const SLOW_GAIN: Number = Number::new(0.05, (0.1, 3.0), 2, "×");
const FAST_GAIN: Number = Number::new(0.05, (0.1, 4.0), 2, "×");
const SHAPE: Number = Number::new(0.05, (0.25, 4.0), 2, "");
const RADIUS: Number = Number::new(0.01, (0.05, 0.5), 2, " pad widths");
const DEADZONE: Number = Number::new(0.01, (0.0, 0.5), 2, "");
const DISTANCE: Number = Number::new(0.01, (0.1, 0.6), 2, " pad widths");

/// Changes what the fingers do on `side`.
fn motion(config: &mut Config, owner: Owner, side: TouchSide, change: impl FnOnce(&mut TouchpadMotion) -> bool) -> bool {
    touchpad_mut(config, owner).is_some_and(|t| t.sides().contains(&side) && change(t.motion_mut(side)))
}

/// A number of the mouse kind, changed by `field`.
fn mouse_num((owner, side): (Owner, TouchSide), label: &str, value: f32, n: &Number, field: fn(&mut f32, &mut SpeedCurve, f32) -> bool) -> Row {
    n.row(label, value, move |c, v| {
        motion(c, owner, side, |m| match m {
            TouchpadMotion::Mouse { speed, curve, .. } => field(speed, curve, v),
            _ => false,
        })
    })
}

/// A switch of the mouse kind: tap to click, two-finger scroll, or gliding.
fn mouse_switch(owner: Owner, side: TouchSide, label: &str, on: bool, field: fn(&mut bool, &mut bool, &mut bool, bool) -> bool) -> Row {
    Row::toggle(label, on, move |c, v| {
        motion(c, owner, side, |m| match m {
            TouchpadMotion::Mouse { tap, scroll, inertia, .. } => field(tap, scroll, inertia, v),
            _ => false,
        })
    })
}

/// The touchpad's page, or its right half's when it is split.
pub(super) fn touch_rows(config: &Config, owner: Owner, side: TouchSide) -> Vec<Row> {
    let Some(pad) = owner::touchpad(config, owner) else { return override_rows(owner, Part::Touchpad, "Touchpad") };
    if !pad.sides().contains(&side) {
        return Vec::new();
    }
    let m = pad.motion(side);
    let kind = Kind::of(m);
    let first_menu = config.active_game().menus.first().map(|x| x.name.clone());
    let mut rows = vec![Row::step(format!("Fingers: {}", kind.name()), move |c, dir| {
        let next = cycle(&KINDS, kind, dir).motion(first_menu.clone());
        motion(c, owner, side, |m| set(m, next))
    })];
    rows.extend(motion_rows(config, owner, side, m));
    let click = pad.click(side).clone();
    let off = if matches!(m, TouchpadMotion::Menu { .. }) { " (off the items)" } else { "" };
    rows.push(Row::open(format!("Click{off}: {}", click.summary()), Page::Action(ActionAt::new(Slot::Touch(owner, TouchSlot::Click(side))))).keyword(click.keyword()));
    if side == TouchSide::Left {
        rows.extend(split_rows(owner, pad));
        rows.extend(back_to_profile(owner, Part::Touchpad));
    }
    rows
}

/// Splitting the pad down the middle, and the right half's page.
fn split_rows(owner: Owner, pad: &TouchpadConfig) -> Vec<Row> {
    let split = pad.right.is_some();
    let mut rows = vec![Row::toggle("Split into halves", split, move |c, on| {
        touchpad_mut(c, owner).is_some_and(|t| set(&mut t.right, on.then_some(TouchRegion { motion: TouchpadMotion::Off, click: ButtonAction::Disabled })))
    })];
    if split {
        rows.push(Row::open("Right half…", Page::Touch(owner, TouchSide::Right)));
    }
    rows
}

fn motion_rows(config: &Config, owner: Owner, side: TouchSide, m: &TouchpadMotion) -> Vec<Row> {
    match m {
        TouchpadMotion::Off => Vec::new(),
        TouchpadMotion::Mouse { speed, curve, tap, scroll, inertia } => mouse_rows((owner, side), *speed, curve, (*tap, *scroll, *inertia)),
        &TouchpadMotion::Joystick { stick, relative, radius, deadzone } => joystick_rows((owner, side), stick, relative, (radius, deadzone)),
        TouchpadMotion::Menu { menu, choose, overlay } => menu_rows(config, (owner, side), menu, *choose, *overlay),
        TouchpadMotion::Swipe { distance, .. } => swipe_rows((owner, side), m, *distance),
    }
}

/// The pointer's speed and response, tapping, scrolling and gliding.
fn mouse_rows((owner, side): (Owner, TouchSide), speed: f32, curve: &SpeedCurve, (tap, scroll, inertia): (bool, bool, bool)) -> Vec<Row> {
    let preset = curve.preset(TOUCHPAD_CURVE_SCALE);
    let mut rows = vec![
        mouse_num((owner, side), "Speed", speed, &SPEED, |speed, _, v| set(speed, v)),
        Row::step(format!("Response: {preset}"), move |c, dir| {
            let next = cycle(&CurvePreset::NAMED, preset, dir).speed_curve(TOUCHPAD_CURVE_SCALE).unwrap_or_default();
            motion(c, owner, side, |m| match m {
                TouchpadMotion::Mouse { curve, .. } => set(curve, next),
                _ => false,
            })
        }),
    ];
    if !curve.is_flat() {
        rows.push(mouse_num((owner, side), "Slow sensitivity", curve.slow_gain, &SLOW_GAIN, |_, c, v| set(&mut c.slow_gain, v)));
        rows.push(mouse_num((owner, side), "Fast sensitivity", curve.fast_gain, &FAST_GAIN, |_, c, v| set(&mut c.fast_gain, v)));
        rows.push(mouse_num((owner, side), "Shape", curve.power, &SHAPE, |_, c, v| set(&mut c.power, v)));
    }
    rows.push(mouse_switch(owner, side, "Tap to click", tap, |tap, _, _, v| set(tap, v)));
    rows.push(mouse_switch(owner, side, "Two-finger scroll", scroll, |_, scroll, _, v| set(scroll, v)));
    rows.push(mouse_switch(owner, side, "Keep gliding after a flick", inertia, |_, _, inertia, v| set(inertia, v)));
    rows
}

/// The stick the finger pushes, where its center is, and how far a full push is.
fn joystick_rows((owner, side): (Owner, TouchSide), stick: Stick, relative: bool, (radius, deadzone): (f32, f32)) -> Vec<Row> {
    let joystick = move |c: &mut Config, next: TouchpadMotion| motion(c, owner, side, |m| set(m, next));
    let mut rows = vec![
        Row::step(format!("Sends: {}", stick_name(stick)), move |c, _| {
            let other = if stick == Stick::Left { Stick::Right } else { Stick::Left };
            joystick(c, TouchpadMotion::Joystick { stick: other, relative, radius, deadzone })
        }),
        Row::toggle("Center where the finger lands", relative, move |c, on| joystick(c, TouchpadMotion::Joystick { stick, relative: on, radius, deadzone })),
    ];
    if relative {
        rows.push(RADIUS.row("Full push at", radius, move |c, v| joystick(c, TouchpadMotion::Joystick { stick, relative, radius: v, deadzone })));
    }
    rows.push(DEADZONE.row("Deadzone", deadzone, move |c, v| joystick(c, TouchpadMotion::Joystick { stick, relative, radius, deadzone: v })));
    rows
}

/// The menu laid out on the pad, and how its items are chosen.
fn menu_rows(config: &Config, (owner, side): (Owner, TouchSide), menu: &str, choose: TouchChoose, overlay: bool) -> Vec<Row> {
    let names: Vec<String> = config.active_game().menus.iter().map(|x| x.name.clone()).collect();
    let entry = move |c: &mut Config, next: TouchpadMotion| motion(c, owner, side, |m| set(m, next));
    let mut rows = Vec::new();
    if names.is_empty() {
        rows.push(Row::note("Make a menu first, then pick it here"));
    } else {
        let now = menu.to_string();
        rows.push(Row::step(format!("Menu: {}", if menu.is_empty() { "none" } else { menu }), move |c, dir| {
            let at = names.iter().position(|n| *n == now).map_or(0, |i| (i as i32 + dir).rem_euclid(names.len() as i32) as usize);
            entry(c, TouchpadMotion::Menu { menu: names[at].clone(), choose, overlay })
        }));
    }
    let (m1, m2) = (menu.to_string(), menu.to_string());
    rows.push(Row::step(format!("Chosen by: {choose}"), move |c, dir| {
        entry(c, TouchpadMotion::Menu { menu: m1.clone(), choose: cycle(&TouchChoose::ALL, choose, dir), overlay })
    }));
    rows.push(Row::toggle("Shows the menu", overlay, move |c, on| entry(c, TouchpadMotion::Menu { menu: m2.clone(), choose, overlay: on })));
    rows
}

/// Each swipe's action, and how long a swipe is.
fn swipe_rows((owner, side): (Owner, TouchSide), m: &TouchpadMotion, distance: f32) -> Vec<Row> {
    let mut rows: Vec<Row> = SwipeDirection::ALL
        .into_iter()
        .map(|dir| {
            let action = m.swipe(dir).cloned().unwrap_or(ButtonAction::Disabled);
            Row::open(format!("Swipe {}: {}", swipe_name(dir), action.summary()), Page::Action(ActionAt::new(Slot::Touch(owner, TouchSlot::Swipe(side, dir))))).keyword(action.keyword())
        })
        .collect();
    rows.push(DISTANCE.row("Swipe length", distance, move |c, v| {
        motion(c, owner, side, |m| match m {
            TouchpadMotion::Swipe { distance, .. } => set(distance, v),
            _ => false,
        })
    }));
    rows
}
