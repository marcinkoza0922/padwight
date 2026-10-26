//! A style's decorations (outlines, shadows, the selected item's glow, the background's
//! gradient), resolved once per panel into colors sized for its scale.

use super::{Rgba, blend, paint};
use crate::config::{OverlayStyle, Shade, Stroke};

/// Panels round to this radius at full corners; their size depends on their content.
pub const PANEL_CORNER: f32 = 40.0;

/// A line of `width` pixels in `color`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
    pub width: f32,
    pub color: Rgba,
}

/// A blurred shadow, moved by `offset` (right, down).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Shadow {
    pub color: Rgba,
    pub offset: (f32, f32),
    pub blur: f32,
}

/// What a style adds to its colors, ready to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The panel's outline, or `None` for none.
    pub border: Option<Outline>,
    pub shadow: Option<Shadow>,
    /// The second color of the background, its angle in radians, and whether that turns while
    /// the overlay is up.
    pub gradient: Option<(Rgba, f32, bool)>,
    /// Every item's outline width and color; unset, the faint default.
    pub item_border: Option<Outline>,
    pub glow: Option<Shadow>,
    /// Whether the glow breathes: its strength swells and fades while the overlay is up.
    pub glow_pulse: bool,
    /// How deep the items' relief is, 0 to 1, and the style's scale; unset, they're flat.
    pub bevel: Option<(f32, f32)>,
    /// Drawn without decorations, the default ones included.
    pub plain: bool,
}

impl Look {
    /// The decorations of `style` at scale `s`. The panel's border defaults to a faint line in
    /// `text`, the panel's own text color.
    pub fn of(style: &OverlayStyle, s: f32, text: Rgba) -> Look {
        let plain = style.lite.plain;
        if plain {
            return Look { border: None, shadow: None, gradient: None, item_border: None, glow: None, glow_pulse: false, bevel: None, plain };
        }
        let border = match &style.border {
            Some(stroke) => outline(stroke, s),
            None => Some(Outline { width: 1.0, color: Rgba { a: 0.15, ..text } }),
        };
        let shadow = shade(style.shadow.as_ref().unwrap_or(&Shade::panel()), s);
        // Looping effects stop when the performance settings say nothing loops.
        let gradient = style.gradient.as_ref().map(|g| (paint(&g.paint(), [0, 0, 0]), g.angle.to_radians(), g.drift && !style.lite.calm));
        // A width of 0 is no outline at all, the selected item's included.
        let item_border = style.item_border.as_ref().map(|stroke| outline(stroke, s).unwrap_or(Outline { width: 0.0, color: Rgba::TRANSPARENT }));
        let glow = style.glow.as_ref().and_then(|g| shade(g, s));
        let glow_pulse = style.glow.as_ref().is_some_and(|g| g.pulse) && !style.lite.calm;
        let bevel = style.ornament.bevel.as_ref().map(|b| (b.depth.clamp(0.0, 1.0), s));
        Look { border, shadow, gradient, item_border, glow, glow_pulse, bevel, plain }
    }

    /// Every color through `f`, as fading the rest of the style does.
    pub fn faded(self, f: impl Fn(Rgba) -> Rgba) -> Look {
        let shadow = |s: Shadow| Shadow { color: f(s.color), ..s };
        let outline = |o: Outline| Outline { color: f(o.color), ..o };
        Look {
            border: self.border.map(outline),
            shadow: self.shadow.map(shadow),
            gradient: self.gradient.map(|(c, angle, drift)| (f(c), angle, drift)),
            item_border: self.item_border.map(outline),
            glow: self.glow.map(shadow),
            ..self
        }
    }

    /// The background gradient's angle in radians at `clock`: a drifting one sways a little
    /// either side of its angle, slowly. `None` without a gradient.
    pub fn gradient_angle(&self, clock: f32) -> Option<f32> {
        self.gradient.map(|(_, angle, drift)| if drift { angle + (clock * 0.35).sin() * 0.5 } else { angle })
    }

    /// The panel's shadow, fading with the panel's background (`alpha`), so a see-through panel
    /// casts less.
    pub fn panel_shadow(&self, alpha: f32) -> Option<Shadow> {
        self.shadow.map(|s| Shadow { color: Rgba { a: s.color.a * alpha, ..s.color }, ..s })
    }

