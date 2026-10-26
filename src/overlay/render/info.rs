//! An info overlay: its cells in a table, so the rows line up across the columns. This is
//! `info_panel` in `draw/info.rs`, with its numbers.
//!
//! A row is as tall as its tallest cell, and a cell's segments wrap onto the next line when the
//! panel is held to its max width, so the whole row grows rather than the cell alone.

#[cfg(test)]
mod tests;

use super::{Align, Node, Style, info_cell};
use crate::{info::{InfoView, Segment}, motion::Anim, overlay::fit::Fit};

/// The panel for `view`, in `font`, held to `fit`, as `anim` has it.
pub fn info_panel(view: &InfoView, font: Option<&'static str>, fit: &Fit, anim: &Anim) -> Node {
    let opacity = view.opacity * anim.opacity();
    let style = Style::of(&view.style, font).faded(opacity);
    let s = view.style.scale.clamp(0.5, 2.0);
    let (pad_x, pad_y) = (16.0 * s, 12.0 * s);
    let cell = |segments: &[Segment]| info_cell(segments, &style, s, opacity);
    // Cells are padded by half the gap between columns on each side.
    let room = fit.width.unwrap_or(fit.max_width) - 2.0 * pad_x;
    let widths = column_widths(&view.rows, room, 20.0 * s, |c| cell(c).measure(f32::INFINITY).x);
    let rows = view.rows.iter().map(|row| {
        let cells = widths.iter().enumerate().map(|(j, w)| {
            let inner = row.get(j).map_or_else(Node::space, |c| cell(c));
            Node::container(inner).padding([2.0 * s, 10.0 * s]).width(w + 20.0 * s).align_y(Align::Center)
        });
        Node::row(cells).align_y(Align::Center)
    });
    // iced's table has an empty header row of its own, as tall as the padding round a row.
    let mut body = Node::column(std::iter::once(Node::space().height(4.0 * s)).chain(rows));
    if let Some(title) = &view.title {
        body = Node::column([cell(title), body]).spacing(8.0 * s);
    }
    super::framed(body, &style, &view.style, [pad_y, pad_x], fit)
}

/// The width of each column's content: as wide as its widest cell (`natural` of each), unless
/// the table (with `gutter` added to each column) would pass `room`, when the widest columns give
/// up width, down to the next widest, until it fits. A column keeps at least a sliver, so its
/// cells can still wrap.
pub fn column_widths<T>(rows: &[Vec<T>], room: f32, gutter: f32, natural: impl Fn(&T) -> f32) -> Vec<f32> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths: Vec<f32> =
        (0..columns).map(|j| rows.iter().filter_map(|r| r.get(j)).map(&natural).fold(0.0, f32::max).ceil()).collect();
    let mut over = widths.iter().sum::<f32>() + gutter * columns as f32 - room;
    while over > 0.5 {
        let top = widths.iter().copied().fold(0.0, f32::max);
        if top <= 1.0 {
            break;
        }
        let next = widths.iter().copied().filter(|w| *w < top).fold(1.0, f32::max);
        let tied = widths.iter().filter(|w| **w >= top).count() as f32;
        let cut = (top - next).min(over / tied);
        for w in widths.iter_mut().filter(|w| **w >= top) {
            *w -= cut;
        }
        over -= cut * tied;
    }
    widths
}
