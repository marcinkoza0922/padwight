use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::{motion::MotionSet, pad_identity::ControllerSupport, sound::SoundSet};

mod curve;
mod guide;
mod gyro;
mod log;
pub mod modes;
mod ornament;
mod performance;
pub mod rumble_cues;
mod shortcuts;
mod finite;
mod store;
mod style;
mod summary;
mod typography;
mod assets;
pub use summary::{Keyword, Piece};
mod touchpad;
mod window_game;
mod menus;
mod layers;
mod templates;
pub use menus::*;
pub use layers::*;

pub use curve::*;
pub use finite::check_finite;
pub use guide::*;
pub use gyro::*;
pub use log::*;
pub use modes::Mode;
pub use performance::Performance;
pub use rumble_cues::RumbleSet;
pub use shortcuts::{Group, GuideOpener, MenuKeys, OPEN_GUIDE, PanelPress, Shortcut, ShortcutInput, Shortcuts};
pub use ornament::*;
pub use style::*;
pub use typography::*;
pub use assets::*;
use style::{default_keyboard_style, default_media_style, default_menu_style, default_numpad_style};
pub use touchpad::*;

/// Normalized gamepad button, following the Linux gamepad spec (Documentation/input/gamepad.rst).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Button {
    South,
    East,
    North,
    West,
    LeftBumper,
    RightBumper,
    Select,
    Start,
    Guide,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    /// Virtual buttons: a stick pushed past its press threshold in one direction. They
    /// work like any button (actions, gestures, combos), and as gamepad outputs they push
    /// the virtual stick that way.
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,
    /// Back paddles, on controllers that have them (see [`Button::PADDLES`]). Inputs only:
    /// the virtual pad is an Xbox 360 pad, so they can't be gamepad outputs.
    LeftPaddle,
    RightPaddle,
    LeftPaddle2,
    RightPaddle2,
}

impl Button {
    /// The four face buttons, the ones the Nintendo button layout swaps.
    pub const FACE: [Button; 4] = [Button::South, Button::East, Button::West, Button::North];

    pub const ALL: [Button; 15] = [
        Button::South,
        Button::East,
        Button::North,
        Button::West,
        Button::LeftBumper,
        Button::RightBumper,
        Button::Select,
        Button::Start,
        Button::Guide,
        Button::LeftStick,
        Button::RightStick,
        Button::DpadUp,
        Button::DpadDown,
        Button::DpadLeft,
        Button::DpadRight,
    ];

    /// The physical buttons plus the stick-direction virtual buttons.
    pub const EVERY: [Button; 23] = [
        Button::South,
        Button::East,
        Button::North,
        Button::West,
        Button::LeftBumper,
        Button::RightBumper,
        Button::Select,
        Button::Start,
        Button::Guide,
        Button::LeftStick,
        Button::RightStick,
        Button::DpadUp,
        Button::DpadDown,
        Button::DpadLeft,
        Button::DpadRight,
        Button::LeftStickUp,
        Button::LeftStickDown,
        Button::LeftStickLeft,
        Button::LeftStickRight,
        Button::RightStickUp,
        Button::RightStickDown,
        Button::RightStickLeft,
        Button::RightStickRight,
    ];

    /// The back paddles, kept apart from [`Button::ALL`] because only some controllers have them.
    pub const PADDLES: [Button; 4] = [Button::LeftPaddle, Button::RightPaddle, Button::LeftPaddle2, Button::RightPaddle2];

    /// A stick's direction buttons: up, down, left, right.
    pub fn stick_directions(s: Stick) -> [Button; 4] {
        match s {
            Stick::Left => [Button::LeftStickUp, Button::LeftStickDown, Button::LeftStickLeft, Button::LeftStickRight],
            Stick::Right => [Button::RightStickUp, Button::RightStickDown, Button::RightStickLeft, Button::RightStickRight],
        }
    }

    /// For a stick-direction button: its stick and unit direction (y positive is down).
    pub fn stick_direction(self) -> Option<(Stick, (f32, f32))> {
        Some(match self {
            Button::LeftStickUp => (Stick::Left, (0.0, -1.0)),
            Button::LeftStickDown => (Stick::Left, (0.0, 1.0)),
            Button::LeftStickLeft => (Stick::Left, (-1.0, 0.0)),
            Button::LeftStickRight => (Stick::Left, (1.0, 0.0)),
            Button::RightStickUp => (Stick::Right, (0.0, -1.0)),
            Button::RightStickDown => (Stick::Right, (0.0, 1.0)),
            Button::RightStickLeft => (Stick::Right, (-1.0, 0.0)),
            Button::RightStickRight => (Stick::Right, (1.0, 0.0)),
            _ => return None,
        })
    }
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Button::South => "A / Cross (South)",
            Button::East => "B / Circle (East)",
            Button::North => "Y / Triangle (North)",
            Button::West => "X / Square (West)",
            Button::LeftBumper => "Left Bumper",
            Button::RightBumper => "Right Bumper",
            Button::Select => "Select / Back",
            Button::Start => "Start",
            Button::Guide => "Guide / Home",
            Button::LeftStick => "Left Stick Click (L3)",
            Button::RightStick => "Right Stick Click (R3)",
            Button::DpadUp => "D-pad Up",
            Button::DpadDown => "D-pad Down",
            Button::DpadLeft => "D-pad Left",
            Button::DpadRight => "D-pad Right",
            Button::LeftPaddle => "Left Paddle",
            Button::RightPaddle => "Right Paddle",
            Button::LeftPaddle2 => "Left Paddle 2",
            Button::RightPaddle2 => "Right Paddle 2",
            Button::LeftStickUp => "Left Stick Up",
            Button::LeftStickDown => "Left Stick Down",
            Button::LeftStickLeft => "Left Stick Left",
            Button::LeftStickRight => "Left Stick Right",
            Button::RightStickUp => "Right Stick Up",
            Button::RightStickDown => "Right Stick Down",
            Button::RightStickLeft => "Right Stick Left",
            Button::RightStickRight => "Right Stick Right",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stick {
    Left,
    Right,
}

impl fmt::Display for Stick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Stick::Left => "Left Stick",
            Stick::Right => "Right Stick",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Trigger {
    Left,
    Right,
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Trigger::Left => "Left Trigger",
            Trigger::Right => "Right Trigger",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl MouseButton {
    pub const ALL: [MouseButton; 5] = [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ];
}

impl fmt::Display for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// What a digital input produces. Keys are evdev names such as `KEY_A` or `KEY_LEFTCTRL`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonAction {
    Disabled,
    Gamepad(Button),
    /// Pressed together, released together (e.g. `["KEY_LEFTCTRL", "KEY_C"]`).
    Keys(Vec<String>),
    Mouse(MouseButton),
    /// One wheel notch on press; keeps scrolling while held (after a short delay).
    Wheel(WheelDirection),
    /// Several actions at once, pressed in order and released in reverse.
    Multi(Vec<ButtonAction>),
    /// First press holds the inner action down, the next press releases it.
    Toggle(Toggled),
    /// While held, presses and releases the inner action `rate` times per second. A macro
    /// inside is played once every `every_ms` instead (see [`macro_turbo_ms`]).
    Turbo {
        action: Box<ButtonAction>,
        rate: f32,
        #[serde(default)]
        every_ms: u64,
    },
    /// Plays the macro named `name`: once per press, or looping while held if `repeat`.
    Macro { name: String, #[serde(default)] repeat: bool },
    /// Opens or closes the on-screen overlay (keyboard). While it is open the controller
    /// drives the overlay; holding East closes it.
    ToggleOverlay,
    /// Opens or closes the on-screen numpad (works like the keyboard).
    ToggleNumpad,
    /// Saves a screenshot to `~/Pictures/Screenshots/<game>/`.
    Screenshot,
    /// Starts screen recording to `~/Videos/Recordings/<game>/`, or stops it if on.
    ToggleRecording,
    /// Opens or closes the media controls at the top of the screen; while open, the controller
    /// drives whatever MPRIS player is running (Spotify, a browser, a local player).
    ToggleMedia,
    /// Opens Quick Settings, the menu over the game that also leads to Edit Controls.
    QuickSettings,
    /// While held for a couple of seconds, ends the focused window's process (a game or any
    /// other app, but never the desktop).
    ForceQuit,
    /// Shows the info overlay named here while held (wrap in Toggle to keep it up).
    ShowInfo(String),
    /// Shows the log overlay named here while held (wrap in Toggle to keep it up).
    ShowLog(String),
    /// Shows the menu named here. As a menu item's action it opens a submenu.
    OpenMenu(String),
    /// While held, the game's layer named here applies on top of the active profile and mode.
    Layer(String),
    /// Switches to the game's mode named here, which replaces whichever mode was on. Pressed
    /// while that mode is already on, it goes back to the default mode (no mode).
    Mode(String),
}

/// A Toggle's inner action, and whether it switches on by itself when the game starts.
/// Written as just the inner action unless it starts on, so configs and packs without that
/// read and write as before.
#[derive(Debug, Clone, PartialEq)]
pub struct Toggled {
    pub action: Box<ButtonAction>,
    /// On when the game starts (its first focus after launching), as if pressed.
    pub start_on: bool,
}

impl Serialize for Toggled {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Full<'a> {
            action: &'a ButtonAction,
            start_on: bool,
        }
        if self.start_on {
            Full { action: &self.action, start_on: true }.serialize(s)
        } else {
            self.action.serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for Toggled {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Full {
                action: Box<ButtonAction>,
                #[serde(default)]
                start_on: bool,
            },
            Plain(Box<ButtonAction>),
        }
        Ok(match Repr::deserialize(d)? {
            Repr::Full { action, start_on } => Toggled { action, start_on },
            Repr::Plain(action) => Toggled { action, start_on: false },
        })
    }
}

impl ButtonAction {
    /// A Toggle of `inner` that starts off.
    pub fn toggle(inner: ButtonAction) -> Self {
        ButtonAction::Toggle(Toggled { action: Box::new(inner), start_on: false })
    }
}

/// A named sequence of inputs, usable by every profile of its game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    pub steps: Vec<MacroStep>,
}

