//! The radial preview of a button ring: its sectors around a stick, the inner radius, and the
//! live stick position with the sector it points into highlighted.

use iced::Element;
use iced::widget::svg;

use super::*;
use crate::engine::stick::ring_sector_at;
use crate::figures::ring::{SIZE, render};

/// The preview for a ring, highlighting the sector the live stick points into.
pub(super) fn view<'a>(sectors: u8, start_angle: f32, inner_radius: f32, stick: Option<(f32, f32)>) -> Element<'a, Message> {
    let active = stick.and_then(|pos| ring_sector_at(pos, sectors, start_angle, inner_radius));
    let handle = svg::Handle::from_memory(render(sectors, start_angle, inner_radius, active, stick).into_bytes());
    figure("Ring", svg(handle).width(SIZE).height(SIZE), None, SIZE)
}
