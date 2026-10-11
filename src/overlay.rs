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

use evdev::KeyCode;
use serde::{Deserialize, Serialize};

use crate::{
    config::Button,
    input::{Axis, InputEvent},
    keyboard::{self, Cursor, Layout},
};

/// Holding East this long closes the overlay.
pub const HOLD_TO_CLOSE: Duration = Duration::from_millis(700);
/// Holding a direction moves once, then repeats after this delay...
const REPEAT_DELAY: Duration = Duration::from_millis(350);
/// ...this often.
const REPEAT_EVERY: Duration = Duration::from_millis(90);
/// Left trigger pull that holds Shift on the keyboard, and where it lets go.
const SHIFT_PRESS: f32 = 0.5;
const SHIFT_RELEASE: f32 = 0.3;
/// Stick deflection that counts as a direction, and where it lets go.
const STICK_PRESS: f32 = 0.6;
const STICK_RELEASE: f32 = 0.45;

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
    /// How each kind of overlay moves as it opens, closes, and as the cursor and picks move.
    #[serde(default)]
    pub motion: crate::motion::MotionSet,
    /// What the performance settings cut from every overlay.
    #[serde(default, skip_serializing_if = "crate::config::Lite::is_off")]
    pub lite: crate::config::Lite,
}

impl OverlayFrame {
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
    #[serde(default)]
    pub layout: Layout,
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
}

/// Where a key sits in its layout, counting every slot in reading order: the number the motion
/// tracking and the drawing both use for the cursor and the pressed key.
fn key_index(layout: Layout, code: &str) -> Option<u32> {
    layout.rows().iter().flat_map(|keys| keys.iter()).position(|key| key.code == code).map(|i| i as u32)
}

impl crate::motion::Tracked for OverlayView {
    fn key(&self) -> String {
        match self {
            OverlayView::Keyboard(k) => if k.layout == Layout::Numpad { "numpad" } else { "keyboard" }.into(),
            OverlayView::Menu(m) => format!("{}\u{0}{}", m.crumbs.join(" › "), m.title),
            OverlayView::Media(_) => "media".into(),
            OverlayView::Offer(_) => "offer".into(),
        }
    }

    fn kind(&self) -> crate::motion::OverlayKind {
        use crate::motion::OverlayKind;
        match self {
            OverlayView::Keyboard(k) if k.layout == Layout::Numpad => OverlayKind::Numpad,
            OverlayView::Keyboard(_) => OverlayKind::Keyboard,
            OverlayView::Menu(_) => OverlayKind::Menu,
            OverlayView::Media(_) => OverlayKind::Media,
            OverlayView::Offer(_) => OverlayKind::Offer,
        }
    }

    fn cursor(&self) -> Option<u32> {
        match self {
            OverlayView::Keyboard(k) => key_index(k.layout, keyboard::key_at(k.layout, k.cursor).code),
            OverlayView::Menu(m) => m.selected.map(|i| i as u32),
            OverlayView::Offer(o) => (!o.choices.is_empty()).then_some(o.selected as u32),
            OverlayView::Media(_) => None,
        }
    }

    fn pick(&self) -> Option<u32> {
        match self {
            OverlayView::Keyboard(k) => k.pressed.as_deref().and_then(|code| key_index(k.layout, code)),
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
    layout: Layout,
    cursor: Cursor,
    latched: Vec<KeyCode>,
    pressed: Option<KeyCode>,
    dpad: HashSet<Button>,
    stick: (f32, f32),
    stick_dir: Option<(i32, i32)>,
    /// Direction being held and when it next repeats.
    repeat: Option<((i32, i32), Instant)>,
    east_since: Option<Instant>,
    /// Guide is down: Guide + X closes the keyboard (Guide + Y the numpad), as it opened it.
    guide_held: bool,
    /// Shift held down by the left trigger.
    trigger_shift: bool,
}

fn code(name: &str) -> Option<KeyCode> {
    KeyCode::from_str(name).ok()
}

fn tap(name: &str) -> Vec<OverlayAction> {
    code(name).map(|k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)]).unwrap_or_default()
}

