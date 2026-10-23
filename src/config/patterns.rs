//! The patterned color filters over a panel: plaid, argyle and stripes.

use serde::{Deserialize, Serialize};

use super::FilterOn;

/// A plaid (a tartan): a ground crossed both ways by wide bands and thin lines, woven in a
/// twill, the pattern repeating every `size` pixels at scale 1. `colors` are the ground, the
/// bands and the lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plaid {
    #[serde(default = "default_plaid_colors")]
    pub colors: [String; 3],
    #[serde(default = "default_pattern_opacity")]
    pub opacity: f32,
    #[serde(default = "default_plaid_size")]
    pub size: f32,
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

/// Argyle: diamonds `size` pixels wide at scale 1 in two alternating colors, with thin lines
/// in a third crossing through their middles. `colors` are the two diamonds' and the lines'.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Argyle {
    #[serde(default = "default_argyle_colors")]
    pub colors: [String; 3],
    #[serde(default = "default_pattern_opacity")]
    pub opacity: f32,
    #[serde(default = "default_argyle_size")]
    pub size: f32,
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

/// Stripes `size` pixels wide at scale 1 in two alternating colors, with a thin pinstripe in a
/// third between each two. `colors` are the two stripes' and the pinstripes'.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stripes {
    #[serde(default = "default_stripes_colors")]
    pub colors: [String; 3],
    #[serde(default)]
    pub direction: StripeDirection,
    #[serde(default = "default_pattern_opacity")]
    pub opacity: f32,
    #[serde(default = "default_stripes_size")]
    pub size: f32,
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

/// Which way stripes run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StripeDirection {
    /// Across the panel.
    Horizontal,
    /// Up and down it.
    Vertical,
    /// Slanting up to the right.
    #[default]
    Diagonal,
}

fn colors(ground: &str, bands: &str, lines: &str) -> [String; 3] {
    [ground.into(), bands.into(), lines.into()]
}

/// A red tartan: navy bands and yellow lines.
pub fn default_plaid_colors() -> [String; 3] {
    colors("#a8202a", "#1f2f4a", "#e8c547")
}

/// Navy and camel diamonds under cream lines.
pub fn default_argyle_colors() -> [String; 3] {
    colors("#2f4a6b", "#c9b48a", "#e8e2d0")
}

/// Navy and cream, with red pinstripes.
pub fn default_stripes_colors() -> [String; 3] {
    colors("#1f3a5f", "#e8e2d0", "#b0202a")
}

pub(super) fn default_pattern_opacity() -> f32 {
    0.25
}

pub(super) fn default_plaid_size() -> f32 {
    80.0
}

pub(super) fn default_argyle_size() -> f32 {
    40.0
}

pub(super) fn default_stripes_size() -> f32 {
    16.0
}
