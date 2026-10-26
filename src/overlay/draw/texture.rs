//! The procedural pictures that texture a panel or an item, as iced elements: the pictures
//! themselves are made in `overlay::art`. The materials, grain and blocks are translucent white
//! and black, so they lighten and darken whatever they sit on.

use iced::Element;

use crate::overlay::art;

mod anchored;

pub use anchored::anchor;
pub use art::Shape;
use anchored::sized;

/// A picture as a layer the size of what it covers.
pub fn layer<'a, M: 'a>(pattern: art::Pattern, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(pattern, shape, fade)
}

/// A flat wash of the tint's color as a picture, for a shape a box's rounded corners can't draw.
pub fn wash<'a, M: 'a>(t: &art::Tint, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::wash(t), shape, fade)
}
