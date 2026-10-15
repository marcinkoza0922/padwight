//! The system shortcuts the player can move: what opens the Guide overlay, the Guide mode's
//! shortcuts and mouse mode's keys. They're app-wide, never per setup, so they live in
//! `Config::shortcuts`, and the built-in modes are built from them (see [`Layer::guide_with`]).

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use super::{
    Button, ButtonAction, Combo, GUIDE_MODE, GestureKind, Layer, MOUSE_MODE, MouseButton, MouseModeExtras, Profile,
    TouchpadConfig,
};

/// The mode name the opener's gesture or combo is injected with into a profile. The engine reads
/// it as "toggle the Guide overlay" (and "leave mouse mode" while that's on). It's never stored.
pub const OPEN_GUIDE: &str = "\u{1}open-guide";

/// An input a shortcut sits on: one button, a gesture of one button, or a combo of buttons.
/// No buttons means the shortcut is switched off.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ShortcutInput {
    pub buttons: Vec<Button>,
    /// A double tap, triple tap or long press of the one button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gesture: Option<GestureKind>,
}

impl ShortcutInput {
    /// The buttons a shortcut can be put on. Guide is the Guide mode's own (see [`Shortcut`]).
    pub fn choices() -> Vec<Button> {
        Button::ALL.into_iter().filter(|b| *b != Button::Guide).chain(Button::PADDLES).collect()
    }

    pub fn button(b: Button) -> Self {
        ShortcutInput { buttons: vec![b], gesture: None }
    }

    pub fn gesture(b: Button, kind: GestureKind) -> Self {
        ShortcutInput { buttons: vec![b], gesture: Some(kind) }
    }

    pub fn combo(buttons: &[Button]) -> Self {
        ShortcutInput { buttons: super::combo_key(buttons), gesture: None }
    }

    /// Without repeated buttons, and a gesture only on one button.
    pub fn tidied(mut self) -> Self {
        let mut seen: Vec<Button> = Vec::new();
        self.buttons.retain(|b| {
            let new = !seen.contains(b);
            seen.push(*b);
            new
        });
        if self.gesture.is_some() {
            self.buttons.truncate(1);
        }
        self
    }

    pub fn is_off(&self) -> bool {
        self.buttons.is_empty()
    }

    pub fn is_combo(&self) -> bool {
        self.buttons.len() > 1
    }

    pub fn uses(&self, b: Button) -> bool {
        self.buttons.contains(&b)
    }

    /// Whether the two can't both work: the same input, a combo that takes a button the other
    /// uses on its own, or combos where one contains the other. A button and a gesture of it
    /// coexist, and so do gestures of different kinds.
    pub fn overlaps(&self, other: &ShortcutInput) -> bool {
        if self.is_off() || other.is_off() {
            return false;
        }
        let (a, b) = (super::combo_key(&self.buttons), super::combo_key(&other.buttons));
        if self.is_combo() || other.is_combo() {
            return a.iter().all(|x| b.contains(x)) || b.iter().all(|x| a.contains(x));
        }
        a == b && self.gesture == other.gesture
    }

    /// "Start", "Double tap Select", "Start + Select".
    pub fn describe(&self) -> String {
        let names: Vec<&str> = self.buttons.iter().map(|b| short_name(*b)).collect();
        match (self.gesture, names.as_slice()) {
            (_, []) => "Not set".into(),
            (Some(kind), names) => format!("{kind} {}", names.join(" + ")),
            (None, names) => names.join(" + "),
        }
    }

    /// Puts `action` on this input in a layer: a button, a gesture or a combo.
    fn bind(&self, layer: &mut Layer, action: ButtonAction) {
        match (self.buttons.as_slice(), self.gesture) {
            ([], _) => {}
            ([b], None) => {
                layer.buttons.insert(*b, action);
            }
            ([b], Some(kind)) => *layer.gestures.entry(*b).or_default().slot(kind) = Some(action),
            (buttons, _) => layer.combos.push(Combo { buttons: super::combo_key(buttons), action }),
        }
    }
}

impl From<Button> for ShortcutInput {
    fn from(b: Button) -> Self {
        ShortcutInput::button(b)
    }
}

