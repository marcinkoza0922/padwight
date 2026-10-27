//! Doodle's canvas: the strokes and text boxes drawn over the game, and the pen, eraser and text
//! tools that put them there. The Doodle screen (`doodle/screen.rs`) feeds it the controller, and
//! the overlay draws what it holds. The daemon keeps one canvas, so doodles stay on screen after
//! the tool closes, until they're wiped.
//!
//! Positions are shares of the screen: x of its width, y of its height, 0 to 1, as the magnifying
//! glass's place is. Sizes are shares of its height, so a doodle keeps its proportions at any
//! resolution. Distances are measured as if the screen were 16:9, which is close enough for hit
//! tests on any other shape.

use serde::{Deserialize, Serialize};

mod screen;
mod text;

pub use screen::{Doodle, Step};

/// The palette's colors, as the pen and text pick them. Placeholder values until the palette is
/// chosen.
pub const PALETTE: [[u8; 3]; 12] = [
    [255, 255, 255],
    [0, 0, 0],
    [230, 57, 70],
    [241, 143, 1],
    [255, 210, 63],
    [106, 190, 48],
    [0, 150, 136],
    [33, 150, 243],
    [63, 81, 181],
    [156, 39, 176],
    [233, 30, 99],
    [121, 85, 72],
];

/// Font names the text tool cycles through. Placeholder until the list is chosen.
pub const FONTS: [&str; 8] = ["Sans", "Serif", "Mono", "Hand", "Slab", "Round", "Display", "Condensed"];

/// Size limits for the pen and for text, as shares of the screen's height, the size they start
/// at, and how far a D-pad press moves them.
pub const PEN_SIZES: (f32, f32) = (0.002, 0.044);
pub const PEN_SIZE: f32 = 0.006;
pub const TEXT_SIZES: (f32, f32) = (0.011, 0.09);
pub const TEXT_SIZE: f32 = 0.026;
pub const SIZE_STEP: f32 = 0.002;
pub const TEXT_STEP: f32 = 0.004;

/// How far apart a stroke's points are kept, as a share of its pen's size, and at least.
const POINT_STEP: f32 = 0.2;
const MIN_POINT_STEP: f32 = 0.001;

/// The width of a 16:9 screen as a multiple of its height, which distances are measured with.
const ASPECT: f32 = 16.0 / 9.0;
/// Rough size of a character as a share of the font size. The overlay measures real text when it
/// draws; this only sizes a text box for hit tests.
const CHAR_WIDTH: f32 = 0.6;
const LINE_HEIGHT: f32 = 1.2;

/// A point on the screen: x of its width, y of its height, 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    /// The distance to `other`, in screen heights.
    fn distance(self, other: Point) -> f32 {
        ((self.x - other.x) * ASPECT).hypot(self.y - other.y)
    }

    /// The distance from `self` to the nearest point of the segment from `a` to `b`.
    fn distance_to_segment(self, a: Point, b: Point) -> f32 {
        let (dx, dy) = ((b.x - a.x) * ASPECT, b.y - a.y);
        let (px, py) = ((self.x - a.x) * ASPECT, self.y - a.y);
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            return self.distance(a);
        }
        let t = ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0);
        (px - t * dx).hypot(py - t * dy)
    }
}

/// The next entry in `all` after `current`, wrapping round to the first.
fn cycle<T: Copy + PartialEq>(all: &[T], current: T) -> T {
    let index = all.iter().position(|&v| v == current).unwrap_or(0);
    all[(index + 1) % all.len()]
}

/// `value` moved by `steps` of `step`, kept within `limits`.
pub fn resize(value: f32, steps: i32, limits: (f32, f32), step: f32) -> f32 {
    (value + steps as f32 * step).clamp(limits.0, limits.1)
}

/// The tools, in the order LB and RB step through them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Pen,
    Eraser,
    Text,
}

impl Tool {
    const ALL: [Tool; 3] = [Tool::Pen, Tool::Eraser, Tool::Text];

    /// Where the tool sits in the carousel, from 0.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    /// The tool after this one, wrapping from Text back to Pen.
    pub fn next(self) -> Tool {
        cycle(&Self::ALL, self)
    }

    /// The tool before this one, wrapping from Pen to Text.
    pub fn previous(self) -> Tool {
        let index = Self::ALL.iter().position(|&t| t == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// The tip a pen stroke is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tip {
    #[default]
    Round,
    Square,
    Chisel,
    Diagonal,
}

impl Tip {
    const ALL: [Tip; 4] = [Tip::Round, Tip::Square, Tip::Chisel, Tip::Diagonal];

    pub fn next(self) -> Tip {
        cycle(&Self::ALL, self)
    }
}

/// The kind of mark a pen stroke makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    Solid,
    Brush,
    Spray,
}

impl Kind {
    const ALL: [Kind; 3] = [Kind::Solid, Kind::Brush, Kind::Spray];

    pub fn next(self) -> Kind {
        cycle(&Self::ALL, self)
    }
}

/// How the pen draws: the color (an index into [`PALETTE`]), the size, the tip and the kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pen {
    pub color: usize,
    pub size: f32,
    pub tip: Tip,
    pub kind: Kind,
}

