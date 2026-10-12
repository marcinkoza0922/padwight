//! On-screen overlay: a keyboard you type on with the controller, and action menus. The
//! daemon owns the state (`OverlayController` here, `menu::MenuSession`) and routes
//! controller input to it while something is shown; the overlay process
//! (`padwight overlay`) only draws it, on a Wayland layer-shell surface that never
//! takes keyboard focus, so typed keys reach the window underneath. The process stays
//! running as an invisible 1×1 surface between uses so menus appear instantly.

use std::{
    collections::HashSet,
    str::FromStr,
    time::{Duration, Instant},
};

use crate::keycode::KeyCode;
use serde::{Deserialize, Serialize};

use crate::{
    config::Button,
    input::{Axis, InputEvent},
    keyboard::{self, Board, Cursor, Layout},
};

/// Holding East this long closes the overlay.
pub const HOLD_TO_CLOSE: Duration = Duration::from_millis(700);
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

/// The keyboard's reminder of its controls, with `{tokens}` drawn as the pad's glyphs.
pub const KEYBOARD_LEGEND: &[&str] = &[
    "{dpad} move",
    "{confirm} press",
    "{keyboard_backspace} backspace (hold to repeat)",
    "{keyboard_space} space",
    "{keyboard_enter} enter",
    "{keyboard_left} {keyboard_right} move the text cursor",
    "hold {lt} shift",
    "Shift, Ctrl, Alt, AltGr and Super stay on for one key; press one again to let go",
    "hold {keyboard_close} or press {guide} to close",
];

/// The numpad's reminder of its controls.
pub const NUMPAD_LEGEND: &[&str] =
    &["{dpad} move", "{confirm} press", "{backspace} backspace (hold to repeat)", "{enter} enter", "hold {numpad_close} or press {guide} to close"];

/// What the overlay process draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverlayView {
    Keyboard(KeyboardView),
    Menu(crate::menu::MenuView),
    Media(crate::media::MediaView),
    Offer(crate::offer::OfferView),
}

/// Everything the overlay window shows at once: info overlays, plus the keyboard or a menu.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OverlayFrame {
    pub info: Vec<crate::info::InfoView>,
    /// Absent from an older daemon's frames.
    #[serde(default)]
    pub logs: Vec<crate::inputlog::LogView>,
    pub active: Option<OverlayView>,
    /// The font everything here is drawn in; the system's when unset.
    #[serde(default)]
    pub font: Option<String>,
    /// Whether menu rows are tinted in colors color-blind people can tell apart. An older daemon
    /// spells it `colourblind`.
    #[serde(default, alias = "colourblind")]
    pub colorblind: bool,
    /// The controller in use, whose button glyphs menus draw as the info overlays do.
    #[serde(default)]
    pub family: crate::info::PadFamily,
    /// Whether menus draw their button glyphs in the Nintendo layout.
    #[serde(default)]
    pub nintendo_layout: bool,
    /// What `{guide}` says: whatever opens the Guide overlay.
    #[serde(default)]
    pub opener: crate::info::OpenerLabel,
    /// What `{confirm}` and `{back}` say: the buttons that confirm and go back.
    #[serde(default)]
    pub menu_keys: crate::config::MenuKeys,
    /// Which buttons the media controls' and numpad's keys sit on, for their hints.
    #[serde(default)]
    pub panel_keys: crate::config::Shortcuts,
    /// How each kind of overlay moves as it opens, closes, and as the cursor and picks move.
    #[serde(default)]
    pub motion: crate::motion::MotionSet,
    /// What the performance settings cut from every overlay.
    #[serde(default, skip_serializing_if = "crate::config::Lite::is_off")]
    pub lite: crate::config::Lite,
}

impl OverlayFrame {
    /// Whether this frame can stand in for `older`, sent just before it, without the overlay
    /// missing anything it animates: the same panel, or none, with the same pick. A panel opening
    /// or closing, or a pick, must be seen even when the next frame follows at once, as the
    /// close does after a pick that closes a menu.
    pub fn stands_in_for(&self, older: &OverlayFrame) -> bool {
        use crate::motion::Tracked;
        let scene = |frame: &OverlayFrame| frame.active.as_ref().map(|view| (view.key(), view.pick()));
        scene(self) == scene(older)
    }

    /// Hands the frame's performance settings to every style in it, where the drawing reads them.
    fn apply_lite(&mut self) {
        let lite = self.lite;
        let styles = self.info.iter_mut().map(|v| &mut v.style).chain(self.logs.iter_mut().map(|v| &mut v.style));
        let active = self.active.as_mut().map(|view| match view {
            OverlayView::Keyboard(k) => &mut k.style,
            OverlayView::Menu(m) => &mut m.style,
            OverlayView::Media(m) => &mut m.style,
            OverlayView::Offer(o) => &mut o.style,
        });
        for style in styles.chain(active) {
            style.lite = lite;
        }
    }
}

