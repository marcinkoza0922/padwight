//! The keys the controller types on: the basic and full keyboards and the numpad, and moving a
//! cursor across them.

use super::{Key, Layout, gap, k, w};

/// The keys for typing into a game: letters, digits and symbols, Enter, Backspace, Escape, AltGr
/// for the characters a layout puts there, and the arrows for moving through the text.
const BASIC: [&[Key]; 5] = [
    &[
        k("`", "KEY_GRAVE"), k("1", "KEY_1"), k("2", "KEY_2"), k("3", "KEY_3"), k("4", "KEY_4"),
        k("5", "KEY_5"), k("6", "KEY_6"), k("7", "KEY_7"), k("8", "KEY_8"), k("9", "KEY_9"),
        k("0", "KEY_0"), k("-", "KEY_MINUS"), k("=", "KEY_EQUAL"), w("Backspace", "KEY_BACKSPACE", 2.0),
    ],
    &[
        w("Tab", "KEY_TAB", 1.5), k("Q", "KEY_Q"), k("W", "KEY_W"), k("E", "KEY_E"), k("R", "KEY_R"),
        k("T", "KEY_T"), k("Y", "KEY_Y"), k("U", "KEY_U"), k("I", "KEY_I"), k("O", "KEY_O"),
        k("P", "KEY_P"), k("[", "KEY_LEFTBRACE"), k("]", "KEY_RIGHTBRACE"), w("\\", "KEY_BACKSLASH", 1.5),
    ],
    &[
        w("Caps", "KEY_CAPSLOCK", 1.75), k("A", "KEY_A"), k("S", "KEY_S"), k("D", "KEY_D"),
        k("F", "KEY_F"), k("G", "KEY_G"), k("H", "KEY_H"), k("J", "KEY_J"), k("K", "KEY_K"),
        k("L", "KEY_L"), k(";", "KEY_SEMICOLON"), k("'", "KEY_APOSTROPHE"), w("Enter", "KEY_ENTER", 2.25),
    ],
    &[
        w("Shift", "KEY_LEFTSHIFT", 2.25), k("Z", "KEY_Z"), k("X", "KEY_X"), k("C", "KEY_C"),
        k("V", "KEY_V"), k("B", "KEY_B"), k("N", "KEY_N"), k("M", "KEY_M"), k(",", "KEY_COMMA"),
        k(".", "KEY_DOT"), k("/", "KEY_SLASH"), w("Shift", "KEY_RIGHTSHIFT", 2.75),
    ],
    &[
        w("Esc", "KEY_ESC", 1.25), w("Ctrl", "KEY_LEFTCTRL", 1.25), w("Alt", "KEY_LEFTALT", 1.25),
        w("Space", "KEY_SPACE", 5.0), w("AltGr", "KEY_RIGHTALT", 1.25),
        w("←", "KEY_LEFT", 1.25), w("↑", "KEY_UP", 1.25), w("↓", "KEY_DOWN", 1.25), w("→", "KEY_RIGHT", 1.25),
    ],
];

/// A whole keyboard: the main block with the function keys, and the editing keys and arrows
/// beside it.
const FULL: [&[Key]; 6] = [
    &[
        k("Esc", "KEY_ESC"), gap(1.0),
        k("F1", "KEY_F1"), k("F2", "KEY_F2"), k("F3", "KEY_F3"), k("F4", "KEY_F4"), gap(0.5),
        k("F5", "KEY_F5"), k("F6", "KEY_F6"), k("F7", "KEY_F7"), k("F8", "KEY_F8"), gap(0.5),
        k("F9", "KEY_F9"), k("F10", "KEY_F10"), k("F11", "KEY_F11"), k("F12", "KEY_F12"), gap(0.5),
        w("PrtSc", "KEY_SYSRQ", 1.2), w("ScrLk", "KEY_SCROLLLOCK", 1.2), w("Pause", "KEY_PAUSE", 1.2),
    ],
    &[
        k("`", "KEY_GRAVE"), k("1", "KEY_1"), k("2", "KEY_2"), k("3", "KEY_3"), k("4", "KEY_4"),
        k("5", "KEY_5"), k("6", "KEY_6"), k("7", "KEY_7"), k("8", "KEY_8"), k("9", "KEY_9"),
        k("0", "KEY_0"), k("-", "KEY_MINUS"), k("=", "KEY_EQUAL"), w("Backspace", "KEY_BACKSPACE", 2.0), gap(0.5),
        w("Ins", "KEY_INSERT", 1.2), w("Home", "KEY_HOME", 1.2), w("PgUp", "KEY_PAGEUP", 1.2),
    ],
    &[
        w("Tab", "KEY_TAB", 1.5), k("Q", "KEY_Q"), k("W", "KEY_W"), k("E", "KEY_E"), k("R", "KEY_R"),
        k("T", "KEY_T"), k("Y", "KEY_Y"), k("U", "KEY_U"), k("I", "KEY_I"), k("O", "KEY_O"),
        k("P", "KEY_P"), k("[", "KEY_LEFTBRACE"), k("]", "KEY_RIGHTBRACE"), w("\\", "KEY_BACKSLASH", 1.5), gap(0.5),
        w("Del", "KEY_DELETE", 1.2), w("End", "KEY_END", 1.2), w("PgDn", "KEY_PAGEDOWN", 1.2),
    ],
    &[
        w("Caps", "KEY_CAPSLOCK", 1.75), k("A", "KEY_A"), k("S", "KEY_S"), k("D", "KEY_D"),
        k("F", "KEY_F"), k("G", "KEY_G"), k("H", "KEY_H"), k("J", "KEY_J"), k("K", "KEY_K"),
        k("L", "KEY_L"), k(";", "KEY_SEMICOLON"), k("'", "KEY_APOSTROPHE"), w("Enter", "KEY_ENTER", 2.25), gap(4.1),
    ],
    &[
        w("Shift", "KEY_LEFTSHIFT", 2.25), k("Z", "KEY_Z"), k("X", "KEY_X"), k("C", "KEY_C"),
        k("V", "KEY_V"), k("B", "KEY_B"), k("N", "KEY_N"), k("M", "KEY_M"), k(",", "KEY_COMMA"),
        k(".", "KEY_DOT"), k("/", "KEY_SLASH"), w("Shift", "KEY_RIGHTSHIFT", 2.75), gap(1.7),
        w("↑", "KEY_UP", 1.2), gap(1.2),
    ],
    &[
        w("Ctrl", "KEY_LEFTCTRL", 1.25), w("Super", "KEY_LEFTMETA", 1.25), w("Alt", "KEY_LEFTALT", 1.25),
        w("Space", "KEY_SPACE", 6.25), w("AltGr", "KEY_RIGHTALT", 1.25), w("Super", "KEY_RIGHTMETA", 1.25),
        w("Menu", "KEY_COMPOSE", 1.25), w("Ctrl", "KEY_RIGHTCTRL", 1.25), gap(0.5),
        w("←", "KEY_LEFT", 1.2), w("↓", "KEY_DOWN", 1.2), w("→", "KEY_RIGHT", 1.2),
    ],
];

