//! The profile's Gyro tab, in a box laid out as a stick's (see `device_box`): the mode and its
//! output, how turning is read (space, axes, each axis's sensitivity), the response, when gyro is
//! on, filtering in an "Advanced" fold, and live pictures.

use iced::widget::{Column, row};

use super::*;
use crate::config::{CurvePreset, GyroAxes, SpeedCurve};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GyroModeKind {
    Off,
    Mouse,
    Stick,
    Position,
    Steering,
}

impl GyroModeKind {
    const ALL: [GyroModeKind; 5] = [GyroModeKind::Off, GyroModeKind::Mouse, GyroModeKind::Stick, GyroModeKind::Position, GyroModeKind::Steering];

    fn of(mode: &GyroMode) -> Self {
        match mode {
            GyroMode::Off => GyroModeKind::Off,
            GyroMode::Mouse { .. } => GyroModeKind::Mouse,
            GyroMode::Stick { .. } => GyroModeKind::Stick,
            GyroMode::Position { .. } => GyroModeKind::Position,
            GyroMode::Steering { .. } => GyroModeKind::Steering,
        }
    }

    /// The mode with its starting settings.
    fn mode(self) -> GyroMode {
        match self {
            GyroModeKind::Off => GyroMode::Off,
            GyroModeKind::Mouse => GyroMode::Mouse { sensitivity: 15.0 },
            GyroModeKind::Stick => GyroMode::Stick { stick: Stick::Right, full_rate: 360.0, anti_deadzone: 0.15 },
            GyroModeKind::Position => GyroMode::Position { stick: Stick::Right, max_angle: 30.0, anti_deadzone: 0.15, lock_at_edge: true },
            GyroModeKind::Steering => GyroMode::Steering { stick: Stick::Left, max_angle: 45.0 },
        }
    }
}

impl fmt::Display for GyroModeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GyroModeKind::Off => "Off",
            GyroModeKind::Mouse => "Mouse (gyro aiming)",
            GyroModeKind::Stick => "Gamepad stick (turn speed)",
            GyroModeKind::Position => "Gamepad stick (turn position)",
            GyroModeKind::Steering => "Steering (tilt to steer)",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActivationKind {
    Always,
    WhileHeld,
    UnlessHeld,
    Toggle,
}

impl fmt::Display for ActivationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ActivationKind::Always => "Always on",
            ActivationKind::WhileHeld => "Only while held",
            ActivationKind::UnlessHeld => "Off while held (clutch)",
            ActivationKind::Toggle => "Toggle with",
        })
    }
}

/// A choice for a pick list where `None` means nothing is picked: no recenter input, or no
/// stick that pauses gyro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Optional<T>(pub(super) Option<T>);

impl<T: fmt::Display> fmt::Display for Optional<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(v) => write!(f, "{v}"),
            None => f.write_str("(none)"),
        }
    }
}

/// The gyro's editor, for its box: the mode on the header line and, unless it's off, everything that tunes it,
/// the "Advanced" fold (open when `open`) and the pictures. A real-world sensitivity starts from
/// `flick_turn`, the profile's flick stick's full turn, when it has one.
pub(super) fn gyro_editor<'a>(cfg: &'a GyroConfig, flick_turn: Option<f32>, open: bool, pictures: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let with = move |f: &dyn Fn(&mut GyroConfig)| {
        let mut c = cfg.clone();
        f(&mut c);
        Message::SetGyro(c)
    };
    let mut rows = Column::new().spacing(8).push(header(cfg, with));
    if cfg.mode != GyroMode::Off {
        rows = mode_rows(rows, cfg, with, flick_turn);
        if !matches!(cfg.mode, GyroMode::Steering { .. }) {
            rows = aim_rows(rows, cfg, with);
        }
        if cfg.mode.uses_curve() {
            rows = curve_rows(rows, cfg, with);
        }
        rows = activation_rows(rows, cfg, with).push(advanced(cfg, open, with)).push(pictures);
    }
    rows.into()
}

