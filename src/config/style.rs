//! How an overlay looks: where it sits, its size and colors, and the decorations that set one
//! game's overlays apart from another's (outlines, shadows, a glow on the selected item, a
//! gradient behind the panel).

use serde::{Deserialize, Serialize};

use super::{BackgroundImage, Ornament, ScreenPosition, Typography, is_zero};

/// A color (`#rrggbb`) with an opacity (0..1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paint {
    pub color: String,
    pub opacity: f32,
}

impl Paint {
    pub fn new(color: &str, opacity: f32) -> Self {
        Paint { color: color.into(), opacity }
    }

    /// The color as RGB, if `color` is a valid `#rrggbb`.
    pub fn rgb(&self) -> Option<[u8; 3]> {
        parse_hex(&self.color)
    }
}

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let hex = s.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// An outline: its width in pixels at scale 1, and its color. A width of 0 draws none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    #[serde(default = "default_stroke_width")]
    pub width: f32,
    #[serde(default = "white")]
    pub color: String,
    #[serde(default = "opaque")]
    pub opacity: f32,
}

impl Stroke {
    pub fn paint(&self) -> Paint {
        Paint::new(&self.color, self.opacity)
    }

    pub fn set_paint(&mut self, paint: Paint) {
        self.color = paint.color;
        self.opacity = paint.opacity;
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke { width: default_stroke_width(), color: white(), opacity: opaque() }
    }
}

/// A shadow, or a glow when it isn't moved: its color, how far it blurs, and how far it is moved
/// right (`x`) and down (`y`), in pixels at scale 1. Opacity 0 draws none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shade {
    #[serde(default = "black")]
    pub color: String,
    #[serde(default = "default_shade_opacity")]
    pub opacity: f32,
    #[serde(default = "default_blur")]
    pub blur: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub y: f32,
    /// A glow that breathes: its strength swells and fades while the overlay is up.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pulse: bool,
}

impl Shade {
    pub fn paint(&self) -> Paint {
        Paint::new(&self.color, self.opacity)
    }

    pub fn set_paint(&mut self, paint: Paint) {
        self.color = paint.color;
        self.opacity = paint.opacity;
    }

    /// The shadow panels have when their style doesn't set one: soft, dark and a little low.
    pub fn panel() -> Self {
        Shade { color: black(), opacity: 0.35, blur: 18.0, x: 0.0, y: 6.0, pulse: false }
    }

    /// A glow in `color`, the same all round.
    pub fn glow(color: &str) -> Self {
        Shade { color: color.into(), opacity: 0.8, blur: default_blur(), x: 0.0, y: 0.0, pulse: false }
    }
}

/// The panel's background fading into a second color. `angle` is the way it runs, in degrees:
/// 0 bottom to top, 90 left to right, 180 top to bottom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    #[serde(default = "black")]
    pub color: String,
    #[serde(default = "opaque")]
    pub opacity: f32,
    #[serde(default = "default_angle")]
    pub angle: f32,
    /// The direction turns slowly while the overlay is up, so the panel seems to shimmer.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub drift: bool,
}

impl Gradient {
    pub fn paint(&self) -> Paint {
        Paint::new(&self.color, self.opacity)
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Gradient { color: black(), opacity: opaque(), angle: default_angle(), drift: false }
    }
}

fn default_stroke_width() -> f32 {
    2.0
}

fn default_shade_opacity() -> f32 {
    0.5
}

fn default_blur() -> f32 {
    16.0
}

fn default_angle() -> f32 {
    180.0
}

pub(super) fn white() -> String {
    "#ffffff".into()
}

pub(super) fn black() -> String {
    "#000000".into()
}

pub(super) fn opaque() -> f32 {
    1.0
}

