//! What a shape is filled and outlined with: a color or a linear gradient, and strokes.

use std::{
    f32::consts::FRAC_PI_2,
    hash::{Hash, Hasher},
};

use super::{Point, Rect, Rgba, geom::hash_floats};

/// A color at a point along a gradient, 0 at its start to 1 at its end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub offset: f32,
    pub color: Rgba,
}

impl Hash for Stop {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.offset]);
        self.color.hash(state);
    }
}

/// A linear gradient across the box of what it fills, as iced's `gradient::Linear` holds it:
/// an angle in radians (0 runs bottom to top, π/2 left to right, π top to bottom) and its stops,
/// which are padded past either end.
#[derive(Debug, Clone, PartialEq)]
pub struct Linear {
    pub angle: f32,
    pub stops: Vec<Stop>,
}

impl Linear {
    pub fn new(angle: f32) -> Linear {
        Linear { angle, stops: Vec::new() }
    }

    /// This gradient with a stop added.
    pub fn stop(mut self, offset: f32, color: Rgba) -> Linear {
        self.stops.push(Stop { offset, color });
        self
    }

    /// Where it starts and ends over `bounds`, by iced's maths (`Radians::to_distance`): through
    /// the middle along the angle, as far as the box reaches that way.
    pub fn points(&self, bounds: Rect) -> (Point, Point) {
        let (sin, cos) = (self.angle - FRAC_PI_2).sin_cos();
        let reach = (cos * bounds.w / 2.0).abs().max((sin * bounds.h / 2.0).abs());
        let c = bounds.center();
        (Point::new(c.x - cos * reach, c.y - sin * reach), Point::new(c.x + cos * reach, c.y + sin * reach))
    }
}

impl Hash for Linear {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.angle]);
        self.stops.hash(state);
    }
}

/// What the inside of a shape is painted with. A gradient spans the box of the shape: the rect
/// of a rect op, or the bounds of a path.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Fill {
    Solid(Rgba),
    Linear(Linear),
}

impl From<Rgba> for Fill {
    fn from(c: Rgba) -> Fill {
        Fill::Solid(c)
    }
}

/// Which parts of a path that crosses itself are inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Rule {
    #[default]
    NonZero,
    EvenOdd,
}

/// How a stroke's open ends are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Cap {
    #[default]
    Butt,
    Round,
    Square,
}

/// How a stroke turns its corners. A miter is cut off past 4 times the width, as in tiny-skia
/// and SVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Join {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// A line along a path, centered on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: Rgba,
    pub cap: Cap,
    pub join: Join,
}

impl Stroke {
    /// Butt ends and mitered corners, SVG's defaults.
    pub fn new(width: f32, color: Rgba) -> Stroke {
        Stroke { width, color, cap: Cap::Butt, join: Join::Miter }
    }

    /// How far past the path the stroke can reach.
    pub fn reach(&self) -> f32 {
        let half = self.width / 2.0;
        match (self.join, self.cap) {
            (Join::Miter, _) => half * 4.0,
            (_, Cap::Square) => half * std::f32::consts::SQRT_2,
            _ => half,
        }
    }
}

impl Hash for Stroke {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_floats(state, &[self.width]);
        (self.color, self.cap, self.join).hash(state);
    }
}
