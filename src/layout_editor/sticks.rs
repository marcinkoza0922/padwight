//! The stick pages of Edit Controls: what a stick does (a gamepad stick, the mouse, the wheel,
//! a ring of buttons, a flick stick) with each kind's settings, its deadzone and zones, and a
//! ring's sectors.

use crate::config::{Analog, ButtonAction, Config, CurvePreset, DeadzoneShape, FlickVertical, MouseResponse, Stick, StickAction, StickConfig};

use super::{
    action::{ActionAt, Slot},
    owner::{self, Owner, Part, stick_mut},
    pages::{Page, stick_name},
    rows::{Number, Row, counted, cycle, set},
    triggers::{back_to_profile, override_rows},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Disabled,
    Gamepad,
    Mouse,
    Scroll,
    Ring,
    Flick,
}

const KINDS: [Kind; 6] = [Kind::Disabled, Kind::Gamepad, Kind::Mouse, Kind::Scroll, Kind::Ring, Kind::Flick];

impl Kind {
    fn of(action: &StickAction) -> Kind {
        match action {
            StickAction::Disabled => Kind::Disabled,
            StickAction::Gamepad { .. } => Kind::Gamepad,
            StickAction::Mouse { .. } => Kind::Mouse,
            StickAction::Scroll { .. } => Kind::Scroll,
            StickAction::Ring { .. } => Kind::Ring,
            StickAction::Flick { .. } => Kind::Flick,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Disabled => "Off",
            Kind::Gamepad => "Gamepad stick",
            Kind::Mouse => "Mouse pointer",
            Kind::Scroll => "Scroll wheel",
            Kind::Ring => "Button ring",
            Kind::Flick => "Flick stick",
        }
    }

    /// The action a stick switched to this kind starts with.
    fn action(self, s: Stick) -> StickAction {
        match self {
            Kind::Disabled => StickAction::Disabled,
            Kind::Gamepad => StickAction::Gamepad { stick: s, invert_y: false },
            Kind::Mouse => StickAction::mouse(1200.0),
            Kind::Scroll => StickAction::Scroll { speed: 15.0, invert_y: false },
            Kind::Ring => StickAction::ring(8),
            Kind::Flick => StickAction::flick(),
        }
    }
}

const DEADZONE: Number = Number::new(0.01, (0.0, 0.5), 2, "");
const FRACTION: Number = Number::new(0.05, (0.05, 0.95), 2, "");
const CURVE: Number = Number::new(0.05, (1.0, 3.0), 2, "");
const MOUSE_SPEED: Number = Number::new(50.0, (100.0, 4000.0), 0, " px/s");
const SCROLL_SPEED: Number = Number::new(1.0, (1.0, 60.0), 0, " notches/s");
const ACCEL: Number = Number::new(5.0, (0.0, 100.0), 0, "% faster");
const RAMP: Number = Number::new(50.0, (100.0, 2000.0), 0, " ms");
const BOOST: Number = Number::new(0.05, (0.0, 1.0), 2, "");
const Y_SCALE: Number = Number::new(0.05, (0.25, 2.0), 2, "×");
const SMOOTHING: Number = Number::new(5.0, (0.0, 200.0), 0, " ms");
const START_ANGLE: Number = Number::new(15.0, (0.0, 345.0), 0, "°");
const STICKINESS: Number = Number::new(0.05, (0.0, 0.4), 2, "");
const TURN: Number = Number::new(100.0, (1000.0, 30_000.0), 0, " px");
const FLICK_AT: Number = Number::new(0.05, (0.5, 1.0), 2, "");
const FLICK_TIME: Number = Number::new(10.0, (0.0, 300.0), 0, " ms");
const DEAD_ANGLE: Number = Number::new(1.0, (0.0, 30.0), 0, "°");
const VERTICAL_SPEED: Number = Number::new(50.0, (100.0, 4000.0), 0, " px/s");

/// The owner's two sticks.
pub(super) fn sticks_rows(config: &Config, owner: Owner) -> Vec<Row> {
    [Stick::Left, Stick::Right]
        .into_iter()
        .map(|s| {
            let now = owner::stick(config, owner, s).map_or("same as profile".into(), |c| Kind::of(&c.action).name().to_string());
            Row::open(format!("{}: {now}", stick_name(s)), Page::Stick(owner, s))
        })
        .collect()
}

