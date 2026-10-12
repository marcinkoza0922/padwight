//! The ornaments drawn over and around an overlay's panel, and the picture behind it.

use serde::{Deserialize, Serialize};

use super::style::{black, opaque};
use super::Stroke;

/// The ornaments drawn over and around a panel: lines between items, a frame at its corners, a
/// scanline screen and a vignette. Each is off unless set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ornament {
    /// A line between the rows of a list or button menu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividers: Option<Stroke>,
    /// Brackets or chamfers at the panel's corners.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    /// Thin lines across the panel, like a screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanlines: Option<Scanlines>,
    /// Darkens the panel's edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vignette: Option<Vignette>,
}

impl Ornament {
    pub fn is_default(&self) -> bool {
        *self == Ornament::default()
    }
}

/// What a frame's corners look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameKind {
    /// An L at each corner, pointing into the panel.
    Brackets,
    /// A corner cut off at 45 degrees.
    Chamfer,
}

/// A frame drawn over each corner: its shape, how big it is, and its color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub kind: FrameKind,
    /// How far each corner's mark reaches, in pixels at scale 1.
    #[serde(default = "default_frame_size")]
    pub size: f32,
    /// Line width in pixels at scale 1.
    #[serde(default = "default_frame_width")]
    pub width: f32,
    #[serde(default = "default_frame_color")]
    pub color: String,
    #[serde(default = "opaque")]
    pub opacity: f32,
}

pub(super) fn default_frame_size() -> f32 {
    14.0
}

pub(super) fn default_frame_width() -> f32 {
    2.0
}

pub(super) fn default_frame_color() -> String {
    "#ffffff".into()
}

/// Thin lines across the panel, `spacing` pixels apart at scale 1. With `drift` they slowly
/// roll down the panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scanlines {
    #[serde(default = "default_scan_spacing")]
    pub spacing: f32,
    #[serde(default = "default_scan_opacity")]
    pub opacity: f32,
    #[serde(default = "black")]
    pub color: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub drift: bool,
}

pub(super) fn default_scan_spacing() -> f32 {
    4.0
}

pub(super) fn default_scan_opacity() -> f32 {
    0.2
}

/// Darkens the panel's edges: `color` at `opacity` on each edge, fading out `reach` percent of
/// the panel's size in from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vignette {
    #[serde(default = "black")]
    pub color: String,
    #[serde(default = "default_vignette_opacity")]
    pub opacity: f32,
    #[serde(default = "default_vignette_reach")]
    pub reach: f32,
}

pub(super) fn default_vignette_opacity() -> f32 {
    0.6
}

pub(super) fn default_vignette_reach() -> f32 {
    25.0
}

/// A picture behind the panel's contents: an image file (a PNG, JPEG or SVG) in the backgrounds
/// folder (see `assets`), or at a full path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundImage {
    pub file: String,
    #[serde(default = "opaque")]
    pub opacity: f32,
    #[serde(default)]
    pub fit: ImageFit,
}

/// How a background picture fills the panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    /// Covers the panel, cropping what sticks out.
    #[default]
    Cover,
    /// Fits inside the panel, leaving bars where it doesn't reach.
    Contain,
    /// Stretched to the panel's shape.
    Stretch,
}