/// How a button reads in a sentence.
fn short_name(b: Button) -> &'static str {
    match b {
        Button::South => "A",
        Button::East => "B",
        Button::West => "X",
        Button::North => "Y",
        Button::LeftBumper => "LB",
        Button::RightBumper => "RB",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::Guide => "Guide",
        Button::LeftStick => "L3",
        Button::RightStick => "R3",
        Button::DpadUp => "D-pad Up",
        Button::DpadDown => "D-pad Down",
        Button::DpadLeft => "D-pad Left",
        Button::DpadRight => "D-pad Right",
        Button::LeftStickUp => "Left Stick Up",
        Button::LeftStickDown => "Left Stick Down",
        Button::LeftStickLeft => "Left Stick Left",
        Button::LeftStickRight => "Left Stick Right",
        Button::RightStickUp => "Right Stick Up",
        Button::RightStickDown => "Right Stick Down",
        Button::RightStickLeft => "Right Stick Left",
        Button::RightStickRight => "Right Stick Right",
        Button::LeftPaddle => "Left Paddle",
        Button::RightPaddle => "Right Paddle",
        Button::LeftPaddle2 => "Left Paddle 2",
        Button::RightPaddle2 => "Right Paddle 2",
    }
}

/// Which mode a shortcut belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// What opens the overlay, which also ends mouse mode.
    Opener,
    Guide,
    Mouse,
    /// The buttons that confirm and go back in the menus and panels.
    Menu,
    /// The media controls' own buttons (see `panels`).
    Media,
    /// The on-screen numpad's own buttons.
    Numpad,
    /// The on-screen keyboard's own buttons.
    Keyboard,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Group::Opener => "Guide overlay",
            Group::Guide => "Guide mode shortcuts",
            Group::Mouse => "Mouse mode buttons",
            Group::Menu => "Menu buttons",
            Group::Media => "Media controls buttons",
            Group::Numpad => "On-screen numpad buttons",
            Group::Keyboard => "On-screen keyboard buttons",
        }
    }
}

/// A shortcut that can be moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shortcut {
    /// Opens and closes the Guide overlay; held, it turns the Guide mode on. Guide, unless moved.
    OpenGuide,
    Keyboard,
    Numpad,
    Media,
    Screenshot,
    Recording,
    QuickSettings,
    ForceQuit,
    MouseMode,
    Shift,
    Enter,
    Tab,
    Escape,
    Ctrl,
    Alt,
    Delete,
    MiddleClick,
    Back,
    Forward,
    /// Confirms in menus and panels (A unless moved).
    MenuConfirm,
    /// Goes back or closes in menus and panels (B unless moved).
    MenuBack,
    /// The media controls' play or pause, next and previous track, player and mute.
    MediaPlay,
    MediaNext,
    MediaPrevious,
    MediaPlayer,
    MediaMute,
    /// Closes the media controls (East unless moved).
    MediaClose,
    /// The numpad's backspace and enter, and what closes it (East unless moved).
    NumpadBackspace,
    NumpadEnter,
    NumpadClose,
    /// The on-screen keyboard's backspace, space, enter, text cursor left and right, and close
    /// (see `panels`). Its press is the menu confirm button.
    KeyboardBackspace,
    KeyboardSpace,
    KeyboardEnter,
    KeyboardLeft,
    KeyboardRight,
    KeyboardClose,
}

