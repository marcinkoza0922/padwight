//! Triggers on the Sticks & triggers tab, and the zones both sticks and triggers can have: extra
//! actions held within a range of travel.

use iced::widget::{column, row};

use crate::config::{Gestures, TriggerConfig};

use super::*;

/// A trigger's action in a few words.
pub(super) fn trigger_summary(cfg: &crate::config::TriggerConfig) -> String {
    let action = match &cfg.action {
        TriggerAction::Disabled => "Disabled".to_string(),
        TriggerAction::Gamepad(t) => format!("pad {t}"),
        TriggerAction::Button { action, .. } => summarize(action),
    };
    let edges: String = match &cfg.action {
        TriggerAction::Button { .. } => GestureKind::EDGES.into_iter().filter(|k| cfg.gestures.get(*k).is_some()).map(|k| format!(" + {}", k.to_string().to_lowercase())).collect(),
        _ => String::new(),
    };
    format!("{action}{edges}{}", zones_note(cfg.zones.len()))
}

/// " + 2 zones", or nothing.
pub(super) fn zones_note(n: usize) -> String {
    match n {
        0 => String::new(),
        1 => " + 1 zone".into(),
        n => format!(" + {n} zones"),
    }
}

/// A button-acting trigger's On press and On release, each with a ✕, then the buttons that add
/// the ones not set, like a button's gestures.
fn edge_rows<'a>(t: Trigger, gestures: &'a Gestures, default_button: Button, names: &Names) -> Vec<Element<'a, Message>> {
    let mut rows = Vec::new();
    for kind in GestureKind::EDGES {
        let Some(action) = gestures.get(kind) else { continue };
        let target = Target::TriggerGesture(t, kind);
        rows.push(labeled(
            format!("    {kind}"),
            fill_x(
                row![
                    action_editor(action, default_button, &ACTION_KINDS, set_action(target), KeyField::root(target), names),
                    button(text("✕").size(13)).style(style::secondary).on_press(Message::RemoveTriggerGesture(t, kind)),
                ]
                .spacing(8)
                .align_y(Alignment::Start),
            ),
        ));
    }
    let adds: Vec<Element<'a, Message>> = GestureKind::EDGES
        .into_iter()
        .filter(|k| gestures.get(*k).is_none())
        .map(|k| button(text(format!("+ {k}")).size(13)).style(style::secondary).padding([3, 10]).on_press(Message::AddTriggerGesture(t, k)).into())
        .collect();
    if !adds.is_empty() {
        rows.push(labeled("", row(adds).spacing(6).into()));
    }
    rows
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ZonePreset {
    Empty,
    /// Left Shift while a stick is only partly pushed (walk in WASD games).
    Walk,
}

/// Extra actions held while the stick/trigger is within a range of travel, in a labeled row.
pub(super) fn zone_editor<'a>(analog: Analog, zones: &'a [Zone], names: &Names) -> Element<'a, Message> {
    labeled("    Zones", zone_list(analog, zones, names))
}

