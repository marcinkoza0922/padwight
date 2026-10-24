//! A style's filters, laid where each says it goes: on the panel's background, under its
//! contents, or on each item and key, under its label.

use iced::{
    Element, Length,
    widget::{Container, Stack, container, space},
};

use super::{Colors, ornament, texture};
use crate::config::{FilterOn, OverlayStyle};

/// The style's filters that cover the background (or, with `items`, an item), as layers from
/// the bottom up, rounded `round` pixels at the corners: the materials, then scanlines, grain,
/// pixel blocks, camo, plaid, argyle, stripes and a tint.
pub(super) fn filters<'a, M: 'a>(c: Colors, style: &OverlayStyle, items: bool, round: f32) -> Vec<Element<'a, M>> {
    let mut layers = Vec::new();
    if style.lite.plain {
        return layers;
    }
    let covers = |on: FilterOn| if items { on.items() } else { on.background() };
    let s = style.scale.clamp(0.5, 2.0);
    let o = &style.ornament;
    // Looping effects stop when the performance settings say nothing loops.
    let moving = |on: bool| (on && !style.lite.calm).then_some(c.anim.clock);
    // The first material lies over the ones after it, should several be on.
    for (material, t) in [
        (texture::Material::Wood, &o.wood),
        (texture::Material::Steel, &o.steel),
        (texture::Material::Leather, &o.leather),
        (texture::Material::Silk, &o.silk),
        (texture::Material::Denim, &o.denim),
        (texture::Material::Cloth, &o.cloth),
        (texture::Material::Concrete, &o.concrete),
        (texture::Material::BrushedMetal, &o.brushed_metal),
        (texture::Material::Paper, &o.paper),
    ] {
        if let Some(t) = t.as_ref().filter(|t| covers(t.on)) {
            layers.push(texture::layer(material, t, round, c.fade));
        }
    }
    if let Some(lines) = o.scanlines.as_ref().filter(|l| covers(l.on)) {
        layers.push(texture::scanlines(lines, s, moving(lines.drift), round, c.fade));
    }
    if let Some(grain) = o.grain.as_ref().filter(|g| covers(g.on)) {
        layers.push(texture::grain(grain, s, moving(grain.animated), round, c.fade));
    }
    if let Some(pixels) = o.pixels.as_ref().filter(|p| covers(p.on)) {
        layers.push(texture::pixels(pixels, s, moving(pixels.shimmer), round, c.fade));
    }
    if let Some(camo) = o.camo.as_ref().filter(|p| covers(p.on)) {
        layers.push(texture::camo(camo, s, round, c.fade));
    }
    if let Some(plaid) = o.plaid.as_ref().filter(|p| covers(p.on)) {
        layers.push(texture::plaid(plaid, s, round, c.fade));
    }
    if let Some(argyle) = o.argyle.as_ref().filter(|p| covers(p.on)) {
        layers.push(texture::argyle(argyle, s, round, c.fade));
    }
    if let Some(stripes) = o.stripes.as_ref().filter(|p| covers(p.on)) {
        layers.push(texture::stripes(stripes, s, round, c.fade));
    }
    if let Some(tint) = o.tint.as_ref().filter(|p| covers(p.on)) {
        layers.push(ornament::tint(tint, round, c.fade));
    }
    layers
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
    let layers = filters(c, style, true, round);
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
