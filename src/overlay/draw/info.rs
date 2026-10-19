//! The info overlays: their cells in a table, and one cell as a chip for the settings window.

use iced::widget::{column, space};

use super::*;

/// An info overlay: its cells in a table, so the rows line up across the columns. A row is as
/// tall as its tallest cell, and a cell's segments wrap onto the next line when the panel is
/// held to its max width, so the whole row grows rather than the cell alone.
pub fn info_panel<'a, M: 'a>(v: &InfoView, font: Font, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let opacity = v.opacity * anim.opacity();
    let c = colors(&v.style, font).faded(opacity);
    let s = v.style.scale.clamp(0.5, 2.0);
    let columns = v.rows.iter().map(Vec::len).max().unwrap_or(0);
    let cols = (0..columns).map(|j| {
        iced::widget::table::column(space(), move |i: usize| -> Element<'a, M> {
            match v.rows[i].get(j) {
                Some(segments) => tag::tag(container(info_cell(segments, c, s, opacity)).align_y(Vertical::Center), Part::Cell(i, j)),
                None => space().into(),
            }
        })
    });
    // Columns are as wide as their widest cell; the table's padding is half the gap between them.
    // A table of no columns divides by zero, so an overlay with no cells yet (a new one) has none.
    let grid: Element<'a, M> = match columns {
        0 => space().into(),
        _ => iced::widget::table(cols, 0..v.rows.len()).padding_x(10.0 * s).padding_y(2.0 * s).separator(0.0).into(),
    };
    let body: Element<'a, M> = match &v.title {
        Some(title) => column![info_cell(title, c, s, opacity), grid].spacing(8.0 * s).into(),
        None => grid,
    };
    framed(body, c, &v.style, [12.0 * s, 16.0 * s], fit)
}

/// `segments` as an info overlay's cell draws them, on a chip of the default panel's color, for
/// the settings window's previews.
pub fn chip<'a, M: 'a>(segments: &[Segment], font: Font) -> Element<'a, M> {
    let c = colors(&OverlayStyle::default(), font);
    let background = Color { a: 1.0, ..c.background };
    container(info_cell(segments, c, 0.85, 1.0))
        .padding([3.0, 8.0])
        .style(move |_: &iced::Theme| container::Style {
            background: Some(background.into()),
            border: Border { radius: 6.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

/// A cell's segments in a row that wraps them onto the next line when they're too wide.
pub(super) fn info_cell<'a, M: 'a>(segments: &[Segment], c: Colors, s: f32, opacity: f32) -> Element<'a, M> {
    marked::line(
        segments,
        c,
        marked::Setting { size: 16.0 * s, color: c.background_text, scale: s, gap: 2.0 * s, opacity, typed: false, trim: false },
    )
}
