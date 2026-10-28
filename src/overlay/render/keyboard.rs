//! The on-screen keyboard and numpad (`draw/keyboard.rs`): keys that light up under the cursor
//! and latch when a modifier is held, the legend under them, the closing bar, and the line typed
//! so far or the name of the field. The numbers are iced's.
//!
//! Each key is an item box (`item`), lit by the cursor or the press; a latched key is a green
//! one, and Caps Lock lights up like a latched key while it's on.

#[cfg(test)]
mod tests;

use super::{Align, Item, Length, Look, Node, Outline, Rgba, Style, bar_of, item, rich, text::Spec};
use crate::{
    config::OverlayStyle,
    keyboard::{self, Board, Key},
    overlay::{
        FIELD_LEGEND, KEYBOARD_LEGEND, KeyboardView, NUMPAD_LEGEND,
        fit::Fit,
    },
};

/// A key's width and height at scale 1.
pub const UNIT: f32 = 46.0;
/// The gap between keys at scale 1.
pub const GAP: f32 = 4.0;
/// The space the keyboard's padding takes inside its box, at scale 1.
pub const KEYBOARD_PADDING: f32 = 16.0;

/// The width of a keyboard's body at scale 1: the legend's or the keys', whichever is wider.
pub fn keyboard_body_width(board: Board) -> f32 {
    if board == Board::Numpad {
        // Numpad keys are fewer, so bigger; its body has a floor of 230 at any scale.
        (board.units_wide() * (UNIT * 1.5 + GAP)).max(230.0)
    } else {
        board.units_wide() * (UNIT + GAP)
    }
}

/// The keyboard panel for `v`: its keys, legend, bar and typed line, framed. `style` is `v.style`
/// resolved for drawing (see `Style::of`, `with_glyphs`, `faded`), with the animation in `anim`.
pub fn keyboard_panel(v: &KeyboardView, style: &Style, fit: &Fit) -> Node {
    let s = scale(v, fit);
    let mut body = Node::column([]).spacing(12.0 * s).width(body_width(v, s)).align_x(Align::Center);
    body = typed_line(body, v, style, s);
    body = body.push(keys(v, style, &v.style, s)).push(legend(v, style, s));
    if v.closing > 0.0 {
        body = body.push(bar_of(v.closing, 0.0, 4.0 * s).width(Length::Fill));
    }
    super::framed(body, style, &v.style, KEYBOARD_PADDING * s, fit)
}

/// What the keys and text are scaled by: a set width fits the keys to it, otherwise the style's
/// scale applies.
pub fn scale(v: &KeyboardView, fit: &Fit) -> f32 {
    match fit.width {
        Some(w) => w / (keyboard_body_width(v.board) + 2.0 * KEYBOARD_PADDING),
        None => v.style.scale.clamp(0.5, 2.0),
    }
}

/// A key's unit at scale `s`: numpad keys are fewer, so bigger.
fn unit(board: Board, s: f32) -> f32 {
    (if board == Board::Numpad { UNIT * 1.5 } else { UNIT }) * s
}

/// The body's width: the keys', or the numpad's, which has a floor.
fn body_width(v: &KeyboardView, s: f32) -> f32 {
    let wide = v.board.units_wide() * (unit(v.board, s) + GAP * s);
    if v.board == Board::Numpad { wide.max(230.0 * s) } else { wide }
}

/// What the cursor and the press are doing to a key.
struct KeyState {
    cursor: bool,
    pressed: bool,
    /// Latched (or Caps Lock on) and not under the cursor: drawn green.
    green: bool,
    /// The key's number in reading order.
    here: u32,
}