/// "Gyro", the mode, and the stick it sends (and Invert, for steering) on one line.
fn header<'a>(cfg: &GyroConfig, with: impl Fn(&dyn Fn(&mut GyroConfig)) -> Message + Copy + 'a) -> Element<'a, Message> {
    let options = match cfg.mode {
        GyroMode::Stick { stick, full_rate, anti_deadzone } => {
            Vec::from(as_stick(stick, move |s| with(&|c| c.mode = GyroMode::Stick { stick: s, full_rate, anti_deadzone })))
        }
        GyroMode::Position { stick, max_angle, anti_deadzone, lock_at_edge } => {
            Vec::from(as_stick(stick, move |s| with(&|c| c.mode = GyroMode::Position { stick: s, max_angle, anti_deadzone, lock_at_edge })))
        }
        GyroMode::Steering { stick, max_angle } => {
            let mut options = Vec::from(as_stick(stick, move |s| with(&|c| c.mode = GyroMode::Steering { stick: s, max_angle })));
            options.push(checkbox(cfg.invert_x).label("Invert").on_toggle(move |v| with(&|c| c.invert_x = v)).into());
            options
        }
        GyroMode::Off | GyroMode::Mouse { .. } => Vec::new(),
    };
    box_header("Gyro".into(), GyroModeKind::ALL, GyroModeKind::of(&cfg.mode), move |k| with(&|c| c.mode = k.mode()), options)
}

/// How far the stick starts out, for the stick modes.
const ANTI_DEADZONE: &str = "Starts the stick this far out, past the game's own stick deadzone, so small turns aren't lost.";

/// The mode's own settings: sensitivity, or how far the stick is pushed.
fn mode_rows<'a>(rows: Column<'a, Message>, cfg: &'a GyroConfig, with: impl Fn(&dyn Fn(&mut GyroConfig)) -> Message + Copy + 'a, flick_turn: Option<f32>) -> Column<'a, Message> {
    match cfg.mode {
        GyroMode::Off => rows,
        GyroMode::Mouse { sensitivity } => {
            let set = move |v: f32| with(&|c| c.mode = GyroMode::Mouse { sensitivity: v });
            let real_world = cfg.full_turn_px.is_some();
            let rows = if real_world {
                rows.push(box_row("Sensitivity", slider_editor(0.1..=5.0, sensitivity, 0.05, "× real world", set)))
            } else {
                rows.push(box_row("Sensitivity", slider_editor(2.0..=60.0, sensitivity, 1.0, "px per degree", set)))
            };
            let real = checkbox(real_world).label("Real-world sensitivity").on_toggle(move |on| {
                with(&|c| {
                    c.full_turn_px = on.then(|| flick_turn.unwrap_or(8000.0));
                    c.mode = GyroMode::Mouse { sensitivity: if on { 1.0 } else { 15.0 } };
                })
            });
            let rows = rows.push(box_row("", with_help(real, "At 1×, the game turns as far as the controller does.")));
            match cfg.full_turn_px {
                Some(turn) => rows.push(sub_row(
                    "Game's full turn",
                    with_help(
                        slider_editor(500.0..=30000.0, turn, 50.0, "px", move |v| with(&|c| c.full_turn_px = Some(v))),
                        "Mouse pixels that turn the game all the way round; find it as for a flick stick. A flick stick in this profile shares it.",
                    ),
                )),
                None => rows,
            }
        }
        GyroMode::Stick { stick, full_rate, anti_deadzone } => rows
            .push(box_row("Full tilt at", slider_editor(60.0..=720.0, full_rate, 10.0, "°/s turning", move |v| with(&|c| c.mode = GyroMode::Stick { stick, full_rate: v, anti_deadzone }))))
            .push(box_row(
                "Anti-deadzone",
                with_help(slider_editor(0.0..=0.4, anti_deadzone, 0.01, "of a full push", move |v| with(&|c| c.mode = GyroMode::Stick { stick, full_rate, anti_deadzone: v })), ANTI_DEADZONE),
            )),
        GyroMode::Position { stick, max_angle, anti_deadzone, lock_at_edge } => {
            let set = move |max_angle: f32, anti_deadzone: f32, lock_at_edge: bool| with(&|c| c.mode = GyroMode::Position { stick, max_angle, anti_deadzone, lock_at_edge });
            rows.push(box_row("Full push at", slider_editor(5.0..=90.0, max_angle, 1.0, "° turned", move |v| set(v, anti_deadzone, lock_at_edge))))
                .push(box_row("Anti-deadzone", with_help(slider_editor(0.0..=0.4, anti_deadzone, 0.01, "of a full push", move |v| set(max_angle, v, lock_at_edge)), ANTI_DEADZONE)))
                .push(box_row(
                    "",
                    with_help(
                        checkbox(lock_at_edge).label("Lock at edge").on_toggle(move |on| set(max_angle, anti_deadzone, on)),
                        "Turning back leaves the edge at once, rather than first undoing the turn made past it.",
                    ),
                ))
        }
        GyroMode::Steering { stick, max_angle } => {
            rows.push(box_row("Full lock at", slider_editor(15.0..=90.0, max_angle, 1.0, "° tilt", move |v| with(&|c| c.mode = GyroMode::Steering { stick, max_angle: v }))))
        }
    }
}

