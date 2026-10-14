//! The log overlay pages of Edit Controls, and what an input display follows (which controller,
//! which inputs, how they group), shared by log overlays and info overlays' `{current_input}`
//! cells.

use crate::config::{CurrentInput, INPUT_GROUPS, InputLogSettings, InputTracking, ItemKind, LogEnd, LogOverlay, LogSource, ScreenPosition};

use super::{
    named,
    owner::game_mut,
    pages::Page,
    rows::{Act, Nav, Number, Row, cycle, set},
    style::StyleAt,
};

/// Whose input display a tracking page sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracked {
    Log(usize),
    /// An info overlay's `{current_input}` cells.
    Info(usize),
}

const LINGER: Number = Number::new(0.5, (0.0, 30.0), 1, " s");
const LINES: Number = Number::new(1.0, (1.0, 20.0), 0, "");
const FADE: Number = Number::new(0.5, (0.0, 30.0), 1, " s");
const MAX_INPUTS: Number = Number::new(1.0, (1.0, 30.0), 0, " inputs");
const GAP: Number = Number::new(50.0, (100.0, 1000.0), 0, " ms");
const STAY: Number = Number::new(100.0, (0.0, 5000.0), 0, " ms");
/// Which controller a display follows: every one, or one by its number.
const DEVICES: [Option<u8>; 5] = [None, Some(0), Some(1), Some(2), Some(3)];

fn log_mut(config: &mut crate::config::Config, i: usize) -> Option<&mut LogOverlay> {
    game_mut(config)?.logs.get_mut(i)
}

fn settings_mut(config: &mut crate::config::Config, i: usize) -> Option<&mut InputLogSettings> {
    let LogSource::Input(s) = &mut log_mut(config, i)?.source;
    Some(s)
}

/// The game's log overlays, and a row to add one.
pub(super) fn list_rows(config: &crate::config::Config) -> Vec<Row> {
    let logs = &config.active_game().logs;
    let mut rows: Vec<Row> = logs
        .iter()
        .enumerate()
        .map(|(i, o)| Row::open(format!("{} · {} · {}", o.name, if o.always { "always up" } else { "shown by an action" }, o.style.position), Page::Log(i)))
        .collect();
    let new = logs.len();
    rows.push(Row::add("Add log overlay", move |c| {
        let name = named::new_name(c, ItemKind::Log, "Input log");
        let Some(game) = game_mut(c) else { return Act::stay() };
        game.logs.push(LogOverlay { always: true, ..LogOverlay::new(&name) });
        Act::changed().then(Nav::Open(Page::Log(new)))
    }));
    rows
}

/// A switch of the log's settings.
fn switch(i: usize, label: &str, on: bool, field: fn(&mut InputLogSettings, bool) -> bool) -> Row {
    Row::toggle(label, on, move |c, v| settings_mut(c, i).is_some_and(|s| field(s, v)))
}

/// A log overlay's page.
pub(super) fn log_rows(config: &crate::config::Config, i: usize) -> Vec<Row> {
    let Some(o) = config.active_game().logs.get(i) else { return Vec::new() };
    let LogSource::Input(s) = &o.source;
    let mut rows = vec![
        named::rename_row(config, ItemKind::Log, i),
        Row::toggle("Always on screen", o.always, move |c, on| log_mut(c, i).is_some_and(|o| set(&mut o.always, on))),
    ];
    if !o.always {
        let mut linger = LINGER.row("Stays after it's let go", o.linger.unwrap_or(0.0), move |c, v| log_mut(c, i).is_some_and(|o| set(&mut o.linger, (v > 0.0).then_some(v))));
        if o.linger.is_none() {
            linger.item.label = "Stays after it's let go: no".into();
        }
        rows.push(linger);
    }
    let newest = s.newest;
    rows.extend([
        Row::open("What it follows…", Page::Tracking(Tracked::Log(i))),
        LINES.row("Lines", f32::from(s.lines), move |c, v| settings_mut(c, i).is_some_and(|s| set(&mut s.lines, v as u8))),
        Row::step(format!("Newest line: {}", if newest == LogEnd::Top { "at the top" } else { "at the bottom" }), move |c, _| {
            let other = if newest == LogEnd::Top { LogEnd::Bottom } else { LogEnd::Top };
            settings_mut(c, i).is_some_and(|s| set(&mut s.newest, other))
        }),
    ]);
    let mut fade = FADE.row("Lines fade after", s.fade_after, move |c, v| settings_mut(c, i).is_some_and(|s| set(&mut s.fade_after, v)));
    if s.fade_after <= 0.0 {
        fade.item.label = "Lines fade after: never".into();
    }
    rows.push(fade);
    rows.push(switch(i, "Merge repeats (X×3)", s.merge_repeats, |s, v| set(&mut s.merge_repeats, v)));
    rows.push(switch(i, "What each press did", s.show_labels, |s, v| set(&mut s.show_labels, v)));
    rows.push(switch(i, "How long each press was held", s.show_holds, |s, v| set(&mut s.show_holds, v)));
    let position = o.style.position;
    rows.push(Row::step(format!("Position: {position}"), move |c, dir| log_mut(c, i).is_some_and(|o| set(&mut o.style.position, cycle(&ScreenPosition::GRID, position, dir)))));
    rows.push(Row::open("Look…", Page::Style(StyleAt::Log(i))));
    rows.push(named::delete_row(config, ItemKind::Log, i, move |c| {
        let Some(game) = game_mut(c).filter(|g| i < g.logs.len()) else { return false };
        game.logs.remove(i);
        true
    }));
    rows
}

