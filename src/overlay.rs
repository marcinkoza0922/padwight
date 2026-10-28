//! On-screen overlay: a keyboard you type on with the controller, and action menus. The
//! daemon owns the state (`OverlayController` here, `menu::MenuSession`) and routes
//! controller input to it while something is shown; the overlay process
//! (`padwight overlay`) only draws it, on a Wayland layer-shell surface that never
//! takes keyboard focus, so typed keys reach the window underneath. The process stays
//! running as an invisible 1×1 surface between uses so menus appear instantly.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::keyboard::{self, Board, Cursor};

/// Holding East this long closes the overlay.
pub const HOLD_TO_CLOSE: Duration = Duration::from_millis(700);
/// The keyboard's reminder of its controls, with `{tokens}` drawn as the pad's glyphs.
/// The legend of the keyboard while it types into a field of the in-game menu.
pub const FIELD_LEGEND: &[&str] = &[
    "{dpad} move",
    "{confirm} type",
    "{keyboard_backspace} delete (hold to repeat)",
    "{keyboard_space} space",
    "{keyboard_enter} done",
    "hold {lt} shift",
    "hold {back} cancel",
];

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

impl OverlayView {
    /// The style the view is drawn with.
    pub fn style(&self) -> &crate::config::OverlayStyle {
        match self {
            OverlayView::Keyboard(k) => &k.style,
            OverlayView::Menu(m) => &m.style,
            OverlayView::Media(m) => &m.style,
            OverlayView::Offer(o) => &o.style,
        }
    }
}

/// Everything the overlay window shows at once: info overlays, plus the keyboard or a menu.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OverlayFrame {
    pub info: Vec<crate::info::InfoView>,
    /// Absent from an older daemon's frames.
    #[serde(default)]
    pub logs: Vec<crate::inputlog::LogView>,
    pub active: Option<OverlayView>,
    /// A menu being edited, drawn under `active` where it goes (see `Preview`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<crate::menu::MenuView>,
    /// `active` is held back so what's under it shows: it stays up, but isn't drawn.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub peek: bool,
    /// The magnifying glass, while one is held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lens: Option<Lens>,
    /// The doodles over the game, drawn under the magnifying glass and the panels.
    #[serde(default, skip_serializing_if = "crate::tools::doodle::Canvas::is_empty")]
    pub doodles: crate::tools::doodle::Canvas,
    /// `doodles` was sent without its finished items, which are those sent last: the daemon
    /// sends them only when one is added or goes (see `leave_out_doodles_sent`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub doodles_kept: bool,
    /// The Doodle screen's cursor, while that screen is up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doodle_cursor: Option<DoodleCursor>,
    /// The Doodle screen's HUD (its carousel and tray), while that screen is up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doodle_hud: Option<DoodleHud>,
    /// The font everything here is drawn in; the system's when unset.
    #[serde(default)]
    pub font: Option<String>,
    /// Whether menu rows are tinted in colors color-blind people can tell apart.
    #[serde(default)]
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
    /// The icons the game in use carries, which `{icon:…}` can name besides the built-in ones.
    /// `None` keeps the last ones: the daemon sends them only when they change, as they can be
    /// tens of KB and the frames come with every input while an overlay is up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_icons: Option<std::collections::BTreeMap<String, String>>,
    /// Where the fonts the game in use carries are, as files, by name (see `font::share_pack_fonts`).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub pack_fonts: std::collections::BTreeMap<String, std::path::PathBuf>,
    /// How each kind of overlay moves as it opens, closes, and as the cursor and picks move.
    #[serde(default)]
    pub motion: crate::motion::MotionSet,
    /// What the performance settings cut from every overlay.
    #[serde(default, skip_serializing_if = "crate::config::Lite::is_off")]
    pub lite: crate::config::Lite,
}

/// The Doodle screen's cursor: where it is, the radius it marks, its color and whether it's a beam (see
/// `tools::doodle`). Positions and radii are shares of the screen's height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoodleCursor {
    pub at: crate::tools::doodle::Point,
    pub radius: f32,
    pub color: [u8; 3],
    /// Drawn as a text editor's I-beam, from `at` down by the diameter, rather than a ring.
    #[serde(default)]
    pub beam: bool,
}

/// The Doodle screen's HUD: the tool the carousel across the top highlights (its index in
/// `tools::doodle::Tool`), and the tray along the bottom with the current settings. The tray is
/// left off while the keyboard is up. Hints are text with controller tokens, such as `{lb}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoodleHud {
    pub tool: usize,
    pub tray: bool,
    pub cells: Vec<DoodleCell>,
    /// The checklist of text styles, above the tray, while it's open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<DoodleChecks>,
    /// The menus' style, which the HUD is drawn in.
    pub style: crate::config::OverlayStyle,
}

/// The checklist a text box's styles are ticked in: one row per style, the row the cursor is on,
/// and the tray cell it opens from, which it stands over.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoodleChecks {
    pub rows: Vec<DoodleCheck>,
    pub cursor: usize,
    pub cell: usize,
}

