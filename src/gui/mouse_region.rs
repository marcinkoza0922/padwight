//! A mouse region's settings, shared by the stick and touchpad boxes: its shape, where its middle
//! is on the desktop and how big it is, and its picture.

use iced::widget::{Column, svg};

use super::*;
use crate::{
    config::{MouseRegion, RegionShape},
    figures::region::{HEIGHT, WIDTH, region_svg},
};

/// The region's shape, and its middle and size as shares of the desktop.
pub(super) fn region_rows<'a>(rows: Column<'a, Message>, region: MouseRegion, set: impl Fn(MouseRegion) -> Message + Copy + 'a) -> Column<'a, Message> {
    let why = "The region is part of the desktop, all your monitors together. The pointer goes to the spot in it that \
               matches the stick's or finger's position: the middle for the middle, an edge for an edge.";
    let presets = row![
        button(text("Whole desktop").size(13)).style(style::secondary).on_press(set(region.placed_as(MouseRegion::FULL))),
        button(text("Middle half").size(13)).style(style::secondary).on_press(set(region.placed_as(MouseRegion::default()))),
        help(why.into()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let shape_why = "A circle fits a stick: a full push in any direction reaches its edge. Give it a different width and height \
                     to stretch it into an ellipse. With a rectangle, a full push into a diagonal reaches a corner.";
    let shape = row![dropdown(RegionShape::ALL.to_vec(), Some(region.shape), move |shape| set(MouseRegion { shape, ..region })).width(120), help(shape_why.into())]
        .spacing(10)
        .align_y(Alignment::Center);
    rows.push(box_row("Region", presets.into()))
        .push(sub_row("Shape", shape.into()))
        .push(sub_row("Middle across", fraction_editor(0.0..=1.0, region.x, 0.01, "from the left", move |v| set(MouseRegion { x: v, ..region }))))
        .push(sub_row("Middle down", fraction_editor(0.0..=1.0, region.y, 0.01, "from the top", move |v| set(MouseRegion { y: v, ..region }))))
        .push(sub_row("Width", fraction_editor(0.05..=1.0, region.width, 0.01, "of the desktop", move |v| set(MouseRegion { width: v, ..region }))))
        .push(sub_row("Height", fraction_editor(0.05..=1.0, region.height, 0.01, "of the desktop", move |v| set(MouseRegion { height: v, ..region }))))
}

/// The desktop with the region on it, and where the pointer would be for `pointer` (shares of the
/// desktop).
pub(super) fn region_figure<'a>(region: &MouseRegion, pointer: Option<(f32, f32)>) -> Element<'a, Message> {
    let picture = svg(svg::Handle::from_memory(region_svg(region, pointer).into_bytes())).width(WIDTH).height(HEIGHT);
    figure("Desktop", picture, None, WIDTH)
}