    /// An item's outline: faint, or the selected text's color on the selected item, at the
    /// style's width. `lit` runs from 0 (a plain item) to 1 (the selected one) as the highlight
    /// glides. `item_text` and `selected_text` are the style's colors for those.
    pub fn item_outline(&self, item_text: Rgba, selected_text: Rgba, selected: bool, lit: f32) -> Outline {
        let lit_color = |faint: Rgba| blend(faint, selected_text, lit);
        let (width, color) = match self.item_border {
            Some(o) => (o.width, lit_color(o.color)),
            // Plain items are bare, and only the selected one is outlined.
            None if self.plain => (if selected { 2.0 } else { 0.0 }, selected_text),
            None => (if selected { 2.0 } else { 1.0 }, lit_color(item_text.scale_alpha(0.12))),
        };
        // A bevelled item with no outline of its own catches light along its edge.
        match self.bevel {
            Some((depth, _)) if self.item_border.is_none() && !selected => Outline { width: 1.0, color: Rgba { a: 0.12 + 0.3 * depth, ..Rgba::WHITE } },
            _ => Outline { width, color },
        }
    }

    /// The selected item's glow, at `lit` of its strength, `clock` seconds into the overlay.
    pub fn item_glow(&self, lit: f32, clock: f32) -> Shadow {
        match self.glow {
            Some(glow) if lit > 0.0 => {
                // A pulsing glow breathes between 60% and 100% of its strength, every 2.5 seconds.
                let breath = if self.glow_pulse { 0.8 + 0.2 * (clock * std::f32::consts::TAU / 2.5).sin() } else { 1.0 };
                Shadow { color: Rgba { a: glow.color.a * lit.min(1.0) * breath, ..glow.color }, ..glow }
            }
            _ => Shadow::default(),
        }
    }

    /// An item's fill as two colors (top, bottom) when it has a bevel: a lit top to a darker
    /// bottom that flips as the item is pressed in (`lit` of the way). Both ends take `item`'s
    /// own opacity (its fade-in). `None` for a flat item, which is just `item`.
    pub fn item_fill(&self, item: Rgba, lit: f32) -> Option<(Rgba, Rgba)> {
        let (depth, _) = self.bevel?;
        let (up, down) = (blend(item, Rgba::WHITE, 0.22 * depth), blend(item, Rgba::BLACK, 0.28 * depth));
        let (top, bottom) = (blend(up, down, lit), blend(down, up, lit));
        Some((Rgba { a: item.a, ..top }, Rgba { a: item.a, ..bottom }))
    }

    /// An item's shadow: its glow when it is selected, and with a bevel the shadow of a raised
    /// button, which sinks away as the item is pressed in.
    pub fn item_shadow(&self, item: Rgba, lit: f32, clock: f32) -> Shadow {
        let glow = self.item_glow(lit, clock);
        match self.bevel {
            Some((depth, s)) if glow.color.a == 0.0 => Shadow {
                color: Rgba { a: 0.45 * depth * item.a * (1.0 - lit.min(1.0)), ..Rgba::BLACK },
                offset: (0.0, 3.0 * depth * s),
                blur: 5.0 * depth * s,
            },
            _ => glow,
        }
    }
}

/// A stroke at scale `s`, or `None` when it draws nothing.
fn outline(stroke: &Stroke, s: f32) -> Option<Outline> {
    let color = paint(&stroke.paint(), [0xff, 0xff, 0xff]);
    (stroke.width > 0.0 && color.a > 0.0).then_some(Outline { width: stroke.width.min(12.0) * s, color })
}

/// A shadow at scale `s`, or `None` when it draws nothing.
fn shade(shade: &Shade, s: f32) -> Option<Shadow> {
    let color = paint(&shade.paint(), [0, 0, 0]);
    let moved = shade.x != 0.0 || shade.y != 0.0;
    (color.a > 0.0 && (shade.blur > 0.0 || moved)).then(|| Shadow {
        color,
        offset: (shade.x.clamp(-60.0, 60.0) * s, shade.y.clamp(-60.0, 60.0) * s),
        blur: shade.blur.clamp(0.0, 80.0) * s,
    })
}