/// A number of a stick's config, changed by `field`.
fn num((owner, s): (Owner, Stick), label: &str, value: f32, n: &Number, field: fn(&mut StickConfig, f32) -> bool) -> Row {
    n.row(label, value, move |c, v| stick_mut(c, owner, s).is_some_and(|cfg| field(cfg, v)))
}

/// A switch of a stick's config, changed by `field`.
fn switch(owner: Owner, s: Stick, label: &str, on: bool, field: fn(&mut StickConfig, bool) -> bool) -> Row {
    Row::toggle(label, on, move |c, v| stick_mut(c, owner, s).is_some_and(|cfg| field(cfg, v)))
}

/// A stick's page: its kind, the kind's settings, then what every kind has.
pub(super) fn stick_rows(config: &Config, owner: Owner, s: Stick) -> Vec<Row> {
    let Some(cfg) = owner::stick(config, owner, s) else { return override_rows(owner, Part::Stick(s), &stick_name(s)) };
    let kind = Kind::of(&cfg.action);
    let mut rows = vec![Row::step(format!("Kind: {}", kind.name()), move |c, dir| {
        let next = cycle(&KINDS, kind, dir);
        stick_mut(c, owner, s).is_some_and(|cfg| set(&mut cfg.action, next.action(s)))
    })];
    rows.extend(match &cfg.action {
        StickAction::Disabled => Vec::new(),
        StickAction::Gamepad { stick, invert_y } => {
            let sends = *stick;
            vec![
                Row::step(format!("Sends: {}", stick_name(sends)), move |c, _| {
                    let other = if sends == Stick::Left { Stick::Right } else { Stick::Left };
                    stick_mut(c, owner, s).is_some_and(|cfg| match &mut cfg.action {
                        StickAction::Gamepad { stick, .. } => set(stick, other),
                        _ => false,
                    })
                }),
                switch(owner, s, "Invert Y", *invert_y, set_invert),
                num((owner, s), "Curve", cfg.curve, &CURVE, |c, v| set(&mut c.curve, v)),
            ]
        }
        StickAction::Mouse { speed, response, invert_y } => mouse_rows((owner, s), cfg, (*speed, response, *invert_y)),
        StickAction::Scroll { speed, invert_y } => vec![
            num((owner, s), "Speed", *speed, &SCROLL_SPEED, |c, v| match &mut c.action {
                StickAction::Scroll { speed, .. } => set(speed, v),
                _ => false,
            }),
            switch(owner, s, "Invert Y", *invert_y, set_invert),
            num((owner, s), "Curve", cfg.curve, &CURVE, |c, v| set(&mut c.curve, v)),
        ],
        StickAction::Ring { sectors, start_angle, inner_radius, hysteresis, .. } => ring_settings((owner, s), *sectors, (*start_angle, *inner_radius, *hysteresis)),
        StickAction::Flick { .. } => flick_rows(owner, s, &cfg.action),
    });
    rows.push(num((owner, s), "Deadzone", cfg.deadzone, &DEADZONE, |c, v| set(&mut c.deadzone, v)));
    rows.push(Row::step(format!("Deadzone shape: {}", cfg.deadzone_shape), move |c, dir| {
        stick_mut(c, owner, s).is_some_and(|sc| {
            let next = cycle(&DeadzoneShape::ALL, sc.deadzone_shape, dir);
            set(&mut sc.deadzone_shape, next)
        })
    }));
    rows.push(num((owner, s), "Anti-deadzone", cfg.anti_deadzone, &DEADZONE, |c, v| set(&mut c.anti_deadzone, v)));
    rows.push(num((owner, s), "Anti-deadzone hold", cfg.anti_deadzone_buffer, &DEADZONE, |c, v| set(&mut c.anti_deadzone_buffer, v)));
    rows.push(switch(owner, s, "Invert X", cfg.invert_x, |c, on| set(&mut c.invert_x, on)));
    rows.push(num((owner, s), "Rotate output", cfg.rotate, &START_ANGLE, |c, v| set(&mut c.rotate, v)));
    rows.push(num((owner, s), "Directions press at", cfg.key_threshold, &FRACTION, |c, v| set(&mut c.key_threshold, v)));
    rows.push(Row::open(counted("Zones", cfg.zones.len()), Page::Zones(owner, Analog::Stick(s))));
    rows.extend(back_to_profile(owner, Part::Stick(s)));
    rows
}

