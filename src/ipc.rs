//! Daemon <-> client protocol: one JSON request line, one JSON response line per connection.
//! The protocol is specified in `docs/protocol.md`; a change to the wire format of anything it
//! covers is a change of [`PROTOCOL_MAJOR`] or [`PROTOCOL_MINOR`].

use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub use std::os::unix::net::{UnixListener, UnixStream};

use crate::{
    config::{Button, Config, ProfileRef},
    info::PadModel,
    inputlog::LogView,
    input::Axis,
};

/// The protocol's version. Within one major version a request or a field only ever gets added;
/// removing or reshaping one is a new major version (see `docs/protocol.md`).
pub const PROTOCOL_MAJOR: u32 = 1;
pub const PROTOCOL_MINOR: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Which protocol version the daemon speaks, and which build it is.
    Version,
    Status,
    /// Runs `padwight update` in the background (the daemon restarts at its end).
    InstallUpdate,
    GetConfig,
    /// Replace profiles/ignore list. The daemon keeps its current `enabled` and active profile.
    SetConfig(Box<Config>),
    /// Re-read the config file from disk.
    Reload,
    SetEnabled(bool),
    /// Lock the active profile so rules and the default profile don't switch it away. Not kept
    /// across restarts, and cleared by any switch of profile.
    SetProfileLock(bool),
    /// The settings window's unsaved edits, sent on every poll: a stop of the daemon waits for them
    /// (see `daemon/stop.rs`).
    Unsaved(bool),
    /// The settings window keeps its unsaved edits: a stop the daemon is waiting on is dropped.
    CancelStop,
    /// Switch to a profile found by name (see `Config::find_profile`).
    SetProfile(String),
    /// Switch to exactly this profile.
    Activate(ProfileRef),
    /// Sets how a remembered controller is told apart and whether it passes through.
    SetController(ControllerInfo),
    /// Forgets a remembered controller: on its next connection it counts as new.
    ForgetController(String),
    /// Turns the Nintendo button layout on or off for controller `id`.
    SetNintendoLayout { id: String, on: bool },
    /// Makes a profile of the active game the one controller `id` uses.
    ActivateFor { id: String, profile: String },
    NextProfile,
    /// Keep the connection open; the daemon streams one `Option<InputSnapshot>` JSON line per
    /// update (at most ~60/s). `null` means no controller is active.
    WatchInput,
    /// Play a short test pattern on the controller at this device path.
    TestRumble(String),
    /// Average the gyro of the controller at this path for a moment, as its drift bias.
    CalibrateGyro(String),
    /// After 3 seconds, move the mouse this many pixels to the right over a second, to see how
    /// far it turns a game's camera.
    TestTurn(i32),
    /// Open or close the on-screen keyboard.
    ToggleOverlay,
    /// Open or close the on-screen numpad.
    ToggleNumpad,
    /// Show the menu with this name (items run on the most recently used controller).
    OpenMenu(String),
    /// Close every overlay that has the controller, and every Guide overlay.
    CloseOverlays,
    /// Keep the connection open; the daemon streams one `Option<OverlayView>` JSON line per
    /// change. `null` means nothing is shown (the resident overlay idles).
    WatchOverlay,
    /// Keep the connection open; the daemon streams one `Option<Feeds>` JSON line per change: the
    /// latest presses of the controller with this id (the most recently used one for `None`), and
    /// what padwight exported for them. `null` when that controller isn't connected.
    WatchFeed(Option<String>),
    /// Debug mode only (the daemon must run with `--debug`): adds a controller that no hardware
    /// backs, and replies `Attached` with its device path. Its output is recorded, and also sent
    /// to real virtual devices if `live`.
    DebugAttach { model: Option<PadModel>, live: bool },
    /// Debug mode only: input from the injected controller at this path.
    DebugInput { path: String, events: Vec<DebugEvent> },
    /// Debug mode only: what the injected controller's mappings have output, as `Lines`.
    DebugOutput { path: String, clear: bool },
    /// Debug mode only: removes the injected controller at this path, or every one for `all`.
    DebugDetach(String),
    /// Debug mode only: the identity every virtual pad presents as, whatever the game and the
    /// controller say; `None` goes back to choosing.
    DebugIdentify(Option<crate::pad_identity::PadIdentity>),
}

