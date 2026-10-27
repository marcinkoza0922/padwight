//! The Doodle screen's controls: the left stick moves the cursor, South uses the current tool,
//! LB and RB change tools, and the D-pad sets each tool's settings. Text boxes take the on-screen
//! keyboard (see `text.rs`). It edits the daemon's canvas and says when it's done. The canvas
//! itself is in the parent module.

#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use super::{
    Canvas, FONTS, PALETTE, PEN_SIZES, Pen, Point, SIZE_STEP, TEXT_SIZES, TEXT_STEP, TextStyle, Tool,
    resize,
    text::{Entry, Grab, finished},
};
use crate::{
    config::{Button, OverlayStyle},
    input::{Axis, InputEvent},
    overlay::{DoodleCell, DoodleCursor, DoodleHud, KeyboardView, OverlayAction},
};

/// How long East is held to leave Doodle. A shorter press undoes the last item instead.
pub const EXIT_HOLD: Duration = Duration::from_millis(500);
/// How fast the left stick moves the cursor at full push, in screen heights a second.
const CURSOR_SPEED: f32 = 1.2;
/// Stick pushes this small don't move the cursor.
const DEAD_ZONE: f32 = 0.12;
/// The eraser's reach, as a share of the screen's height: how far from a doodle it still touches.
const REACH: (f32, f32) = (0.002, 0.06);
const REACH_STEP: f32 = 0.004;
const DEFAULT_REACH: f32 = 0.01;
/// The smallest the cursor's ring is, so it's seen whatever the pen's size.
pub(super) const CURSOR_MIN: f32 = 0.012;
/// The screen height sizes are shown against, so they read as pixels on a 1080-line screen.
const SHOWN_HEIGHT: f32 = 1080.0;


/// What a press asks the daemon to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Stay in Doodle.
    Stay,
    /// Hand the controller back. The doodles stay on screen.
    Leave,
}

/// The Doodle screen as it is: where the cursor is, which tool is up and its settings, and what
/// the controller is doing with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Doodle {
    cursor: Point,
    /// The left stick's push, -1 to 1 (y positive down), past the dead zone.
    push: (f32, f32),
    tool: Tool,
    pen: Pen,
    text: TextStyle,
    reach: f32,
    /// South is held in a tool that marks: the pen draws, the eraser erases.
    marking: bool,
    /// When East went down, while it may still be a hold that leaves.
    east_down: Option<Instant>,
    /// Whether L3 and R3 are held, which together wipe the canvas.
    clicks: [bool; 2],
    /// The keyboard typing into a text box, which takes the controller while it's up.
    entry: Option<Entry>,
    /// A text box South is holding, to move or to open.
    grab: Option<Grab>,
}

impl Doodle {
    pub fn new(cursor: Point) -> Doodle {
        Doodle {
            cursor,
            push: (0.0, 0.0),
            tool: Tool::default(),
            pen: Pen::default(),
            text: TextStyle::default(),
            reach: DEFAULT_REACH,
            marking: false,
            east_down: None,
            clicks: [false; 2],
            entry: None,
            grab: None,
        }
    }

    /// The cursor as the overlay draws it: a ring the size of what the tool touches, in the color
    /// of its ink. It's always there on this screen.
    pub fn cursor_ring(&self) -> DoodleCursor {
        let (radius, color) = match self.tool {
            Tool::Pen => (self.pen.size / 2.0, PALETTE[self.pen.color]),
            Tool::Eraser => (self.reach, [200, 200, 200]),
            Tool::Text => (self.text.size / 2.0, PALETTE[self.text.color]),
        };
        DoodleCursor { at: self.cursor, radius: radius.max(CURSOR_MIN), color }
    }

    /// The HUD: the tool for the carousel, and the tray's cells with a hint for each control.
    pub fn hud(&self, style: OverlayStyle) -> DoodleHud {
        let shown = |share: f32| (share * SHOWN_HEIGHT).round() as i32;
        let cells = match self.tool {
            Tool::Pen => vec![
                DoodleCell::Palette { current: self.pen.color, hint: "{leftright} colour".into() },
                DoodleCell::Text(format!("{{updown}} size {}", shown(self.pen.size))),
                DoodleCell::Tip { tip: self.pen.tip, text: format!("{{north}} {:?}", self.pen.tip) },
                DoodleCell::Kind { kind: self.pen.kind, text: format!("{{west}} {:?}", self.pen.kind) },
            ],
            Tool::Eraser => vec![DoodleCell::Text(format!("{{updown}} reach {}", shown(self.reach)))],
            Tool::Text => {
                let faces: Vec<&str> = [(self.text.italic, "italic"), (self.text.bold, "bold"), (self.text.underline, "underline")]
                    .into_iter()
                    .filter_map(|(on, face)| on.then_some(face))
                    .collect();
                let faces = if faces.is_empty() { "plain".to_string() } else { faces.join(" ") };
                vec![
                    DoodleCell::Palette { current: self.text.color, hint: "{leftright} colour".into() },
                    DoodleCell::Text(format!("{{updown}} size {}", shown(self.text.size))),
                    DoodleCell::Text(format!("{{west}} font {}", FONTS[self.text.font])),
                    DoodleCell::Text(format!("{{north}} style {faces}")),
                ]
            }
        };
        DoodleHud { tool: self.tool.index(), tray: self.entry.is_none(), cells, style }
    }