impl Macro {
    /// How long one play of the macro takes, in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.steps.iter().map(MacroStep::duration_ms).sum()
    }
}

/// The shortest gap between a turbo's presses of a macro, in milliseconds (about 60 a second).
pub const MIN_MACRO_TURBO_MS: u64 = 17;

/// The gap a turbo of a macro really uses: what was asked for, but never less than the macro
/// takes to play, so presses don't overlap.
pub fn macro_turbo_ms(every_ms: u64, macro_ms: u64) -> u64 {
    every_ms.max(macro_ms).max(MIN_MACRO_TURBO_MS)
}

/// An on-screen panel of text laid out in a grid, e.g. a game's button mappings. Cells may
/// hold `{tokens}` for controller glyphs and live values (see `crate::info`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfoOverlay {
    pub name: String,
    /// Shown whenever a profile of its game is active, rather than only by `ShowInfo`.
    #[serde(default)]
    pub always: bool,
    /// Also shown for this many seconds when the game starts (its first focus after
    /// launching), fading out at the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_start: Option<f32>,
    /// Seconds it stays after the action showing it lets go (released, or toggled off),
    /// fading out at the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linger: Option<f32>,
    /// A heading above the grid (may hold `{tokens}` too), to say what the cells are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default = "OverlayStyle::info")]
    pub style: OverlayStyle,
    /// Rows of cells; cells line up in columns.
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
    /// What its `{current_input}` cells follow.
    #[serde(default, skip_serializing_if = "CurrentInput::is_default")]
    pub current_input: CurrentInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroStep {
    /// Press, hold for `hold_ms`, release.
    Tap { action: ButtonAction, hold_ms: u64 },
    /// Press and keep held (until a matching Release or the macro ends).
    Press(ButtonAction),
    Release(ButtonAction),
    /// Pause, in milliseconds.
    Wait(u64),
    /// Moves a virtual-pad stick to (x, y), -1..1 with y positive down, until another step
    /// moves it; it returns to center when the macro ends.
    Stick { stick: Stick, x: f32, y: f32 },
}

impl MacroStep {
    pub fn action(&self) -> Option<&ButtonAction> {
        match self {
            MacroStep::Tap { action, .. } | MacroStep::Press(action) | MacroStep::Release(action) => Some(action),
            MacroStep::Wait(_) | MacroStep::Stick { .. } => None,
        }
    }

    pub fn action_mut(&mut self) -> Option<&mut ButtonAction> {
        match self {
            MacroStep::Tap { action, .. } | MacroStep::Press(action) | MacroStep::Release(action) => Some(action),
            MacroStep::Wait(_) | MacroStep::Stick { .. } => None,
        }
    }

    /// Time this step takes, in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        match self {
            MacroStep::Tap { hold_ms, .. } => *hold_ms,
            MacroStep::Wait(ms) => *ms,
            MacroStep::Press(_) | MacroStep::Release(_) | MacroStep::Stick { .. } => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WheelDirection {
    Up,
    Down,
    Left,
    Right,
}

impl WheelDirection {
    pub const ALL: [WheelDirection; 4] =
        [WheelDirection::Up, WheelDirection::Down, WheelDirection::Left, WheelDirection::Right];

    /// Notches as (horizontal, vertical); up and right are positive, like REL_WHEEL/HWHEEL.
    pub fn vector(self) -> (f32, f32) {
        match self {
            WheelDirection::Up => (0.0, 1.0),
            WheelDirection::Down => (0.0, -1.0),
            WheelDirection::Left => (-1.0, 0.0),
            WheelDirection::Right => (1.0, 0.0),
        }
    }
}

impl fmt::Display for WheelDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WheelDirection::Up => "Scroll up",
            WheelDirection::Down => "Scroll down",
            WheelDirection::Left => "Scroll left",
            WheelDirection::Right => "Scroll right",
        })
    }
}

impl ButtonAction {
    /// Wheel directions this action scrolls in while held, including inside `Multi`. Toggle and
    /// Turbo are left out: they press their inner action on their own schedule.
    pub fn wheel_directions(&self) -> Vec<WheelDirection> {
        match self {
            ButtonAction::Wheel(d) => vec![*d],
            ButtonAction::Multi(actions) => actions.iter().flat_map(ButtonAction::wheel_directions).collect(),
            _ => Vec::new(),
        }
    }

    /// What the Guide button's mapping says. The engine ignores it: Guide is built in (see the
    /// Guide overlay), so this only matters to older files.
    pub fn guide_hold() -> Self {
        ButtonAction::Mode(GUIDE_MODE.into())
    }

    /// Every key name used by this action, including inside `Multi`.
    pub fn key_names(&self) -> Vec<&String> {
        match self {
            ButtonAction::Keys(keys) => keys.iter().collect(),
            ButtonAction::Multi(actions) => actions.iter().flat_map(|a| a.key_names()).collect(),
            ButtonAction::Toggle(Toggled { action: inner, .. }) | ButtonAction::Turbo { action: inner, .. } => inner.key_names(),
            _ => Vec::new(),
        }
    }

    /// This action and every action nested inside it.
    pub fn walk(&self, f: &mut dyn FnMut(&ButtonAction)) {
        f(self);
        match self {
            ButtonAction::Multi(actions) => actions.iter().for_each(|a| a.walk(f)),
            ButtonAction::Toggle(Toggled { action: inner, .. }) | ButtonAction::Turbo { action: inner, .. } => inner.walk(f),
            _ => {}
        }
    }

    pub fn walk_mut(&mut self, f: &mut dyn FnMut(&mut ButtonAction)) {
        f(self);
        match self {
            ButtonAction::Multi(actions) => actions.iter_mut().for_each(|a| a.walk_mut(f)),
            ButtonAction::Toggle(Toggled { action: inner, .. }) | ButtonAction::Turbo { action: inner, .. } => inner.walk_mut(f),
            _ => {}
        }
    }
}

/// Buttons pressed together that act as their own input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Combo {
    pub buttons: Vec<Button>,
    pub action: ButtonAction,
}

fn default_combo_window_ms() -> u64 {
    60
}

fn default_tap_window_ms() -> u64 {
    250
}

fn default_long_press_ms() -> u64 {
    500
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GestureKind {
    DoubleTap,
    TripleTap,
    LongPress,
}

impl GestureKind {
    pub const ALL: [GestureKind; 3] = [GestureKind::DoubleTap, GestureKind::TripleTap, GestureKind::LongPress];
}

impl fmt::Display for GestureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GestureKind::DoubleTap => "Double tap",
            GestureKind::TripleTap => "Triple tap",
            GestureKind::LongPress => "Long press",
        })
    }
}

/// Extra actions for one button beyond its normal (single press) action.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Gestures {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_tap: Option<ButtonAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triple_tap: Option<ButtonAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_press: Option<ButtonAction>,
}

impl Gestures {
    pub fn get(&self, kind: GestureKind) -> Option<&ButtonAction> {
        match kind {
            GestureKind::DoubleTap => self.double_tap.as_ref(),
            GestureKind::TripleTap => self.triple_tap.as_ref(),
            GestureKind::LongPress => self.long_press.as_ref(),
        }
    }

    pub fn slot(&mut self, kind: GestureKind) -> &mut Option<ButtonAction> {
        match kind {
            GestureKind::DoubleTap => &mut self.double_tap,
            GestureKind::TripleTap => &mut self.triple_tap,
            GestureKind::LongPress => &mut self.long_press,
        }
    }

    pub fn is_empty(&self) -> bool {
        GestureKind::ALL.iter().all(|k| self.get(*k).is_none())
    }

    /// Most presses in a tap sequence that mean something (1 = no multi-tap gestures).
    pub fn max_taps(&self) -> u8 {
        if self.triple_tap.is_some() {
            3
        } else if self.double_tap.is_some() {
            2
        } else {
            1
        }
    }

