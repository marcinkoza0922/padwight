//! Info overlays: grids of text with `{tokens}` that turn into controller button glyphs
//! (drawn for the kind of controller in use) and live values such as the time or CPU load.
//! The daemon resolves an [`InfoOverlay`] into an [`InfoView`] for the overlay window.

use std::{fs, path::Path, time::Instant};

#[cfg(target_os = "linux")]
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

mod catalog;
mod guide;
mod opener;
mod pad_model;
mod timers;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
use windows::{battery, cpu_times, gpu_busy, local_time, memory, wifi_strength};
#[cfg(windows)]
pub use windows::form_factor;

pub use catalog::{Entry, Fold, folds};
pub use guide::{DEFAULT_MODE, GuideView, is_guide_panel, notes_panel};
pub use opener::{OpenerLabel, set_menu_keys, set_opener, set_panel_keys};
pub use pad_model::PadModel;
pub use timers::{Timers, Toast, fade, redraw_at, seconds};
#[cfg(test)]
pub use timers::FADE_OUT;

use crate::config::{Button, CurrentInput, GestureKind, InfoOverlay, Layer, MouseButton, OverlayStyle, ScreenPosition, Stick, Trigger};

/// The `{token}` that draws `b`'s glyph, for buttons the pad has.
pub fn button_token(b: Button) -> Option<&'static str> {
    Some(match b {
        Button::South => "south",
        Button::East => "east",
        Button::West => "west",
        Button::North => "north",
        Button::LeftBumper => "lb",
        Button::RightBumper => "rb",
        Button::Select => "select",
        Button::Start => "start",
        Button::Guide => "guide",
        Button::LeftStick => "l3",
        Button::RightStick => "r3",
        Button::DpadUp => "up",
        Button::DpadDown => "down",
        Button::DpadLeft => "left",
        Button::DpadRight => "right",
        _ => return None,
    })
}

/// A cheat sheet of what `layer` changes: each input it sets, as a glyph and a short
/// description, two pairs to a row. Buttons set to do nothing are left out.
pub fn layer_sheet(layer: &Layer) -> InfoOverlay {
    let entries = layer_entries(layer);
    let rows = entries.chunks(2).map(|pair| pair.iter().flat_map(|(t, l)| [t.clone(), l.clone()]).collect()).collect();
    InfoOverlay {
        name: format!("layer {}", layer.name),
        always: true,
        on_start: None,
        linger: None,
        current_input: Default::default(),
        style: layer.indicator_style.clone(),
        title: layer.indicator_title.clone(),
        rows,
    }
}

/// What `layer` changes: each input it sets, as a glyph and a short description. Buttons set
/// to do nothing are left out.
fn layer_entries(layer: &Layer) -> Vec<(String, String)> {
    use crate::config::{ButtonAction, StickAction, TriggerAction};
    let mut entries: Vec<(String, String)> = Vec::new();
    for b in Button::ALL {
        let Some(action) = layer.buttons.get(&b).filter(|a| **a != ButtonAction::Disabled) else { continue };
        let Some(token) = button_token(b) else { continue };
        entries.push((format!("{{{token}}}"), action.summary()));
    }
    for (token, trigger) in [("lt", &layer.left_trigger), ("rt", &layer.right_trigger)] {
        let Some(trigger) = trigger else { continue };
        let label = match &trigger.action {
            TriggerAction::Button { action, .. } => action.summary(),
            TriggerAction::Gamepad(_) => "Pad trigger".into(),
            TriggerAction::Disabled => continue,
        };
        entries.push((format!("{{{token}}}"), label));
    }
    for (token, stick) in [("ls", &layer.left_stick), ("rs", &layer.right_stick)] {
        let Some(stick) = stick else { continue };
        let label = match &stick.action {
            StickAction::Mouse { .. } => "Move the mouse",
            StickAction::Scroll { .. } => "Scroll",
            _ => "Remapped",
        };
        entries.push((format!("{{{token}}}"), label.into()));
    }
    entries
}

/// The `{token}` that draws an input's glyph.
fn input_token(input: crate::config::GuideInput) -> Option<&'static str> {
    use crate::config::{GuideInput, Stick, Trigger};
    match input {
        GuideInput::Button(b) => button_token(b),
        GuideInput::Trigger(Trigger::Left) => Some("lt"),
        GuideInput::Trigger(Trigger::Right) => Some("rt"),
        GuideInput::Stick(Stick::Left) => Some("ls"),
        GuideInput::Stick(Stick::Right) => Some("rs"),
    }
}

/// Whose button names and symbols to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PadFamily {
    #[default]
    Xbox,
    PlayStation,
    Nintendo,
}

impl PadFamily {
    pub const ALL: [PadFamily; 3] = [PadFamily::Xbox, PadFamily::PlayStation, PadFamily::Nintendo];

    /// Recognizes the family from the USB/Bluetooth vendor ID, then the device name; `None`
    /// when neither gives it away, so the caller can fall back to the user's choice.
    pub fn detect(vendor: u16, name: &str) -> Option<Self> {
        match vendor {
            0x045e => return Some(PadFamily::Xbox),
            0x054c => return Some(PadFamily::PlayStation),
            0x057e => return Some(PadFamily::Nintendo),
            // Steam Deck and Steam Controller label their face buttons like Xbox pads.
            0x28de => return Some(PadFamily::Xbox),
            _ => {}
        }
        let name = name.to_lowercase();
        let has = |names: &[&str]| names.iter().any(|n| name.contains(n));
        if has(&["dualsense", "dualshock", "playstation", "sony"]) {
            Some(PadFamily::PlayStation)
        } else if has(&["nintendo", "pro controller", "joy-con"]) {
            Some(PadFamily::Nintendo)
        } else if has(&["xbox", "x-box", "microsoft"]) {
            Some(PadFamily::Xbox)
        } else {
            None
        }
    }
}

impl std::fmt::Display for PadFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PadFamily::Xbox => "Xbox",
            PadFamily::PlayStation => "PlayStation",
            PadFamily::Nintendo => "Nintendo",
        })
    }
}