/// How turning is read: the space, which axes move, each axis's sensitivity, and inverting.
fn aim_rows<'a>(rows: Column<'a, Message>, cfg: &'a GyroConfig, with: impl Fn(&dyn Fn(&mut GyroConfig)) -> Message + Copy + 'a) -> Column<'a, Message> {
    let mut rows = rows
        .push(box_row("Turning read as", dropdown(GyroHorizontal::ALL, Some(cfg.horizontal), move |h| with(&|c| c.horizontal = h)).width(SETTING_WIDTH).into()))
        .push(box_row("Axes", dropdown(GyroAxes::ALL, Some(cfg.axes), move |a| with(&|c| c.axes = a)).width(SETTING_WIDTH).into()));
    let mut inverts = row![].spacing(16);
    if cfg.axes.horizontal() {
        rows = rows.push(box_row("Horizontal speed", slider_editor(0.1..=3.0, cfg.x_scale, 0.05, "×", move |v| with(&|c| c.x_scale = v))));
        inverts = inverts.push(checkbox(cfg.invert_x).label("Invert horizontal").on_toggle(move |v| with(&|c| c.invert_x = v)));
    }
    if cfg.axes.vertical() {
        rows = rows.push(box_row("Vertical speed", slider_editor(0.1..=3.0, cfg.y_scale, 0.05, "×", move |v| with(&|c| c.y_scale = v))));
        inverts = inverts.push(checkbox(cfg.invert_y).label("Invert vertical").on_toggle(move |v| with(&|c| c.invert_y = v)));
    }
    rows.push(box_row("", inverts.into()))
}

/// The response: a preset, and its settings when it isn't linear.
fn curve_rows<'a>(rows: Column<'a, Message>, cfg: &'a GyroConfig, with: impl Fn(&dyn Fn(&mut GyroConfig)) -> Message + Copy + 'a) -> Column<'a, Message> {
    let curve = cfg.curve;
    let preset = curve.preset(1.0);
    let picker = curve_preset_picker(preset, move |p: CurvePreset| {
        let picked = p.speed_curve(1.0).unwrap_or_else(|| curve.custom_from(1.0));
        with(&|c| c.curve = picked)
    });
    let rows = rows.push(box_row("Response", picker));
    if curve.is_flat() {
        return rows;
    }
    let set = move |f: fn(&mut SpeedCurve, f32)| move |v: f32| with(&|c| f(&mut c.curve, v));
    rows.push(sub_row("Slow sensitivity", slider_editor(0.1..=3.0, curve.slow_gain, 0.05, "×", set(|c, v| c.slow_gain = v))))
        .push(sub_row("Fast sensitivity", slider_editor(0.1..=4.0, curve.fast_gain, 0.05, "×", set(|c, v| c.fast_gain = v))))
        .push(sub_row("Slow up to", slider_editor(0.0..=200.0, curve.slow_speed, 1.0, "°/s", set(|c, v| c.slow_speed = v.min(c.fast_speed - 1.0)))))
        .push(sub_row("Fast from", slider_editor(5.0..=600.0, curve.fast_speed, 5.0, "°/s", set(|c, v| c.fast_speed = v.max(c.slow_speed + 1.0)))))
        .push(sub_row("Shape", slider_editor(0.25..=4.0, curve.power, 0.05, "(1 = straight, more = slow for longer)", set(|c, v| c.power = v))))
}

