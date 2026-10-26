//! A log overlay: one line per sequence of inputs, each input over what it did and how long it
//! was held. This is `log_panel` in `draw/log_panel.rs`, with its numbers.

#[cfg(test)]
mod tests;

use super::{Align, Node, Rgba, Style, info_cell, text::Spec};
use crate::{
    info::Segment,
    inputlog::{LogCell, LogView},
    motion::Anim,
    overlay::fit::Fit,
};

/// How bright a let-go input is next to a held one.
pub const RELEASED: f32 = 0.55;
/// Labels longer than this are cut short, so one long macro name doesn't stretch the line.
const LABEL_CHARS: usize = 22;

/// The panel for `view`, in `font`, held to `fit`, as `anim` has it. A log with no lines is nothing.
pub fn log_panel(view: &LogView, font: Option<&'static str>, fit: &Fit, anim: &Anim) -> Node {
    if view.lines.is_empty() {
        return Node::space();
    }
    let opacity = view.opacity * anim.opacity();
    let style = Style::of(&view.style, font).faded(opacity);
    let s = view.style.scale.clamp(0.5, 2.0);
    let lines = view.lines.iter().map(|line| {
        let cells = line.cells.iter().map(|cell| log_cell(cell, style.faded(line.opacity), s, view, opacity * line.opacity));
        Node::wrap(cells).spacing(12.0 * s).line_spacing(4.0 * s).align_y(Align::Start)
    });
    let body = Node::column(lines).spacing(8.0 * s);
    super::framed(body, &style, &view.style, [10.0 * s, 14.0 * s], fit)
}

fn log_cell(cell: &LogCell, style: Style, s: f32, view: &LogView, opacity: f32) -> Node {
    let (style, opacity) = if cell.held { (style, opacity) } else { (style.faded(RELEASED), opacity * RELEASED) };
    let mut glyph = cell.glyph.clone();
    if cell.count > 1 {
        glyph.push(Segment::Text(format!("×{}", cell.count)));
    }
    let mut col = Node::column([info_cell(&glyph, &style, s, opacity)]).spacing(2.0 * s).align_x(Align::Center);
    if view.show_labels
        && let Some(label) = &cell.label
    {
        col = col.push(text(shorten(label), 12.0 * s, &style, style.background_text));
    }
    if view.show_holds
        && let Some(ms) = cell.hold_ms
    {
        col = col.push(text(format!("{ms} ms"), 11.0 * s, &style, style.muted));
    }
    col
}

fn text(t: String, size: f32, style: &Style, color: Rgba) -> Node {
    Node::text(Spec { family: style.font, ..Spec::new(t, size) }, color)
}

/// `label`, cut to `LABEL_CHARS` characters with an ellipsis.
pub fn shorten(label: &str) -> String {
    if label.chars().count() <= LABEL_CHARS {
        return label.to_string();
    }
    let mut cut: String = label.chars().take(LABEL_CHARS - 1).collect();
    cut.push('…');
    cut
}