    /// Action for a sequence of `taps` presses; `None` for 1 (the normal action) or unset ones.
    pub fn for_taps(&self, taps: u8) -> Option<&ButtonAction> {
        match taps {
            2 => self.double_tap.as_ref(),
            3 => self.triple_tap.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StickAction {
    Disabled,
    Gamepad {
        stick: Stick,
        #[serde(default)]
        invert_y: bool,
    },
    /// `speed` is pixels/second at full deflection.
    Mouse {
        speed: f32,
        #[serde(flatten)]
        response: MouseResponse,
        #[serde(default)]
        invert_y: bool,
    },
    /// `speed` is wheel notches/second at full deflection.
    Scroll {
        speed: f32,
        #[serde(default)]
        invert_y: bool,
    },
    /// The stick's angle picks one of `sectors` equal slices, and that slice's action is held
    /// while the stick points into it.
    Ring {
        #[serde(default = "default_sectors")]
        sectors: u8,
        /// Degrees clockwise from up to the middle of the first sector.
        #[serde(default)]
        start_angle: f32,
        /// Deflection (after the deadzone) from which a sector is active.
        #[serde(default = "default_ring_inner_radius")]
        inner_radius: f32,
        /// How far, as a share of a sector's width, the stick must pass a boundary to change sector.
        #[serde(default = "default_ring_hysteresis")]
        hysteresis: f32,
        /// One action per sector, clockwise from the first; missing ones do nothing.
        #[serde(default)]
        actions: Vec<ButtonAction>,
    },
    /// Flick stick: pushing the stick out turns the camera to that direction at once, then
    /// rotating the stick turns it the same angle. Horizontal mouse movement only, unless
    /// `vertical` adds looking up and down.
    Flick {
        /// Mouse pixels that make a 360 degree turn in the game.
        #[serde(default = "default_full_turn_px")]
        full_turn_px: f32,
        /// Deflection (after the deadzone) at which a flick starts.
        #[serde(default = "default_flick_threshold")]
        flick_threshold: f32,
        /// The initial turn is spread over this long.
        #[serde(default = "default_flick_time_ms")]
        flick_time_ms: u32,
        /// Time constant of a low-pass filter on the turning that follows a flick; 0 is off.
        #[serde(default)]
        rotate_smoothing_ms: u32,
        /// Degrees either side of straight up in which a flick turns nothing.
        #[serde(default)]
        forward_deadzone: f32,
        #[serde(default)]
        vertical: FlickVertical,
        /// Pixels/second at full deflection, when `vertical` is `look`.
        #[serde(default = "default_vertical_speed")]
        vertical_speed: f32,
    },
}


/// How a mouse stick's deflection turns into pointer speed, beyond the deadzone and curve.
/// The defaults change nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseResponse {
    /// 0.0..1.0: extra speed gained while held at full deflection, as a share of `speed`.
    #[serde(skip_serializing_if = "is_zero")]
    pub accel: f32,
    /// Milliseconds at full deflection to reach the full acceleration.
    #[serde(skip_serializing_if = "is_default_ramp")]
    pub accel_ramp_ms: u32,
    /// Vertical speed relative to horizontal.
    #[serde(skip_serializing_if = "is_one")]
    pub y_scale: f32,
    /// Extra speed multiplier in the outer tenth of the stick's travel.
    #[serde(skip_serializing_if = "is_zero")]
    pub outer_boost: f32,
    /// Time constant (milliseconds) of a low-pass filter on the stick position; 0 is off.
    #[serde(skip_serializing_if = "is_zero_ms")]
    pub smoothing_ms: u32,
}

impl Default for MouseResponse {
    fn default() -> Self {
        MouseResponse { accel: 0.0, accel_ramp_ms: 400, y_scale: 1.0, outer_boost: 0.0, smoothing_ms: 0 }
    }
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_one(v: &f32) -> bool {
    *v == 1.0
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_default_ramp(v: &u32) -> bool {
    *v == MouseResponse::default().accel_ramp_ms
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_zero_ms(v: &u32) -> bool {
    *v == 0
}

/// What a flick stick does with the stick's up/down deflection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlickVertical {
    #[default]
    Off,
    /// Moves the mouse vertically in proportion to it, using the stick's curve.
    Look,
}

fn default_full_turn_px() -> f32 {
    8000.0
}

fn default_flick_threshold() -> f32 {
    0.9
}

fn default_flick_time_ms() -> u32 {
    100
}

fn default_vertical_speed() -> f32 {
    1200.0
}

fn default_sectors() -> u8 {
    8
}

fn default_ring_inner_radius() -> f32 {
    0.5
}

fn default_ring_hysteresis() -> f32 {
    0.1
}

impl StickAction {
    /// A ring of `sectors` slices that do nothing yet.
    pub fn ring(sectors: u8) -> Self {
        StickAction::Ring {
            sectors,
            start_angle: 0.0,
            inner_radius: default_ring_inner_radius(),
            hysteresis: default_ring_hysteresis(),
            actions: vec![ButtonAction::Disabled; usize::from(sectors)],
        }
    }

    /// A flick stick with the default settings.
    pub fn flick() -> Self {
        StickAction::Flick {
            full_turn_px: default_full_turn_px(),
            flick_threshold: default_flick_threshold(),
            flick_time_ms: default_flick_time_ms(),
            rotate_smoothing_ms: 0,
            forward_deadzone: 0.0,
            vertical: FlickVertical::Off,
            vertical_speed: default_vertical_speed(),
        }
    }

    /// The ring's sector actions; none for other modes.
    pub fn ring_actions(&self) -> &[ButtonAction] {
        match self {
            StickAction::Ring { actions, .. } => actions,
            _ => &[],
        }
    }

    pub fn ring_actions_mut(&mut self) -> &mut [ButtonAction] {
        match self {
            StickAction::Ring { actions, .. } => actions,
            _ => &mut [],
        }
    }

    /// Mouse pointer at `speed` pixels/second, with the default response.
    pub fn mouse(speed: f32) -> Self {
        StickAction::Mouse { speed, response: MouseResponse::default(), invert_y: false }
    }

    /// A ring of eight directions that press keys: each sector is one direction, and the
    /// diagonals press both of theirs (e.g. W and D).
    pub fn directions(up: &str, down: &str, left: &str, right: &str) -> Self {
        let key = |k: &str| ButtonAction::Keys(vec![k.to_string()]);
        let both = |a: &str, b: &str| ButtonAction::Multi(vec![key(a), key(b)]);
        let actions = vec![
            key(up),
            both(up, right),
            key(right),
            both(down, right),
            key(down),
            both(down, left),
            key(left),
            both(up, left),
        ];
        StickAction::Ring {
            sectors: 8,
            start_angle: 0.0,
            inner_radius: default_ring_inner_radius(),
            hysteresis: default_ring_hysteresis(),
            actions,
        }
    }

    /// A ring starting at `inner_radius` (deflection after the deadzone). Other actions are
    /// unchanged.
    pub fn starting_at(mut self, inner_radius: f32) -> Self {
        if let StickAction::Ring { inner_radius: radius, .. } = &mut self {
            *radius = inner_radius;
        }
        self
    }

    pub fn wasd() -> Self {
        Self::directions("KEY_W", "KEY_S", "KEY_A", "KEY_D")
    }

    pub fn arrows() -> Self {
        Self::directions("KEY_UP", "KEY_DOWN", "KEY_LEFT", "KEY_RIGHT")
    }
}

/// Rewrites stick actions saved in the old direction-keys mode (`keys = { up, down, left,
/// right }`) as the equivalent [`StickAction::directions`] ring, so older configs and packs
/// keep working. Everything else is left alone.
pub fn migrate_direction_keys(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            // The threshold the keys pressed at was the stick's, next to its action.
            let threshold = table.get("key_threshold").and_then(toml::Value::as_float).map_or(default_key_threshold(), |t| t as f32);
            if let Some(ring) = table.get("action").and_then(|a| direction_keys_ring(a, threshold)) {
                table.insert("action".into(), ring);
            }
            table.iter_mut().for_each(|(_, v)| migrate_direction_keys(v));
        }
        toml::Value::Array(items) => items.iter_mut().for_each(migrate_direction_keys),
        _ => {}
    }
}

/// The ring that replaces a `keys` table, if `action` is one.
fn direction_keys_ring(action: &toml::Value, key_threshold: f32) -> Option<toml::Value> {
    let keys = action.as_table()?.get("keys")?.as_table()?;
    let key = |dir: &str| keys.get(dir)?.as_str().map(str::to_string);
    let (up, down, left, right) = (key("up")?, key("down")?, key("left")?, key("right")?);
    let ring = StickAction::directions(&up, &down, &left, &right).starting_at(key_threshold);
    toml::Value::try_from(ring).ok()
}

/// `text` with [`migrate_direction_keys`] applied. Text that isn't TOML comes back as it was,
/// for the parse to report.
pub fn migrate_text(text: &str) -> String {
    let Ok(table) = text.parse::<toml::Table>() else { return text.to_string() };
    let mut value = toml::Value::Table(table);
    migrate_direction_keys(&mut value);
    toml::to_string(&value).unwrap_or_else(|_| text.to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StickConfig {
    pub action: StickAction,
    /// Radial deadzone, 0.0..1.0.
    pub deadzone: f32,
    /// Response curve exponent for mouse/scroll (1.0 = linear).
    #[serde(default = "default_curve")]
    pub curve: f32,
    /// Deflection (after the deadzone) at which direction keys and the stick's direction
    /// buttons (e.g. `Button::LeftStickUp`) press.
    #[serde(default = "default_key_threshold")]
    pub key_threshold: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<Zone>,
}

fn default_curve() -> f32 {
    2.0
}

fn default_key_threshold() -> f32 {
    0.5
}

/// Extra action held while a stick's deflection or a trigger's pull is within
/// `min..max` (0.0..1.0). Only active once the input has left its rest position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub min: f32,
    pub max: f32,
    pub action: ButtonAction,
}

/// A stick or trigger, for addressing zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Analog {
    Stick(Stick),
    Trigger(Trigger),
}

impl StickConfig {
    pub fn new(action: StickAction, deadzone: f32, curve: f32) -> Self {
        StickConfig { action, deadzone, curve, key_threshold: default_key_threshold(), zones: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerAction {
    Disabled,
    Gamepad(Trigger),
    /// Acts as a digital button once pulled past `threshold` (0.0..1.0).
    Button { action: ButtonAction, threshold: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "TriggerConfigRepr")]
pub struct TriggerConfig {
    pub action: TriggerAction,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<Zone>,
}

impl From<TriggerAction> for TriggerConfig {
    fn from(action: TriggerAction) -> Self {
        TriggerConfig { action, zones: Vec::new() }
    }
}

/// Accepts both the current `{ action, zones }` table and the older bare `TriggerAction`.
#[derive(Deserialize)]
#[serde(untagged)]
enum TriggerConfigRepr {
    Full {
        action: TriggerAction,
        #[serde(default)]
        zones: Vec<Zone>,
    },
    Legacy(TriggerAction),
}

impl From<TriggerConfigRepr> for TriggerConfig {
    fn from(repr: TriggerConfigRepr) -> Self {
        match repr {
            TriggerConfigRepr::Full { action, zones } => TriggerConfig { action, zones },
            TriggerConfigRepr::Legacy(action) => action.into(),
        }
    }
}

/// What to do about the active profile when the controllers' features change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refit {
    Keep,
    /// Switch to `to`, because the active profile needs `missing`.
    Leave { missing: Vec<Feature>, to: ProfileRef },
    /// A profile left earlier can be played again.
    Return(ProfileRef),
    /// The profile left earlier is no longer the one to return to.
    Forget,
}

/// An input beyond the XInput baseline (face buttons, bumpers, analog triggers, Select,
/// Start, Guide, two clickable sticks and the D-pad).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Gyro,
    BackPaddles,
    Touchpad,
}

impl Feature {
    pub const ALL: [Feature; 3] = [Feature::Gyro, Feature::BackPaddles, Feature::Touchpad];

    pub fn label(self) -> &'static str {
        match self {
            Feature::Gyro => "gyro",
            Feature::BackPaddles => "back paddles",
            Feature::Touchpad => "touchpad",
        }
    }

    /// What a player without it loses, for warnings.
    pub fn missing(self) -> &'static str {
        match self {
            Feature::Gyro => "gyro controls won't work",
            Feature::BackPaddles => "paddle buttons won't work",
            Feature::Touchpad => "touchpad controls won't work",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub buttons: BTreeMap<Button, ButtonAction>,
    pub left_stick: StickConfig,
    pub right_stick: StickConfig,
    pub left_trigger: TriggerConfig,
    pub right_trigger: TriggerConfig,
    #[serde(default)]
    pub combos: Vec<Combo>,
    /// How long a combo member waits for the rest of its combo before acting on its own.
    #[serde(default = "default_combo_window_ms")]
    pub combo_window_ms: u64,
    #[serde(default)]
    pub gestures: BTreeMap<Button, Gestures>,
    /// Longest gap between presses that still continues a double/triple tap.
    #[serde(default = "default_tap_window_ms")]
    pub tap_window_ms: u64,
    /// Hold time before a press counts as a long press.
    #[serde(default = "default_long_press_ms")]
    pub long_press_ms: u64,
    /// Only used by controllers with motion sensors (PlayStation, Switch).
    #[serde(default)]
    pub gyro: GyroConfig,
    /// Inputs beyond a plain XInput pad that the profile can't be played without. Stated by
    /// its author, never worked out from the settings: a profile that merely adds gyro
    /// aiming to a scheme that works without it leaves this empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Feature>,
    /// The touchpad's click and what finger movement does, for controllers that have one.
    #[serde(default, skip_serializing_if = "TouchpadConfig::is_default")]
    pub touchpad: TouchpadConfig,
    /// What the Guide overlay shows: notes, and how its Mappings list is edited.
    #[serde(default, skip_serializing_if = "GuideSettings::is_default")]
    pub guide: GuideSettings,
}

impl Profile {
    /// Whether a controller with these features can play the profile.
    pub fn usable_with(&self, have: &[Feature]) -> bool {
        self.requires.iter().all(|f| have.contains(f))
    }

    /// True if `b` belongs to a combo (2+ buttons), so its own action is held back briefly.
    pub fn in_combo(&self, b: Button) -> bool {
        self.combos.iter().any(|c| c.buttons.len() >= 2 && c.buttons.contains(&b))
    }

    pub fn button(&self, b: Button) -> &ButtonAction {
        self.buttons.get(&b).unwrap_or(&ButtonAction::Disabled)
    }

    /// Whether every face button's plain press still sends its own button. Only the base
    /// assignments count: a layer's changes don't turn the Nintendo layout's swap off.
    pub fn faces_default(&self) -> bool {
        Button::FACE.iter().all(|&b| *self.button(b) == ButtonAction::Gamepad(b))
    }

    /// The button's gestures, if it has any set.
    /// Every top-level action in the profile: buttons, gestures, combos, triggers and zones.
    pub fn actions(&self) -> Vec<&ButtonAction> {
        let mut all: Vec<&ButtonAction> = self.buttons.values().collect();
        all.extend(self.gestures.values().flat_map(|g| GestureKind::ALL.into_iter().filter_map(|k| g.get(k))));
        all.extend(self.combos.iter().map(|c| &c.action));
        for t in [&self.left_trigger, &self.right_trigger] {
            if let TriggerAction::Button { action, .. } = &t.action {
                all.push(action);
            }
            all.extend(t.zones.iter().map(|z| &z.action));
        }
        all.extend(self.left_stick.zones.iter().chain(&self.right_stick.zones).map(|z| &z.action));
        all.extend(self.touchpad.actions());
        all.extend(self.left_stick.action.ring_actions().iter().chain(self.right_stick.action.ring_actions()));
        all
    }

    pub fn actions_mut(&mut self) -> Vec<&mut ButtonAction> {
        let mut all: Vec<&mut ButtonAction> = self.buttons.values_mut().collect();
        for g in self.gestures.values_mut() {
            all.extend([&mut g.double_tap, &mut g.triple_tap, &mut g.long_press].into_iter().filter_map(|s| s.as_mut()));
        }
        all.extend(self.combos.iter_mut().map(|c| &mut c.action));
        for t in [&mut self.left_trigger, &mut self.right_trigger] {
            if let TriggerAction::Button { action, .. } = &mut t.action {
                all.push(action);
            }
            all.extend(t.zones.iter_mut().map(|z| &mut z.action));
        }
        for stick in [&mut self.left_stick, &mut self.right_stick] {
            all.extend(stick.zones.iter_mut().map(|z| &mut z.action));
            all.extend(stick.action.ring_actions_mut());
        }
        all.extend(self.touchpad.actions_mut());
        all
    }

    pub fn gestures(&self, b: Button) -> Option<&Gestures> {
        self.gestures.get(&b).filter(|g| !g.is_empty())
    }

    pub fn set_button(&mut self, b: Button, action: ButtonAction) {
        self.buttons.insert(b, action);
    }

    pub fn stick(&self, s: Stick) -> &StickConfig {
        match s {
            Stick::Left => &self.left_stick,
            Stick::Right => &self.right_stick,
        }
    }

    pub fn stick_mut(&mut self, s: Stick) -> &mut StickConfig {
        match s {
            Stick::Left => &mut self.left_stick,
            Stick::Right => &mut self.right_stick,
        }
    }

    pub fn trigger(&self, t: Trigger) -> &TriggerAction {
        match t {
            Trigger::Left => &self.left_trigger.action,
            Trigger::Right => &self.right_trigger.action,
        }
    }

    pub fn trigger_mut(&mut self, t: Trigger) -> &mut TriggerAction {
        match t {
            Trigger::Left => &mut self.left_trigger.action,
            Trigger::Right => &mut self.right_trigger.action,
        }
    }

    pub fn zones(&self, a: Analog) -> &[Zone] {
        match a {
            Analog::Stick(s) => &self.stick(s).zones,
            Analog::Trigger(Trigger::Left) => &self.left_trigger.zones,
            Analog::Trigger(Trigger::Right) => &self.right_trigger.zones,
        }
    }

    pub fn zones_mut(&mut self, a: Analog) -> &mut Vec<Zone> {
        match a {
            Analog::Stick(s) => &mut self.stick_mut(s).zones,
            Analog::Trigger(Trigger::Left) => &mut self.left_trigger.zones,
            Analog::Trigger(Trigger::Right) => &mut self.right_trigger.zones,
        }
    }

    /// Sets Guide to the built-in mapping. The engine doesn't read it (Guide opens the Guide
    /// overlay), so it has no gestures: Steam is reached from the overlay's menu.
    pub fn take_guide(&mut self) {
        self.set_button(Button::Guide, ButtonAction::guide_hold());
        self.gestures.remove(&Button::Guide);
    }

    /// This profile with `layers` on top, oldest first: later ones win.
    pub fn with_layers<'a>(&self, layers: impl IntoIterator<Item = &'a Layer>) -> Profile {
        let mut p = self.clone();
        for layer in layers {
            layer.apply(&mut p);
        }
        p
    }

}

/// What a per-game rule compares against the focused window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    /// Executable file name, e.g. `factorio` or (Wine/Proton) `eldenring.exe`.
    Executable,
    SteamAppId,
    WindowClass,
}

impl RuleKind {
    pub const ALL: [RuleKind; 3] = [RuleKind::Executable, RuleKind::SteamAppId, RuleKind::WindowClass];
}

impl fmt::Display for RuleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RuleKind::Executable => "Executable",
            RuleKind::SteamAppId => "Steam App ID",
            RuleKind::WindowClass => "Window class",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub kind: RuleKind,
    pub value: String,
    /// One of its game's profiles.
    pub profile: String,
    /// Off when an imported game's rule for the same window took over.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

impl Rule {
    pub fn new(kind: RuleKind, value: impl Into<String>, profile: impl Into<String>) -> Self {
        Rule { kind, value: value.into(), profile: profile.into(), enabled: true }
    }