/// When gyro is on: activation, recentering, pausing with a stick, and the toggle's rumble.
fn activation_rows<'a>(rows: Column<'a, Message>, cfg: &'a GyroConfig, with: impl Fn(&dyn Fn(&mut GyroConfig)) -> Message + Copy + 'a) -> Column<'a, Message> {
    let (activation_kind, activation_input) = match cfg.activation {
        GyroActivation::Always => (ActivationKind::Always, None),
        GyroActivation::WhileHeld(i) => (ActivationKind::WhileHeld, Some(i)),
        GyroActivation::UnlessHeld(i) => (ActivationKind::UnlessHeld, Some(i)),
        GyroActivation::Toggle(i) => (ActivationKind::Toggle, Some(i)),
    };
    let activation = move |kind: ActivationKind, input: GyroInput| match kind {
        ActivationKind::Always => GyroActivation::Always,
        ActivationKind::WhileHeld => GyroActivation::WhileHeld(input),
        ActivationKind::UnlessHeld => GyroActivation::UnlessHeld(input),
        ActivationKind::Toggle => GyroActivation::Toggle(input),
    };
    let current_input = activation_input.unwrap_or(GyroInput::LeftTrigger);
    let kinds = [ActivationKind::Always, ActivationKind::WhileHeld, ActivationKind::UnlessHeld, ActivationKind::Toggle];
    let mut active_row = row![dropdown(kinds, Some(activation_kind), move |k| with(&|c| c.activation = activation(k, current_input))).width(SETTING_WIDTH)]
        .spacing(10)
        .align_y(Alignment::Center);
    if activation_input.is_some() {
        active_row = active_row.push(dropdown(GyroInput::all(), activation_input, move |i| with(&|c| c.activation = activation(activation_kind, i))).width(200));
    }
    if activation_kind == ActivationKind::Toggle {
        active_row = active_row.push(checkbox(cfg.toggle_rumble).label("Rumble").on_toggle(move |v| with(&|c| c.toggle_rumble = v)));
    }
    let recenters = matches!(cfg.mode, GyroMode::Steering { .. } | GyroMode::Position { .. });
    let mut recenter_choices = vec![Optional(None)];
    recenter_choices.extend(GyroInput::all().into_iter().map(|i| Optional(Some(i))));
    let pause_choices = [Optional(None), Optional(Some(Stick::Left)), Optional(Some(Stick::Right))];
    let hinted = |picker: Element<'a, Message>, hint: &'a str| -> Element<'a, Message> {
        row![picker, text(hint).size(12).color(MUTED_COLOR)].spacing(10).align_y(Alignment::Center).into()
    };
    rows.push(box_row("Active", active_row.into()))
        .push(box_row(
            "Recenter with",
            hinted(
                dropdown(recenter_choices, Some(Optional(cfg.recenter)), move |r: Optional<GyroInput>| with(&|c| c.recenter = r.0)).width(SETTING_WIDTH).into(),
                if recenters { "sets the current direction as straight" } else { "clears leftover motion" },
            ),
        ))
        .push(box_row(
            "Pause while pushing",
            hinted(
                dropdown(pause_choices, Some(Optional(cfg.pause_with_stick)), move |s: Optional<Stick>| with(&|c| c.pause_with_stick = s.0)).width(SETTING_WIDTH).into(),
                "so the stick and gyro don't fight",
            ),
        ))
}

