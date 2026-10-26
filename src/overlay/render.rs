//! The overlay's own renderer, free of iced: colors and the style resolved for drawing, the
//! geometry and the display list, the layout tree that emits it, and the rasterizer that draws it.

mod anchor;
mod color;
mod figure;
mod fill;
mod geom;
mod glyphs;
mod info;
mod item;
mod keyboard;
pub mod layout;
mod log;
mod look;
mod media;
mod op;
mod panel;
mod radial;
pub mod raster;
mod rich;
#[cfg(test)]
mod sample;
mod style;
pub mod text;
mod typeset;

pub use color::{Rgba, blend, paint, text_on};
pub use figure::{figure, touchpad};
pub use fill::{Cap, Fill, Join, Linear, Rule, Stop, Stroke};
pub use glyphs::{bar_of, dpad, glyph, icon_svg, segment, state_icon};
pub use geom::{Corners, Layer, Path, Point, Rect, Seg};
pub use layout::{Align, Length, Node, Padding, Painter, Pin};
pub use look::{Look, Outline, PANEL_CORNER, Shadow};
pub use op::{File, Fit, Markup, Op, PatternRef, RunKey, Svg, balanced, bounds, digest};
pub use info::{column_widths, info_panel};
pub use item::{Item, item};
pub use log::{RELEASED, log_panel, shorten};
pub use media::{media, offer};
#[expect(unused_imports, reason = "drawn by the screen from step 4.1")]
pub use keyboard::{keyboard_panel, scale as keyboard_scale};
pub use keyboard::{GAP, KEYBOARD_PADDING, UNIT, keyboard_body_width};
pub use panel::framed;
pub use radial::radial;
pub use rich::{Setting, info_cell, line, plain, rich};
pub use style::Style;
pub use typeset::Type;
