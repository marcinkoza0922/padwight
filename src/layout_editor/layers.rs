//! The mode and layer pages of Edit Controls: the game's modes or layers, and one of them, whose
//! buttons, combos, sticks, triggers and gyro use the profile's pages, pointed at it.

use crate::config::{Combo, Config, Indicator, ItemKind, Layer};

use super::{
    named,
    owner::{self, Owner, Part, Shift, layer, layer_mut},
    pages::Page,
    rows::{Act, Nav, Number, Row, counted, set},
    style::StyleAt,
};

const DELAY: Number = Number::new(50.0, (0.0, 2000.0), 0, " ms");

fn kind(shift: Shift) -> ItemKind {
    match shift {
        Shift::Mode => ItemKind::Mode,
        Shift::Layer => ItemKind::Layer,
    }
}

/// The game's modes or layers, and a row to add one.
pub(super) fn list_rows(config: &Config, shift: Shift) -> Vec<Row> {
    let list = shift.list(config.active_game());
    let mut rows: Vec<Row> = list.iter().enumerate().map(|(i, l)| Row::open(format!("{} ({} bound)", l.name, l.buttons.len()), Page::Layer(shift, i))).collect();
    let new = list.len();
    rows.push(Row::add(format!("Add {}", shift.title().to_lowercase()), move |c| {
        let name = named::new_name(c, kind(shift), shift.title());
        let Some(game) = owner::game_mut(c) else { return Act::stay() };
        shift.list_mut(game).push(Layer::new(&name));
        Act::changed().then(Nav::Open(Page::Layer(shift, new)))
    }));
    rows
}

/// Something a mode or layer may override, with whether it does.
fn part_row(config: &Config, owner: Owner, part: Part, label: &str, page: Page) -> Row {
    let own = owner::overrides(config, owner, part);
    Row::open(format!("{label}: {}", if own { "its own" } else { "same as profile" }), page)
}

/// A mode's or layer's page.
pub(super) fn layer_rows(config: &Config, shift: Shift, i: usize) -> Vec<Row> {
    let Some(l) = layer(config, shift, i) else { return Vec::new() };
    let owner = Owner::Shift(shift, i);
    let parts = [
        (Part::Stick(crate::config::Stick::Left), "Left Stick", Page::Stick(owner, crate::config::Stick::Left)),
        (Part::Stick(crate::config::Stick::Right), "Right Stick", Page::Stick(owner, crate::config::Stick::Right)),
        (Part::Trigger(crate::config::Trigger::Left), "Left Trigger", Page::Trigger(owner, crate::config::Trigger::Left)),
        (Part::Trigger(crate::config::Trigger::Right), "Right Trigger", Page::Trigger(owner, crate::config::Trigger::Right)),
        (Part::Gyro, "Gyro", Page::Gyro(owner)),
        (Part::Touchpad, "Touchpad", Page::Touch(owner, crate::config::TouchSide::Left)),
    ];
    let mut rows = vec![
        named::rename_row(config, kind(shift), i),
        Row::open(counted("Buttons", l.buttons.len()), Page::Buttons(owner)),
        Row::open(counted("Combos", l.combos.len()), Page::Combos(owner)),
        Row::open(counted("Profile combos it turns off", l.disabled_combos.len()), Page::OffCombos(shift, i)),
    ];
    rows.extend(parts.into_iter().map(|(part, label, page)| part_row(config, owner, part, label, page)));
    rows.push(Row::toggle("Other buttons do nothing", l.swallow_unbound, move |c, on| layer_mut(c, shift, i).is_some_and(|l| set(&mut l.swallow_unbound, on))));
    rows.extend(indicator_rows(config, shift, i, l));
    rows.push(named::delete_row(config, kind(shift), i, move |c| {
        let Some(game) = owner::game_mut(c) else { return false };
        let list = shift.list_mut(game);
        if i >= list.len() {
            return false;
        }
        list.remove(i);
        true
    }));
    rows
}

/// What shows while the mode or layer is on, after how long, and its heading.
fn indicator_rows(config: &Config, shift: Shift, i: usize, l: &Layer) -> Vec<Row> {
    // Each choice: the name label, the bindings sheet, nothing, or one of the game's info overlays.
    let mut choices = vec![Indicator::Name, Indicator::Bindings, Indicator::Off];
    choices.extend(config.active_game().info.iter().map(|o| Indicator::Info(o.name.clone())));
    let label = match &l.indicator {
        Indicator::Name => "its name".to_string(),
        Indicator::Bindings => "its bindings".into(),
        Indicator::Off => "nothing".into(),
        Indicator::Info(name) => format!("info overlay {name}"),
    };
    let now = l.indicator.clone();
    let mut rows = vec![Row::step(format!("Shows: {label}"), move |c, dir| {
        let at = choices.iter().position(|x| *x == now).unwrap_or(0) as i32;
        let next = choices[(at + dir).rem_euclid(choices.len() as i32) as usize].clone();
        layer_mut(c, shift, i).is_some_and(|l| set(&mut l.indicator, next))
    })];
    if l.indicator != Indicator::Off {
        rows.push(DELAY.row_u("Shows after", u64::from(l.indicator_delay_ms), move |c, v| layer_mut(c, shift, i).is_some_and(|l| set(&mut l.indicator_delay_ms, v as u32))));
    }
    if l.indicator == Indicator::Name {
        rows.push(Row::open("Name label's look", Page::Style(StyleAt::Indicator(shift, i))));
    }
    if l.indicator == Indicator::Bindings {
        let title = l.indicator_title.clone().unwrap_or_default();
        rows.push(Row::text(format!("Heading: {}", if title.is_empty() { "none" } else { &title }), "Heading of the bindings sheet", title, move |c, typed| {
            let typed = typed.trim();
            layer_mut(c, shift, i).is_some_and(|l| set(&mut l.indicator_title, (!typed.is_empty()).then(|| typed.to_string())))
        }));
    }
    rows
}

/// The profile's combos, each ticked when the mode or layer turns it off.
pub(super) fn off_combo_rows(config: &Config, shift: Shift, i: usize) -> Vec<Row> {
    let Some(l) = layer(config, shift, i) else { return Vec::new() };
    let combos: Vec<Combo> = config.active().map(|p| p.combos.clone()).unwrap_or_default();
    if combos.is_empty() {
        return vec![Row::note("The profile has no combos")];
    }
    combos
        .into_iter()
        .map(|combo| {
            let off = l.disabled_combos.contains(&combo.buttons);
            let buttons = combo.buttons.clone();
            Row::apply(format!("{}{}", if off { "turned off: " } else { "" }, combo.action.summary()), move |c| {
                let Some(l) = layer_mut(c, shift, i) else { return Act::stay() };
                match l.disabled_combos.iter().position(|b| *b == buttons) {
                    Some(at) => drop(l.disabled_combos.remove(at)),
                    None => l.disabled_combos.push(buttons.clone()),
                }
                Act::changed()
            })
            .buttons(combo.buttons)
        })
        .collect()
}
