//! The geometry the display list is drawn in: points, boxes, corner radii, vector paths and the
//! layer transform. Everything is in logical pixels, x right and y down; the rasterizer applies
//! the device scale.
//!
//! Equality is f32's own, and the hashes take each float's bits with -0 made 0, so equal values
//! hash equal.

use std::hash::{Hash, Hasher};

/// Feeds `values` to `state` by their bits, -0 as 0.
pub(super) fn hash_floats<H: Hasher>(state: &mut H, values: &[f32]) {
    for &v in values {
        state.write_u32(if v == 0.0 { 0 } else { v.to_bits() });
    }
}

/// A point, or an offset from one.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.x, self.y]);
    }
}

/// A box: its top left corner and size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Whether it covers nothing.
    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    /// This box moved by `by`.
    pub fn shifted(self, by: Point) -> Rect {
        Rect { x: self.x + by.x, y: self.y + by.y, ..self }
    }

    /// This box grown by `d` on every side (shrunk for a negative `d`).
    pub fn outset(self, d: f32) -> Rect {
        Rect::new(self.x - d, self.y - d, self.w + 2.0 * d, self.h + 2.0 * d)
    }

    /// The smallest box holding both.
    pub fn union(self, other: Rect) -> Rect {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        Rect::new(x, y, self.right().max(other.right()) - x, self.bottom().max(other.bottom()) - y)
    }

    /// Where the two overlap, or `None` where they don't.
    pub fn intersection(self, other: Rect) -> Option<Rect> {
        let (x, y) = (self.x.max(other.x), self.y.max(other.y));
        let r = Rect::new(x, y, self.right().min(other.right()) - x, self.bottom().min(other.bottom()) - y);
        (!r.is_empty()).then_some(r)
    }

    /// The smallest box holding every point, or `None` for none.
    pub fn around(points: impl IntoIterator<Item = Point>) -> Option<Rect> {
        points.into_iter().fold(None, |r: Option<Rect>, p| {
            let at = Rect::new(p.x, p.y, 0.0, 0.0);
            Some(r.map_or(at, |r| r.union(at)))
        })
    }
}

impl Hash for Rect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.x, self.y, self.w, self.h]);
    }
}

/// How round each corner of a box is, in pixels, in iced's order. The rasterizer holds each to
/// half the box's shorter side, as iced does.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub const ZERO: Corners = Corners::all(0.0);

    pub const fn all(r: f32) -> Corners {
        Corners { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
    }

    /// The radii as iced lists them: top left, top right, bottom right, bottom left.
    pub fn to_array(self) -> [f32; 4] {
        [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
    }
}

impl Hash for Corners {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &self.to_array());
    }
}

/// How a layer is laid back onto what is under it: at `opacity` as one picture, `scale` times its
/// size about `about`, then moved by `shift`, as `draw/transform.rs` moves a widget. A point `p`
/// lands at `about + shift + (p - about) * scale`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    pub opacity: f32,
    pub scale: f32,
    pub about: Point,
    pub shift: Point,
}

impl Layer {
    /// Laid down as it is.
    pub const IDENTITY: Layer = Layer { opacity: 1.0, scale: 1.0, about: Point::ZERO, shift: Point::ZERO };

    /// Only faded.
    pub const fn faded(opacity: f32) -> Layer {
        Layer { opacity, ..Layer::IDENTITY }
    }

    /// Where `p` lands.
    pub fn apply(&self, p: Point) -> Point {
        let Layer { scale, about, shift, .. } = *self;
        Point::new(about.x + shift.x + (p.x - about.x) * scale, about.y + shift.y + (p.y - about.y) * scale)
    }

    /// Where box `r` lands (still a box: the transform only scales and moves).
    pub fn map(&self, r: Rect) -> Rect {
        let (a, b) = (self.apply(Point::new(r.x, r.y)), self.apply(Point::new(r.right(), r.bottom())));
        Rect::new(a.x.min(b.x), a.y.min(b.y), (b.x - a.x).abs(), (b.y - a.y).abs())
    }
}

