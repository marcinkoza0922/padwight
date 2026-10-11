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
    "{south} press",
    "{west} backspace (hold to repeat)",
    "{north} space",
    "{start} enter",
    "{lb} {rb} move the text cursor",
    "hold {lt} shift",
    "Shift, Ctrl, Alt, AltGr and Super stay on for one key; press one again to let go",
    "hold {east} or press {guide} to close",
];

/// The numpad's reminder of its controls.
pub const NUMPAD_LEGEND: &[&str] =
    &["{dpad} move", "{south} press", "{west} backspace (hold to repeat)", "{start} enter", "hold {east} or press {guide} to close"];

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
    /// Guide is down: Guide + X closes the keyboard (Guide + Y the numpad), as it opened it.
    guide_held: bool,
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
        OverlayController {
            board: board.into(),
            cursor,
            latched: Vec::new(),
            pressed: None,
            dpad: HashSet::new(),
            stick: (0.0, 0.0),
            stick_dir: crate::nav::StickDir::default(),
            repeat: crate::nav::Repeat::default(),
            east_since: None,
            guide_held: false,
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
        let closes_with = if keyboard { Button::West } else { Button::North };
        match (b, pressed) {
            (Button::Guide, _) => {
                self.guide_held = pressed;
                Vec::new()
            }
            (b, true) if b == closes_with && self.guide_held => vec![OverlayAction::Close],
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

mod ui {
    use std::time::{Duration, Instant};

    use iced::{
        Color, Element, Size, Subscription, Task,
        futures::{SinkExt, channel::mpsc},
        widget::{column, responsive, space, stack},
    };
    use iced_layershell::{
        reexport::{Anchor, KeyboardInteractivity, Layer},
        settings::{LayerShellSettings, Settings, StartMode},
        to_layer_message,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    use super::{
        OverlayFrame, OverlayView, draw,
        fit::Fit,
        timing::{frame_summary, menu_log, timing_log, unix_ns},
    };
    use crate::{
        config::ScreenPosition,
        info::InfoView,
        inputlog::LogView,
        ipc::{self, Request},
        motion::{Anim, Presence},
    };

    #[to_layer_message]
    #[derive(Debug, Clone)]
    enum Message {
        State(OverlayFrame),
        /// The daemon went away: nothing left to draw for.
        Disconnected,
        /// A frame of animation.
        Tick,
    }

    /// What is on screen, and what is fading out, each with its own timing.
    struct Overlay {
        frame: OverlayFrame,
        panels: Presence<OverlayView>,
        info: Presence<InfoView>,
        logs: Presence<LogView>,
    }

    impl Overlay {
        fn new() -> Self {
            Overlay { frame: OverlayFrame::default(), panels: Presence::single(), info: Presence::default(), logs: Presence::default() }
        }

        /// Whether anything is drawn: something shown, or something still fading out.
        fn shown(&self) -> bool {
            !self.panels.is_empty() || !self.info.is_empty() || !self.logs.is_empty()
        }

        /// Whether something is still moving at `now`, so the screen needs redrawing.
        fn busy(&self, now: Instant) -> bool {
            let motion = &self.frame.motion;
            self.panels.busy(now, motion) || self.info.busy(now, motion) || self.logs.busy(now, motion)
        }

        /// Drops what has finished closing.
        fn prune(&mut self, now: Instant) {
            let motion = &self.frame.motion;
            self.panels.prune(now, motion);
            self.info.prune(now, motion);
            self.logs.prune(now, motion);
        }
    }

    /// Idle: a 1×1 invisible surface in a corner. Showing: the whole screen, transparent
    /// except for what is drawn, with clicks passing through.
    fn idle() -> (Anchor, (u32, u32)) {
        (Anchor::Top | Anchor::Right, (1, 1))
    }

    fn full_screen() -> (Anchor, (u32, u32)) {
        (Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right, (0, 0))
    }

    pub fn run() -> anyhow::Result<()> {
        let (anchor, size) = idle();
        let app = iced_layershell::application(boot, namespace, update, view)
            .style(|_, _| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: Color::WHITE });
        crate::font::BUNDLED
            .iter()
            .fold(app, |app, b| app.font(b.bytes))
            .subscription(subscription)
            .settings(Settings {
                layer_settings: LayerShellSettings {
                    size: Some(size),
                    exclusive_zone: -1,
                    anchor,
                    layer: Layer::Overlay,
                    margin: (0, 0, 0, 0),
                    // Never take keyboard focus: typed keys must reach the window underneath.
                    keyboard_interactivity: KeyboardInteractivity::None,
                    events_transparent: true,
                    start_mode: StartMode::Active,
                },
                ..Default::default()
            })
            .run()
            .map_err(|e| anyhow::anyhow!("overlay: {e}"))
    }

    fn boot() -> (Overlay, Task<Message>) {
        (Overlay::new(), Task::none())
    }

    fn namespace() -> String {
        "padwight_overlay".into()
    }

    fn update(state: &mut Overlay, message: Message) -> Task<Message> {
        match message {
            Message::State(mut frame) => {
                timing_log(&format!("recv {} {}", unix_ns(), frame_summary(&frame)));
                frame.apply_lite();
                let now = Instant::now();
                let was_shown = state.shown();
                state.panels.update(now, frame.active.iter().cloned().collect());
                state.info.update(now, frame.info.clone());
                state.logs.update(now, frame.logs.clone());
                state.frame = frame;
                state.prune(now);
                return resize(was_shown, state.shown());
            }
            Message::Tick => {
                let was_shown = state.shown();
                state.prune(Instant::now());
                return resize(was_shown, state.shown());
            }
            Message::Disconnected => return iced::exit(),
            _ => {}
        }
        Task::none()
    }

    /// Goes to the whole screen when something starts to show, and back to the corner once
    /// nothing is left to draw.
    fn resize(was_shown: bool, shown: bool) -> Task<Message> {
        let (anchor, size) = match (was_shown, shown) {
            (false, true) => full_screen(),
            (true, false) => idle(),
            _ => return Task::none(),
        };
        Task::done(Message::AnchorSizeChange(anchor, size))
    }

    fn subscription(state: &Overlay) -> Subscription<Message> {
        let watch = Subscription::run(watch);
        if state.busy(Instant::now()) {
            Subscription::batch([watch, iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)])
        } else {
            watch
        }
    }

    fn watch() -> impl iced::futures::Stream<Item = Message> {
        iced::stream::channel(16, async |mut output: mpsc::Sender<Message>| {
            let mut request = serde_json::to_string(&Request::WatchOverlay).unwrap_or_default();
            request.push('\n');
            if let Ok(mut stream) = tokio::net::UnixStream::connect(ipc::socket_path()).await
                && stream.write_all(request.as_bytes()).await.is_ok()
            {
                let mut lines = BufReader::new(stream).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(state) = serde_json::from_str::<OverlayFrame>(&line) {
                        let _ = output.send(Message::State(state)).await;
                    }
                }
            }
            let _ = output.send(Message::Disconnected).await;
            // Keep the stream alive until iced exits.
            tokio::time::sleep(Duration::from_secs(3600)).await;
        })
    }

    fn view(state: &Overlay) -> Element<'_, Message> {
        if !state.shown() {
            return space().into();
        }
        // The surface covers the whole display, so each overlay sizes itself as shares of it.
        responsive(move |display| {
            let started = Instant::now();
            let built: Element<'_, Message> = stack(layers(state, display)).into();
            timing_log(&format!("built {} {} {}x{}", unix_ns(), started.elapsed().as_nanos(), display.width, display.height));
            built
        })
        .into()
    }

    /// The overlays on screen, sized for a display of `display`.
    fn layers(state: &Overlay, display: Size) -> Vec<Element<'_, Message>> {
        let now = Instant::now();
        let motion = &state.frame.motion;
        let font = crate::font::resolve(state.frame.font.as_deref());
        // Info and log overlays sharing a spot stack up there; the keyboard or a menu goes on top.
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        for spot in ScreenPosition::GRID {
            let infos: Vec<_> = state.info.shown(now, motion).filter(|(v, _)| v.style.position == spot).collect();
            let logs: Vec<_> = state.logs.shown(now, motion).filter(|(v, _)| v.style.position == spot).collect();
            let style = infos.first().map(|(v, _)| &v.style).or_else(|| logs.first().map(|(v, _)| &v.style));
            if let Some(style) = style {
                let panels = infos
                    .iter()
                    .map(|(v, anim)| draw::info_panel(v, font, &Fit::of(&v.style, display), anim))
                    .chain(logs.iter().map(|(v, anim)| draw::log_panel(v, font, &Fit::of(&v.style, display), anim)))
                    .collect::<Vec<Element<'_, Message>>>();
                let fit = Fit::of(style, display);
                layers.push(draw::place(column(panels).spacing(12).into(), style, &fit, &Anim::still()));
            }
        }
        let glyphs = crate::info::Glyphs { family: state.frame.family, nintendo_layout: state.frame.nintendo_layout };
        for (view, anim) in state.panels.shown(now, motion) {
            layers.push(match view {
                OverlayView::Keyboard(k) => {
                    let fit = Fit::of(&k.style, display);
                    draw::place(draw::keyboard_panel(k, font, glyphs, &fit, &anim), &k.style, &fit, &anim)
                }
                OverlayView::Menu(m) => {
                    let look = draw::MenuLook {
                        colorblind: state.frame.colorblind,
                        family: state.frame.family,
                        nintendo_layout: state.frame.nintendo_layout,
                        anim,
                    };
                    menu_log(m, &anim);
                    let fit = Fit::of(&m.style, display);
                    draw::place(draw::menu_panel(m, font, look, &fit), &m.style, &fit, &anim)
                }
                OverlayView::Media(m) => {
                    let fit = Fit::of(&m.style, display);
                    draw::place(draw::media_panel(m, font, glyphs, &fit, &anim), &m.style, &fit, &anim)
                }
                OverlayView::Offer(o) => {
                    let fit = Fit::of(&o.style, display);
                    draw::place(draw::offer_panel(o, font, glyphs, &fit, &anim), &o.style, &fit, &anim)
                }
            });
        }
        layers
    }

    #[cfg(test)]
    mod motion_tests {
        use super::*;
        use crate::{config::MenuKind, menu::MenuView, motion::MotionStyle};

        fn menu_frame(motion: MotionStyle) -> OverlayFrame {
            let view = MenuView {
                title: "Belt".into(),
                kind: MenuKind::List,
                items: Vec::new(),
                selected: Some(0),
                picks: 0,
                crumbs: Vec::new(),
                hint: String::new(),
                style: Default::default(),
                touch: false,
                aim: None,
            };
            OverlayFrame {
                active: Some(OverlayView::Menu(view)),
                motion: crate::motion::MotionSet { menu: motion, ..Default::default() },
                ..Default::default()
            }
        }

        #[test]
        fn a_menu_that_closes_stays_up_for_its_close() {
            let mut state = Overlay::new();
            let _ = update(&mut state, Message::State(menu_frame(MotionStyle::Subtle)));
            assert!(state.shown());
            let _ = update(&mut state, Message::State(OverlayFrame { motion: menu_frame(MotionStyle::Subtle).motion, ..Default::default() }));
            assert!(state.shown(), "fading out, so the surface stays up");
            assert!(state.busy(Instant::now()));
        }

        #[test]
        fn a_frame_stands_in_for_an_older_one_only_when_nothing_opened_closed_or_was_picked() {
            let open = menu_frame(MotionStyle::Subtle);
            let mut moved = open.clone();
            let mut picked = open.clone();
            if let (Some(OverlayView::Menu(m)), Some(OverlayView::Menu(p))) = (&mut moved.active, &mut picked.active) {
                m.selected = Some(1);
                p.picks = 1;
            }
            assert!(moved.stands_in_for(&open), "a cursor move can be skipped in a burst");
            assert!(!picked.stands_in_for(&open), "the frame before a pick still shows");
            assert!(!OverlayFrame::default().stands_in_for(&picked), "a pick isn't lost to the close after it");
            assert!(!open.stands_in_for(&OverlayFrame::default()), "nor an opening");
        }

        #[test]
        fn a_menu_closed_by_a_pick_holds_its_pick_a_moment_even_with_motion_off() {
            let mut state = Overlay::new();
            let _ = update(&mut state, Message::State(menu_frame(MotionStyle::Off)));
            let mut picked = menu_frame(MotionStyle::Off);
            if let Some(OverlayView::Menu(m)) = &mut picked.active {
                m.picks = 1;
            }
            let _ = update(&mut state, Message::State(picked));
            let _ = update(&mut state, Message::State(OverlayFrame { motion: menu_frame(MotionStyle::Off).motion, ..Default::default() }));
            assert!(state.shown(), "held so the pick is seen");
            let later = Instant::now() + Duration::from_secs_f32(crate::motion::PICK_HOLD + 0.05);
            state.prune(later);
            assert!(!state.shown(), "then gone");
        }

        #[test]
        fn with_motion_off_a_closed_menu_goes_at_once() {
            let mut state = Overlay::new();
            let _ = update(&mut state, Message::State(menu_frame(MotionStyle::Off)));
            let closing = OverlayFrame { motion: menu_frame(MotionStyle::Off).motion, ..Default::default() };
            let _ = update(&mut state, Message::State(closing));
            assert!(!state.shown());
        }
    }
}

