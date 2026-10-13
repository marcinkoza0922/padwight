//! On-screen action menus: their kinds, items and where they sit.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::*;

/// An on-screen action menu, usable by every profile of its game and opened with
/// `ButtonAction::OpenMenu`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Menu {
    pub name: String,
    pub kind: MenuKind,
    pub items: Vec<MenuItem>,
    /// Button that backs out of the menu (one level for submenus). Defaults to the kind's own
    /// (see [`Menu::default_cancel`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel: Option<Button>,
    /// The menu has no close (or back) button at all: it goes when its opener is let go, or
    /// when an item is chosen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_cancel: bool,
    /// Choosing an item leaves the menu up for more picks, instead of closing it. Off unless
    /// asked for; a radial menu chooses as it's let go, so it closes anyway.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep_open: bool,
    /// The item a grid's cursor starts on (counting cells left to right, then down).
    #[serde(default, skip_serializing_if = "is_first")]
    pub start: usize,
    #[serde(default)]
    pub style: OverlayStyle,
}

fn is_first(n: &usize) -> bool {
    *n == 0
}

impl Menu {
    /// The button that closes the menu (or backs out of a submenu), if it has one.
    pub fn cancel_button(&self) -> Option<Button> {
        if self.no_cancel { None } else { self.cancel.or(self.default_cancel()) }
    }

    /// The close button a menu of this kind has unless another is chosen: East for a list,
    /// grid or carousel, and none for a radial, directional or button menu, which go as they
    /// pick or as their input is let go.
    pub fn default_cancel(&self) -> Option<Button> {
        match self.kind {
            MenuKind::List | MenuKind::Grid { .. } | MenuKind::Carousel { .. } => Some(Button::East),
            MenuKind::Radial { .. } | MenuKind::Directional { .. } | MenuKind::Buttons => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItem {
    pub label: String,
    pub action: ButtonAction,
    /// Quick-select button (button menus).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button: Option<Button>,
    /// Share of a radial menu's circle this item's arc takes, relative to the others.
    #[serde(default = "default_weight", skip_serializing_if = "is_default_weight")]
    pub weight: f32,
    /// The cells a grid item covers, merged from its own and those right of and below it.
    #[serde(default, skip_serializing_if = "Span::is_one")]
    pub span: Span,
}

impl MenuItem {
    /// An item doing `action`, with no quick-select button, an even share, one cell and no icon.
    pub fn new(label: impl Into<String>, action: ButtonAction) -> Self {
        MenuItem { label: label.into(), action, button: None, weight: 1.0, span: Span::ONE }
    }
}

/// How many columns and rows of a grid's cells an item covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub columns: u8,
    pub rows: u8,
}

impl Span {
    pub const ONE: Span = Span { columns: 1, rows: 1 };

    pub fn is_one(&self) -> bool {
        *self == Span::ONE
    }
}

impl Default for Span {
    fn default() -> Self {
        Span::ONE
    }
}

pub fn default_weight() -> f32 {
    1.0
}

fn is_default_weight(weight: &f32) -> bool {
    (weight - 1.0).abs() < f32::EPSILON
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuKind {
    /// Shown while the opening input is held: aim `stick` at an item, release to choose it.
    /// Items are arcs of a circle (sized by their weights) unless `boxes` puts them in boxes
    /// at equal angles.
    Radial {
        stick: Stick,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        boxes: bool,
    },
    /// Four slots (up, right, down, left) on the D-pad or face buttons. A slot fires its
    /// action; give it "Open menu" to make it a submenu.
    #[serde(alias = "cascade")]
    Directional { cluster: Cluster },
    /// A list moved through with the D-pad or left stick; A chooses.
    List,
    /// A list whose items can also be chosen directly with their quick-select button.
    Buttons,
    /// A row of items cycled with `controls`; A chooses.
    Carousel { controls: CarouselControls },
    /// A list laid out `columns` wide (up to `GRID_MAX` rows), moved through with the D-pad
    /// or left stick in all four directions; A chooses.
    Grid { columns: u8 },
}

/// A grid menu is at most this many columns and this many rows.
pub const GRID_MAX: usize = 6;

impl MenuKind {
    pub fn default_for(kind: MenuKindTag) -> Self {
        match kind {
            MenuKindTag::Radial => MenuKind::Radial { stick: Stick::Right, boxes: false },
            MenuKindTag::Directional => MenuKind::Directional { cluster: Cluster::DPad },
            MenuKindTag::List => MenuKind::List,
            MenuKindTag::Buttons => MenuKind::Buttons,
            MenuKindTag::Carousel => MenuKind::Carousel { controls: CarouselControls::Bumpers },
            MenuKindTag::Grid => MenuKind::Grid { columns: 3 },
        }
    }

    pub fn tag(self) -> MenuKindTag {
        match self {
            MenuKind::Radial { .. } => MenuKindTag::Radial,
            MenuKind::Directional { .. } => MenuKindTag::Directional,
            MenuKind::List => MenuKindTag::List,
            MenuKind::Buttons => MenuKindTag::Buttons,
            MenuKind::Carousel { .. } => MenuKindTag::Carousel,
            MenuKind::Grid { .. } => MenuKindTag::Grid,
        }
    }

    /// A grid's column count, kept to 1..=`GRID_MAX` whatever the config says.
    pub fn grid_columns(self) -> Option<usize> {
        match self {
            MenuKind::Grid { columns } => Some(usize::from(columns).clamp(1, GRID_MAX)),
            _ => None,
        }
    }
}

/// Menu kinds without their settings, for pickers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKindTag {
    Radial,
    Directional,
    List,
    Buttons,
    Carousel,
    Grid,
}

impl MenuKindTag {
    pub const ALL: [MenuKindTag; 6] = [
        MenuKindTag::Radial,
        MenuKindTag::Directional,
        MenuKindTag::List,
        MenuKindTag::Buttons,
        MenuKindTag::Carousel,
        MenuKindTag::Grid,
    ];
}

impl MenuKindTag {
    /// A one-word name, for summaries.
    pub fn short(self) -> &'static str {
        match self {
            MenuKindTag::Radial => "Radial",
            MenuKindTag::Directional => "Directional",
            MenuKindTag::List => "List",
            MenuKindTag::Buttons => "Button menu",
            MenuKindTag::Carousel => "Carousel",
            MenuKindTag::Grid => "Grid",
        }
    }
}

impl fmt::Display for MenuKindTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MenuKindTag::Radial => "Radial (hold, aim, release)",
            MenuKindTag::Directional => "Directional (D-pad / face buttons)",
            MenuKindTag::List => "List",
            MenuKindTag::Buttons => "Button menu (list + quick buttons)",
            MenuKindTag::Carousel => "Carousel",
            MenuKindTag::Grid => "Grid (up to 6 × 6)",
        })
    }
}

