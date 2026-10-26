//! The procedural pictures that texture a panel or an item, as iced elements: the pictures
//! themselves are made in `overlay::art`. The materials, grain and blocks are translucent white
//! and black, so they lighten and darken whatever they sit on.

use iced::Element;

use crate::{
    config::{Argyle, Camo, Checkerboard, Grain, Plaid, PolkaDots, Pixels, Scanlines, Stripes, Texture, Tint},
    overlay::art,
};

mod anchored;

pub use anchored::anchor;
pub use art::{Material, Shape};
use anchored::sized;

/// The material as a layer the size of what it covers: the relief, repeated if that's bigger.
pub fn layer<'a, M: 'a>(kind: Material, t: &Texture, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::material(kind, t), shape, fade)
}

/// Noise in blocks, as a layer. Given a clock, it picks one of a few patterns by it.
pub fn grain<'a, M: 'a>(g: &Grain, s: f32, clock: Option<f32>, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::grain(g, s, clock), shape, fade)
}

/// Thin lines across, as a layer. Given a clock, they drift down by it.
pub fn scanlines<'a, M: 'a>(sc: &Scanlines, s: f32, clock: Option<f32>, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::scanlines(sc, s, clock), shape, fade)
}

/// Blocks lightened or darkened at random, as a layer. Given a clock, they shimmer.
pub fn pixels<'a, M: 'a>(p: &Pixels, s: f32, clock: Option<f32>, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::pixels(p, s, clock), shape, fade)
}

/// Blotches of three colors on a base, as a layer.
pub fn camo<'a, M: 'a>(c: &Camo, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::camo(c, s), shape, fade)
}

/// A flat wash of the tint's color as a picture, for a shape a box's rounded corners can't draw.
pub fn wash<'a, M: 'a>(t: &Tint, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::wash(t), shape, fade)
}

/// A tartan, as a layer.
pub fn plaid<'a, M: 'a>(p: &Plaid, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::plaid(p, s), shape, fade)
}

/// Diamonds with a line along each diagonal, as a layer.
pub fn argyle<'a, M: 'a>(a: &Argyle, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::argyle(a, s), shape, fade)
}

/// Stripes in two alternating colors, as a layer.
pub fn stripes<'a, M: 'a>(st: &Stripes, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::stripes(st, s), shape, fade)
}

/// Squares in two alternating colors, as a layer.
pub fn checkerboard<'a, M: 'a>(c: &Checkerboard, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::checkerboard(c, s), shape, fade)
}

/// Round dots on a ground, as a layer.
pub fn polka_dots<'a, M: 'a>(d: &PolkaDots, s: f32, shape: Shape, fade: f32) -> Element<'a, M> {
    sized(art::polka_dots(d, s), shape, fade)
}
