//! How an overlay's text is set: its case, letter spacing, size, weight, slant and color.

use serde::{Deserialize, Serialize};

use super::is_zero;

/// How an overlay's text is set: its case, spacing, size, weight, slant and color. The defaults
/// draw text as it always has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Typography {
    /// Forces every letter to capitals or to lower case. Titles and labels both.
    pub case: Case,
    /// Space added between letters, in pixels at scale 1 (0 to 12).
    #[serde(skip_serializing_if = "is_zero")]
    pub letter_spacing: f32,
    /// Text size as a factor of the default: 1 is the default, 0.5..2.
    pub size: f32,
    /// Draws the family's bold face. A family with no bold face keeps its regular one.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    /// Draws the family's italic face. A family with no italic face keeps its upright one.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    /// The color (`#rrggbb`) of all the text: titles, hints and items. Unset, the colors chosen
    /// to suit each part (see `text` for the panel alone).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Which letters are forced to capitals or lower case.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    /// As written.
    #[default]
    None,
    Upper,
    Lower,
}

impl Case {
    pub fn apply(self, text: &str) -> String {
        match self {
            Case::None => text.to_string(),
            Case::Upper => text.to_uppercase(),
            Case::Lower => text.to_lowercase(),
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Typography { case: Case::None, letter_spacing: 0.0, size: 1.0, bold: false, italic: false, color: None }
    }
}

impl Typography {
    pub fn is_default(&self) -> bool {
        *self == Typography::default()
    }
}