impl Shortcut {
    /// Where the shortcut must sit when it's a single button: the menu buttons and the panel keys.
    /// `None` for the shortcuts that take any input.
    pub fn choices(self) -> Option<&'static [Button]> {
        match self.group() {
            Group::Menu => Some(&MenuKeys::CHOICES),
            Group::Media | Group::Numpad | Group::Keyboard => Some(&panels::CHOICES),
            _ => None,
        }
    }

    /// Whether a single button fits the shortcut (see [`Shortcut::choices`]).
    fn takes(self, input: &ShortcutInput) -> bool {
        self.choices().is_some_and(|c| input.gesture.is_none() && matches!(input.buttons.as_slice(), [b] if c.contains(b)))
    }

    pub const ALL: [Shortcut; 36] = [
        Shortcut::OpenGuide,
        Shortcut::Keyboard,
        Shortcut::Numpad,
        Shortcut::Media,
        Shortcut::Screenshot,
        Shortcut::Recording,
        Shortcut::QuickSettings,
        Shortcut::ForceQuit,
        Shortcut::MouseMode,
        Shortcut::Shift,
        Shortcut::Enter,
        Shortcut::Tab,
        Shortcut::Escape,
        Shortcut::Ctrl,
        Shortcut::Alt,
        Shortcut::Delete,
        Shortcut::MiddleClick,
        Shortcut::Back,
        Shortcut::Forward,
        Shortcut::MenuConfirm,
        Shortcut::MenuBack,
        Shortcut::MediaPlay,
        Shortcut::MediaNext,
        Shortcut::MediaPrevious,
        Shortcut::MediaPlayer,
        Shortcut::MediaMute,
        Shortcut::MediaClose,
        Shortcut::NumpadBackspace,
        Shortcut::NumpadEnter,
        Shortcut::NumpadClose,
        Shortcut::KeyboardBackspace,
        Shortcut::KeyboardSpace,
        Shortcut::KeyboardEnter,
        Shortcut::KeyboardLeft,
        Shortcut::KeyboardRight,
        Shortcut::KeyboardClose,
    ];

    pub fn group(self) -> Group {
        match self {
            Shortcut::OpenGuide => Group::Opener,
            Shortcut::Keyboard
            | Shortcut::Numpad
            | Shortcut::Media
            | Shortcut::Screenshot
            | Shortcut::Recording
            | Shortcut::QuickSettings
            | Shortcut::ForceQuit
            | Shortcut::MouseMode => Group::Guide,
            Shortcut::MenuConfirm | Shortcut::MenuBack => Group::Menu,
            Shortcut::MediaPlay | Shortcut::MediaNext | Shortcut::MediaPrevious | Shortcut::MediaPlayer | Shortcut::MediaMute | Shortcut::MediaClose => {
                Group::Media
            }
            Shortcut::NumpadBackspace | Shortcut::NumpadEnter | Shortcut::NumpadClose => Group::Numpad,
            Shortcut::KeyboardBackspace
            | Shortcut::KeyboardSpace
            | Shortcut::KeyboardEnter
            | Shortcut::KeyboardLeft
            | Shortcut::KeyboardRight
            | Shortcut::KeyboardClose => Group::Keyboard,
            _ => Group::Mouse,
        }
    }

    /// The shortcuts of `group`, in the order they're listed.
    pub fn of(group: Group) -> impl Iterator<Item = Shortcut> {
        Shortcut::ALL.into_iter().filter(move |s| s.group() == group)
    }

    pub fn label(self) -> &'static str {
        match self {
            Shortcut::OpenGuide => "Open the Guide overlay",
            Shortcut::Keyboard => "On-screen keyboard",
            Shortcut::Numpad => "On-screen numpad",
            Shortcut::Media => "Media controls",
            Shortcut::Screenshot => "Screenshot",
            Shortcut::Recording => "Start or stop recording",
            Shortcut::QuickSettings => "Quick Settings menu",
            Shortcut::ForceQuit => "Force quit (hold 2 s)",
            Shortcut::MouseMode => "Mouse mode",
            Shortcut::Shift => "Shift",
            Shortcut::Enter => "Enter",
            Shortcut::Tab => "Tab",
            Shortcut::Escape => "Escape",
            Shortcut::Ctrl => "Ctrl",
            Shortcut::Alt => "Alt",
            Shortcut::Delete => "Delete",
            Shortcut::MiddleClick => "Middle click",
            Shortcut::Back => "Mouse back",
            Shortcut::Forward => "Mouse forward",
            Shortcut::MenuConfirm => "Confirm",
            Shortcut::MenuBack => "Back / close",
            Shortcut::MediaPlay => "Play or pause",
            Shortcut::MediaNext => "Next track",
            Shortcut::MediaPrevious => "Previous track",
            Shortcut::MediaPlayer => "Switch player",
            Shortcut::MediaMute => "Mute",
            Shortcut::NumpadBackspace => "Numpad backspace",
            Shortcut::MediaClose => "Close",
            Shortcut::NumpadEnter => "Numpad enter",
            Shortcut::NumpadClose => "Numpad close",
            Shortcut::KeyboardBackspace => "Keyboard backspace",
            Shortcut::KeyboardSpace => "Keyboard space",
            Shortcut::KeyboardEnter => "Keyboard enter",
            Shortcut::KeyboardLeft => "Text cursor left",
            Shortcut::KeyboardRight => "Text cursor right",
            Shortcut::KeyboardClose => "Keyboard close",
        }
    }

    /// Where it sits unless moved.
    pub fn default_input(self) -> ShortcutInput {
        ShortcutInput::button(match self {
            Shortcut::OpenGuide => Button::Guide,
            Shortcut::Keyboard | Shortcut::Ctrl => Button::West,
            Shortcut::Numpad | Shortcut::Alt => Button::North,
            Shortcut::Media | Shortcut::Back => Button::LeftBumper,
            Shortcut::Screenshot | Shortcut::Forward => Button::RightBumper,
            Shortcut::Recording | Shortcut::MiddleClick => Button::LeftStick,
            Shortcut::QuickSettings => Button::Start,
            Shortcut::ForceQuit => Button::Select,
            Shortcut::MouseMode => Button::RightStick,
            Shortcut::Shift => Button::DpadUp,
            Shortcut::Enter => Button::DpadRight,
            Shortcut::Tab => Button::DpadDown,
            Shortcut::Escape => Button::DpadLeft,
            Shortcut::Delete | Shortcut::MenuBack => Button::East,
            Shortcut::MenuConfirm | Shortcut::MediaPlay => Button::South,
            Shortcut::MediaNext => Button::RightBumper,
            Shortcut::MediaPrevious => Button::LeftBumper,
            Shortcut::MediaPlayer => Button::North,
            Shortcut::MediaMute | Shortcut::NumpadBackspace => Button::West,
            Shortcut::MediaClose | Shortcut::NumpadClose => Button::East,
            Shortcut::NumpadEnter | Shortcut::KeyboardEnter => Button::Start,
            Shortcut::KeyboardBackspace => Button::West,
            Shortcut::KeyboardSpace => Button::North,
            Shortcut::KeyboardLeft => Button::LeftBumper,
            Shortcut::KeyboardRight => Button::RightBumper,
            Shortcut::KeyboardClose => Button::East,
        })
    }

    /// What it does, for the shortcuts that are a mode's binding (not the opener).
    fn action(self) -> Option<ButtonAction> {
        use ButtonAction::*;
        let key = |k: &str| Keys(vec![k.into()]);
        Some(match self {
            Shortcut::OpenGuide => return None,
            Shortcut::Keyboard => ToggleOverlay,
            Shortcut::Numpad => ToggleNumpad,
            Shortcut::Media => ToggleMedia,
            Shortcut::Screenshot => Screenshot,
            Shortcut::Recording => ToggleRecording,
            Shortcut::QuickSettings => QuickSettings,
            Shortcut::ForceQuit => ForceQuit,
            Shortcut::MouseMode => Mode(MOUSE_MODE.into()),
            Shortcut::Shift => key("KEY_LEFTSHIFT"),
            Shortcut::Enter => key("KEY_ENTER"),
            Shortcut::Tab => key("KEY_TAB"),
            Shortcut::Escape => key("KEY_ESC"),
            Shortcut::Ctrl => key("KEY_LEFTCTRL"),
            Shortcut::Alt => key("KEY_LEFTALT"),
            Shortcut::Delete => key("KEY_DELETE"),
            Shortcut::MiddleClick => Mouse(MouseButton::Middle),
            Shortcut::Back => Mouse(MouseButton::Back),
            Shortcut::Forward => Mouse(MouseButton::Forward),
            Shortcut::MenuConfirm
            | Shortcut::MenuBack
            | Shortcut::MediaPlay
            | Shortcut::MediaNext
            | Shortcut::MediaPrevious
            | Shortcut::MediaPlayer
            | Shortcut::MediaMute
            | Shortcut::MediaClose
            | Shortcut::NumpadBackspace
            | Shortcut::NumpadEnter
            | Shortcut::NumpadClose
            | Shortcut::KeyboardBackspace
            | Shortcut::KeyboardSpace
            | Shortcut::KeyboardEnter
            | Shortcut::KeyboardLeft
            | Shortcut::KeyboardRight
            | Shortcut::KeyboardClose => return None,
        })
    }
}