    /// Same window match (kind and value), whichever profile it picks.
    pub fn same_match(&self, other: &Rule) -> bool {
        self.kind == other.kind && self.value.trim().eq_ignore_ascii_case(other.value.trim())
    }
}

/// A profile, by its game (`None`: General) and name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ProfileRef {
    #[serde(default, rename = "setup", alias = "game", skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    pub profile: String,
}

impl ProfileRef {
    pub fn new(game: Option<&str>, profile: &str) -> Self {
        ProfileRef { game: game.map(str::to_string), profile: profile.to_string() }
    }
}

impl fmt::Display for ProfileRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.game {
            Some(game) => write!(f, "{game} › {}", self.profile),
            None => f.write_str(&self.profile),
        }
    }
}

/// Per-game profile switching. The rules themselves belong to games.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoSwitch {
    pub enabled: bool,
    /// Profile to return to when a game (one with rules) loses focus; `None` leaves the
    /// current profile alone.
    #[serde(default = "AutoSwitch::gamepad", with = "keep_or_profile")]
    pub default_profile: Option<ProfileRef>,
}

impl AutoSwitch {
    fn gamepad() -> Option<ProfileRef> {
        Some(ProfileRef::new(None, "Gamepad"))
    }
}

impl Default for AutoSwitch {
    fn default() -> Self {
        AutoSwitch { enabled: true, default_profile: AutoSwitch::gamepad() }
    }
}

