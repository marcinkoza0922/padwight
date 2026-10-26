//! The overlay's own renderer, free of iced: colors and the style resolved for drawing, the
//! geometry and the display list, the layout tree that emits it, and the rasterizer that draws it.

mod anchor;
mod color;
mod fill;
mod geom;
mod item;
pub mod layout;
mod look;
mod op;
mod panel;
pub mod raster;
#[cfg(test)]
mod sample;
mod style;
pub mod text;
mod typeset;

pub use color::{Rgba, blend, paint, text_on};
pub use fill::{Cap, Fill, Join, Linear, Rule, Stop, Stroke};
pub use geom::{Corners, Layer, Path, Point, Rect, Seg};
pub use layout::{Align, Length, Node, Padding, Painter, Pin};
pub use look::{Look, Outline, PANEL_CORNER, Shadow};
pub use op::{File, Fit, Markup, Op, PatternRef, RunKey, Svg, balanced, bounds, digest};
pub use item::{Item, item};
pub use panel::framed;
pub use style::Style;
pub use typeset::Type;
