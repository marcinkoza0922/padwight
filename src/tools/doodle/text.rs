//! Text boxes on the Doodle screen: the keyboard that types into one, and the grab that moves one.
//! South on empty space opens a new box; South on a box grabs it, and lifting South without moving
//! opens it for editing instead. Enter sets the text, and closing the keyboard keeps what was there.

use std::time::Instant;

use super::{Canvas, Point, TextBox, TextStyle};
use crate::{
    keycode::KeyCode,
    overlay::{KeyboardView, OverlayAction, OverlayController},
};

/// The styles the text tool's checklist lists (North opens it), in its order. Each is a face a
/// text box can take.
pub const FACES: [&str; 3] = ["Italic", "Bold", "Underline"];

/// Whether face `face` (an index into [`FACES`]) is set in `style`.
pub fn face_on(style: &TextStyle, face: usize) -> bool {
    match face {
        0 => style.italic,
        1 => style.bold,
        _ => style.underline,
    }
}

/// Sets face `face` in `style` the other way.
pub fn toggle_face(style: &mut TextStyle, face: usize) {
    match face {
        0 => style.italic = !style.italic,
        1 => style.bold = !style.bold,
        _ => style.underline = !style.underline,
    }
}

/// The keyboard typing into a text box, which is an existing one when `index` is set.
#[derive(Debug, Clone)]
pub struct Entry {
    index: Option<usize>,
    at: Point,
    keys: OverlayController,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        (self.index, self.at, self.keys.typed()) == (other.index, other.at, other.keys.typed())
    }
}

impl Entry {
    /// A keyboard for a new box at `at`, which starts empty.
    pub fn new(at: Point) -> Entry {
        Entry { index: None, at, keys: OverlayController::for_field("Text", "") }
    }

    /// A keyboard editing the box at `index`, which holds `text` to start with.
    pub fn editing(index: usize, at: Point, text: &str) -> Entry {
        Entry { index: Some(index), at, keys: OverlayController::for_field("Text", text) }
    }

    /// The keyboard, for input and repeats.
    pub fn keys(&mut self) -> &mut OverlayController {
        &mut self.keys
    }

    /// The keyboard as the overlay draws it.
    pub fn view(&self, now: Instant) -> KeyboardView {
        self.keys.view(now)
    }

    /// Sets the text: a new box is added when there's text, and a box emptied is removed.
    pub fn commit(self, canvas: &mut Canvas, style: TextStyle) {
        let words = self.keys.typed().to_string();
        match (self.index, words.is_empty()) {
            (Some(index), true) => canvas.remove(index),
            (Some(index), false) => {
                if let Some(text) = canvas.text_mut(index) {
                    text.text = words;
                }
            }
            (None, false) => {
                canvas.add_text(TextBox { at: self.at, text: words, style });
            }
            (None, true) => {}
        }
    }
}

/// What the keyboard did: whether it asks to be done, or closed. `None` when it's just typing.
pub fn finished(actions: &[OverlayAction]) -> Option<bool> {
    actions.iter().find_map(|action| match action {
        OverlayAction::Key(KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER, true) => Some(true),
        OverlayAction::Close => Some(false),
        _ => None,
    })
}

/// A text box held by South, and where the cursor last was. It moves with the cursor, and counts
/// as a press to edit only if it never moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grab {
    pub index: usize,
    pub last: Point,
    pub moved: bool,
}
