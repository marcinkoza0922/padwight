//! Sticks on the Sticks & triggers tab: each stick's mode (gamepad, mouse, scroll, button ring,
//! flick stick, mouse region) and its settings.
//!
//! Each stick sits in a box of its own (see `device_box`), in the same order whatever its mode:
//! the mode and its output options on the header line, then the settings, an "Advanced" fold
//! where the mode has one, one row of pictures, and last the stick's direction buttons and zones.

use iced::widget::{Column, column, row};

use super::*;
use super::mouse_region::{region_figure, region_rows};
use crate::config::{DeadzoneShape, FlickVertical, MouseRegion};

mod mouse;
use mouse::{mouse_advanced, mouse_rows};

/// A stick's mode in a few words.
pub(super) fn stick_summary(cfg: &StickConfig) -> String {
    let mode = match &cfg.action {
        StickAction::Disabled => "off".to_string(),
        StickAction::Gamepad { stick, .. } => format!("pad {stick}"),
        StickAction::Mouse { speed, response, .. } if response.accel > 0.0 => {
            format!("mouse, {speed:.0} px/s, accel {:.0}%", response.accel * 100.0)
        }
        StickAction::Mouse { speed, .. } => format!("mouse, {speed:.0} px/s"),
        StickAction::Scroll { speed, .. } => format!("scroll, {speed:.0} notches/s"),
        StickAction::Ring { sectors, .. } => format!("button ring, {sectors} sectors"),
        StickAction::Flick { full_turn_px, .. } => format!("flick stick, {full_turn_px:.0} px/turn"),
        StickAction::Region { region } => format!("mouse region, {} {:.0}% × {:.0}%", region.shape.to_string().to_lowercase(), region.width * 100.0, region.height * 100.0),
    };
    format!("{mode}{}", zones_note(cfg.zones.len()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StickKind {
    Disabled,
    Gamepad,
    Mouse,
    Scroll,
    Ring,
    Flick,
    Region,
}

impl fmt::Display for StickKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            StickKind::Disabled => "Off",
            StickKind::Gamepad => "Gamepad stick",
            StickKind::Mouse => "Mouse pointer",
            StickKind::Scroll => "Scroll wheel",
            StickKind::Ring => "Button ring",
            StickKind::Flick => "Flick stick",
            StickKind::Region => "Mouse region",
        })
    }
}

pub(super) fn stick_editor<'a>(s: Stick, cfg: &'a StickConfig, ui: &Ui<'_>) -> Element<'a, Message> {
    let live = ui.live_sticks.map(|l| l[usize::from(s == Stick::Right)]);
    let open = ui.expanded.contains(&Target::StickAdvanced(s));
    let with = move |action: StickAction| {
        let mut c = cfg.clone();
        c.action = action;
        Message::SetStick(s, c)
    };
    let set = move |f: fn(&mut StickConfig, f32)| {
        move |v| {
            let mut c = cfg.clone();
            f(&mut c, v);
            Message::SetStick(s, c)
        }
    };

    let mut rows = column![header(s, cfg, with)].spacing(8);
    match &cfg.action {
        StickAction::Mouse { .. } => rows = mouse_rows(rows, s, cfg, with),
        StickAction::Scroll { speed, invert_y } => {
            let invert_y = *invert_y;
            rows = rows.push(box_row("Speed", slider_editor(1.0..=60.0, *speed, 1.0, "notches/s", move |v| with(StickAction::Scroll { speed: v, invert_y }))));
        }
        StickAction::Ring { .. } => rows = ring_settings(rows, cfg, with),
        StickAction::Flick { .. } => rows = flick_rows(rows, cfg, with),
        StickAction::Region { region } => rows = region_rows(rows, *region, move |region| with(StickAction::Region { region })),
        StickAction::Gamepad { .. } | StickAction::Disabled => {}
    }
    // A mouse stick's curve is part of its response, with the preset.
    if matches!(cfg.action, StickAction::Scroll { .. }) {
        rows = rows.push(box_row("Curve", slider_editor(1.0..=3.0, cfg.curve, 0.1, "(1 = straight, more = finer near center)", set(|c, v| c.curve = v))));
    }
    if !matches!(cfg.action, StickAction::Disabled) {
        rows = rows.push(box_row(
            "Deadzone",
            row![fraction_editor(0.0..=0.5, cfg.deadzone, 0.01, "", set(|c, v| c.deadzone = v)), help(deadzone_graph::explain(cfg.deadzone))]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
        ));
    }
    if !matches!(cfg.action, StickAction::Disabled) {
        rows = deadzone_extras(rows, s, cfg);
    }
    match cfg.action {
        StickAction::Mouse { .. } => rows = mouse_advanced(rows, cfg, open, s, with),
        StickAction::Flick { .. } => rows = flick_advanced(rows, cfg, open, s, with),
        _ => {}
    }
    // The visualizers' panel sets off what comes after it; anything else gets a rule.
    let pictures = visualizers(cfg, live, ui.stick_probe);
    let mut panel_last = pictures.is_some();
    rows = rows.push(pictures);
    if let StickAction::Ring { .. } = cfg.action {
        rows = ring_sectors(rows, s, cfg, ui.names, with);
        panel_last = false;
    }
    if !panel_last {
        rows = rows.push(rule::horizontal(1));
    }
    // The stick's direction buttons and zones work in every mode, so they come last, apart.
    rows.push(box_row("Rotate output", slider_editor(0.0..=345.0, cfg.rotate, 15.0, "° clockwise", set(|c, v| c.rotate = v))))
        .push(box_row("Directions press at", fraction_editor(0.05..=0.95, cfg.key_threshold, 0.05, "of the way out", set(|c, v| c.key_threshold = v))))
        .push(box_row("Zones", zone_list(Analog::Stick(s), &cfg.zones, ui.names)))
        .into()
}