impl OverlayController {
    pub fn new(layout: Layout, cursor: Cursor) -> Self {
        OverlayController {
            layout,
            cursor,
            latched: Vec::new(),
            pressed: None,
            dpad: HashSet::new(),
            stick: (0.0, 0.0),
            stick_dir: None,
            repeat: None,
            east_since: None,
            guide_held: false,
            trigger_shift: false,
        }
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn view(&self, now: Instant) -> KeyboardView {
        let name = |k: KeyCode| format!("{k:?}");
        KeyboardView {
            layout: self.layout,
            style: crate::config::OverlayStyle::keyboard(),
            cursor: self.cursor,
            latched: self.latched.iter().chain(self.trigger_shift.then_some(&KeyCode::KEY_LEFTSHIFT)).map(|k| name(*k)).collect(),
            pressed: self.pressed.map(name),
            closing: self
                .east_since
                .map(|t| (now.duration_since(t).as_secs_f32() / HOLD_TO_CLOSE.as_secs_f32()).min(1.0))
                .unwrap_or(0.0),
        }
    }

    /// Tells the controller Guide is already down as it opens (the chord that opened it).
    pub fn set_guide_held(&mut self, held: bool) {
        self.guide_held = held;
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
            InputEvent::Axis(Axis::LeftTrigger, v) if self.layout == Layout::Keyboard => self.left_trigger(v),
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
        let dir = match b {
            Button::DpadUp => Some((0, -1)),
            Button::DpadDown => Some((0, 1)),
            Button::DpadLeft => Some((-1, 0)),
            Button::DpadRight => Some((1, 0)),
            _ => None,
        };
        if let Some(dir) = dir {
            if pressed && self.dpad.insert(b) {
                self.start_moving(dir, now);
            } else if !pressed && self.dpad.remove(&b) && self.repeat.is_some_and(|(d, _)| d == dir) {
                self.repeat = None;
            }
            return Vec::new();
        }
        let closes_with = if self.layout == Layout::Keyboard { Button::West } else { Button::North };
        match (b, pressed) {
            (Button::Guide, _) => {
                self.guide_held = pressed;
                Vec::new()
            }
            (b, true) if b == closes_with && self.guide_held => vec![OverlayAction::Close],
            (Button::South, true) => self.press_selected(),
            (Button::South, false) => self.release_selected(),
            (Button::West, true) => self.shortcut("KEY_BACKSPACE"),
            (Button::North, true) if self.layout == Layout::Keyboard => self.shortcut("KEY_SPACE"),
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
        let (x, y) = self.stick;
        let threshold = if self.stick_dir.is_some() { STICK_RELEASE } else { STICK_PRESS };
        let dir = if x.abs().max(y.abs()) < threshold {
            None
        } else if x.abs() > y.abs() {
            Some((x.signum() as i32, 0))
        } else {
            Some((0, y.signum() as i32))
        };
        if dir == self.stick_dir {
            return;
        }
        self.stick_dir = dir;
        match dir {
            Some(d) => self.start_moving(d, now),
            None => self.repeat = None,
        }
    }

    fn start_moving(&mut self, dir: (i32, i32), now: Instant) {
        self.cursor = keyboard::step(self.layout, self.cursor, dir.0, dir.1);
        self.repeat = Some((dir, now + REPEAT_DELAY));
    }

    fn press_selected(&mut self) -> Vec<OverlayAction> {
        let name = keyboard::key_at(self.layout, self.cursor).code;
        let Some(k) = code(name) else { return Vec::new() };
        // Modifiers latch (held down) until the next key, like sticky keys.
        if MODIFIERS.contains(&name) {
            if let Some(i) = self.latched.iter().position(|l| *l == k) {
                self.latched.remove(i);
                return vec![OverlayAction::Key(k, false)];
            }
            self.latched.push(k);
            return vec![OverlayAction::Key(k, true)];
        }
        if self.pressed.is_some() {
            return Vec::new();
        }
        self.pressed = Some(k);
        vec![OverlayAction::Key(k, true)]
    }

    fn release_selected(&mut self) -> Vec<OverlayAction> {
        let Some(k) = self.pressed.take() else { return Vec::new() };
        let mut out = vec![OverlayAction::Key(k, false)];
        out.extend(self.release_latched());
        out
    }

    fn shortcut(&mut self, name: &str) -> Vec<OverlayAction> {
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
        if let Some((dir, mut next)) = self.repeat {
            while now >= next {
                self.cursor = keyboard::step(self.layout, self.cursor, dir.0, dir.1);
                next += REPEAT_EVERY;
                changed = true;
            }
            self.repeat = Some((dir, next));
        }
        (Vec::new(), changed)
    }

    /// When `tick` next has work (key repeat or the close countdown).
    pub fn next_deadline(&self) -> Option<Instant> {
        let close = self.east_since.map(|t| t + HOLD_TO_CLOSE);
        let repeat = self.repeat.map(|(_, t)| t);
        // While East is held the closing bar animates, so keep ticking.
        let animate = self.east_since.map(|_| Instant::now() + Duration::from_millis(30));
        [close, repeat, animate].into_iter().flatten().min()
    }

    /// Lets go of everything the overlay is holding (when it closes).
    pub fn release_all(&mut self) -> Vec<OverlayAction> {
        let mut out: Vec<OverlayAction> = self.pressed.take().map(|k| OverlayAction::Key(k, false)).into_iter().collect();
        out.extend(self.release_latched());
        if std::mem::take(&mut self.trigger_shift) {
            out.push(OverlayAction::Key(KeyCode::KEY_LEFTSHIFT, false));
        }
        self.repeat = None;
        self.east_since = None;
        self.dpad.clear();
        self.stick_dir = None;
        out
    }
}


/// Runs the overlay window: a layer-shell surface at the bottom of the screen drawing the
/// state the daemon streams, and exiting when the daemon hides it.
pub fn run() -> anyhow::Result<()> {
    // iced_layershell unwraps when the compositor has no layer-shell (GNOME): report that
    // as an error instead of a panic and backtrace.
    std::panic::set_hook(Box::new(|_| {}));
    std::panic::catch_unwind(ui::run).unwrap_or_else(|panic| {
        let why = panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(ToString::to_string));
        anyhow::bail!(
            "the overlay window could not start (the compositor probably lacks wlr-layer-shell, as GNOME does): {}",
            why.unwrap_or_default()
        )
    })
}

mod ui;

pub use ui::snapshot;

/// Drawing for overlays, generic over the message type so the settings GUI can show the
/// same thing as a live preview.
pub mod draw;
pub mod fit;

#[cfg(test)]
mod tests;
