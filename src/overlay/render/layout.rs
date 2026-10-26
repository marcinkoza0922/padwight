//! The layout tree: what a panel is made of (rows, columns, wrapping rows, stacks, text and
//! painted leaves), measured at a width and then placed into a box, where it emits its ops.
//!
//! It follows iced's layout where the overlay's panels rely on it, so their numbers port as
//! they are:
//! - A row, column or wrapping row holding a child that fills an axis fills its parent along
//!   it too (iced's `enclose`), unless its own size is set after the child is added.
//! - A row lays out the children that don't fill first, in order, each offered what the ones
//!   before it left, then shares what remains among the ones that fill, by their portions.
//! - Heights follow widths: contents are measured with no limit on their height, as inside the
//!   scrollable that iced's panels hold their contents in, so a `Fill` height in a column is its
//!   contents' height. A `Fill` height is as tall as the row it is in (the tallest of the row's
//!   other children), a stack's base, or the box it is pinned in.
//! - Inside a box set to `Shrink` that holds something that fills, a `Fill` width shrinks to its
//!   contents (iced's compression); in a column like that, it is as wide as the widest of the
//!   others.
//! - `align_x` and `align_y` line up a row's children across it, a column's, and each line of a
//!   wrapping row (inside its widest line), as in iced. Along a row's or column's own axis they
//!   place its children as a block when the box is bigger than they are (`Start` is iced's), so
//!   a column of one child is iced's container.
//!
//! Effects (opacity, scale, shift) are drawn as a layer and don't move anything around them.

mod build;
mod flex;

use super::{
    Corners, Layer, Op, Point, Rect, Rgba,
    text::{self, Spec},
};
use flex::Room;

#[cfg(test)]
mod tests;

pub use text::Align;

/// How much a run of text or a wrapping row may overrun its width without breaking: measured
/// again at a width it reached by adding up its parts, it must break where it did before.
const SLACK: f32 = 0.01;

/// How big a node is along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    /// As big as its contents.
    #[default]
    Shrink,
    /// As big as its parent lets it be, sharing that with its siblings that fill.
    Fill,
    /// Like `Fill`, taking this many shares.
    FillPortion(u16),
    Fixed(f32),
}

impl Length {
    /// How many shares of the space left it takes: 0 when it doesn't fill.
    fn portions(self) -> u16 {
        match self {
            Length::Fill => 1,
            Length::FillPortion(n) => n,
            Length::Shrink | Length::Fixed(_) => 0,
        }
    }

    fn fills(self) -> bool {
        self.portions() > 0
    }

    /// This length grown to hold a child of `other`'s: a box that shrinks fills once its child does.
    fn enclose(self, other: Length) -> Length {
        match self {
            Length::Shrink if other.fills() => other,
            _ => self,
        }
    }
}

impl From<f32> for Length {
    fn from(amount: f32) -> Length {
        Length::Fixed(amount)
    }
}

/// The space inside a node's box around its contents.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Padding {
    pub const ZERO: Padding = Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };

    fn x(&self) -> f32 {
        self.left + self.right
    }

    fn y(&self) -> f32 {
        self.top + self.bottom
    }

    /// The part of `rect` inside the padding.
    fn inside(&self, rect: Rect) -> Rect {
        Rect::new(rect.x + self.left, rect.y + self.top, (rect.w - self.x()).max(0.0), (rect.h - self.y()).max(0.0))
    }
}

impl From<f32> for Padding {
    fn from(all: f32) -> Padding {
        Padding { top: all, right: all, bottom: all, left: all }
    }
}

/// Top and bottom, then left and right, as iced takes them.
impl From<[f32; 2]> for Padding {
    fn from([y, x]: [f32; 2]) -> Padding {
        Padding { top: y, right: x, bottom: y, left: x }
    }
}

/// Where a pinned child sits in its parent's box (padding and all). It doesn't change the
/// parent's size, and is drawn over the parent's contents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pin {
    /// Its top left corner this far from the box's.
    At(Point),
    /// Lined up inside the box.
    Aligned(Align, Align),
}

/// Something a leaf draws: the ops for the box it was given (less its padding).
pub trait Painter {
    fn paint(&self, rect: Rect, ops: &mut Vec<Op>);
}

impl<F: Fn(Rect, &mut Vec<Op>)> Painter for F {
    fn paint(&self, rect: Rect, ops: &mut Vec<Op>) {
        self(rect, ops);
    }
}

/// A node drawn as one picture, faded, scaled about `about` (from its top left corner; its
/// center when unset) and moved, without moving anything around it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Effect {
    opacity: f32,
    scale: f32,
    shift: Point,
    about: Option<Point>,
}

impl Effect {
    const NONE: Effect = Effect { opacity: 1.0, scale: 1.0, shift: Point::ZERO, about: None };

    /// The layer to draw a node in `rect` as, or `None` when it is drawn as it is.
    fn layer(&self, rect: Rect) -> Option<Layer> {
        if self.opacity == 1.0 && self.scale == 1.0 && self.shift == Point::ZERO {
            return None;
        }
        let about = self.about.map_or(rect.center(), |p| Point::new(rect.x + p.x, rect.y + p.y));
        Some(Layer { opacity: self.opacity, scale: self.scale.max(0.0), about, shift: self.shift })
    }
}

enum Kind {
    Space,
    Text { spec: Spec, color: Rgba },
    Leaf(Box<dyn Painter>),
    Row(Vec<Node>),
    Column(Vec<Node>),
    Wrap(Vec<Node>),
    /// Layers drawn bottom up; the one at `base` sets the size, the others fill it.
    Stack { layers: Vec<Node>, base: usize },
}