/// How the overlay process is told to cut overlays down, from the App settings' performance
/// card. The daemon sends it with each frame; it is never part of a saved style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lite {
    /// No decorations: no shadows, gradients, glows, outlines or ornaments, whatever the style
    /// says.
    pub plain: bool,
    /// Nothing loops: glows don't pulse, gradients and scanlines don't drift, grain holds still. Set by the
    /// performance card's Looping animations or Motion switch turned off.
    pub calm: bool,
    /// What share of the display's pixels the overlay window draws at.
    pub resolution: super::performance::Resolution,
}

impl Lite {
    pub fn is_off(&self) -> bool {
        *self == Lite::default()
    }
}

/// How an overlay (a menu, or the on-screen keyboard) looks and where it sits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayStyle {
    #[serde(default)]
    pub position: ScreenPosition,
    /// Size relative to the default, 0.5..2.
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_background")]
    pub background: Paint,
    #[serde(default = "default_items")]
    pub items: Paint,
    #[serde(default = "default_selected")]
    pub selected: Paint,
    /// How round the corners are, 0..1: 0 is square, 1 is a circle wherever the shape allows.
    #[serde(default = "default_corners", skip_serializing_if = "is_default_corners")]
    pub corners: f32,
    /// Width, as a percentage of the display. Unset, the overlay fits its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Length (height), as a percentage of the display. Unset, the overlay fits its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// Moves the overlay right, as a percentage of the display's width (negative moves left).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x_offset: f32,
    /// Moves the overlay down, as a percentage of the display's height (negative moves up).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub y_offset: f32,
    /// Widest the overlay may stretch, as a percentage of the display. Past it, contents wrap onto
    /// the next line, and the overlay grows taller.
    #[serde(default = "default_max_size", skip_serializing_if = "is_default_max_size")]
    pub max_width: f32,
    /// Tallest the overlay may grow, as a percentage of the display. Past it, contents scroll.
    #[serde(default = "default_max_size", skip_serializing_if = "is_default_max_size")]
    pub max_height: f32,
    /// The panel's outline. Unset, a faint 1 px line in the text's color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<Stroke>,
    /// The panel's shadow. Unset, a soft dark one just below it ([`Shade::panel`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shade>,
    /// The background fades into this color across the panel. Unset, it is one color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Gradient>,
    /// Every item's outline, and every key's. The selected item's turns the selected text's color.
    /// Unset, a faint 1 px line, 2 px on the selected item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_border: Option<Stroke>,
    /// A glow (or shadow) around the selected item. Unset, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glow: Option<Shade>,
    /// How the text is set: its case, spacing and size. Weight, slant and color are set inside
    /// the text itself (`{b}`, `{i}`, `{color:…}`).
    #[serde(default, rename = "type", skip_serializing_if = "Typography::is_default")]
    pub typography: Typography,
    /// Lines, frames, filters, textures and bevels over and around the panel.
    #[serde(default, skip_serializing_if = "Ornament::is_default")]
    pub ornament: Ornament,
    /// A picture behind the panel's contents. Unset, the panel is one color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image: Option<BackgroundImage>,
    /// Set by the overlay process from the frame; never read from or written to a file.
    #[serde(skip)]
    pub lite: Lite,
}

fn default_scale() -> f32 {
    1.0
}

fn default_corners() -> f32 {
    0.4
}

fn is_default_corners(corners: &f32) -> bool {
    (corners - default_corners()).abs() < f32::EPSILON
}

/// Most of the display an overlay may cover, unless its style says less.
fn default_max_size() -> f32 {
    90.0
}

fn is_default_max_size(max: &f32) -> bool {
    (max - default_max_size()).abs() < f32::EPSILON
}

fn default_background() -> Paint {
    Paint::new("#16181c", 0.92)
}

fn default_items() -> Paint {
    Paint::new("#30343c", 0.95)
}

fn default_selected() -> Paint {
    Paint::new("#2f5db0", 1.0)
}

