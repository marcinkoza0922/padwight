//! The gyro page of Edit Controls: its mode and the mode's settings, how turning is read, the
//! response curve, when it's on, and the finer settings the settings window folds away.

use crate::config::{Button, Config, CurvePreset, GyroActivation, GyroAxes, GyroConfig, GyroHorizontal, GyroInput, GyroMode, SpeedCurve, Stick};

use super::{
    owner::{self, Owner, Part, gyro_mut},
    pages::{Page, stick_name},
    rows::{Act, Nav, Number, Row, cycle, set},
    triggers::{back_to_profile, override_rows},
};

/// Which input a picker on the gyro page sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GyroPick {
    /// The input that holds, lets go of or toggles the gyro.
    Activation,
    Recenter,
}

impl GyroPick {
    pub fn title(self) -> &'static str {
        match self {
            GyroPick::Activation => "Gyro · turned on with",
            GyroPick::Recenter => "Gyro · recentered with",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Off,
    Mouse,
    Stick,
    Position,
    Steering,
}

const MODES: [Mode; 5] = [Mode::Off, Mode::Mouse, Mode::Stick, Mode::Position, Mode::Steering];

impl Mode {
    fn of(mode: &GyroMode) -> Mode {
        match mode {
            GyroMode::Off => Mode::Off,
            GyroMode::Mouse { .. } => Mode::Mouse,
            GyroMode::Stick { .. } => Mode::Stick,
            GyroMode::Position { .. } => Mode::Position,
            GyroMode::Steering { .. } => Mode::Steering,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Mode::Off => "Off",
            Mode::Mouse => "Mouse",
            Mode::Stick => "Stick (turning speed)",
            Mode::Position => "Stick (position)",
            Mode::Steering => "Steering",
        }
    }

    /// The mode with its starting settings, as the settings window starts them.
    fn mode(self) -> GyroMode {
        match self {
            Mode::Off => GyroMode::Off,
            Mode::Mouse => GyroMode::Mouse { sensitivity: 15.0 },
            Mode::Stick => GyroMode::Stick { stick: Stick::Right, full_rate: 360.0, anti_deadzone: 0.15 },
            Mode::Position => GyroMode::Position { stick: Stick::Right, max_angle: 30.0, anti_deadzone: 0.15, lock_at_edge: true },
            Mode::Steering => GyroMode::Steering { stick: Stick::Left, max_angle: 45.0 },
        }
    }
}

/// When the gyro is on, without its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Active {
    Always,
    WhileHeld,
    UnlessHeld,
    Toggle,
}

const ACTIVES: [Active; 4] = [Active::Always, Active::WhileHeld, Active::UnlessHeld, Active::Toggle];

impl Active {
    fn of(a: GyroActivation) -> (Active, Option<GyroInput>) {
        match a {
            GyroActivation::Always => (Active::Always, None),
            GyroActivation::WhileHeld(i) => (Active::WhileHeld, Some(i)),
            GyroActivation::UnlessHeld(i) => (Active::UnlessHeld, Some(i)),
            GyroActivation::Toggle(i) => (Active::Toggle, Some(i)),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Active::Always => "Always on",
            Active::WhileHeld => "Only while held",
            Active::UnlessHeld => "Off while held (clutch)",
            Active::Toggle => "Toggled",
        }
    }

    fn with(self, input: GyroInput) -> GyroActivation {
        match self {
            Active::Always => GyroActivation::Always,
            Active::WhileHeld => GyroActivation::WhileHeld(input),
            Active::UnlessHeld => GyroActivation::UnlessHeld(input),
            Active::Toggle => GyroActivation::Toggle(input),
        }
    }
}