fn set_invert(cfg: &mut StickConfig, on: bool) -> bool {
    match &mut cfg.action {
        StickAction::Gamepad { invert_y, .. } | StickAction::Mouse { invert_y, .. } | StickAction::Scroll { invert_y, .. } => set(invert_y, on),
        _ => false,
    }
}

/// Changes a mouse stick's response.
fn response(cfg: &mut StickConfig, change: impl FnOnce(&mut MouseResponse) -> bool) -> bool {
    match &mut cfg.action {
        StickAction::Mouse { response, .. } => change(response),
        _ => false,
    }
}

/// A mouse stick's speed and response: a preset, its settings when it isn't linear, and the
/// vertical speed and smoothing no preset sets.
fn mouse_rows((owner, s): (Owner, Stick), cfg: &StickConfig, (speed, r, invert_y): (f32, &MouseResponse, bool)) -> Vec<Row> {
    let preset = CurvePreset::of_stick(cfg.curve, r);
    let mut rows = vec![
        num((owner, s), "Speed", speed, &MOUSE_SPEED, |c, v| match &mut c.action {
            StickAction::Mouse { speed, .. } => set(speed, v),
            _ => false,
        }),
        switch(owner, s, "Invert Y", invert_y, set_invert),
        Row::step(format!("Response: {preset}"), move |c, dir| {
            let next = cycle(&CurvePreset::NAMED, preset, dir);
            stick_mut(c, owner, s).is_some_and(|cfg| {
                let StickAction::Mouse { response, .. } = cfg.action else { return false };
                let Some((curve, response)) = next.stick_response(response) else { return false };
                cfg.curve = curve;
                self::response(cfg, |r| set(r, response))
            })
        }),
    ];
    if preset != CurvePreset::Linear {
        rows.push(num((owner, s), "Curve", cfg.curve, &CURVE, |c, v| set(&mut c.curve, v)));
        rows.push(num((owner, s), "Acceleration", r.accel * 100.0, &ACCEL, |c, v| response(c, |r| set(&mut r.accel, v / 100.0))));
        if r.accel > 0.0 {
            rows.push(num((owner, s), "Ramp time", r.accel_ramp_ms as f32, &RAMP, |c, v| response(c, |r| set(&mut r.accel_ramp_ms, v as u32))));
        }
        rows.push(num((owner, s), "Outer boost", r.outer_boost, &BOOST, |c, v| response(c, |r| set(&mut r.outer_boost, v))));
    }
    rows.push(num((owner, s), "Vertical speed", r.y_scale, &Y_SCALE, |c, v| response(c, |r| set(&mut r.y_scale, v))));
    rows.push(num((owner, s), "Smoothing", r.smoothing_ms as f32, &SMOOTHING, |c, v| response(c, |r| set(&mut r.smoothing_ms, v as u32))));
    rows
}

/// Changes a ring's settings, keeping one action per sector.
fn ring(cfg: &mut StickConfig, change: impl FnOnce(&mut u8, &mut f32, &mut f32, &mut f32) -> bool) -> bool {
    let StickAction::Ring { sectors, start_angle, inner_radius, hysteresis, actions } = &mut cfg.action else { return false };
    let changed = change(sectors, start_angle, inner_radius, hysteresis);
    actions.resize(usize::from(*sectors), ButtonAction::Disabled);
    changed
}

fn ring_settings((owner, s): (Owner, Stick), sectors: u8, (start_angle, inner_radius, hysteresis): (f32, f32, f32)) -> Vec<Row> {
    vec![
        Row::step(format!("Sectors: {sectors}"), move |c, dir| {
            let next = cycle(&[4_u8, 8, 12], sectors, dir);
            stick_mut(c, owner, s).is_some_and(|cfg| ring(cfg, |n, _, _, _| set(n, next)))
        }),
        Row::open(format!("Sector actions ({sectors})"), Page::Ring(owner, s)),
        num((owner, s), "First sector at", start_angle, &START_ANGLE, |c, v| ring(c, |_, a, _, _| set(a, v))),
        num((owner, s), "Starts at", inner_radius, &FRACTION, |c, v| ring(c, |_, _, r, _| set(r, v))),
        num((owner, s), "Stickiness", hysteresis, &STICKINESS, |c, v| ring(c, |_, _, _, h| set(h, v))),
        Row::apply("Fill with WASD", move |c| super::rows::Act::changed_if(fill_ring(c, owner, s, StickAction::wasd()))),
        Row::apply("Fill with the arrows", move |c| super::rows::Act::changed_if(fill_ring(c, owner, s, StickAction::arrows()))),
    ]
}