/// The default profile, or `"keep"` for none: a missing value means General's Gamepad.
mod keep_or_profile {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::ProfileRef;

    const KEEP: &str = "keep";

    pub fn serialize<S: Serializer>(value: &Option<ProfileRef>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(at) => at.serialize(s),
            None => s.serialize_str(KEEP),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ProfileRef>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Stored {
            At(ProfileRef),
            Keep(String),
        }
        match Stored::deserialize(d)? {
            Stored::At(at) => Ok(Some(at)),
            Stored::Keep(word) if word == KEEP => Ok(None),
            Stored::Keep(word) => Err(serde::de::Error::custom(format!("expected a profile or \"{KEEP}\", found {word:?}"))),
        }
    }
}

/// A pack this game was made from, and how to tell whether it has been edited since.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    pub id: String,
    /// The pack's own name, which the game may have been renamed from.
    #[serde(default)]
    pub name: String,
    pub version: String,
    /// From the built-in library rather than a file.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub library: bool,
    /// Content hash of each item at import (see `crate::pack::item_hashes`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hashes: BTreeMap<String, String>,
}

/// Another pack, credited by a fork.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PackRef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
}

/// What a game's next export says about itself.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PackInfo {
    /// Empty until the first export.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub made_with: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub based_on: Option<PackRef>,
}

impl PackInfo {
    fn is_empty(&self) -> bool {
        *self == PackInfo::default()
    }
}

/// A controller setup: its profiles, the macros, menus and info overlays they use, and the rules
/// that switch to it. General is a setup too, with no rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    #[serde(default, skip_serializing_if = "PackInfo::is_empty")]
    pub pack: PackInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    /// Mapped with `ButtonAction::Macro`.
    #[serde(default)]
    pub macros: Vec<Macro>,
    /// Opened with `ButtonAction::OpenMenu`.
    #[serde(default)]
    pub menus: Vec<Menu>,
    /// Shown by `ButtonAction::ShowInfo`, or always while the game is active.
    #[serde(default, rename = "info_overlays")]
    pub info: Vec<InfoOverlay>,
    /// Shown by `ButtonAction::ShowLog`, or always while the game is active.
    #[serde(default, rename = "log_overlays", skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<LogOverlay>,
    /// Held with `ButtonAction::Layer`, over whichever profile and mode are active.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
    /// Switched to with `ButtonAction::Mode`, one at a time, over whichever profile is active.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<Mode>,
    /// Looks of the on-screen keyboard and numpad while this game is active; the global ones
    /// (`Config::keyboard_style`, `Config::numpad_style`) apply when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_style: Option<OverlayStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numpad_style: Option<OverlayStyle>,
    /// Looks of the media controls and the in-game menu (Guide + Start, Edit Controls included)
    /// while this game is active; the global ones (`Config::media_style`, `Config::menu_style`)
    /// apply when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_style: Option<OverlayStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_style: Option<OverlayStyle>,
    /// Font of every overlay, menu and keyboard while this game is active; `Config::overlay_font`
    /// applies when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_font: Option<String>,
    /// The Nintendo button layout while this game is active; `None` follows
    /// `Config::nintendo_layout`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nintendo_layout: Option<bool>,
    /// The sounds of each overlay's cursor steps and picks, while this game is active; the
    /// defaults when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sounds: Option<SoundSet>,
    /// The controller's buzzes for each overlay, while this game is active; silent when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rumble: Option<RumbleSet>,
    /// How each overlay moves while this game is active; the global set (`Config::motion`)
    /// applies when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<MotionSet>,
    /// The icons and sounds this game carries, usable only while it is active.
    #[serde(default, skip_serializing_if = "PackAssets::is_empty")]
    pub assets: PackAssets,
    /// Other controllers this game supports. When one of them is connected, the virtual pad
    /// presents itself as it (see [`crate::pad_identity`]); `None` is an Xbox 360 pad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controllers: Option<ControllerSupport>,
    /// Use this setup even when Steam starts the game and lists it with full controller support.
    /// Otherwise Padwight lets go of the controllers while the game is in front, for Steam Input.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub over_steam: bool,
}

