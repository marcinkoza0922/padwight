//! The ornaments drawn over and around an overlay's panel, and the picture behind it.

use serde::{Deserialize, Serialize};

use super::style::{black, opaque};
use super::Stroke;

/// The ornaments drawn over and around a panel: lines between items, a frame at its corners, a
/// scanline screen, a vignette, filters (scanlines, grain, pixels, a tint), a texture and
/// bevelled buttons. Each is off unless set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ornament {
    /// A line between the rows of a list or button menu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dividers: Option<Stroke>,
    /// A mark at each of the panel's corners: brackets, a chamfer or an ornament.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    /// Thin lines across the panel, like a screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanlines: Option<Scanlines>,
    /// Darkens the panel's edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vignette: Option<Vignette>,
    /// Film grain: noise over the panel, optionally flickering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<Grain>,
    /// A mosaic of pixel blocks over the panel, like a low-resolution screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixels: Option<Pixels>,
    /// A camouflage pattern over the panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camo: Option<Camo>,
    /// A wash of color over the panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<Tint>,
    /// A material over the panel's background: paper or brushed metal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<Texture>,
    /// Makes the items (and keys) look like raised buttons.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bevel: Option<Bevel>,
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
    /// A stepped corner with a sunburst and tapering double lines.
    ArtDeco,
    /// A vine sweeping round the corner, with curling tendrils and a leaf.
    ArtNouveau,
    /// A band of runes along each edge, a lozenge at the corner.
    Runic,
    /// A cusped arch across the corner, with a roundel in the spandrel.
    Gothic,
    /// Curling scrolls and leaves around a teardrop.
    Baroque,
    /// Circuit-board traces with pads.
    Circuit,
    /// A riveted iron plate.
    Riveted,
}

impl FrameKind {
    pub const ALL: [FrameKind; 9] = [
        FrameKind::Brackets,
        FrameKind::Chamfer,
        FrameKind::ArtDeco,
        FrameKind::ArtNouveau,
        FrameKind::Runic,
        FrameKind::Gothic,
        FrameKind::Baroque,
        FrameKind::Circuit,
        FrameKind::Riveted,
    ];

    /// The size its detail reads well at, in pixels at scale 1: what picking it sets.
    pub fn natural_size(self) -> f32 {
        match self {
            FrameKind::Brackets | FrameKind::Chamfer => default_frame_size(),
            FrameKind::Riveted => 32.0,
            FrameKind::ArtDeco | FrameKind::Gothic | FrameKind::Circuit => 44.0,
            FrameKind::ArtNouveau | FrameKind::Baroque => 52.0,
            FrameKind::Runic => 60.0,
        }
    }
}

impl std::fmt::Display for FrameKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FrameKind::Brackets => "Brackets",
            FrameKind::Chamfer => "Chamfer",
            FrameKind::ArtDeco => "Art deco",
            FrameKind::ArtNouveau => "Art nouveau",
            FrameKind::Runic => "Runic",
            FrameKind::Gothic => "Gothic",
            FrameKind::Baroque => "Baroque",
            FrameKind::Circuit => "Circuit",
            FrameKind::Riveted => "Riveted",
        })
    }
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

/// What a texture looks like.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureKind {
    /// Fine grain, soft blotches and a few fibres.
    #[default]
    Paper,
    /// Fine streaks along the panel, with a soft sheen.
    BrushedMetal,
}

/// A material drawn over the panel's background and under its contents. It only lightens and
/// darkens, so the background's color still shows through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub kind: TextureKind,
    /// How strong the grain is, 0 to 1.
    #[serde(default = "default_texture_opacity")]
    pub opacity: f32,
}

pub(super) fn default_texture_opacity() -> f32 {
    0.5
}

/// Gives items and keys depth: a lit top edge and a darker bottom under a drop shadow, with
/// the selected one pressed in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bevel {
    /// How pronounced the relief is, 0 to 1.
    #[serde(default = "default_bevel_depth")]
    pub depth: f32,
}

pub(super) fn default_bevel_depth() -> f32 {
    0.5
}

/// Film grain: noise in blocks `size` pixels across at scale 1, `opacity` strong. Animated, it
/// reshuffles a dozen times a second.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grain {
    #[serde(default = "default_grain_opacity")]
    pub opacity: f32,
    #[serde(default = "default_grain_size")]
    pub size: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub animated: bool,
}

pub(super) fn default_grain_opacity() -> f32 {
    0.15
}

pub(super) fn default_grain_size() -> f32 {
    1.0
}

/// Blocks `size` pixels across at scale 1, each a little lighter or darker, with a dark seam
/// between them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pixels {
    #[serde(default = "default_pixels_size")]
    pub size: f32,
    #[serde(default = "default_pixels_opacity")]
    pub opacity: f32,
}

pub(super) fn default_pixels_size() -> f32 {
    6.0
}

pub(super) fn default_pixels_opacity() -> f32 {
    0.25
}

/// A flat wash of `color` over the panel and what's on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tint {
    #[serde(default = "default_tint_color")]
    pub color: String,
    #[serde(default = "default_tint_opacity")]
    pub opacity: f32,
}

pub(super) fn default_tint_color() -> String {
    "#ffa040".into()
}

pub(super) fn default_tint_opacity() -> f32 {
    0.12
}

/// The colors of a camouflage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CamoScheme {
    /// Greens and browns.
    #[default]
    Woodland,
    /// Sand, tan and brown.
    Desert,
    /// Greys.
    Urban,
    /// White and pale blue-grey.
    Arctic,
}

/// A camouflage pattern of soft-edged blotches, `size` pixels across at scale 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camo {
    #[serde(default)]
    pub scheme: CamoScheme,
    #[serde(default = "default_camo_opacity")]
    pub opacity: f32,
    #[serde(default = "default_camo_size")]
    pub size: f32,
}

pub(super) fn default_camo_opacity() -> f32 {
    0.3
}

pub(super) fn default_camo_size() -> f32 {
    60.0
}
