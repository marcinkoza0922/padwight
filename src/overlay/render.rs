//! The overlay's own renderer, free of iced: colors and the style resolved for drawing, as the
//! display list, rasterizer and layout (later steps) read them.

mod color;
mod look;
mod style;
mod typeset;

pub use color::{Rgba, blend, paint, text_on};
pub use look::{Look, Outline, PANEL_CORNER, Shadow};
pub use style::Style;
pub use typeset::Type;
