//! The on-screen keyboard's controls: moving across the keys, pressing them (with latched
//! modifiers and Shift on the left trigger), and what it can tell has been typed.

use std::{
    collections::HashSet,
    str::FromStr,
    time::{Duration, Instant},
};

use crate::{
    config::Button,
    input::{Axis, InputEvent},
    keyboard::{self, Board, Cursor, Layout},
    keycode::KeyCode,
};

use super::{HOLD_TO_CLOSE, KeyboardView};

/// How often the closing bar is redrawn while East is held.
const CLOSING_FRAME: Duration = Duration::from_millis(30);

/// Left trigger pull that holds Shift on the keyboard, and where it lets go.
const SHIFT_PRESS: f32 = 0.5;
const SHIFT_RELEASE: f32 = 0.3;

const MODIFIERS: [&str; 8] = [
    "KEY_LEFTSHIFT",
    "KEY_RIGHTSHIFT",
    "KEY_LEFTCTRL",
    "KEY_RIGHTCTRL",
    "KEY_LEFTALT",
    "KEY_RIGHTALT",
    "KEY_LEFTMETA",
    "KEY_RIGHTMETA",
];

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayAction {
    Key(KeyCode, bool),
    Close,
}

/// Turns controller input into keyboard navigation and key presses.
#[derive(Debug, Clone)]
pub struct OverlayController {
    board: Board,
    cursor: Cursor,
    latched: Vec<KeyCode>,
    pressed: Option<KeyCode>,
    dpad: HashSet<Button>,
    stick: (f32, f32),
    stick_dir: crate::nav::StickDir,
    /// The direction being held, repeating.
    repeat: crate::nav::Repeat,
    east_since: Option<Instant>,
    /// When the closing bar next needs a frame while East is held, kept between ticks so the
    /// deadline comes due instead of sliding away each time it is asked for.
    next_frame: Instant,
    /// The opener is down: with it, the keyboard's own shortcut closes the keyboard (the numpad's
    /// the numpad), as it opened it.
    guide_held: bool,
    /// The button that is held to open (Guide unless moved; none when a gesture or combo does).
    guide_button: Option<Button>,
    /// The button of this layout's Guide shortcut, which closes it with the opener held.
    close_button: Option<Button>,
    /// Shift held down by the left trigger.
    trigger_shift: bool,
    /// The right trigger is holding Left Ctrl.
    trigger_ctrl: bool,
    /// Caps Lock is on, counting the keyboard's own presses of it.
    caps: bool,
    /// X is held: when backspace next repeats.
    backspace: Option<Instant>,
    /// The key held with A needed Shift to type its digit (a numpad digit in AZERTY).
    pressed_shift: bool,
    /// What's been typed lately (see [`KeyboardView::typed`]).
    typed: String,
    /// Where the text cursor sits in `typed`, counted in characters.
    caret: usize,
    /// Typing into a field, named here (see [`KeyboardView::field`]): the text is kept whole, and
    /// Enter and Escape don't clear it.
    field: Option<String>,
}

/// The most a field keeps of what's typed into it.
const FIELD_KEEP: usize = 1000;

fn code(name: &str) -> Option<KeyCode> {
    KeyCode::from_str(name).ok()
}

fn tap(name: &str) -> Vec<OverlayAction> {
    code(name).map(|k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)]).unwrap_or_default()
}

/// The presses that type `text` into the game: each character on the key that types it in the
/// user's layout, with Shift held where that takes it. Characters no key types are left out.
pub fn typing(text: &str) -> Vec<OverlayAction> {
    let mut out = Vec::new();
    for (name, shift) in text.chars().filter_map(keyboard::key_for) {
        let shift_key = shift.then_some(KeyCode::KEY_LEFTSHIFT);
        out.extend(shift_key.map(|k| OverlayAction::Key(k, true)));
        out.extend(tap(name));
        out.extend(shift_key.map(|k| OverlayAction::Key(k, false)));
    }
    out
}