/// A live value an info overlay can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    Time,
    Time12,
    Date,
    Profile,
    App,
    Title,
    Pid,
    Cpu,
    Ram,
    Gpu,
    Controller,
    Layer,
    Mode,
}

/// One of the gyro's three axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GyroAxis {
    Pitch,
    Roll,
    Yaw,
}

/// A battery's level, shown as a percentage, or with `:icon` as a gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gauge {
    SystemBattery,
    ControllerBattery,
}

/// A token drawn as an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconToken {
    /// The machine's form factor: monitor, laptop or handheld.
    Pc,
    /// The controller in use, drawn for its kind.
    Controller,
}

/// Formatting that holds from its tag to the matching closing tag (`{b}…{/b}`). A tag left open
/// holds to the end of its cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mark {
    Bold(bool),
    Italic(bool),
    Underline(bool),
    /// A light outline around the text.
    Boxed(bool),
    /// A color for the text; `None` goes back to the panel's own.
    Color(Option<[u8; 3]>),
    /// A font, by name (a bundled one, an installed family, or one the game's pack carries);
    /// `None` goes back to the panel's own.
    Font(Option<String>),
}

/// The colors `{color:name}` knows, besides `{color:#rrggbb}`.
pub const COLOR_NAMES: [(&str, [u8; 3]); 10] = [
    ("red", [0xe8, 0x5a, 0x5a]),
    ("orange", [0xf0, 0x9a, 0x3c]),
    ("yellow", [0xf2, 0xd2, 0x4a]),
    ("green", [0x5a, 0xc8, 0x6e]),
    ("cyan", [0x4a, 0xc8, 0xd8]),
    ("blue", [0x5a, 0x8c, 0xe8]),
    ("purple", [0xa4, 0x7a, 0xe8]),
    ("pink", [0xe8, 0x7a, 0xc0]),
    ("gray", [0x9a, 0xa0, 0xa8]),
    ("white", [0xff, 0xff, 0xff]),
];

