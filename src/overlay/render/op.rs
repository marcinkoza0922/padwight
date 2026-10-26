//! The display list: a panel drawn as flat, comparable data. Layout emits it, the rasterizer
//! draws it in order (later ops over earlier ones), and a panel whose list equals last frame's
//! keeps its picture. Coordinates are logical pixels; the device scale is the rasterizer's, so a
//! picture kept for a list is kept for a list and a scale.
//!
//! `Clip` and `Layer` each open a group that the next `Unclip` or `Unlayer` closes. Groups nest
//! as a stack, each closed in the reverse order it was opened (see `balanced`). The ops in a
//! group are in the same coordinates as those around it: a clip cuts them, and a layer draws
//! them as one picture at full strength that is then faded, scaled and moved as a whole.

use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use super::{Corners, Fill, Layer, Outline, Path, Point, Rect, Rgba, Rule, Shadow, Stroke, geom::hash_floats};
use crate::{config::ImageFit, overlay::art};

#[cfg(test)]
mod tests;

/// A file to draw, by its path.
pub type File = Arc<std::path::Path>;

/// Names a shaped run in the text cache (step 2.2) by everything that shapes it: its text, font,
/// size, weight, slant, underline, letter spacing, wrapping width and line alignment. Its color
/// is the op's, so a run is shaped once however it fades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunKey(pub u64);

impl RunKey {
    /// The key of whatever describes a run.
    pub fn of(what: &impl Hash) -> RunKey {
        let mut hasher = DefaultHasher::new();
        what.hash(&mut hasher);
        RunKey(hasher.finish())
    }
}

/// How a picture fills its box, as `config::ImageFit` and iced's `ContentFit` do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Fit {
    /// As big as fits whole, centered, keeping its shape.
    #[default]
    Contain,
    /// Covering the box, keeping its shape, centered and cut to the box.
    Cover,
    /// Stretched to the box.
    Stretch,
}

impl From<ImageFit> for Fit {
    fn from(fit: ImageFit) -> Fit {
        match fit {
            ImageFit::Cover => Fit::Cover,
            ImageFit::Contain => Fit::Contain,
            ImageFit::Stretch => Fit::Stretch,
        }
    }
}

/// SVG made in code (icons, frame marks, figures), with its hash taken once, so comparing and
/// hashing a list doesn't read the text again. The hash also names it in the rasterizer's cache.
#[derive(Clone)]
pub struct Markup {
    text: Arc<str>,
    hash: u64,
}

impl Markup {
    pub fn new(text: impl Into<Arc<str>>) -> Markup {
        let text = text.into();
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Markup { hash: hasher.finish(), text }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn key(&self) -> u64 {
        self.hash
    }
}

impl PartialEq for Markup {
    fn eq(&self, other: &Markup) -> bool {
        self.hash == other.hash && (Arc::ptr_eq(&self.text, &other.text) || self.text == other.text)
    }
}

impl Hash for Markup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl fmt::Debug for Markup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Markup({:016x}, {} bytes)", self.hash, self.text.len())
    }
}

/// Where an SVG comes from: a file (a background picture) or markup made in code.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Svg {
    File(File),
    Markup(Markup),
}

/// A filter's picture, shared, compared and hashed by its key: the same key builds the same
/// pixels (as the iced renderer's picture cache already relies on).
#[derive(Clone)]
pub struct PatternRef(pub Arc<art::Pattern>);

impl PatternRef {
    pub fn new(pattern: art::Pattern) -> PatternRef {
        PatternRef(Arc::new(pattern))
    }

    pub fn pattern(&self) -> &art::Pattern {
        &self.0
    }

    pub fn key(&self) -> [u32; 3] {
        self.0.key
    }
}

impl PartialEq for PatternRef {
    fn eq(&self, other: &PatternRef) -> bool {
        self.key() == other.key()
    }
}

impl Hash for PatternRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl fmt::Debug for PatternRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PatternRef({:?})", self.key())
    }
}

/// One thing to draw, or the start or end of a group.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// A box's shadow, by iced tiny-skia's formula: the rounded box grown by `spread` and moved
    /// by the offset, its distance through a smoothstep over the blur either side. It covers the
    /// box too, so it goes before the box it belongs to. Iced has no spread: 0 matches it.
    Shadow { rect: Rect, radii: Corners, shadow: Shadow, spread: f32 },
    /// A rounded box, filled, with its border inside the box.
    Rect { rect: Rect, radii: Corners, fill: Option<Fill>, border: Option<Outline> },
    /// A shaped run from the text cache, the top left corner of its box at `at`, in `color`
    /// (underlines too). `ink` is where its glyphs reach once placed there, as the text cache
    /// reports it, for the bounds.
    Text { run: RunKey, at: Point, color: Rgba, ink: Rect },
    /// A vector shape, filled, then stroked.
    Path { path: Path, fill: Option<Fill>, rule: Rule, stroke: Option<Stroke> },
    /// A PNG or JPEG fitted to `rect`.
    Picture { file: File, rect: Rect, fit: Fit, opacity: f32 },
    /// An SVG fitted to `rect`.
    Svg { svg: Svg, rect: Rect, fit: Fit, opacity: f32 },
    /// The part of a filter's picture under `rect`, cut to `cut` (in pixels from the rect's
    /// top left corner). The picture's own top left corner lies at `origin`, the panel's corner,
    /// so a filter runs on unbroken across everything it covers. `opacity` includes the
    /// pattern's own.
    Pattern { pattern: PatternRef, rect: Rect, origin: Point, cut: art::Shape, opacity: f32 },
    /// Opens a group drawn only inside the rounded box.
    Clip { rect: Rect, radii: Corners },
    Unclip,
    /// Opens a group drawn as one picture and laid down as the layer says.
    Layer(Layer),
    Unlayer,
}