impl OverlayController {
    pub fn new(board: impl Into<Board>, cursor: Cursor) -> Self {
        let board = board.into();
        let close_button = Some(if board.layout() == Layout::Numpad { Button::North } else { Button::West });
        OverlayController {
            board,
            cursor,
            latched: Vec::new(),
            pressed: None,
            dpad: HashSet::new(),
            stick: (0.0, 0.0),
            stick_dir: crate::nav::StickDir::default(),
            repeat: crate::nav::Repeat::default(),
            east_since: None,
            next_frame: Instant::now(),
            guide_held: false,
            guide_button: Some(Button::Guide),
            close_button,
            trigger_shift: false,
            trigger_ctrl: false,
            caps: false,
            backspace: None,
            pressed_shift: false,
            typed: String::new(),
            caret: 0,
            field: None,
        }
    }

    /// A keyboard typing into the field called `title`, which holds `text` to start with.
    pub fn for_field(title: &str, text: &str) -> Self {
        let mut keyboard = OverlayController::new(Board::Basic, Board::Basic.home());
        keyboard.guide_button = None;
        keyboard.close_button = None;
        keyboard.typed = text.to_string();
        keyboard.caret = text.chars().count();
        keyboard.field = Some(title.to_string());
        keyboard
    }

    /// The text typed so far.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn layout(&self) -> Layout {
        self.board.layout()
    }

    pub fn board(&self) -> Board {
        self.board
    }

    pub fn view(&self, now: Instant) -> KeyboardView {
        let name = |k: KeyCode| format!("{k:?}");
        KeyboardView {
            board: self.board,
            style: crate::config::OverlayStyle::keyboard(),
            cursor: self.cursor,
            latched: self
                .latched
                .iter()
                .chain(self.trigger_shift.then_some(&KeyCode::KEY_LEFTSHIFT))
                .chain(self.trigger_ctrl.then_some(&KeyCode::KEY_LEFTCTRL))
                .map(|k| name(*k))
                .collect(),
            pressed: self.pressed.map(name),
            closing: self
                .east_since
                .map(|t| (now.duration_since(t).as_secs_f32() / HOLD_TO_CLOSE.as_secs_f32()).min(1.0))
                .unwrap_or(0.0),
            caps: self.caps,
            typed: self.typed.clone(),
            caret: self.caret,
            field: self.field.clone(),
        }
    }

    /// Tells the controller which button is held to open the Guide overlay and which one closes
    /// this layout with it (the shortcut it was opened by), and whether the opener is already down
    /// as it opens (the chord that opened it).
    pub fn set_guide(&mut self, held: bool, guide_button: Option<Button>, close_button: Option<Button>) {
        self.guide_held = held;
        self.guide_button = guide_button;
        self.close_button = close_button;
    }

    pub fn handle(&mut self, ev: InputEvent, now: Instant) -> Vec<OverlayAction> {
        match ev {
            InputEvent::Button(b, pressed) => self.button(b, pressed, now),
            InputEvent::Axis(Axis::LeftX, v) => {
                self.stick.0 = v;
                self.stick_moved(now);
                Vec::new()
            }
            InputEvent::Axis(Axis::LeftY, v) => {
                self.stick.1 = v;
                self.stick_moved(now);
                Vec::new()
            }
            InputEvent::Axis(Axis::LeftTrigger, v) if self.layout() == Layout::Keyboard => self.trigger(KeyCode::KEY_LEFTSHIFT, v),
            InputEvent::Axis(Axis::RightTrigger, v) if self.layout() == Layout::Keyboard => self.trigger(KeyCode::KEY_LEFTCTRL, v),
            InputEvent::Axis(..) | InputEvent::Touchpad(_) => Vec::new(),
        }
    }

    /// Holding the left trigger holds Shift, the right one Left Ctrl (alongside any latched modifiers).
    fn trigger(&mut self, key: KeyCode, value: f32) -> Vec<OverlayAction> {
        let latched = self.latched.contains(&key);
        let on = if key == KeyCode::KEY_LEFTSHIFT { &mut self.trigger_shift } else { &mut self.trigger_ctrl };
        if !*on && value >= SHIFT_PRESS {
            *on = true;
            // Already down if latched; still mark it so letting go of the trigger is quiet.
            if !latched {
                return vec![OverlayAction::Key(key, true)];
            }
        } else if *on && value < SHIFT_RELEASE {
            *on = false;
            if !latched {
                return vec![OverlayAction::Key(key, false)];
            }
        }
        Vec::new()
    }

    fn button(&mut self, b: Button, pressed: bool, now: Instant) -> Vec<OverlayAction> {
        if let Some(dir) = crate::nav::dpad_dir(b) {
            if pressed && self.dpad.insert(b) {
                self.start_moving(dir, now);
            } else if !pressed && self.dpad.remove(&b) {
                self.repeat.release(dir);
            }
            return Vec::new();
        }
        let keyboard = self.layout() == Layout::Keyboard;
        if Some(b) == self.guide_button {
            self.guide_held = pressed;
            return Vec::new();
        }
        match (b, pressed) {
            (b, true) if Some(b) == self.close_button && self.guide_held => vec![OverlayAction::Close],
            (Button::South, true) => self.press_selected(),
            (Button::South, false) => self.release_selected(),
            // Held, X keeps deleting at the cursor's repeat rate.
            (Button::West, true) => {
                self.backspace = Some(now + crate::nav::REPEAT_DELAY);
                self.shortcut("KEY_BACKSPACE")
            }
            (Button::West, false) => {
                self.backspace = None;
                Vec::new()
            }
            (Button::North, true) if keyboard => self.shortcut("KEY_SPACE"),
            // The bumpers move the text cursor, as the D-pad moves the keyboard's.
            (Button::LeftBumper, true) if keyboard => self.move_caret("KEY_LEFT"),
            (Button::RightBumper, true) if keyboard => self.move_caret("KEY_RIGHT"),
            (Button::Start, true) => self.enter(),
            (Button::East, true) => {
                self.east_since = Some(now);
                self.next_frame = now + CLOSING_FRAME;
                Vec::new()
            }
            (Button::East, false) => {
                self.east_since = None;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// The stick acts like a D-pad: its strongest direction past the threshold.
    fn stick_moved(&mut self, now: Instant) {
        match self.stick_dir.update(self.stick) {
            Some(Some(d)) => self.start_moving(d, now),
            Some(None) => self.repeat.stop(),
            None => {}
        }
    }

    fn start_moving(&mut self, dir: (i32, i32), now: Instant) {
        self.cursor = keyboard::step(self.board, self.cursor, dir.0, dir.1);
        self.repeat.start(dir, now);
    }

    fn press_selected(&mut self) -> Vec<OverlayAction> {
        let name = keyboard::key_at(self.board, self.cursor).code;
        if name == keyboard::PASTE {
            return self.paste();
        }
        let Some(k) = code(name) else { return Vec::new() };
        if k == KeyCode::KEY_CAPSLOCK {
            self.caps = !self.caps;
        }
        // Modifiers latch (held down) until the next key, like sticky keys. Shift held by the
        // trigger is already down, and stays down when its latch comes off.
        if MODIFIERS.contains(&name) {
            let held = (self.trigger_shift && k == KeyCode::KEY_LEFTSHIFT) || (self.trigger_ctrl && k == KeyCode::KEY_LEFTCTRL);
            if let Some(i) = self.latched.iter().position(|l| *l == k) {
                self.latched.remove(i);
                return if held { Vec::new() } else { vec![OverlayAction::Key(k, false)] };
            }
            self.latched.push(k);
            return if held { Vec::new() } else { vec![OverlayAction::Key(k, true)] };
        }
        if self.pressed.is_some() {
            return Vec::new();
        }
        self.note_typed(name);
        self.pressed = Some(k);
        // The numpad types digits: with Shift where the layout puts them there.
        let label = keyboard::key_at(self.board, self.cursor).label;
        self.pressed_shift = self.board == Board::Numpad && !self.shift_down() && keyboard::needs_shift(name, label) == Some(true);
        let shift = self.pressed_shift.then_some(OverlayAction::Key(KeyCode::KEY_LEFTSHIFT, true));
        let mut out: Vec<OverlayAction> = shift.into_iter().chain([OverlayAction::Key(k, true)]).collect();
        if name == "KEY_ENTER" && self.layout() == Layout::Keyboard {
            out.push(OverlayAction::Close);
        }
        out
    }

    /// Start's Enter: sends it, and closes the keyboard the way the Enter key does.
    fn enter(&mut self) -> Vec<OverlayAction> {
        let mut out = self.shortcut("KEY_ENTER");
        if self.layout() == Layout::Keyboard {
            out.push(OverlayAction::Close);
        }
        out
    }

    /// Ctrl+V into the game, which pastes what the clipboard holds. A field of the in-game menu
    /// takes its text from the keys it's typed with, so there it does nothing.
    fn paste(&mut self) -> Vec<OverlayAction> {
        if self.field.is_some() {
            return Vec::new();
        }
        let mut out = vec![
            OverlayAction::Key(KeyCode::KEY_LEFTCTRL, true),
            OverlayAction::Key(KeyCode::KEY_V, true),
            OverlayAction::Key(KeyCode::KEY_V, false),
            OverlayAction::Key(KeyCode::KEY_LEFTCTRL, false),
        ];
        out.extend(self.release_latched());
        out
    }

    fn release_selected(&mut self) -> Vec<OverlayAction> {
        let Some(k) = self.pressed.take() else { return Vec::new() };
        let mut out = vec![OverlayAction::Key(k, false)];
        if std::mem::take(&mut self.pressed_shift) {
            out.push(OverlayAction::Key(KeyCode::KEY_LEFTSHIFT, false));
        }
        out.extend(self.release_latched());
        out
    }

    /// Whether Shift is already down: latched, or held by the trigger.
    fn shift_down(&self) -> bool {
        self.trigger_shift || self.latched.iter().any(|k| matches!(*k, KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT))
    }

    /// Follows what key `code` types, for the line above the keys: a character in the user's
    /// layout, a space, or a backspace; Enter starts afresh. A shortcut (Ctrl, Alt, AltGr or
    /// Super latched) types nothing it can tell.
    fn note_typed(&mut self, code: &str) {
        const KEEP: usize = 60;
        let shortcut = self.trigger_ctrl
            || self.latched.iter().any(|k| {
            matches!(*k, KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL | KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT | KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA)
        });
        let mut chars: Vec<char> = self.typed.chars().collect();
        let at = self.caret.min(chars.len());
        let insert = |text: &str, chars: &mut Vec<char>, caret: &mut usize| {
            let new: Vec<char> = text.chars().collect();
            *caret = at + new.len();
            chars.splice(at..at, new);
        };
        let mut caret = at;
        match code {
            _ if shortcut => {}
            "KEY_LEFT" => caret = at.saturating_sub(1),
            "KEY_RIGHT" => caret = (at + 1).min(chars.len()),
            "KEY_HOME" => caret = 0,
            "KEY_END" => caret = chars.len(),
            "KEY_BACKSPACE" if at > 0 => {
                chars.remove(at - 1);
                caret = at - 1;
            }
            "KEY_DELETE" if at < chars.len() => drop(chars.remove(at)),
            // A field keeps its text: Enter is how it's done.
            "KEY_ENTER" | "KEY_KPENTER" | "KEY_ESC" if self.field.is_some() => {}
            "KEY_ENTER" | "KEY_KPENTER" | "KEY_ESC" => {
                chars.clear();
                caret = 0;
            }
            "KEY_SPACE" => insert(" ", &mut chars, &mut caret),
            "KEY_KPMINUS" => insert("-", &mut chars, &mut caret),
            _ if self.board == Board::Numpad => insert(keyboard::key_at(self.board, self.cursor).label, &mut chars, &mut caret),
            _ => insert(&keyboard::typed(code, self.shift_down(), self.caps).unwrap_or_default(), &mut chars, &mut caret),
        }
        let keep = if self.field.is_some() { FIELD_KEEP } else { KEEP };
        let extra = chars.len().saturating_sub(keep);
        self.typed = chars[extra..].iter().collect();
        self.caret = caret.saturating_sub(extra);
    }

    /// A bumper: moves the text cursor in the game, and the caret on the typed line with it.
    fn move_caret(&mut self, name: &str) -> Vec<OverlayAction> {
        self.note_typed(name);
        tap(name)
    }

    fn shortcut(&mut self, name: &str) -> Vec<OverlayAction> {
        self.note_typed(name);
        let mut out = tap(name);
        out.extend(self.release_latched());
        out
    }

    fn release_latched(&mut self) -> Vec<OverlayAction> {
        // Shift and Ctrl stay down while a trigger holds them.
        let held = [self.trigger_shift.then_some(KeyCode::KEY_LEFTSHIFT), self.trigger_ctrl.then_some(KeyCode::KEY_LEFTCTRL)];
        self.latched.drain(..).rev().filter(|k| !held.contains(&Some(*k))).map(|k| OverlayAction::Key(k, false)).collect()
    }

    /// Key repeat for held directions and hold-to-close. Returns what happened and whether
    /// the view changed.
    pub fn tick(&mut self, now: Instant) -> (Vec<OverlayAction>, bool) {
        let mut changed = false;
        if self.east_since.is_some() && now >= self.next_frame {
            changed = true;
            while self.next_frame <= now {
                self.next_frame += CLOSING_FRAME;
            }
        }
        if let Some(since) = self.east_since
            && now.duration_since(since) >= HOLD_TO_CLOSE
        {
            self.east_since = None;
            return (vec![OverlayAction::Close], true);
        }
        if let Some(dir) = self.repeat.due(now) {
            self.cursor = keyboard::step(self.board, self.cursor, dir.0, dir.1);
            changed = true;
        }
        if let Some(due) = self.backspace
            && now >= due
        {
            self.backspace = Some(now + crate::nav::REPEAT_EVERY);
            self.note_typed("KEY_BACKSPACE");
            return (tap("KEY_BACKSPACE"), true);
        }
        (Vec::new(), changed)
    }

    /// When `tick` next has work (key repeat or the close countdown).
    pub fn next_deadline(&self) -> Option<Instant> {
        let close = self.east_since.map(|t| t + HOLD_TO_CLOSE);
        let repeat = self.repeat.deadline().into_iter().chain(self.backspace).min();
        // While East is held the closing bar animates, so keep ticking.
        let animate = self.east_since.map(|_| self.next_frame);
        [close, repeat, animate].into_iter().flatten().min()
    }

    /// Lets go of everything the overlay is holding (when it closes).
    pub fn release_all(&mut self) -> Vec<OverlayAction> {
        let mut out: Vec<OverlayAction> = self.pressed.take().map(|k| OverlayAction::Key(k, false)).into_iter().collect();
        if std::mem::take(&mut self.pressed_shift) {
            out.push(OverlayAction::Key(KeyCode::KEY_LEFTSHIFT, false));
        }
        out.extend(self.release_latched());
        if std::mem::take(&mut self.trigger_shift) {
            out.push(OverlayAction::Key(KeyCode::KEY_LEFTSHIFT, false));
        }
        if std::mem::take(&mut self.trigger_ctrl) {
            out.push(OverlayAction::Key(KeyCode::KEY_LEFTCTRL, false));
        }
        self.repeat.stop();
        self.backspace = None;
        self.east_since = None;
        self.dpad.clear();
        self.stick_dir.clear();
        out
    }
}