/// One row of a [`DoodleChecks`]: a style's name, and whether it's ticked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoodleCheck {
    pub label: String,
    pub on: bool,
}

/// One part of the HUD's tray. Parts are drawn left to right, with a separator between them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DoodleCell {
    /// The full palette, with the color in use ringed. `hint` says how to change it.
    Palette { current: usize, hint: String },
    /// A setting as text, with its hint.
    Text(String),
    /// A pen tip: its name, with a hint, and an icon of the mark it makes.
    Tip { tip: crate::tools::doodle::Tip, text: String },
    /// A pen style, the same way as a tip.
    Kind { kind: crate::tools::doodle::Kind, text: String },
}

/// The magnifying glass: a lens over a frozen picture of the screen, centered on a point of it
/// and showing that part `zoom` times as large.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lens {
    /// Where the lens is, across and down the screen, 0 to 1.
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    /// Its radius as a share of the screen's shorter side.
    pub size: f32,
    /// A line under the lens, if it has something to say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The saved picture of the screen.
    pub shot: std::path::PathBuf,
}

impl OverlayFrame {
    /// Whether this frame can stand in for `older`, sent just before it, without the overlay
    /// missing anything it animates: the same panel, or none, with the same pick. A panel opening
    /// or closing, or a pick, must be seen even when the next frame follows at once, as the
    /// close does after a pick that closes a menu.
    /// Leaves the doodles' finished items out when they are those last sent on this stream
    /// (`sent`), for `put_back_doodles` at the other end. A screen of strokes is hundreds of KB
    /// as JSON, and a frame goes with each tick the Doodle cursor moves; the items change only
    /// when a stroke ends or one goes.
    pub fn leave_out_doodles_sent(&mut self, sent: &mut Vec<crate::tools::doodle::Item>) {
        let items = self.doodles.items_mut();
        if items == sent {
            items.clear();
            self.doodles_kept = true;
        } else {
            sent.clone_from(items);
        }
    }

    /// Puts back the finished doodles a frame was sent without (`leave_out_doodles_sent`), from
    /// those `kept` from the frames before it, and keeps those of a frame sent with them.
    pub fn put_back_doodles(&mut self, kept: &mut Vec<crate::tools::doodle::Item>) {
        if std::mem::take(&mut self.doodles_kept) {
            self.doodles.items_mut().clone_from(kept);
        } else {
            kept.clone_from(self.doodles.items_mut());
        }
    }

    pub fn stands_in_for(&self, older: &OverlayFrame) -> bool {
        use crate::motion::Tracked;
        let scene = |frame: &OverlayFrame| frame.active.as_ref().map(|view| (view.key(), view.pick()));
        scene(self) == scene(older)
    }

    /// Whether something in the frame loops (see `OverlayStyle::loops`) and the performance
    /// settings let it, so the screen keeps redrawing while it shows.
    pub fn loops(&self) -> bool {
        let styles = self.info.iter().map(|v| &v.style).chain(self.logs.iter().map(|v| &v.style)).chain(self.active.as_ref().map(OverlayView::style)).chain(self.backdrop.as_ref().map(|m| &m.style));
        styles.into_iter().any(|style| !style.lite.calm && style.loops())
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
        let backdrop = self.backdrop.as_mut().map(|m| &mut m.style);
        for style in styles.chain(active).chain(backdrop) {
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
    /// Typing into a field of the in-game menu, which this names: the keys go into `typed`, not
    /// to the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Whether the text caret is showing: it blinks while a field is typed into, like a text
    /// editor's.
    #[serde(default = "caret_on")]
    pub caret: bool,
}

fn caret_on() -> bool {
    true
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

/// Runs the overlay window: a layer-shell surface at the bottom of the screen drawing the
/// state the daemon streams, and exiting when the daemon hides it.
#[cfg(target_os = "linux")]
pub fn run() -> anyhow::Result<()> {
    wayland::run()
}

/// Runs the overlay window: a click-through layered window over the screen drawing the state
/// the daemon streams, and exiting when the daemon goes away.
#[cfg(windows)]
pub fn run() -> anyhow::Result<()> {
    windows::run()
}

#[cfg(target_os = "linux")]
mod wayland;
mod controller;
mod ui;
pub use controller::{OverlayAction, OverlayController, typing};
#[cfg(windows)]
mod windows;

pub use ui::{film, snapshot, snapshot_iced};

/// The procedural pictures behind panel filters, free of any drawing library.
pub mod art;
/// Drawing for overlays, generic over the message type so the settings GUI can show the
/// same thing as a live preview.
pub mod draw;
#[cfg(test)]
mod fade_frames;
pub mod fit;
/// The overlay's own renderer, free of iced.
#[expect(dead_code, reason = "built up step by step; nothing draws with it yet")]
#[cfg_attr(not(test), expect(unused_imports, reason = "built up step by step; nothing draws with it yet"))]
pub mod render;
mod timing;

#[cfg(test)]
mod tests;
