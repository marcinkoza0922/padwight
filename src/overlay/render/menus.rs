//! The menus: a panel's heading, its list, buttons, D-pad or face cluster, grid or carousel, the
//! editor's figure beside it, and the about text and hint under it. This is `draw/menus.rs`, with
//! its numbers. The radial menu's body is `radial.rs`'s; a radial menu has no panel here yet.

mod kinds;
#[cfg(test)]
mod tests;

use super::{
    Align, Corners, Item, Length, Node, Point, Rgba, Style, figure, framed, item, plain, rich,
    glyphs::{backed, label, picture, segment},
    text::Spec,
};
use crate::{
    config::OverlayStyle,
    info::{Glyphs, button_glyph},
    menu::{ItemView, MenuView, Tone},
    config::MenuKind,
    overlay::fit::Fit,
    motion::Anim,
};

/// How much of a menu shows through under a question asked over it.
pub const UNDER_PROMPT: f32 = 0.35;

/// The style a menu is drawn in, before it is dimmed under a question: its own settings, set in
/// `font`, for the controller in use and as `anim` has it moving.
pub fn menu_style(m: &MenuView, font: Option<&'static str>, anim: Anim, glyphs: Glyphs, colorblind: bool) -> Style {
    Style { anim, colorblind, ..Style::of(&m.style, font).with_glyphs(glyphs) }
}

/// A menu's panel, or `None` for a radial menu. `solid` is `menu_style`; the menu is dimmed in it
/// when a question is asked over it. Its fade is the screen's, applied to its picture.
pub fn menu_panel(m: &MenuView, solid: &Style, fit: &Fit) -> Option<Node> {
    let s = m.style.scale.clamp(0.5, 2.0);
    let dim = if m.prompt.is_some() { UNDER_PROMPT } else { 1.0 };
    let style = solid.faded(dim);
    let body = match m.kind {
        MenuKind::Radial { .. } => return None,
        MenuKind::Directional { .. } => kinds::directional(m, &style, s),
        MenuKind::List | MenuKind::Buttons => kinds::list(m, &style, s),
        MenuKind::Carousel { .. } => kinds::carousel(m, &style, s),
        MenuKind::Grid { .. } => kinds::grid(m, &style, s),
    };
    Some(frame(m, body, &style, s, fit))
}

/// The panel round a menu's `body`: the editor's figure beside it, the crumbs and title above,
/// and the about text and hint below.
pub fn frame(m: &MenuView, body: Node, style: &Style, s: f32, fit: &Fit) -> Node {
    let body = match &m.figure {
        Some(fig) => Node::row([body, figure(fig, style, s)]).spacing(24.0 * s).align_y(Align::Start),
        None => body,
    };
    // Where this page is in the menus, above its title: "Menu › Edit Controls › A button".
    let title = plain(&m.title, style, 20.0 * s, style.background_text);
    let heading = if m.crumbs.is_empty() {
        title
    } else {
        let crumbs = plain(&m.crumbs.join(" › "), style, 13.0 * s, style.muted);
        Node::column([crumbs, title]).spacing(4.0 * s).align_x(Align::Center)
    };
    let mut contents = Node::column([heading, body]).spacing(14.0 * s).align_x(Align::Center);
    // What the highlighted row means, in the editor's own words, just above the controls.
    if !m.about.is_empty() {
        let spec = Spec { family: style.font, align: Align::Center, ..Spec::new(m.about.clone(), 14.0 * s) };
        contents = contents.push(Node::container(Node::text(spec, style.background_text)).max_width(520.0 * s));
    }
    let contents = contents.push(rich(&m.hint, style, 13.0 * s, style.muted));
    framed(contents, style, &m.style, 22.0 * s, fit)
}

/// An item as the menu draws it: its label, how it is colored and moving, and whether the cursor
/// is on it.
pub(super) struct Cell {
    pub label: Node,
    pub colors: Style,
    pub state: Item,
    pub note: bool,
}

/// Item `i`'s label, its colors (toned, highlighted and flashed) and its animation state.
pub(super) fn cell(m: &MenuView, i: usize, style: &Style, size: f32) -> Cell {
    let view = &m.items[i];
    let note = view.tone == Tone::Note;
    let mut state = Item::of(&style.anim, i, m.selected == Some(i));
    // A note has no box to light or fade, though it still slides and swells with the rest.
    if note {
        state = Item { lit: 0.0, flash: 0.0, fade: 1.0, ..state };
    }
    let toned = style.toned(view.tone);
    let colors = if note { toned } else { state.colors(&toned) };
    state.selected = state.selected && !note;
    // A list's rows are as wide as the list, so their arrows line up at the right.
    let fills = matches!(m.kind, MenuKind::List | MenuKind::Buttons);
    let label = item_text(&colors, view, size, fills);
    Cell { label, colors, state, note }
}

impl Cell {
    /// The cell as `body` makes it of its label, in a box `round` pixels rounded, or bare for a note.
    pub(super) fn boxed(self, body: impl FnOnce(Node) -> Node, config: &OverlayStyle, round: f32) -> Node {
        let body = body(self.label);
        if self.note {
            return body.scale(self.state.scale).shift(Point::new(self.state.shift, 0.0));
        }
        item(body, &self.colors, &self.state, config, round)
    }
}

/// An item's glyphs, its label, and a ▸ for submenus, at the row's right end when it `fills` one.
/// A text badge (a stick's LS, say) is drawn as a small tag. Radial menus label their items so too.
pub(super) fn item_text(style: &Style, view: &ItemView, size: f32, fills: bool) -> Node {
    // Glyphs, badges and icons fade in and out with the label.
    let fade = style.item_text.a;
    let mut parts: Vec<Node> = Vec::new();
    // The glyphs are sized from the label, which is 17 units when the scale is 1.
    for b in &view.buttons {
        parts.push(segment(&button_glyph(*b, style.family, style.nintendo_layout), style, size / 17.0, fade));
    }
    if let Some(b) = view.button.as_deref().filter(|b| !b.is_empty()) {
        let tag = label(style, b, size - 2.0, Rgba::BLACK.scale_alpha(fade));
        let back = Rgba::rgb(0.85, 0.87, 0.9).scale_alpha(fade);
        parts.push(backed(Node::container(tag).padding([1.0, size * 0.4]), Corners::all(5.0), Some(back), None));
    }
    if let Some(k) = view.keyword {
        parts.push(picture(crate::keyword_icon::svg(k), size, fade));
    }
    parts.push(rich(&toned_label(style, view), style, size, style.item_text));
    if view.submenu {
        if fills {
            parts.push(Node::space().width(Length::Fill));
        }
        parts.push(label(style, "▸", size, style.item_text.scale_alpha(0.7)));
    }
    Node::row(parts).spacing(size * 0.5).align_y(Align::Center)
}

/// An item's label, with a + or − in front of adding and removing rows in color-blind mode.
fn toned_label(style: &Style, view: &ItemView) -> String {
    match (style.colorblind, view.tone) {
        (true, Tone::Add) => format!("+ {}", view.label),
        (true, Tone::Remove) => format!("− {}", view.label),
        _ => view.label.clone(),
    }
}
