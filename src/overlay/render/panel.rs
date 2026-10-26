//! A panel's frame: the box every overlay panel is drawn in, with its shadow, background (or
//! gradient), border, the style's filters and picture behind the contents, its vignette and frame
//! marks over them, all cut to the rounded box. It holds the panel's contents (a `Node`) and
//! returns the whole panel as a `Node`.
//!
//! This is `framed` in `draw.rs`, with the numbers it has: the box rounds by `PANEL_CORNER` times
//! the style's corners, it is held to the fit's max width and height, and the contents are cut off
//! where the padding below them starts.

mod ornament;
#[cfg(test)]
mod tests;

use super::{Align, Corners, Fill, Length, Linear, Node, Op, PANEL_CORNER, Padding, Point, Rect, Style};
use crate::{config::OverlayStyle, overlay::fit::Fit};

/// The panel for `body`: `padding` around it, in a box at `fit`'s size.
///
/// `style` is the colors and decorations resolved for drawing (already faded for a panel on its
/// way in or out, see `Style::faded`) and `config` the style's own settings, for the ornaments
/// the resolved style doesn't carry (its picture, filters, vignette and frame; its scale sets the
/// frame marks' size). Layers lie in this order, bottom up: the box with its shadow, border and
/// background; the picture; the filters in the order of `art::filters`; the contents; the
/// vignette; the frame marks. Everything but the shadow is cut to the rounded box.
///
/// The contents are measured as tall as they like and cut off at the box's height (the fit's
/// height, or else its max height), less the padding.
pub fn framed(body: Node, style: &Style, config: &OverlayStyle, padding: impl Into<Padding>, fit: &Fit) -> Node {
    let padding = padding.into();
    let radii = Corners::all(style.corners * PANEL_CORNER);
    let cut = (fit.height.unwrap_or(fit.max_height) - padding.top - padding.bottom).max(0.0);
    let height = if fit.height.is_some() { Length::Fill } else { Length::Shrink };
    let contents = Node::container(body).clip(Corners::ZERO).height(height).max_height(cut);
    let base = Node::container(contents).padding(padding).align_x(Align::Center);
    let mut layers = Node::stack(base);
    for layer in ornament::over(style, config) {
        layers = layers.over(layer);
    }
    for layer in ornament::under(style, config).into_iter().rev() {
        layers = layers.under(layer);
    }
    let mut boxed = Node::stack(layers.clip(radii)).under(backdrop(*style, radii)).max_width(fit.max_width).max_height(fit.max_height);
    if let Some(w) = fit.width {
        boxed = boxed.width(w);
    }
    if let Some(h) = fit.height {
        boxed = boxed.height(h);
    }
    boxed
}

/// The box itself: its shadow, then its background and border.
fn backdrop(style: Style, radii: Corners) -> Node {
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        // The panel's filters, and its items', start from here (`anchor`).
        super::anchor::set(Point::new(rect.x, rect.y));
        // The shadow fades with the panel's background, so a see-through panel casts less.
        if let Some(shadow) = style.look.panel_shadow(style.background.a) {
            ops.push(Op::Shadow { rect, radii, shadow, spread: 0.0 });
        }
        let fill = match (style.look.gradient, style.look.gradient_angle(style.anim.clock)) {
            (Some((to, ..)), Some(angle)) => Fill::Linear(Linear::new(angle).stop(0.0, style.background).stop(1.0, to)),
            _ => Fill::Solid(style.background),
        };
        ops.push(Op::Rect { rect, radii, fill: Some(fill), border: style.look.border });
    })
    .width(Length::Fill)
    .height(Length::Fill)
}
