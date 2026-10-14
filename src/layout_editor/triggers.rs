//! The trigger and zone pages of Edit Controls, and the rows a mode or layer shows for a part
//! it leaves to the profile.

use crate::{
    config::{Analog, ButtonAction, Config, Trigger, TriggerAction, Zone},
    menu::Tone,
};

use super::{
    action::{ActionAt, Slot},
    owner::{self, Owner, Part, set_override, trigger_mut, zones_mut},
    pages::{Page, trigger_name},
    rows::{Act, Nav, Number, Row, set},
};

const PRESS_AT: Number = Number::new(0.05, (0.05, 0.95), 2, "");
/// How far a zone's edge moves with one step.
const EDGE: f32 = 0.05;
/// Where a new zone starts: the top half of the travel.
const NEW_ZONE: (f32, f32) = (0.5, 1.0);

/// What a mode or layer shows for a part it leaves to the profile: a row to set it here.
pub(super) fn override_rows(owner: Owner, part: Part, name: &str) -> Vec<Row> {
    vec![
        Row::note(format!("Uses the profile's {}", name.to_lowercase())),
        Row::apply(format!("Set the {} here", name.to_lowercase()), move |c| Act::changed_if(set_override(c, owner, part, true))).tone(Tone::Add),
    ]
}

/// The row that gives a part back to the profile, on a mode's or layer's page.
pub(super) fn back_to_profile(owner: Owner, part: Part) -> Option<Row> {
    owner.is_shift().then(|| Row::apply("Use the profile's", move |c| Act::changed_if(set_override(c, owner, part, false))).tone(Tone::Remove))
}

pub(super) fn triggers_rows(config: &Config, owner: Owner) -> Vec<Row> {
    [Trigger::Left, Trigger::Right]
        .into_iter()
        .map(|t| {
            let now = owner::trigger(config, owner, t).map_or("the profile's".into(), |c| action_name(&c.action));
            let glyph = if t == Trigger::Left { "{lt}" } else { "{rt}" };
            Row::open(format!("{glyph} {}: {now}", trigger_name(t)), Page::Trigger(owner, t))
        })
        .collect()
}

fn action_name(action: &TriggerAction) -> String {
    match action {
        TriggerAction::Disabled => "Off".into(),
        TriggerAction::Gamepad(_) => "Gamepad trigger".into(),
        TriggerAction::Button { action, .. } => format!("Button ({})", action.summary()),
    }
}

/// A trigger's page: what it does, a button trigger's action and press point, and its zones.
pub(super) fn trigger_rows(config: &Config, owner: Owner, t: Trigger) -> Vec<Row> {
    let Some(cfg) = owner::trigger(config, owner, t) else { return override_rows(owner, Part::Trigger(t), &trigger_name(t)) };
    let kind = match cfg.action {
        TriggerAction::Disabled => 0,
        TriggerAction::Gamepad(_) => 1,
        TriggerAction::Button { .. } => 2,
    };
    let label = ["Off", "Gamepad trigger", "Button"][kind];
    let mut rows = vec![Row::step(format!("Kind: {label}"), move |c, dir| {
        let next = match (kind as i32 + dir).rem_euclid(3) {
            0 => TriggerAction::Disabled,
            1 => TriggerAction::Gamepad(t),
            _ => TriggerAction::Button { action: ButtonAction::Gamepad(crate::config::Button::South), threshold: 0.5 },
        };
        trigger_mut(c, owner, t).is_some_and(|cfg| set(&mut cfg.action, next))
    })];
    match &cfg.action {
        TriggerAction::Gamepad(sends) => {
            let other = if *sends == Trigger::Left { Trigger::Right } else { Trigger::Left };
            rows.push(Row::step(format!("Sends: {}", trigger_name(*sends)), move |c, _| {
                trigger_mut(c, owner, t).is_some_and(|cfg| set(&mut cfg.action, TriggerAction::Gamepad(other)))
            }));
        }
        TriggerAction::Button { action, threshold } => {
            rows.push(Row::open(format!("Press: {}", action.summary()), Page::Action(ActionAt::new(Slot::Trigger(owner, t)))).keyword(action.keyword()));
            rows.push(PRESS_AT.row("Press at", *threshold, move |c, v| {
                trigger_mut(c, owner, t).is_some_and(|cfg| match &mut cfg.action {
                    TriggerAction::Button { threshold, .. } => set(threshold, v),
                    _ => false,
                })
            }));
        }
        TriggerAction::Disabled => {}
    }
    rows.push(Row::open(format!("Zones ({})", cfg.zones.len()), Page::Zones(owner, Analog::Trigger(t))));
    rows.extend(back_to_profile(owner, Part::Trigger(t)));
    rows
}

