//! The filters over a panel (scanlines, grain, pixel blocks, camo and a tint) and the
//! materials, with what each covers. Plaid, argyle, stripes, the checkerboard and
//! polka dots are in `patterns`.

use serde::{Deserialize, Serialize};

use super::style::black;

/// What a filter covers: the panel's background, its items and keys, or both. Either way it
/// sits under the text, so labels stay crisp.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOn {
    /// The panel's background, under its contents.
    Background,
    /// Each item and key, under its label.
    Items,
    /// The background and each item and key.
    #[default]
    Both,
}

impl FilterOn {
    pub const ALL: [FilterOn; 3] = [FilterOn::Background, FilterOn::Items, FilterOn::Both];

    pub fn background(self) -> bool {
        self != FilterOn::Items
    }

    pub fn items(self) -> bool {
        self != FilterOn::Background
    }

    pub(super) fn is_both(&self) -> bool {
        *self == FilterOn::Both
    }

    pub(super) fn is_background(&self) -> bool {
        *self == FilterOn::Background
    }
}

impl std::fmt::Display for FilterOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FilterOn::Background => "Background",
            FilterOn::Items => "Items and keys",
            FilterOn::Both => "Both",
        })
    }
}

/// A filter, which covers what its `on` says.
pub trait Covers {
    fn on(&self) -> FilterOn;
    fn set_on(&mut self, on: FilterOn);
}

macro_rules! covers {
    ($($filter:ty),*) => {$(
        impl Covers for $filter {
            fn on(&self) -> FilterOn {
                self.on
            }
            fn set_on(&mut self, on: FilterOn) {
                self.on = on;
            }
        }
    )*};
}

covers!(Scanlines, Texture, Grain, Pixels, Tint, Camo, super::Plaid, super::Argyle, super::Stripes, super::Checkerboard, super::PolkaDots);

/// A material's own default: the background alone.
fn background() -> FilterOn {
    FilterOn::Background
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
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

fn default_scan_spacing() -> f32 {
    4.0
}

fn default_scan_opacity() -> f32 {
    0.2
}

/// A material (paper, brushed metal, concrete, cloth, denim, silk, leather, steel or wood). It
/// only lightens and darkens, so the color under it still shows through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    /// How strong the grain is, 0 to 1.
    #[serde(default = "default_texture_opacity")]
    pub opacity: f32,
    /// Unlike the other filters, a material covers the background alone unless it says so.
    #[serde(default = "background", skip_serializing_if = "FilterOn::is_background")]
    pub on: FilterOn,
}

impl Texture {
    /// A material on the background, `opacity` strong.
    pub fn new(opacity: f32) -> Texture {
        Texture { opacity, on: FilterOn::Background }
    }
}

fn default_texture_opacity() -> f32 {
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
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

fn default_grain_opacity() -> f32 {
    0.15
}

fn default_grain_size() -> f32 {
    1.0
}

/// Blocks `size` pixels across at scale 1, each a little lighter or darker, with a dark seam
/// between them. With `shimmer` each block slowly brightens and dims, out of step with the rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pixels {
    #[serde(default = "default_pixels_size")]
    pub size: f32,
    #[serde(default = "default_pixels_opacity")]
    pub opacity: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shimmer: bool,
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

fn default_pixels_size() -> f32 {
    6.0
}

fn default_pixels_opacity() -> f32 {
    0.25
}

/// A flat wash of `color`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tint {
    #[serde(default = "default_tint_color")]
    pub color: String,
    #[serde(default = "default_tint_opacity")]
    pub opacity: f32,
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

fn default_tint_color() -> String {
    "#ffa040".into()
}

fn default_tint_opacity() -> f32 {
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
    #[serde(default, skip_serializing_if = "FilterOn::is_both")]
    pub on: FilterOn,
}

fn default_camo_opacity() -> f32 {
    0.3
}

fn default_camo_size() -> f32 {
    60.0
}
