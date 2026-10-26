//! Text: a run described by what shapes it, shaped once into the cache and named by its
//! `RunKey`, then drawn wherever an `Op::Text` puts it. Layout calls `shape` to measure, the
//! rasterizer calls `draw`.
//!
//! The bodies here are placeholders until shaping lands (step 2.2): `shape` guesses a size from
//! the character count and `draw` draws nothing. Don't write tests against their numbers.

use tiny_skia::{Mask, PixmapMut, Transform};

use super::{Point, Rect, Rgba, RunKey};
use crate::config::Case;

/// Lines are this many times the font size apart, as in iced.
pub const LINE_HEIGHT: f32 = 1.3;

/// How a run's lines line up inside its width.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

/// Everything that shapes a run. Its color isn't here: that is the op's.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    /// As written; `case` is applied when shaping.
    pub text: String,
    /// A family from `font::family`, or the default font.
    pub family: Option<&'static str>,
    /// In logical pixels, with the typography's size already applied.
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub case: Case,
    /// The space between letters, in logical pixels.
    pub spacing: f32,
    /// Wrap between words at this width; `None` keeps each line whole.
    pub width: Option<f32>,
    pub align: Align,
}

impl Spec {
    /// Plain text at a size, in the default font.
    pub fn new(text: impl Into<String>, size: f32) -> Spec {
        Spec {
            text: text.into(),
            family: None,
            size,
            bold: false,
            italic: false,
            underline: false,
            case: Case::None,
            spacing: 0.0,
            width: None,
            align: Align::Start,
        }
    }
}

/// A shaped run, as layout needs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shaped {
    pub run: RunKey,
    /// The run's box: as wide as its widest line (or `Spec::width` when aligned within it), and
    /// its lines times `LINE_HEIGHT` tall.
    pub size: Point,
    /// Where its glyphs can draw, relative to the box's top-left. It may reach past the box.
    pub ink: Rect,
}

/// Shapes a run, or finds it in the cache.
pub fn shape(spec: &Spec) -> Shaped {
    let chars = spec.text.chars().count() as f32;
    let w = chars * (spec.size * 0.55 + spec.spacing);
    let w = spec.width.map_or(w, |max| w.min(max));
    let size = Point::new(w, spec.size * LINE_HEIGHT);
    let pad = spec.size * 0.25;
    Shaped { run: RunKey(0), size, ink: Rect::new(0.0, 0.0, size.x, size.y).outset(pad) }
}

/// Where text is drawn: `pixmap`'s pixels, cut by `mask` when there is one. `transform` maps
/// logical pixels to the pixmap's: the device scale and a translation, never a rotation or skew.
pub struct Canvas<'a> {
    pub pixmap: PixmapMut<'a>,
    pub transform: Transform,
    pub mask: Option<&'a Mask>,
}

/// Draws a shaped run with its box's top-left at `at` (logical pixels, sub-pixel), in `color`,
/// with glyphs drawn at the scale the canvas's transform gives.
pub fn draw(run: RunKey, at: Point, color: Rgba, canvas: &mut Canvas) {
    let _ = (run, at, color, canvas);
}