/// The buttons that confirm and go back in the menus and panels, which are A and B unless
/// moved. The one that's moved to takes the place of A (or B), and A (or B) takes its place, so
/// each still does what the button it swapped with did. Any button the menus don't hardcode can be
/// chosen ([`MenuKeys::CHOICES`]): the D-pad moves the menus and Guide opens the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuKeys {
    pub confirm: Button,
    pub back: Button,
}

impl Default for MenuKeys {
    fn default() -> Self {
        MenuKeys { confirm: Button::South, back: Button::East }
    }
}

impl MenuKeys {
    /// What the confirm and back buttons can be moved to: every button but the D-pad and Guide.
    pub const CHOICES: [Button; 14] = [
        Button::South,
        Button::East,
        Button::North,
        Button::West,
        Button::LeftBumper,
        Button::RightBumper,
        Button::Select,
        Button::Start,
        Button::LeftStick,
        Button::RightStick,
        Button::LeftPaddle,
        Button::RightPaddle,
        Button::LeftPaddle2,
        Button::RightPaddle2,
    ];

    /// What the panels should make of a press of `b`: A where the confirm button is, B where the
    /// back button is (only when `back` says it's in use), and each of those two where the
    /// others sit.
    pub fn logical(self, b: Button, back: bool) -> Button {
        let swap = |x: Button, a: Button, c: Button| if x == a { c } else if x == c { a } else { x };
        let b = swap(b, self.confirm, Button::South);
        if !back {
            return b;
        }
        let back_now = swap(self.back, self.confirm, Button::South);
        swap(b, back_now, Button::East)
    }
}

