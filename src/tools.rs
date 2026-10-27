//! The Tools menu, opened over the game with Guide + right trigger: a list of small tools, and the
//! calculator first among them. This holds the menu's cursor and what each press does; the daemon
//! routes the controller here and does what an [`Outcome`] asks (types, copies or closes).

pub mod calculator;
pub mod doodle;

use std::time::Instant;

use calculator::{Calculator, Press, symbol};
use doodle::{Canvas, Doodle, Point, Step};

use crate::{
    config::{Button, MenuKind, OverlayStyle},
    input::Axis,
    menu::{ItemView, MenuView},
    nav::{self, Dir},
    overlay::{DoodleCursor, DoodleHud, KeyboardView},
};

/// The tools on the list, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Calculator,
    Doodle,
}

const TOOLS: [Tool; 2] = [Tool::Calculator, Tool::Doodle];

impl Tool {
    fn name(self) -> &'static str {
        match self {
            Tool::Calculator => "Calculator",
            Tool::Doodle => "Doodle",
        }
    }
}

/// Columns of the calculator's keypad.
const COLUMNS: usize = 4;

/// One cell of the keypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Calc(Press),
    /// Types the answer into the game, and closes.
    Send,
    /// Copies the answer to the clipboard, and closes.
    Copy,
}

const fn digit(c: char) -> Cell {
    Cell::Calc(Press::Digit(c))
}

const fn op(c: char) -> Cell {
    Cell::Calc(Press::Op(c))
}

/// The keypad, row by row: four columns, five rows.
const PAD: [Cell; 20] = [
    digit('7'), digit('8'), digit('9'), op('/'),
    digit('4'), digit('5'), digit('6'), op('*'),
    digit('1'), digit('2'), digit('3'), op('-'),
    digit('0'), Cell::Calc(Press::Dot), Cell::Calc(Press::Backspace), op('+'),
    Cell::Calc(Press::Clear), Cell::Calc(Press::Equals), Cell::Send, Cell::Copy,
];

impl Cell {
    fn label(self) -> String {
        match self {
            Cell::Calc(Press::Digit(c)) => c.to_string(),
            Cell::Calc(Press::Op(c)) => symbol(c).to_string(),
            Cell::Calc(Press::Dot) => ".".into(),
            Cell::Calc(Press::Backspace) => "⌫".into(),
            Cell::Calc(Press::Clear) => "C".into(),
            Cell::Calc(Press::Equals) => "=".into(),
            Cell::Send => "Send".into(),
            Cell::Copy => "Copy".into(),
        }
    }
}

/// What a press asks the daemon to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Put the menu away.
    Close,
    /// Type this text into the game, then put the menu away.
    Type(String),
    /// Copy this text to the clipboard, then put the menu away.
    Copy(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    List,
    Calculator,
    /// Drawing over the game: the menu is gone, and the doodles and the cursor are what shows.
    Doodle,
}

/// The open Tools menu: which screen, where its cursor is, and the calculator's keys, which
/// are kept from one opening of the calculator to the next. The doodles themselves belong to the
/// daemon, so they stay on screen once the menu is closed.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolsSession {
    screen: Screen,
    cursor: usize,
    calc: Calculator,
    doodle: Doodle,
}

impl Default for ToolsSession {
    fn default() -> Self {
        ToolsSession { screen: Screen::List, cursor: 0, calc: Calculator::default(), doodle: Doodle::new(Point::new(0.5, 0.5)) }
    }
}

/// The direction a D-pad button or a left stick push is, if it is one.
fn direction(b: Button) -> Option<Dir> {
    nav::dpad_dir(b).or(match b {
        Button::LeftStickUp => Some((0, -1)),
        Button::LeftStickDown => Some((0, 1)),
        Button::LeftStickLeft => Some((-1, 0)),
        Button::LeftStickRight => Some((1, 0)),
        _ => None,
    })
}

/// Where the cursor goes on the keypad, wrapping at the edges.
fn pad_step(cursor: usize, (dx, dy): Dir) -> usize {
    let rows = (PAD.len() / COLUMNS) as i32;
    let col = (cursor % COLUMNS) as i32 + dx;
    let row = (cursor / COLUMNS) as i32 + dy;
    row.rem_euclid(rows) as usize * COLUMNS + col.rem_euclid(COLUMNS as i32) as usize
}