/// The deadzone's shape and anti-deadzone rows.
fn deadzone_extras<'a>(rows: Column<'a, Message>, s: Stick, cfg: &'a StickConfig) -> Column<'a, Message> {
    let set = move |f: fn(&mut StickConfig, f32)| {
        move |v| {
            let mut c = cfg.clone();
            f(&mut c, v);
            Message::SetStick(s, c)
        }
    };
    let shape = dropdown(DeadzoneShape::ALL.to_vec(), Some(cfg.deadzone_shape), move |v| {
        let mut c = cfg.clone();
        c.deadzone_shape = v;
        Message::SetStick(s, c)
    });
    let with_help = |editor: Element<'a, Message>, why: &str| row![editor, help(why.to_string())].spacing(10).align_y(Alignment::Center).into();
    rows.push(box_row("Deadzone shape", shape.width(120).into()))
        .push(box_row(
            "Anti-deadzone",
            with_help(
                fraction_editor(0.0..=0.5, cfg.anti_deadzone, 0.01, "", set(|c, v| c.anti_deadzone = v)),
                "Starts the output this far out as soon as the stick leaves the deadzone, to make up for a game's own deadzone.",
            ),
        ))
        .push(box_row(
            "Anti-deadzone hold",
            with_help(
                fraction_editor(0.0..=0.5, cfg.anti_deadzone_buffer, 0.01, "", set(|c, v| c.anti_deadzone_buffer = v)),
                "Extra travel past the deadzone where the output stays at the anti-deadzone before it starts to rise.",
            ),
        ))
}

/// The stick's name, its mode, and the mode's output options (which stick it sends, Invert Y),
/// on one line.
fn header<'a>(s: Stick, cfg: &StickConfig, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Element<'a, Message> {
    let kind = match cfg.action {
        StickAction::Disabled => StickKind::Disabled,
        StickAction::Gamepad { .. } => StickKind::Gamepad,
        StickAction::Mouse { .. } => StickKind::Mouse,
        StickAction::Scroll { .. } => StickKind::Scroll,
        StickAction::Ring { .. } => StickKind::Ring,
        StickAction::Flick { .. } => StickKind::Flick,
        StickAction::Region { .. } => StickKind::Region,
    };
    let kinds = [StickKind::Disabled, StickKind::Gamepad, StickKind::Mouse, StickKind::Scroll, StickKind::Ring, StickKind::Flick, StickKind::Region];
    let pick = move |k| {
        with(match k {
            StickKind::Disabled => StickAction::Disabled,
            StickKind::Gamepad => StickAction::Gamepad { stick: s, invert_y: false },
            StickKind::Mouse => StickAction::mouse(1200.0),
            StickKind::Scroll => StickAction::Scroll { speed: 15.0, invert_y: false },
            StickKind::Ring => StickAction::ring(8),
            StickKind::Flick => StickAction::flick(),
            StickKind::Region => StickAction::Region { region: MouseRegion::default() },
        })
    };
    let invert = |on: bool, set: Box<dyn Fn(bool) -> Message + 'a>| checkbox(on).label("Invert Y").on_toggle(set).into();
    let invert_x = {
        let mut c = cfg.clone();
        c.invert_x = !c.invert_x;
        checkbox(cfg.invert_x).label("Invert X").on_toggle(move |_| Message::SetStick(s, c.clone())).into()
    };
    let options: Vec<Element<'a, Message>> = match cfg.action {
        StickAction::Gamepad { stick, invert_y } => {
            let mut options = Vec::from(as_stick(stick, move |t| with(StickAction::Gamepad { stick: t, invert_y })));
            options.push(invert(invert_y, Box::new(move |inv| with(StickAction::Gamepad { stick, invert_y: inv }))));
            options
        }
        StickAction::Mouse { speed, response, invert_y } => vec![invert(invert_y, Box::new(move |inv| with(StickAction::Mouse { speed, response, invert_y: inv })))],
        StickAction::Scroll { speed, invert_y } => vec![invert(invert_y, Box::new(move |inv| with(StickAction::Scroll { speed, invert_y: inv })))],
        _ => Vec::new(),
    };
    let mut options = options;
    if cfg.action != StickAction::Disabled {
        options.push(invert_x);
    }
    // Shortcuts that turn the stick into a button ring: one press and the sectors are bound.
    if !matches!(cfg.action, StickAction::Ring { .. }) {
        options.push(text("Use as").size(13).color(MUTED_COLOR).into());
        for (label, ring) in [("WASD", StickAction::wasd as fn() -> StickAction), ("Arrows", StickAction::arrows), ("D-pad", StickAction::dpad)] {
            options.push(button(text(label).size(13)).style(style::secondary).on_press(with(ring())).into());
        }
    }
    box_header(s.to_string(), kinds, kind, pick, options)
}