impl Default for OverlayStyle {
    fn default() -> Self {
        OverlayStyle {
            position: ScreenPosition::Center,
            scale: default_scale(),
            background: default_background(),
            items: default_items(),
            selected: default_selected(),
            corners: default_corners(),
            width: None,
            height: None,
            x_offset: 0.0,
            y_offset: 0.0,
            max_width: default_max_size(),
            max_height: default_max_size(),
            border: None,
            shadow: None,
            gradient: None,
            item_border: None,
            glow: None,
            typography: Typography::default(),
            ornament: Ornament::default(),
            background_image: None,
            lite: Lite::default(),
        }
    }
}

impl OverlayStyle {
    /// Whether something in the style loops while the overlay is up: a glow that pulses, or a
    /// gradient, scanlines, grain or pixel blocks that drift, flicker or shimmer. The overlay keeps redrawing for these unless the
    /// performance settings say nothing loops.
    pub fn loops(&self) -> bool {
        self.glow.as_ref().is_some_and(|g| g.pulse)
            || self.gradient.as_ref().is_some_and(|g| g.drift)
            || self.ornament.scanlines.as_ref().is_some_and(|s| s.drift)
            || self.ornament.grain.as_ref().is_some_and(|g| g.animated)
            || self.ornament.pixels.as_ref().is_some_and(|p| p.shimmer)
    }

    /// The on-screen keyboard's default: along the bottom of the screen.
    pub fn keyboard() -> Self {
        OverlayStyle { position: ScreenPosition::BottomCenter, ..OverlayStyle::default() }
    }

    /// The on-screen numpad's default: bottom center, like the keyboard.
    pub fn numpad() -> Self {
        OverlayStyle { position: ScreenPosition::BottomCenter, ..OverlayStyle::default() }
    }

    /// The media controls: top center, like the notices.
    pub fn media() -> Self {
        OverlayStyle { position: ScreenPosition::TopCenter, ..OverlayStyle::default() }
    }

    /// A layer's name label: top center, small and see-through.
    pub fn indicator() -> Self {
        OverlayStyle {
            position: ScreenPosition::TopCenter,
            scale: 0.7,
            background: Paint::new("#16181c", 0.7),
            ..OverlayStyle::default()
        }
    }

    /// Info overlays: top right, smaller and see-through so they can stay up during play.
    pub fn info() -> Self {
        OverlayStyle {
            position: ScreenPosition::TopRight,
            scale: 0.8,
            background: Paint::new("#16181c", 0.7),
            ..OverlayStyle::default()
        }
    }

    /// A layer's generated cheat sheet: bottom left, see-through enough to play behind.
    pub fn sheet() -> Self {
        OverlayStyle {
            position: ScreenPosition::BottomLeft,
            scale: 0.8,
            background: Paint::new("#16181c", 0.85),
            ..OverlayStyle::default()
        }
    }

    /// Notices such as the profile just switched to.
    pub fn toast() -> Self {
        OverlayStyle { position: ScreenPosition::TopCenter, background: Paint::new("#16181c", 0.85), ..OverlayStyle::default() }
    }

}

pub(super) fn default_keyboard_style() -> OverlayStyle {
    OverlayStyle::keyboard()
}

pub(super) fn default_numpad_style() -> OverlayStyle {
    OverlayStyle::numpad()
}

pub(super) fn default_media_style() -> OverlayStyle {
    OverlayStyle::media()
}

