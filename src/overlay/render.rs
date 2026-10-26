//! The overlay's own renderer, free of iced: colors and the style resolved for drawing, the
//! geometry and the display list, and the rasterizer that draws it (layout is a later step).

mod color;
mod fill;
mod geom;
mod look;
mod op;
pub mod raster;
mod style;
pub mod text;
mod typeset;

pub use color::{Rgba, blend, paint, text_on};
pub use fill::{Cap, Fill, Join, Linear, Rule, Stop, Stroke};
pub use geom::{Corners, Layer, Path, Point, Rect, Seg};
pub use look::{Look, Outline, PANEL_CORNER, Shadow};
pub use op::{File, Fit, Markup, Op, PatternRef, RunKey, Svg, balanced, bounds, digest};
pub use style::Style;
pub use typeset::Type;