impl ToolsSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the cursor is on the screen that's up.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Handles a press (`down`) or a release of a button. The menus only act on presses; the
    /// Doodle screen takes both. `None` when the menu stays as it is, or only changes on screen.
    pub fn handle(&mut self, b: Button, down: bool, canvas: &mut Canvas, now: Instant) -> Option<Outcome> {
        match self.screen {
            Screen::Doodle => match self.doodle.handle(canvas, b, down, now) {
                Step::Stay => None,
                Step::Leave => self.leave_doodle(),
            },
            _ if !down => None,
            Screen::List => self.list(b),
            Screen::Calculator => self.pad(b),
        }
    }

    /// Takes a stick reading, which only the Doodle screen uses.
    pub fn axis(&mut self, canvas: &mut Canvas, axis: Axis, value: f32, now: Instant) {
        if self.screen == Screen::Doodle {
            self.doodle.axis(canvas, axis, value, now);
        }
    }

    /// Whether the Doodle screen needs ticking, to move its cursor or carry a mark.
    pub fn needs_tick(&self) -> bool {
        self.screen == Screen::Doodle && self.doodle.needs_tick()
    }

    /// Moves the Doodle cursor on, once a tick has passed.
    pub fn tick(&mut self, canvas: &mut Canvas, dt: f32, now: Instant) -> Option<Outcome> {
        if self.screen != Screen::Doodle {
            return None;
        }
        match self.doodle.tick(canvas, dt, now) {
            Step::Stay => None,
            Step::Leave => self.leave_doodle(),
        }
    }

    /// Whether the Doodle screen is up.
    pub fn in_doodle(&self) -> bool {
        self.screen == Screen::Doodle
    }

    /// The Doodle cursor, while the Doodle screen is up.
    pub fn doodle_cursor(&self) -> Option<DoodleCursor> {
        self.in_doodle().then(|| self.doodle.cursor_ring())
    }

    /// The Doodle screen's HUD, while it's up, drawn in `style`.
    pub fn doodle_hud(&self, style: OverlayStyle) -> Option<DoodleHud> {
        self.in_doodle().then(|| self.doodle.hud(style))
    }

    /// The keyboard for a text box, while it's up in place of the menu.
    pub fn keyboard(&self, now: Instant) -> Option<KeyboardView> {
        self.doodle.keyboard(now)
    }

    /// Leaves the Doodle screen for good: the menu closes, and the doodles stay where they are.
    fn leave_doodle(&mut self) -> Option<Outcome> {
        self.screen = Screen::List;
        Some(Outcome::Close)
    }

    fn list(&mut self, b: Button) -> Option<Outcome> {
        if let Some((_, dy)) = direction(b) {
            self.cursor = (self.cursor as i32 + dy).rem_euclid(TOOLS.len() as i32) as usize;
        }
        match (b, TOOLS[self.cursor]) {
            (Button::South, Tool::Calculator) => {
                self.screen = Screen::Calculator;
                self.cursor = 0;
                None
            }
            (Button::South, Tool::Doodle) => {
                self.screen = Screen::Doodle;
                None
            }
            (Button::East, _) => Some(Outcome::Close),
            _ => None,
        }
    }

    fn pad(&mut self, b: Button) -> Option<Outcome> {
        if b == Button::East {
            self.screen = Screen::List;
            self.cursor = 0;
            return None;
        }
        if b == Button::South {
            return self.press(PAD[self.cursor]);
        }
        if let Some(dir) = direction(b) {
            self.cursor = pad_step(self.cursor, dir);
        }
        None
    }

    fn press(&mut self, cell: Cell) -> Option<Outcome> {
        match cell {
            Cell::Calc(key) => self.calc.press(key),
            Cell::Send => return self.answer().map(Outcome::Type),
            Cell::Copy => return self.answer().map(Outcome::Copy),
        }
        None
    }

    /// The answer to send or copy. When there is none, the calculator says why and stays up.
    fn answer(&mut self) -> Option<String> {
        match self.calc.answer() {
            Ok(text) => Some(text),
            Err(message) => {
                self.calc.fail(message);
                None
            }
        }
    }

    /// The menu as the overlay draws it. `None` on the Doodle screen, which draws no menu.
    pub fn view(&self, style: OverlayStyle) -> Option<MenuView> {
        Some(match self.screen {
            Screen::Doodle => return None,
            Screen::List => MenuView {
                title: "Tools".into(),
                kind: MenuKind::List,
                items: TOOLS.iter().map(|tool| ItemView { label: tool.name().into(), submenu: true, ..ItemView::plain() }).collect(),
                selected: Some(self.cursor),
                picks: 0,
                crumbs: Vec::new(),
                hint: "{confirm} choose · {back} close".into(),
                about: String::new(),
                style,
                touch: false,
                aim: None,
                prompt: None,
                one_column: false,
                figure: None,
            },
            Screen::Calculator => MenuView {
                title: Tool::Calculator.name().into(),
                kind: MenuKind::Grid { columns: COLUMNS as u8 },
                items: PAD.iter().map(|cell| ItemView { label: cell.label(), ..ItemView::plain() }).collect(),
                selected: Some(self.cursor),
                picks: 0,
                crumbs: vec!["Tools".into()],
                hint: "{dpad} move · {confirm} press · {back} back".into(),
                about: self.calc.line(),
                style,
                touch: false,
                aim: None,
                prompt: None,
                one_column: false,
                figure: None,
            },
        })
    }
}

#[cfg(test)]
mod tests;