pub(super) fn default_menu_style() -> OverlayStyle {
    OverlayStyle::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Case, FrameKind, ImageFit, Scanlines};

    #[test]
    fn decorations_are_optional_and_fill_in_what_is_left_out() {
        let style: OverlayStyle = toml::from_str("border = { color = \"#c8a040\" }\nglow = { color = \"#ff0000\", blur = 30 }\n").unwrap();
        assert_eq!(style.border, Some(Stroke { color: "#c8a040".into(), ..Stroke::default() }));
        let glow = style.glow.unwrap();
        assert_eq!((glow.blur, glow.x, glow.opacity), (30.0, 0.0, 0.5));
        assert!(style.shadow.is_none() && style.gradient.is_none());
        let plain = toml::to_string(&OverlayStyle::default()).unwrap();
        for key in ["border", "shadow", "gradient", "item_border", "glow", "lite", "type", "ornament", "background_image"] {
            assert!(!plain.contains(key), "{key} in {plain}");
        }
    }

    #[test]
    fn textures_and_bevels_round_trip() {
        let style: OverlayStyle = toml::from_str("[ornament.brushed_metal]\n\n[ornament.bevel]\n").unwrap();
        assert_eq!(style.ornament.brushed_metal.as_ref().map(|t| t.opacity), Some(0.5));
        assert_eq!(style.ornament.paper, None);
        assert_eq!(style.ornament.bevel.as_ref().map(|b| b.depth), Some(0.5));
        let again: OverlayStyle = toml::from_str(&toml::to_string(&style).unwrap()).unwrap();
        assert_eq!(again.ornament, style.ornament);
    }

    #[test]
    fn filters_round_trip() {
        let style: OverlayStyle = toml::from_str("[ornament.grain]\nanimated = true\n\n[ornament.pixels]\n\n[ornament.tint]\n").unwrap();
        assert!(style.loops());
        assert_eq!(style.ornament.pixels.as_ref().map(|p| p.size), Some(6.0));
        let again: OverlayStyle = toml::from_str(&toml::to_string(&style).unwrap()).unwrap();
        assert_eq!(again.ornament, style.ornament);
    }

    #[test]
    fn typography_ornaments_and_pictures_are_optional_and_round_trip() {
        let style: OverlayStyle = toml::from_str(
            "[type]\ncase = \"upper\"\nletter_spacing = 2\n\n[ornament.frame]\nkind = \"chamfer\"\n\n[background_image]\nfile = \"crest.png\"\n",
        )
        .unwrap();
        assert_eq!((style.typography.case, style.typography.letter_spacing, style.typography.size), (Case::Upper, 2.0, 1.0));
        assert_eq!(style.ornament.frame.as_ref().map(|f| (f.kind, f.size)), Some((FrameKind::Chamfer, 14.0)));
        assert_eq!(style.background_image.as_ref().map(|b| (b.file.as_str(), b.fit)), Some(("crest.png", ImageFit::Cover)));
        let plain = toml::to_string(&OverlayStyle::default()).unwrap();
        assert!(!plain.contains("type") && !plain.contains("ornament") && !plain.contains("background_image"));
        let back: OverlayStyle = toml::from_str(&toml::to_string(&style).unwrap()).unwrap();
        assert_eq!(back, style);
    }

    #[test]
    fn looping_effects_are_flagged_so_the_overlay_keeps_redrawing() {
        assert!(!OverlayStyle::default().loops());
        let pulsing = OverlayStyle { glow: Some(Shade { pulse: true, ..Shade::glow("#ffcc40") }), ..OverlayStyle::default() };
        assert!(pulsing.loops());
        let drifting = OverlayStyle { gradient: Some(Gradient { drift: true, ..Gradient::default() }), ..OverlayStyle::default() };
        assert!(drifting.loops());
        let scanned = OverlayStyle {
            ornament: Ornament { scanlines: Some(Scanlines { spacing: 4.0, opacity: 0.2, color: black(), drift: true, on: crate::config::FilterOn::Both }), ..Ornament::default() },
            ..OverlayStyle::default()
        };
        assert!(scanned.loops());
    }

    #[test]
    fn a_decorated_style_round_trips() {
        let style = OverlayStyle {
            border: Some(Stroke { width: 3.0, ..Stroke::default() }),
            shadow: Some(Shade { y: 10.0, ..Shade::panel() }),
            gradient: Some(Gradient { angle: 90.0, ..Gradient::default() }),
            item_border: Some(Stroke { width: 0.0, ..Stroke::default() }),
            glow: Some(Shade::glow("#2f5db0")),
            ..OverlayStyle::default()
        };
        let back: OverlayStyle = toml::from_str(&toml::to_string(&style).unwrap()).unwrap();
        assert_eq!(back, style);
    }
}
