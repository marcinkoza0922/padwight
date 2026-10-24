//! The ornaments drawn over and around an overlay's panel, and the picture behind it.

use serde::{Deserialize, Serialize};

use super::style::{black, opaque};
use super::{Argyle, Camo, Checkerboard, Grain, Pixels, Plaid, PolkaDots, Scanlines, Stripes, Stroke, Texture, Tint};

/// The ornaments drawn over and around a panel: lines between items, a frame at its corners, a
/// vignette, filters (scanlines, grain, pixels, camo, plaid, argyle, stripes, a checkerboard,
/// polka dots, a tint and the materials, each on the background, the items or both) and
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
    /// Thin lines across, like a screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanlines: Option<Scanlines>,
    /// Darkens the panel's edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vignette: Option<Vignette>,
    /// Film grain: noise, optionally flickering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grain: Option<Grain>,
    /// A mosaic of pixel blocks, like a low-resolution screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixels: Option<Pixels>,
    /// A camouflage pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camo: Option<Camo>,
    /// A plaid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaid: Option<Plaid>,
    /// An argyle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argyle: Option<Argyle>,
    /// Stripes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stripes: Option<Stripes>,
    /// A checkerboard.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkerboard: Option<Checkerboard>,
    /// Polka dots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polka_dots: Option<PolkaDots>,
    /// A wash of color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<Tint>,
    /// Paper on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper: Option<Texture>,
    /// Brushed metal on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brushed_metal: Option<Texture>,
    /// Concrete on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concrete: Option<Texture>,
    /// Woven cloth on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloth: Option<Texture>,
    /// Denim twill on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denim: Option<Texture>,
    /// Lustrous silk on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silk: Option<Texture>,
    /// Pebbled leather on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leather: Option<Texture>,
    /// Steel tread plate on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steel: Option<Texture>,
    /// Wooden planks on the panel's background or items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood: Option<Texture>,
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
    /// Two strands woven over and under each other, through a ring at the corner.
    Celtic,
    /// A running Greek key between two rails, a boxed square at the corner.
    GreekKey,
    /// A lotus frieze between two rails, a sun disc boxed at the corner.
    Egyptian,
    /// A blossom at the corner, a leafy vine running from it along each edge.
    Floral,
}

impl FrameKind {
    pub const ALL: [FrameKind; 13] = [
        FrameKind::Brackets,
        FrameKind::Chamfer,
        FrameKind::ArtDeco,
        FrameKind::ArtNouveau,
        FrameKind::Runic,
        FrameKind::Gothic,
        FrameKind::Baroque,
        FrameKind::Circuit,
        FrameKind::Riveted,
        FrameKind::Celtic,
        FrameKind::GreekKey,
        FrameKind::Egyptian,
        FrameKind::Floral,
    ];

    /// The size its detail reads well at, in pixels at scale 1: what picking it sets.
    pub fn natural_size(self) -> f32 {
        match self {
            FrameKind::Brackets | FrameKind::Chamfer => default_frame_size(),
            FrameKind::Riveted => 32.0,
            FrameKind::ArtDeco | FrameKind::Gothic | FrameKind::Circuit => 44.0,
            FrameKind::ArtNouveau | FrameKind::Baroque => 52.0,
            FrameKind::Runic | FrameKind::Celtic | FrameKind::Floral => 60.0,
            FrameKind::GreekKey | FrameKind::Egyptian => 72.0,
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
            FrameKind::Celtic => "Celtic knot",
            FrameKind::GreekKey => "Greek key",
            FrameKind::Egyptian => "Egyptian",
            FrameKind::Floral => "Floral",
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
