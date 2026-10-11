//! Sticks on the Sticks & triggers tab: each stick's mode (gamepad, mouse, scroll, button ring,
//! flick stick) and its settings.
//!
//! Each stick sits in a box of its own, in the same order whatever its mode: the mode and its
//! output options on the header line, then the settings, an "Advanced" fold where the mode has
//! one, one row of pictures, and last the stick's direction buttons and zones.

use iced::widget::{Column, column, row};

use super::*;
use crate::config::{FlickVertical, MouseResponse};

/// A stick's mode in a few words.
pub(super) fn stick_summary(cfg: &StickConfig) -> String {
    let mode = match &cfg.action {
        StickAction::Disabled => "Disabled".to_string(),
        StickAction::Gamepad { stick, .. } => format!("pad {stick}"),
        StickAction::Mouse { speed, response, .. } if response.accel > 0.0 => {
            format!("mouse, {speed:.0} px/s, accel {:.0}%", response.accel * 100.0)
        }
        StickAction::Mouse { speed, .. } => format!("mouse, {speed:.0} px/s"),
        StickAction::Scroll { speed, .. } => format!("scroll, {speed:.0} notches/s"),
        StickAction::Ring { sectors, .. } => format!("button ring, {sectors} sectors"),
        StickAction::Flick { full_turn_px, .. } => format!("flick stick, {full_turn_px:.0} px/turn"),
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
}

impl fmt::Display for StickKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            StickKind::Disabled => "Disabled",
            StickKind::Gamepad => "Gamepad stick",
            StickKind::Mouse => "Mouse pointer",
            StickKind::Scroll => "Scroll wheel",
            StickKind::Ring => "Button ring",
            StickKind::Flick => "Flick stick",
        })
    }
}

/// The space between a box's edge (a stick's, or an "Advanced" one in it) and the rows in it.
const BOX_PADDING: f32 = 12.0;

/// The label column in a stick's box: narrower by the box's padding, so the stick's editors line
/// up with the triggers' below.
const ROW_LABEL: f32 = LABEL_WIDTH - BOX_PADDING;

/// A labeled row in a stick's box.
fn stick_row<'a>(label: impl text::IntoFragment<'a>, editor: Element<'a, Message>) -> Element<'a, Message> {
    labeled_in(ROW_LABEL, label, editor)
}

/// One stick's rows (its editor, or a layer's "Same as base" line) in a box of their own, so the
/// two sticks read as two separate blocks.
pub(super) fn stick_box<'a>(rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    container(Column::with_children(rows).spacing(10)).padding(BOX_PADDING).width(Length::Fill).style(style::inset).into()
}