/// Where on screen an overlay sits, relative to the screen so it suits any size or shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    #[default]
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl ScreenPosition {
    /// In reading order, for a 3×3 picker.
    pub const GRID: [ScreenPosition; 9] = [
        ScreenPosition::TopLeft,
        ScreenPosition::TopCenter,
        ScreenPosition::TopRight,
        ScreenPosition::CenterLeft,
        ScreenPosition::Center,
        ScreenPosition::CenterRight,
        ScreenPosition::BottomLeft,
        ScreenPosition::BottomCenter,
        ScreenPosition::BottomRight,
    ];

    /// Column and row in the 3×3 grid: 0 = left/top, 1 = center, 2 = right/bottom.
    pub fn cell(self) -> (usize, usize) {
        let i = Self::GRID.iter().position(|p| *p == self).unwrap_or(4);
        (i % 3, i / 3)
    }
}

impl fmt::Display for ScreenPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScreenPosition::TopLeft => "Top left",
            ScreenPosition::TopCenter => "Top center",
            ScreenPosition::TopRight => "Top right",
            ScreenPosition::CenterLeft => "Center left",
            ScreenPosition::Center => "Center",
            ScreenPosition::CenterRight => "Center right",
            ScreenPosition::BottomLeft => "Bottom left",
            ScreenPosition::BottomCenter => "Bottom center",
            ScreenPosition::BottomRight => "Bottom right",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cluster {
    DPad,
    FaceButtons,
}

impl Cluster {
    /// The four slot buttons: up, right, down, left.
    pub fn slots(self) -> [Button; 4] {
        match self {
            Cluster::DPad => [Button::DpadUp, Button::DpadRight, Button::DpadDown, Button::DpadLeft],
            Cluster::FaceButtons => [Button::North, Button::East, Button::South, Button::West],
        }
    }
}

impl fmt::Display for Cluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Cluster::DPad => "D-pad",
            Cluster::FaceButtons => "Face buttons",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarouselControls {
    Bumpers,
    Triggers,
    DPad,
    LeftStick,
    RightStick,
}

impl CarouselControls {
    pub const ALL: [CarouselControls; 5] = [
        CarouselControls::Bumpers,
        CarouselControls::Triggers,
        CarouselControls::DPad,
        CarouselControls::LeftStick,
        CarouselControls::RightStick,
    ];
}

impl fmt::Display for CarouselControls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CarouselControls::Bumpers => "Bumpers (LB / RB)",
            CarouselControls::Triggers => "Triggers (LT / RT)",
            CarouselControls::DPad => "D-pad left / right",
            CarouselControls::LeftStick => "Left stick",
            CarouselControls::RightStick => "Right stick",
        })
    }
}
