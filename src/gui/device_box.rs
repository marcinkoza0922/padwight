//! The box an input's settings sit in on the Sticks, Gyro and Touchpad tabs, so the three read
//! alike: a header line with the input's name, its mode and the mode's output options, then the
//! settings in one label column, an "Advanced" fold where there is one, and a "Visualizers" panel.

use iced::widget::{Column, column, row};

use super::*;

/// The space between a box's edge (an input's, or an "Advanced" one in it) and the rows in it.
pub(super) const BOX_PADDING: f32 = 12.0;

/// The label column in a box: narrower by the box's padding, so the editors in it line up with
/// the rows outside it.
pub(super) const ROW_LABEL: f32 = LABEL_WIDTH - BOX_PADDING;

/// The width of the mode dropdown on a box's header line.
const MODE_WIDTH: f32 = 250.0;

/// A labeled row in a box.
pub(super) fn box_row<'a>(label: impl text::IntoFragment<'a>, editor: Element<'a, Message>) -> Element<'a, Message> {
    labeled_in(ROW_LABEL, label, editor)
}

/// A setting that belongs to the one above it (the response's curve settings, smoothing's time),
/// set in under it.
pub(super) fn sub_row<'a>(label: &str, editor: Element<'a, Message>) -> Element<'a, Message> {
    box_row(format!("    {label}"), editor)
}

/// An editor with a ⓘ explaining it beside it.
pub(super) fn with_help<'a>(editor: impl Into<Element<'a, Message>>, why: &str) -> Element<'a, Message> {
    row![editor.into(), help(why.into())].spacing(10).align_y(Alignment::Center).into()
}

/// An input's rows (its editor, or a layer's "Same as base" line) in a box of their own, so each
/// reads as a separate block.
pub(super) fn device_box<'a>(rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    container(Column::with_children(rows).spacing(10)).padding(BOX_PADDING).width(Length::Fill).style(style::inset).into()
}

/// A box's header line: the input's name, its mode picker, and the mode's output options after it.
pub(super) fn box_header<'a, T, L>(name: String, modes: L, mode: T, on_pick: impl Fn(T) -> Message + 'a, options: Vec<Element<'a, Message>>) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
{
    let picker = dropdown(modes, Some(mode), on_pick).width(MODE_WIDTH);
    row![text(name).size(16).width(ROW_LABEL), picker].extend(options).spacing(10).align_y(Alignment::Center).into()
}

/// The output stick option on a header line: "as [Right Stick]".
pub(super) fn as_stick<'a>(stick: Stick, set: impl Fn(Stick) -> Message + 'a) -> [Element<'a, Message>; 2] {
    [text("as").size(13).color(MUTED_COLOR).into(), dropdown([Stick::Left, Stick::Right], Some(stick), set).width(150).into()]
}

/// The space around an "Advanced" fold's header and its rows; with the rows' own inset it adds up
/// to `BOX_PADDING`, so `fold_row` lines up.
const FOLD_PADDING: f32 = 4.0;

/// An "Advanced" fold: a panel whose whole header line opens it, naming what it holds while it's
/// shut, so it's hard to miss. `rows` are its rows when open; `target` keeps it open.
pub(super) fn advanced_fold<'a>(title: &str, holds: &'a str, target: Target, rows: Option<Column<'a, Message>>) -> Element<'a, Message> {
    let chevron = if rows.is_some() { "▾" } else { "▸" };
    let mut line = row![text(format!("{chevron}  {title}")).size(14), space::horizontal()].align_y(Alignment::Center);
    if rows.is_none() {
        line = line.push(text(holds).size(12).color(MUTED_COLOR));
    }
    let header = button(line).style(style::fold_header).padding([6.0, BOX_PADDING - FOLD_PADDING]).width(Length::Fill);
    let mut body = column![header.on_press(Message::ToggleExpanded(target))];
    if let Some(rows) = rows {
        let inner = BOX_PADDING - FOLD_PADDING;
        body = body.push(container(rows.spacing(8)).padding(iced::Padding { top: FOLD_PADDING, right: inner, bottom: inner, left: inner }));
    }
    container(body).padding(FOLD_PADDING).width(Length::Fill).style(style::panel).into()
}

/// A row in an "Advanced" fold. Its label column is narrower by the fold's padding, so its editor
/// lines up with the editors outside it.
pub(super) fn fold_row<'a>(label: impl text::IntoFragment<'a>, editor: Element<'a, Message>) -> Element<'a, Message> {
    labeled_in(ROW_LABEL - BOX_PADDING, label, editor)
}

/// The pictures in a panel of their own, centered in one row that wraps when the window is
/// narrow, with an optional ⓘ by the title and a line above the pictures.
pub(super) fn visualizer_panel<'a>(pics: Vec<Element<'a, Message>>, why: Option<&str>, status: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let mut title = row![text("Visualizers").size(14)].spacing(8).align_y(Alignment::Center);
    if let Some(why) = why {
        title = title.push(help(why.into()));
    }
    let pics = iced::widget::Row::with_children(pics).spacing(20).wrap().vertical_spacing(12);
    let body = column![title].push(status).push(container(pics).center_x(Length::Fill)).spacing(8);
    container(body).padding(BOX_PADDING).width(Length::Fill).style(style::panel).into()
}

/// A picture with a small caption above it and an optional note under it, `width` wide.
pub(super) fn figure<'a>(title: &'a str, picture: impl Into<Element<'a, Message>>, note: Option<String>, width: f32) -> Element<'a, Message> {
    let mut col = column![text(title).size(12).color(MUTED_COLOR), picture.into()].spacing(4).width(width);
    if let Some(note) = note {
        col = col.push(text(note).size(12).color(MUTED_COLOR));
    }
    col.into()
}
