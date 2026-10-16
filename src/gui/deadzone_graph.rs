//! The deadzone in two pictures: where the stick is on its travel, with the dead middle shaded,
//! and the response along one push, flat until the deadzone and then rising to full. When a
//! controller is connected, the live stick is drawn in both: hollow where it is, filled where the
//! deadzone puts it.

use iced::Element;
use iced::widget::svg;

use super::*;
use crate::config::Deadzone;
use crate::figures::deadzone::{SIDE, response_svg, travel_svg};

/// The deadzone's two pictures, for the stick's row of pictures.
pub(super) fn figures<'a>(dz: Deadzone, live: Option<(f32, f32)>) -> [Element<'a, Message>; 2] {
    let travel = svg(svg::Handle::from_memory(travel_svg(dz, live).into_bytes())).width(SIDE).height(SIDE);
    let response = svg(svg::Handle::from_memory(response_svg(dz, live).into_bytes())).width(SIDE).height(SIDE);
    let note = if dz.size <= 0.0 { "No deadzone".to_string() } else { format!("Flat for the first {:.0}%", dz.size * 100.0) };
    [figure("Stick travel", travel, None, SIDE), figure("Response", response, Some(note), SIDE)]
}

/// What the deadzone does, in words, for the Deadzone row's ⓘ.
pub(super) fn explain(deadzone: f32) -> String {
    if deadzone <= 0.0 {
        "No deadzone: every push counts.".to_string()
    } else {
        format!("The first {:.0}% of the stick's travel does nothing. Past it, the stick counts from 0 again.", deadzone * 100.0)
    }
}
