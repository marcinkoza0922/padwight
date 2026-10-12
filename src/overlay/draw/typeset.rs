//! Text in the overlays, set as the style's typography says: its case, size, weight, slant and
//! letter spacing. Iced can't space letters itself, so spaced text is laid out letter by letter,
//! in words that wrap as a line of text does.

use iced::{
    Alignment, Color, Element, Font,
    font::{Style as Slant, Weight},
    widget::{row, space, text},
};

use crate::config::{Case, Typography};

/// Letter spacing is capped here, in pixels at scale 1, so a stray value can't tear words apart.
const MAX_SPACING: f32 = 12.0;

/// A style's typography at its panel's scale, ready to set text with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    case: Case,
    /// The space between letters, in pixels.
    spacing: f32,
    /// What every text size is multiplied by.
    size: f32,
}

impl Type {
    pub fn of(typography: &Typography, scale: f32) -> Type {
        Type {
            case: typography.case,
            spacing: typography.letter_spacing.clamp(0.0, MAX_SPACING) * scale,
            size: typography.size.clamp(0.5, 2.0),
        }
    }

    /// `size` as the typography sets it.
    pub fn size(self, size: f32) -> f32 {
        size * self.size
    }
}

/// `base` with the bold and italic faces the typography asks for. A family without those faces
/// keeps its regular ones.
pub fn font_for(base: Font, typography: &Typography) -> Font {
    Font {
        weight: if typography.bold { Weight::Bold } else { base.weight },
        style: if typography.italic { Slant::Italic } else { base.style },
        ..base
    }
}

/// `t` in the typography's case, at `size` (as the typography scales it), in `color`, with its
/// letter spacing. A line of text that wraps keeps wrapping.
pub fn line<'a, M: 'a>(ty: Type, font: Font, t: &str, size: f32, color: Color) -> Element<'a, M> {
    let t = ty.case.apply(t);
    let size = ty.size(size);
    if ty.spacing <= 0.0 {
        return text(t).font(font).size(size).color(color).into();
    }
    let mut parts: Vec<Element<'a, M>> = Vec::new();
    for (i, word) in t.split(' ').enumerate() {
        if i > 0 {
            // A real space, then the letter spacing after it, as after any letter.
            parts.push(text(" ").font(font).size(size).color(color).into());
            parts.push(space().width(ty.spacing).into());
        }
        let mut letters = row![].spacing(ty.spacing).align_y(Alignment::Center);
        for ch in word.chars() {
            letters = letters.push(text(ch.to_string()).font(font).size(size).color(color));
        }
        parts.push(letters.into());
    }
    row(parts).wrap().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_is_forced_before_anything_is_drawn() {
        assert_eq!(Case::Upper.apply("Ab c"), "AB C");
        assert_eq!(Case::Lower.apply("Ab C"), "ab c");
        assert_eq!(Case::None.apply("Ab C"), "Ab C");
    }

    #[test]
    fn spacing_and_size_are_clamped_and_scaled() {
        let ty = Type::of(&Typography { letter_spacing: 3.0, size: 9.0, ..Typography::default() }, 2.0);
        assert_eq!(ty.spacing, 6.0);
        assert_eq!(ty.size(10.0), 20.0);
        let negative = Type::of(&Typography { letter_spacing: -4.0, size: 0.1, ..Typography::default() }, 1.0);
        assert_eq!((negative.spacing, negative.size(10.0)), (0.0, 5.0));
    }

    #[test]
    fn bold_and_italic_pick_the_faces_asked_for() {
        let base = Font::with_name("Cinzel");
        let typo = Typography { bold: true, italic: true, ..Typography::default() };
        let styled = font_for(base, &typo);
        assert_eq!((styled.weight, styled.style), (Weight::Bold, Slant::Italic));
        assert_eq!(font_for(base, &Typography::default()), base);
    }
}