impl Default for Pen {
    fn default() -> Self {
        Pen { color: 2, size: PEN_SIZE, tip: Tip::Round, kind: Kind::Solid }
    }
}

/// A pen stroke: the pen it was drawn with, and the points the stick moved through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub pen: Pen,
    pub points: Vec<Point>,
}

impl Stroke {
    /// Whether `p` is within `reach` of the stroke's line, both in screen heights.
    fn touched_by(&self, p: Point, reach: f32) -> bool {
        let reach = reach + self.pen.size / 2.0;
        match self.points.as_slice() {
            [only] => p.distance(*only) <= reach,
            points => points.windows(2).any(|w| p.distance_to_segment(w[0], w[1]) <= reach),
        }
    }
}

/// How the text tool styles text. Italic, bold and underline belong to each box, not to the tool.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub color: usize,
    pub font: usize,
    pub size: f32,
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle { color: 0, font: 0, size: TEXT_SIZE, italic: false, bold: false, underline: false }
    }
}

/// A text box: where it sits, what it says, and how it's styled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBox {
    pub at: Point,
    pub text: String,
    pub style: TextStyle,
}

impl TextBox {
    /// The box's rough extent on screen, as its top left and bottom right corners.
    fn bounds(&self) -> (Point, Point) {
        let width = self.text.chars().count() as f32 * self.style.size * CHAR_WIDTH / ASPECT;
        let height = self.style.size * LINE_HEIGHT;
        (self.at, Point::new(self.at.x + width, self.at.y + height))
    }

    fn touched_by(&self, p: Point, reach: f32) -> bool {
        let (min, max) = self.bounds();
        let reach_x = reach / ASPECT;
        p.x >= min.x - reach_x && p.x <= max.x + reach_x && p.y >= min.y - reach && p.y <= max.y + reach
    }
}

/// One thing on the canvas: a stroke or a text box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Stroke(Stroke),
    Text(TextBox),
}

impl Item {
    fn touched_by(&self, p: Point, reach: f32) -> bool {
        match self {
            Item::Stroke(stroke) => stroke.touched_by(p, reach),
            Item::Text(text) => text.touched_by(p, reach),
        }
    }
}

/// The doodles on screen: the finished items in the order they were added, and the stroke the
/// pen is drawing now, if any.
///
/// Undo takes back the last item added. Erasing removes an item for good.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    items: Vec<Item>,
    pending: Option<Stroke>,
}

impl Canvas {
    /// The finished items, bottom to top.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The stroke being drawn, which isn't an item until the pen lifts.
    pub fn pending(&self) -> Option<&Stroke> {
        self.pending.as_ref()
    }

    /// Whether there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.pending.is_none()
    }

    /// Puts the pen down at `at`, with `pen`.
    pub fn begin_stroke(&mut self, pen: Pen, at: Point) {
        self.pending = Some(Stroke { pen, points: vec![at] });
    }

    /// Moves the drawing stroke to `at`. Does nothing when the pen is up. A point that lands
    /// closer than [`POINT_STEP`] of the pen's size to the one before the stroke's last moves the
    /// last one there instead, so a stroke gains a point a step along rather than one each tick:
    /// fewer points to send and draw, and its end is still where the cursor is.
    pub fn extend_stroke(&mut self, at: Point) {
        let Some(stroke) = &mut self.pending else { return };
        let step = (stroke.pen.size * POINT_STEP).max(MIN_POINT_STEP);
        match stroke.points.as_mut_slice() {
            [.., before, last] if before.distance(at) < step => *last = at,
            _ => stroke.points.push(at),
        }
    }

    /// Lifts the pen, and adds the stroke to the canvas.
    pub fn end_stroke(&mut self) {
        if let Some(stroke) = self.pending.take() {
            self.items.push(Item::Stroke(stroke));
        }
    }

    /// Adds a text box, and returns its index.
    pub fn add_text(&mut self, text: TextBox) -> usize {
        self.items.push(Item::Text(text));
        self.items.len() - 1
    }

    /// The text box at `index`, to edit or move.
    pub fn text_mut(&mut self, index: usize) -> Option<&mut TextBox> {
        match self.items.get_mut(index) {
            Some(Item::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// The topmost text box under `p`, as its index. Only text boxes count, so South on one
    /// edits it rather than making a second box on top.
    pub fn text_at(&self, p: Point) -> Option<usize> {
        self.items.iter().rposition(|item| matches!(item, Item::Text(text) if text.touched_by(p, 0.0)))
    }

    /// Removes the topmost item that `p` touches, within `radius` (a share of the height). Returns
    /// whether one went.
    pub fn erase_at(&mut self, p: Point, radius: f32) -> bool {
        match self.items.iter().rposition(|item| item.touched_by(p, radius)) {
            Some(index) => {
                self.items.remove(index);
                true
            }
            None => false,
        }
    }

    /// Removes the item at `index`, if there is one.
    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }

    /// Takes back the last item added. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        self.items.pop().is_some()
    }

    /// Clears every doodle and text box, and any stroke in progress.
    pub fn wipe(&mut self) {
        self.items.clear();
        self.pending = None;
    }
}

#[cfg(test)]
mod tests;