/// The on-screen numpad. Digits are the top-row keys, which type numbers whatever the Num Lock
/// state (keypad keys turn into arrows and Home/End with it off); the minus is the keypad's,
/// which is a minus in every layout.
pub(super) const PAD: [&[Key]; 4] = [
    &[k("7", "KEY_7"), k("8", "KEY_8"), k("9", "KEY_9"), k("⌫", "KEY_BACKSPACE")],
    &[k("4", "KEY_4"), k("5", "KEY_5"), k("6", "KEY_6"), k("−", "KEY_KPMINUS")],
    &[k("1", "KEY_1"), k("2", "KEY_2"), k("3", "KEY_3"), k(",", "KEY_COMMA")],
    &[w("0", "KEY_0", 2.0), k(".", "KEY_DOT"), k("Enter", "KEY_ENTER")],
];

/// Which set of keys the controller moves across.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Board {
    #[default]
    Basic,
    Full,
    Numpad,
}

impl From<Layout> for Board {
    /// The keyboard's basic board, or the numpad.
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Keyboard => Board::Basic,
            Layout::Numpad => Board::Numpad,
        }
    }
}

impl Board {
    pub fn rows(self) -> &'static [&'static [Key]] {
        match self {
            Board::Basic => &BASIC,
            Board::Full => &FULL,
            Board::Numpad => &PAD,
        }
    }

    /// The overlay this board is drawn as: the keyboard (either size) or the numpad.
    pub fn layout(self) -> Layout {
        match self {
            Board::Basic | Board::Full => Layout::Keyboard,
            Board::Numpad => Layout::Numpad,
        }
    }

    /// Where the cursor starts the first time.
    pub fn home(self) -> Cursor {
        let code = match self {
            Board::Basic | Board::Full => "KEY_Q",
            Board::Numpad => "KEY_5",
        };
        find(self, code).unwrap_or_default()
    }

    /// How wide its widest row is, in key units.
    pub fn units_wide(self) -> f32 {
        self.rows().iter().map(|row| row.iter().map(|k| k.width).sum::<f32>()).fold(0.0, f32::max)
    }

    /// Every key on it, spacers left out.
    pub fn keys(self) -> impl Iterator<Item = &'static Key> {
        self.rows().iter().flat_map(|r| r.iter()).filter(|k| !k.code.is_empty())
    }
}

/// A key position in a board for controller navigation (gaps skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

/// A board without spacers: each key with its horizontal center in key units.
pub fn grid(board: impl Into<Board>) -> Vec<Vec<(&'static Key, f32)>> {
    board
        .into()
        .rows()
        .iter()
        .map(|row| {
            let mut x = 0.0;
            let mut keys = Vec::new();
            for key in row.iter() {
                if !key.code.is_empty() {
                    keys.push((key, x + key.width / 2.0));
                }
                x += key.width;
            }
            keys
        })
        .collect()
}

/// The key under the cursor (clamped into the grid).
pub fn key_at(board: impl Into<Board>, cursor: Cursor) -> &'static Key {
    let grid = grid(board);
    let row = &grid[cursor.row.min(grid.len() - 1)];
    row[cursor.col.min(row.len() - 1)].0
}

/// Moves the cursor one step: left/right along the row (wrapping), up/down to the key in
/// the next row whose center lines up best (also wrapping).
pub fn step(board: impl Into<Board>, cursor: Cursor, dx: i32, dy: i32) -> Cursor {
    let grid = grid(board);
    let row = cursor.row.min(grid.len() - 1);
    let col = cursor.col.min(grid[row].len() - 1);
    if dy == 0 {
        let len = grid[row].len() as i32;
        return Cursor { row, col: (col as i32 + dx).rem_euclid(len) as usize };
    }
    let x = grid[row][col].1;
    let new_row = (row as i32 + dy).rem_euclid(grid.len() as i32) as usize;
    let new_col = grid[new_row]
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1.1 - x).abs().total_cmp(&(b.1.1 - x).abs()))
        .map(|(i, _)| i)
        .unwrap_or(0);
    Cursor { row: new_row, col: new_col }
}

/// Where a key code sits in the grid.
pub fn find(board: impl Into<Board>, code: &str) -> Option<Cursor> {
    grid(board).iter().enumerate().find_map(|(row, keys)| keys.iter().position(|(k, _)| k.code == code).map(|col| Cursor { row, col }))
}