pub(super) fn stick_editor<'a>(s: Stick, cfg: &'a StickConfig, ui: &Ui<'_>) -> Element<'a, Message> {
    let live = ui.live_sticks.map(|l| l[usize::from(s == Stick::Right)]);
    let open = ui.expanded.contains(&Target::StickResponse(s));
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
        StickAction::Mouse { .. } => rows = mouse_rows(rows, cfg, with),
        StickAction::Scroll { speed, invert_y } => {
            let invert_y = *invert_y;
            rows = rows.push(stick_row("Speed", slider_editor(1.0..=60.0, *speed, 1.0, "notches/s", move |v| with(StickAction::Scroll { speed: v, invert_y }))));
        }
        StickAction::Ring { .. } => rows = ring_settings(rows, cfg, with),
        StickAction::Flick { .. } => rows = flick_rows(rows, cfg, with),
        StickAction::Gamepad { .. } | StickAction::Disabled => {}
    }
    if matches!(cfg.action, StickAction::Mouse { .. } | StickAction::Scroll { .. }) {
        rows = rows.push(stick_row("Curve", slider_editor(1.0..=3.0, cfg.curve, 0.1, "(1 = linear)", set(|c, v| c.curve = v))));
    }
    if !matches!(cfg.action, StickAction::Disabled) {
        rows = rows.push(stick_row(
            "Deadzone",
            row![slider_editor(0.0..=0.5, cfg.deadzone, 0.01, "", set(|c, v| c.deadzone = v)), help(deadzone_graph::explain(cfg.deadzone))]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
        ));
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
    rows.push(stick_row("Directions press at", slider_editor(0.05..=0.95, cfg.key_threshold, 0.05, "", set(|c, v| c.key_threshold = v))))
        .push(stick_row("Zones", zone_list(Analog::Stick(s), &cfg.zones, ui.names)))
        .into()
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
    };
    let kinds = [StickKind::Disabled, StickKind::Gamepad, StickKind::Mouse, StickKind::Scroll, StickKind::Ring, StickKind::Flick];
    let picker = dropdown(kinds, Some(kind), move |k| {
        with(match k {
            StickKind::Disabled => StickAction::Disabled,
            StickKind::Gamepad => StickAction::Gamepad { stick: s, invert_y: false },
            StickKind::Mouse => StickAction::mouse(1200.0),
            StickKind::Scroll => StickAction::Scroll { speed: 15.0, invert_y: false },
            StickKind::Ring => StickAction::ring(8),
            StickKind::Flick => StickAction::flick(),
        })
    })
    .width(170);
    let mut line = row![text(s.to_string()).size(16).width(ROW_LABEL), picker].spacing(10).align_y(Alignment::Center);
    let invert = |on: bool, set: Box<dyn Fn(bool) -> Message + 'a>| checkbox(on).label("Invert Y").on_toggle(set);
    match cfg.action {
        StickAction::Gamepad { stick, invert_y } => {
            line = line
                .push(text("as").size(13).color(MUTED_COLOR))
                .push(dropdown([Stick::Left, Stick::Right], Some(stick), move |t| with(StickAction::Gamepad { stick: t, invert_y })).width(150))
                .push(invert(invert_y, Box::new(move |inv| with(StickAction::Gamepad { stick, invert_y: inv }))));
        }
        StickAction::Mouse { speed, response, invert_y } => {
            line = line.push(invert(invert_y, Box::new(move |inv| with(StickAction::Mouse { speed, response, invert_y: inv }))));
        }
        StickAction::Scroll { speed, invert_y } => {
            line = line.push(invert(invert_y, Box::new(move |inv| with(StickAction::Scroll { speed, invert_y: inv }))));
        }
        _ => {}
    }
    line.into()
}

/// The stick's pictures in a panel of their own, centered in one row that wraps when the window
/// is narrow: the ring, the deadzone, and a mouse stick's speed.
fn visualizers<'a>(cfg: &StickConfig, live: Option<(f32, f32)>, probe: f32) -> Option<Element<'a, Message>> {
    let mut pics = Vec::new();
    match cfg.action {
        StickAction::Disabled => return None,
        StickAction::Ring { sectors, start_angle, inner_radius, .. } => pics.push(super::ring_preview::view(sectors, start_angle, inner_radius, live)),
        _ => {}
    }
    pics.extend(deadzone_graph::figures(cfg.deadzone, live));
    if let StickAction::Mouse { speed, response, .. } = cfg.action {
        pics.extend(response_graph::figures(response_graph::Shown { speed, curve: cfg.curve, response, deadzone: cfg.deadzone, probe, live }));
    }
    let pics = iced::widget::Row::with_children(pics).spacing(20).wrap().vertical_spacing(12);
    let body = column![text("Visualizers").size(14), container(pics).center_x(Length::Fill)].spacing(8);
    Some(container(body).padding(BOX_PADDING).width(Length::Fill).style(style::panel).into())
}