impl Game {
    /// A game with these profiles and no items of its own.
    pub fn new(name: &str, profiles: Vec<Profile>) -> Self {
        Game {
            name: name.into(),
            pack: PackInfo::default(),
            origin: None,
            rules: Vec::new(),
            profiles,
            macros: Vec::new(),
            menus: Vec::new(),
            info: Vec::new(),
            logs: Vec::new(),
            layers: Vec::new(),
            modes: Vec::new(),
            keyboard_style: None,
            numpad_style: None,
            media_style: None,
            menu_style: None,
            overlay_font: None,
            nintendo_layout: None,
            sounds: None,
            rumble: None,
            motion: None,
            assets: PackAssets::default(),
            controllers: None,
            over_steam: false,
        }
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Drops any built-in mode a file stored in the game (older ones stored them as layers).
    /// They're never stored: every game has them, fixed (see [`built_in_mode`]).
    pub fn strip_built_in_modes(&mut self) {
        self.layers.retain(|l| built_in_mode(&l.name).is_none());
        self.modes.retain(|l| built_in_mode(&l.name).is_none());
    }

    /// Takes its menus out of any Toggle (see [`untoggle_menus`]), and makes its toggled layers
    /// modes (see [`modes::toggled_layers_to_modes`]).
    pub fn untoggle_menus(&mut self) {
        untoggle_menus(&mut self.profiles, &mut self.menus, &mut self.layers);
        untoggle_menus(&mut [], &mut [], &mut self.modes);
        modes::toggled_layers_to_modes(&mut self.profiles, &mut self.menus, &mut self.layers, &mut self.modes);
    }

    /// Points every reference to the macro, menu or info overlay `old` (of `kind`) at `new`:
    /// in profiles, menu items and macro steps.
    pub fn rename_refs(&mut self, kind: ItemKind, old: &str, new: &str) {
        rename_in(&mut self.profiles, &mut self.menus, &mut self.macros, &mut self.layers, kind, old, new);
        rename_in(&mut [], &mut [], &mut [], &mut self.modes, kind, old, new);
    }

    /// The names of its items of one kind.
    pub fn names(&self, kind: ItemKind) -> Vec<&str> {
        match kind {
            ItemKind::Macro => self.macros.iter().map(|m| m.name.as_str()).collect(),
            ItemKind::Menu => self.menus.iter().map(|m| m.name.as_str()).collect(),
            ItemKind::Info => self.info.iter().map(|o| o.name.as_str()).collect(),
            ItemKind::Log => self.logs.iter().map(|o| o.name.as_str()).collect(),
            ItemKind::Layer => self.layers.iter().map(|l| l.name.as_str()).collect(),
            ItemKind::Mode => self.modes.iter().map(|l| l.name.as_str()).collect(),
        }
    }

    /// Renames its item `old` of `kind` to `new`, without following references (see
    /// [`Game::rename_refs`]).
    pub fn rename_item(&mut self, kind: ItemKind, old: &str, new: &str) {
        let new = new.to_string();
        match kind {
            ItemKind::Macro => self.macros.iter_mut().filter(|m| m.name == old).for_each(|m| m.name = new.clone()),
            ItemKind::Menu => self.menus.iter_mut().filter(|m| m.name == old).for_each(|m| m.name = new.clone()),
            ItemKind::Info => self.info.iter_mut().filter(|o| o.name == old).for_each(|o| o.name = new.clone()),
            ItemKind::Log => self.logs.iter_mut().filter(|o| o.name == old).for_each(|o| o.name = new.clone()),
            ItemKind::Layer => self.layers.iter_mut().filter(|l| l.name == old).for_each(|l| l.name = new.clone()),
            ItemKind::Mode => self.modes.iter_mut().filter(|l| l.name == old).for_each(|l| l.name = new.clone()),
        }
    }

    /// Its layers named in `active` (oldest first), skipping names it has none of.
    pub fn layers_named<'a>(&'a self, active: &'a [String]) -> impl Iterator<Item = &'a Layer> {
        active.iter().filter_map(|n| self.layers.iter().find(|l| &l.name == n))
    }
}

/// `base` if it's free, else `base (2)`, `base (3)`, …: how an item or game is renamed when
/// its name is taken.
pub fn free_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..).map(|i| format!("{base} ({i})")).find(|n| !taken(n)).unwrap()
}

/// The kinds of named item a profile refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemKind {
    Macro,
    Menu,
    Info,
    Log,
    Layer,
    Mode,
}

impl ItemKind {
    pub const ALL: [ItemKind; 6] = [ItemKind::Macro, ItemKind::Menu, ItemKind::Info, ItemKind::Log, ItemKind::Layer, ItemKind::Mode];

    pub fn noun(self) -> &'static str {
        match self {
            ItemKind::Macro => "macro",
            ItemKind::Menu => "menu",
            ItemKind::Info => "info overlay",
            ItemKind::Log => "log overlay",
            ItemKind::Layer => "layer",
            ItemKind::Mode => "mode",
        }
    }

    /// The name `action` refers to, if it's a reference to this kind.
    pub fn name_in(self, action: &ButtonAction) -> Option<&String> {
        match (self, action) {
            (ItemKind::Macro, ButtonAction::Macro { name, .. })
            | (ItemKind::Menu, ButtonAction::OpenMenu(name))
            | (ItemKind::Info, ButtonAction::ShowInfo(name))
            | (ItemKind::Log, ButtonAction::ShowLog(name))
            | (ItemKind::Layer, ButtonAction::Layer(name))
            | (ItemKind::Mode, ButtonAction::Mode(name)) => Some(name),
            _ => None,
        }
    }

    fn name_in_mut(self, action: &mut ButtonAction) -> Option<&mut String> {
        match (self, action) {
            (ItemKind::Macro, ButtonAction::Macro { name, .. })
            | (ItemKind::Menu, ButtonAction::OpenMenu(name))
            | (ItemKind::Info, ButtonAction::ShowInfo(name))
            | (ItemKind::Log, ButtonAction::ShowLog(name))
            | (ItemKind::Layer, ButtonAction::Layer(name))
            | (ItemKind::Mode, ButtonAction::Mode(name)) => Some(name),
            _ => None,
        }
    }
}

/// Takes every menu opened through a Toggle out of it. A menu can't be toggled: one with a close
/// button is put away with that, so a toggled one would leave the Toggle on with nothing open.
pub fn untoggle_menus(profiles: &mut [Profile], menus: &mut [Menu], layers: &mut [Layer]) {
    let mut untoggle = |a: &mut ButtonAction| {
        a.walk_mut(&mut |a| {
            if let ButtonAction::Toggle(Toggled { action, .. }) = a
                && let ButtonAction::OpenMenu(_) = **action
            {
                *a = (**action).clone();
            }
        })
    };
    let actions = profiles.iter_mut().flat_map(Profile::actions_mut);
    let items = menus.iter_mut().flat_map(|m| m.items.iter_mut().map(|item| &mut item.action));
    actions.chain(items).chain(layers.iter_mut().flat_map(Layer::actions_mut)).for_each(&mut untoggle);
}

#[expect(clippy::too_many_arguments, reason = "predates the size lints")]
fn rename_in(
    profiles: &mut [Profile],
    menus: &mut [Menu],
    macros: &mut [Macro],
    layers: &mut [Layer],
    kind: ItemKind,
    old: &str,
    new: &str,
) {
    let mut follow = |a: &mut ButtonAction| {
        a.walk_mut(&mut |a| {
            if let Some(name) = kind.name_in_mut(a)
                && name == old
            {
                *name = new.to_string();
            }
        })
    };
    for p in profiles.iter_mut() {
        p.actions_mut().into_iter().for_each(&mut follow);
    }
    for m in menus {
        m.items.iter_mut().for_each(|item| follow(&mut item.action));
    }
    for m in macros {
        m.steps.iter_mut().filter_map(MacroStep::action_mut).for_each(&mut follow);
    }
    for l in layers.iter_mut() {
        l.actions_mut().into_iter().for_each(&mut follow);
    }
    // Touchpads in menu mode name their menu outside any action.
    if kind == ItemKind::Menu {
        let pads = profiles.iter_mut().map(|p| &mut p.touchpad).chain(layers.iter_mut().filter_map(|l| l.touchpad.as_mut()));
        for name in pads.flat_map(TouchpadConfig::menus_mut).filter(|n| *n == old) {
            *name = new.to_string();
        }
    }
    // An info overlay used as a layer's indicator follows its rename too.
    if kind == ItemKind::Info {
        for l in layers.iter_mut() {
            if let Indicator::Info(name) = &mut l.indicator
                && name == old
            {
                *name = new.to_string();
            }
        }
    }
}

/// What the active profile can use: its game's items.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scope {
    pub macros: Vec<Macro>,
    pub menus: Vec<Menu>,
    pub info: Vec<InfoOverlay>,
    pub logs: Vec<LogOverlay>,
    /// The game's own, then the built-in ones.
    pub layers: Vec<Layer>,
    pub modes: Vec<Mode>,
    /// The active game's overlay sounds.
    pub sounds: SoundSet,
    /// The active game's rumble cues.
    pub rumble: RumbleSet,
}

impl Scope {
    /// The game's own modes, without the built-in ones that follow them.
    pub fn game_modes(&self) -> &[Mode] {
        let own = self.modes.iter().take_while(|m| built_in_mode(&m.name).is_none()).count();
        &self.modes[..own]
    }
}

/// Like [`Scope`], borrowed: what a game's profiles and items can refer to.
#[derive(Debug, Clone, Default)]
pub struct ScopeRef<'a> {
    pub macros: Vec<&'a Macro>,
    pub menus: Vec<&'a Menu>,
    pub info: Vec<&'a InfoOverlay>,
    pub logs: Vec<&'a LogOverlay>,
    pub layers: Vec<&'a Layer>,
    pub modes: Vec<std::borrow::Cow<'a, Mode>>,
}

impl ScopeRef<'_> {
    pub fn names(&self, kind: ItemKind) -> Vec<&str> {
        match kind {
            ItemKind::Macro => self.macros.iter().map(|m| m.name.as_str()).collect(),
            ItemKind::Menu => self.menus.iter().map(|m| m.name.as_str()).collect(),
            ItemKind::Info => self.info.iter().map(|o| o.name.as_str()).collect(),
            ItemKind::Log => self.logs.iter().map(|o| o.name.as_str()).collect(),
            ItemKind::Layer => self.layers.iter().map(|l| l.name.as_str()).collect(),
            ItemKind::Mode => self.modes.iter().map(|l| l.name.as_str()).collect(),
        }
    }
}

