//! A style's typography at its panel's scale, ready to set text with.

use crate::config::{Case, Typography};

/// Letter spacing is capped here, in pixels at scale 1, so a stray value can't tear words apart.
const MAX_SPACING: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    pub case: Case,
    /// The space between letters, in pixels.
    pub spacing: f32,
    /// What every text size is multiplied by.
    pub size: f32,
}

impl Type {
    /// Text as written, at its own size, with no spacing.
    pub const PLAIN: Type = Type { case: Case::None, spacing: 0.0, size: 1.0 };

    pub fn of(typography: &Typography, scale: f32) -> Type {
        Type {
            case: typography.case,
            spacing: typography.letter_spacing.clamp(0.0, MAX_SPACING) * scale,
            size: typography.size.clamp(0.5, 2.0),
        }
    }

    /// `size` as the typography sets it.
    pub fn sized(self, size: f32) -> f32 {
        size * self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_and_size_are_clamped_and_scaled() {
        let ty = Type::of(&Typography { letter_spacing: 3.0, size: 9.0, ..Typography::default() }, 2.0);
        assert_eq!((ty.spacing, ty.sized(10.0)), (6.0, 20.0));
        let negative = Type::of(&Typography { letter_spacing: -4.0, size: 0.1, ..Typography::default() }, 1.0);
        assert_eq!((negative.spacing, negative.sized(10.0)), (0.0, 5.0));
        assert_eq!(Type::of(&Typography::default(), 1.0), Type::PLAIN);
    }
}