impl Hash for Layer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.opacity, self.scale, self.about.x, self.about.y, self.shift.x, self.shift.y]);
    }
}

/// One step of a path.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Seg {
    MoveTo(Point),
    LineTo(Point),
    /// A quadratic curve: its control point, then where it ends.
    QuadTo(Point, Point),
    /// A cubic curve: its two control points, then where it ends.
    CubicTo(Point, Point, Point),
    Close,
}

/// A vector outline, as tiny-skia's path builder takes it.
#[derive(Debug, Clone, PartialEq, Hash, Default)]
pub struct Path {
    pub segs: Vec<Seg>,
}

impl Path {
    pub fn new() -> Path {
        Path::default()
    }

    pub fn move_to(&mut self, p: Point) -> &mut Path {
        self.segs.push(Seg::MoveTo(p));
        self
    }

    pub fn line_to(&mut self, p: Point) -> &mut Path {
        self.segs.push(Seg::LineTo(p));
        self
    }

    pub fn quad_to(&mut self, c: Point, p: Point) -> &mut Path {
        self.segs.push(Seg::QuadTo(c, p));
        self
    }

    pub fn cubic_to(&mut self, c1: Point, c2: Point, p: Point) -> &mut Path {
        self.segs.push(Seg::CubicTo(c1, c2, p));
        self
    }

    pub fn close(&mut self) -> &mut Path {
        self.segs.push(Seg::Close);
        self
    }

    /// Part of a circle of radius `r` around `center`, from `start` to `end` degrees clockwise
    /// from the top, as a radial menu measures its arcs (`end` below `start` runs back). It
    /// starts with a line from the last point, or a move where there is none.
    pub fn arc(&mut self, center: Point, r: f32, start: f32, end: f32) -> &mut Path {
        let at = |degrees: f32| {
            let (sin, cos) = degrees.to_radians().sin_cos();
            Point::new(center.x + sin * r, center.y - cos * r)
        };
        let first = at(start);
        match self.segs.last() {
            None | Some(Seg::Close) => {
                self.move_to(first);
            }
            Some(&Seg::MoveTo(p) | &Seg::LineTo(p) | &Seg::QuadTo(_, p) | &Seg::CubicTo(_, _, p)) if p == first => {}
            Some(_) => {
                self.line_to(first);
            }
        }
        // Quarter turns or less, each a cubic whose handles are `k` of the radius long.
        let pieces = ((end - start).abs() / 90.0).ceil().max(1.0);
        let step = (end - start) / pieces;
        let k = 4.0 / 3.0 * (step.to_radians() / 4.0).tan();
        // The tangent at `degrees`, clockwise, `k * r` long.
        let tangent = |degrees: f32| {
            let (sin, cos) = degrees.to_radians().sin_cos();
            Point::new(cos * k * r, sin * k * r)
        };
        for i in 0..pieces as usize {
            let (a, b) = (start + step * i as f32, start + step * (i + 1) as f32);
            let (pa, pb, ta, tb) = (at(a), at(b), tangent(a), tangent(b));
            self.cubic_to(Point::new(pa.x + ta.x, pa.y + ta.y), Point::new(pb.x - tb.x, pb.y - tb.y), pb);
        }
        self
    }

    /// A whole circle, closed, as a shape of its own.
    pub fn circle(&mut self, center: Point, r: f32) -> &mut Path {
        self.move_to(Point::new(center.x, center.y - r)).arc(center, r, 0.0, 360.0).close()
    }

    /// The box around every point and control point: the curves lie inside it.
    pub fn bounds(&self) -> Option<Rect> {
        Rect::around(self.segs.iter().flat_map(|seg| match *seg {
            Seg::MoveTo(p) | Seg::LineTo(p) => vec![p],
            Seg::QuadTo(c, p) => vec![c, p],
            Seg::CubicTo(c1, c2, p) => vec![c1, c2, p],
            Seg::Close => vec![],
        }))
    }
}