/// The stick's pictures in a panel of their own, centered in one row that wraps when the window
/// is narrow: the ring, the deadzone, a mouse stick's speed, and a mouse region on the desktop.
fn visualizers<'a>(cfg: &StickConfig, live: Option<(f32, f32)>, probe: f32) -> Option<Element<'a, Message>> {
    let mut pics = Vec::new();
    match cfg.action {
        StickAction::Disabled => return None,
        StickAction::Ring { sectors, start_angle, inner_radius, .. } => pics.push(super::ring_preview::view(sectors, start_angle, inner_radius, live)),
        _ => {}
    }
    if let StickAction::Region { region } = cfg.action {
        let pointer = live.map(|pos| crate::engine::stick::region::stick_point(&region, crate::engine::apply_deadzone(pos.0, pos.1, cfg.dead())));
        pics.push(region_figure(&region, pointer));
    }
    pics.extend(deadzone_graph::figures(cfg.dead(), live));
    if let StickAction::Mouse { speed, response, .. } = cfg.action {
        pics.extend(response_graph::figures(response_graph::Shown { speed, curve: cfg.curve, response, deadzone: cfg.dead(), probe, live }));
    }
    let why = "Push the stick to see it live: where it is on the ring, its path, and how much output each push gives. \
               The live parts follow the active profile's saved settings.";
    Some(visualizer_panel(pics, Some(why), None))
}

/// A button ring's settings: its sectors and where they start.
fn ring_settings<'a>(rows: Column<'a, Message>, cfg: &'a StickConfig, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let StickAction::Ring { sectors, start_angle, inner_radius, hysteresis, actions } = &cfg.action else {
        return rows;
    };
    let (sectors, start_angle, inner_radius, hysteresis) = (*sectors, *start_angle, *inner_radius, *hysteresis);
    let set = move |sectors: u8, start_angle: f32, inner_radius: f32, hysteresis: f32| {
        let mut actions = actions.clone();
        actions.resize(usize::from(sectors), ButtonAction::Disabled);
        with(StickAction::Ring { sectors, start_angle, inner_radius, hysteresis, actions })
    };
    rows.push(box_row("Sectors", dropdown([4_u8, 8, 12], Some(sectors), move |n| set(n, start_angle, inner_radius, hysteresis)).width(170).into()))
        .push(box_row("First sector at", slider_editor(0.0..=345.0, start_angle, 15.0, "° from up", move |v| set(sectors, v, inner_radius, hysteresis))))
        .push(box_row("Starts at", fraction_editor(0.05..=0.95, inner_radius, 0.05, "of the way out", move |v| set(sectors, start_angle, v, hysteresis))))
        .push(box_row("Stickiness", fraction_editor(0.0..=0.4, hysteresis, 0.05, "of a sector", move |v| set(sectors, start_angle, inner_radius, v))))
}

