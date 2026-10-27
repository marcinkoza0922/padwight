//! The Tools menu, opened over the game with Guide + right trigger: a list of small tools, and the
//! calculator first among them. This holds the menu's cursor and what each press does; the daemon
//! routes the controller here and does what an [`Outcome`] asks (types, copies or closes).

pub mod calculator;

use calculator::{Calculator, Press, symbol};

use crate::{
    config::{Button, MenuKind, OverlayStyle},
    menu::{ItemView, MenuView},
    nav::{self, Dir},
};

/// The tools on the list, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Calculator,
}

const TOOLS: [Tool; 1] = [Tool::Calculator];

impl Tool {
    fn name(self) -> &'static str {
        match self {
            Tool::Calculator => "Calculator",
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
}

/// The open Tools menu: which screen, where its cursor is, and the calculator's keys, which
/// are kept from one opening of the calculator to the next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsSession {
    screen: Screen,
    cursor: usize,
    calc: Calculator,
}

impl Default for ToolsSession {
    fn default() -> Self {
        ToolsSession { screen: Screen::List, cursor: 0, calc: Calculator::default() }
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

    /// Handles one press of a button the menu takes. `None` when the menu stays as it is, or
    /// only changes on screen.
    pub fn handle(&mut self, b: Button) -> Option<Outcome> {
        match self.screen {
            Screen::List => self.list(b),
            Screen::Calculator => self.pad(b),
        }
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

    /// The menu as the overlay draws it.
    pub fn view(&self, style: OverlayStyle) -> MenuView {
        match self.screen {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Moves the cursor to the keypad cell `index`, and presses it.
    fn tap(session: &mut ToolsSession, index: usize) -> Option<Outcome> {
        session.cursor = index;
        session.handle(Button::South)
    }

    /// The keypad cell at `row`, `column`.
    fn at(row: usize, column: usize) -> usize {
        row * COLUMNS + column
    }

    #[test]
    fn east_closes_the_list_and_south_opens_the_calculator() {
        let mut session = ToolsSession::new();
        assert_eq!(session.handle(Button::South), None);
        assert_eq!(session.view(OverlayStyle::default()).title, "Calculator");
        assert_eq!(session.handle(Button::East), None);
        assert_eq!(session.view(OverlayStyle::default()).title, "Tools");
        assert_eq!(session.handle(Button::East), Some(Outcome::Close));
    }

    #[test]
    fn the_keypad_wraps_at_its_edges() {
        assert_eq!(pad_step(at(0, 0), (-1, 0)), at(0, COLUMNS - 1));
        assert_eq!(pad_step(at(0, 0), (0, -1)), at(PAD.len() / COLUMNS - 1, 0));
        assert_eq!(pad_step(at(1, 1), (1, 1)), at(2, 2));
    }

    #[test]
    fn the_d_pad_moves_across_the_keypad() {
        let mut session = ToolsSession::new();
        session.handle(Button::South);
        session.handle(Button::DpadRight);
        assert_eq!(session.cursor(), 1);
        session.handle(Button::DpadDown);
        assert_eq!(session.cursor(), COLUMNS + 1);
    }

    #[test]
    fn a_sum_is_keyed_on_the_pad_and_sent_to_the_game() {
        let mut session = ToolsSession::new();
        session.handle(Button::South);
        // 7 × 6 =, then Send
        for index in [0, 7, 6, 17] {
            assert_eq!(tap(&mut session, index), None);
        }
        assert_eq!(session.calc.line(), "42");
        assert_eq!(tap(&mut session, 18), Some(Outcome::Type("42".into())));
    }

    #[test]
    fn copy_sends_the_answer_to_the_clipboard_and_an_empty_sum_says_so() {
        let mut session = ToolsSession::new();
        session.handle(Button::South);
        assert_eq!(tap(&mut session, 19), None);
        assert_eq!(session.calc.line(), "Nothing to send yet");
        tap(&mut session, 5);
        assert_eq!(tap(&mut session, 19), Some(Outcome::Copy("5".into())));
    }

    #[test]
    fn a_sum_left_unfinished_is_sent_as_the_number_before_its_operator() {
        let mut session = ToolsSession::new();
        session.handle(Button::South);
        tap(&mut session, 5);
        tap(&mut session, 7);
        assert_eq!(tap(&mut session, 18), Some(Outcome::Type("5".into())));
    }

    #[test]
    fn the_calculator_keeps_its_sum_when_the_list_is_opened_again() {
        let mut session = ToolsSession::new();
        session.handle(Button::South);
        tap(&mut session, 2);
        session.handle(Button::East);
        session.handle(Button::South);
        assert_eq!(session.calc.line(), "9");
    }
}
