//! A panel's style resolved for drawing: its colors, decorations and typography, with how it is
//! moving, as `Colors` holds them for iced.

use super::{Look, Rgba, Type, blend, paint, text_on};
use crate::{
    config::OverlayStyle,
    info::{Glyphs, PadFamily},
    menu::Tone,
    motion::Anim,
};

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub anim: Anim,
    /// The font family, or `None` for the system default (see `font::family`).
    pub font: Option<&'static str>,
    /// The controller in use, for button glyphs.
    pub family: PadFamily,
    /// Face buttons drawn with the Nintendo layout's labels.
    pub nintendo_layout: bool,
    /// Tints for added and removed rows in colors color-blind people can tell apart.
    pub colorblind: bool,
    pub background: Rgba,
    pub background_text: Rgba,
    pub muted: Rgba,
    pub item: Rgba,
    pub item_text: Rgba,
    pub selected: Rgba,
    pub selected_text: Rgba,
    /// 0 square .. 1 round, from the style's corners setting.
    pub corners: f32,
    pub look: Look,
    /// The typography: case, letter spacing and size.
    pub typ: Type,
    /// How much of the panel is shown, 0 to 1: its fade in and out, applied to pictures and
    /// ornaments as it is to colors.
    pub fade: f32,
}

impl Style {
    /// `style` at rest, set in `font`.
    pub fn of(style: &OverlayStyle, font: Option<&'static str>) -> Style {
        let background = paint(&style.background, [0x16, 0x18, 0x1c]);
        let item = paint(&style.items, [0x30, 0x34, 0x3c]);
        let selected = paint(&style.selected, [0x2f, 0x5d, 0xb0]);
        let scale = style.scale.clamp(0.5, 2.0);
        // Each text takes the color that reads on what it sits on.
        let background_text = text_on(background);
        Style {
            anim: Anim::still(),
            font,
            family: PadFamily::default(),
            nintendo_layout: false,
            colorblind: false,
            background,
            background_text,
            muted: Rgba { a: 0.75, ..background_text },
            item,
            item_text: text_on(item),
            selected,
            selected_text: text_on(selected),
            corners: style.corners,
            look: Look::of(style, scale, background_text),
            typ: Type::of(&style.typography, scale),
            fade: 1.0,
        }
    }

    /// This style with button glyphs drawn as `glyphs` says.
    pub fn with_glyphs(self, glyphs: Glyphs) -> Style {
        Style { family: glyphs.family, nintendo_layout: glyphs.nintendo_layout, ..self }
    }

    /// These colors for an item `lit` (0 to 1) and `flash`ing (0 to 1) after a pick, whose
    /// stagger fade is `fade`: its highlight blended in, its pick flashed.
    pub fn lit_item(&self, lit: f32, flash: f32, fade: f32) -> Style {
        let faded = |c: Rgba| Rgba { a: c.a * fade, ..c };
        Style {
            item: faded(blend(blend(self.item, self.selected, lit), Rgba::WHITE, flash * 0.5)),
            item_text: faded(blend(self.item_text, self.selected_text, lit)),
            ..*self
        }
    }

    /// Every color at `opacity` times its own, for fading out.
    pub fn faded(self, opacity: f32) -> Style {
        let f = |c: Rgba| Rgba { a: c.a * opacity.clamp(0.0, 1.0), ..c };
        Style {
            background: f(self.background),
            background_text: f(self.background_text),
            muted: f(self.muted),
            item: f(self.item),
            item_text: f(self.item_text),
            selected: f(self.selected),
            selected_text: f(self.selected_text),
            look: self.look.faded(f),
            fade: self.fade * opacity.clamp(0.0, 1.0),
            ..self
        }
    }

    /// The colors of a row of this kind: adding rows are tinted green and removing ones red, so
    /// they stand out from the items they act on.
    pub fn toned(self, tone: Tone) -> Style {
        // Lime with white text stays readable at this mix (about 4.7:1 on the default items).
        let (tint, share) = match (tone, self.colorblind) {
            (Tone::Normal, _) => return self,
            // No box of its own: muted words on the panel, like the hint under it.
            (Tone::Note, _) => return Style { item: Rgba::TRANSPARENT, item_text: self.muted, ..self },
            (Tone::Add, false) => (Rgba::from_rgb8(0x5a, 0x9a, 0x1a), 0.75),
            (Tone::Remove, false) => (Rgba::from_rgb8(0xd0, 0x64, 0x64), 0.4),
            (Tone::Add, true) => (Rgba::from_rgb8(0x4a, 0x8f, 0xe0), 0.4),
            (Tone::Remove, true) => (Rgba::from_rgb8(0xe8, 0x96, 0x2e), 0.4),
        };
        let mix = |a: f32, b: f32| a + (b - a) * share;
        let item = Rgba { r: mix(self.item.r, tint.r), g: mix(self.item.g, tint.g), b: mix(self.item.b, tint.b), a: self.item.a };
        // The text keeps the faded colors' opacity.
        Style { item, item_text: Rgba { a: self.item_text.a, ..text_on(item) }, ..self }
    }
}
