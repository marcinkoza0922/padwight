//! An item's box: the cell behind a menu entry or a key, with its fill (or bevel), outline,
//! glow or shadow, and the style's filters between the box and the contents. This is `item_box`
//! with `lit_cell_style` in `draw/filtered.rs` and `draw/look.rs`, and `Colors::moving` for how
//! it swells and slides.
//!
//! The item is one layer: its stagger fade is the opacity of the whole box, filters and contents
//! together, not of each color in it, and its swell and slide scale and shift that layer.

#[cfg(test)]
mod tests;

use super::{Corners, Fill, Linear, Node, Op, PatternRef, Point, Rect, Rgba, Style, anchor};
use crate::{
    config::OverlayStyle,
    motion::Anim,
    overlay::art::{self, Filter},
};

/// How an item is, from the animation: which of the highlight, flash, fade and swell it has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Item {
    /// The cursor is on it. Its outline, bevel and shadow follow this, not the glide.
    pub selected: bool,
    /// How lit it is, 0 to 1, as the highlight glides in and out (`Anim::lit`).
    pub lit: f32,
    /// How much a pick still flashes it, 0 to 1.
    pub flash: f32,
    /// How visible it is, 0 to 1: its stagger fade as it comes in.
    pub fade: f32,
    /// How big it is drawn against its size (`Anim::item_scale`).
    pub scale: f32,
    /// Pixels it is off its place, sideways, as it comes in.
    pub shift: f32,
}

impl Item {
    /// An item at rest: neither lit nor moving.
    pub const REST: Item = Item { selected: false, lit: 0.0, flash: 0.0, fade: 1.0, scale: 1.0, shift: 0.0 };

    /// Item `index` of a menu whose cursor is on it or not (`selected`), as `anim` has it.
    pub fn of(anim: &Anim, index: usize, selected: bool) -> Item {
        Item {
            selected,
            lit: anim.lit(selected, anim.timing.from == Some(index as u32)),
            flash: if selected { anim.flash() } else { 0.0 },
            fade: anim.item_fade(index),
            scale: anim.item_scale(selected),
            shift: anim.item_shift(index),
        }
    }

    /// The colors for the item's contents and box: `style` (already toned) with the highlight
    /// blended in and the pick flashed. The stagger fade is not in them; `item` fades the layer.
    pub fn colors(&self, style: &Style) -> Style {
        style.lit_item(self.lit, self.flash, 1.0)
    }
}

/// `body` in an item's box `round` pixels rounded (half the item's height at full corners, so a
/// square cell becomes a circle). `colors` are `Item::colors` of the item's style; `config` is
/// the style's own settings, for its filters on items. The body sets the box's size.
pub fn item(body: Node, colors: &Style, state: &Item, config: &OverlayStyle, round: f32) -> Node {
    let mut cell = Node::stack(body);
    for layer in filters(colors, config, round).into_iter().rev() {
        cell = cell.under(layer);
    }
    let cell = cell.under(boxed(*colors, state.selected, round));
    let opacity = state.fade.clamp(0.0, 1.0);
    cell.opacity(opacity).scale(state.scale).shift(Point::new(state.shift, 0.0))
}

/// The box: its shadow (glow, or a bevelled item's), then its fill and outline.
fn boxed(colors: Style, selected: bool, round: f32) -> Node {
    let lit = if selected { 1.0 } else { 0.0 };
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        let look = &colors.look;
        let radii = Corners::all(round);
        let shadow = look.item_shadow(colors.item, lit, colors.anim.clock);
        if shadow.color.a > 0.0 {
            ops.push(Op::Shadow { rect, radii, shadow, spread: 0.0 });
        }
        let fill = match look.item_fill(colors.item, lit) {
            Some((top, bottom)) => Fill::Linear(Linear::new(std::f32::consts::PI).stop(0.0, top).stop(1.0, bottom)),
            None => Fill::Solid(colors.item),
        };
        let border = look.item_outline(colors.item_text, colors.selected_text, selected, lit);
        ops.push(Op::Rect { rect, radii, fill: Some(fill), border: (border.width > 0.0).then_some(border) });
    })
    .width(super::Length::Fill)
    .height(super::Length::Fill)
}

/// The style's filters on items, bottom up, each the size of the item and cut to its corners.
fn filters(colors: &Style, config: &OverlayStyle, round: f32) -> Vec<Node> {
    let fade = colors.fade;
    art::filters(config, true, Some(colors.anim.clock))
        .into_iter()
        .map(|filter| match filter {
            Filter::Pattern(pattern) => {
                let pattern = PatternRef::new(pattern);
                let opacity = (pattern.pattern().opacity * fade).clamp(0.0, 1.0);
                // The picture is the panel's, so a filter runs on across the items.
                layer(move |rect, ops| {
                    let origin = anchor::origin();
                    ops.push(Op::Pattern { pattern: pattern.clone(), rect, origin, cut: art::Shape::Rounded(round), opacity });
                })
            }
            Filter::Tint(tint) => {
                let [r, g, b] = tint.rgb;
                let color = Rgba::from_rgba8(r, g, b, (tint.opacity * fade).clamp(0.0, 1.0));
                layer(move |rect, ops| {
                    ops.push(Op::Rect { rect, radii: Corners::all(round), fill: Some(Fill::Solid(color)), border: None });
                })
            }
        })
        .collect()
}

fn layer(paint: impl Fn(Rect, &mut Vec<Op>) + 'static) -> Node {
    Node::leaf(paint).width(super::Length::Fill).height(super::Length::Fill)
}