/// How the engine takes the opener.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GuideOpener {
    /// The Guide button, which is also what the engine falls back on.
    #[default]
    Guide,
    /// Another button, used like Guide: held it turns the Guide mode on, tapped it opens the
    /// overlay.
    Button(Button),
    /// A gesture or a combo, which the profile's own machinery detects (see [`Shortcuts::inject`]),
    /// or nothing at all. These are the buttons it uses.
    Triggered(Vec<Button>),
}

/// What the player moved. Anything not in it sits at its default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Shortcuts {
    moved: BTreeMap<Shortcut, ShortcutInput>,
}

impl Shortcuts {
    pub fn is_default(&self) -> bool {
        self.moved.is_empty()
    }

    /// Where `s` is now: its default unless moved. `None` when it's switched off.
    pub fn input(&self, s: Shortcut) -> Option<ShortcutInput> {
        let input = self.moved.get(&s).cloned().unwrap_or_else(|| s.default_input());
        (!input.is_off()).then_some(input)
    }

    pub fn is_moved(&self, s: Shortcut) -> bool {
        self.moved.contains_key(&s)
    }

    /// Puts `s` on `input`, or switches it off with `None`. Back on its default, it's no longer
    /// moved. A panel key is always on one of its choices: one another key of its panel has is
    /// traded for the one it had (see `panels`), and anything else is ignored.
    pub fn set(&mut self, s: Shortcut, input: Option<ShortcutInput>) {
        let mut input = input.unwrap_or_default();
        if s.is_panel_key() {
            if !s.takes(&input) {
                return;
            }
            let b = input.buttons[0];
            input = ShortcutInput::button(b);
            if let Some(other) = Shortcut::of(s.group()).find(|o| *o != s && self.button(*o) == b) {
                let was = self.button(s);
                self.put(other, ShortcutInput::button(was));
            }
        }
        self.put(s, input);
    }

    fn put(&mut self, s: Shortcut, input: ShortcutInput) {
        // The menu buttons are always somewhere: off is the default.
        if input == s.default_input() || (s.group() == Group::Menu && input.is_off()) {
            self.moved.remove(&s);
        } else {
            self.moved.insert(s, input);
        }
    }

    pub fn reset(&mut self, s: Shortcut) {
        self.moved.remove(&s);
    }