/// A `{b}`-style formatting tag (`name` is what is between the braces, lower-cased and trimmed
/// of arguments), or `None` for anything else.
fn mark(name: &str, args: &[&str]) -> Option<Mark> {
    let (closing, name) = match name.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, name),
    };
    let on = !closing;
    match (name, args) {
        ("b" | "bold", []) => Some(Mark::Bold(on)),
        ("i" | "italic", []) => Some(Mark::Italic(on)),
        ("u" | "underline", []) => Some(Mark::Underline(on)),
        ("box", []) => Some(Mark::Boxed(on)),
        ("font", []) if closing => Some(Mark::Font(None)),
        ("font", [font]) if on => crate::font::valid_name(font).then(|| Mark::Font(Some((*font).to_string()))),
        ("color", []) if closing => Some(Mark::Color(None)),
        ("color", [value]) if on => {
            let value = value.to_lowercase();
            let named = COLOR_NAMES.iter().find(|(n, _)| *n == value).map(|(_, rgb)| *rgb);
            named.or_else(|| crate::config::parse_hex(&value)).map(|rgb| Mark::Color(Some(rgb)))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Button(Button),
    /// Formatting for the text after it.
    Mark(Mark),
    /// An icon, by name: a built-in one, or one the game's pack carries (`{icon:bow}`).
    Picture(String),
    /// A gyro axis's rate of turn (`{gyro:p}`, `{gyro:r}`, `{gyro:y}`).
    Gyro(GyroAxis),
    /// Where the touchpad is touched and whether it is pressed (`{trackpad}`).
    Trackpad,
    /// A face or other button, drawn as the controller the daemon presents itself as (`{pad:south}`).
    PadButton(Button),
    /// A keyboard key, by its evdev code (`{keyboard:a}`).
    Key(&'static str),
    /// A mouse button (`{mouse:leftclick}`).
    Mouse(MouseButton),
    Trigger(Trigger),
    /// The stick itself (for "move"), not its click.
    Stick(Stick),
    /// A D-pad with these arms lit (`[up, down, left, right]`): none for `{dpad}`, both arms of one
    /// axis for "change" and "scroll" hints (`{leftright}`, `{updown}`).
    DpadArms([bool; 4]),
    Stat(Stat),
    Icon(IconToken),
    Gauge { gauge: Gauge, icon: bool },
    /// Wi-Fi signal: its icon, then the percentage.
    Wifi,
    /// The inputs just pressed, as the overlay's input settings say, except for what the
    /// token itself sets: `{current_input_device_N}` for one controller, then optionally
    /// `:gap_ms:stay_ms`.
    CurrentInput { device: Option<u8>, gap_ms: Option<u64>, stay_ms: Option<u64> },
    /// A gesture's badge: `{gesture:double}` (blue ×2), `{gesture:triple}` (red ×3) or
    /// `{gesture:hold}` (a yellow box), as a row's mark says how it's done.
    Gesture(GestureKind),
}

/// The gesture a `{gesture:…}` token names.
fn gesture_kind(name: &str) -> Option<GestureKind> {
    match name.to_lowercase().as_str() {
        "double" => Some(GestureKind::DoubleTap),
        "triple" => Some(GestureKind::TripleTap),
        "hold" => Some(GestureKind::LongPress),
        _ => None,
    }
}

/// `{current_input…}`: `rest` is what follows the name, and `args` the `:`-separated timings.
fn current_input_token(rest: &str, args: &[&str]) -> Option<Token> {
    let device = match rest {
        "" => None,
        n => Some(n.strip_prefix("_device_")?.parse().ok()?),
    };
    // An argument that isn't a number leaves the whole token as text.
    let num = |i: usize| match args.get(i) {
        None => Some(None),
        Some(a) => a.parse::<u64>().ok().map(Some),
    };
    if args.len() > 2 {
        return None;
    }
    let (gap_ms, stay_ms) = (num(0)?, num(1)?);
    Some(Token::CurrentInput { device, gap_ms, stay_ms })
}

/// A token from what's between the braces: a name, and for `current_input` optional
/// `:`-separated timings.
fn token(inner: &str) -> Option<Token> {
    let mut parts = inner.split(':');
    let name = parts.next()?.trim().to_lowercase();
    let args: Vec<&str> = parts.map(str::trim).collect();
    if let Some(rest) = name.strip_prefix("current_input") {
        return current_input_token(rest, &args);
    }
    if let Some(m) = mark(&name, &args) {
        return Some(Token::Mark(m));
    }
    match (name.as_str(), args.as_slice()) {
        ("icon", [icon]) => return crate::icons::valid_name(icon).then(|| Token::Picture((*icon).to_string())),
        ("gyro", [axis]) => {
            return match axis.to_lowercase().as_str() {
                "p" | "pitch" => Some(Token::Gyro(GyroAxis::Pitch)),
                "r" | "roll" => Some(Token::Gyro(GyroAxis::Roll)),
                "y" | "yaw" => Some(Token::Gyro(GyroAxis::Yaw)),
                _ => None,
            };
        }
        ("gesture", [kind]) => return gesture_kind(kind).map(Token::Gesture),
        _ => {}
    }
    let gauge = match name.as_str() {
        "system_battery" => Some(Gauge::SystemBattery),
        "controller_battery" => Some(Gauge::ControllerBattery),
        _ => None,
    };
    if let Some(gauge) = gauge {
        let icon = match args.as_slice() {
            [] => false,
            ["icon"] => true,
            _ => return None,
        };
        return Some(Token::Gauge { gauge, icon });
    }
    if name == "wifi" && args.is_empty() {
        return Some(Token::Wifi);
    }
    if name == "controller" && args == ["name"] {
        return Some(Token::Stat(Stat::Controller));
    }
    match (name.as_str(), args.as_slice()) {
        ("pad", [button]) => return pad_button(button).map(Token::PadButton),
        ("keyboard", [key]) => return crate::keyboard::code_for_token(&key.to_lowercase()).map(Token::Key),
        ("mouse", [button]) => return mouse_button(button).map(Token::Mouse),
        _ => {}
    }
    if !args.is_empty() {
        return None;
    }
    if let Some(b) = opener::menu_key(&name).or_else(|| opener::panel_key(&name)) {
        return Some(Token::Button(b));
    }
    plain_token(&name)
}

/// The buttons a `{pad:…}` token can name: the ones with a button token.
const PAD_BUTTONS: [Button; 15] = [
    Button::South,
    Button::East,
    Button::West,
    Button::North,
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

fn pad_button(name: &str) -> Option<Button> {
    let name = name.to_lowercase();
    PAD_BUTTONS.into_iter().find(|b| button_token(*b) == Some(name.as_str()))
}

/// The `{mouse:…}` token name of a mouse button.
pub fn mouse_token(m: MouseButton) -> &'static str {
    match m {
        MouseButton::Left => "leftclick",
        MouseButton::Right => "rightclick",
        MouseButton::Middle => "middleclick",
        MouseButton::Back => "backclick",
        MouseButton::Forward => "forwardclick",
    }
}

fn mouse_button(name: &str) -> Option<MouseButton> {
    let name = name.to_lowercase();
    MouseButton::ALL.into_iter().find(|m| mouse_token(*m) == name)
}

/// A token with no arguments.
fn plain_token(name: &str) -> Option<Token> {
    Some(match name {
        "trackpad" | "touchpad" => Token::Trackpad,
        "pc" => Token::Icon(IconToken::Pc),
        "controller" => Token::Icon(IconToken::Controller),
        "south" => Token::Button(Button::South),
        "east" => Token::Button(Button::East),
        "west" => Token::Button(Button::West),
        "north" => Token::Button(Button::North),
        "lb" => Token::Button(Button::LeftBumper),
        "rb" => Token::Button(Button::RightBumper),
        "lt" => Token::Trigger(Trigger::Left),
        "rt" => Token::Trigger(Trigger::Right),
        "select" => Token::Button(Button::Select),
        "start" => Token::Button(Button::Start),
        "guide" => Token::Button(Button::Guide),
        "ls" => Token::Stick(Stick::Left),
        "rs" => Token::Stick(Stick::Right),
        "l3" => Token::Button(Button::LeftStick),
        "r3" => Token::Button(Button::RightStick),
        "dpad" => Token::DpadArms([false; 4]),
        "up" => Token::Button(Button::DpadUp),
        "down" => Token::Button(Button::DpadDown),
        "left" => Token::Button(Button::DpadLeft),
        "right" => Token::Button(Button::DpadRight),
        "leftright" => Token::DpadArms([false, false, true, true]),
        "updown" => Token::DpadArms([true, true, false, false]),
        "time" => Token::Stat(Stat::Time),
        "time12" => Token::Stat(Stat::Time12),
        "date" => Token::Stat(Stat::Date),
        "profile" => Token::Stat(Stat::Profile),
        "layer" => Token::Stat(Stat::Layer),
        "mode" => Token::Stat(Stat::Mode),
        "app" => Token::Stat(Stat::App),
        "title" => Token::Stat(Stat::Title),
        "pid" => Token::Stat(Stat::Pid),
        "cpu" => Token::Stat(Stat::Cpu),
        "ram" => Token::Stat(Stat::Ram),
        "gpu" => Token::Stat(Stat::Gpu),
        _ => return None,
    })
}

/// A piece of a cell: plain text, or a button glyph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Segment {
    Text(String),
    /// `fill` is the button's own color (e.g. Xbox A green); `None` draws it neutral.
    Glyph { label: String, fill: Option<[u8; 3]>, round: bool },
    /// A cross-shaped D-pad with its lit arms, `[up, down, left, right]` (two for a diagonal).
    Dpad([bool; 4]),
    /// A drawn icon: the form factor, the controller's kind, or a gauge.
    Icon(Icon),
    /// An icon by name (`{icon:bow}`): one built in, or one of the game's pack.
    Picture(String),
    /// Formatting for the segments after it, up to its closing tag.
    Mark(Mark),
    /// A bar filled from 0 to 1, such as how long is left to hold a button.
    Bar(f32),
}

/// What an icon shows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Icon {
    Form(FormFactor),
    Controller(PadFamily),
    Battery(Charge),
    /// Wi-Fi signal, as a percentage.
    Wifi(u8),
}