    /// The keyboard typing into a text box, as the overlay draws it, while it's up.
    pub fn keyboard(&self, now: Instant) -> Option<KeyboardView> {
        self.entry.as_ref().map(|entry| entry.view(now))
    }

    /// Whether the cursor, the keyboard or the East hold needs the screen ticked.
    pub fn needs_tick(&self) -> bool {
        self.entry.is_some() || self.grab.is_some() || self.push != (0.0, 0.0) || self.marking || self.east_down.is_some()
    }

    /// Takes a stick reading. The left stick moves the cursor, or types while the keyboard is up.
    pub fn axis(&mut self, canvas: &mut Canvas, axis: Axis, value: f32, now: Instant) {
        if self.entry.is_some() {
            self.keys(canvas, InputEvent::Axis(axis, value), now);
            return;
        }
        let value = if value.abs() < DEAD_ZONE { 0.0 } else { value.clamp(-1.0, 1.0) };
        match axis {
            Axis::LeftX => self.push.0 = value,
            Axis::LeftY => self.push.1 = value,
            _ => {}
        }
    }

    /// Moves the cursor by the stick's push, and carries a mark or a grabbed text box along with
    /// it. Returns `Leave` once East has been held for [`EXIT_HOLD`].
    pub fn tick(&mut self, canvas: &mut Canvas, dt: f32, now: Instant) -> Step {
        if let Some(entry) = &mut self.entry {
            let (actions, _) = entry.keys().tick(now);
            self.finish_entry(canvas, &actions);
            return Step::Stay;
        }
        if self.east_down.is_some_and(|went_down| now.duration_since(went_down) >= EXIT_HOLD) {
            return Step::Leave;
        }
        let before = self.cursor;
        self.cursor = Point::new(
            (self.cursor.x + self.push.0 * CURSOR_SPEED * dt).clamp(0.0, 1.0),
            (self.cursor.y + self.push.1 * CURSOR_SPEED * dt).clamp(0.0, 1.0),
        );
        if self.cursor != before {
            self.carry(canvas, before);
        }
        Step::Stay
    }

    /// Moves what the cursor carries: a mark, or the text box South holds.
    fn carry(&mut self, canvas: &mut Canvas, before: Point) {
        if let Some(grab) = &mut self.grab {
            let (dx, dy) = (self.cursor.x - before.x, self.cursor.y - before.y);
            if let Some(text) = canvas.text_mut(grab.index) {
                text.at = Point::new(text.at.x + dx, text.at.y + dy);
            }
            grab.moved = true;
        } else if self.marking {
            match self.tool {
                Tool::Pen => canvas.extend_stroke(self.cursor),
                Tool::Eraser => {
                    canvas.erase_at(self.cursor, self.reach);
                }
                Tool::Text => {}
            }
        }
    }

    /// Handles a press (`down`) or a release of `b`.
    pub fn handle(&mut self, canvas: &mut Canvas, b: Button, down: bool, now: Instant) -> Step {
        if self.entry.is_some() {
            self.keys(canvas, InputEvent::Button(b, down), now);
            return Step::Stay;
        }
        match b {
            Button::Guide if down => return Step::Leave,
            Button::East => self.east(canvas, down, now),
            Button::LeftStick | Button::RightStick => self.click(canvas, b, down),
            Button::South => self.south(canvas, down),
            Button::LeftBumper if down => self.change_tool(canvas, self.tool.previous()),
            Button::RightBumper if down => self.change_tool(canvas, self.tool.next()),
            _ if down => self.setting(b),
            _ => {}
        }
        Step::Stay
    }

