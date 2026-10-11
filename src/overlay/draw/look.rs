//! A style's decorations (outlines, shadows, the selected item's glow, the background's
//! gradient), resolved once per panel into colors sized for its scale, and the container styles
//! that draw them.

use iced::{Background, Border, Color, Shadow, Vector, gradient, widget::container};

use super::Colors;
use crate::config::{OverlayStyle, Shade, Stroke};

/// Panels round to this radius at full corners; their size depends on their content.
const PANEL_CORNER: f32 = 40.0;

/// What a style adds to its colors, ready to draw.
#[derive(Clone, Copy)]
pub(super) struct Look {
    /// The panel's outline, or `None` for none.
    border: Option<(f32, Color)>,
    shadow: Option<Shadow>,
    /// The second color of the background, and its angle in radians.
    gradient: Option<(Color, f32)>,
    /// Every item's outline width and color; unset, the faint default.
    item_border: Option<(f32, Color)>,
    glow: Option<Shadow>,
    /// Drawn without decorations, the default ones included.
    plain: bool,
}

impl Look {
    /// The decorations of `style` at scale `s`. The panel's border defaults to a faint line in
    /// `text`, the panel's own text color.
    pub(super) fn of(style: &OverlayStyle, s: f32, text: Color) -> Look {
        let plain = style.lite.plain;
        if plain {
            return Look { border: None, shadow: None, gradient: None, item_border: None, glow: None, plain };
        }
        let border = match &style.border {
            Some(stroke) => outline(stroke, s),
            None => Some((1.0, Color { a: 0.15, ..text })),
        };
        let shadow = shade(style.shadow.as_ref().unwrap_or(&Shade::panel()), s);
        let gradient = style.gradient.as_ref().map(|g| (super::paint(&g.paint(), [0, 0, 0]), g.angle.to_radians()));
        // A width of 0 is no outline at all, the selected item's included.
        let item_border = style.item_border.as_ref().map(|stroke| outline(stroke, s).unwrap_or((0.0, Color::TRANSPARENT)));
        let glow = style.glow.as_ref().and_then(|g| shade(g, s));
        Look { border, shadow, gradient, item_border, glow, plain }
    }

    /// Every color at `opacity` times its own, as `Colors::faded` does for the rest.
    pub(super) fn faded(self, f: impl Fn(Color) -> Color) -> Look {
        let shadow = |s: Shadow| Shadow { color: f(s.color), ..s };
        Look {
            border: self.border.map(|(w, c)| (w, f(c))),
            shadow: self.shadow.map(shadow),
            gradient: self.gradient.map(|(c, angle)| (f(c), angle)),
            item_border: self.item_border.map(|(w, c)| (w, f(c))),
            glow: self.glow.map(shadow),
            ..self
        }
    }
}

/// A stroke at scale `s`, or `None` when it draws nothing.
fn outline(stroke: &Stroke, s: f32) -> Option<(f32, Color)> {
    let color = super::paint(&stroke.paint(), [0xff, 0xff, 0xff]);
    (stroke.width > 0.0 && color.a > 0.0).then_some((stroke.width.min(12.0) * s, color))
}

/// A shadow at scale `s`, or `None` when it draws nothing.
fn shade(shade: &Shade, s: f32) -> Option<Shadow> {
    let color = super::paint(&shade.paint(), [0, 0, 0]);
    let moved = shade.x != 0.0 || shade.y != 0.0;
    (color.a > 0.0 && (shade.blur > 0.0 || moved)).then(|| Shadow {
        color,
        offset: Vector::new(shade.x.clamp(-60.0, 60.0) * s, shade.y.clamp(-60.0, 60.0) * s),
        blur_radius: shade.blur.clamp(0.0, 80.0) * s,
    })
}

/// A panel's box: its background (with its gradient), outline and shadow.
pub(super) fn panel_style(c: Colors) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| {
        let look = c.look;
        let background = match look.gradient {
            Some((to, angle)) => Background::Gradient(gradient::Linear::new(angle).add_stop(0.0, c.background).add_stop(1.0, to).into()),
            None => c.background.into(),
        };
        let (width, color) = look.border.unwrap_or((0.0, Color::TRANSPARENT));
        container::Style {
            background: Some(background),
            border: Border { width, radius: (c.corners * PANEL_CORNER).into(), color },
            // The shadow fades with the panel's background, so a see-through panel casts less.
            shadow: look.shadow.map(|s| Shadow { color: Color { a: s.color.a * c.background.a, ..s.color }, ..s }).unwrap_or_default(),
            ..container::Style::default()
        }
    }
}

/// An item's outline: faint, or the selected text's color on the selected item, at the style's
/// width. `lit` runs from 0 (a plain item) to 1 (the selected one) as the highlight glides.
pub(super) fn item_outline(c: &Colors, selected: bool, lit: f32, radius: f32) -> Border {
    let lit_color = |faint: Color| super::blend(faint, c.selected_text, lit);
    let (width, color) = match c.look.item_border {
        Some((width, color)) => (width, lit_color(color)),
        // Plain items are bare, and only the selected one is outlined.
        None if c.look.plain => (if selected { 2.0 } else { 0.0 }, c.selected_text),
        None => (if selected { 2.0 } else { 1.0 }, lit_color(c.item_text.scale_alpha(0.12))),
    };
    Border { width, radius: radius.into(), color }
}

/// The selected item's glow, at `lit` of its strength.
pub(super) fn item_glow(c: &Colors, lit: f32) -> Shadow {
    match c.look.glow {
        Some(glow) if lit > 0.0 => Shadow { color: Color { a: glow.color.a * lit.min(1.0), ..glow.color }, ..glow },
        _ => Shadow::default(),
    }
}

/// An item's box, `height` tall, whose colors already include its highlight (see
/// `Colors::lit_item`): its corners round by half of that at full corners, so a square cell
/// becomes a circle.
pub(super) fn lit_cell_style(c: Colors, selected: bool, height: f32) -> impl Fn(&iced::Theme) -> container::Style {
    let lit = if selected { 1.0 } else { 0.0 };
    move |_| container::Style {
        background: Some(c.item.into()),
        border: item_outline(&c, selected, lit, c.corners * height / 2.0),
        shadow: item_glow(&c, lit),
        ..container::Style::default()
    }
}
