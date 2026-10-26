//! A radial item's label: its glyphs, quick-select tag and keyword icon, its words, and a ▸ for
//! a submenu (`item_text` and `item_cell` in `draw/menus.rs`, with their numbers).

use super::super::{
    Align, Corners, Item, Node, Rgba, Style, glyphs::{backed, label, picture}, rich, segment,
};
use crate::menu::{ItemView, MenuView, Tone};

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

/// Item `i`'s label in `colors` (see `colors`) at text size `size`.
pub(super) fn content(item: &ItemView, colors: &Style, size: f32) -> Node {
    let fg = colors.item_text;
    // Glyphs, badges and icons fade in and out with the label.
    let fade = fg.a;
    let mut line = Vec::new();
    // The glyphs are sized from the label, which is 17 units when the scale is 1.
    for b in &item.buttons {
        line.push(segment(&crate::info::button_glyph(*b, colors.family, colors.nintendo_layout), colors, size / 17.0, fade));
    }
    if let Some(b) = item.button.as_deref().filter(|b| !b.is_empty()) {
        let text = Node::container(label(colors, b, size - 2.0, Rgba { a: fade, ..Rgba::BLACK })).padding([1.0, size * 0.4]);
        line.push(backed(text, Corners::all(5.0), Some(Rgba { a: fade, ..Rgba::rgb(0.85, 0.87, 0.9) }), None));
    }
    if let Some(k) = item.keyword {
        line.push(picture(crate::keyword_icon::svg(k), size, fade));
    }
    line.push(rich(&worded(colors, item), colors, size, fg));
    if item.submenu {
        line.push(label(colors, "▸", size, fg.scale_alpha(0.7)));
    }
    Node::row(line).spacing(size * 0.5).align_y(Align::Center)
}

/// An item's label, with a + or − in front of adding and removing rows in color-blind mode.
fn worded(colors: &Style, item: &ItemView) -> String {
    match (colors.colorblind, item.tone) {
        (true, Tone::Add) => format!("+ {}", item.label),
        (true, Tone::Remove) => format!("− {}", item.label),
        _ => item.label.clone(),
    }
}