/// The rows of keys.
fn keys(v: &KeyboardView, style: &Style, config: &OverlayStyle, s: f32) -> Node {
    let (unit, gap) = (unit(v.board, s), GAP * s);
    let cursor_code = keyboard::key_at(v.board, v.cursor).code;
    let shifted = v.latched.iter().any(|l| l == "KEY_LEFTSHIFT" || l == "KEY_RIGHTSHIFT");
    // Keys are numbered in reading order, as the motion tracking numbers them.
    let mut slot = 0u32;
    let rows = v.board.rows().iter().map(|row| {
        let line = row.iter().map(|key| {
            let here = slot;
            slot += 1;
            let width = key.width * (unit + gap) - gap;
            if key.code.is_empty() {
                return Node::space().width(width).height(unit);
            }
            let (cursor, pressed) = (key.code == cursor_code, v.pressed.as_deref() == Some(key.code));
            // Caps Lock lights up like a latched key while it's on.
            let latched = v.latched.iter().any(|l| l == key.code) || (v.caps && key.code == "KEY_CAPSLOCK");
            let state = KeyState { cursor, pressed, green: latched && !(cursor || pressed), here };
            cap(key, v, (style, config), &state, (width, unit, shifted, s))
        });
        Node::row(line.collect::<Vec<_>>()).spacing(gap)
    });
    Node::column(rows.collect::<Vec<_>>()).spacing(gap)
}

/// One key: its label in its box. `size` is its width, height, whether Shift is on, and the scale.
fn cap(key: &Key, v: &KeyboardView, (style, config): (&Style, &OverlayStyle), k: &KeyState, size: (f32, f32, bool, f32)) -> Node {
    let (width, unit, shifted, s) = size;
    let anim = &style.anim;
    let selected = k.cursor || k.pressed;
    // The cursor's key lights up and the one it left goes dark, sliding between them.
    let lit = if k.pressed { 1.0 } else { anim.lit(k.cursor, anim.timing.from == Some(k.here)) };
    let state = Item {
        selected,
        lit,
        flash: if k.pressed { anim.flash() } else { 0.0 },
        fade: 1.0,
        scale: anim.item_scale(k.cursor),
        shift: 0.0,
    };
    let round = style.corners * unit / 2.0;
    let colors = if k.green { latched(style) } else { state.colors(style) };
    let label_size = if v.board == Board::Numpad { 22.0 } else { 14.0 };
    let size = if selected { label_size + 2.0 } else { label_size } * s;
    let spec = Spec { family: style.font, ..Spec::new(keyboard::cap_label(key, v.board, shifted, v.caps), size) };
    let body = Node::text(spec, colors.item_text).center_x(width).center_y(unit);
    item(body, &colors, &state, config, round)
}

/// The style of a latched key: green with white text and a green outline, without relief or glow.
fn latched(style: &Style) -> Style {
    let edge = Outline { width: 1.0, color: Rgba::from_rgb8(0x3f, 0xb9, 0x50) };
    let look = Look { item_border: Some(edge), glow: None, bevel: None, ..style.look };
    Style { item: Rgba::from_rgb8(0x2e, 0x7d, 0x46), item_text: Rgba::WHITE, look, ..*style }
}

/// The legend under the keys: the hints in a centered row that wraps, or a field's footer.
fn legend(v: &KeyboardView, style: &Style, s: f32) -> Node {
    let numpad = v.board == Board::Numpad;
    if v.field.is_some() && !numpad {
        // Typing into the in-game menu: a footer like the menu's own, one line in its size.
        return rich(&FIELD_LEGEND.join(" · "), style, 13.0 * s, style.muted);
    }
    let (hints, gap) = if numpad { (NUMPAD_LEGEND, 14.0) } else { (KEYBOARD_LEGEND, 18.0) };
    let hints = hints.iter().map(|t| rich(t, style, 14.0 * s, style.background_text)).collect::<Vec<_>>();
    Node::wrap(hints).spacing(gap * s).line_spacing(4.0 * s).align_x(Align::Center).align_y(Align::Center)
}

/// What's been typed, above the keys, for when the game's own text box is hidden behind them. A
/// field shows its name, and its text even while empty, so it's clear what is being typed into.
fn typed_line(mut body: Node, v: &KeyboardView, style: &Style, s: f32) -> Node {
    let text = |t: String, size: f32, color: Rgba| Node::text(Spec { family: style.font, ..Spec::new(t, size * s) }, color);
    if let Some(field) = &v.field {
        body = body.push(text(field.clone(), 15.0, style.muted));
    }
    if !v.typed.is_empty() || v.field.is_some() {
        body = body.push(text(format!("{}{}", v.typed, if v.caret { "|" } else { " " }), 18.0, style.background_text));
    }
    body
}