const REAL_SENSITIVITY: Number = Number::new(0.05, (0.1, 5.0), 2, "× real world");
const SENSITIVITY: Number = Number::new(1.0, (2.0, 60.0), 0, " px per degree");
const FULL_TURN: Number = Number::new(50.0, (500.0, 30_000.0), 0, " px");
const FULL_RATE: Number = Number::new(10.0, (60.0, 720.0), 0, "°/s");
const ANTI_DEADZONE: Number = Number::new(0.01, (0.0, 0.4), 2, "");
const POSITION_ANGLE: Number = Number::new(1.0, (5.0, 90.0), 0, "°");
const STEERING_ANGLE: Number = Number::new(1.0, (15.0, 90.0), 0, "°");
const AXIS_SCALE: Number = Number::new(0.05, (0.1, 3.0), 2, "×");
const SLOW_GAIN: Number = Number::new(0.05, (0.1, 3.0), 2, "×");
const FAST_GAIN: Number = Number::new(0.05, (0.1, 4.0), 2, "×");
const SLOW_SPEED: Number = Number::new(1.0, (0.0, 200.0), 0, "°/s");
const FAST_SPEED: Number = Number::new(5.0, (5.0, 600.0), 0, "°/s");
const SHAPE: Number = Number::new(0.05, (0.25, 4.0), 2, "");
const JITTER: Number = Number::new(0.1, (0.0, 5.0), 1, "°/s");
const SMOOTH_BELOW: Number = Number::new(1.0, (0.0, 40.0), 0, "°/s");
const SMOOTH_TIME: Number = Number::new(5.0, (20.0, 400.0), 0, " ms");

/// A number of the gyro's config, changed by `field`.
fn num(owner: Owner, label: &str, value: f32, n: &Number, field: fn(&mut GyroConfig, f32) -> bool) -> Row {
    n.row(label, value, move |c, v| gyro_mut(c, owner).is_some_and(|g| field(g, v)))
}

fn switch(owner: Owner, label: &str, on: bool, field: fn(&mut GyroConfig, bool) -> bool) -> Row {
    Row::toggle(label, on, move |c, v| gyro_mut(c, owner).is_some_and(|g| field(g, v)))
}

/// The short name of a gyro input.
fn input_name(i: GyroInput) -> &'static str {
    match i {
        GyroInput::Button(b) => b.short_name(),
        GyroInput::LeftTrigger => "LT",
        GyroInput::RightTrigger => "RT",
    }
}

pub(super) fn gyro_rows(config: &Config, owner: Owner) -> Vec<Row> {
    let Some(g) = owner::gyro(config, owner) else { return override_rows(owner, Part::Gyro, "Gyro") };
    let mode = Mode::of(&g.mode);
    let mut rows = vec![Row::step(format!("Mode: {}", mode.name()), move |c, dir| {
        let next = cycle(&MODES, mode, dir).mode();
        gyro_mut(c, owner).is_some_and(|g| set(&mut g.mode, next))
    })];
    if mode != Mode::Off {
        rows.extend(mode_rows(owner, g));
        if mode != Mode::Steering {
            rows.extend(aim_rows(owner, g));
        }
        if g.mode.uses_curve() {
            rows.extend(curve_rows(owner, &g.curve));
        }
        rows.extend(activation_rows(owner, g));
        rows.extend(fine_rows(owner, g, mode == Mode::Steering));
    }
    rows.extend(back_to_profile(owner, Part::Gyro));
    rows
}

/// Changes the stick a stick mode sends.
fn sends_row(owner: Owner, stick: Stick) -> Row {
    Row::step(format!("Sends: {}", stick_name(stick)), move |c, _| {
        let other = if stick == Stick::Left { Stick::Right } else { Stick::Left };
        gyro_mut(c, owner).is_some_and(|g| match &mut g.mode {
            GyroMode::Stick { stick, .. } | GyroMode::Position { stick, .. } | GyroMode::Steering { stick, .. } => set(stick, other),
            _ => false,
        })
    })
}