/// One input from an injected controller.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DebugEvent {
    Button(Button, bool),
    /// Sticks are -1.0..1.0 (Y positive = down), triggers 0.0..1.0.
    Axis(Axis, f32),
    /// One motion reading: angular velocity in degrees/second (pitch, yaw, roll) over `ms`.
    Motion { gyro: [f32; 3], ms: u32 },
    /// A finger on the touchpad, or its click.
    Touchpad(crate::input::TouchpadEvent),
}

// One reply per connection, so the size difference doesn't matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Version(VersionInfo),
    Status(Status),
    Config(Box<Config>),
    /// The device path of an injected controller.
    Attached(String),
    Lines(Vec<String>),
    Error(String),
}

/// The reply to [`Request::Version`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub protocol_major: u32,
    pub protocol_minor: u32,
    /// The daemon's own version (`CARGO_PKG_VERSION`), for bug reports; not for compatibility checks.
    pub app: String,
}

impl VersionInfo {
    pub fn current() -> Self {
        Self { protocol_major: PROTOCOL_MAJOR, protocol_minor: PROTOCOL_MINOR, app: env!("CARGO_PKG_VERSION").into() }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub enabled: bool,
    pub active_profile: String,
    /// The active profile's game; `None` is General.
    #[serde(default)]
    pub active_game: Option<String>,
    /// Layers active right now (oldest first), on top of the active profile.
    #[serde(default)]
    pub active_layers: Vec<String>,
    /// The mode switched to (under the layers); `None` is the default mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_mode: Option<String>,
    pub devices: Vec<DeviceInfo>,
    /// Every controller remembered, connected or not.
    #[serde(default)]
    pub controllers: Vec<ControllerInfo>,
    #[serde(default)]
    pub focus_backend: FocusBackend,
    /// Currently focused window (when the desktop reports it).
    #[serde(default)]
    pub focused: Option<WindowInfo>,
    /// Recently focused windows, newest first, for building rules.
    #[serde(default)]
    pub recent_windows: Vec<WindowInfo>,
    /// Controller motion sensors the daemon may not open (needs the udev rule in dist/).
    #[serde(default)]
    pub motion_access_denied: Vec<String>,
    /// Managed controllers that other programs also read through their raw HID node, past the grab.
    #[serde(default)]
    pub raw_readers: Vec<RawReaders>,
    /// The on-screen keyboard is up.
    #[serde(default)]
    pub overlay_visible: bool,
    #[serde(default)]
    pub numpad_visible: bool,
    /// The daemon is ending, and waits for the settings window to save or discard its unsaved edits.
    #[serde(default)]
    pub stop_pending: bool,
    /// The active profile is locked against automatic switching.
    #[serde(default)]
    pub profile_locked: bool,
    /// Profile switches, newest first, each with the reason the daemon made it.
    #[serde(default)]
    pub switches: Vec<SwitchEvent>,
    /// A newer release than this one, found by the daily check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<UpdateInfo>,
}

/// A release newer than the running one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    /// Whether `padwight update` may replace this copy; if not, `advice` says where to get it.
    pub self_updating: bool,
    pub advice: String,
}

/// The Overview's two feeds for one controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feeds {
    /// The controller they are of, as the header names it.
    #[serde(default)]
    pub controller: String,
    /// The presses as the controller labels them, with what each did.
    pub presses: LogView,
    /// The buttons padwight exported for those presses, labeled as the controller in use labels them.
    pub exports: LogView,
}

/// A profile switch and why it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwitchEvent {
    /// Milliseconds since the switch.
    pub age_ms: u64,
    /// The profile's setup; `None` is General.
    pub game: Option<String>,
    pub profile: String,
    /// What caused it, in words, e.g. `Doom (doom.exe) matches Executable “doom.exe”`.
    pub why: String,
}

/// How the daemon learns which game is active.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusBackend {
    /// KWin script reports focus changes (KDE Plasma, Wayland or X11).
    Kwin,
    /// Our GNOME Shell extension reports focus changes (GNOME, Wayland or X11).
    GnomeShell,
    /// Our Cinnamon extension reports focus changes (Cinnamon, Wayland or X11).
    Cinnamon,
    /// COSMIC's toplevel info protocol reports focus changes.
    Cosmic,
    /// Sway's IPC socket reports focus changes.
    Sway,
    /// Hyprland's event socket reports focus changes.
    Hyprland,
    /// niri's IPC event stream reports focus changes.
    Niri,
    /// The compositor's `wlr-foreign-toplevel-management` reports focus changes (labwc, Wayfire, river, and others).
    Wlroots,
    /// No focus information: rules match against running processes instead.
    #[default]
    ProcessScan,
}