/// Sets a ring's sectors to a preset's, keeping where it starts.
fn fill_ring(config: &mut Config, owner: Owner, s: Stick, preset: StickAction) -> bool {
    let Some(cfg) = stick_mut(config, owner, s) else { return false };
    let StickAction::Ring { inner_radius: radius, .. } = cfg.action else { return false };
    set(&mut cfg.action, preset.starting_at(radius))
}

/// A ring's sectors, each with the direction it points and its action.
pub(super) fn ring_rows(config: &Config, owner: Owner, s: Stick) -> Vec<Row> {
    let Some(StickAction::Ring { sectors, start_angle, actions, .. }) = owner::stick(config, owner, s).map(|c| &c.action) else { return Vec::new() };
    let width = 360.0 / f32::from((*sectors).max(1));
    (0..usize::from(*sectors))
        .map(|i| {
            let angle = start_angle + i as f32 * width;
            let arrow = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"][((angle / 45.0).round() as usize) % 8];
            let action = actions.get(i).cloned().unwrap_or(ButtonAction::Disabled);
            Row::open(format!("{arrow} {}", action.summary()), Page::Action(ActionAt::new(Slot::Ring(owner, s, i)))).keyword(action.keyword())
        })
        .collect()
}

/// Changes one of a flick stick's settings.
fn flick(cfg: &mut StickConfig, change: impl FnOnce(&mut StickAction) -> bool) -> bool {
    matches!(cfg.action, StickAction::Flick { .. }) && change(&mut cfg.action)
}

/// A field of a flick stick, by name.
macro_rules! flick_num {
    ($owner:expr, $s:expr, $label:expr, $value:expr, $n:expr, $field:ident, $from:ty) => {
        num(($owner, $s), $label, $value as f32, $n, |c, v| {
            flick(c, |a| match a {
                StickAction::Flick { $field, .. } => set($field, v as $from),
                _ => false,
            })
        })
    };
}

fn flick_rows(owner: Owner, s: Stick, action: &StickAction) -> Vec<Row> {
    let StickAction::Flick { full_turn_px, flick_threshold, flick_time_ms, rotate_smoothing_ms, forward_deadzone, vertical, vertical_speed } = *action else {
        return Vec::new();
    };
    let look = vertical == FlickVertical::Look;
    let mut rows = vec![
        flick_num!(owner, s, "Turn size", full_turn_px, &TURN, full_turn_px, f32),
        flick_num!(owner, s, "Flick at", flick_threshold, &FLICK_AT, flick_threshold, f32),
        flick_num!(owner, s, "Flick time", flick_time_ms, &FLICK_TIME, flick_time_ms, u32),
        flick_num!(owner, s, "Turning smoothing", rotate_smoothing_ms, &SMOOTHING, rotate_smoothing_ms, u32),
        flick_num!(owner, s, "Dead angle", forward_deadzone, &DEAD_ANGLE, forward_deadzone, f32),
        switch(owner, s, "Up and down look", look, |c, on| {
            flick(c, |a| match a {
                StickAction::Flick { vertical, .. } => set(vertical, if on { FlickVertical::Look } else { FlickVertical::Off }),
                _ => false,
            })
        }),
    ];
    if look {
        rows.push(flick_num!(owner, s, "Vertical speed", vertical_speed, &VERTICAL_SPEED, vertical_speed, f32));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_name_and_starts_as_that_kind() {
        for kind in KINDS {
            assert_eq!(Kind::of(&kind.action(Stick::Left)), kind, "{}", kind.name());
        }
    }

    #[test]
    fn a_ring_keeps_one_action_per_sector() {
        let mut cfg = StickConfig::new(StickAction::ring(8), 0.1, 1.0);
        assert!(ring(&mut cfg, |n, _, _, _| set(n, 4)));
        assert_eq!(cfg.action.ring_actions().len(), 4);
    }
}