/// Whether the settings window is light or dark. `Auto` follows the desktop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Auto,
    Light,
    Dark,
}

impl Appearance {
    fn is_auto(&self) -> bool {
        *self == Appearance::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    #[serde(default)]
    pub active: ProfileRef,
    /// Device names the daemon should leave alone.
    #[serde(default)]
    pub ignored_devices: Vec<String>,
    #[serde(default)]
    pub auto_switch: AutoSwitch,
    /// Gyro drift (degrees/second per raw sensor axis) measured by "Calibrate gyro", per controller.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gyro_calibration: BTreeMap<String, [f32; 3]>,
    #[serde(default = "default_keyboard_style")]
    pub keyboard_style: OverlayStyle,
    /// The on-screen keyboard has every key (function keys, editing keys) rather than just
    /// what typing into a game needs.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub full_keyboard: bool,
    /// How strongly the controllers rumble, as a share of what games ask for (0 to 1).
    #[serde(default = "full_rumble", skip_serializing_if = "is_full_rumble")]
    pub rumble_strength: f32,
    /// The controller gives a faint pulse as an overlay's item is picked (a key, a menu item).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pick_rumble: bool,
    #[serde(default = "default_numpad_style")]
    pub numpad_style: OverlayStyle,
    #[serde(default = "default_media_style")]
    pub media_style: OverlayStyle,
    /// The in-game menu (Guide + Start), Edit Controls included.
    #[serde(default = "default_menu_style")]
    pub menu_style: OverlayStyle,
    /// Whose button glyphs info overlays use when the controller in use isn't recognized.
    #[serde(default)]
    pub info_glyphs: crate::info::PadFamily,
    /// Swaps A with B and X with Y, to match prompts drawn in the Nintendo layout. On a Nintendo
    /// pad the face buttons send the letters on their labels; a face button's plain press that
    /// is reassigned turns that off. Other pads only get swapped labels.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nintendo_layout: bool,
    /// Where the system shortcuts sit: the Guide overlay's opener, the Guide mode's shortcuts and
    /// mouse mode's keys. They're the same in every setup.
    #[serde(default, skip_serializing_if = "Shortcuts::is_default")]
    pub shortcuts: Shortcuts,
    /// Whether the gyro and touchpad move the pointer in mouse mode. Same in every setup.
    #[serde(default, skip_serializing_if = "MouseModeExtras::is_default")]
    pub mouse_mode: MouseModeExtras,
    /// Draws the added and removed rows of menus in blue and orange, with a + or − in front, so
    /// they can be told apart without red and green.
    /// The file keeps the key's first spelling, so configs that set it still load.
    #[serde(default, skip_serializing_if = "std::ops::Not::not", rename = "colourblind_tones")]
    pub colorblind_tones: bool,
    /// Light, dark or the desktop's choice, for the settings window.
    #[serde(default, skip_serializing_if = "Appearance::is_auto")]
    pub appearance: Appearance,
    /// How each kind of overlay moves, unless the active game sets its own.
    #[serde(default, skip_serializing_if = "MotionSet::is_default", deserialize_with = "crate::motion::deserialize_set")]
    pub motion: MotionSet,
    /// The sounds of each overlay, unless the active game sets its own.
    #[serde(default, skip_serializing_if = "SoundSet::is_default")]
    pub sounds: SoundSet,
    /// What every overlay goes without, whatever the setups say: decorations, motion, sounds,
    /// drawn glyphs.
    #[serde(default, skip_serializing_if = "Performance::is_default")]
    pub performance: Performance,
    /// Font of every overlay, menu and keyboard (a bundled or installed family); the system's
    /// own when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_font: Option<String>,
    /// Profiles that aren't for a particular game (desktop, plain gamepad). Read from its folder.
    #[serde(default = "default_general")]
    pub general: Game,
    #[serde(default, rename = "setups", alias = "games")]
    pub games: Vec<Game>,
    /// IDs of library packs the user asked never to be offered again (see `offer`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declined_packs: Vec<String>,
}

/// The General setup a config starts with, for a `config.toml` that doesn't say.
fn default_general() -> Game {
    Config::default().general
}

fn full_rumble() -> f32 {
    1.0
}

fn is_full_rumble(strength: &f32) -> bool {
    (*strength - 1.0).abs() < f32::EPSILON
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            active: ProfileRef::new(None, "Gamepad"),
            ignored_devices: Vec::new(),
            auto_switch: AutoSwitch::default(),
            colorblind_tones: false,
            appearance: Appearance::Auto,
            motion: MotionSet::default(),
            sounds: SoundSet::default(),
            performance: Performance::default(),
            gyro_calibration: BTreeMap::new(),
            keyboard_style: OverlayStyle::keyboard(),
            full_keyboard: false,
            rumble_strength: 1.0,
            pick_rumble: false,
            numpad_style: OverlayStyle::numpad(),
            media_style: OverlayStyle::media(),
            menu_style: OverlayStyle::default(),
            info_glyphs: crate::info::PadFamily::default(),
            nintendo_layout: false,
            shortcuts: Shortcuts::default(),
            mouse_mode: MouseModeExtras::default(),
            overlay_font: None,
            general: Game::new("General", vec![Profile::passthrough("Gamepad"), Profile::desktop("Desktop")]),
            games: Vec::new(),
            declined_packs: Vec::new(),
        }
    }
}

impl Config {
    /// `config.toml` in the user's config folder: `~/.config/padwight/` on Linux, and
    /// `%APPDATA%\padwight\` on Windows, where `XDG_CONFIG_HOME` can name another, as on Linux.
    pub fn path() -> PathBuf {
        let windows_override = if cfg!(windows) { std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) } else { None };
        windows_override
            .or_else(dirs::config_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("padwight")
            .join("config.toml")
    }