/// The on-screen keyboard's (or numpad's) state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyboardView {
    /// Which keys are drawn: the basic or full keyboard, or the numpad.
    #[serde(default)]
    pub board: Board,
    /// Set by the daemon from the config.
    #[serde(default = "crate::config::OverlayStyle::keyboard")]
    pub style: crate::config::OverlayStyle,
    pub cursor: Cursor,
    /// Modifiers latched for the next key (evdev names).
    pub latched: Vec<String>,
    /// A key currently held down with A.
    pub pressed: Option<String>,
    /// 0..1 while East is being held to close.
    pub closing: f32,
    /// Caps Lock is on, as far as the keyboard knows (it counts its own Caps Lock presses).
    #[serde(default)]
    pub caps: bool,
    /// What's been typed lately, as far as the keyboard can tell, for the line above its keys.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub typed: String,
}

/// Where a key sits in its layout, counting every slot in reading order: the number the motion
/// tracking and the drawing both use for the cursor and the pressed key.
fn key_index(board: Board, code: &str) -> Option<u32> {
    board.rows().iter().flat_map(|keys| keys.iter()).position(|key| key.code == code).map(|i| i as u32)
}

impl crate::motion::Tracked for OverlayView {
    fn key(&self) -> String {
        match self {
            OverlayView::Keyboard(k) => if k.board == Board::Numpad { "numpad" } else { "keyboard" }.into(),
            OverlayView::Menu(m) => format!("{}\u{0}{}", m.crumbs.join(" › "), m.title),
            OverlayView::Media(_) => "media".into(),
            OverlayView::Offer(_) => "offer".into(),
        }
    }

    fn kind(&self) -> crate::motion::OverlayKind {
        use crate::motion::OverlayKind;
        match self {
            OverlayView::Keyboard(k) if k.board == Board::Numpad => OverlayKind::Numpad,
            OverlayView::Keyboard(_) => OverlayKind::Keyboard,
            OverlayView::Menu(_) => OverlayKind::Menu,
            OverlayView::Media(_) => OverlayKind::Media,
            OverlayView::Offer(_) => OverlayKind::Offer,
        }
    }

    fn cursor(&self) -> Option<u32> {
        match self {
            OverlayView::Keyboard(k) => key_index(k.board, keyboard::key_at(k.board, k.cursor).code),
            OverlayView::Menu(m) => m.selected.map(|i| i as u32),
            OverlayView::Offer(_) | OverlayView::Media(_) => None,
        }
    }