/// The "Advanced" fold: the jitter threshold, smoothing of slow turning, and drift
/// auto-calibration.
fn advanced<'a>(cfg: &'a GyroConfig, open: bool, with: impl Fn(&dyn Fn(&mut GyroConfig)) -> Message + Copy + 'a) -> Element<'a, Message> {
    let steering = matches!(cfg.mode, GyroMode::Steering { .. });
    let holds = if steering { "Drift" } else { "Jitter, smoothing, drift" };
    if !open {
        return advanced_fold("Advanced", holds, Target::GyroAdvanced, None);
    }
    let mut rows = Column::new();
    if !steering {
        rows = rows
            .push(fold_row("Ignore jitter below", slider_editor(0.0..=5.0, cfg.noise_threshold, 0.1, "°/s", move |v| with(&|c| c.noise_threshold = v))))
            .push(fold_row("Smooth below", slider_editor(0.0..=40.0, cfg.smoothing_threshold, 1.0, "°/s turning (0 = off)", move |v| with(&|c| c.smoothing_threshold = v))));
        if cfg.smoothing_threshold > 0.0 {
            rows = rows.push(fold_row("    Smoothing time", slider_editor(20.0..=400.0, cfg.smoothing_ms as f32, 5.0, "ms", move |v| with(&|c| c.smoothing_ms = v as u32))));
        }
    }
    rows = rows.push(fold_row(
        "",
        checkbox(cfg.auto_calibrate).label("Re-measure drift whenever the controller is set down still").on_toggle(move |v| with(&|c| c.auto_calibrate = v)).into(),
    ));
    advanced_fold("Advanced", holds, Target::GyroAdvanced, Some(rows))
}

/// The gyro's pictures, with what keeps them from moving when something does.
fn pictures<'a>(cfg: &GyroConfig, ui: &Ui) -> Element<'a, Message> {
    let pics = super::gyro_graph::figures(cfg, ui.live_gyro, ui.gyro_trail);
    let why = if ui.any_gyro { "Turn the controller to see the gyro at work. The live parts follow the active profile's saved settings." } else { "Connect a controller with a gyro to see it live." };
    let status = ui.gyro_note.filter(|_| ui.any_gyro).map(|why| text(why).size(13).color(style_accent()).into());
    visualizer_panel(pics, Some(why), status)
}

/// The Gyro tab: the settings (or, in a layer, the profile's) with their pictures, and in a
/// profile what it can't be played without.
pub(super) fn gyro_sections<'a>(p: &'a Profile, ui: &Ui) -> Vec<Element<'a, Message>> {
    let base = ui.layer.map_or(&p.gyro, |m| &m.base.gyro);
    let summary = if base.mode == GyroMode::Off { "off".to_string() } else { "on".to_string() };
    let flick_turn = [&p.left_stick, &p.right_stick].into_iter().find_map(|s| match s.action {
        StickAction::Flick { full_turn_px, .. } => Some(full_turn_px),
        _ => None,
    });
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    // Without a gyro controller this is a status worth seeing, not just help.
    if !ui.any_gyro {
        rows.push(
            text("None of your managed controllers have a gyro (PlayStation and Switch controllers do). These settings take effect once one is connected.")
                .size(13)
                .color(MUTED_COLOR)
                .into(),
        );
    }
    let open = ui.expanded.contains(&Target::GyroAdvanced);
    rows.push(device_box(layer_part(ui, LayerPart::Gyro, "Gyro".into(), summary, || vec![gyro_editor(&p.gyro, flick_turn, open, Some(pictures(&p.gyro, ui)))])));
    vec![section(
        "Gyro",
        Some("Aim by turning the controller. If the aim drifts, calibrate the gyro from the controller list on the Overview page.".into()),
        rows,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_builds_with_every_option_on() {
        let curve = CurvePreset::Precision.speed_curve(1.0).unwrap();
        for kind in GyroModeKind::ALL {
            for axes in GyroAxes::ALL {
                let cfg = GyroConfig {
                    mode: kind.mode(),
                    axes,
                    curve,
                    full_turn_px: Some(7200.0),
                    activation: GyroActivation::Toggle(GyroInput::LeftTrigger),
                    smoothing_threshold: 5.0,
                    pause_with_stick: Some(Stick::Right),
                    ..GyroConfig::default()
                };
                let _ = gyro_editor(&cfg, None, true, None);
            }
        }
    }

    #[test]
    fn custom_starts_from_a_curve_of_its_own() {
        let cfg = GyroConfig { mode: GyroMode::Mouse { sensitivity: 10.0 }, ..GyroConfig::default() };
        // Linear shows no curve settings; each named preset is recognized as itself.
        assert_eq!(cfg.curve.preset(1.0), CurvePreset::Linear);
        for p in CurvePreset::NAMED {
            assert_eq!(p.speed_curve(1.0).unwrap().preset(1.0), p);
        }
    }
}