impl FocusBackend {
    /// The desktop whose windows are tracked, `None` when only processes are.
    pub fn desktop(self) -> Option<&'static str> {
        match self {
            Self::Kwin => Some("KWin"),
            Self::GnomeShell => Some("GNOME Shell"),
            Self::Cinnamon => Some("Cinnamon"),
            Self::Cosmic => Some("COSMIC"),
            Self::Sway => Some("Sway"),
            Self::Hyprland => Some("Hyprland"),
            Self::Niri => Some("niri"),
            Self::Wlroots => Some("wlroots"),
            Self::ProcessScan => None,
        }
    }

    /// What per-game switching follows, for people.
    pub fn label(self) -> String {
        match self.desktop() {
            Some(desktop) => format!("focused window ({desktop})"),
            None => "running processes".into(),
        }
    }
}

/// What we know about a window (or, for process scanning, a process).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub class: String,
    pub title: String,
    pub pid: u32,
    /// Executable file name; for Wine/Proton the Windows `.exe`, not the Wine loader.
    pub exe: String,
    /// Full path of that executable, as the process gives it (a Windows path for Wine/Proton);
    /// empty when unknown. Rules that name a folder match its end.
    #[serde(default)]
    pub path: String,
    pub steam_app_id: Option<String>,
}

/// A remembered controller (see `config::ControllerEntry`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ControllerInfo {
    pub id: String,
    /// The name it last connected under.
    pub name: String,
    pub nickname: String,
    pub color: Option<String>,
    pub player: Option<u8>,
    pub pass_through: bool,
    /// Its face buttons use the Nintendo layout (a pack's own choice wins).
    #[serde(default)]
    pub nintendo_layout: bool,
    /// Its set-up question is still unanswered.
    pub pending: bool,
    pub connected: bool,
}

impl ControllerInfo {
    /// What to call it: its nickname, else the name it last connected under, else its id.
    pub fn nickname_or_name(&self) -> &str {
        [&self.nickname, &self.name, &self.id].into_iter().find(|s| !s.is_empty()).map_or("", |s| s.as_str())
    }
}

/// A managed controller and the other programs that have its raw HID node (/dev/hidrawN) open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawReaders {
    pub controller: String,
    /// Their process names, such as `steam`.
    pub programs: Vec<String>,
}

impl RawReaders {
    /// The programs, as a sentence starts with them: "Steam", "Brave, steam".
    pub fn who(&self) -> String {
        let all = self.programs.join(", ");
        let mut chars = all.chars();
        chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
    }

    /// Says that they get the controller's presses as well as Padwight's virtual pad.
    pub fn warning(&self) -> String {
        let (verb, gets) = if self.programs.len() == 1 { ("reads", "it gets") } else { ("read", "they get") };
        format!("{} also {verb} {} directly, so {gets} your presses as well as Padwight's virtual pad.", self.who(), self.controller)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    /// What settings saved for this controller are keyed by (see `controller_id`); empty when it has none.
    #[serde(default)]
    pub id: String,
    pub path: String,
    /// The profile it uses now (empty for a controller that isn't managed).
    #[serde(default)]
    pub profile: String,
    /// Grabbed and being remapped right now.
    pub managed: bool,
    pub ignored: bool,
    /// False when the triggers only report on/off (e.g. Switch controllers).
    #[serde(default = "yes")]
    pub analog_triggers: bool,
    /// Supports rumble (FF_RUMBLE), so it can be tested and receives game rumble.
    #[serde(default)]
    pub rumble: bool,
    /// A motion-sensor device is paired with it.
    #[serde(default)]
    pub gyro: bool,
    /// Has back paddles.
    #[serde(default)]
    pub paddles: bool,
    /// Its touchpad is paired with it.
    #[serde(default)]
    pub touchpad: bool,
    /// Whose button glyphs it uses, when it's recognized (for a managed controller).
    #[serde(default)]
    pub family: Option<crate::info::PadFamily>,
    /// Which model it is, when recognized (for its picture in the editor).
    #[serde(default)]
    pub model: Option<crate::info::PadModel>,
}

fn yes() -> bool {
    true
}

/// Physical (pre-mapping) input state of the most recently used controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSnapshot {
    pub device: String,
    /// The controller's model, when recognized: which picture the editor draws for it.
    #[serde(default)]
    pub model: Option<crate::info::PadModel>,
    /// Whose button glyphs the controller uses, when recognized.
    #[serde(default)]
    pub family: Option<crate::info::PadFamily>,
    pub buttons: Vec<Button>,
    /// Sticks are -1.0..1.0 (Y positive = down).
    pub left_stick: (f32, f32),
    pub right_stick: (f32, f32),
    /// 0.0..1.0
    pub left_trigger: f32,
    pub right_trigger: f32,
    /// Degrees/second pitch, yaw, roll, for controllers with a gyro.
    #[serde(default)]
    pub gyro: Option<[f32; 3]>,
    /// What the gyro is doing under the active profile, while it is live.
    #[serde(default)]
    pub gyro_out: Option<GyroOut>,
    /// Fingers on the touchpad, each (x, y) 0..1 across it from its top left corner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<(f32, f32)>,
    /// Whether the touchpad is clicked.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub touch_click: bool,
}