/// The mode's own settings: sensitivity, or how far the stick is pushed.
fn mode_rows(owner: Owner, g: &GyroConfig) -> Vec<Row> {
    match g.mode {
        GyroMode::Off => Vec::new(),
        GyroMode::Mouse { sensitivity } => {
            let real = g.full_turn_px.is_some();
            let field = |g: &mut GyroConfig, v: f32| match &mut g.mode {
                GyroMode::Mouse { sensitivity } => set(sensitivity, v),
                _ => false,
            };
            let mut rows = vec![
                num(owner, "Sensitivity", sensitivity, if real { &REAL_SENSITIVITY } else { &SENSITIVITY }, field),
                switch(owner, "Real-world sensitivity", real, |g, on| {
                    let changed = set(&mut g.full_turn_px, on.then_some(8000.0));
                    g.mode = GyroMode::Mouse { sensitivity: if on { 1.0 } else { 15.0 } };
                    changed
                }),
            ];
            if let Some(turn) = g.full_turn_px {
                rows.push(num(owner, "Game's full turn", turn, &FULL_TURN, |g, v| set(&mut g.full_turn_px, Some(v))));
            }
            rows
        }
        GyroMode::Stick { stick, full_rate, anti_deadzone } => vec![
            sends_row(owner, stick),
            num(owner, "Full tilt at", full_rate, &FULL_RATE, |g, v| match &mut g.mode {
                GyroMode::Stick { full_rate, .. } => set(full_rate, v),
                _ => false,
            }),
            num(owner, "Anti-deadzone", anti_deadzone, &ANTI_DEADZONE, set_anti_deadzone),
        ],
        GyroMode::Position { stick, max_angle, anti_deadzone, lock_at_edge } => vec![
            sends_row(owner, stick),
            num(owner, "Full push at", max_angle, &POSITION_ANGLE, set_max_angle),
            num(owner, "Anti-deadzone", anti_deadzone, &ANTI_DEADZONE, set_anti_deadzone),
            switch(owner, "Lock at edge", lock_at_edge, |g, on| match &mut g.mode {
                GyroMode::Position { lock_at_edge, .. } => set(lock_at_edge, on),
                _ => false,
            }),
        ],
        GyroMode::Steering { stick, max_angle } => vec![
            sends_row(owner, stick),
            num(owner, "Full lock at", max_angle, &STEERING_ANGLE, set_max_angle),
            switch(owner, "Invert", g.invert_x, |g, on| set(&mut g.invert_x, on)),
        ],
    }
}

fn set_anti_deadzone(g: &mut GyroConfig, v: f32) -> bool {
    match &mut g.mode {
        GyroMode::Stick { anti_deadzone, .. } | GyroMode::Position { anti_deadzone, .. } => set(anti_deadzone, v),
        _ => false,
    }
}

fn set_max_angle(g: &mut GyroConfig, v: f32) -> bool {
    match &mut g.mode {
        GyroMode::Position { max_angle, .. } | GyroMode::Steering { max_angle, .. } => set(max_angle, v),
        _ => false,
    }
}

/// How turning is read: the space, which axes move, each axis's speed, and inverting.
fn aim_rows(owner: Owner, g: &GyroConfig) -> Vec<Row> {
    let (horizontal, axes) = (g.horizontal, g.axes);
    let mut rows = vec![
        Row::step(format!("Turning read as: {horizontal}"), move |c, dir| {
            gyro_mut(c, owner).is_some_and(|g| set(&mut g.horizontal, cycle(&GyroHorizontal::ALL, horizontal, dir)))
        }),
        Row::step(format!("Axes: {axes}"), move |c, dir| gyro_mut(c, owner).is_some_and(|g| set(&mut g.axes, cycle(&GyroAxes::ALL, axes, dir)))),
    ];
    if axes.horizontal() {
        rows.push(num(owner, "Horizontal speed", g.x_scale, &AXIS_SCALE, |g, v| set(&mut g.x_scale, v)));
        rows.push(switch(owner, "Invert horizontal", g.invert_x, |g, on| set(&mut g.invert_x, on)));
    }
    if axes.vertical() {
        rows.push(num(owner, "Vertical speed", g.y_scale, &AXIS_SCALE, |g, v| set(&mut g.y_scale, v)));
        rows.push(switch(owner, "Invert vertical", g.invert_y, |g, on| set(&mut g.invert_y, on)));
    }
    rows
}

/// The response: a preset, and its settings when it isn't linear.
fn curve_rows(owner: Owner, curve: &SpeedCurve) -> Vec<Row> {
    let preset = curve.preset(1.0);
    let mut rows = vec![Row::step(format!("Response: {preset}"), move |c, dir| {
        let next = cycle(&CurvePreset::NAMED, preset, dir).speed_curve(1.0).unwrap_or_default();
        gyro_mut(c, owner).is_some_and(|g| set(&mut g.curve, next))
    })];
    if curve.is_flat() {
        return rows;
    }
    rows.push(num(owner, "Slow sensitivity", curve.slow_gain, &SLOW_GAIN, |g, v| set(&mut g.curve.slow_gain, v)));
    rows.push(num(owner, "Fast sensitivity", curve.fast_gain, &FAST_GAIN, |g, v| set(&mut g.curve.fast_gain, v)));
    rows.push(num(owner, "Slow up to", curve.slow_speed, &SLOW_SPEED, |g, v| {
        let v = v.min(g.curve.fast_speed - 1.0);
        set(&mut g.curve.slow_speed, v)
    }));
    rows.push(num(owner, "Fast from", curve.fast_speed, &FAST_SPEED, |g, v| {
        let v = v.max(g.curve.slow_speed + 1.0);
        set(&mut g.curve.fast_speed, v)
    }));
    rows.push(num(owner, "Shape", curve.power, &SHAPE, |g, v| set(&mut g.curve.power, v)));
    rows
}

