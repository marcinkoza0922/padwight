//! The mouse stick's response in two pictures: the pointer's speed against how far the stick is
//! pushed (the acceleration curve), and the pointer's speed in every direction at one push, with
//! the live stick's speed when a controller is connected. Hovering the curve picks the push both
//! show, so the curve's settings can be read off at any point.

use iced::widget::{column, mouse_area, row, space, svg};
use iced::{Element, Length};

use super::*;
use crate::figures::response::{H, PAD, SIDE, W, curve_readout, curve_svg, shape_svg};
pub(super) use crate::figures::response::Shown;

/// Both pictures, for the stick's row of pictures. Hovering the curve sets the push both of them
/// show.
pub(super) fn figures<'a>(shown: Shown) -> [Element<'a, Message>; 2] {
    let curve_pic = mouse_area(svg(svg::Handle::from_memory(curve_svg(&shown).into_bytes())).width(W).height(H))
        .on_move(|at| Message::ProbeStick(((at.x - PAD) / (W - 2.0 * PAD)).clamp(0.0, 1.0)))
        .on_exit(Message::ProbeStick(1.0));
    let shape_pic = svg(svg::Handle::from_memory(shape_svg(&shown).into_bytes())).width(SIDE).height(SIDE);

    let faint = |s: &'static str| text(s).size(11).color(MUTED_COLOR);
    let ticks = row![faint("0%"), space().width(Length::Fill), faint("Stick push"), space().width(Length::Fill), faint("100%")].width(W);
    let sideways = shown.held(shown.push());
    let across = format!("{sideways:.0} px/s across, {:.0} up and down", sideways * shown.response.y_scale);
    [
        figure("Pointer speed", column![curve_pic, ticks].spacing(2), Some(curve_readout(&shown)), W),
        figure("Speed by direction", shape_pic, Some(across), SIDE + 110.0),
    ]
}