    fn pick(&self) -> Option<u32> {
        match self {
            OverlayView::Keyboard(k) => k.pressed.as_deref().and_then(|code| key_index(k.board, code)),
            OverlayView::Menu(m) => (m.picks > 0).then_some(m.picks),
            OverlayView::Offer(_) | OverlayView::Media(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverlayAction {
    Key(KeyCode, bool),
    Close,
}

/// Turns controller input into keyboard navigation and key presses.
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
    /// The opener is down: with it, the keyboard's own shortcut closes the keyboard (the numpad's
    /// the numpad), as it opened it.
    guide_held: bool,
    /// The button that is held to open (Guide unless moved; none when a gesture or combo does).
    guide_button: Option<Button>,
    /// The button of this layout's Guide shortcut, which closes it with the opener held.
    close_button: Option<Button>,
    /// Shift held down by the left trigger.
    trigger_shift: bool,
    /// Caps Lock is on, counting the keyboard's own presses of it.
    caps: bool,
    /// X is held: when backspace next repeats.
    backspace: Option<Instant>,
    /// The key held with A needed Shift to type its digit (a numpad digit in AZERTY).
    pressed_shift: bool,
    /// What's been typed lately (see [`KeyboardView::typed`]).
    typed: String,
}

fn code(name: &str) -> Option<KeyCode> {
    KeyCode::from_str(name).ok()
}

fn tap(name: &str) -> Vec<OverlayAction> {
    code(name).map(|k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)]).unwrap_or_default()
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
            guide_held: false,
            guide_button: Some(Button::Guide),
            close_button,
            trigger_shift: false,
            caps: false,
            backspace: None,
            pressed_shift: false,
            typed: String::new(),
        }
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
            latched: self.latched.iter().chain(self.trigger_shift.then_some(&KeyCode::KEY_LEFTSHIFT)).map(|k| name(*k)).collect(),
            pressed: self.pressed.map(name),
            closing: self
                .east_since
                .map(|t| (now.duration_since(t).as_secs_f32() / HOLD_TO_CLOSE.as_secs_f32()).min(1.0))
                .unwrap_or(0.0),
            caps: self.caps,
            typed: self.typed.clone(),
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
            InputEvent::Axis(Axis::LeftTrigger, v) if self.layout() == Layout::Keyboard => self.left_trigger(v),
            InputEvent::Axis(..) | InputEvent::Touchpad(_) => Vec::new(),
        }
    }

    /// Holding the left trigger holds Shift (alongside any latched modifiers).
    fn left_trigger(&mut self, value: f32) -> Vec<OverlayAction> {
        let shift = KeyCode::KEY_LEFTSHIFT;
        let latched = self.latched.contains(&shift);
        if !self.trigger_shift && value >= SHIFT_PRESS {
            self.trigger_shift = true;
            // Already down if latched; still mark it so letting go of the trigger is quiet.
            if !latched {
                return vec![OverlayAction::Key(shift, true)];
            }
        } else if self.trigger_shift && value < SHIFT_RELEASE {
            self.trigger_shift = false;
            if !latched {
                return vec![OverlayAction::Key(shift, false)];
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
            (Button::LeftBumper, true) if keyboard => tap("KEY_LEFT"),
            (Button::RightBumper, true) if keyboard => tap("KEY_RIGHT"),
            (Button::Start, true) => self.shortcut("KEY_ENTER"),
            (Button::East, true) => {
                self.east_since = Some(now);
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
        let Some(k) = code(name) else { return Vec::new() };
        if k == KeyCode::KEY_CAPSLOCK {
            self.caps = !self.caps;
        }
        // Modifiers latch (held down) until the next key, like sticky keys. Shift held by the
        // trigger is already down, and stays down when its latch comes off.
        if MODIFIERS.contains(&name) {
            let held = self.trigger_shift && k == KeyCode::KEY_LEFTSHIFT;
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
        shift.into_iter().chain([OverlayAction::Key(k, true)]).collect()
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
        let shortcut = self.latched.iter().any(|k| {
            matches!(*k, KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL | KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT | KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA)
        });
        match code {
            _ if shortcut => {}
            "KEY_BACKSPACE" => drop(self.typed.pop()),
            "KEY_ENTER" | "KEY_KPENTER" | "KEY_ESC" => self.typed.clear(),
            "KEY_SPACE" => self.typed.push(' '),
            "KEY_KPMINUS" => self.typed.push('-'),
            _ if self.board == Board::Numpad => self.typed.push_str(keyboard::key_at(self.board, self.cursor).label),
            _ => self.typed.push_str(&keyboard::typed(code, self.shift_down(), self.caps).unwrap_or_default()),
        }
        let extra = self.typed.chars().count().saturating_sub(KEEP);
        if extra > 0 {
            self.typed = self.typed.chars().skip(extra).collect();
        }
    }

    fn shortcut(&mut self, name: &str) -> Vec<OverlayAction> {
        self.note_typed(name);
        let mut out = tap(name);
        out.extend(self.release_latched());
        out
    }

    fn release_latched(&mut self) -> Vec<OverlayAction> {
        // Shift stays down while the trigger holds it.
        let held = self.trigger_shift.then_some(KeyCode::KEY_LEFTSHIFT);
        self.latched.drain(..).rev().filter(|k| Some(*k) != held).map(|k| OverlayAction::Key(k, false)).collect()
    }

    /// Key repeat for held directions and hold-to-close. Returns what happened and whether
    /// the view changed.
    pub fn tick(&mut self, now: Instant) -> (Vec<OverlayAction>, bool) {
        let mut changed = self.east_since.is_some();
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
            return (tap("KEY_BACKSPACE"), changed);
        }
        (Vec::new(), changed)
    }

    /// When `tick` next has work (key repeat or the close countdown).
    pub fn next_deadline(&self) -> Option<Instant> {
        let close = self.east_since.map(|t| t + HOLD_TO_CLOSE);
        let repeat = self.repeat.deadline().into_iter().chain(self.backspace).min();
        // While East is held the closing bar animates, so keep ticking.
        let animate = self.east_since.map(|_| Instant::now() + Duration::from_millis(30));
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
        self.repeat.stop();
        self.backspace = None;
        self.east_since = None;
        self.dpad.clear();
        self.stick_dir.clear();
        out
    }
}


/// Runs the overlay window: a layer-shell surface at the bottom of the screen drawing the
/// state the daemon streams, and exiting when the daemon hides it.
#[cfg(target_os = "linux")]
pub fn run() -> anyhow::Result<()> {
    // iced_layershell unwraps when the compositor has no layer-shell (GNOME): report that
    // as an error instead of a panic and backtrace.
    std::panic::set_hook(Box::new(|_| {}));
    std::panic::catch_unwind(layershell::run).unwrap_or_else(|panic| {
        let why = panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(ToString::to_string));
        anyhow::bail!(
            "the overlay window could not start (the compositor probably lacks wlr-layer-shell, as GNOME does): {}",
            why.unwrap_or_default()
        )
    })
}

/// Runs the overlay window: a click-through layered window over the screen drawing the state
/// the daemon streams, and exiting when the daemon goes away.
#[cfg(windows)]
pub fn run() -> anyhow::Result<()> {
    windows::run()
}

#[cfg(target_os = "linux")]
mod layershell;
mod ui;
#[cfg(windows)]
mod windows;

pub use ui::snapshot;

/// Drawing for overlays, generic over the message type so the settings GUI can show the
/// same thing as a live preview.
pub mod draw;
#[cfg(test)]
mod fade_frames;
pub mod fit;
mod timing;

#[cfg(test)]
mod tests;
