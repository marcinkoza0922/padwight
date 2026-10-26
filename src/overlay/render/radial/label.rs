//! A radial item's colors and whether it is drawn as the cursor's (`item_cell` in
//! `draw/menus.rs`); its label is the menus' (`menus::item_text`).

use super::super::{Item, Style};
use crate::menu::{MenuView, Tone};

/// The colors for item `i`'s box and label: its tone, then (unless it only says something) its
/// highlight blended in and its pick flashed, and `fade` (an arc's label fades with its arc; a
/// box's colors don't, its layer does).
pub(super) fn colors(m: &MenuView, i: usize, style: &Style, state: &Item, fade: f32) -> Style {
    let toned = style.toned(m.items[i].tone);
    if m.items[i].tone == Tone::Note { toned } else { toned.lit_item(state.lit, state.flash, fade) }
}

/// Whether item `i` is drawn as the cursor's: it is, unless it only says something.
pub(super) fn selected(m: &MenuView, i: usize) -> bool {
    m.selected == Some(i) && m.items[i].tone != Tone::Note
}