/// A picture with a small caption above it and an optional note under it, `width` wide.
pub(super) fn figure<'a>(title: &'a str, picture: impl Into<Element<'a, Message>>, note: Option<String>, width: f32) -> Element<'a, Message> {
    let mut col = column![text(title).size(12).color(MUTED_COLOR), picture.into()].spacing(4).width(width);
    if let Some(note) = note {
        col = col.push(text(note).size(12).color(MUTED_COLOR));
    }
    col.into()
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
    rows.push(stick_row("Sectors", dropdown([4_u8, 8, 12], Some(sectors), move |n| set(n, start_angle, inner_radius, hysteresis)).width(170).into()))
        .push(stick_row("First sector at", slider_editor(0.0..=345.0, start_angle, 15.0, "° from up", move |v| set(sectors, v, inner_radius, hysteresis))))
        .push(stick_row("Starts at", slider_editor(0.05..=0.95, inner_radius, 0.05, "of the way out", move |v| set(sectors, start_angle, v, hysteresis))))
        .push(stick_row("Stickiness", slider_editor(0.0..=0.4, hysteresis, 0.05, "of a sector", move |v| set(sectors, start_angle, inner_radius, v))))
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
    rows = rows.push(stick_row(
        "Sector actions",
        row![
            text("Fill with").size(13).color(MUTED_COLOR),
            button(text("WASD").size(13)).style(style::secondary).on_press(with(StickAction::wasd())),
            button(text("Arrows").size(13)).style(style::secondary).on_press(with(StickAction::arrows())),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
    ));
    let width = 360.0 / f32::from((*sectors).max(1));
    let mut list = column![].spacing(14);
    for i in 0..usize::from(*sectors) {
        let angle = start_angle + i as f32 * width;
        let arrow = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"][((angle / 45.0).round() as usize) % 8];
        let action = actions.get(i).unwrap_or(&ButtonAction::Disabled);
        let target = Target::RingSector(s, i);
        // Top-aligned, since an editor can be several lines tall: the label stays by its first.
        let label = container(text(format!("{arrow}  Sector {}", i + 1))).padding([6, 0]).width(ROW_LABEL);
        let editor = fill_x(action_editor(action, Button::South, &ACTION_KINDS, set_action(target), KeyField::root(target), names));
        list = list.push(row![label, editor].spacing(10));
    }
    rows.push(list)
}

/// The space around an "Advanced" fold's header and its rows; with the rows' own inset it adds up
/// to `BOX_PADDING`, so `boxed_row` lines up.
const FOLD_PADDING: f32 = 4.0;

/// An "Advanced" fold: a panel whose whole header line opens it, naming what it holds while it's
/// shut, so it's hard to miss. `rows` are its rows when open.
fn advanced_fold<'a>(title: &str, holds: &'a str, s: Stick, rows: Option<Column<'a, Message>>) -> Element<'a, Message> {
    let chevron = if rows.is_some() { "▾" } else { "▸" };
    let mut line = row![text(format!("{chevron}  {title}")).size(14), space::horizontal()].align_y(Alignment::Center);
    if rows.is_none() {
        line = line.push(text(holds).size(12).color(MUTED_COLOR));
    }
    let header = button(line).style(style::fold_header).padding([6.0, BOX_PADDING - FOLD_PADDING]).width(Length::Fill);
    let mut body = column![header.on_press(Message::ToggleExpanded(Target::StickResponse(s)))];
    if let Some(rows) = rows {
        let inner = BOX_PADDING - FOLD_PADDING;
        body = body.push(container(rows.spacing(8)).padding(iced::Padding { top: FOLD_PADDING, right: inner, bottom: inner, left: inner }));
    }
    container(body).padding(FOLD_PADDING).width(Length::Fill).style(style::panel).into()
}

/// A row in an "Advanced" fold. Its label column is narrower by the fold's padding, so its editor
/// lines up with the editors outside it.
fn boxed_row<'a>(label: &'a str, editor: Element<'a, Message>) -> Element<'a, Message> {
    labeled_in(ROW_LABEL - BOX_PADDING, label, editor)
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
    rows.push(stick_row("Turn size", slider_editor(1000.0..=30000.0, full_turn_px, 100.0, "px per 360°", move |v| flick_with!(with, a.clone(), full_turn_px, v))))
        .push(stick_row(
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
        .push(stick_row("Flick at", slider_editor(0.5..=1.0, flick_threshold, 0.05, "of the way out", move |v| flick_with!(with, b.clone(), flick_threshold, v))))
}

/// A flick stick's "Advanced flick" fold.
fn flick_advanced<'a>(rows: Column<'a, Message>, cfg: &StickConfig, open: bool, s: Stick, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let (_, _, flick_time_ms, rotate_smoothing_ms, forward_deadzone, vertical, vertical_speed) = flick_fields!(cfg, rows);
    const HOLDS: &str = "Flick time, smoothing, dead angle, looking up and down";
    if !open {
        return rows.push(advanced_fold("Advanced flick", HOLDS, s, None));
    }
    let a = cfg.action.clone();
    let (a1, a2, a3, a4) = (a.clone(), a.clone(), a.clone(), a.clone());
    let mut advanced = column![
        boxed_row("Flick time", slider_editor(0.0..=300.0, flick_time_ms as f32, 10.0, "ms", move |v| flick_with!(with, a1.clone(), flick_time_ms, v as u32))),
        boxed_row("Turning smoothing", slider_editor(0.0..=200.0, rotate_smoothing_ms as f32, 5.0, "ms", move |v| flick_with!(with, a2.clone(), rotate_smoothing_ms, v as u32))),
        boxed_row("Dead angle", slider_editor(0.0..=30.0, forward_deadzone, 1.0, "° around forward", move |v| flick_with!(with, a3.clone(), forward_deadzone, v))),
        boxed_row(
            "",
            checkbox(vertical == FlickVertical::Look)
                .label("Also look up and down with the stick")
                .on_toggle(move |on| flick_with!(with, a4.clone(), vertical, if on { FlickVertical::Look } else { FlickVertical::Off }))
                .into(),
        ),
    ];
    if vertical == FlickVertical::Look {
        advanced = advanced.push(boxed_row("Vertical speed", slider_editor(100.0..=4000.0, vertical_speed, 50.0, "px/s", move |v| flick_with!(with, a.clone(), vertical_speed, v))));
    }
    rows.push(advanced_fold("Advanced flick", HOLDS, s, Some(advanced)))
}

/// A mouse stick's speed and acceleration.
fn mouse_rows<'a>(rows: Column<'a, Message>, cfg: &StickConfig, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let StickAction::Mouse { speed, response, invert_y } = cfg.action else {
        return rows;
    };
    rows.push(stick_row("Speed", slider_editor(100.0..=4000.0, speed, 50.0, "px/s", move |v| with(StickAction::Mouse { speed: v, response, invert_y }))))
        .push(stick_row(
            "Acceleration",
            slider_editor(0.0..=100.0, response.accel * 100.0, 5.0, "%", move |v| {
                with(StickAction::Mouse { speed, response: MouseResponse { accel: v / 100.0, ..response }, invert_y })
            }),
        ))
}

/// A mouse stick's "Advanced response" fold: the fine-tuning of its response.
fn mouse_advanced<'a>(rows: Column<'a, Message>, cfg: &StickConfig, open: bool, s: Stick, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let StickAction::Mouse { speed, response, invert_y } = cfg.action else {
        return rows;
    };
    const HOLDS: &str = "Ramp time, outer boost, vertical speed, smoothing";
    if !open {
        return rows.push(advanced_fold("Advanced response", HOLDS, s, None));
    }
    let set = move |r: MouseResponse| with(StickAction::Mouse { speed, response: r, invert_y });
    rows.push(advanced_fold("Advanced response", HOLDS, s, Some(column![
        boxed_row("Ramp time", slider_editor(100.0..=2000.0, response.accel_ramp_ms as f32, 50.0, "ms", move |v| {
            set(MouseResponse { accel_ramp_ms: v as u32, ..response })
        })),
        boxed_row("Outer boost", slider_editor(0.0..=1.0, response.outer_boost, 0.05, "", move |v| set(MouseResponse { outer_boost: v, ..response }))),
        boxed_row("Vertical speed", slider_editor(0.25..=2.0, response.y_scale, 0.05, "× horizontal", move |v| set(MouseResponse { y_scale: v, ..response }))),
        boxed_row("Smoothing", slider_editor(0.0..=200.0, response.smoothing_ms as f32, 5.0, "ms", move |v| set(MouseResponse { smoothing_ms: v as u32, ..response }))),
    ])))
}