/// Drawing for overlays, generic over the message type so the settings GUI can show the
/// same thing as a live preview.
pub mod draw;
#[cfg(test)]
mod fade_frames;
pub mod fit;
mod timing;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_from_an_older_daemon_still_load() {
        let frame: OverlayFrame = serde_json::from_str(r#"{"info":[],"active":null}"#).unwrap();
        assert!(frame.logs.is_empty() && frame.info.is_empty() && frame.active.is_none());
        assert!(frame.motion.is_default(), "older frames have no motion, so the default");
    }

    fn at(c: &OverlayController) -> &'static str {
        keyboard::key_at(Layout::Keyboard, c.cursor()).code
    }

    fn ms(t: Instant, n: u64) -> Instant {
        t + Duration::from_millis(n)
    }

    fn start() -> OverlayController {
        OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_Q").unwrap())
    }

    #[test]
    fn dpad_moves_once_then_repeats_while_held() {
        let mut c = start();
        let t0 = Instant::now();
        c.handle(InputEvent::Button(Button::DpadRight, true), t0);
        assert_eq!(at(&c), "KEY_W");
        let (delay, every) = (crate::nav::REPEAT_DELAY, crate::nav::REPEAT_EVERY);
        assert!(!c.tick(t0 + delay / 2).1, "no repeat before the delay");
        c.tick(t0 + delay);
        assert_eq!(at(&c), "KEY_E");
        c.tick(t0 + delay + every);
        c.tick(t0 + delay + every * 2);
        assert_eq!(at(&c), "KEY_T");
        c.handle(InputEvent::Button(Button::DpadRight, false), ms(t0, 600));
        c.tick(ms(t0, 2000));
        assert_eq!(at(&c), "KEY_T", "stops when released");
    }

    #[test]
    fn stick_moves_like_a_dpad_with_hysteresis() {
        let mut c = start();
        let t0 = Instant::now();
        c.handle(InputEvent::Axis(Axis::LeftY, 0.3), t0);
        assert_eq!(at(&c), "KEY_Q", "a light push does nothing");
        c.handle(InputEvent::Axis(Axis::LeftY, 0.9), t0);
        assert_eq!(at(&c), "KEY_A");
        c.handle(InputEvent::Axis(Axis::LeftY, 0.5), t0);
        assert_eq!(at(&c), "KEY_A", "still held inside the release margin");
        c.handle(InputEvent::Axis(Axis::LeftY, 0.0), t0);
        c.handle(InputEvent::Axis(Axis::LeftY, 0.9), t0);
        assert_eq!(at(&c), "KEY_Z", "a fresh push moves again");
    }

    #[test]
    fn a_holds_the_selected_key_and_modifiers_latch_for_one_key() {
        let mut c = OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_LEFTSHIFT").unwrap());
        let t0 = Instant::now();
        let shift = KeyCode::KEY_LEFTSHIFT;
        assert_eq!(c.handle(InputEvent::Button(Button::South, true), t0), vec![OverlayAction::Key(shift, true)]);
        assert!(c.handle(InputEvent::Button(Button::South, false), t0).is_empty(), "shift stays latched");
        assert_eq!(c.view(t0).latched, ["KEY_LEFTSHIFT"]);

        c.handle(InputEvent::Button(Button::DpadRight, true), t0);
        c.handle(InputEvent::Button(Button::DpadRight, false), t0);
        assert_eq!(at(&c), "KEY_Z");
        assert_eq!(c.handle(InputEvent::Button(Button::South, true), t0), vec![OverlayAction::Key(KeyCode::KEY_Z, true)]);
        assert_eq!(
            c.handle(InputEvent::Button(Button::South, false), t0),
            vec![OverlayAction::Key(KeyCode::KEY_Z, false), OverlayAction::Key(shift, false)],
            "capital Z, then shift lets go"
        );
        assert!(c.view(t0).latched.is_empty());
    }

    #[test]
    fn left_trigger_holds_shift() {
        let mut c = start();
        let t0 = Instant::now();
        let shift = KeyCode::KEY_LEFTSHIFT;
        assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.8), t0), vec![OverlayAction::Key(shift, true)]);
        assert!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.9), t0).is_empty());
        assert_eq!(c.view(t0).latched, ["KEY_LEFTSHIFT"]);
        // Typing keeps it held: capitals until the trigger lets go.
        c.handle(InputEvent::Button(Button::South, true), t0);
        assert_eq!(c.handle(InputEvent::Button(Button::South, false), t0), vec![OverlayAction::Key(KeyCode::KEY_Q, false)]);
        assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.1), t0), vec![OverlayAction::Key(shift, false)]);
        assert!(c.view(t0).latched.is_empty());
        // Closing while held lets go of it.
        c.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0);
        assert_eq!(c.release_all(), vec![OverlayAction::Key(shift, false)]);
        // Not on the numpad.
        let mut n = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
        assert!(n.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0).is_empty());
    }

    #[test]
    fn a_latched_shift_coming_off_leaves_the_triggers_shift_down() {
        let mut c = OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_LEFTSHIFT").unwrap());
        let t0 = Instant::now();
        let shift = KeyCode::KEY_LEFTSHIFT;
        assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0), vec![OverlayAction::Key(shift, true)]);
        assert!(c.handle(InputEvent::Button(Button::South, true), t0).is_empty(), "already down");
        assert!(c.handle(InputEvent::Button(Button::South, true), t0).is_empty(), "unlatching keeps it down");
        assert_eq!(c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.0), t0), vec![OverlayAction::Key(shift, false)]);
    }

    #[test]
    fn a_late_tick_moves_one_key() {
        let mut c = start();
        let t0 = Instant::now();
        c.handle(InputEvent::Button(Button::DpadRight, true), t0);
        c.tick(ms(t0, 2000));
        assert_eq!(at(&c), "KEY_E");
    }

    #[test]
    fn holding_x_keeps_deleting_and_the_bumpers_move_the_text_cursor() {
        let mut c = start();
        let t0 = Instant::now();
        let tap = |k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)];
        assert_eq!(c.handle(InputEvent::Button(Button::West, true), t0), tap(KeyCode::KEY_BACKSPACE));
        assert!(c.tick(t0 + crate::nav::REPEAT_DELAY / 2).0.is_empty());
        assert_eq!(c.tick(t0 + crate::nav::REPEAT_DELAY).0, tap(KeyCode::KEY_BACKSPACE));
        c.handle(InputEvent::Button(Button::West, false), t0 + crate::nav::REPEAT_DELAY);
        assert_eq!(c.next_deadline(), None);
        assert_eq!(c.handle(InputEvent::Button(Button::LeftBumper, true), t0), tap(KeyCode::KEY_LEFT));
        assert_eq!(c.handle(InputEvent::Button(Button::RightBumper, true), t0), tap(KeyCode::KEY_RIGHT));
        let mut n = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
        assert!(n.handle(InputEvent::Button(Button::LeftBumper, true), t0).is_empty(), "no text cursor keys on the numpad");
    }

    #[test]
    fn the_keyboard_counts_its_caps_lock_presses_and_the_full_board_has_more_keys() {
        let mut c = OverlayController::new(Board::Basic, keyboard::find(Board::Basic, "KEY_CAPSLOCK").unwrap());
        let t0 = Instant::now();
        assert!(!c.view(t0).caps);
        c.handle(InputEvent::Button(Button::South, true), t0);
        c.handle(InputEvent::Button(Button::South, false), t0);
        assert!(c.view(t0).caps);
        assert!(keyboard::find(Board::Basic, "KEY_F1").is_none() && keyboard::find(Board::Full, "KEY_F1").is_some());
        assert!(keyboard::find(Board::Basic, "KEY_LEFT").is_some(), "the basic board has the arrows");
        assert!(keyboard::find(Board::Numpad, "KEY_KPMINUS").is_some(), "and the numpad a minus");
    }

    #[test]
    fn the_keyboard_follows_what_it_types() {
        let mut c = OverlayController::new(Board::Basic, keyboard::find(Board::Basic, "KEY_H").unwrap());
        let t0 = Instant::now();
        let press = |c: &mut OverlayController, b| {
            c.handle(InputEvent::Button(b, true), t0);
            c.handle(InputEvent::Button(b, false), t0);
        };
        press(&mut c, Button::South);
        c.handle(InputEvent::Axis(Axis::LeftTrigger, 1.0), t0);
        press(&mut c, Button::South);
        c.handle(InputEvent::Axis(Axis::LeftTrigger, 0.0), t0);
        press(&mut c, Button::North);
        let typed = c.view(t0).typed;
        // The labels come from the desktop's layout; H is h in every Latin one.
        assert!(typed.eq_ignore_ascii_case("hh ") && typed.starts_with('h'), "{typed:?}");
        press(&mut c, Button::West);
        assert_eq!(c.view(t0).typed.chars().count(), 2);
        press(&mut c, Button::Start);
        assert!(c.view(t0).typed.is_empty(), "Enter starts afresh");
    }

    #[test]
    fn face_button_shortcuts() {
        let mut c = start();
        let t0 = Instant::now();
        let tap = |k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)];
        assert_eq!(c.handle(InputEvent::Button(Button::West, true), t0), tap(KeyCode::KEY_BACKSPACE));
        assert_eq!(c.handle(InputEvent::Button(Button::North, true), t0), tap(KeyCode::KEY_SPACE));
        assert_eq!(c.handle(InputEvent::Button(Button::Start, true), t0), tap(KeyCode::KEY_ENTER));
    }

    #[test]
    fn guide_with_x_closes_the_keyboard_and_with_y_the_numpad() {
        let t0 = Instant::now();
        let mut keyboard = OverlayController::new(Layout::Keyboard, Layout::Keyboard.home());
        keyboard.handle(InputEvent::Button(Button::Guide, true), t0);
        assert_eq!(keyboard.handle(InputEvent::Button(Button::West, true), t0), vec![OverlayAction::Close]);
        let mut keyboard = OverlayController::new(Layout::Keyboard, Layout::Keyboard.home());
        assert_eq!(keyboard.handle(InputEvent::Button(Button::West, true), t0).len(), 2, "X alone is backspace");

        let mut numpad = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
        numpad.set_guide_held(true);
        assert!(!numpad.handle(InputEvent::Button(Button::West, true), t0).contains(&OverlayAction::Close), "X doesn't close the numpad");
        assert_eq!(numpad.handle(InputEvent::Button(Button::North, true), t0), vec![OverlayAction::Close]);
        numpad.handle(InputEvent::Button(Button::Guide, false), t0);
        assert!(numpad.handle(InputEvent::Button(Button::North, true), t0).is_empty());
    }

    #[test]
    fn numpad_types_digits_and_enter_and_closes_with_b() {
        let mut c = OverlayController::new(Layout::Numpad, Layout::Numpad.home());
        let t0 = Instant::now();
        let tap = |k| vec![OverlayAction::Key(k, true), OverlayAction::Key(k, false)];
        c.handle(InputEvent::Button(Button::DpadUp, true), t0);
        c.handle(InputEvent::Button(Button::DpadUp, false), t0);
        assert_eq!(c.handle(InputEvent::Button(Button::South, true), t0), vec![OverlayAction::Key(KeyCode::KEY_8, true)]);
        assert_eq!(c.handle(InputEvent::Button(Button::South, false), t0), vec![OverlayAction::Key(KeyCode::KEY_8, false)]);
        assert_eq!(c.handle(InputEvent::Button(Button::Start, true), t0), tap(KeyCode::KEY_ENTER));
        assert!(c.handle(InputEvent::Button(Button::North, true), t0).is_empty(), "no space bar on a numpad");
        assert_eq!(c.view(t0).board, Board::Numpad);
        c.handle(InputEvent::Button(Button::East, true), t0);
        assert_eq!(c.tick(ms(t0, 700)).0, vec![OverlayAction::Close]);
    }

    #[test]
    fn holding_east_closes_but_a_tap_does_not() {
        let mut c = start();
        let t0 = Instant::now();
        c.handle(InputEvent::Button(Button::East, true), t0);
        assert!(c.view(ms(t0, 350)).closing > 0.4);
        c.handle(InputEvent::Button(Button::East, false), ms(t0, 400));
        assert_eq!(c.tick(ms(t0, 2000)).0, vec![]);
        assert_eq!(c.view(ms(t0, 2000)).closing, 0.0);

        c.handle(InputEvent::Button(Button::East, true), ms(t0, 3000));
        assert_eq!(c.next_deadline().map(|d| d <= ms(t0, 3700)), Some(true));
        assert_eq!(c.tick(ms(t0, 3700)).0, vec![OverlayAction::Close]);
    }

    #[test]
    fn closing_releases_everything_held() {
        let mut c = OverlayController::new(Layout::Keyboard, keyboard::find(Layout::Keyboard, "KEY_LEFTCTRL").unwrap());
        let t0 = Instant::now();
        c.handle(InputEvent::Button(Button::South, true), t0);
        c.handle(InputEvent::Button(Button::South, false), t0);
        c.handle(InputEvent::Button(Button::DpadUp, true), t0);
        assert!(at(&c) != "KEY_LEFTCTRL");
        c.handle(InputEvent::Button(Button::South, true), t0);
        let out = c.release_all();
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out.contains(&OverlayAction::Key(KeyCode::KEY_LEFTCTRL, false)));
        assert_eq!(c.next_deadline(), None);
    }
}