/// A battery's charge. `estimated` when the driver gives only a coarse level, which is turned
/// into a rough percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Charge {
    pub percent: u8,
    pub estimated: bool,
}

/// What kind of machine this is, from its DMI data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FormFactor {
    #[default]
    Desktop,
    Laptop,
    Handheld,
}

/// What the overlay window draws for one info overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfoView {
    /// The overlay's name (or what it is, for the toast and recording badge), so it keeps its
    /// place while the overlay window animates it.
    #[serde(default)]
    pub name: String,
    pub style: OverlayStyle,
    /// A heading above the grid.
    #[serde(default)]
    pub title: Option<Vec<Segment>>,
    pub rows: Vec<Vec<Vec<Segment>>>,
    /// 1 normally, falling to 0 as it fades out (see [`fade`]).
    #[serde(default = "opaque")]
    pub opacity: f32,
}

impl crate::motion::Tracked for InfoView {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn kind(&self) -> crate::motion::OverlayKind {
        crate::motion::OverlayKind::Info
    }
}

fn opaque() -> f32 {
    1.0
}

/// The colors of the face buttons and of the gesture badges, which share them.
const GREEN: [u8; 3] = [0x3c, 0xa0, 0x3c];
const RED: [u8; 3] = [0xc8, 0x3c, 0x3c];
const BLUE: [u8; 3] = [0x2f, 0x6c, 0xc8];
const YELLOW: [u8; 3] = [0xc8, 0xa0, 0x1e];


/// Splits a cell into text and tokens. Unknown `{words}` stay as they are.
pub fn parse(cell: &str) -> Vec<Result<String, Token>> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = cell;
    while let Some(open) = rest.find('{') {
        let Some(len) = rest[open..].find('}') else { break };
        let inner = &rest[open + 1..open + len];
        text.push_str(&rest[..open]);
        match token(inner) {
            Some(t) => {
                if !text.is_empty() {
                    parts.push(Ok(std::mem::take(&mut text)));
                }
                parts.push(Err(t));
            }
            None => text.push_str(&rest[open..=open + len]),
        }
        rest = &rest[open + len + 1..];
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(Ok(text));
    }
    opener::opener().expand(parts)
}

/// What each `{current_input}` cell of an overlay follows: the overlay's input settings,
/// with whatever its token sets itself.
pub fn input_tokens(overlay: &InfoOverlay) -> Vec<CurrentInput> {
    overlay
        .rows
        .iter()
        .flatten()
        .flat_map(|cell| parse(cell))
        .filter_map(|p| match p {
            Err(Token::CurrentInput { device, gap_ms, stay_ms }) => Some(token_settings(&overlay.current_input, device, gap_ms, stay_ms)),
            _ => None,
        })
        .collect()
}

fn token_settings(base: &CurrentInput, device: Option<u8>, gap_ms: Option<u64>, stay_ms: Option<u64>) -> CurrentInput {
    let mut s = base.clone();
    s.tracking.device = device.or(s.tracking.device);
    s.tracking.gap_ms = gap_ms.unwrap_or(s.tracking.gap_ms);
    s.stay_ms = stay_ms.unwrap_or(s.stay_ms);
    s
}

/// Whether an overlay shows something that moves faster than the clock does (the gyro's rates or
/// where the touchpad is touched), so it is refreshed many times a second.
pub fn is_fast(overlay: &InfoOverlay) -> bool {
    overlay.rows.iter().flatten().any(|cell| parse(cell).iter().any(|p| matches!(p, Err(Token::Gyro(_) | Token::Trackpad))))
}

/// Whether an overlay shows anything that changes over time (so it needs refreshing).
pub fn is_live(overlay: &InfoOverlay) -> bool {
    overlay.rows.iter().flatten().any(|cell| parse(cell).iter().any(|p| matches!(p, Err(Token::Stat(_) | Token::Gauge { .. } | Token::Gyro(_) | Token::Trackpad))))
}

pub fn glyph(label: &str, fill: Option<[u8; 3]>, round: bool) -> Segment {
    Segment::Glyph { label: label.to_string(), fill, round }
}

/// A gesture's badge: a blue ×2 or a red ×3 in a circle, and "hold" in a yellow rounded box.
pub fn gesture_glyph(kind: GestureKind) -> Segment {
    match kind {
        GestureKind::DoubleTap => glyph("×2", Some(BLUE), true),
        GestureKind::TripleTap => glyph("×3", Some(RED), true),
        GestureKind::LongPress => glyph("hold", Some(YELLOW), false),
    }
}

/// A keyboard key as a keycap: its label, in a slate color that tells it from a controller's.
pub fn key_glyph(code: &str) -> Segment {
    const KEY: [u8; 3] = [0x4a, 0x52, 0x5c];
    glyph(&crate::keyboard::label(code), Some(KEY), false)
}

/// The teal mouse outputs are drawn in, which tells them from a key or a controller's button.
const MOUSE: [u8; 3] = [0x2f, 0x7a, 0x7a];

/// A mouse button, in the mouse teal.
pub fn mouse_glyph(m: MouseButton) -> Segment {
    glyph(&format!("{m} click"), Some(MOUSE), false)
}

/// A turn of the mouse wheel, in the mouse buttons' teal.
pub fn wheel_glyph(d: crate::config::WheelDirection) -> Segment {
    glyph(&d.to_string(), Some(MOUSE), false)
}

