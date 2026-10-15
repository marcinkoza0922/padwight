//! Background service: grabs physical gamepads, runs the mapping engine and drives virtual devices.

use crate::sound;
use std::{
    collections::{HashMap, HashSet},
    process::Stdio,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{
    config::{Button, Config, Feature, Game, Group, PanelPress, Profile, ProfileRef, Refit, Scope, Shortcut},
    engine::{Engine, Opener},
    focus::{self, FocusEvent},
    input::{InputEvent, MotionFrame, MotionSample},
    ipc::{self, DeviceInfo, Feeds, FocusBackend, InputSnapshot, Request, Response, Status, WindowInfo},
    keyboard::Layout,
    monitor::{self, InputView, OutputView, log},
    output::{OutEvent, VirtualKbm, VirtualPad},
    pad_identity::{ControllerSupport, PadIdentity},
    media::{MediaOutcome, MediaSession, MediaState},
    offer::{OfferOutcome, OfferSession},
    menu::{MenuOutcome, MenuSession},
    system_menu::{Hold, Outcome as SystemOutcome, Page, SystemMenu},
    overlay::{OverlayAction, OverlayController, OverlayFrame, OverlayView},
    rumble,
    tray::{self, TrayIcon},
};

mod activity;
mod auto_switch;
mod first_pad;
mod guide;
mod in_game_menu;
mod info_overlays;
mod offers;
mod overlay;
mod overlay_input;
mod previews;
/// The controllers themselves: evdev nodes on Linux, XInput slots on Windows.
#[cfg_attr(target_os = "linux", path = "daemon/evdev.rs")]
#[cfg_attr(windows, path = "daemon/xinput.rs")]
mod devices;
mod inject;
mod logs;
mod requests;
mod serve;
/// The process around the service: signals, the control socket, running in the background.
#[cfg_attr(target_os = "linux", path = "daemon/unix.rs")]
#[cfg_attr(windows, path = "daemon/windows.rs")]
mod os;
#[cfg(target_os = "linux")]
mod pads;

/// Why a profile changed when someone picked it.
const BY_HAND: &str = "Chosen by hand (GUI or command line)";
#[cfg(all(test, target_os = "linux"))]
mod kernel_tests;
#[cfg(target_os = "linux")]
mod touchpad;

use devices::{FfSpec, NodeKey, virtual_pad_for};
pub(crate) use os::{bind_socket_at, prepare_private_dir, stop_on_signals};
use ipc::{UnixListener, UnixStream};

const TICK: Duration = Duration::from_millis(4);
/// How often a fading info overlay is redrawn.
const FADE_FRAME: Duration = Duration::from_millis(33);
/// How often info overlays with live values (time, CPU, …) update.
const INFO_REFRESH: Duration = Duration::from_secs(1);
/// How often an overlay showing the gyro or the touchpad refreshes: often enough to follow a hand.
const FAST_INFO_REFRESH: Duration = Duration::from_millis(50);
const SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// Status line redraw rate while only continuous output (mouse/scroll) is changing.
const STATUS_REDRAW: Duration = Duration::from_millis(50);
/// How long "Calibrate gyro" samples a still controller.
const GYRO_CALIBRATION: Duration = Duration::from_secs(2);
/// How many recently focused windows the GUI can offer for new rules.
const RECENT_WINDOWS: usize = 8;

/// How long a ForceQuit action has to be held.
const FORCE_QUIT_HOLD: Duration = Duration::from_secs(2);
/// A Guide press shorter than this, with nothing else pressed, is a tap.
const GUIDE_TAP: Duration = Duration::from_millis(350);

enum Msg {
    /// SIGTERM or SIGINT: release what is held, then exit.
    Stop,
    Input { id: u64, events: Vec<InputEvent> },
    Gone { id: u64 },
    Ipc { req: Request, reply: Sender<Response> },
    Watch(Sender<Option<InputSnapshot>>),
    WatchFeed(Sender<Option<Feeds>>),
    Focus(FocusEvent),
    #[cfg_attr(not(target_os = "linux"), expect(dead_code, reason = "only evdev has motion-sensor and touchpad nodes"))]
    Motion { id: u64, sample: MotionSample },
    #[cfg_attr(not(target_os = "linux"), expect(dead_code, reason = "only evdev has motion-sensor and touchpad nodes"))]
    MotionGone { id: u64 },
    #[cfg_attr(not(target_os = "linux"), expect(dead_code, reason = "only evdev has motion-sensor and touchpad nodes"))]
    TouchpadGone { id: u64 },
    WatchOverlay(Sender<OverlayFrame>),
    /// Something to say in a toast, from a background task (e.g. a screenshot finishing).
    Toast(Vec<String>),
    /// The recorder is running as `pid`, or could not start.
    RecordingStarted { pid: u32 },
    RecordingFailed(String),
    /// The recorder exited (finished its file, or died).
    RecordingEnded { pid: u32, ok: bool, path: PathBuf },
    /// A fresh reading of the media player.
    Media(MediaState),
    /// A choice from the tray icon's menu.
    Tray(tray::Command),
    /// Whether Steam runs, looked up as the Guide overlay opened.
    SteamRunning(bool),
}

struct Managed {
    path: PathBuf,
    name: String,
    /// Whose button names glyphs use, if recognized; otherwise `Config::info_glyphs`.
    family: Option<crate::info::PadFamily>,
    /// Which model it is, if recognized; its picture in the editor depends on it.
    model: Option<crate::info::PadModel>,
    engine: Engine,
    /// The virtual pad it feeds. `None` for an injected controller that stays off the real
    /// virtual devices (see [`Request::DebugAttach`]).
    pad: Option<VirtualPad>,
    /// The controller's force-feedback support, which a rebuilt virtual pad advertises too.
    ff: Option<FfSpec>,
    /// Stops the rumble thread of the current virtual pad; set on its own when the pad is rebuilt.
    rumble_stop: Arc<AtomicBool>,
    /// Its pad is the one kept ahead of the real controllers in Steam Input (see `first_pad.rs`).
    first: bool,
    /// When its pad, taken over from the kept one, is due to be rebuilt to fit it.
    refit_at: Option<Instant>,
    /// What an injected controller's mappings have output, newest last.
    recorded: Option<Vec<String>>,
    stop: Arc<AtomicBool>,
    view: InputView,
    out_view: OutputView,
    /// For pairing with the controller's motion-sensor device.
    parent: Option<PathBuf>,
    #[cfg(target_os = "linux")]
    uniq: Option<String>,
    /// Motion-sensor device feeding this controller's gyro, once found.
    motion: Option<PathBuf>,
    /// Touchpad node of this controller, once found (and taken from the desktop).
    touchpad: Option<PathBuf>,
    /// The nodes it took from the desktop, let go of when it is dropped.
    #[cfg(target_os = "linux")]
    grabs: Vec<devices::Grab>,
    /// Has back paddles (see `input::has_paddles`).
    paddles: bool,
    motion_frame: MotionFrame,
    /// Drift per raw sensor axis (the controller's own frame), subtracted before remapping.
    gyro_bias: [f32; 3],
    calibrating: Option<Calibration>,
    /// The active profile with this controller's mode and layers on top, while it has any, and
    /// which those are.
    layered: Option<(Shifts, Profile)>,
    /// Notices the input that opens the Guide overlay while a panel has the controller.
    opener_watch: crate::opener_watch::Watch,
    /// What it pressed lately, for the input log.
    log: crate::inputlog::InputLog,
    /// The buttons padwight exported to the virtual pad lately, for the Overview's second feed.
    exports: crate::inputlog::InputLog,
    /// 0, 1, 2… in the order controllers connected, kept until this one disconnects
    /// (`{current_input_device_N}`, a log overlay's controller).
    number: u8,
}

/// A controller's mode (`None`: the default) and its held layers, oldest first.
type Shifts = (Option<String>, Vec<String>);

/// What a controller's mappings come from: the active profile, or that profile with the
/// controller's mode and layers on top.
fn layered<'a>(cached: &'a Option<(Shifts, Profile)>, base: &'a Profile) -> &'a Profile {
    cached.as_ref().map_or(base, |(_, p)| p)
}

struct Calibration {
    started: Instant,
    sum: [f64; 3],
    samples: u32,
}

impl Managed {
    /// Makes the virtual pad present itself as `identity`. A pad already like that stays; another
    /// is rebuilt, and its rumble thread follows the new one.
    fn present_as(&mut self, identity: PadIdentity) {
        let Some(pad) = &self.pad else { return };
        if pad.identity() == identity {
            return;
        }
        log!("{} now presents as {identity:?}", self.name);
        self.rebuild_pad(identity);
    }

    /// Replaces the virtual pad with one presenting as `identity` and advertising the
    /// controller's rumble. The old pad goes first, so the new one gets its Steam Input slot.
    fn rebuild_pad(&mut self, identity: PadIdentity) {
        self.refit_at = None;
        self.rumble_stop.store(true, Ordering::Relaxed);
        self.rumble_stop = Arc::new(AtomicBool::new(false));
        self.pad = None;
        let Some(new) = virtual_pad_for(self.ff.as_ref(), identity) else { return };
        if self.ff.is_some() {
            rumble::spawn(new.shared(), self.path.clone(), self.rumble_stop.clone());
        }
        self.pad = Some(new);
    }

    /// Stops the threads that follow this controller: its reader, and its rumble.
    fn stop_threads(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.rumble_stop.store(true, Ordering::Relaxed);
    }

    /// Sends what the mappings output to the virtual devices; an injected controller's output
    /// is also recorded, and without a pad it goes no further.
    fn dispatch(&mut self, kbm: &mut VirtualKbm, out: Vec<OutEvent>) {
        let now = Instant::now();
        // What the game is sent, gamepad and keyboard and mouse alike, for the Overview's feed.
        for ev in &out {
            self.exports.output(ev, now);
        }
        if let Some(recorded) = &mut self.recorded {
            inject::record(recorded, &out);
        }
        match &mut self.pad {
            Some(pad) => dispatch(pad, &mut self.out_view, kbm, out),
            None => out.iter().for_each(|ev| self.out_view.apply(ev)),
        }
    }

    /// Runs `events` through the engine under `base` and sends what comes out, then plays the
    /// rumble a gyro toggle asked for.
    fn feed(&mut self, config: &Config, base: &Profile, events: Vec<InputEvent>, kbm: &mut VirtualKbm) {
        let mut out = Vec::new();
        for ev in events {
            self.view.apply(&ev);
            self.engine.handle(layered(&self.layered, base), ev, Instant::now(), &mut out);
            // A layer started or ended: the next events use it, and sticks, triggers and
            // gyro switch modes right away.
            if self.refresh_layers(config) {
                self.engine.resync(layered(&self.layered, base), &mut out);
            }
        }
        self.dispatch(kbm, out);
        if let Some(on) = self.engine.take_gyro_rumble() {
            rumble::tick(self.path.clone(), on);
        }
    }

    /// The controller's input as the GUI's live view receives it.
    fn snapshot(&self) -> InputSnapshot {
        InputSnapshot { model: self.model, family: self.family, gyro_out: self.engine.gyro_out(), ..self.view.snapshot(&self.name) }
    }

    /// Brings `layered` up to date with the engine's mode and layers. True if they changed.
    fn refresh_layers(&mut self, config: &Config) -> bool {
        let shifts: Shifts = (self.engine.mode().map(str::to_string), self.engine.layers());
        let current = self.layered.as_ref().map(|(shifts, _)| shifts);
        let plain = shifts.0.is_none() && shifts.1.is_empty() && !config.shortcuts.injects();
        if current == Some(&shifts) || (current.is_none() && plain) {
            return false;
        }
        self.layered = if plain {
            None
        } else {
            let game = config.active_game();
            let mode = shifts.0.as_deref().and_then(|m| config.mode_for(game, m));
            config.active().map(|base| {
                let mut profile = base.with_layers(mode.iter().map(|m| &**m).chain(game.layers_named(&shifts.1)));
                config.shortcuts.inject(&mut profile);
                (shifts.clone(), profile)
            })
        };
        true
    }

    /// Terminal status line: physical input, then what the mapping outputs.
    fn draw_status(&self) {
        if monitor::enabled() {
            monitor::show(&format!("{} → {}", self.view.render(&self.name), self.out_view.render()));
        }
    }
}

struct SeenGamepad {
    name: String,
    analog_triggers: bool,
    rumble: bool,
    paddles: bool,
}

/// What an engine asked the daemon to do after input or a timer.
struct Requests {
    overlay: Option<Layout>,
    screenshot: bool,
    recording: bool,
    media: bool,
    settings: bool,
    menu: Option<(String, Opener)>,
}

impl Requests {
    fn take(engine: &mut Engine) -> Self {
        Requests {
            overlay: engine.take_overlay_toggle(),
            screenshot: engine.take_screenshot(),
            recording: engine.take_recording_toggle(),
            media: engine.take_media_toggle(),
            settings: engine.take_settings_request(),
            menu: engine.take_menu_request(),
        }
    }
}

/// Who drives the overlay, and what the on-screen layouts remember between uses.
#[derive(Default)]
struct Driving {
    /// The controller driving the keyboard, numpad, media controls or offer: the one that
    /// opened it, else the first to send it input. Menus keep their own (see
    /// [`Daemon::drives_overlay`]).
    owner: Option<u64>,
    /// Where each on-screen layout's cursor was left, to pick up there next time.
    cursors: HashMap<crate::keyboard::Board, crate::keyboard::Cursor>,
}

/// What the overlay is showing.
enum Active {
    Keyboard(OverlayController),
    /// A menu, and the controller that opened it (its engine runs the chosen item).
    Menu { session: MenuSession, device: u64 },
    /// The media controls at the top of the screen.
    Media(MediaSession),
    /// The offer to add a library game's profiles.
    Offer(OfferSession),
    /// The menu Guide + Start opens: Quick Settings and Edit Controls.
    Settings(SystemMenu, u64),
}

/// The sounds of the on-screen keyboard or the numpad, whichever is up.
fn keyboard_sound(layout: Layout) -> sound::SoundOverlay {
    match layout {
        Layout::Keyboard => sound::SoundOverlay::Keyboard,
        Layout::Numpad => sound::SoundOverlay::Numpad,
    }
}

/// The sounds of whatever overlay is up.
fn active_sound(active: &Active) -> sound::SoundOverlay {
    match active {
        Active::Keyboard(k) => keyboard_sound(k.layout()),
        Active::Menu { .. } => sound::SoundOverlay::Menu,
        Active::Settings(..) => sound::SoundOverlay::Guide,
        Active::Media(_) => sound::SoundOverlay::Media,
        Active::Offer(_) => sound::SoundOverlay::Offer,
    }
}

/// What a press does on the media controls: a step for the volume and seek, a pick for the
/// buttons that act.
fn media_feedback(ev: InputEvent) -> Option<sound::Feedback> {
    use crate::config::Button;
    match ev {
        InputEvent::Button(Button::DpadUp | Button::DpadDown | Button::DpadLeft | Button::DpadRight, true) => {
            Some(sound::Feedback::Step)
        }
        InputEvent::Button(Button::South | Button::North | Button::LeftBumper | Button::RightBumper, true) => {
            Some(sound::Feedback::Pick)
        }
        _ => None,
    }
}

/// What the daemon has noticed about the controllers coming and going.
#[derive(Default)]
struct PadWatch {
    /// Their features as the last scan saw them, and as last settled on.
    seen: Option<Vec<Feature>>,
    settled: Option<Vec<Feature>>,
    /// A profile left for lacking features (what it was, what replaced it), so it returns
    /// when they come back.
    fell_back: Option<(ProfileRef, ProfileRef)>,
    /// Whether the first scan (at startup) is over, after which new controllers are announced.
    scanned: bool,
}

/// Where the offer to add a library game stands.
#[derive(Default)]
struct Offers {
    /// When Guide went down, while nothing else has been pressed with it.
    guide_tap: Option<Instant>,
    /// The library entries the offer on screen is for, and the process of the window it's over.
    shown: Vec<crate::library::Entry>,
    shown_pid: u32,
    /// Library packs answered "not now" to, with the game process they were declined for: they
    /// are offered again once the game has started again, as a new process.
    declined: HashMap<String, u32>,
    /// Game windows (by process, class and executable) already told that Guide adds their
    /// library setup, so switching back to one doesn't say it again.
    hinted: HashSet<(u32, String, String)>,
}


/// The Guide overlay's menu while it's up: the cursor, and whether Steam was running when it
/// opened (the Steam overlay row is listed only then). That is learned off the daemon thread,
/// so the row joins the menu a moment after it opens.
struct GuideMenu {
    cursor: usize,
    steam: bool,
    /// The page of Mappings on show.
    page: usize,
    /// The mode the Mappings show: 0 is the default, then the game's modes in order.
    mode: usize,
    /// Set once an item is chosen: its pick is what's heard, not the menu closing behind it.
    chosen: bool,
}

struct Daemon {
    config: Config,
    /// What the active profile can use: its game's items.
    scope: Scope,
    /// The fonts the active game carries, as files the overlay loads.
    pack_fonts: std::collections::BTreeMap<String, PathBuf>,
    sounds: sound::Sounds,
    kbm: VirtualKbm,
    devices: HashMap<u64, Managed>,
    /// The virtual pad kept while no controller feeds it (see `first_pad.rs`).
    spare_pad: Option<VirtualPad>,
    next_id: u64,
    /// Whether injected controllers are accepted, and how many there have been (see `inject.rs`).
    injection: inject::Injection,
    /// The identity a debug session forced on the virtual pads (see [`Request::DebugIdentify`]).
    identity_override: Option<PadIdentity>,
    /// Gamepads we have seen, managed or not, for status reporting.
    gamepads: HashMap<NodeKey, SeenGamepad>,
    /// The other input nodes the scans found (see `scan_devices`).
    nodes: devices::Nodes,
    /// Clients streaming live input (the GUI's controller view).
    watchers: Vec<Sender<Option<InputSnapshot>>>,
    /// Clients streaming the latest presses and what they did (the GUI's "what's happening").
    feed_watchers: Vec<Sender<Option<Feeds>>>,
    /// Profile switches and why they were made, for the GUI.
    activity: activity::Activity,
    /// Device whose input watchers are shown.
    last_active: Option<u64>,
    last_draw: Instant,
    focus_backend: FocusBackend,
    focused: Option<WindowInfo>,
    /// Focus moved while the in-game menu was open; it's handled once the menu closes.
    focus_waited: bool,
    /// The active profile is locked: automatic switches are held until it changes by hand.
    profile_locked: bool,
    recent_windows: Vec<WindowInfo>,
    /// Last profile chosen by process scanning, so manual switches stick until it changes.
    scan_target: Option<ProfileRef>,
    /// What the on-screen overlay is showing; while set it gets all controller input.
    active: Option<Active>,
    /// Who drives the overlay, and where the keyboard and numpad cursors were left.
    driving: Driving,
    /// When a held ForceQuit started counting, and whether it has already fired.
    quit_hold: Option<(Instant, bool)>,
    /// The screen recorder's process, once running, and whether one is being started.
    recording: Option<u32>,
    recording_starting: bool,
    offers: Offers,
    pads: PadWatch,
    /// The game (or other program) that was in front when recording started, and its process:
    /// the recording ends when it does.
    recording_target: Option<(String, u32)>,
    /// When each active layer came on, to delay its indicator.
    layer_started: HashMap<String, Instant>,
    overlay_watchers: Vec<Sender<OverlayFrame>>,
    /// The frame last sent to the overlay window, to skip sending it again unchanged.
    last_frame: Option<OverlayFrame>,
    /// What the controller driving the in-game menu is doing, for the editor's pictures, and when
    /// its turning last redrew them.
    menu_live: crate::figures::Live,
    live_drawn: Option<Instant>,
    /// The Guide overlay's menu while the overlay is up.
    guide_menu: Option<GuideMenu>,
    /// When shown info overlays with live values next refresh.
    info_refresh: Option<Instant>,
    /// When an info overlay that hides after a while next needs redrawing (fading or gone).
    info_fade: Option<Instant>,
    /// Info overlays shown for a while: at the game's start, or lingering after being let go.
    info_timers: crate::info::Timers,
    /// A notice at the top of the screen, such as the profile just switched to.
    toast: Option<crate::info::Toast>,
    /// Log overlays lingering after being let go.
    log_timers: crate::info::Timers,
    /// When a shown log or `{current_input}` cell next changes on its own.
    log_redraw: Option<Instant>,
    /// Something on screen shows the input log, so new input redraws it.
    shows_input: bool,
    /// Each game's process when it last started, so focusing it again isn't another start.
    launches: HashMap<String, u32>,
    sampler: crate::info::Sampler,
    /// When the overlay window was last started, so a window that can't start (no
    /// layer-shell) isn't restarted every second for info overlays.
    overlay_started: Option<Instant>,
    overlay_process: Option<std::process::Child>,
    /// The settings window, opened from the tray; one at a time.
    settings_process: Option<std::process::Child>,
    tray: Option<TrayIcon>,
    tx: Sender<Msg>,
}

/// Starts the daemon in the background unless one is already running, so opening the settings
/// window is enough to get remapping going. The daemon doesn't belong to the window: it keeps
/// running after the window closes, and its output goes to a log file.
pub fn ensure_running() -> Result<()> {
    if UnixStream::connect(ipc::socket_path()).is_ok() {
        return Ok(());
    }
    let log = log_path();
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let stderr = std::fs::File::create(&log).with_context(|| format!("creating {}", log.display()))?;
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.arg("daemon").stdin(Stdio::null()).stdout(Stdio::null()).stderr(stderr);
    os::detach(&mut command);
    // The child is not waited for: it outlives this process by design.
    command.spawn().context("starting the daemon")?;
    Ok(())
}

fn log_path() -> PathBuf {
    dirs::state_dir().or_else(dirs::data_local_dir).unwrap_or_else(std::env::temp_dir).join("padwight").join("daemon.log")
}

impl Daemon {
    fn new(config: Config, kbm: VirtualKbm, injection: inject::Injection, tx: Sender<Msg>) -> Self {
        crate::info::set_opener(crate::info::OpenerLabel::of(&config.shortcuts));
        crate::info::set_menu_keys(config.shortcuts.menu_keys());
        let pack_fonts = config.active_game().assets.activate_headless();
        Daemon {
            pack_fonts,
            scope: config.scope(),
            sounds: sound::Sounds::start(),
            config,
            kbm,
            devices: HashMap::new(),
            spare_pad: None,
            next_id: 0,
            injection,
            identity_override: None,
            gamepads: HashMap::new(),
            nodes: devices::Nodes::default(),
            watchers: Vec::new(),
            feed_watchers: Vec::new(),
            activity: activity::Activity::default(),
            last_active: None,
            last_draw: Instant::now(),
            focus_backend: FocusBackend::ProcessScan,
            focused: None,
            focus_waited: false,
            profile_locked: false,
            recent_windows: Vec::new(),
            scan_target: None,
            active: None,
            driving: Driving::default(),
            quit_hold: None,
            recording: None,
            recording_starting: false,
            offers: Offers::default(),
            pads: PadWatch::default(),
            recording_target: None,
            layer_started: HashMap::new(),
            overlay_watchers: Vec::new(),
            last_frame: None,
            menu_live: crate::figures::Live::default(),
            live_drawn: None,
            guide_menu: None,
            info_refresh: None,
            info_fade: None,
            info_timers: crate::info::Timers::default(),
            toast: None,
            log_timers: crate::info::Timers::default(),
            log_redraw: None,
            shows_input: false,
            launches: HashMap::new(),
            sampler: crate::info::Sampler::default(),
            overlay_started: None,
            overlay_process: None,
            settings_process: None,
            tray: None,
            tx,
        }
    }
}

/// Runs the service. With `debug`, controllers can also be injected over IPC (`padwight debug`).
pub fn run(debug: bool) -> Result<()> {
    let config = Config::load()?;
    config.log_unreadable();
    let (tx, rx) = mpsc::channel();
    let stop = tx.clone();
    os::stop_on_signals(move || stop.send(Msg::Stop).is_ok())?;
    serve_ipc(tx.clone())?;

    let kbm = VirtualKbm::new().map_err(explain_kbm_failure)?;
    let mut daemon = Daemon::new(config, kbm, inject::Injection::new(debug)?, tx);
    log!("padwight daemon started, socket at {}", ipc::socket_path().display());
    daemon.start_services();
    daemon.scan();
    daemon.run(&rx);
    Ok(())
}

/// Logs what to change when the virtual keyboard and mouse can't be made, which on Linux is almost
/// always /dev/uinput.
#[cfg(target_os = "linux")]
fn explain_kbm_failure(e: anyhow::Error) -> anyhow::Error {
    let report = crate::doctor::problems_report();
    if !report.is_empty() {
        log!("padwight can't start:\n{report}Run `padwight doctor --explain` for what each of these is for.");
    }
    e.context("creating virtual keyboard/mouse (run `padwight doctor` for what to change)")
}

#[cfg(windows)]
fn explain_kbm_failure(e: anyhow::Error) -> anyhow::Error {
    e.context("creating virtual keyboard/mouse")
}

/// Binds the control socket and serves it on a thread.
fn serve_ipc(tx: Sender<Msg>) -> Result<()> {
    let listener = bind_socket()?;
    thread::spawn(move || serve::ipc_server(&listener, &tx));
    Ok(())
}

/// Binds the control socket so that only this user can connect (see `os::bind_socket_at`).
fn bind_socket() -> Result<UnixListener> {
    os::bind_socket_at(&ipc::socket_path())
}

impl Daemon {
    fn run(&mut self, rx: &Receiver<Msg>) {
        let mut next_scan = Instant::now() + SCAN_INTERVAL;
        let mut last_tick = Instant::now();
        loop {
            let ticking = self.needs_tick();
            let mut timeout = if ticking {
                TICK.saturating_sub(last_tick.elapsed())
            } else {
                next_scan.saturating_duration_since(Instant::now())
            };
            if let Some(deadline) = self.next_deadline() {
                timeout = timeout.min(deadline.saturating_duration_since(Instant::now()));
            }
            match rx.recv_timeout(timeout) {
                Ok(msg) => {
                    self.handle(msg);
                    while let Ok(msg) = rx.try_recv() {
                        self.handle(msg);
                    }
                    self.sync_tray();
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }

            self.run_timers();
            self.catch_up_on_focus();
            let now = Instant::now();
            if ticking {
                let dt = now.duration_since(last_tick);
                if dt >= TICK {
                    // Clamp so a stall never turns into a huge cursor jump.
                    self.tick(dt.min(Duration::from_millis(20)).as_secs_f32());
                    last_tick = now;
                }
            } else {
                last_tick = now;
            }
            if now >= next_scan {
                self.scan();
                // A release the kernel refused is sent again here, not only on the next key.
                if let Err(e) = self.kbm.sync() {
                    log!("output error: {e:#}");
                }
                next_scan = now + SCAN_INTERVAL;
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let overlay = match &self.active {
            Some(Active::Keyboard(k)) => k.next_deadline(),
            Some(Active::Menu { session, .. }) => session.next_deadline(),
            Some(Active::Media(m)) => m.next_deadline(),
            Some(Active::Offer(o)) => o.next_deadline(),
            Some(Active::Settings(m, _)) => m.next_deadline(),
            None => None,
        };
        self.devices
            .values()
            .filter_map(|d| d.engine.next_deadline())
            .chain(overlay)
            .chain(self.info_refresh)
            .chain(self.info_fade)
            .chain(self.log_redraw)
            .chain(self.quit_hold.filter(|(_, done)| !done).map(|(since, _)| since + FORCE_QUIT_HOLD))
            .min()
    }

    /// Fires combo members whose combo window has expired.
    fn run_timers(&mut self) {
        let now = Instant::now();
        self.tick_overlays(now);
        let Some(base) = self.config.active() else { return };
        let mut requests = Vec::new();
        let mut fired = Vec::new();
        let mut guide_keys = Vec::new();
        for (id, dev) in self.devices.iter_mut() {
            if dev.engine.next_deadline().is_none_or(|d| d > now) {
                continue;
            }
            fired.push(*id);
            let mut out = Vec::new();
            dev.engine.timers(layered(&dev.layered, base), now, &mut out);
            let keys = dev.engine.take_guide_nav();
            if !keys.is_empty() {
                guide_keys.push((*id, keys));
            }
            if dev.refresh_layers(&self.config) {
                dev.engine.resync(layered(&dev.layered, base), &mut out);
            }
            dev.dispatch(&mut self.kbm, out);
            requests.push((*id, Requests::take(&mut dev.engine)));
            dev.draw_status();
        }
        for (id, asked) in requests {
            self.run_requests(id, asked);
        }
        for (id, keys) in guide_keys {
            self.guide_keys(id, &keys);
        }
        // A gesture or combo window ran out: its action labels the press it came from.
        if fired.into_iter().fold(false, |any, id| self.log_fired(id) | any) {
            self.log_changed();
        }
        self.check_info_changes();
    }

    /// Starts, cancels or completes the countdown of a ForceQuit action being held.
    fn update_force_quit(&mut self) {
        let held = self.devices.values().any(|d| d.engine.force_quit_held());
        let now = Instant::now();
        match (held, self.quit_hold) {
            // Let go before it fired: the countdown goes at once.
            (false, Some((_, false))) => {
                self.quit_hold = None;
                self.toast = None;
                self.broadcast_overlay();
            }
            (false, _) => self.quit_hold = None,
            (true, None) => {
                self.quit_hold = Some((now, false));
                let game = self.focused_target().map_or(String::new(), |(game, _)| format!(" {game}"));
                self.toast = Some(crate::info::Toast::filling(vec![format!("Keep holding to force quit{game}…")], now, FORCE_QUIT_HOLD));
                self.broadcast_overlay();
            }
            (true, Some((since, false))) if now.duration_since(since) >= FORCE_QUIT_HOLD => {
                self.quit_hold = Some((since, true));
                let lines = match self.focused_target() {
                    Some((game, pid)) => {
                        log!("force quitting {game} (pid {pid})");
                        crate::quit::force_quit(pid);
                        vec![format!("Force quit {game}")]
                    }
                    None => vec!["Nothing to force quit".into(), "The desktop and this app are never ended.".into()],
                };
                self.say(lines);
            }
            (true, Some(_)) => {}
        }
    }

    /// Shows `lines` in a toast.
    fn say(&mut self, lines: Vec<String>) {
        self.toast = Some(crate::info::Toast::new(lines, Instant::now()));
        self.broadcast_overlay();
    }

    /// Ends a recording (saving it) once the program that was in front when it started is gone,
    /// so closing the game doesn't leave the screen being recorded.
    fn check_recorded_game(&mut self) {
        let (Some(pid), Some((name, target))) = (self.recording, &self.recording_target) else { return };
        if crate::quit::is_running(*target) {
            return;
        }
        let lines = vec![format!("{name} closed"), "Saving the recording…".into()];
        log!("{name} (pid {target}) exited, ending the recording");
        self.recording_target = None;
        crate::record::stop(pid);
        self.say(lines);
    }

    /// Starts recording in the background, or asks a running recording to finish.
    fn toggle_recording(&mut self) {
        if let Some(pid) = self.recording {
            crate::record::stop(pid);
            self.say(vec!["Stopping recording…".into()]);
            return;
        }
        if std::mem::replace(&mut self.recording_starting, true) {
            return;
        }
        self.recording_target = self.focused_target();
        let (game, tx) = (self.config.active_game().name.clone(), self.tx.clone());
        thread::spawn(move || match crate::record::start(&game) {
            Ok((mut child, path)) => {
                let pid = child.id();
                let _ = tx.send(Msg::RecordingStarted { pid });
                let ok = child.wait().is_ok_and(|s| s.success());
                let _ = tx.send(Msg::RecordingEnded { pid, ok, path });
            }
            Err(e) => {
                let _ = tx.send(Msg::RecordingFailed(format!("{e:#}")));
            }
        });
    }

    /// Takes a screenshot in the background and says where it went (or why not) in a toast.
    fn take_screenshot(&self) {
        let (game, tx) = (self.config.active_game().name.clone(), self.tx.clone());
        thread::spawn(move || {
            let lines = match crate::capture::screenshot(&game) {
                Ok(path) => {
                    log!("screenshot saved to {}", path.display());
                    vec!["Screenshot saved".to_string(), path.display().to_string()]
                }
                Err(e) => {
                    log!("screenshot failed: {e:#}");
                    vec!["Screenshot failed".to_string(), format!("{e:#}")]
                }
            };
            let _ = tx.send(Msg::Toast(lines));
        });
    }

    /// Starts what runs beside the input loop: focus tracking, the overlay window (started now so
    /// menus appear instantly; it idles invisibly) and the tray icon.
    fn start_services(&mut self) {
        rumble::set_strength(self.config.rumble_strength);
        let tx = self.tx.clone();
        focus::spawn(move |ev| {
            let _ = tx.send(Msg::Focus(ev));
        });
        self.ensure_overlay_process();
        self.tray = self.start_tray();
    }

    fn start_tray(&self) -> Option<TrayIcon> {
        if !tray::wanted() {
            return None;
        }
        let tx = self.tx.clone();
        let icon = TrayIcon::start(self.tray_shown(), move |cmd| {
            let _ = tx.send(Msg::Tray(cmd));
        });
        if icon.is_none() {
            log!("no system tray to show the icon in; running without one");
        }
        icon
    }

    fn tray_shown(&self) -> tray::Shown {
        tray::Shown {
            enabled: self.config.enabled,
            profile: self.config.active().map(|p| p.name.clone()).unwrap_or_default(),
        }
    }

    fn sync_tray(&mut self) {
        let shown = self.tray_shown();
        if let Some(icon) = &mut self.tray {
            icon.show(&shown);
        }
    }

    fn tray_command(&mut self, cmd: tray::Command) {
        match cmd {
            tray::Command::ToggleEnabled => {
                self.request(Request::SetEnabled(!self.config.enabled));
            }
            tray::Command::OpenSettings => self.open_settings(),
            tray::Command::Quit => {
                log!("quit from the tray");
                self.shut_down();
            }
        }
    }

    /// Releases every key, button and pad input the daemon holds, then exits. Without this the
    /// keys stay down until the desktop notices the virtual devices have gone, which not every
    /// desktop does.
    fn shut_down(&mut self) -> ! {
        if let Some(mut overlay) = self.overlay_process.take() {
            let _ = overlay.kill();
        }
        self.release_mappings();
        if let Err(e) = self.kbm.release_all() {
            log!("output error at shutdown: {e:#}");
        }
        log!("released what was held; exiting");
        drop(self.tray.take());
        std::process::exit(0);
    }

    fn open_settings(&mut self) {
        if let Some(child) = &mut self.settings_process
            && child.try_wait().ok().flatten().is_none()
        {
            return;
        }
        // Not tied to the daemon's life: a signal would skip the window's unsaved-changes check.
        // The window notices the daemon has gone and closes itself the same way, however it was
        // opened.
        match os::windowless(&mut std::process::Command::new(std::env::current_exe().unwrap_or_default())).spawn() {
            Ok(child) => self.settings_process = Some(child),
            Err(e) => log!("cannot open the settings window: {e}"),
        }
    }

    /// Carries out what controller `id`'s engine asked for, opening anything on its behalf.
    fn run_requests(&mut self, id: u64, asked: Requests) {
        if let Some(layout) = asked.overlay {
            self.toggle_overlay(layout, Some(id));
        }
        if asked.screenshot {
            self.take_screenshot();
        }
        if asked.recording {
            self.toggle_recording();
        }
        if asked.media {
            self.toggle_media(Some(id));
        }
        if asked.settings && self.active.is_none() {
            self.open_system_menu(id, Page::Main);
        }
        self.update_force_quit();
        if let Some((name, opener)) = asked.menu {
            self.open_menu(id, &name, opener);
        }
    }

    fn needs_tick(&self) -> bool {
        let Some(base) = self.config.active() else { return false };
        self.devices.values().any(|d| d.engine.needs_tick(layered(&d.layered, base)))
    }

    fn tick(&mut self, dt: f32) {
        let Some(base) = self.config.active() else { return };
        for dev in self.devices.values_mut() {
            let mut out = Vec::new();
            dev.engine.tick(layered(&dev.layered, base), dt, &mut out);
            dev.dispatch(&mut self.kbm, out);
            dev.out_view.end_tick(dt);
        }
        if self.last_draw.elapsed() >= STATUS_REDRAW
            && let Some(dev) = self.last_active.and_then(|id| self.devices.get(&id))
        {
            dev.draw_status();
            self.last_draw = Instant::now();
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Stop => self.shut_down(),
            Msg::Input { id, events } => self.input(id, events),
            Msg::Gone { id } => self.device_gone(id),
            Msg::Ipc { req, reply } => {
                let _ = reply.send(self.request(req));
            }
            Msg::Tray(cmd) => self.tray_command(cmd),
            Msg::Focus(event) => self.focus_event(event),
            Msg::Toast(lines) => self.say(lines),
            Msg::Media(state) => self.update_media(state),
            Msg::SteamRunning(running) => self.guide_menu_steam(running),
            Msg::RecordingStarted { pid } => {
                (self.recording, self.recording_starting) = (Some(pid), false);
                self.say(vec!["Recording started".into()]);
            }
            Msg::RecordingFailed(reason) => {
                self.recording_starting = false;
                self.recording_target = None;
                log!("recording failed: {reason}");
                self.say(vec!["Recording failed".into(), reason]);
            }
            Msg::RecordingEnded { pid, ok, path } => {
                if self.recording == Some(pid) {
                    self.recording = None;
                    self.recording_target = None;
                }
                log!("recording ended ({}): {}", if ok { "saved" } else { "failed" }, path.display());
                let head = if ok { "Recording saved" } else { "Recording ended unexpectedly" };
                self.say(vec![head.into(), path.display().to_string()]);
            }
            Msg::Motion { id, sample } => self.motion(id, sample),
            Msg::WatchOverlay(watcher) => {
                if watcher.send(self.overlay_frame()).is_ok() {
                    self.overlay_watchers.push(watcher);
                }
            }
            Msg::MotionGone { id } => {
                if let Some(dev) = self.devices.get_mut(&id) {
                    dev.motion = None;
                }
            }
            Msg::TouchpadGone { id } => {
                if let Some(dev) = self.devices.get_mut(&id) {
                    dev.touchpad = None;
                }
            }
            Msg::Watch(watcher) => {
                let current = self.last_active.and_then(|id| self.devices.get(&id));
                if watcher.send(current.map(Managed::snapshot)).is_ok() {
                    self.watchers.push(watcher);
                }
            }
            Msg::WatchFeed(watcher) => self.watch_feed(watcher),
        }
    }

    /// A controller was unplugged or dropped off (not one we let go of ourselves).
    fn device_gone(&mut self, id: u64) {
        if let Some(mut dev) = self.devices.remove(&id) {
            // Its motion and rumble threads follow it out.
            dev.stop_threads();
            monitor::clear();
            log!("device gone: {} ({})", dev.name, dev.path.display());
            self.forget_active(id);
            let mut out = Vec::new();
            dev.engine.release_all(false, &mut out);
            dev.dispatch(&mut self.kbm, out);
            // A menu is run by the controller that opened it, and with no controller
            // left nothing could close an overlay. The in-game menu waits for one instead.
            let orphaned = match &self.active {
                Some(Active::Menu { device, .. }) => *device == id || self.devices.is_empty(),
                Some(Active::Settings(..)) => false,
                Some(_) => self.devices.is_empty(),
                None => false,
            };
            if orphaned {
                self.close_overlay();
            }
            self.say(vec![format!("{} disconnected", dev.name)]);
        }
        self.keep_first_pad();
    }

    fn motion(&mut self, id: u64, mut sample: MotionSample) {
        let Some(dev) = self.devices.get_mut(&id) else { return };
        if let Some(cal) = &mut dev.calibrating {
            for (sum, v) in cal.sum.iter_mut().zip(sample.gyro) {
                *sum += v as f64;
            }
            cal.samples += 1;
            if cal.started.elapsed() < GYRO_CALIBRATION {
                return;
            }
            let n = cal.samples.max(1) as f64;
            let bias = cal.sum.map(|s| (s / n) as f32);
            dev.gyro_bias = bias;
            dev.calibrating = None;
            let name = dev.name.clone();
            log!("gyro calibrated for {name}: bias {bias:.2?} deg/s");
            self.config.gyro_calibration.insert(name, bias);
            if let Err(e) = self.config.save_settings() {
                log!("saving config: {e:#}");
            }
            return;
        }
        for (v, bias) in sample.gyro.iter_mut().zip(dev.gyro_bias) {
            *v -= bias;
        }
        let sample = dev.motion_frame.to_standard(sample);
        dev.view.set_gyro(sample.gyro);
        if matches!(self.active, Some(Active::Settings(_, driver)) if driver == id) {
            self.menu_turned(sample.gyro);
            return;
        }
        let Some(dev) = self.devices.get_mut(&id) else { return };
        let Some(base) = self.config.active().filter(|_| self.active.is_none()) else { return };
        let mut out = Vec::new();
        dev.engine.motion(layered(&dev.layered, base), sample, &mut out);
        dev.dispatch(&mut self.kbm, out);
        if self.last_draw.elapsed() >= STATUS_REDRAW {
            dev.draw_status();
            self.last_draw = Instant::now();
        }
        if self.last_active == Some(id) && !self.watchers.is_empty() {
            let snapshot = dev.snapshot();
            self.watchers.retain(|w| w.send(Some(snapshot.clone())).is_ok());
        }
    }

    /// Briefly says which controller profile is now active, and for which game, at the top
    /// of the screen.
    fn toast_profile(&mut self) {
        let active = self.config.active_ref();
        let lines = [format!("Controller profile active: {}", active.profile)]
            .into_iter()
            .chain(active.game.map(|game| format!("Setup: {game}")))
            .collect();
        self.toast = Some(crate::info::Toast::new(lines, Instant::now()));
        self.broadcast_overlay();
    }

    fn broadcast(&mut self, snapshot: Option<&InputSnapshot>) {
        self.watchers.retain(|w| w.send(snapshot.cloned()).is_ok());
    }

    fn input(&mut self, id: u64, events: Vec<InputEvent>) {
        // A controller let go of has a reader still running for a moment.
        if !self.devices.contains_key(&id) {
            return;
        }
        let logged = self.log_input(id, &events, Instant::now());
        let fires: Vec<bool> = events.iter().map(|ev| self.opener_fired(id, ev)).collect();
        let tapped = self.opener_tapped(&events, &fires);
        if self.take_over_settings(id, &events) {
            return;
        }
        if self.drives_overlay(id) {
            return self.overlay_input(id, events, &fires);
        }
        self.sync_face_swap(id);
        let Some(base) = self.config.active() else { return };
        let Some(dev) = self.devices.get_mut(&id) else { return };
        dev.feed(&self.config, base, events, &mut self.kbm);
        if !dev.engine.needs_tick(layered(&dev.layered, base)) {
            dev.out_view.stop_motion();
        }
        dev.draw_status();
        self.last_draw = Instant::now();
        self.last_active = Some(id);
        if !self.watchers.is_empty() {
            let snapshot = dev.snapshot();
            self.watchers.retain(|w| w.send(Some(snapshot.clone())).is_ok());
        }
        let asked = Requests::take(&mut dev.engine);
        let guide_keys = dev.engine.take_guide_nav();
        if dev.engine.take_touch_menu_changed() {
            self.broadcast_overlay();
        }
        self.run_requests(id, asked);
        if !guide_keys.is_empty() {
            self.guide_keys(id, &guide_keys);
        }
        if self.log_fired(id) | logged {
            self.log_changed();
        }
        self.check_info_changes();
        if tapped {
            self.offer_library_game(id);
        }
    }

    /// The button that is held to open the Guide overlay: Guide, or the one it was moved to. None
    /// when a gesture or combo opens it, or nothing does.
    fn guide_chord_button(&self) -> Option<crate::config::Button> {
        use crate::config::{Button, GuideOpener};
        match self.config.shortcuts.engine_opener() {
            GuideOpener::Guide => Some(Button::Guide),
            GuideOpener::Button(b) => Some(b),
            GuideOpener::Triggered(_) => None,
        }
    }

    /// The button a shortcut of the Guide mode sits on, if it's a plain button.
    fn shortcut_button(&self, s: Shortcut) -> Option<crate::config::Button> {
        self.config.shortcuts.input(s).filter(|i| i.buttons.len() == 1 && i.gesture.is_none()).map(|i| i.buttons[0])
    }

    /// Whether `ev` completes the input that opens the Guide overlay, on controller `id`.
    fn opener_fired(&mut self, id: u64, ev: &InputEvent) -> bool {
        let timing = self
            .config
            .active()
            .map(|p| crate::opener_watch::Timing { tap: Duration::from_millis(p.tap_window_ms), long: Duration::from_millis(p.long_press_ms) })
            .unwrap_or_default();
        self.devices.get_mut(&id).is_some_and(|d| d.opener_watch.feed(ev, Instant::now(), timing))
    }

    /// Whether these events use the Guide opener the way a tap does, which offers a library game
    /// over a game that has none. A button opener must go down and up in a moment with nothing
    /// pressed in between (a held one with another button is a shortcut, not a tap); a gesture
    /// or a combo counts when it completes.
    fn opener_tapped(&mut self, events: &[InputEvent], fires: &[bool]) -> bool {
        let now = Instant::now();
        let held_opener = self.guide_chord_button();
        let mut tapped = false;
        for (ev, fired) in events.iter().zip(fires) {
            let Some(opener) = held_opener else {
                tapped |= *fired;
                continue;
            };
            match *ev {
                InputEvent::Button(b, true) if b == opener => self.offers.guide_tap = Some(now),
                InputEvent::Button(b, false) if b == opener => {
                    tapped |= self.offers.guide_tap.take().is_some_and(|t| now.duration_since(t) <= GUIDE_TAP);
                }
                InputEvent::Button(_, true) => self.offers.guide_tap = None,
                _ => {}
            }
        }
        tapped
    }

    /// What the managed controllers can do beyond a plain XInput pad, together; `None` with
    /// none connected, when there is nothing to judge a profile's needs by.
    fn pad_features(&self) -> Option<Vec<Feature>> {
        (!self.devices.is_empty()).then(|| {
            let gyro = self.devices.values().any(|d| d.motion.is_some()).then_some(Feature::Gyro);
            let paddles = self.devices.values().any(|d| d.paddles).then_some(Feature::BackPaddles);
            let touchpad = self.devices.values().any(|d| d.touchpad.is_some()).then_some(Feature::Touchpad);
            gyro.into_iter().chain(paddles).chain(touchpad).collect()
        })
    }

    /// Recomputes what the active profile can use, after a config or game change.
    fn refresh_scope(&mut self) {
        self.scope = self.config.scope();
        self.pack_fonts = self.config.active_game().assets.activate_headless();
        rumble::set_strength(self.config.rumble_strength);
        for dev in self.devices.values_mut() {
            dev.engine.set_macros(&self.scope.macros);
            dev.engine.set_menus(&self.scope.menus);
            dev.engine.set_opener(self.config.shortcuts.engine_opener());
            dev.opener_watch.set(self.config.shortcuts.opener());
            dev.engine.set_menu_keys(self.config.shortcuts.menu_keys());
        }
        crate::info::set_opener(crate::info::OpenerLabel::of(&self.config.shortcuts));
        crate::info::set_menu_keys(self.config.shortcuts.menu_keys());
        self.sync_identities();
    }

    /// The controllers the active game supports, which decide what the virtual pads present as.
    fn active_support(&self) -> ControllerSupport {
        self.config.active_game().controllers.unwrap_or_default()
    }

    /// Tells pad `id`'s engine whether its face buttons send the Nintendo letters now, under the
    /// active profile: see [`PadIdentity::swaps_face_output`].
    fn sync_face_swap(&mut self, id: u64) {
        let swap = self.config.active().is_some_and(|base| self.face_output_swapped(id, base));
        if let Some(dev) = self.devices.get_mut(&id) {
            dev.engine.set_face_swap(swap);
        }
    }

    fn face_output_swapped(&self, id: u64, base: &Profile) -> bool {
        let Some(dev) = self.devices.get(&id) else { return false };
        let identity = PadIdentity::resolve(self.identity_override, dev.model, self.active_support());
        let nintendo_pad = dev.family == Some(crate::info::PadFamily::Nintendo);
        identity.swaps_face_output(nintendo_pad, self.config.active_nintendo_layout(), base.faces_default())
    }

    /// Makes every controller's virtual pad present the identity the active game asks for.
    fn sync_identities(&mut self) {
        let (forced, support) = (self.identity_override, self.active_support());
        for dev in self.devices.values_mut() {
            dev.present_as(PadIdentity::resolve(forced, dev.model, support));
        }
    }

    /// Switches profile, showing the new one in a toast. `why` says what made the switch.
    fn switch_profile(&mut self, target: ProfileRef, why: &str) {
        if self.set_profile(target, why) {
            self.toast_profile();
        }
    }

    /// Switches profile, recording `why`; false if it was already active.
    fn set_profile(&mut self, target: ProfileRef, why: &str) -> bool {
        if target == self.config.active_ref() {
            return false;
        }
        log!("switching to profile {target}");
        // A new profile starts unlocked: the lock was for the one that was active.
        self.profile_locked = false;
        // An open menu refers to menus by position, which another game's list changes.
        let game_changes = target.game != self.config.active_ref().game;
        if game_changes {
            self.info_timers.clear();
            self.log_timers.clear();
        }
        if game_changes && matches!(self.active, Some(Active::Menu { .. })) {
            self.close_overlay();
        }
        // Modes belong to a game: the mode stays on within it.
        self.release_all(!game_changes);
        self.activity.switched(Instant::now(), &target, why);
        self.config.active = target;
        if game_changes {
            self.refresh_scope();
        }
        self.resync_all();
        // Other info overlays may belong to the new profile.
        self.broadcast_overlay();
        // The in-game menu saves nothing until its Save, which writes this too: written now, it
        // would take the menu's unsaved settings (rumble, overlay size) with it, which Discard
        // then couldn't take back.
        if !matches!(self.active, Some(Active::Settings(..)))
            && let Err(e) = self.config.save_settings()
        {
            log!("saving config: {e:#}");
        }
        true
    }

    /// Releases every output held by every device (before a profile change).
    fn release_all(&mut self, keep_mode: bool) {
        for dev in self.devices.values_mut() {
            let mut out = Vec::new();
            dev.engine.release_all(keep_mode, &mut out);
            dev.dispatch(&mut self.kbm, out);
        }
        // A touch menu that was up goes away with the fingers.
        self.broadcast_overlay();
    }

    /// Re-applies analog state under the active profile (and each controller's layers, which
    /// are rebuilt from it).
    fn resync_all(&mut self) {
        let Some(base) = self.config.active() else { return };
        for dev in self.devices.values_mut() {
            dev.layered = None;
            dev.refresh_layers(&self.config);
            let mut out = Vec::new();
            dev.engine.resync(layered(&dev.layered, base), &mut out);
            dev.dispatch(&mut self.kbm, out);
        }
    }

    /// Whether a scan should take the controller called `name`: remapping is on and the user
    /// hasn't set it aside. Every path that grabs a pad asks this.
    fn should_manage(&self, name: &str) -> bool {
        self.config.enabled && !self.config.ignored_devices.iter().any(|n| n == name)
    }

    /// Stops managing (ungrabs) devices matching `pred`.
    fn release_devices(&mut self, pred: impl Fn(u64, &Managed) -> bool) {
        let ids: Vec<u64> = self.devices.iter().filter(|(id, d)| pred(**id, d)).map(|(id, _)| *id).collect();
        for id in ids {
            let Some(mut dev) = self.devices.remove(&id) else { continue };
            dev.stop_threads();
            monitor::clear();
            self.forget_active(id);
            let mut out = Vec::new();
            dev.engine.release_all(false, &mut out);
            dev.dispatch(&mut self.kbm, out);
            log!("released {} ({})", dev.name, dev.path.display());
        }
        self.keep_first_pad();
    }

    fn status(&self) -> Status {
        let mut devices: Vec<DeviceInfo> = self
            .gamepads
            .iter()
            .map(|((path, _), pad)| DeviceInfo {
                name: pad.name.clone(),
                path: path.display().to_string(),
                managed: self.devices.values().any(|d| &d.path == path),
                ignored: self.config.ignored_devices.contains(&pad.name),
                gyro: self.devices.values().any(|d| &d.path == path && d.motion.is_some()),
                analog_triggers: pad.analog_triggers,
                rumble: pad.rumble,
                paddles: pad.paddles,
                touchpad: self.devices.values().any(|d| &d.path == path && d.touchpad.is_some()),
                family: self.devices.values().find(|d| &d.path == path).and_then(|d| d.family),
                model: self.devices.values().find(|d| &d.path == path).and_then(|d| d.model),
            })
            .collect();
        devices.sort_by(|a, b| a.path.cmp(&b.path));
        devices.extend(self.injected_devices());
        Status {
            enabled: self.config.enabled,
            active_profile: self.config.active().map(|p| p.name.clone()).unwrap_or_default(),
            active_game: self.config.active_ref().game,
            active_layers: self.active_layers(),
            active_mode: self.active_mode(),
            devices,
            focus_backend: self.focus_backend,
            focused: self.focused.clone(),
            recent_windows: self.recent_windows.clone(),
            motion_access_denied: self.nodes.denied_sensors(),
            overlay_visible: matches!(&self.active, Some(Active::Keyboard(k)) if k.layout() == Layout::Keyboard),
            numpad_visible: matches!(&self.active, Some(Active::Keyboard(k)) if k.layout() == Layout::Numpad),
            profile_locked: self.profile_locked,
            switches: self.activity.switches(Instant::now()),
        }
    }

    /// Looks for new controllers and takes the ones we should manage, then catches up on the rest
    /// of what the scan watches.
    fn scan(&mut self) {
        let known: HashSet<u64> = self.devices.keys().copied().collect();
        self.refit_pads();
        self.keep_first_pad();
        self.scan_devices();
        self.check_overlay_process();
        self.check_recorded_game();
        self.scan_processes();
        self.announce_new_controllers(&known);
        self.settle_features();
    }

    /// Says which controllers were just connected (not at startup, when all of them are).
    fn announce_new_controllers(&mut self, known: &HashSet<u64>) {
        let new: Vec<String> = self
            .devices
            .iter()
            .filter(|(id, _)| !known.contains(id))
            .map(|(_, d)| if d.motion.is_some() { format!("{} (with gyro)", d.name) } else { d.name.clone() })
            .collect();
        if self.pads.scanned && !new.is_empty() {
            self.say(new.into_iter().map(|n| format!("{n} connected")).collect());
        }
        self.pads.scanned = true;
    }

    /// Once what the controllers can do has stayed the same for two scans (a pad's motion
    /// sensor shows up a moment after the pad itself), makes the active profile fit it.
    fn settle_features(&mut self) {
        let now = self.pad_features();
        if now == self.pads.seen && now != self.pads.settled {
            self.pads.settled.clone_from(&now);
            if let Some(have) = &now {
                self.fit_profile_to(have);
            }
        }
        self.pads.seen = now;
    }

    /// Controllers came or went, or swapped for ones with other features: a profile that needs
    /// something now missing gives way to one of its game's that doesn't (or to the default
    /// profile), and comes back when the feature does. With no controller at all nothing
    /// changes, so reconnecting the same one finds everything as it was.
    fn fit_profile_to(&mut self, have: &[Feature]) {
        match self.config.refit(have, self.pads.fell_back.as_ref()) {
            Refit::Keep => {}
            Refit::Forget => self.pads.fell_back = None,
            Refit::Leave { missing, to } => {
                let from = self.config.active_ref();
                let names: Vec<&str> = missing.iter().map(|f| f.label()).collect();
                log!("{from} needs {}, which the controllers lack → {to}", names.join(" and "));
                let lines = vec![format!("{} needs {}", from.profile, names.join(" and ")), format!("Using {} instead", to.profile)];
                let why = format!("{from} needs {}, which the controllers lack", names.join(" and "));
                if self.set_profile(to.clone(), &why) {
                    self.pads.fell_back = Some((from, to));
                    self.say(lines);
                }
            }
            Refit::Return(wanted) => {
                self.pads.fell_back = None;
                log!("controllers can play {wanted} again");
                if self.set_profile(wanted.clone(), "The controllers can play it again") {
                    self.say(vec![format!("Back to {}", wanted.profile)]);
                }
            }
        }
    }

    /// A controller with nothing known about it yet; `pad` is the virtual pad it feeds.
    fn new_managed(&mut self, path: PathBuf, name: String, pad: Option<VirtualPad>, stop: Arc<AtomicBool>) -> Managed {
        let mut engine = Engine::default();
        engine.set_macros(&self.scope.macros);
        engine.set_menus(&self.scope.menus);
        engine.set_opener(self.config.shortcuts.engine_opener());
        engine.set_menu_keys(self.config.shortcuts.menu_keys());
        let mut opener_watch = crate::opener_watch::Watch::default();
        opener_watch.set(self.config.shortcuts.opener());
        Managed {
            path,
            name,
            family: None,
            model: None,
            engine,
            pad,
            ff: None,
            rumble_stop: Arc::new(AtomicBool::new(false)),
            first: false,
            refit_at: None,
            recorded: None,
            stop,
            view: InputView::default(),
            out_view: OutputView::default(),
            parent: None,
            #[cfg(target_os = "linux")]
            uniq: None,
            motion: None,
            touchpad: None,
            #[cfg(target_os = "linux")]
            grabs: Vec::new(),
            paddles: false,
            motion_frame: MotionFrame::default(),
            gyro_bias: [0.0; 3],
            calibrating: None,
            layered: None,
            opener_watch,
            log: crate::inputlog::InputLog::default(),
            exports: crate::inputlog::InputLog::default(),
            number: logs::free_number(self.devices.values().map(|d| d.number)),
        }
    }

    /// Starts running `managed` as controller `id`, with its mappings in step with the profile.
    fn add_managed(&mut self, id: u64, mut managed: Managed) {
        if let Some(profile) = self.config.active() {
            let mut out = Vec::new();
            managed.engine.resync(profile, &mut out);
            managed.dispatch(&mut self.kbm, out);
        }
        self.devices.insert(id, managed);
    }
}

fn layout_name(layout: Layout) -> &'static str {
    match layout {
        Layout::Keyboard => "keyboard",
        Layout::Numpad => "numpad",
    }
}

fn dispatch(pad: &mut VirtualPad, view: &mut OutputView, kbm: &mut VirtualKbm, out: Vec<OutEvent>) {
    for ev in out {
        view.apply(&ev);
        let res = match ev {
            OutEvent::PadButton(b, pressed) => pad.button(b, pressed),
            OutEvent::PadAxis(axis, v) => pad.axis(axis, v),
            OutEvent::Key(k, pressed) => kbm.key(k, pressed),
            OutEvent::MouseButton(b, pressed) => kbm.mouse_button(b, pressed),
            OutEvent::MouseMove(dx, dy) => kbm.mouse_move(dx, dy),
            OutEvent::Wheel { vertical, horizontal } => kbm.wheel(vertical, horizontal),
        };
        if let Err(e) = res {
            log!("output error: {e:#}");
        }
    }
}

/// The reason for switching to `target`: `why`, plus what changed when the controller can't play
/// the rule's `picked` profile and another one is used instead.
fn with_fallback(why: String, picked: &ProfileRef, target: &ProfileRef) -> String {
    if picked == target {
        return why;
    }
    format!("{why}, but the controller can't play {} so {} is used", picked.profile, target.profile)
}

/// Short description of a window for log lines.
fn describe(w: &WindowInfo) -> String {
    match &w.steam_app_id {
        Some(id) => format!("{} (Steam {id})", w.exe),
        None if w.exe.is_empty() => w.class.clone(),
        None => w.exe.clone(),
    }
}