/// Equal ops hash equal (see `geom`); a NaN, which never equals itself, only makes its panel
/// redraw.
impl Eq for Op {}

impl Hash for Op {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Op::Shadow { rect, radii, shadow, spread } => {
                (rect, radii, shadow).hash(state);
                hash_floats(state, &[*spread]);
            }
            Op::Rect { rect, radii, fill, border } => (rect, radii, fill, border).hash(state),
            Op::Text { run, at, color, ink } => (run, at, color, ink).hash(state),
            Op::Path { path, fill, rule, stroke } => (path, fill, rule, stroke).hash(state),
            Op::Picture { file, rect, fit, opacity } => {
                (file, rect, fit).hash(state);
                hash_floats(state, &[*opacity]);
            }
            Op::Svg { svg, rect, fit, opacity } => {
                (svg, rect, fit).hash(state);
                hash_floats(state, &[*opacity]);
            }
            Op::Pattern { pattern, rect, origin, cut, opacity } => {
                (pattern, rect, origin).hash(state);
                hash_shape(state, *cut);
                hash_floats(state, &[*opacity]);
            }
            Op::Clip { rect, radii } => (rect, radii).hash(state),
            Op::Layer(layer) => layer.hash(state),
            Op::Unclip | Op::Unlayer => {}
        }
    }
}

fn hash_shape<H: Hasher>(state: &mut H, shape: art::Shape) {
    match shape {
        art::Shape::Rounded(round) => hash_floats(state, &[0.0, round]),
        art::Shape::Arc { center, inner, outer, start, end, gap } => hash_floats(state, &[1.0, center.0, center.1, inner, outer, start, end, gap]),
    }
}

impl Op {
    /// Where this op can put ink on its own, before any clip or layer it is in: `None` for the
    /// group markers and for a shadow with no color.
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Op::Shadow { shadow, .. } if shadow.color.a <= 0.0 => None,
            Op::Shadow { rect, shadow, spread, .. } => Some(rect.outset(*spread).shifted(Point::new(shadow.offset.0, shadow.offset.1)).outset(shadow.blur.max(0.0))),
            Op::Rect { rect, .. } | Op::Picture { rect, .. } | Op::Svg { rect, .. } | Op::Pattern { rect, .. } => Some(*rect),
            Op::Text { ink, .. } => Some(*ink),
            Op::Path { path, stroke, .. } => path.bounds().map(|b| b.outset(stroke.map_or(0.0, |s| s.reach()))),
            Op::Clip { .. } | Op::Unclip | Op::Layer(_) | Op::Unlayer => None,
        }
    }
}

/// Where a list can put ink: every op's bounds, cut by the clips around it and moved by the
/// layers around it. `None` when it draws nothing. A close with nothing open is passed over.
pub fn bounds(ops: &[Op]) -> Option<Rect> {
    let mut ink = None;
    let mut at = 0;
    while at < ops.len() {
        let (inner, next) = group(ops, at);
        ink = union(ink, inner);
        at = next;
    }
    ink
}

/// The bounds of the ops from `at` to the close of the group they are in (or the end), and
/// where to go on from.
fn group(ops: &[Op], mut at: usize) -> (Option<Rect>, usize) {
    let mut ink = None;
    while let Some(op) = ops.get(at) {
        at += 1;
        match op {
            Op::Clip { rect, .. } => {
                let (inner, next) = group(ops, at);
                ink = union(ink, inner.and_then(|b| b.intersection(*rect)));
                at = next;
            }
            Op::Layer(layer) => {
                let (inner, next) = group(ops, at);
                ink = union(ink, inner.map(|b| layer.map(b)));
                at = next;
            }
            Op::Unclip | Op::Unlayer => return (ink, at),
            op => ink = union(ink, op.bounds()),
        }
    }
    (ink, at)
}

fn union(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(b)),
        (a, b) => a.or(b),
    }
}

/// Whether every group is closed, by its own kind of close, in the reverse order they opened.
pub fn balanced(ops: &[Op]) -> bool {
    let mut open = Vec::new();
    for op in ops {
        match op {
            Op::Clip { .. } => open.push(true),
            Op::Layer(_) => open.push(false),
            Op::Unclip if open.pop() != Some(true) => return false,
            Op::Unlayer if open.pop() != Some(false) => return false,
            _ => {}
        }
    }
    open.is_empty()
}

/// A hash of the whole list, for keeping pictures by it (compare the lists too before reusing).
pub fn digest(ops: &[Op]) -> u64 {
    let mut hasher = DefaultHasher::new();
    ops.hash(&mut hasher);
    hasher.finish()
}