    /// Loads the config, writing the default one if none exists yet. See [`Config::load_from`].
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::path())
    }

    /// [`Config::load`] for the app settings at `path`. The setups are read from the folders beside
    /// it (see the `store` module).
    pub fn load_from(path: &Path) -> Result<Self> {
        let first = !path.exists();
        // Without `config.toml` the settings are the defaults, but setups saved beside it stay.
        let mut config: Config = if first {
            Config::default()
        } else {
            let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
            finite::from_toml(&text).with_context(|| format!("parsing {}", path.display()))?
        };
        store::load(store::root_of(path), &mut config)?;
        config.shortcuts = std::mem::take(&mut config.shortcuts).sanitized();
        for game in config.games.iter_mut().chain(std::iter::once(&mut config.general)) {
            // A game left as imported stays so: the upgrade isn't an edit of the player's.
            let unedited = game.origin.is_some() && crate::pack::edited_items(game).is_empty();
            game.strip_built_in_modes();
            game.untoggle_menus();
            tidy_combos(&mut game.profiles, &mut game.layers);
            tidy_combos(&mut [], &mut game.modes);
            if unedited {
                let hashes = crate::pack::item_hashes(game);
                if let Some(origin) = game.origin.as_mut() {
                    origin.hashes = hashes;
                }
            }
        }
        if first {
            config.save_to(path)?;
        }
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::path())
    }

    /// [`Config::save`] to `path`, with the setups written beside it.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let root = store::root_of(path);
        std::fs::create_dir_all(root)?;
        let mut config = self.clone();
        for game in config.games.iter_mut().chain(std::iter::once(&mut config.general)) {
            game.strip_built_in_modes();
        }
        store::save(root, &config)?;
        self.save_settings_to(path)
    }

    /// Saves only `config.toml`: the app settings, such as the active profile, the remapping
    /// switch and gyro calibrations. For changes that leave every setup as it was saved, so a
    /// profile switch doesn't rewrite the whole config folder.
    pub fn save_settings(&self) -> Result<()> {
        self.save_settings_to(&Self::path())
    }

    /// [`Config::save_settings`] to `path`.
    pub fn save_settings_to(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(store::root_of(path))?;
        // Write-then-rename so the daemon never reads a half-written file.
        let tmp = path.with_extension("toml.tmp");
        store::write_private(&tmp, &store::app_settings(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The game named `game`, or General for `None`.
    pub fn game(&self, game: Option<&str>) -> Option<&Game> {
        match game {
            None => Some(&self.general),
            Some(name) => self.games.iter().find(|g| g.name == name),
        }
    }

    pub fn game_mut(&mut self, game: Option<&str>) -> Option<&mut Game> {
        match game {
            None => Some(&mut self.general),
            Some(name) => self.games.iter_mut().find(|g| g.name == name),
        }
    }

    /// General, then every game, each with its `ProfileRef` game part.
    pub fn all_games(&self) -> impl Iterator<Item = (Option<&str>, &Game)> {
        std::iter::once((None, &self.general)).chain(self.games.iter().map(|g| (Some(g.name.as_str()), g)))
    }

    pub fn profile(&self, at: &ProfileRef) -> Option<&Profile> {
        self.game(at.game.as_deref())?.profile(&at.profile)
    }

    /// How the on-screen keyboard looks now: the active game's own style, else the global one.
    pub fn active_keyboard_style(&self) -> &OverlayStyle {
        self.active_game().keyboard_style.as_ref().unwrap_or(&self.keyboard_style)
    }

    /// Like [`Config::active_keyboard_style`], for the numpad.
    pub fn active_numpad_style(&self) -> &OverlayStyle {
        self.active_game().numpad_style.as_ref().unwrap_or(&self.numpad_style)
    }

    /// Like [`Config::active_keyboard_style`], for the media controls.
    pub fn active_media_style(&self) -> &OverlayStyle {
        self.active_game().media_style.as_ref().unwrap_or(&self.media_style)
    }

    /// How each overlay moves: the active game's own set, or the global one.
    pub fn active_motion(&self) -> MotionSet {
        self.performance.motion(self.active_game().motion.unwrap_or(self.motion))
    }

    /// Like [`Config::active_keyboard_style`], for the in-game menu and Edit Controls.
    pub fn active_menu_style(&self) -> &OverlayStyle {
        self.active_game().menu_style.as_ref().unwrap_or(&self.menu_style)
    }

    /// The font overlays are drawn in now: the active game's own, else the global one.
    pub fn active_font(&self) -> Option<&str> {
        self.active_game().overlay_font.as_deref().or(self.overlay_font.as_deref())
    }

    /// Whether button glyphs are swapped to the Nintendo layout now: the active game's own
    /// choice, else the global one.
    pub fn active_nintendo_layout(&self) -> bool {
        self.active_game().nintendo_layout.unwrap_or(self.nintendo_layout)
    }

    /// The active profile's game, or General if it's gone.
    pub fn active_game(&self) -> &Game {
        self.game(self.active.game.as_deref())
            .filter(|g| !g.profiles.is_empty())
            .unwrap_or(&self.general)
    }

    /// The active profile, or the first one of its game (or of General) if it's gone.
    pub fn active(&self) -> Option<&Profile> {
        let game = self.active_game();
        game.profile(&self.active.profile).or(game.profiles.first())
    }

    /// Where the active profile really is, after the fallbacks of [`Config::active`].
    pub fn active_ref(&self) -> ProfileRef {
        let game = self.active_game();
        let name = self.active().map(|p| p.name.as_str()).unwrap_or_default();
        let game = (!std::ptr::eq(game, &self.general)).then_some(game.name.as_str());
        ProfileRef::new(game, name)
    }

    /// Where to go when the active profile disappears (e.g. its game was deleted): the
    /// auto-switch default if it still exists, else General's first profile.
    pub fn fallback_profile(&self) -> ProfileRef {
        let default = self.auto_switch.default_profile.clone().filter(|d| self.profile(d).is_some());
        default.unwrap_or_else(|| ProfileRef::new(None, self.general.profiles.first().map_or("", |p| &p.name)))
    }

    /// Settles which profile is active in this config, which replaces one where `previous`
    /// was: still `previous` if it exists, else this config's own `active` (where the editor
    /// says a renamed profile or game went), else the fallback (it was deleted).
    pub fn carry_active(&mut self, previous: &ProfileRef) {
        self.active = if self.profile(previous).is_some() {
            previous.clone()
        } else if self.profile(&self.active).is_some() {
            self.active.clone()
        } else {
            self.fallback_profile()
        };
    }

    /// `target`, or when it needs features outside `have`, the first profile of its game
    /// that doesn't (still `target` when none fits, or with no controller to judge by).
    pub fn usable_profile(&self, target: ProfileRef, have: Option<&[Feature]>) -> ProfileRef {
        let Some(have) = have else { return target };
        let Some(game) = self.games.iter().find(|g| Some(g.name.as_str()) == target.game.as_deref()) else { return target };
        let fits = |p: &Profile| p.usable_with(have);
        if game.profiles.iter().find(|p| p.name == target.profile).is_none_or(fits) {
            return target;
        }
        match game.profiles.iter().find(|p| fits(p)) {
            Some(p) => ProfileRef { profile: p.name.clone(), ..target },
            None => target,
        }
    }

    /// What the active profile should do now that the controllers have `have`. `fell_back` is
    /// a profile left earlier for lacking features (what it was, what replaced it).
    pub fn refit(&self, have: &[Feature], fell_back: Option<&(ProfileRef, ProfileRef)>) -> Refit {
        let active = self.active_ref();
        let Some(profile) = self.profile(&active) else { return Refit::Keep };
        if !profile.usable_with(have) {
            let missing = profile.requires.iter().copied().filter(|f| !have.contains(f)).collect();
            let mut to = self.usable_profile(active.clone(), Some(have));
            if to == active {
                to = self.fallback_profile();
            }
            return if to == active { Refit::Keep } else { Refit::Leave { missing, to } };
        }
        match fell_back {
            // Changed by hand or by a game rule since: nothing to give back.
            Some((_, instead)) if *instead != active => Refit::Forget,
            Some((wanted, _)) if self.profile(wanted).is_some_and(|p| p.usable_with(have)) => Refit::Return(wanted.clone()),
            _ => Refit::Keep,
        }
    }

    /// The next profile of the active game, wrapping around and skipping those that need
    /// features outside `have` (`None`: no controller to judge by, so none is skipped).
    pub fn next_profile(&self, have: Option<&[Feature]>) -> Option<ProfileRef> {
        let at = self.active_ref();
        let profiles = &self.active_game().profiles;
        let idx = profiles.iter().position(|p| p.name == at.profile)?;
        let usable = |p: &&Profile| have.is_none_or(|h| p.usable_with(h));
        let next = (1..profiles.len()).map(|n| &profiles[(idx + n) % profiles.len()]).find(usable)?;
        Some(ProfileRef { profile: next.name.clone(), ..at })
    }

    /// The items the active profile can use: its game's.
    pub fn scope(&self) -> Scope {
        let s = self.scope_of(self.active_game());
        Scope {
            macros: s.macros.into_iter().cloned().collect(),
            menus: s.menus.into_iter().cloned().collect(),
            info: s.info.into_iter().cloned().collect(),
            logs: s.logs.into_iter().cloned().collect(),
            layers: s.layers.into_iter().cloned().collect(),
            modes: s.modes.into_iter().map(std::borrow::Cow::into_owned).collect(),
            sounds: self.performance.sounds(self.active_game().sounds.unwrap_or(self.sounds)),
            rumble: self.active_game().rumble.unwrap_or_default(),
        }
    }

    /// The profiles of `game` that already use what opens the Guide overlay, with why.
    pub fn shortcut_conflicts(&self, game: &Game) -> Vec<(String, String)> {
        game.profiles.iter().filter_map(|p| self.shortcuts.conflict(p).map(|why| (p.name.clone(), why))).collect()
    }

    /// The built-in modes, with the shortcuts where the player put them.
    pub fn built_in_modes(&self) -> [Layer; 2] {
        [Layer::guide_with(&self.shortcuts), Layer::mouse_with(&self.shortcuts, self.mouse_mode)]
    }

    /// The built-in mode called `name`, built from the shortcuts, else `game`'s own. A game's mode
    /// can't stand in for a built-in one, even one a file put there under its name.
    pub fn mode_for<'a>(&self, game: &'a Game, name: &str) -> Option<std::borrow::Cow<'a, Mode>> {
        match shortcuts::built_in_with(&self.shortcuts, self.mouse_mode, name) {
            Some(mode) => Some(std::borrow::Cow::Owned(mode)),
            None => game.mode_named(name).map(std::borrow::Cow::Borrowed),
        }
    }

    /// What a game's profiles and items can use: its own items, and the built-in layers.
    pub fn scope_of<'a>(&'a self, game: &'a Game) -> ScopeRef<'a> {
        ScopeRef {
            macros: game.macros.iter().collect(),
            menus: game.menus.iter().collect(),
            info: game.info.iter().collect(),
            logs: game.logs.iter().collect(),
            layers: game.layers.iter().collect(),
            modes: game.modes.iter().map(std::borrow::Cow::Borrowed).chain(self.built_in_modes().map(std::borrow::Cow::Owned)).collect(),
        }
    }

    /// Finds a profile by name alone (for the command line): in the active game, then
    /// General, then the first game that has one.
    pub fn find_profile(&self, name: &str) -> Option<ProfileRef> {
        let active = self.active_ref();
        let in_active = self.game(active.game.as_deref()).and_then(|g| g.profile(name)).map(|_| active.game.clone());
        let anywhere = || self.all_games().find(|(_, g)| g.profile(name).is_some()).map(|(id, _)| id.map(str::to_string));
        in_active.or_else(anywhere).map(|game| ProfileRef { game, profile: name.to_string() })
    }
}

#[cfg(test)]
mod tests;