/// One box of a panel, and what it holds.
pub struct Node {
    kind: Kind,
    width: Length,
    height: Length,
    max_width: f32,
    max_height: f32,
    padding: Padding,
    spacing: f32,
    line_spacing: Option<f32>,
    align_x: Align,
    align_y: Align,
    clip: Option<Corners>,
    effect: Effect,
    pins: Vec<(Pin, Node)>,
}

/// Measuring and placing.
impl Node {
    /// Its size (x wide, y tall) when offered `width` (infinite for as wide as it likes); its
    /// height follows from the width, as wrapped text's does.
    pub fn measure(&self, width: f32) -> Point {
        self.size_in(Room { max: width, squeeze: false })
    }

    /// Lays it out in `rect`, its box (normally as big as `measure` says), and adds what it
    /// draws to `ops`. Its contents are laid out at the box's width.
    pub fn place(&self, rect: Rect, ops: &mut Vec<Op>) {
        self.put(rect, false, ops);
    }

    /// Measured at `width` and placed with its top left corner at `at`: its size and its ops.
    pub fn ops(&self, at: Point, width: f32) -> (Point, Vec<Op>) {
        let size = self.measure(width);
        let mut ops = Vec::new();
        self.place(Rect::new(at.x, at.y, size.x, size.y), &mut ops);
        (size, ops)
    }

    /// Whether a fill inside it shrinks to its contents, inside a parent where `squeeze` says
    /// whether one does.
    fn squeezes(&self, squeeze: bool) -> bool {
        match self.width {
            Length::Shrink => true,
            Length::Fixed(_) => false,
            Length::Fill | Length::FillPortion(_) => squeeze,
        }
    }

    /// Its size in `room`, as iced's limits resolve it.
    fn size_in(&self, room: Room) -> Point {
        let max = room.max.min(self.max_width).max(0.0);
        let max = match self.width {
            Length::Fixed(w) => w.min(max).max(0.0),
            _ => max,
        };
        let inner = Room { max: (max - self.padding.x()).max(0.0), squeeze: self.squeezes(room.squeeze) };
        let content = self.content(inner);
        let w = match self.width {
            Length::Fixed(_) => max,
            Length::Fill | Length::FillPortion(_) if !room.squeeze && max.is_finite() => max,
            _ => (content.x + self.padding.x()).min(max),
        };
        let h = match self.height {
            Length::Fixed(h) => h,
            _ => content.y + self.padding.y(),
        };
        Point::new(w, h.min(self.max_height).max(0.0))
    }

    /// The size of what it holds, inside its padding.
    fn content(&self, room: Room) -> Point {
        match &self.kind {
            Kind::Space | Kind::Leaf(_) => Point::ZERO,
            Kind::Text { spec, .. } => {
                let width = room.max.is_finite().then_some(room.max + SLACK);
                text::shape(&Spec { width, align: Align::Start, ..spec.clone() }).size
            }
            Kind::Row(children) => flex::row(self, children, room).1,
            Kind::Column(children) => flex::column(self, children, room).1,
            Kind::Wrap(children) => flex::wrap(self, children, room).1,
            Kind::Stack { layers, base } => layers[*base].size_in(room),
        }
    }

    /// Lays it out in `rect` inside a parent where `squeeze` says whether a fill shrinks.
    fn put(&self, rect: Rect, squeeze: bool, ops: &mut Vec<Op>) {
        let layer = self.effect.layer(rect);
        if layer.is_some_and(|l| l.opacity <= 0.0) {
            return;
        }
        if let Some(layer) = layer {
            ops.push(Op::Layer(layer));
        }
        if let Some(radii) = self.clip {
            ops.push(Op::Clip { rect, radii });
        }
        let inner = self.padding.inside(rect);
        self.put_content(inner, Room { max: inner.w, squeeze: self.squeezes(squeeze) }, ops);
        for (pin, child) in &self.pins {
            flex::put_pin(child, *pin, rect, ops);
        }
        if self.clip.is_some() {
            ops.push(Op::Unclip);
        }
        if layer.is_some() {
            ops.push(Op::Unlayer);
        }
    }

    fn put_content(&self, inner: Rect, room: Room, ops: &mut Vec<Op>) {
        match &self.kind {
            Kind::Space => {}
            Kind::Leaf(painter) => painter.paint(inner, ops),
            Kind::Text { spec, color } => self.put_text(spec, *color, inner, ops),
            Kind::Row(children) => flex::put_row(self, children, inner, room, ops),
            Kind::Column(children) => flex::put_column(self, children, inner, room, ops),
            Kind::Wrap(children) => flex::put_wrap(self, children, inner, room, ops),
            Kind::Stack { .. } => flex::put_stack(self, inner, room, ops),
        }
    }

    /// The run shaped at the box's width, its lines lined up in it as `align_x` says, and the
    /// run lined up down it as `align_y` says.
    fn put_text(&self, spec: &Spec, color: Rgba, inner: Rect, ops: &mut Vec<Op>) {
        if spec.text.is_empty() {
            return;
        }
        let shaped = text::shape(&Spec { width: Some(inner.w + SLACK), align: self.align_x, ..spec.clone() });
        let at = Point::new(
            inner.x + (inner.w - shaped.size.x) * share(self.align_x),
            inner.y + (inner.h - shaped.size.y) * share(self.align_y),
        );
        ops.push(Op::Text { run: shaped.run, at, color, ink: shaped.ink.shifted(at) });
    }
}

/// How much of the space left over goes before something lined up this way.
fn share(align: Align) -> f32 {
    match align {
        Align::Start => 0.0,
        Align::Center => 0.5,
        Align::End => 1.0,
    }
}