/// The badge shown while the screen is being recorded: a red "● REC" in the top left.
pub fn recording_view() -> InfoView {
    InfoView {
        name: "recording".into(),
        style: OverlayStyle { position: ScreenPosition::TopLeft, ..OverlayStyle::indicator() },
        title: None,
        rows: vec![vec![vec![glyph("● REC", Some(RED), false)]]],
        opacity: 1.0,
    }
}

/// The face button whose glyph `b` shows once the Nintendo layout swaps A with B and X with Y:
/// the bottom button shows B, not A. Other buttons are unchanged.
pub fn nintendo_face(b: Button) -> Button {
    match b {
        Button::South => Button::East,
        Button::East => Button::South,
        Button::West => Button::North,
        Button::North => Button::West,
        other => other,
    }
}

/// How button glyphs are drawn: whose labels, and whether the face buttons use the Nintendo layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Glyphs {
    pub family: PadFamily,
    pub nintendo_layout: bool,
}

/// `text` with its controller tokens (`{south}`, `{lt}`, `{ls}`, `{dpad}`, `{keyboard:a}`…) as
/// glyphs in `glyphs`' labels, for hints and menu labels. Other tokens stay as written.
pub fn glyph_text(text: &str, glyphs: Glyphs) -> Vec<Segment> {
    let Glyphs { family, nintendo_layout } = glyphs;
    let mut segments: Vec<Segment> = Vec::new();
    for part in parse(text) {
        let segment = match part {
            Ok(t) => Segment::Text(t),
            Err(Token::Button(b) | Token::PadButton(b)) => button_glyph(b, family, nintendo_layout),
            Err(Token::Key(code)) => key_glyph(code),
            Err(Token::Mouse(m)) => mouse_glyph(m),
            Err(Token::Trigger(t)) => trigger_glyph(t, family),
            Err(Token::Gesture(kind)) => gesture_glyph(kind),
            Err(Token::Stick(Stick::Left)) => glyph("LS", None, true),
            Err(Token::Stick(Stick::Right)) => glyph("RS", None, true),
            Err(Token::DpadArms(lit)) => Segment::Dpad(lit),
            Err(Token::Picture(name)) => Segment::Picture(name),
            Err(Token::Mark(m)) => Segment::Mark(m),
            Err(_) => continue,
        };
        match (segments.last_mut(), segment) {
            (Some(Segment::Text(prev)), Segment::Text(t)) => prev.push_str(&t),
            (_, segment) => segments.push(segment),
        }
    }
    segments
}

/// The `{token}` text that draws `b` in a hint, or its plain badge for a button without one.
pub fn button_hint(b: Button) -> String {
    button_token(b).map_or_else(|| crate::menu::button_badge(b).to_string(), |t| format!("{{{t}}}"))
}

/// A button's glyph as the given family labels it. Face buttons are round and, for Xbox
/// and PlayStation, in their usual colors. `swapped` draws the Nintendo layout's labels.
pub fn button_glyph(b: Button, family: PadFamily, swapped: bool) -> Segment {
    use PadFamily::*;
    let b = if swapped { nintendo_face(b) } else { b };
    const PS_BLUE: [u8; 3] = [0x5a, 0x8c, 0xdc];
    const PS_RED: [u8; 3] = [0xdc, 0x5a, 0x5a];
    const PS_PINK: [u8; 3] = [0xc8, 0x6e, 0xc8];
    const PS_GREEN: [u8; 3] = [0x32, 0xb4, 0x96];
    let face = |xbox: (&str, [u8; 3]), ps: (&str, [u8; 3]), nintendo: &str| match family {
        Xbox => glyph(xbox.0, Some(xbox.1), true),
        PlayStation => glyph(ps.0, Some(ps.1), true),
        Nintendo => glyph(nintendo, None, true),
    };
    let named = |xbox: &str, ps: &str, nintendo: &str| {
        glyph(
            match family {
                Xbox => xbox,
                PlayStation => ps,
                Nintendo => nintendo,
            },
            None,
            false,
        )
    };
    match b {
        Button::South => face(("A", GREEN), ("✕", PS_BLUE), "B"),
        Button::East => face(("B", RED), ("○", PS_RED), "A"),
        Button::West => face(("X", BLUE), ("□", PS_PINK), "Y"),
        Button::North => face(("Y", YELLOW), ("△", PS_GREEN), "X"),
        Button::LeftBumper => named("LB", "L1", "L"),
        Button::RightBumper => named("RB", "R1", "R"),
        Button::Select => named("View", "Share", "−"),
        Button::Start => named("Menu", "Options", "+"),
        Button::Guide => named("Guide", "PS", "Home"),
        Button::DpadUp => Segment::Dpad([true, false, false, false]),
        Button::DpadDown => Segment::Dpad([false, true, false, false]),
        Button::DpadLeft => Segment::Dpad([false, false, true, false]),
        Button::DpadRight => Segment::Dpad([false, false, false, true]),
        other => glyph(crate::menu::button_badge(other), None, false),
    }
}

/// A button's name as the family's controller prints it, with its position or generic name kept
/// where that helps. Buttons without a family-specific name keep their generic one.
pub fn button_name(b: Button, family: PadFamily) -> String {
    use PadFamily::*;
    let named = |xbox: &'static str, ps: &'static str, nintendo: &'static str| -> String {
        match family {
            Xbox => xbox,
            PlayStation => ps,
            Nintendo => nintendo,
        }
        .into()
    };
    match b {
        Button::South => named("A (South)", "Cross (South)", "B (South)"),
        Button::East => named("B (East)", "Circle (East)", "A (East)"),
        Button::West => named("X (West)", "Square (West)", "Y (West)"),
        Button::North => named("Y (North)", "Triangle (North)", "X (North)"),
        Button::LeftBumper => named("Left Bumper (LB)", "Left Bumper (L1)", "Left Bumper (L)"),
        Button::RightBumper => named("Right Bumper (RB)", "Right Bumper (R1)", "Right Bumper (R)"),
        Button::Select => named("Back (View)", "Share", "Minus (−)"),
        Button::Start => named("Start (Menu)", "Options", "Plus (+)"),
        Button::Guide => named("Guide (Xbox)", "PS", "Home"),
        other => other.to_string(),
    }
}

