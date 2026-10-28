//! The on-screen keyboard and numpad: their keys, which light up under the cursor and latch
//! when a modifier is held, and their legend.

use iced::{
    Alignment, Color, Element, Font,
    alignment::Horizontal,
    widget::{column, container, progress_bar, row, space, text},
};

use super::*;
use crate::{
    keyboard,
    overlay::{FIELD_LEGEND, KEYBOARD_LEGEND, NUMPAD_LEGEND, render::{KEYBOARD_PADDING, keyboard_body_width}},
};

#[expect(clippy::too_many_lines, reason = "predates the size lints")]
pub fn keyboard_panel<'a, M: 'a>(v: &KeyboardView, font: Font, glyphs: crate::info::Glyphs, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let c = colors(&v.style, font).with_glyphs(glyphs).faded(anim.opacity());
    let numpad = v.board == crate::keyboard::Board::Numpad;
    // A set width fits the keys (and their text) to it; otherwise the style's scale applies.
    let s = match fit.width {
        Some(w) => w / (keyboard_body_width(v.board) + 2.0 * KEYBOARD_PADDING),
        None => v.style.scale.clamp(0.5, 2.0),
    };
    // Numpad keys are fewer, so bigger.
    let unit = if numpad { UNIT * 1.5 } else { UNIT } * s;
    let gap = GAP * s;
    let cursor_code = keyboard::key_at(v.board, v.cursor).code;
    let shifted = v.latched.iter().any(|l| l == "KEY_LEFTSHIFT" || l == "KEY_RIGHTSHIFT");
    let label_size = if numpad { 22.0 } else { 14.0 };
    let mut rows = column![].spacing(gap);
    // Keys are numbered in reading order, as the motion tracking numbers them.
    let mut slot = 0u32;
    for keys in v.board.rows().iter() {
        let mut line = row![].spacing(gap);
        for key in keys.iter() {
            let here = slot;
            slot += 1;
            let width = key.width * (unit + gap) - gap;
            if key.code.is_empty() {
                line = line.push(space().width(width).height(unit));
                continue;
            }
            let cursor = key.code == cursor_code;
            let pressed = v.pressed.as_deref() == Some(key.code);
            let selected = cursor || pressed;
            // Caps Lock lights up like a latched key while it's on.
            let latched = v.latched.iter().any(|l| l == key.code) || (v.caps && key.code == "KEY_CAPSLOCK");
            // The cursor's key lights up and the one it left goes dark, sliding between them.
            let lit = if pressed { 1.0 } else { anim.lit(cursor, anim.timing.from == Some(here)) };
            let round = c.corners * unit / 2.0;
            let (bg, fg, border, glow): (iced::Background, _, _, _) = if latched && !selected {
                let green = Color::from_rgb8(0x2e, 0x7d, 0x46);
                (green.into(), Color::WHITE, Border { width: 1.0, radius: round.into(), color: Color::from_rgb8(0x3f, 0xb9, 0x50) }, iced::Shadow::default())
            } else {
                let flash = if pressed { anim.flash() * 0.5 } else { 0.0 };
                (
                    look::item_fill(&Colors { item: blend(blend(c.item, c.selected, lit), Color::WHITE, flash), ..c }, lit),
                    blend(c.item_text, c.selected_text, lit),
                    look::item_outline(&c, selected, lit, round),
                    look::item_shadow(&c, lit),
                )
            };
            let cap = container(text(keyboard::cap_label(key, v.board, shifted, v.caps)).font(c.font).size(if selected { label_size + 2.0 } else { label_size } * s).color(fg))
                .center_x(width)
                .center_y(unit);
            let cap = item_box(cap, move |_| container::Style { background: Some(bg), border, shadow: glow, ..container::Style::default() }, c, &v.style, round);
            line = line.push(transform(cap, anim.item_scale(cursor), iced::Vector::ZERO));
        }
        rows = rows.push(line);
    }
    let hint = |t: &'static str| rich_text::<M>(t, c, 14.0 * s, c.background_text);
    let (legend, width): (Element<'a, M>, f32) = if numpad {
        let legend = row(NUMPAD_LEGEND.iter().map(|t| hint(t))).spacing(14.0 * s).wrap().vertical_spacing(4.0 * s).align_x(Horizontal::Center);
        (legend.into(), (v.board.units_wide() * (unit + gap)).max(230.0 * s))
    } else if v.field.is_some() {
        // Typing into the in-game menu: a footer like the menu's own, one line in its size.
        (rich_text::<M>(&FIELD_LEGEND.join(" · "), c, 13.0 * s, c.muted), v.board.units_wide() * (unit + gap))
    } else {
        let legend = row(KEYBOARD_LEGEND.iter().map(|t| hint(t)))
        .spacing(18.0 * s)
        .align_y(Alignment::Center)
        .wrap()
        .vertical_spacing(4.0 * s)
        .align_x(Horizontal::Center);
        (legend.into(), v.board.units_wide() * (unit + gap))
    };
    let mut body = column![].spacing(12.0 * s).width(width).align_x(Alignment::Center);
    body = typed_line(body, v, &c, s);
    body = body.push(rows).push(legend);
    if v.closing > 0.0 {
        body = body.push(progress_bar(0.0..=1.0, v.closing).girth(4.0 * s));
    }
    framed(body.into(), c, &v.style, KEYBOARD_PADDING * s, fit)
}

/// What's been typed, above the keys, for when the game's own text box is hidden behind them. A
/// field shows its name, and its text even while empty, so it's clear what is being typed into.
fn typed_line<'a, M: 'a>(mut body: iced::widget::Column<'a, M>, v: &KeyboardView, c: &Colors, s: f32) -> iced::widget::Column<'a, M> {
    if let Some(field) = &v.field {
        body = body.push(text(field.clone()).font(c.font).size(15.0 * s).color(c.muted));
    }
    if !v.typed.is_empty() || v.field.is_some() {
        body = body.push(text(format!("{}{}", v.typed, if v.caret { "|" } else { " " })).font(c.font).size(18.0 * s).color(c.background_text));
    }
    body
}
