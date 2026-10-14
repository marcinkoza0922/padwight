//! The info overlay pages of Edit Controls: the game's info overlays, one overlay's settings,
//! and its cells, row by row. Cells take the same `{tokens}` as in the settings window.

use crate::config::{Config, InfoOverlay, ItemKind, OverlayStyle, ScreenPosition};

use super::{
    logs::Tracked,
    named,
    owner::game_mut,
    pages::Page,
    rows::{Act, Nav, Number, Row, cycle, set},
    style::StyleAt,
};

const ON_START: Number = Number::new(1.0, (0.0, 60.0), 0, " s");
const LINGER: Number = Number::new(0.5, (0.0, 30.0), 1, " s");

fn info_mut(config: &mut Config, i: usize) -> Option<&mut InfoOverlay> {
    game_mut(config)?.info.get_mut(i)
}

fn cells_mut(config: &mut Config, i: usize, r: usize) -> Option<&mut Vec<String>> {
    info_mut(config, i)?.rows.get_mut(r)
}

/// The game's info overlays, and a row to add one.
pub(super) fn list_rows(config: &Config) -> Vec<Row> {
    let infos = &config.active_game().info;
    let mut rows: Vec<Row> = infos
        .iter()
        .enumerate()
        .map(|(i, o)| Row::open(format!("{} · {} · {}", o.name, if o.always { "always up" } else { "shown by an action" }, o.style.position), Page::Info(i)))
        .collect();
    let new = infos.len();
    rows.push(Row::add("Add info overlay", move |c| {
        let name = named::new_name(c, ItemKind::Info, "Info");
        let Some(game) = game_mut(c) else { return Act::stay() };
        game.info.push(InfoOverlay { name, always: false, on_start: None, linger: None, title: None, style: OverlayStyle::info(), rows: Vec::new(), current_input: Default::default() });
        Act::changed().then(Nav::Open(Page::Info(new)))
    }));
    rows
}

/// Seconds that are off at 0.
fn seconds(label: &str, value: Option<f32>, n: &Number, store: impl Fn(&mut InfoOverlay, Option<f32>) -> bool + 'static, i: usize) -> Row {
    let mut row = n.row(label, value.unwrap_or(0.0), move |c, v| info_mut(c, i).is_some_and(|o| store(o, (v > 0.0).then_some(v))));
    if value.is_none() {
        row.item.label = format!("{label}: off");
    }
    row
}

/// An info overlay's page.
pub(super) fn info_rows(config: &Config, i: usize) -> Vec<Row> {
    let Some(o) = config.active_game().info.get(i) else { return Vec::new() };
    let title = o.title.clone().unwrap_or_default();
    let position = o.style.position;
    let mut rows = vec![
        named::rename_row(config, ItemKind::Info, i),
        Row::toggle("Always on screen", o.always, move |c, on| info_mut(c, i).is_some_and(|o| set(&mut o.always, on))),
        seconds("When the game starts, for", o.on_start, &ON_START, |o, v| set(&mut o.on_start, v), i),
        seconds("Stays after it's let go", o.linger, &LINGER, |o, v| set(&mut o.linger, v), i),
        Row::text(format!("Heading: {}", if title.is_empty() { "none" } else { &title }), "Heading of the overlay", title, move |c, typed| {
            let typed = typed.trim();
            info_mut(c, i).is_some_and(|o| set(&mut o.title, (!typed.is_empty()).then(|| typed.to_string())))
        }),
        Row::open(format!("Cells ({} rows)", o.rows.len()), Page::InfoCells(i)),
    ];
    // Settings for the `{current_input}` cells, once the overlay has any.
    if !crate::info::input_tokens(o).is_empty() {
        rows.push(Row::open("Current input: what it follows…", Page::Tracking(Tracked::Info(i))));
    }
    rows.extend([
        Row::step(format!("Position: {position}"), move |c, dir| info_mut(c, i).is_some_and(|o| set(&mut o.style.position, cycle(&ScreenPosition::GRID, position, dir)))),
        Row::open("Look…", Page::Style(StyleAt::Info(i))),
        named::delete_row(config, ItemKind::Info, i, move |c| {
            let Some(game) = game_mut(c).filter(|g| i < g.info.len()) else { return false };
            game.info.remove(i);
            true
        }),
    ]);
    rows
}

/// An overlay's rows of cells, and a row to add one.
pub(super) fn cells_rows(config: &Config, i: usize) -> Vec<Row> {
    let Some(o) = config.active_game().info.get(i) else { return Vec::new() };
    let mut rows: Vec<Row> = o.rows.iter().enumerate().map(|(r, cells)| Row::open(format!("{}. {}", r + 1, cells.join(" · ")), Page::InfoRow(i, r))).collect();
    let new = o.rows.len();
    rows.push(Row::add("Add row", move |c| {
        let Some(o) = info_mut(c, i) else { return Act::stay() };
        o.rows.push(vec![String::new()]);
        Act::changed().then(Nav::Open(Page::InfoRow(i, new)))
    }));
    rows
}

/// One row of cells: each cell's text, adding and removing cells, moving and deleting the row.
pub(super) fn row_rows(config: &Config, i: usize, r: usize) -> Vec<Row> {
    let Some(cells) = config.active_game().info.get(i).and_then(|o| o.rows.get(r)) else { return Vec::new() };
    let mut rows: Vec<Row> = cells
        .iter()
        .enumerate()
        .map(|(k, text)| {
            Row::text(format!("Cell {}: {text}", k + 1), format!("Cell {} of row {}", k + 1, r + 1), text.clone(), move |c, typed| {
                cells_mut(c, i, r).and_then(|cells| cells.get_mut(k)).is_some_and(|cell| set(cell, typed.to_string()))
            })
        })
        .collect();
    rows.push(Row::add("Add cell", move |c| Act::changed_if(cells_mut(c, i, r).is_some_and(|cells| {
        cells.push(String::new());
        true
    }))));
    if cells.len() > 1 {
        rows.push(Row::apply("Remove the last cell", move |c| Act::changed_if(cells_mut(c, i, r).is_some_and(|cells| cells.pop().is_some()))).tone(crate::menu::Tone::Remove));
    }
    let len = config.active_game().info[i].rows.len();
    for (label, to) in [("Move up", r.checked_sub(1)), ("Move down", (r + 1 < len).then_some(r + 1))] {
        let Some(to) = to else { continue };
        rows.push(Row::apply(label, move |c| {
            let Some(o) = info_mut(c, i) else { return Act::stay() };
            o.rows.swap(r, to);
            Act::changed().then(Nav::Follow(Page::InfoRow(i, to), to))
        }));
    }
    rows.push(Row::delete("Delete row", move |c| {
        let Some(o) = info_mut(c, i).filter(|o| r < o.rows.len()) else { return false };
        o.rows.remove(r);
        true
    }));
    rows
}
