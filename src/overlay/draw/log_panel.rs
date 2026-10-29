//! A log overlay: one line per sequence of inputs, each input over what it did and how long
//! it was held.

use iced::{
    Alignment, Element, Font,
    widget::{column, row, space, text},
};

use super::{Colors, Fit, colors, framed, info_cell};
use crate::{
    info::Segment,
    inputlog::{LogCell, LogView},
    motion::Anim,
    overlay::render::{RELEASED, shorten},
};

pub fn log_panel<'a, M: 'a>(v: &LogView, font: Font, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    if v.lines.is_empty() {
        return space().into();
    }
    let opacity = v.opacity * anim.opacity();
    let c = colors(&v.style, font).faded(opacity);
    let s = v.style.scale.clamp(0.5, 2.0);
    let mut lines = column![].spacing(8.0 * s);
    for line in &v.lines {
        let mut cells = row![].spacing(12.0 * s).align_y(Alignment::Start);
        for cell in &line.cells {
            cells = cells.push(log_cell(cell, c.faded(line.opacity), s, v, opacity * line.opacity));
        }
        lines = lines.push(cells.wrap().vertical_spacing(4.0 * s));
    }
    let body: Element<'a, M> = if v.heading.is_empty() {
        lines.into()
    } else {
        column![info_cell(&[Segment::Text(v.heading.clone())], c, s, opacity), lines].spacing(8.0 * s).into()
    };
    framed(body, c, &v.style, [10.0 * s, 14.0 * s], fit)
}

fn log_cell<'a, M: 'a>(cell: &LogCell, c: Colors, s: f32, v: &LogView, opacity: f32) -> Element<'a, M> {
    let (c, opacity) = if cell.held { (c, opacity) } else { (c.faded(RELEASED), opacity * RELEASED) };
    let mut glyph = cell.glyph.clone();
    if cell.count > 1 {
        glyph.push(Segment::Text(format!("×{}", cell.count)));
    }
    let mut col = column![info_cell(&glyph, c, s, opacity)].spacing(2.0 * s).align_x(Alignment::Center);
    if v.show_labels
        && let Some(label) = &cell.label
    {
        col = col.push(text(shorten(label)).font(c.font).size(12.0 * s).color(c.background_text));
    }
    if v.show_holds
        && let Some(ms) = cell.hold_ms
    {
        col = col.push(text(format!("{ms} ms")).font(c.font).size(11.0 * s).color(c.muted));
    }
    col.into()
}
