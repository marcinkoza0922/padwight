//! A style's decorations (outlines, shadows, the selected item's glow, the background's
//! gradient), resolved once per panel into colors sized for its scale, and the container styles
//! that draw them.

use iced::{Background, Border, Color, Shadow, Vector, gradient, widget::container};

use super::Colors;
use crate::config::{OverlayStyle, Shade, Stroke};

/// Panels round to this radius at full corners; their size depends on their content.
pub(super) const PANEL_CORNER: f32 = 40.0;

/// What a style adds to its colors, ready to draw.
#[derive(Clone, Copy)]
pub(super) struct Look {
    /// The panel's outline, or `None` for none.
    border: Option<(f32, Color)>,
    shadow: Option<Shadow>,
    /// The second color of the background, its angle in radians, and whether that turns while
    /// the overlay is up.
    gradient: Option<(Color, f32, bool)>,
    /// Every item's outline width and color; unset, the faint default.
    item_border: Option<(f32, Color)>,
    glow: Option<Shadow>,
    /// Whether the glow breathes: its strength swells and fades while the overlay is up.
    glow_pulse: bool,
    /// How deep the items' relief is, 0 to 1, and the style's scale; unset, they're flat.
    bevel: Option<(f32, f32)>,
    /// Drawn without decorations, the default ones included.
    plain: bool,
}

impl Look {
    /// The decorations of `style` at scale `s`. The panel's border defaults to a faint line in
    /// `text`, the panel's own text color.
    pub(super) fn of(style: &OverlayStyle, s: f32, text: Color) -> Look {
        let plain = style.lite.plain;
        if plain {
            return Look { border: None, shadow: None, gradient: None, item_border: None, glow: None, glow_pulse: false, bevel: None, plain };
        }
        let border = match &style.border {
            Some(stroke) => outline(stroke, s),
            None => Some((1.0, Color { a: 0.15, ..text })),
        };
        let shadow = shade(style.shadow.as_ref().unwrap_or(&Shade::panel()), s);
        // Looping effects stop when the performance settings say nothing loops.
        let gradient = style.gradient.as_ref().map(|g| (super::paint(&g.paint(), [0, 0, 0]), g.angle.to_radians(), g.drift && !style.lite.calm));
        // A width of 0 is no outline at all, the selected item's included.
        let item_border = style.item_border.as_ref().map(|stroke| outline(stroke, s).unwrap_or((0.0, Color::TRANSPARENT)));
        let glow = style.glow.as_ref().and_then(|g| shade(g, s));
        let glow_pulse = style.glow.as_ref().is_some_and(|g| g.pulse) && !style.lite.calm;
        let bevel = style.ornament.bevel.as_ref().map(|b| (b.depth.clamp(0.0, 1.0), s));
        Look { border, shadow, gradient, item_border, glow, glow_pulse, bevel, plain }
    }

    /// Every color at `opacity` times its own, as `Colors::faded` does for the rest.
    pub(super) fn faded(self, f: impl Fn(Color) -> Color) -> Look {
        let shadow = |s: Shadow| Shadow { color: f(s.color), ..s };
        Look {
            border: self.border.map(|(w, c)| (w, f(c))),
            shadow: self.shadow.map(shadow),
            gradient: self.gradient.map(|(c, angle, drift)| (f(c), angle, drift)),
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
            Some((to, angle, drift)) => {
                // A drifting gradient sways a little either side of its angle, slowly.
                let angle = if drift { angle + (c.anim.clock * 0.35).sin() * 0.5 } else { angle };
                Background::Gradient(gradient::Linear::new(angle).add_stop(0.0, c.background).add_stop(1.0, to).into())
            }
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
    // A bevelled item with no outline of its own catches light along its edge.
    let (width, color) = match c.look.bevel {
        Some((depth, _)) if c.look.item_border.is_none() && !selected => (1.0, Color { a: 0.12 + 0.3 * depth, ..Color::WHITE }),
        _ => (width, color),
    };
    Border { width, radius: radius.into(), color }
}

/// The selected item's glow, at `lit` of its strength.
pub(super) fn item_glow(c: &Colors, lit: f32) -> Shadow {
    match c.look.glow {
        Some(glow) if lit > 0.0 => {
            // A pulsing glow breathes between 60% and 100% of its strength, every 2.5 seconds.
            let breath = if c.look.glow_pulse { 0.8 + 0.2 * (c.anim.clock * std::f32::consts::TAU / 2.5).sin() } else { 1.0 };
            Shadow { color: Color { a: glow.color.a * lit.min(1.0) * breath, ..glow.color }, ..glow }
        }
        _ => Shadow::default(),
    }
}

/// An item's box, `height` tall, whose colors already include its highlight (see
/// `Colors::lit_item`): its corners round by half of that at full corners, so a square cell
/// becomes a circle.
pub(super) fn lit_cell_style(c: Colors, selected: bool, height: f32) -> impl Fn(&iced::Theme) -> container::Style {
    let lit = if selected { 1.0 } else { 0.0 };
    move |_| container::Style {
        background: Some(item_fill(&c, lit)),
        border: item_outline(&c, selected, lit, c.corners * height / 2.0),
        shadow: item_shadow(&c, lit),
        ..container::Style::default()
    }
}

/// An item's fill: its color, or with a bevel a gradient from a lit top to a darker bottom that
/// flips as the item is pressed in (`lit` of the way).
pub(super) fn item_fill(c: &Colors, lit: f32) -> Background {
    let Some((depth, _)) = c.look.bevel else { return c.item.into() };
    let (up, down) = (super::blend(c.item, Color::WHITE, 0.22 * depth), super::blend(c.item, Color::BLACK, 0.28 * depth));
    let (top, bottom) = (super::blend(up, down, lit), super::blend(down, up, lit));
    // The item's own opacity (its fade-in) carries over to both ends.
    let with_alpha = |x: Color| Color { a: c.item.a, ..x };
    Background::Gradient(gradient::Linear::new(std::f32::consts::PI).add_stop(0.0, with_alpha(top)).add_stop(1.0, with_alpha(bottom)).into())
}

/// An item's shadow: its glow when it is selected, and with a bevel the shadow of a raised
/// button, which sinks away as the item is pressed in.
pub(super) fn item_shadow(c: &Colors, lit: f32) -> Shadow {
    let glow = item_glow(c, lit);
    match c.look.bevel {
        Some((depth, s)) if glow.color.a == 0.0 => Shadow {
            color: Color { a: 0.45 * depth * c.item.a * (1.0 - lit.min(1.0)), ..Color::BLACK },
            offset: Vector::new(0.0, 3.0 * depth * s),
            blur_radius: 5.0 * depth * s,
        },
        _ => glow,
    }
}