    /// Without what can't be: Guide belongs to the Guide overlay, so no other shortcut sits on it,
    /// and the opener keeps it only as a plain button. The single-button ones keep to their choices,
    /// and no input lists a button twice (see [`ShortcutInput::tidied`]).
    pub fn sanitized(mut self) -> Self {
        self.moved.values_mut().for_each(|input| *input = std::mem::take(input).tidied());
        self.moved.retain(|s, input| match s {
            Shortcut::OpenGuide => !input.uses(Button::Guide) || *input == ShortcutInput::button(Button::Guide),
            s if s.choices().is_some() => s.takes(input),
            _ => !input.uses(Button::Guide),
        });
        self
    }

    /// The button a shortcut is on, where a single button is wanted: its default unless moved.
    pub fn button(&self, s: Shortcut) -> Button {
        self.input(s).and_then(|i| i.buttons.first().copied()).unwrap_or(s.default_input().buttons[0])
    }

    /// The buttons that confirm and go back in menus and panels.
    pub fn menu_keys(&self) -> MenuKeys {
        MenuKeys { confirm: self.button(Shortcut::MenuConfirm), back: self.button(Shortcut::MenuBack) }
    }

    pub fn opener(&self) -> Option<ShortcutInput> {
        self.input(Shortcut::OpenGuide)
    }

    /// What the engine does about the opener.
    pub fn engine_opener(&self) -> GuideOpener {
        match self.opener() {
            Some(i) if i == ShortcutInput::button(Button::Guide) => GuideOpener::Guide,
            Some(i) if i.buttons.len() == 1 && i.gesture.is_none() => GuideOpener::Button(i.buttons[0]),
            Some(i) => GuideOpener::Triggered(i.buttons),
            None => GuideOpener::Triggered(Vec::new()),
        }
    }

    /// Puts the opener's gesture or combo on `p`, doing [`OPEN_GUIDE`]. Plain buttons and the
    /// Guide button are the engine's own, so they need nothing.
    pub fn inject(&self, p: &mut Profile) {
        let Some(input) = self.opener() else { return };
        let action = ButtonAction::Mode(OPEN_GUIDE.into());
        match (input.buttons.as_slice(), input.gesture) {
            ([b], Some(kind)) => *p.gestures.entry(*b).or_default().slot(kind) = Some(action),
            (buttons, None) if buttons.len() > 1 => {
                let key = super::combo_key(buttons);
                p.combos.retain(|c| super::combo_key(&c.buttons) != key);
                p.combos.push(Combo { buttons: key, action });
            }
            _ => {}
        }
    }

    /// Whether the profile has to be changed for the opener (see [`Shortcuts::inject`]).
    pub fn injects(&self) -> bool {
        matches!(self.engine_opener(), GuideOpener::Triggered(b) if !b.is_empty())
    }

    /// The soft warning about the opener being off.
    pub fn opener_off(&self) -> bool {
        self.opener().is_none()
    }
}

impl Layer {
    /// The Guide mode with the shortcuts where `shortcuts` puts them.
    pub fn guide_with(shortcuts: &Shortcuts) -> Self {
        let mut layer = Layer::guide_base();
        for s in Shortcut::of(Group::Guide) {
            if let (Some(input), Some(action)) = (shortcuts.input(s), s.action()) {
                input.bind(&mut layer, action);
            }
        }
        layer
    }

    /// Mouse mode with its keys where `shortcuts` puts them, and the gyro and touchpad as
    /// `extras` says. Its sticks and triggers stay.
    pub fn mouse_with(shortcuts: &Shortcuts, extras: MouseModeExtras) -> Self {
        let mut layer = Layer::mouse_base();
        layer.gyro_pointer = extras.gyro;
        if extras.touchpad {
            layer.touchpad = Some(TouchpadConfig::default());
        }
        for s in Shortcut::of(Group::Mouse) {
            if let (Some(input), Some(action)) = (shortcuts.input(s), s.action()) {
                input.bind(&mut layer, action);
            }
        }
        layer
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// The built-in modes by name, built from `shortcuts` and what mouse mode lets through.
pub fn built_in_with(shortcuts: &Shortcuts, extras: MouseModeExtras, name: &str) -> Option<Layer> {
    match name {
        GUIDE_MODE => Some(Layer::guide_with(shortcuts)),
        MOUSE_MODE => Some(Layer::mouse_with(shortcuts, extras)),
        _ => None,
    }
}

mod conflicts;
mod panels;
pub use panels::PanelPress;
#[cfg(test)]
mod tests;
