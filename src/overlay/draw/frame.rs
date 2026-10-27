//! The marks a frame draws at a panel's four corners. The SVG for each comes from `art`; here
//! it is placed in its corner as an iced widget.

use iced::{
    Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{container, svg},
};

use crate::{
    config::{Frame, Paint},
    overlay::art,
};

/// Where a corner is in the panel, 0 or 1 on each axis; its mark's arms run away from it.
const CORNERS: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

/// The SVG of the frame's mark at `corner`, at scale `s`, in the frame's color.
fn markup(f: &Frame, corner: (f32, f32), s: f32, fade: f32) -> String {
    let ink = Paint::new(&f.color, 1.0).rgb().unwrap_or([0xff, 0xff, 0xff]);
    art::frame::markup(f, ink, corner, s, fade)
}

/// A mark at each of the panel's four corners. `s` is the style's scale.
pub fn marks<'a, M: 'a>(f: &Frame, s: f32, fade: f32) -> Vec<Element<'a, M>> {
    let reach = f.size.max(1.0) * s;
    CORNERS
        .iter()
        .map(|&corner| {
            let mark = svg(svg::Handle::from_memory(markup(f, corner, s, fade).into_bytes())).width(reach).height(reach);
            let horizontal = if corner.0 == 0.0 { Horizontal::Left } else { Horizontal::Right };
            let vertical = if corner.1 == 0.0 { Vertical::Top } else { Vertical::Bottom };
            container(mark).width(Length::Fill).height(Length::Fill).align_x(horizontal).align_y(vertical).into()
        })
        .collect()
}