fn tracking(config: &crate::config::Config, of: Tracked) -> Option<&InputTracking> {
    let game = config.active_game();
    match of {
        Tracked::Log(i) => game.logs.get(i).map(|o| {
            let LogSource::Input(s) = &o.source;
            &s.tracking
        }),
        Tracked::Info(i) => game.info.get(i).map(|o| &o.current_input.tracking),
    }
}

fn tracking_mut(config: &mut crate::config::Config, of: Tracked) -> Option<&mut InputTracking> {
    match of {
        Tracked::Log(i) => Some(&mut settings_mut(config, i)?.tracking),
        Tracked::Info(i) => Some(&mut game_mut(config)?.info.get_mut(i)?.current_input.tracking),
    }
}

fn current_input_mut(config: &mut crate::config::Config, i: usize) -> Option<&mut CurrentInput> {
    Some(&mut game_mut(config)?.info.get_mut(i)?.current_input)
}

/// What an input display follows: the controller, each group of inputs, how many a line keeps,
/// and the pause that starts a new one (and for current-input cells, how long they stay).
pub(super) fn tracking_rows(config: &crate::config::Config, of: Tracked) -> Vec<Row> {
    let Some(t) = tracking(config, of) else { return Vec::new() };
    let device = t.device;
    let mut rows = vec![Row::step(format!("Controller: {}", device.map_or("every one".into(), |n| format!("number {n}"))), move |c, dir| {
        tracking_mut(c, of).is_some_and(|t| set(&mut t.device, cycle(&DEVICES, device, dir)))
    })];
    for (g, (name, members)) in INPUT_GROUPS.iter().enumerate() {
        let tracked = members.iter().filter(|m| t.tracks(**m)).count();
        let all = tracked == members.len();
        let label = match tracked {
            n if n == members.len() || n == 0 => format!("{name}: {}", if all { "shown" } else { "left out" }),
            n => format!("{name}: {n} of {}", members.len()),
        };
        rows.push(Row::step(label, move |c, _| {
            tracking_mut(c, of).is_some_and(|t| {
                members.iter().for_each(|m| t.set_tracked(*m, !all));
                true
            })
        }));
        if members.len() > 1 {
            rows.push(Row::open(format!("    Each of {}…", name.to_lowercase()), Page::TrackGroup(of, g)));
        }
    }
    let what = if matches!(of, Tracked::Log(_)) { "line" } else { "sequence" };
    rows.push(MAX_INPUTS.row(&format!("Per {what}, at most"), f32::from(t.max_inputs), move |c, v| tracking_mut(c, of).is_some_and(|t| set(&mut t.max_inputs, v as u8))));
    rows.push(GAP.row_u(&format!("New {what} after"), t.gap_ms, move |c, v| tracking_mut(c, of).is_some_and(|t| set(&mut t.gap_ms, v))));
    if let Tracked::Info(i) = of
        && let Some(o) = config.active_game().info.get(i)
    {
        rows.push(STAY.row_u("Stays for", o.current_input.stay_ms, move |c, v| current_input_mut(c, i).is_some_and(|ci| set(&mut ci.stay_ms, v))));
    }
    rows
}

/// The inputs of one group, each ticked when it's shown.
pub(super) fn group_rows(config: &crate::config::Config, of: Tracked, g: usize) -> Vec<Row> {
    let (Some(t), Some((_, members))) = (tracking(config, of), INPUT_GROUPS.get(g)) else { return Vec::new() };
    members
        .iter()
        .map(|&m| {
            Row::toggle(m.name(), t.tracks(m), move |c, on| {
                tracking_mut(c, of).is_some_and(|t| {
                    let before = t.tracks(m);
                    t.set_tracked(m, on);
                    before != on
                })
            })
        })
        .collect()
}

/// The title of a tracking page.
pub(super) fn tracking_title(config: &crate::config::Config, of: Tracked) -> String {
    let game = config.active_game();
    match of {
        Tracked::Log(i) => format!("{} · what it follows", game.logs.get(i).map_or("Log overlay", |o| o.name.as_str())),
        Tracked::Info(i) => format!("{} · current input", game.info.get(i).map_or("Info overlay", |o| o.name.as_str())),
    }
}

pub(super) fn log_name(config: &crate::config::Config, i: usize) -> String {
    config.active_game().logs.get(i).map_or_else(|| "Log overlay".into(), |o| o.name.clone())
}