/// The zones and the buttons that add them: the editor part of a zone row.
pub(super) fn zone_list<'a>(analog: Analog, zones: &'a [Zone], names: &Names) -> Element<'a, Message> {
    let what = match analog {
        Analog::Stick(_) => "pushed",
        Analog::Trigger(_) => "pulled",
    };
    let zones_help = help(format!(
        "Extra outputs held, on top of its main action, while it's {what} within a range (0 = just \
         past rest, 1 = all the way)."
    ));
    let mut col = column![].spacing(8);

    for (i, zone) in zones.iter().enumerate() {
        let (min, max) = (zone.min, zone.max);
        let range = row![
            text("From").size(13),
            slider(0.0..=1.0, min, move |v| Message::SetZoneRange(analog, i, v, max)).step(0.05_f32).width(140),
            text(format!("{min:.2}")).size(13),
            text("to").size(13),
            slider(0.0..=1.0, max, move |v| Message::SetZoneRange(analog, i, min, v)).step(0.05_f32).width(140),
            text(format!("{max:.2}")).size(13),
            space::horizontal(),
            button(text("✕").size(13)).style(style::secondary).on_press(Message::RemoveZone(analog, i)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let mut body = column![
            range,
            fill_x(action_editor(&zone.action, Button::South, &ACTION_KINDS, set_action(Target::Zone(analog, i)), KeyField::root(Target::Zone(analog, i)), names)),
        ]
        .spacing(8);
        if min >= max {
            body = body.push(text("The range must start below where it ends.").size(12).color(ERROR_COLOR));
        }
        col = col.push(container(body).padding(10).style(style::inset));
    }

    let mut buttons = row![
        button(text("+ Add zone").size(13)).style(style::secondary).on_press(Message::AddZone(analog, ZonePreset::Empty)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if matches!(analog, Analog::Stick(_)) {
        buttons = buttons.push(
            button(text("+ Walk modifier (Shift on partial push)").size(13))
                .style(style::secondary)
                .on_press(Message::AddZone(analog, ZonePreset::Walk)),
        );
    }
    col.push(buttons.push(zones_help)).into()
}

pub(super) fn trigger_editor<'a>(t: Trigger, cfg: &'a TriggerConfig, analog: bool, names: &Names) -> Element<'a, Message> {
    let (action, zones) = (&cfg.action, cfg.zones.as_slice());
    // A trigger acting as a button presses the matching bumper by default.
    let default_button = if t == Trigger::Left { Button::LeftBumper } else { Button::RightBumper };
    let (threshold, current) = match action {
        TriggerAction::Button { action, threshold } => (*threshold, action.clone()),
        _ => (0.5, ButtonAction::Disabled),
    };
    let analog_out = if let TriggerAction::Gamepad(out) = action { Some(*out) } else { None };
    let on_change: OnAction<'a> = Rc::new(move |a| {
        Message::SetTrigger(t, if a == ButtonAction::Disabled { TriggerAction::Disabled } else { TriggerAction::Button { action: a, threshold } })
    });
    let picker = super::action_picker::trigger_button(t, analog_out, &current, names, on_change);

    let mut line = row![text(t.to_string()).width(LABEL_WIDTH), picker].spacing(10).align_y(Alignment::Start);
    let mut rows = column![].spacing(8);
    if let TriggerAction::Button { action: inner, threshold } = action {
        let threshold = *threshold;
        let on_change: OnAction<'a> = Rc::new(move |a| Message::SetTrigger(t, TriggerAction::Button { action: a, threshold }));
        line = line.push(action_value(inner, default_button, on_change, KeyField::root(Target::Trigger(t)), names));
        rows = rows.push(line);
        if analog {
            let inner = inner.clone();
            rows = rows.push(labeled(
                "    Presses at",
                fraction_editor(0.05..=0.95, threshold, 0.05, "of a full pull", move |v| {
                    Message::SetTrigger(t, TriggerAction::Button { action: inner.clone(), threshold: v })
                }),
            ));
        }
        for row in edge_rows(t, &cfg.gestures, default_button, names) {
            rows = rows.push(row);
        }
    } else {
        rows = rows.push(line);
    }
    if analog {
        return rows.push(zone_editor(Analog::Trigger(t), zones, names)).into();
    }
    rows = rows.push(labeled(
        "",
        text("Your controller's triggers are on/off only, so pull thresholds and zones don't apply.")
            .size(12)
            .color(MUTED_COLOR)
            .into(),
    ));
    // Keep existing zones reachable so they can be removed rather than silently lingering.
    if !zones.is_empty() {
        rows = rows
            .push(labeled(
                "",
                text("This profile still has zones for this trigger; a full pull acts as the top zone.")
                    .size(12)
                    .color(ERROR_COLOR)
                    .into(),
            ))
            .push(zone_editor(Analog::Trigger(t), zones, names));
    }
    rows.into()
}