/// A stick's or trigger's zones, and a row to add one.
pub(super) fn zones_rows(config: &Config, owner: Owner, a: Analog) -> Vec<Row> {
    let zones = owner::zones(config, owner, a);
    let mut rows: Vec<Row> = zones
        .iter()
        .enumerate()
        .map(|(i, z)| Row::open(format!("{:.2}–{:.2}: {}", z.min, z.max, z.action.summary()), Page::Zone(owner, a, i)).keyword(z.action.keyword()))
        .collect();
    let new = zones.len();
    rows.push(Row::add("Add zone", move |c| {
        let Some(zones) = zones_mut(c, owner, a) else { return Act::stay() };
        zones.push(Zone { min: NEW_ZONE.0, max: NEW_ZONE.1, action: ButtonAction::Disabled });
        Act::changed().then(Nav::Open(Page::Zone(owner, a, new)))
    }));
    rows
}

/// A zone's page: where it starts and ends, its action, and deleting it. The start never passes
/// the end, and neither leaves the travel.
pub(super) fn zone_rows(config: &Config, owner: Owner, a: Analog, i: usize) -> Vec<Row> {
    let Some(zone) = owner::zones(config, owner, a).get(i) else { return Vec::new() };
    let edge = move |from: bool| {
        move |c: &mut Config, dir: i32| {
            let Some(z) = zones_mut(c, owner, a).and_then(|zs| zs.get_mut(i)) else { return false };
            let before = (z.min, z.max);
            // The other edge bounds this one with `min` and `max`, not `clamp`, which panics when
            // its bounds cross, as a zone read from a file can make them.
            let inside = |e: f32| e.clamp(0.0, 1.0);
            let step = EDGE * dir as f32;
            if from {
                z.min = inside(z.min + step).min(inside(z.max));
            } else {
                z.max = inside(z.max + step).max(inside(z.min));
            }
            before != (z.min, z.max)
        }
    };
    vec![
        Row::step(format!("From: {:.2}", zone.min), edge(true)),
        Row::step(format!("To: {:.2}", zone.max), edge(false)),
        Row::open(format!("Action: {}", zone.action.summary()), Page::Action(ActionAt::new(Slot::Zone(owner, a, i)))).keyword(zone.action.keyword()),
        Row::delete("Delete zone", move |c| {
            let Some(zones) = zones_mut(c, owner, a).filter(|zs| i < zs.len()) else { return false };
            zones.remove(i);
            true
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zone_read_outside_the_travel_comes_back_inside_when_moved() {
        let mut config = Config::default();
        let a = Analog::Trigger(Trigger::Right);
        for (min, max) in [(1.5, 2.0), (-1.0, -0.5), (0.8, 0.2)] {
            zones_mut(&mut config, Owner::Profile, a).unwrap().push(Zone { min, max, action: ButtonAction::Disabled });
            let i = owner::zones(&config, Owner::Profile, a).len() - 1;
            for (row, dir) in [(0, 1), (0, -1), (1, 1), (1, -1)] {
                let rows = zone_rows(&config, Owner::Profile, a, i);
                (rows[row].step.as_ref().unwrap())(&mut config, dir);
                let z = &owner::zones(&config, Owner::Profile, a)[i];
                let moved = if row == 0 { z.min } else { z.max };
                assert!((0.0..=1.0).contains(&moved), "{min}–{max}: {z:?}");
            }
        }
    }
}