pub fn trigger_glyph(t: Trigger, family: PadFamily) -> Segment {
    let label = match (t, family) {
        (Trigger::Left, PadFamily::Xbox) => "LT",
        (Trigger::Right, PadFamily::Xbox) => "RT",
        (Trigger::Left, PadFamily::PlayStation) => "L2",
        (Trigger::Right, PadFamily::PlayStation) => "R2",
        (Trigger::Left, PadFamily::Nintendo) => "ZL",
        (Trigger::Right, PadFamily::Nintendo) => "ZR",
    };
    glyph(label, None, false)
}

/// Everything live values are read from.
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub profile: String,
    pub app: String,
    pub title: String,
    pub pid: u32,
    pub controller: String,
    /// Active layers, oldest first.
    pub layers: Vec<String>,
    /// The mode switched to; `None` is the default mode.
    pub mode: Option<String>,
    pub family: PadFamily,
    /// The family of the controller the daemon presents itself as, for `{pad:…}` tokens.
    pub pad_family: PadFamily,
    /// Face buttons drawn with the Nintendo layout's labels (see [`nintendo_face`]).
    pub nintendo_layout: bool,
    pub system: SystemStats,
    /// Charge of the controller in use, where it reports one.
    pub controller_battery: Option<Charge>,
    pub form: FormFactor,
    /// Recent input, for `{current_input}`.
    pub inputs: crate::inputlog::Inputs,
    /// The controller's turning rates in degrees per second (pitch, yaw, roll), if it has a gyro.
    pub gyro: Option<[f32; 3]>,
    /// The controller's touchpad, if it has one.
    pub touchpad: Option<Touchpad>,
}

/// A touchpad as it is now.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Touchpad {
    /// Each finger down, `(x, y)` from 0 to 1 across the pad from its top left corner.
    pub touches: Vec<(f32, f32)>,
    pub pressed: bool,
}

impl Live {
    /// Made-up values for the settings preview.
    pub fn sample(family: PadFamily) -> Self {
        Live {
            profile: "My game".into(),
            app: "game.exe".into(),
            title: "My Game".into(),
            pid: 4242,
            controller: format!("{family} controller"),
            layers: vec!["Hotkeys".into()],
            mode: Some("Driving".into()),
            family,
            pad_family: family,
            nintendo_layout: false,
            system: SystemStats {
                cpu: Some(23.0),
                ram: Some((7.4, 31.2)),
                gpu: Some(61.0),
                battery: Some(Charge { percent: 78, estimated: false }),
                wifi: Some(72),
            },
            controller_battery: Some(Charge { percent: 64, estimated: true }),
            form: FormFactor::Laptop,
            inputs: crate::inputlog::Inputs::sample(),
            gyro: Some([12.0, -35.0, 3.0]),
            touchpad: Some(Touchpad { touches: vec![(0.42, 0.58)], pressed: true }),
        }
    }

    /// The same values, with face buttons in the Nintendo layout or not.
    pub fn with_layout(self, nintendo_layout: bool) -> Self {
        Live { nintendo_layout, ..self }
    }
}

fn stat(s: Stat, live: &Live) -> String {
    let or_dash = |v: &str| if v.is_empty() { "—".to_string() } else { v.to_string() };
    match s {
        Stat::Time => local_time("%H:%M"),
        Stat::Time12 => local_time("%-I:%M %p"),
        Stat::Date => local_time("%a %-d %b %Y"),
        Stat::Profile => or_dash(&live.profile),
        Stat::App => or_dash(&live.app),
        Stat::Title => or_dash(&live.title),
        Stat::Pid if live.pid == 0 => "—".into(),
        Stat::Pid => live.pid.to_string(),
        Stat::Cpu => live.system.cpu.map_or("n/a".into(), |c| format!("{c:.0}%")),
        Stat::Ram => live.system.ram.map_or("n/a".into(), |(used, total)| format!("{used:.1}/{total:.1} GB")),
        Stat::Gpu => live.system.gpu.map_or("n/a".into(), |g| format!("{g:.0}%")),
        Stat::Controller => or_dash(&live.controller),
        Stat::Layer => or_dash(&live.layers.join(" + ")),
        Stat::Mode => live.mode.clone().unwrap_or_else(|| DEFAULT_MODE.into()),
    }
}

/// Whether `overlay` has nothing written in it yet: no title and no cell with anything in it.
/// The overlays drawn in game leave it out.
pub fn is_blank(overlay: &InfoOverlay) -> bool {
    overlay.title.as_ref().is_none_or(|t| t.trim().is_empty()) && overlay.rows.iter().flatten().all(|c| c.trim().is_empty())
}

/// Turns an overlay's cells into text and glyphs, with live values filled in. A blank overlay
/// shows its name over "No rows yet", so its preview has something to find it by.
pub fn resolve(overlay: &InfoOverlay, live: &Live) -> InfoView {
    if is_blank(overlay) {
        let name = if overlay.name.trim().is_empty() { "Info overlay" } else { &overlay.name };
        let note = vec![Segment::Mark(Mark::Italic(true)), Segment::Text("No rows yet".into()), Segment::Mark(Mark::Italic(false))];
        let title = Some(vec![Segment::Text(name.into())]);
        return InfoView { name: overlay.name.clone(), style: overlay.style.clone(), title, rows: vec![vec![note]], opacity: 1.0 };
    }
    let rows = overlay.rows.iter().map(|row| row.iter().map(|cell| cell_segments(&overlay.current_input, cell, live)).collect()).collect();
    let title = overlay.title.as_ref().filter(|t| !t.trim().is_empty()).map(|t| cell_segments(&overlay.current_input, t, live));
    InfoView { name: overlay.name.clone(), style: overlay.style.clone(), title, rows, opacity: 1.0 }
}