/// The gyro's live output, for the settings page's pictures.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GyroOut {
    /// Turning speed (degrees/second) the acceleration curve reads.
    pub speed: f32,
    /// The turning sent on (degrees/second, positive = right/down), after the curve and the
    /// axes' sensitivity, before the mode's own sensitivity.
    pub aim: (f32, f32),
    /// The virtual-pad stick deflection gyro sends, in the stick modes.
    pub stick: Option<(f32, f32)>,
}

/// The runtime directory, for files that the desktop or another instance must reach.
#[cfg(target_os = "linux")]
pub fn shared_runtime_dir() -> Option<PathBuf> {
    dirs::runtime_dir()
}

pub fn socket_path() -> PathBuf {
    match shared_runtime_dir() {
        Some(dir) => dir.join("padwight.sock"),
        None => fallback_socket_dir().join("padwight.sock"),
    }
}

/// Without a runtime dir, a per-user directory in the shared temp dir (the daemon makes it
/// private before binding; see `daemon::bind_socket`).
#[cfg(target_os = "linux")]
fn fallback_socket_dir() -> PathBuf {
    // SAFETY: getuid cannot fail.
    std::env::temp_dir().join(format!("padwight-{}", unsafe { libc::getuid() }))
}

/// Sends one of the watch requests and returns the JSON lines the daemon streams back, until the
/// connection ends. A thread reads them, so this works the same over every platform's socket.
pub fn watch(req: &Request) -> Result<iced::futures::channel::mpsc::UnboundedReceiver<String>> {
    let path = socket_path();
    let stream = UnixStream::connect(&path).with_context(|| format!("daemon not reachable at {}", path.display()))?;
    let mut line = serde_json::to_string(req)?;
    line.push('\n');
    (&stream).write_all(line.as_bytes())?;
    let (tx, rx) = iced::futures::channel::mpsc::unbounded();
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            // The receiver is gone once whoever watched has stopped.
            if line.ok().is_none_or(|line| tx.unbounded_send(line).is_err()) {
                return;
            }
        }
    });
    Ok(rx)
}

/// How long the daemon waits for its event loop to answer a request before it says so. A client
/// waits a little longer, so it hears that rather than timing out first.
pub const REPLY_WAIT: Duration = Duration::from_secs(2);

/// Blocking request to the daemon. Fine to call from a GUI via `spawn_blocking`.
pub fn request(req: &Request) -> Result<Response> {
    request_at(&socket_path(), req)
}

fn request_at(path: &std::path::Path, req: &Request) -> Result<Response> {
    let stream = UnixStream::connect(path)
        .with_context(|| format!("daemon not reachable at {}", path.display()))?;
    stream.set_read_timeout(Some(REPLY_WAIT + Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let mut line = serde_json::to_string(req)?;
    line.push('\n');
    (&stream).write_all(line.as_bytes())?;

    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply).context("no reply from the daemon")?;
    if reply.is_empty() {
        bail!("the daemon closed the connection without replying");
    }
    match serde_json::from_str(&reply).context("the daemon's reply")? {
        Response::Error(e) => bail!(e),
        r => Ok(r),
    }
}

#[cfg(test)]
mod tests;
