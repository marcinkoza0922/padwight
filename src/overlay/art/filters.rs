//! A style's filters as a list, in the order they lie: each a picture or a flat tint. A renderer
//! lays them from the first (the bottom) up, cut to the shape of what they cover.

use super::{Material, Pattern, camo, checkerboard, argyle, grain, material, pixels, plaid, polka_dots, scanlines, stripes};
use crate::config::{FilterOn, OverlayStyle, Paint, Tint as TintConfig};

/// A flat wash of one color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    pub rgb: [u8; 3],
    /// How strongly it shows, before any fade.
    pub opacity: f32,
}

impl Tint {
    fn of(t: &TintConfig) -> Self {
        Self { rgb: Paint::new(&t.color, 1.0).rgb().unwrap_or([0xff, 0xa0, 0x40]), opacity: t.opacity }
    }
}

/// One filter of a style.
pub enum Filter {
    /// A picture spanning what it covers.
    Pattern(Pattern),
    /// A flat color over what it covers.
    Tint(Tint),
}

/// The style's filters that cover the background (or, with `items`, an item), from the bottom up:
/// the materials, then scanlines, grain, pixel blocks, camo, plaid, argyle, stripes, a
/// checkerboard, polka dots and a tint. `clock` is the time in seconds for the effects that move,
/// or `None` for them to stand still. The first material lies over the ones after it, should
/// several be on.
pub fn filters(style: &OverlayStyle, items: bool, clock: Option<f32>) -> Vec<Filter> {
    let mut list = Vec::new();
    if style.lite.plain {
        return list;
    }
    let covers = |on: FilterOn| if items { on.items() } else { on.background() };
    let s = style.scale.clamp(0.5, 2.0);
    let o = &style.ornament;
    // Looping effects stop when the performance settings say nothing loops.
    let moving = |on: bool| if on && !style.lite.calm { clock } else { None };
    for (kind, t) in [
        (Material::Wood, &o.wood),
        (Material::Steel, &o.steel),
        (Material::Leather, &o.leather),
        (Material::Silk, &o.silk),
        (Material::Denim, &o.denim),
        (Material::Cloth, &o.cloth),
        (Material::Concrete, &o.concrete),
        (Material::BrushedMetal, &o.brushed_metal),
        (Material::Paper, &o.paper),
    ] {
        if let Some(t) = t.as_ref().filter(|t| covers(t.on)) {
            list.push(Filter::Pattern(material(kind, t)));
        }
    }
    if let Some(lines) = o.scanlines.as_ref().filter(|l| covers(l.on)) {
        list.push(Filter::Pattern(scanlines(lines, s, moving(lines.drift))));
    }
    if let Some(g) = o.grain.as_ref().filter(|g| covers(g.on)) {
        list.push(Filter::Pattern(grain(g, s, moving(g.animated))));
    }
    if let Some(p) = o.pixels.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(pixels(p, s, moving(p.shimmer))));
    }
    if let Some(p) = o.camo.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(camo(p, s)));
    }
    if let Some(p) = o.plaid.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(plaid(p, s)));
    }
    if let Some(p) = o.argyle.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(argyle(p, s)));
    }
    if let Some(p) = o.stripes.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(stripes(p, s)));
    }
    if let Some(p) = o.checkerboard.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(checkerboard(p, s)));
    }
    if let Some(p) = o.polka_dots.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Pattern(polka_dots(p, s)));
    }
    if let Some(t) = o.tint.as_ref().filter(|p| covers(p.on)) {
        list.push(Filter::Tint(Tint::of(t)));
    }
    list
}