/// What `cell` shows on a controller of `family`, with sample values for the live ones: for the
/// previews in the token picker.
pub fn preview(cell: &str, family: PadFamily) -> Vec<Segment> {
    cell_segments(&CurrentInput::default(), cell, &Live::sample(family))
}

/// One cell's text and glyphs.
fn cell_segments(input: &CurrentInput, cell: &str, live: &Live) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    for part in parse(cell) {
        let segment = match part {
            Err(Token::CurrentInput { device, gap_ms, stay_ms }) => {
                let settings = token_settings(input, device, gap_ms, stay_ms);
                segments.extend(live.inputs.current(&settings, live.family, live.nintendo_layout));
                continue;
            }
            Ok(text) => Segment::Text(text),
            Err(Token::Button(b)) => button_glyph(b, live.family, live.nintendo_layout),
            Err(Token::PadButton(b)) => button_glyph(b, live.pad_family, live.nintendo_layout),
            Err(Token::Key(code)) => key_glyph(code),
            Err(Token::Mouse(m)) => mouse_glyph(m),
            Err(Token::Trigger(t)) => trigger_glyph(t, live.family),
            Err(Token::Gesture(kind)) => gesture_glyph(kind),
            Err(Token::Stick(Stick::Left)) => glyph("LS", None, true),
            Err(Token::Stick(Stick::Right)) => glyph("RS", None, true),
            Err(Token::DpadArms(lit)) => Segment::Dpad(lit),
            Err(Token::Stat(s)) => Segment::Text(stat(s, live)),
            Err(Token::Picture(name)) => Segment::Picture(name),
            Err(Token::Mark(m)) => Segment::Mark(m),
            Err(Token::Gyro(axis)) => Segment::Text(gyro_text(axis, live.gyro)),
            Err(Token::Trackpad) => Segment::Text(trackpad_text(live.touchpad.as_ref())),
            Err(Token::Icon(IconToken::Pc)) => Segment::Icon(Icon::Form(live.form)),
            Err(Token::Icon(IconToken::Controller)) => Segment::Icon(Icon::Controller(live.family)),
            Err(Token::Gauge { gauge, icon }) => match (gauge_reading(gauge, live), icon) {
                (Some(reading), true) => Segment::Icon(reading),
                (Some(reading), false) => Segment::Text(reading_text(&reading)),
                (None, _) => Segment::Text("n/a".into()),
            },
            Err(Token::Wifi) => {
                // Without a signal the bars stay dim, next to "n/a".
                let signal = live.system.wifi;
                segments.push(Segment::Icon(Icon::Wifi(signal.unwrap_or(0))));
                Segment::Text(signal.map_or("n/a".into(), |p| format!("{p}%")))
            }
        };
        // Live values join the text around them.
        match (segments.last_mut(), segment) {
            (Some(Segment::Text(prev)), Segment::Text(t)) => prev.push_str(&t),
            (_, segment) => segments.push(segment),
        }
    }
    segments
}

/// A gyro axis's rate of turn: "+12°/s", or "n/a" without a gyro.
fn gyro_text(axis: GyroAxis, gyro: Option<[f32; 3]>) -> String {
    // The gyro's own order is pitch, yaw, roll.
    let index = match axis {
        GyroAxis::Pitch => 0,
        GyroAxis::Yaw => 1,
        GyroAxis::Roll => 2,
    };
    gyro.map_or("n/a".into(), |g| {
        let rate = g[index].round();
        // A rate that rounds to nothing shows no sign, so it doesn't flicker between + and −.
        if rate == 0.0 { "0°/s".into() } else { format!("{rate:+.0}°/s") }
    })
}

/// Where the touchpad is touched, across it and down it from 0 to 1, and "pressed" while it is
/// clicked: "(0.42, 0.58) pressed". "n/a" for a controller without a touchpad.
fn trackpad_text(pad: Option<&Touchpad>) -> String {
    let Some(pad) = pad else { return "n/a".into() };
    let mut text = if pad.touches.is_empty() {
        "(–, –)".to_string()
    } else {
        pad.touches.iter().map(|(x, y)| format!("({x:.2}, {y:.2})")).collect::<Vec<_>>().join(" ")
    };
    if pad.pressed {
        text.push_str(" pressed");
    }
    text
}

/// A gauge's reading as an icon, if there is one.
fn gauge_reading(gauge: Gauge, live: &Live) -> Option<Icon> {
    match gauge {
        Gauge::SystemBattery => live.system.battery.map(Icon::Battery),
        Gauge::ControllerBattery => live.controller_battery.map(Icon::Battery),
    }
}

/// A gauge's reading as text: "78%", or "~75%" when estimated.
fn reading_text(reading: &Icon) -> String {
    match reading {
        Icon::Battery(Charge { percent, estimated: true }) => format!("~{percent}%"),
        Icon::Battery(Charge { percent, .. }) | Icon::Wifi(percent) => format!("{percent}%"),
        Icon::Form(_) | Icon::Controller(_) => String::new(),
    }
}

/// Formats the local time with a `strftime` pattern.
#[cfg(target_os = "linux")]
fn local_time(pattern: &str) -> String {
    let Ok(pattern) = std::ffi::CString::new(pattern) else { return String::new() };
    // SAFETY: localtime_r and strftime write into the buffers given; tm is plain data.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return String::new();
        }
        let mut buf = [0u8; 64];
        let len = libc::strftime(buf.as_mut_ptr().cast(), buf.len(), pattern.as_ptr(), &tm);
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }
}

/// System load, as of the last sample.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemStats {
    /// Percent busy across all cores.
    pub cpu: Option<f32>,
    /// Used and total, in GB.
    pub ram: Option<(f32, f32)>,
    /// Percent busy, where the driver reports it (amdgpu).
    pub gpu: Option<f32>,
    /// Charge of the first system battery, where there is one.
    pub battery: Option<Charge>,
    /// Wi-Fi signal of the first wireless interface, as a percentage.
    pub wifi: Option<u8>,
}