/// When gyro is on, recentering, and pausing with a stick.
fn activation_rows(owner: Owner, g: &GyroConfig) -> Vec<Row> {
    let (active, input) = Active::of(g.activation);
    let mut rows = vec![Row::step(format!("Active: {}", active.name()), move |c, dir| {
        let next = cycle(&ACTIVES, active, dir).with(input.unwrap_or(GyroInput::LeftTrigger));
        gyro_mut(c, owner).is_some_and(|g| set(&mut g.activation, next))
    })];
    if let Some(input) = input {
        rows.push(Row::open(format!("With: {}", input_name(input)), Page::GyroInput(owner, GyroPick::Activation)));
    }
    if active == Active::Toggle {
        rows.push(switch(owner, "Rumble on toggle", g.toggle_rumble, |g, on| set(&mut g.toggle_rumble, on)));
    }
    rows.push(Row::open(format!("Recenter with: {}", g.recenter.map_or("none", input_name)), Page::GyroInput(owner, GyroPick::Recenter)));
    let pause = g.pause_with_stick;
    let label = pause.map_or("no".into(), stick_name);
    rows.push(Row::step(format!("Pause while pushing: {label}"), move |c, dir| {
        let next = cycle(&[None, Some(Stick::Left), Some(Stick::Right)], pause, dir);
        gyro_mut(c, owner).is_some_and(|g| set(&mut g.pause_with_stick, next))
    }));
    rows
}

/// The jitter threshold, smoothing of slow turning, and drift auto-calibration.
fn fine_rows(owner: Owner, g: &GyroConfig, steering: bool) -> Vec<Row> {
    let mut rows = Vec::new();
    if !steering {
        rows.push(num(owner, "Ignore jitter below", g.noise_threshold, &JITTER, |g, v| set(&mut g.noise_threshold, v)));
        rows.push(num(owner, "Smooth below", g.smoothing_threshold, &SMOOTH_BELOW, |g, v| set(&mut g.smoothing_threshold, v)));
        if g.smoothing_threshold > 0.0 {
            rows.push(num(owner, "Smoothing time", g.smoothing_ms as f32, &SMOOTH_TIME, |g, v| set(&mut g.smoothing_ms, v as u32)));
        }
    }
    rows.push(switch(owner, "Re-measure drift when set down", g.auto_calibrate, |g, on| set(&mut g.auto_calibrate, on)));
    rows
}

/// The inputs a gyro picker offers: every button and both triggers (and none, to recenter).
pub(super) fn input_rows(config: &Config, owner: Owner, pick: GyroPick) -> Vec<Row> {
    let Some(g) = owner::gyro(config, owner) else { return Vec::new() };
    let (active, _) = Active::of(g.activation);
    let mut choices: Vec<Option<GyroInput>> = if pick == GyroPick::Recenter { vec![None] } else { Vec::new() };
    choices.extend(GyroInput::all().into_iter().filter(|i| *i != GyroInput::Button(Button::Guide)).map(Some));
    choices
        .into_iter()
        .map(|choice| {
            let buttons = match choice {
                Some(GyroInput::Button(b)) => vec![b],
                _ => Vec::new(),
            };
            let label = match choice {
                Some(GyroInput::Button(_)) => String::new(),
                Some(i) => input_name(i).into(),
                None => "None".into(),
            };
            Row::apply(label, move |c| {
                let Some(g) = gyro_mut(c, owner) else { return Act::stay() };
                let changed = match (pick, choice) {
                    (GyroPick::Activation, Some(i)) => set(&mut g.activation, active.with(i)),
                    (GyroPick::Recenter, r) => set(&mut g.recenter, r),
                    (GyroPick::Activation, None) => false,
                };
                Act::changed_if(changed).then(Nav::Back(1))
            })
            .buttons(buttons)
        })
        .collect()
}
