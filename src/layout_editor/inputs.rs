//! The button pages of Edit Controls: an owner's buttons, a button's gestures, and combos.

use crate::config::{Button, ButtonAction, Combo, Config, GestureKind, SwipeDirection, TouchSide, TouchSlot};

use super::{
    Cx,
    action::{ActionAt, Slot},
    owner::{self, Owner, combos_mut},
    pages::{Page, gesture_name},
    rows::{Act, Nav, Row},
};

/// Each button, with what it does now. A mode or layer shows the buttons it leaves alone as not set.
pub(super) fn button_rows(cx: &Cx, owner: Owner) -> Vec<Row> {
    cx.inputs
        .iter()
        .map(|&b| {
            let action = owner::button(cx.config, owner, b);
            let label = action.as_ref().map_or("not set".into(), ButtonAction::summary);
            Row::open(label, Page::Action(ActionAt::new(Slot::Button(owner, b)))).buttons(vec![b]).keyword(action.as_ref().and_then(ButtonAction::keyword))
        })
        .collect()
}

/// A button's double tap, triple tap, long press, On press and On release.
pub(super) fn gesture_rows(config: &Config, owner: Owner, b: Button) -> Vec<Row> {
    GestureKind::ALL
        .iter()
        .map(|&kind| {
            let action = owner::gestures(config, owner).and_then(|g| g.get(&b)).and_then(|g| g.get(kind));
            let label = format!("{}: {}", gesture_name(kind), action.map_or("not set".into(), ButtonAction::summary));
            Row::open(label, Page::Action(ActionAt::new(Slot::Gesture(owner, b, kind)))).buttons(vec![b])
        })
        .collect()
}

/// The owner's combos, and a row to add one.
pub(super) fn combo_rows(config: &Config, owner: Owner) -> Vec<Row> {
    let combos = owner::combos(config, owner);
    let mut rows: Vec<Row> = combos.iter().enumerate().map(|(i, c)| combo_row(c, Page::Combo(owner, i))).collect();
    let new = combos.len();
    rows.push(Row::add("Add combo", move |c| {
        let Some(combos) = combos_mut(c, owner) else { return Act::stay() };
        combos.push(Combo { buttons: Vec::new(), action: ButtonAction::Disabled });
        Act::changed().then(Nav::Open(Page::Combo(owner, new)))
    }));
    rows
}

/// A combo's row: its buttons' glyphs, then what it does.
pub(super) fn combo_row(combo: &Combo, page: Page) -> Row {
    let label = combo.action.summary();
    if combo.buttons.is_empty() {
        return Row::open(format!("(no buttons yet): {label}"), page);
    }
    Row::open(label, page).buttons(combo.buttons.clone()).keyword(combo.action.keyword())
}

/// A combo's page: each button, ticked when it's in the combo (two or more make it one), its
/// action, and deleting it.
pub(super) fn combo_page(cx: &Cx, owner: Owner, i: usize) -> Vec<Row> {
    let Some(combo) = owner::combos(cx.config, owner).get(i) else { return Vec::new() };
    let order = cx.inputs.clone();
    let mut rows: Vec<Row> = cx
        .inputs
        .iter()
        .map(|&b| {
            let member = combo.buttons.contains(&b);
            let order = order.clone();
            Row::apply(if member { "included" } else { "" }, move |c| {
                let Some(combo) = combos_mut(c, owner).and_then(|cs| cs.get_mut(i)) else { return Act::stay() };
                match combo.buttons.iter().position(|&x| x == b) {
                    Some(at) => drop(combo.buttons.remove(at)),
                    None => {
                        combo.buttons.push(b);
                        combo.buttons.sort_by_key(|x| order.iter().position(|y| y == x));
                    }
                }
                Act::changed()
            })
            .buttons(vec![b])
        })
        .collect();
    rows.push(Row::open(format!("Action: {}", combo.action.summary()), Page::Action(ActionAt::new(Slot::Combo(owner, i)))).keyword(combo.action.keyword()));
    rows.push(Row::delete("Delete combo", move |c| {
        let Some(combos) = combos_mut(c, owner).filter(|cs| i < cs.len()) else { return false };
        combos.remove(i);
        true
    }));
    rows
}

/// What a touchpad slot is called.
pub(super) fn touch_slot_name(slot: TouchSlot) -> String {
    let half = |side| match side {
        TouchSide::Left => "Touchpad",
        TouchSide::Right => "Touchpad right half",
    };
    match slot {
        TouchSlot::Click(side) => format!("{} click", half(side)),
        TouchSlot::Swipe(side, dir) => format!("{} swipe {}", half(side), swipe_name(dir)),
    }
}

pub(super) fn swipe_name(dir: SwipeDirection) -> &'static str {
    match dir {
        SwipeDirection::Up => "up",
        SwipeDirection::Down => "down",
        SwipeDirection::Left => "left",
        SwipeDirection::Right => "right",
    }
}
