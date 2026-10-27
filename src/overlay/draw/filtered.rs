//! A style's filters, laid where each says it goes: on the panel's background, under its
//! contents, or on each item and key, under its label.

use iced::{
    Element, Length,
    widget::{Container, Stack, container, space},
};

use super::{
    Colors, ornament,
    texture::{self, Shape},
};
use crate::{
    config::OverlayStyle,
    overlay::art::{self, Filter},
};

/// The style's filters that cover the background (or, with `items`, an item), as layers from
/// the bottom up, cut to `shape` (see `art::filters` for what they are and their order).
pub(super) fn filters<'a, M: 'a>(c: Colors, style: &OverlayStyle, items: bool, shape: Shape) -> Vec<Element<'a, M>> {
    art::filters(style, items, Some(c.anim.clock))
        .into_iter()
        .map(|filter| match filter {
            Filter::Pattern(pattern) => texture::layer(pattern, shape, c.fade),
            Filter::Tint(tint) => match shape {
                Shape::Rounded(round) => ornament::tint(&tint, round, c.fade),
                Shape::Arc { .. } => texture::wash(&tint, shape, c.fade),
            },
        })
        .collect()
}

/// An item's or key's box: `content` in a box drawn by `look`, rounded `round` pixels, with the
/// style's filters on items between the box and the content.
pub(super) fn item_box<'a, M: 'a>(
    content: Container<'a, M>,
    look: impl Fn(&iced::Theme) -> container::Style + 'a,
    c: Colors,
    style: &OverlayStyle,
    round: f32,
) -> Element<'a, M> {
    let layers = filters(c, style, true, Shape::Rounded(round));
    if layers.is_empty() {
        return content.style(look).into();
    }
    // The content sets the size; the box and the filters fill it, under the content.
    let mut cell = Stack::new().push(content);
    for layer in layers.into_iter().rev() {
        cell = cell.push_under(layer);
    }
    cell.push_under(container(space()).width(Length::Fill).height(Length::Fill).style(look)).into()
}