    /// Hands a controller event to the keyboard, and does what the keyboard's keys ask. Keys
    /// typed go to the text box, never to the game.
    fn keys(&mut self, canvas: &mut Canvas, ev: InputEvent, now: Instant) {
        let Some(entry) = &mut self.entry else { return };
        let actions = entry.keys().handle(ev, now);
        self.finish_entry(canvas, &actions);
    }

    /// Ends the keyboard when it asks to be done, setting the text box or closing without.
    fn finish_entry(&mut self, canvas: &mut Canvas, actions: &[OverlayAction]) {
        let Some(done) = finished(actions) else { return };
        if let Some(entry) = self.entry.take()
            && done
        {
            entry.commit(canvas, self.text);
        }
    }

    /// A tap of East undoes the last item. Holding it leaves, which [`Doodle::tick`] says once the
    /// hold is long enough, so a long hold doesn't also undo.
    fn east(&mut self, canvas: &mut Canvas, down: bool, now: Instant) {
        if down {
            self.east_down = Some(now);
        } else if let Some(went_down) = self.east_down.take()
            && now.duration_since(went_down) < EXIT_HOLD
            && !self.marking
        {
            canvas.undo();
        }
    }

    /// L3 and R3 pressed together wipe every doodle. Each click only counts while it's held.
    fn click(&mut self, canvas: &mut Canvas, b: Button, down: bool) {
        let index = usize::from(b == Button::RightStick);
        self.clicks[index] = down;
        if down && self.clicks == [true; 2] {
            canvas.wipe();
            self.marking = false;
            self.grab = None;
        }
    }

    /// South marks with the pen or the eraser, and on text it opens a new box or grabs one. Lifting
    /// it ends the mark, and opens a box that was grabbed but never moved.
    fn south(&mut self, canvas: &mut Canvas, down: bool) {
        if !down {
            canvas.end_stroke();
            self.marking = false;
            if let Some(grab) = self.grab.take()
                && !grab.moved
                && let Some(text) = canvas.text_mut(grab.index)
            {
                self.entry = Some(Entry::editing(grab.index, text.at, &text.text));
            }
            return;
        }
        match self.tool {
            Tool::Pen => canvas.begin_stroke(self.pen, self.cursor),
            Tool::Eraser => {
                canvas.erase_at(self.cursor, self.reach);
            }
            Tool::Text => {
                match canvas.text_at(self.cursor) {
                    Some(index) => self.grab = Some(Grab { index, last: self.cursor, moved: false }),
                    None => self.entry = Some(Entry::new(self.cursor)),
                }
                return;
            }
        }
        self.marking = true;
    }

    /// Changes tool. A mark in progress is finished first, so the strokes don't join up.
    fn change_tool(&mut self, canvas: &mut Canvas, tool: Tool) {
        canvas.end_stroke();
        self.marking = false;
        self.grab = None;
        self.tool = tool;
    }

    /// The D-pad and the face buttons, which set the current tool's settings.
    fn setting(&mut self, b: Button) {
        let steps = match b {
            Button::DpadUp => 1,
            Button::DpadDown => -1,
            _ => 0,
        };
        let sideways = match b {
            Button::DpadRight => 1,
            Button::DpadLeft => -1,
            _ => 0,
        };
        match self.tool {
            Tool::Pen => {
                self.pen.size = resize(self.pen.size, steps, PEN_SIZES, SIZE_STEP);
                self.pen.color = (self.pen.color as i32 + sideways).rem_euclid(PALETTE.len() as i32) as usize;
                match b {
                    Button::North => self.pen.tip = self.pen.tip.next(),
                    Button::West => self.pen.kind = self.pen.kind.next(),
                    _ => {}
                }
            }
            Tool::Eraser => self.reach = resize(self.reach, steps, REACH, REACH_STEP),
            Tool::Text => {
                self.text.size = resize(self.text.size, steps, TEXT_SIZES, TEXT_STEP);
                self.text.color = (self.text.color as i32 + sideways).rem_euclid(PALETTE.len() as i32) as usize;
                match b {
                    Button::West => self.text.font = (self.text.font + 1) % FONTS.len(),
                    Button::North => self.face_next(),
                    _ => {}
                }
            }
        }
    }

    /// North steps the text through italic, bold, underline and plain, one at a time.
    fn face_next(&mut self) {
        let (italic, bold, underline) = (self.text.italic, self.text.bold, self.text.underline);
        (self.text.italic, self.text.bold, self.text.underline) = match (italic, bold, underline) {
            (false, false, false) => (true, false, false),
            (true, false, false) => (true, true, false),
            (true, true, false) => (true, true, true),
            _ => (false, false, false),
        };
    }
}
