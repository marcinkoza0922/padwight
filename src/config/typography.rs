//! How an overlay's text is set: its case, letter spacing and size. Weight, slant, underline,
//! color and outlines are set inside the text itself, with formatting tags (see `info::Mark`).

use serde::{Deserialize, Serialize};

use super::is_zero;

/// How an overlay's text is set: its case, spacing and size. The defaults
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
        Typography { case: Case::None, letter_spacing: 0.0, size: 1.0 }
    }
}

impl Typography {
    pub fn is_default(&self) -> bool {
        *self == Typography::default()
    }
}