/// Samples CPU, memory and GPU load from /proc and /sys. CPU load is the change since the
/// previous sample.
#[derive(Default)]
pub struct Sampler {
    last_cpu: Option<(u64, u64)>,
    pub stats: SystemStats,
    pub sampled_at: Option<Instant>,
}

impl Sampler {
    pub fn sample(&mut self) {
        let cpu = cpu_times();
        self.stats.cpu = match (self.last_cpu, cpu) {
            (Some((busy0, total0)), Some((busy1, total1))) if total1 > total0 => {
                Some((busy1 - busy0) as f32 * 100.0 / (total1 - total0) as f32)
            }
            _ => self.stats.cpu,
        };
        self.last_cpu = cpu;
        self.stats.ram = memory();
        self.stats.gpu = gpu_busy();
        self.stats.battery = battery();
        self.stats.wifi = wifi_strength();
        self.sampled_at = Some(Instant::now());
    }
}

/// Busy and total jiffies from the first line of /proc/stat.
#[cfg(target_os = "linux")]
fn cpu_times() -> Option<(u64, u64)> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let fields: Vec<u64> = stat.lines().next()?.split_whitespace().skip(1).filter_map(|f| f.parse().ok()).collect();
    let total: u64 = fields.iter().take(8).sum();
    // idle + iowait
    let idle = fields.get(3)? + fields.get(4).copied().unwrap_or(0);
    Some((total - idle, total))
}

#[cfg(target_os = "linux")]
fn memory() -> Option<(f32, f32)> {
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    let field = |name: &str| -> Option<f32> {
        let line = info.lines().find(|l| l.starts_with(name))?;
        line.split_whitespace().nth(1)?.parse::<f32>().ok()
    };
    let total = field("MemTotal:")?;
    let available = field("MemAvailable:")?;
    let gb = 1024.0 * 1024.0;
    Some(((total - available) / gb, total / gb))
}

#[cfg(target_os = "linux")]
fn gpu_busy() -> Option<f32> {
    let cards = fs::read_dir("/sys/class/drm").ok()?;
    cards
        .flatten()
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("card") && !n.contains('-')))
        .find_map(|e| fs::read_to_string(e.path().join("device/gpu_busy_percent")).ok())
        .and_then(|v| v.trim().parse().ok())
}

/// The charge of the first system battery (a laptop's or handheld's) under
/// /sys/class/power_supply. Controllers' batteries are skipped: they are scoped "Device".
#[cfg(target_os = "linux")]
fn battery() -> Option<Charge> {
    let supplies = fs::read_dir("/sys/class/power_supply").ok()?;
    supplies.flatten().map(|e| e.path()).find_map(|dir| {
        let scope = fs::read_to_string(dir.join("scope")).unwrap_or_default();
        (scope.trim() != "Device").then_some(())?;
        battery_charge(&dir)
    })
}

/// The charge of the battery a controller reports, from its HID device's power_supply entries
/// (where the kernel's drivers register them).
pub fn controller_battery(hid: &Path) -> Option<Charge> {
    let supplies = fs::read_dir(hid.join("power_supply")).ok()?;
    supplies.flatten().map(|e| e.path()).find_map(|dir| battery_charge(&dir))
}

/// A power supply's charge, if it is a battery. Drivers that report only a coarse level (the
/// Switch controllers' say "High", say) give an estimate.
fn battery_charge(dir: &Path) -> Option<Charge> {
    let kind = fs::read_to_string(dir.join("type")).ok()?;
    (kind.trim() == "Battery").then_some(())?;
    if let Some(percent) = fs::read_to_string(dir.join("capacity")).ok().and_then(|c| c.trim().parse().ok()) {
        return Some(Charge { percent, estimated: false });
    }
    let level = fs::read_to_string(dir.join("capacity_level")).ok()?;
    let percent = match level.trim() {
        "Full" => 100,
        "High" => 75,
        "Normal" | "Medium" => 50,
        "Low" => 25,
        "Critical" => 10,
        _ => return None,
    };
    Some(Charge { percent, estimated: true })
}

/// Signal of the first wireless interface, from /proc/net/wireless.
#[cfg(target_os = "linux")]
fn wifi_strength() -> Option<u8> {
    parse_wireless(&fs::read_to_string("/proc/net/wireless").ok()?)
}

/// Link quality (out of 70, as drivers report it) of the first interface listed.
#[cfg(any(target_os = "linux", test))]
fn parse_wireless(text: &str) -> Option<u8> {
    let (_, rest) = text.lines().skip(2).find_map(|line| line.split_once(':'))?;
    let link: f32 = rest.split_whitespace().nth(1)?.trim_end_matches('.').parse().ok()?;
    Some((link * 100.0 / 70.0).round().min(100.0) as u8)
}

/// The form factor from DMI data: product and vendor name the handhelds whose chassis type
/// doesn't say so; the chassis type (SMBIOS) says the rest.
#[cfg(target_os = "linux")]
pub fn form_factor() -> FormFactor {
    static FORM: OnceLock<FormFactor> = OnceLock::new();
    *FORM.get_or_init(|| {
        let dmi = |field: &str| fs::read_to_string(Path::new("/sys/class/dmi/id").join(field)).unwrap_or_default();
        detect_form(dmi("product_name").trim(), dmi("sys_vendor").trim(), dmi("chassis_type").trim())
    })
}

fn detect_form(product: &str, vendor: &str, chassis: &str) -> FormFactor {
    let handheld = (vendor == "Valve" && (product.starts_with("Jupiter") || product.starts_with("Galileo")))
        || (vendor.starts_with("ASUS") && product.starts_with("RC7"))
        || (vendor == "LENOVO" && product == "83E1");
    match chassis.parse::<u8>() {
        _ if handheld => FormFactor::Handheld,
        // Handheld PC, then portable, laptop, notebook, sub-notebook, convertible, detachable.
        Ok(11) => FormFactor::Handheld,
        Ok(8 | 9 | 10 | 14 | 31 | 32) => FormFactor::Laptop,
        _ => FormFactor::Desktop,
    }
}

#[cfg(test)]
mod tests;