/// A button ring's action for each sector, with presets that fill them all.
fn ring_sectors<'a>(
    mut rows: Column<'a, Message>,
    s: Stick,
    cfg: &'a StickConfig,
    names: &Names,
    with: impl Fn(StickAction) -> Message + Copy + 'a,
) -> Column<'a, Message> {
    let StickAction::Ring { sectors, start_angle, actions, .. } = &cfg.action else {
        return rows;
    };
    rows = rows.push(box_row(
        "Sector actions",
        row![
            text("Fill with").size(13).color(MUTED_COLOR),
            button(text("WASD").size(13)).style(style::secondary).on_press(with(StickAction::wasd())),
            button(text("Arrows").size(13)).style(style::secondary).on_press(with(StickAction::arrows())),
            button(text("D-pad").size(13)).style(style::secondary).on_press(with(StickAction::dpad())),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
    ));
    let width = 360.0 / f32::from((*sectors).max(1));
    let mut list = column![].spacing(14);
    for i in 0..usize::from(*sectors) {
        let angle = start_angle + i as f32 * width;
        // The nearest of the eight directions, in words: an arrow glyph is too small to tell apart.
        let direction = crate::config::COMPASS[((angle / 45.0).round() as usize) % 8].0;
        let action = actions.get(i).unwrap_or(&ButtonAction::Disabled);
        let target = Target::RingSector(s, i);
        // Top-aligned, since an editor can be several lines tall: the label stays by its first.
        let label = container(text(format!("Sector {} · {direction}", i + 1)).wrapping(text::Wrapping::None)).padding([6, 0]).width(ROW_LABEL);
        let editor = fill_x(action_editor(action, Button::South, &ACTION_KINDS, set_action(target), KeyField::root(target), names));
        list = list.push(row![label, editor].spacing(10));
    }
    rows.push(list)
}

/// The fields of a flick stick, for changing one of them.
macro_rules! flick_fields {
    ($cfg:expr, $rows:expr) => {{
        let StickAction::Flick { full_turn_px, flick_threshold, flick_time_ms, rotate_smoothing_ms, forward_deadzone, vertical, vertical_speed } = $cfg.action else {
            return $rows;
        };
        (full_turn_px, flick_threshold, flick_time_ms, rotate_smoothing_ms, forward_deadzone, vertical, vertical_speed)
    }};
}

/// Sends the flick stick `$a` with one field changed.
macro_rules! flick_with {
    ($with:expr, $a:expr, $name:ident, $value:expr) => {{
        let mut a = $a;
        if let StickAction::Flick { $name, .. } = &mut a {
            *$name = $value;
        }
        $with(a)
    }};
}

/// A flick stick's turn size and trigger point.
fn flick_rows<'a>(rows: Column<'a, Message>, cfg: &StickConfig, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let (full_turn_px, flick_threshold, ..) = flick_fields!(cfg, rows);
    let a = cfg.action.clone();
    let b = a.clone();
    rows.push(box_row("Turn size", slider_editor(1000.0..=30000.0, full_turn_px, 100.0, "px per 360°", move |v| flick_with!(with, a.clone(), full_turn_px, v))))
        .push(box_row(
            "",
            row![
                button(text("Test turn").size(13)).style(style::secondary).on_press(Message::TestTurn(full_turn_px as i32)),
                help(
                    "Moves the mouse one full turn to the right after 3 seconds, so you can watch your game. \
                     If it turned N°, set the size to this × 360 / N."
                        .into()
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
        ))
        .push(box_row("Flick at", fraction_editor(0.5..=1.0, flick_threshold, 0.05, "of the way out", move |v| flick_with!(with, b.clone(), flick_threshold, v))))
}

/// A flick stick's "Advanced flick" fold.
fn flick_advanced<'a>(rows: Column<'a, Message>, cfg: &StickConfig, open: bool, s: Stick, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let (_, _, flick_time_ms, rotate_smoothing_ms, forward_deadzone, vertical, vertical_speed) = flick_fields!(cfg, rows);
    const HOLDS: &str = "Flick time, smoothing, dead angle, looking up and down";
    if !open {
        return rows.push(advanced_fold("Advanced flick", HOLDS, Target::StickAdvanced(s), None));
    }
    let a = cfg.action.clone();
    let (a1, a2, a3, a4) = (a.clone(), a.clone(), a.clone(), a.clone());
    let mut advanced = column![
        fold_row("Flick time", slider_editor(0.0..=300.0, flick_time_ms as f32, 10.0, "ms", move |v| flick_with!(with, a1.clone(), flick_time_ms, v as u32))),
        fold_row("Turning smoothing", slider_editor(0.0..=200.0, rotate_smoothing_ms as f32, 5.0, "ms", move |v| flick_with!(with, a2.clone(), rotate_smoothing_ms, v as u32))),
        fold_row("Dead angle", slider_editor(0.0..=30.0, forward_deadzone, 1.0, "° around forward", move |v| flick_with!(with, a3.clone(), forward_deadzone, v))),
        fold_row(
            "",
            checkbox(vertical == FlickVertical::Look)
                .label("Also look up and down with the stick")
                .on_toggle(move |on| flick_with!(with, a4.clone(), vertical, if on { FlickVertical::Look } else { FlickVertical::Off }))
                .into(),
        ),
    ];
    if vertical == FlickVertical::Look {
        advanced = advanced.push(fold_row("Vertical speed", slider_editor(100.0..=4000.0, vertical_speed, 50.0, "px/s", move |v| flick_with!(with, a.clone(), vertical_speed, v))));
    }
    rows.push(advanced_fold("Advanced flick", HOLDS, Target::StickAdvanced(s), Some(advanced)))
}
