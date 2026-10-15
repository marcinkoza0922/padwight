//! iced front-end. Talks to the daemon over IPC; falls back to editing the config file directly
//! when the daemon is not running.

use std::{
    borrow::Borrow,
    collections::HashSet,
    fmt,
    mem::Discriminant,
    rc::Rc,
    str::FromStr,
    time::{Duration, Instant},
};

use crate::keycode::KeyCode;
use iced::{
    Alignment, Color, Element, Length, Subscription, Task,
    widget::{
        button, center, column, container, mouse_area, opaque, pick_list, row, rule,
        rich_text, scrollable, slider, space, span, stack, svg, text, text_input, tooltip,
    },
};

use crate::{
    config::{
        Analog, Appearance, Button, ButtonAction, CarouselControls, Cluster, Combo, Config, Feature, Game, GestureKind, GRID_MAX, GyroActivation,
        CurrentInput, GyroConfig, GyroHorizontal, GyroInput, GyroMode, InfoOverlay, ItemKind, LogOverlay, Macro, MacroStep, Menu, MenuItem,
        MenuKind, MenuKindTag, MouseButton, OverlayStyle, Paint, Profile, ProfileRef, Rule, RuleKind,
        ScreenPosition, Stick, StickAction, StickConfig, Toggled, TouchSlot, TouchpadConfig, Trigger, TriggerAction, WheelDirection, Template, Zone, free_name, unique_name, Motion, StickPreset,
    },
    engine::Opener,
    motion::{MotionStyle, OverlayKind},
    info::{PadFamily, PadModel},
    pad_identity::PadIdentity,
    ipc::{self, Feeds, InputSnapshot, Request, Response, Status, WindowInfo},
    keyboard::{self, Layout},
    launchers, library,
    menu::MenuSession,
    pack, pad_svg,
    sound::{OverlaySounds, SoundKind, SoundOverlay, SoundSet, SoundSpec},
    style,
};

mod actions;
mod activity;
mod chars;
mod checks;
mod font_menu;
mod games;
mod goto;
mod guide_tab;
mod shortcuts_tab;
mod gyro;
mod gyro_graph;
mod details;
mod overlay_cards;
mod items;
mod menu_combos;
mod motion_demo;
mod menu_grid;
#[cfg(test)]
mod menu_suite;
mod open_cards;
mod keys;
mod layers;
mod logs;
mod manual;
mod overlays;
mod deadzone_graph;
mod device_box;
mod pack_files;
mod pieces;
mod packs;
mod profile;
mod response_graph;
mod ring_preview;
mod search;
mod system_check;
pub(crate) mod shoot;
mod settings;
mod sound_picker;
mod sounds;
mod sliders;
mod sticks;
mod style_editor;
mod style_extras;
mod tab_icons;
mod touchpad;
mod tour;
mod tokens;
mod tracking;
mod triggers;
mod widgets;

use actions::*;
use checks::*;
use device_box::*;
use sounds::overlay_sounds;
use items::*;
use menu_grid::*;
use keys::*;
use chars::{CharPicker, char_button};
use font_menu::FontSlot;
use motion_demo::MotionDemo;
use sound_picker::{SoundPicker, SoundTarget};
use tokens::{TokenPicker, TokenTarget, insert_button};
use layers::IndicatorChoice;
use packs::{BrowseSource, Dialog, PackField};
use pieces::*;
use profile::*;
use settings::*;
use sliders::*;
use sticks::*;
use style_editor::*;
use tracking::*;
use triggers::*;
use goto::goto_button;
use tour::TourMove;
use search::*;
use widgets::*;

pub fn run() -> iced::Result {
    // scripts/headless.sh sets PADWIGHT_NO_DAEMON: a daemon started from there would outlive the
    // virtual display and take the real controllers.
    if std::env::var_os("PADWIGHT_NO_DAEMON").is_none()
        && let Err(e) = crate::daemon::ensure_running()
    {
        crate::monitor::log!("cannot start the daemon: {e:#}");
    }
    let app = iced::application(App::boot, App::update, App::view)
        .title("Padwight")
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window_settings());
    crate::font::BUNDLED.iter().fold(app, |app, b| app.font(b.bytes)).run()
}

/// The settings window. Closing it is asked about first (`Message::CloseRequested`) rather than
/// done by iced, so this must say so: `Application::window` replaces every window setting, and
/// would undo an earlier `exit_on_close_request(false)`.
fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        size: iced::Size::new(1280.0, 960.0),
        icon: crate::icon::window(),
        exit_on_close_request: false,
        ..Default::default()
    }
}

/// What every part of the window that needs the daemon says while it isn't running, in small
/// gray text.
const NEEDS_DAEMON: &str = "Needs the daemon.";

/// How many edits Ctrl+Z can take back.
const UNDO_STEPS: usize = 100;
/// Edits of the same kind this close together (a slider drag, typing) undo as one.
const EDIT_RUN: Duration = Duration::from_secs(1);

/// How long a controller's picture stays on the editor once shown, before another one replaces it.
const PAD_HOLD: Duration = Duration::from_secs(2);

const LABEL_WIDTH: f32 = 170.0;
const SIDEBAR_WIDTH: f32 = 210.0;
const ERROR_COLOR: Color = Color::from_rgb(0.9, 0.3, 0.3);
const MUTED_COLOR: Color = Color::from_rgb(0.55, 0.55, 0.6);
/// Something that saves but likely won't work as meant, such as a menu item nothing can pick.
const WARNING_COLOR: Color = Color::from_rgb(0.85, 0.58, 0.1);

/// A controller as the editor sees it: its model (`None` for a generic pad) and the family whose
/// button glyphs it uses.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PadPick {
    model: Option<PadModel>,
    family: PadFamily,
}

/// The controller on the editor, and when it went up.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PadShown {
    pick: PadPick,
    since: Instant,
}

/// The controller to show: `wanted`, unless the one up now went up less than [`PAD_HOLD`] ago.
fn settle(shown: Option<PadShown>, wanted: PadPick, now: Instant) -> Option<PadShown> {
    let held = shown.is_some_and(|s| now.duration_since(s.since) < PAD_HOLD);
    match shown {
        Some(s) if s.pick == wanted || held => shown,
        _ => Some(PadShown { pick: wanted, since: now }),
    }
}

struct App {
    /// Working copy being edited.
    config: Config,
    /// Plays the previews of menu sounds.
    sounds: crate::sound::Sounds,
    /// Last copy known to be applied; `config != saved` means unsaved edits.
    saved: Config,
    /// `None` while the daemon is unreachable.
    status: Option<Status>,
    /// What the system check found, when the window opened or it was last run.
    system_checks: Vec<system_check::Check>,
    page: Page,
    /// The profile being edited, by index in the current game's profiles.
    editing: usize,
    game_tab: GameTab,
    /// The chapter open on the Manual page, by index in `manual::CHAPTERS`.
    manual_chapter: usize,
    /// What the search boxes on the App settings, Manual and profile editor pages hold.
    settings_query: String,
    manual_query: String,
    button_query: String,
    /// The manual's chapters, parsed for drawing (see `manual.rs`).
    manual: Vec<iced::widget::markdown::Content>,
    /// Light or dark, from the system; the app's theme follows it.
    theme_mode: iced::theme::Mode,
    /// Filters the sidebar's game list.
    game_search: String,
    message: Option<(String, bool)>,
    /// Latest physical input streamed from the daemon.
    live: Option<InputSnapshot>,
    /// The controller picture shown on the editor (see `settle_pad`).
    pad_shown: Option<PadShown>,
    /// The latest presses and what each did, streamed from the daemon.
    feeds: Option<Feeds>,
    picker: Option<KeyPicker>,
    token_picker: Option<TokenPicker>,
    sound_picker: Option<SoundPicker>,
    char_picker: Option<CharPicker>,
    /// The font dropdown that is open.
    font_menu: Option<FontSlot>,
    motion_demo: MotionDemo,
    dialog: Option<Dialog>,
    profile_tab: ProfileTab,
    /// The Guide tab's notes editor, loaded from the profile when the tab is opened.
    guide_notes: iced::widget::text_editor::Content,
    /// The Export dialog's description editor, loaded when the dialog opens.
    export_description: iced::widget::text_editor::Content,
    /// The controller picture is folded away on the Profiles tab.
    picture_hidden: bool,
    /// Rows showing their full editor instead of a one-line summary.
    expanded: HashSet<Target>,
    /// How far the mouse stick's response graphs show the stick pushed (0..1), set by hovering them.
    stick_probe: f32,
    /// Where the gyro's aim has been lately, for the Gyro tab's live picture.
    gyro_trail: gyro_graph::Trail,
    /// The App settings tabs' state.
    shortcuts_ui: shortcuts_tab::Ui,
    /// Waiting for a controller button press to jump to its row.
    finding: bool,
    found: Option<Button>,
    /// Macros whose cards are open, by index in the shown list.
    open_macros: HashSet<usize>,
    /// Menus (by index) whose card is open.
    open_menus: HashSet<usize>,
    /// Appearance editors that are open: a menu's, or the keyboard's (`None`).
    open_appearance: HashSet<Option<usize>>,
    /// The numpad card's Appearance section is open.
    numpad_appearance: bool,
    /// Overlays whose sound settings are open in the Overlay sounds cards.
    open_sounds: HashSet<crate::sound::SoundOverlay>,
    /// Cues whose sliders (volume, pitch, length and effects) are open under them.
    open_cue_edits: HashSet<(crate::sound::SoundOverlay, crate::sound::Feedback)>,
    /// The media controls' and the in-game menu's Appearance sections are open.
    media_appearance: bool,
    menu_appearance: bool,
    /// Info overlays (by index) whose card, or Appearance section, is open.
    open_infos: HashSet<usize>,
    open_info_appearance: HashSet<usize>,
    /// Log overlays (by index) whose card, or Appearance section, is open.
    open_logs: HashSet<usize>,
    open_log_appearance: HashSet<usize>,
    /// Input groups listing their own inputs, by overlay kind, overlay and group.
    open_input_groups: HashSet<(ItemKind, usize, usize)>,
    /// The built-in game library.
    library: Vec<library::Entry>,
    /// Installed and running games, once looked up for the library picker.
    installed: Option<launchers::Installed>,
    /// Games and profiles renamed since the last save. The daemon still reports the active
    /// profile by its old name until then.
    renames: Vec<Rename>,
    /// The shown game's items changed since it was imported (see `pack::edited_items`),
    /// worked out after each edit rather than every frame.
    edited: Vec<String>,
    /// The layer shown on the Layers tab, by index in the game's layers.
    layer: usize,
    /// The mode shown on the Modes tab, by index in the game's modes.
    mode: usize,
    /// The profile a layer or mode is shown over, by index in the game's profiles.
    compare: usize,
    /// What the layer (or mode) editor edits: the compared profile with the layer on top (see
    /// `layers.rs`).
    layer_view: Option<Profile>,
    /// The layer name label's Appearance section is open.
    indicator_appearance: bool,
    /// The assignment the last failed save is about, to jump to.
    problem_at: Option<checks::Place>,
    /// Copies of `config` from before each edit, newest last, for Ctrl+Z.
    undo: Vec<Config>,
    /// The kind of the last edit recorded in `undo`, and when it was made.
    last_edit: Option<(Discriminant<Message>, Instant)>,
    /// Save & quit: close the window once the save goes through.
    quit_after_save: bool,
    /// Why the config on disk couldn't be read. The page then shows the defaults, and saving them
    /// would write over every setup, so saving waits until it reads.
    unreadable: Option<String>,
    /// The guided tour's step on show (see `tour.rs`).
    tour: Option<usize>,
}

/// What the sidebar shows on the right.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Page {
    Overview,
    Settings,
    /// The in-app manual.
    Manual,
    /// A game's page; `None` is General.
    Game(Option<String>),
}

/// Sections of a game's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameTab {
    Profiles,
    Modes,
    Layers,
    Macros,
    Menus,
    Info,
    Logs,
    Details,
}

impl GameTab {
    const ALL: [GameTab; 8] = [
        GameTab::Profiles,
        GameTab::Modes,
        GameTab::Layers,
        GameTab::Macros,
        GameTab::Menus,
        GameTab::Info,
        GameTab::Logs,
        GameTab::Details,
    ];
}

impl fmt::Display for GameTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GameTab::Profiles => "Profiles",
            GameTab::Modes => "Modes",
            GameTab::Layers => "Layers",
            GameTab::Macros => "Macros",
            GameTab::Menus => "Menus",
            GameTab::Info => "Info overlays",
            GameTab::Logs => "Log overlays",
            GameTab::Details => "Details",
        })
    }
}

/// A rename not yet saved, for following the daemon's active profile.
#[derive(Debug, Clone, PartialEq)]
enum Rename {
    Game { old: String, new: String },
    Profile { game: Option<String>, old: String, new: String },
}

/// Where `at` is once these renames apply.
fn follow_renames(renames: &[Rename], mut at: ProfileRef) -> ProfileRef {
    for r in renames {
        match r {
            Rename::Game { old, new } if at.game.as_ref() == Some(old) => at.game = Some(new.clone()),
            Rename::Profile { game, old, new } if &at.game == game && &at.profile == old => at.profile = new.clone(),
            _ => {}
        }
    }
    at
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Button(Button),
    Trigger(Trigger),
    Combo(usize),
    Gesture(Button, GestureKind),
    Zone(Analog, usize),
    /// Step `.1` of macro `.0` (in the shown list), not part of any profile.
    MacroStep(usize, usize),
    /// Item `.1` of menu `.0` (in the shown list).
    MenuItem(usize, usize),
    /// A stick's "Advanced" fold.
    StickAdvanced(Stick),
    /// The gyro's "Advanced" fold.
    GyroAdvanced,
    /// Sector `.1` of a stick's button ring.
    RingSector(Stick, usize),
    /// The touchpad's click or a swipe.
    Touch(TouchSlot),
}

#[derive(Debug, Clone)]
enum Message {
    Poll,
    LiveInput(Option<InputSnapshot>),
    /// Boxed: a feed's log views are far larger than any other message.
    LiveFeed(Option<Box<Feeds>>),
    StatusLoaded(Result<Status, String>),
    ConfigLoaded(Box<Config>, Option<String>),
    SetEnabled(bool),
    SetProfileLock(bool),
    ActivateProfile(ProfileRef),
    Done(Result<(), String>),
    SelectPage(Page),
    SetGameSearch(String),
    SelectGameTab(GameTab),
    SelectChapter(usize),
    /// Open the Manual at a tutorial, from the link on a panel (the tutorial's chapter file).
    OpenTutorial(&'static str),
    /// Shows an item of the current setup on its tab (see `goto.rs`).
    GoToItem(ItemKind, String),
    Tour(TourMove),
    SetSettingsSearch(String),
    SelectSettingsTab(shortcuts_tab::SettingsTab),
    SetShortcut(crate::config::Shortcut, Option<crate::config::ShortcutInput>),
    ResetShortcut(crate::config::Shortcut),
    ResetAllShortcuts,
    SetMouseModeExtras(crate::config::MouseModeExtras),
    ListenShortcut(Option<(crate::config::Shortcut, usize)>),
    SetManualSearch(String),
    SetButtonSearch(String),
    /// A link in the manual; relative links name another chapter's file.
    ManualLink(String),
    SystemMode(iced::theme::Mode),
    RenameGame(String),
    EditProfile(String),
    RenameProfile(String),
    AddProfile(Template),
    DeleteProfile,
    SetAction(Target, ButtonAction),
    SetStick(Stick, StickConfig),
    SetTrigger(Trigger, TriggerAction),
    SetIgnored(String, bool),
    SetAutoSwitch(bool),
    SetDefaultProfile(DefaultChoice),
    AddRule(Option<WindowInfo>),
    RemoveRule(usize),
    SetRuleKind(usize, RuleKind),
    SetRuleValue(usize, String),
    SetRuleProfile(usize, String),
    SetRuleEnabled(usize, bool),
    TestRumble(String),
    TestTurn(i32),
    ToggleOverlay,
    ToggleNumpad,
    ToggleNumpadAppearance,
    ToggleSoundCues(crate::sound::SoundOverlay),
    ToggleSoundEdit(crate::sound::SoundOverlay, crate::sound::Feedback),
    SetNumpadStyle(OverlayStyle),
    ToggleMediaAppearance,
    SetMediaStyle(OverlayStyle),
    ToggleInGameMenuAppearance,
    SetInGameMenuStyle(OverlayStyle),
    CalibrateGyro(String),
    OpenSystemCheck,
    RecheckSystem,
    CopyFix(usize),
    SetGyro(GyroConfig),
    SetTouchpad(TouchpadConfig),
    Guide(guide_tab::GuideMsg),
    SetRequires(Vec<Feature>),
    AddCombo,
    AddMenuCombos(Vec<Combo>),
    RemoveCombo(usize),
    AddComboButton(usize, Button),
    RemoveComboButton(usize, Button),
    SetComboWindow(f32),
    AddGesture(Button, GestureKind),
    RemoveGesture(Button, GestureKind),
    SetTapWindow(f32),
    SetLongPress(f32),
    AddZone(Analog, ZonePreset),
    RemoveZone(Analog, usize),
    SetZoneRange(Analog, usize, f32, f32),
    SelectProfileTab(ProfileTab),
    ToggleExpanded(Target),
    /// The push (0..1) the mouse stick's response graphs show.
    ProbeStick(f32),
    TogglePicture,
    /// Expand (true) or collapse every row in the current profile section.
    ExpandAll(bool),
    StartFind,
    CancelFind,
    ToggleMacro(usize),
    NewInfo,
    ToggleInfo(usize),
    DeleteInfo(usize),
    RenameInfo(usize, String),
    SetInfoAlways(usize, bool),
    /// Seconds info overlay `.0` is shown for when the game starts, or `None` for not then.
    SetInfoOnStart(usize, Option<f32>),
    /// Seconds info overlay `.0` stays after being let go, or `None` to go at once.
    SetInfoLinger(usize, Option<f32>),
    SetInfoTitle(usize, String),
    SetIndicatorTitle(String),
    SetInfoStyle(usize, OverlayStyle),
    ToggleInfoAppearance(usize),
    AddInfoRow(usize),
    /// Row `.1` of info overlay `.0`, moved up if `.2`.
    MoveInfoRow(usize, usize, bool),
    RemoveInfoRow(usize, usize),
    AddInfoCell(usize, usize),
    /// Cell `.2` of row `.1` of info overlay `.0`.
    SetInfoCell(usize, usize, usize, String),
    RemoveInfoCell(usize, usize, usize),
    SetInfoGlyphs(PadFamily),
    SetAppearance(Appearance),
    SetNintendoLayout(bool),
    SetGameNintendoLayout(Option<bool>),
    /// The other controllers a setup supports (its Details tab).
    SetGameControllers(crate::pad_identity::ControllerSupport),
    /// The shown game's own overlay sounds, replacing its set.
    SetGameSounds(SoundSet),
    /// The App settings' sounds, which every setup without its own set uses.
    SetSounds(SoundSet),
    /// The shown game goes back to the default sounds.
    ClearGameSounds,
    PreviewSound(SoundSpec),
    NewLog,
    ToggleLog(usize),
    ToggleLogAppearance(usize),
    DeleteLog(usize),
    RenameLog(usize, String),
    /// Log overlay `.0` replaced by this edited copy (its name is left as it is).
    SetLog(usize, LogOverlay),
    /// What info overlay `.0`'s `{current_input}` cells follow.
    SetInfoInput(usize, crate::config::CurrentInput),
    /// Lists (or hides) the inputs of group `.2` in overlay `.1` of kind `.0`.
    ToggleInputGroup(ItemKind, usize, usize),
    NewMacro,
    DeleteMacro(usize),
    RenameMacro(usize, String),
    AddMacroStep(usize, StepKind),
    /// Step `.1` of macro `.0`.
    SetMacroStep(usize, usize, MacroStep),
    MoveMacroStep(usize, usize, bool),
    RemoveMacroStep(usize, usize),
    InsertMotion(usize, Motion),
    ToggleMenu(usize),
    ToggleAppearance(Option<usize>),
    NewMenu(MenuKindTag),
    DeleteMenu(usize),
    RenameMenu(usize, String),
    SetMenuKind(usize, MenuKind),
    SetMenuStyle(usize, OverlayStyle),
    SetKeyboardStyle(OverlayStyle),
    /// The on-screen keyboard's keys: all of them, or the basic ones.
    SetFullKeyboard(bool),
    /// How strongly the controllers rumble, 0 to 1.
    SetRumbleStrength(f32),
    /// Whether picking an overlay's item gives a faint pulse.
    SetPickRumble(bool),
    SetPeekTrigger(crate::config::PeekTrigger),
    /// The font of all overlays (`None`: the system's), or the shown game's own.
    SetOverlayFont(Option<String>),
    SetColorblindTones(bool),
    /// What every overlay goes without, from the App settings' performance card.
    SetPerformance(crate::config::Performance),
    /// A kind of overlay's motion, in the global set.
    SetMotion(OverlayKind, MotionStyle),
    /// A kind of overlay's motion in the shown game's own set, which starts as the global one.
    SetGameMotion(OverlayKind, MotionStyle),
    /// The shown game gets its own motion, a copy of the global one, or (false) goes back to the
    /// global motion.
    OwnGameMotion(bool),
    SetGameOverlayFont(Option<String>),
    /// A game's own style for the keyboard or numpad, or back to the global one.
    SetGameOverlayStyle(crate::keyboard::Layout, Option<OverlayStyle>),
    /// A game's own look for the media controls or the in-game menu, or back to the global one.
    SetGameMediaStyle(Option<OverlayStyle>),
    SetGameMenuStyle(Option<OverlayStyle>),
    AddMenuItem(usize),
    RemoveMenuItem(usize, usize),
    MoveMenuItem(usize, usize, bool),
    SetMenuItemLabel(usize, usize, String),
    SetMenuItemButton(usize, usize, QuickChoice),
    /// A menu's close button: its kind's own, another, or none.
    SetMenuCancel(usize, items::CloseChoice),
    /// Whether a menu stays up after a pick.
    SetMenuKeepOpen(usize, bool),
    SetMenuItemWeight(usize, usize, f32),
    /// A menu item's icon, by name (`None` for none).
    /// A grid item's merged cells.
    SetMenuItemSpan(usize, usize, crate::config::Span),
    /// The cell a grid's cursor starts on.
    SetMenuStart(usize, usize),
    OpenKeyPicker(KeyField, Vec<String>, bool),
    AddPackFile(pack_files::FileKind),
    /// A file picked for setup `.0` (by name, `None` for General), whichever page is shown when
    /// the file dialog closes.
    PackFilePicked(Option<String>, pack_files::FileKind, Option<Result<(String, Vec<u8>), String>>),
    RemovePackIcon(String),
    RemovePackSound(String),
    RemovePackFont(String),
    PlayPackSound(String),
    OpenTokenPicker(TokenTarget),
    OpenSoundPicker(SoundTarget),
    CloseSoundPicker,
    SoundSearch(String),
    PickSound(sounds::SoundChoice),
    CloseTokenPicker,
    ToggleFontMenu(FontSlot),
    MotionDemoStyle(crate::motion::MotionStyle),
    MotionTick(std::time::Instant),
    OpenCharPicker(TokenTarget),
    CloseCharPicker,
    CharGroup(usize),
    CharSearch(String),
    PickChar(String),
    PickToken(String),
    ToggleTokenFold(&'static str),
    TokenSearch(String),
    PickerKey(&'static str),
    PickerClear,
    PickerClose { apply: bool },
    /// "+ Add game": the library picker.
    OpenAddGame,
    CloseDialog,
    SetLibrarySearch(String),
    /// Library entry to show details of (index in `App::library`).
    SelectLibrary(usize),
    Installed(launchers::Installed),
    AddEmptyGame(Template),
    /// Preview adding the library entry at this index.
    PreviewLibrary(usize),
    /// Preview the library's newer version of this game.
    UpdateFromLibrary(String),
    ImportFile,
    /// A pack file's text, or why it couldn't be read; `None` if the user cancelled.
    PackFileRead(Option<Result<String, String>>),
    SetReplaceGame(bool),
    /// Keep the other game's rule in rule clash `.0`.
    SetKeepMine(usize, bool),
    ConfirmImport,
    OpenExport,
    SetPackField(PackField, String),
    EditPackDescription(iced::widget::text_editor::Action),
    SetLibraryExport(bool),
    SaveExport,
    /// Where the pack was saved, or why not; `None` if the user cancelled.
    Exported(Option<Result<String, String>>),
    AskDeleteGame,
    ConfirmDeleteGame,
    OpenBrowse(ItemKind),
    NewLayer,
    SelectLayer(String),
    RenameLayer(String),
    DeleteLayer,
    /// Which of the game's profiles the layer editor shows the layer over.
    SetCompare(String),
    SetIndicator(IndicatorChoice),
    SetSwallowUnbound(bool),
    SetIndicatorDelay(u64),
    SetIndicatorStyle(OverlayStyle),
    ToggleIndicatorAppearance,
    /// Make the layer set this (a copy of the profile's, to edit).
    OverrideInput(LayerPart),
    /// Stop the layer setting this.
    RevertInput(LayerPart),
    /// Switch a profile combo (by its buttons) off in the layer, or back on.
    ToggleBaseCombo(Vec<Button>),
    BrowseFrom(BrowseSource),
    BrowseOpen(usize),
    CopyItem(usize),
    Save,
    /// The config the daemon (or the file) now has, or why it couldn't take it.
    Saved(Result<Box<Config>, String>),
    Revert,
    Undo,
    /// Jump to the assignment the failed save is about.
    ShowProblem,
    /// The window's close button was pressed.
    CloseRequested,
    /// Save first, then close (from the unsaved-changes prompt).
    SaveAndQuit,
    /// Close without saving (from the unsaved-changes prompt).
    DiscardAndQuit,
    /// Stay open with the unsaved edits (from the unsaved-changes prompt), and let a pending stop go.
    KeepEditing,
}

async fn call(req: Request) -> Result<Response, String> {
    tokio::task::spawn_blocking(move || ipc::request(&req))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

fn call_ok(req: Request) -> Task<Message> {
    Task::perform(call(req), |r| Message::Done(r.map(|_| ())))
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let config = Config::default();
        let app = App {
            saved: config.clone(),
            config,
            sounds: crate::sound::Sounds::start(),
            status: None, system_checks: system_check::run(),
            page: Page::Overview,
            editing: 0,
            game_tab: GameTab::Profiles,
            manual_chapter: 0,
            settings_query: String::new(),
            manual_query: String::new(),
            button_query: String::new(),
            manual: manual::load(), theme_mode: iced::theme::Mode::default(), shortcuts_ui: shortcuts_tab::Ui::default(),
            game_search: String::new(),
            message: None,
            live: None,
            pad_shown: None,
            feeds: None,
            picker: None, token_picker: None, sound_picker: None, char_picker: None, font_menu: None, motion_demo: MotionDemo::new(),
            dialog: None,
            profile_tab: ProfileTab::Buttons,
            guide_notes: iced::widget::text_editor::Content::new(),
            export_description: iced::widget::text_editor::Content::new(),
            picture_hidden: false,
            expanded: HashSet::new(),
            stick_probe: 1.0,
            gyro_trail: gyro_graph::Trail::default(),
            finding: false,
            found: None,
            open_macros: HashSet::new(),
            open_menus: HashSet::new(),
            open_appearance: HashSet::new(),
            numpad_appearance: false,
            open_sounds: HashSet::new(),
            open_cue_edits: HashSet::new(),
            media_appearance: false,
            menu_appearance: false,
            open_infos: HashSet::new(),
            open_info_appearance: HashSet::new(),
            open_logs: HashSet::new(),
            open_log_appearance: HashSet::new(),
            open_input_groups: HashSet::new(),
            library: library::entries().to_vec(),
            installed: None,
            renames: Vec::new(),
            edited: Vec::new(),
            layer: 0,
            mode: 0,
            compare: 0,
            layer_view: None,
            indicator_appearance: false,
            problem_at: None,
            undo: Vec::new(),
            last_edit: None,
            quit_after_save: false, unreadable: None,
            tour: None,
        };
        let tour = if tour::first_run() { Task::done(Message::Tour(TourMove::Start)) } else { Task::none() };
        (app, Task::batch([load_config(), Task::done(Message::Poll), iced::system::theme().map(Message::SystemMode), tour]))
    }

    /// The controller the daemon presents the editor's controller as, under the game on screen's support.
    fn presented_identity(&self) -> PadIdentity {
        PadIdentity::choose(self.shown_pad().model, self.game().controllers.unwrap_or_default())
    }

    /// The button glyphs to draw: the family of the controller the daemon presents as, so they match
    /// what the game will see.
    fn glyph_family(&self) -> PadFamily {
        self.presented_identity().family()
    }

    /// The controller to show: the one used most recently, or the first managed one before any
    /// input has come in.
    fn wanted_pad(&self) -> PadPick {
        let fallback = self.config.info_glyphs;
        match &self.live {
            Some(live) => PadPick { model: live.model, family: live.family.unwrap_or(fallback) },
            None => {
                let managed = self.status.as_ref().and_then(|s| s.devices.iter().find(|d| d.managed));
                PadPick { model: managed.and_then(|d| d.model), family: managed.and_then(|d| d.family).unwrap_or(fallback) }
            }
        }
    }

    /// Switches the editor to the wanted controller, but only once the current one has been up for
    /// [`PAD_HOLD`], so two pads used in turn don't make it flicker.
    fn settle_pad(&mut self, now: Instant) {
        self.pad_shown = settle(self.pad_shown, self.wanted_pad(), now);
    }

    /// The controller on the editor.
    fn shown_pad(&self) -> PadPick {
        self.pad_shown.map_or_else(|| self.wanted_pad(), |shown| shown.pick)
    }

    /// The live input for the controller picture on the editor: only when it comes from that controller.
    fn shown_input(&self) -> Option<&InputSnapshot> {
        self.live.as_ref().filter(|live| live.model == self.shown_pad().model)
    }

    /// Whether button glyphs use the Nintendo layout here: the game on screen's own choice, else
    /// the one set in Settings, unless the controller is presented as a Switch Pro.
    fn nintendo_layout(&self) -> bool {
        let setting = self.game().nintendo_layout.unwrap_or(self.config.nintendo_layout);
        self.presented_identity().applies_nintendo_layout(setting)
    }

    /// How the Nintendo layout shows on the face buttons of profile `p`: the labels, and whether the
    /// buttons send the swapped letters (see [`pieces::Swap`]).
    fn swap_for(&self, p: &Profile) -> pieces::Swap {
        use pieces::Swap;
        let setting = self.game().nintendo_layout.unwrap_or(self.config.nintendo_layout);
        let identity = self.presented_identity();
        if !identity.applies_nintendo_layout(setting) {
            return Swap::Off;
        }
        let pad = self.shown_pad();
        let nintendo_pad = pad.family == PadFamily::Nintendo;
        if identity.swaps_face_output(nintendo_pad, setting, p.faces_default()) {
            Swap::Sends
        } else if nintendo_pad {
            Swap::Quiet
        } else {
            Swap::Labels
        }
    }

    fn theme(&self) -> iced::Theme {
        let mode = match self.config.appearance {
            Appearance::Auto => self.theme_mode,
            Appearance::Light => iced::theme::Mode::Light,
            Appearance::Dark => iced::theme::Mode::Dark,
        };
        <iced::Theme as iced::theme::Base>::default(mode)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Poll),
            Subscription::run(watch_input),
            Subscription::run(watch_feed),
            iced::event::listen_with(undo_shortcut),
            iced::window::close_requests().map(|_| Message::CloseRequested),
            Subscription::run(close_on_signals),
            iced::system::theme_changes().map(Message::SystemMode),
            self.motion_demo_clock(),
        ])
    }

    /// The game whose page is shown (General when it's not a game page).
    fn game_key(&self) -> Option<&str> {
        match &self.page {
            Page::Game(key) => key.as_deref(),
            _ => None,
        }
    }

    fn game(&self) -> &Game {
        self.config.game(self.game_key()).unwrap_or(&self.config.general)
    }

    fn game_mut(&mut self) -> &mut Game {
        let index = self.game_key().and_then(|name| self.config.games.iter().position(|g| g.name == name));
        match index {
            Some(i) => &mut self.config.games[i],
            None => &mut self.config.general,
        }
    }

    fn macros(&self) -> &Vec<Macro> {
        &self.game().macros
    }

    fn macros_mut(&mut self) -> &mut Vec<Macro> {
        &mut self.game_mut().macros
    }

    fn menus(&self) -> &Vec<Menu> {
        &self.game().menus
    }

    fn menus_mut(&mut self) -> &mut Vec<Menu> {
        &mut self.game_mut().menus
    }

    fn infos(&self) -> &Vec<InfoOverlay> {
        &self.game().info
    }

    fn infos_mut(&mut self) -> &mut Vec<InfoOverlay> {
        &mut self.game_mut().info
    }

    /// What the shown profile and items can refer to.
    fn names(&self) -> Names {
        Names::for_game(&self.config, self.game())
    }

    /// The profile the editor shows: the edited profile, or on the Layers tab, the layer over
    /// the profile it's compared with.
    fn profile(&self) -> Option<&Profile> {
        if self.editing_layer() {
            return self.layer_view.as_ref();
        }
        self.game().profiles.get(self.editing)
    }

    fn profile_mut(&mut self) -> Option<&mut Profile> {
        if self.editing_layer() {
            return self.layer_view.as_mut();
        }
        let editing = self.editing;
        self.game_mut().profiles.get_mut(editing)
    }

    /// The edited profile, as the daemon names it.
    fn profile_ref(&self) -> Option<ProfileRef> {
        self.profile().map(|p| ProfileRef::new(self.game_key(), &p.name))
    }

    /// Forgets per-page state when another page or list is shown.
    fn reset_page_state(&mut self) {
        self.open_macros.clear();
        self.open_menus.clear();
        self.open_infos.clear();
        self.open_info_appearance.clear();
        self.open_logs.clear();
        self.open_log_appearance.clear();
        self.open_input_groups.clear();
        self.open_appearance.retain(Option::is_none);
        self.expanded.clear();
        self.found = None;
        self.finding = false;
        self.shortcuts_ui.listening = None;
    }

    /// Shows a game's page, editing its active profile if it has it.
    fn show_game(&mut self, key: Option<String>) {
        let has_active = self.config.active_ref().game == key;
        self.page = Page::Game(key);
        self.editing = if has_active {
            let active = self.config.active_ref();
            self.game().profiles.iter().position(|p| p.name == active.profile).unwrap_or(0)
        } else {
            0
        };
        self.layer = 0;
        self.mode = 0;
        self.compare = self.editing;
        self.reset_page_state();
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        // Polling and live input don't change the config, and arrive many times a second. Live
        // input does while a shortcut listens for its button.
        let listening = matches!(message, Message::LiveInput(_)) && self.shortcuts_ui.listening.is_some();
        let edits = listening || !matches!(message, Message::Poll | Message::MotionTick(_) | Message::LiveInput(_) | Message::LiveFeed(_) | Message::ProbeStick(_) | Message::StatusLoaded(_));
        let message = self.new_layer_for(message);
        // Loading the config and undoing aren't edits to take back.
        let undoable = edits && !matches!(message, Message::ConfigLoaded(..) | Message::Undo);
        let kind = std::mem::discriminant(&message);
        let snapshot = undoable.then(|| self.config.clone());
        // Edits in the layer editor change a working copy; they're kept as the layer's
        // overrides.
        let before = self.layer_view.clone().filter(|_| self.editing_layer());
        let task = self.handle(message);
        if let Some(before) = before
            && self.editing_layer()
        {
            self.write_back(&before);
        }
        if let Some(snapshot) = snapshot {
            self.record_edit(kind, snapshot);
        }
        // Previews draw with the icons and sounds of the game being looked at.
        self.game().assets.activate();
        if edits {
            crate::info::set_opener(crate::info::OpenerLabel::of(&self.config.shortcuts));
            crate::info::set_menu_keys(self.config.shortcuts.menu_keys());
            crate::info::set_panel_keys(&self.config.shortcuts);
            self.refresh_layer_view();
            self.sync_guide_notes();
            self.edited = if self.page == Page::Game(None) { Vec::new() } else { pack::edited_items(self.game()) };
        }
        task
    }

    /// Keeps `before` for Ctrl+Z if the config changed. A run of edits of one kind close
    /// together is a single step.
    fn record_edit(&mut self, kind: Discriminant<Message>, before: Config) {
        if self.config == before {
            return;
        }
        let now = Instant::now();
        let continues = self.last_edit.is_some_and(|(k, at)| k == kind && now.duration_since(at) < EDIT_RUN);
        if !continues {
            self.undo.push(before);
            if self.undo.len() > UNDO_STEPS {
                self.undo.remove(0);
            }
        }
        self.last_edit = Some((kind, now));
    }

    /// After the config was replaced wholesale: leaves a game that's gone, keeps the profile
    /// being edited in range, and the cards open on the items they were open on (`before`).
    fn settle_config(&mut self, before: &open_cards::Shown) {
        let exists = self.game_key().is_none_or(|k| self.config.games.iter().any(|g| g.name == k));
        if !exists {
            self.show_game(None);
        }
        self.editing = self.editing.min(self.game().profiles.len().saturating_sub(1));
        if exists {
            self.follow_open_cards(before);
        }
    }

    /// The app-wide messages: polling, status, saving. Each module handles its own and passes
    /// the rest on: games → items → profile → actions → layers → packs.
    #[expect(clippy::too_many_lines, reason = "predates the size lints")]
    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ProbeStick(push) => self.stick_probe = push,
            Message::Poll => {
                // Polling is the only tick, so a picture waiting out its hold is put up here.
                self.settle_pad(Instant::now());
                // The daemon holds its stop until this window's unsaved edits are dealt with.
                let unsaved = Task::future(call(Request::Unsaved(self.config != self.saved))).discard();
                let status = Task::perform(call(Request::Status), |r| {
                    Message::StatusLoaded(r.and_then(|r| match r {
                        Response::Status(s) => Ok(s),
                        other => Err(format!("The daemon sent an unexpected reply: {other:?}")),
                    }))
                });
                return Task::batch([unsaved, status]);
            }
            Message::LiveInput(snapshot) => {
                if self.shortcuts_ui.listening.is_some()
                    && let Some(b) = snapshot.as_ref().and_then(|s| newly_pressed(self.live.as_ref(), s))
                    && self.heard_shortcut_button(b)
                {
                    self.live = snapshot;
                    return Task::none();
                }
                if self.finding
                    && let Some(b) = snapshot.as_ref().and_then(|s| newly_pressed(self.live.as_ref(), s))
                {
                    return self.jump_to(b);
                }
                self.gyro_trail.push(snapshot.as_ref().and_then(|s| s.gyro_out), Instant::now());
                self.live = snapshot;
                self.settle_pad(Instant::now());
            }
            Message::LiveFeed(feeds) => self.feeds = feeds.map(|f| *f),
            Message::StatusLoaded(Ok(status)) => {
                // Mirror daemon-owned fields so both copies stay comparable.
                let active = ProfileRef { game: status.active_game.clone(), profile: status.active_profile.clone() };
                self.saved.enabled = status.enabled;
                self.saved.active = active.clone();
                self.config.enabled = status.enabled;
                self.config.active = follow_renames(&self.renames, active);
                let stop_pending = status.stop_pending;
                self.status = Some(status);
                if stop_pending && self.config != self.saved && !matches!(self.dialog, Some(Dialog::ConfirmQuit)) {
                    return self.ask_to_save();
                }
            }
            Message::StatusLoaded(Err(_)) => {
                // The daemon has quit (from its tray, say), so close like the window's close button:
                // unsaved edits get asked about first.
                if self.status.take().is_some() {
                    return Task::done(Message::CloseRequested);
                }
            }
            Message::ConfigLoaded(config, err) => {
                let config = *config;
                self.saved = config.clone();
                self.config = config;
                if let Page::Game(key) = self.page.clone() {
                    let exists = key.as_deref().is_none_or(|k| self.config.games.iter().any(|g| g.name == k));
                    self.show_game(if exists { key } else { None });
                }
                if let Some(e) = &err {
                    self.message = Some((format!("Couldn't load the config file: {e}"), true));
                } else if let Some(first) = self.config.unreadable.first() {
                    let more = match self.config.unreadable.len() - 1 {
                        0 => String::new(),
                        n => format!(" (and {n} more; the daemon's log lists them)"),
                    };
                    self.message = Some((format!("Left out of the config, and kept on disk: {}{more}", first.error), true));
                }
                self.unreadable = err;
            }
            Message::SetEnabled(on) => return call_ok(Request::SetEnabled(on)),
            Message::SetProfileLock(on) => return Task::batch([call_ok(Request::SetProfileLock(on)), Task::done(Message::Poll)]),
            Message::ActivateProfile(at) => {
                if self.saved.profile(&at).is_some() {
                    return Task::batch([call_ok(Request::Activate(at)), Task::done(Message::Poll)]);
                }
                self.message = Some(("Save the new profile, then make it active.".into(), true));
            }
            Message::Done(Ok(())) => return Task::done(Message::Poll),
            Message::Done(Err(e)) => self.message = Some((e, true)),
            Message::Save => {
                if self.refuse_unreadable() {
                    self.quit_after_save = false;
                    return Task::none();
                }
                if let Some(err) = self.validate() {
                    self.message = Some((err, true));
                    self.problem_at = self.problem_place();
                    self.quit_after_save = false;
                    return Task::none();
                }
                return self.push(self.config.clone());
            }
            Message::Saved(Ok(config)) => {
                // What was sent, not what's on screen now: edits made since, or kept back (see
                // `SetIgnored`), are still unsaved.
                self.saved = *config;
                self.problem_at = None;
                if self.config == self.saved {
                    self.renames.clear();
                }
                self.message = Some(("Saved and applied.".into(), false));
                if std::mem::take(&mut self.quit_after_save) {
                    return iced::exit();
                }
                return Task::done(Message::Poll);
            }
            Message::Saved(Err(e)) => {
                self.quit_after_save = false;
                self.message = Some((e, true));
            }
            Message::Revert => {
                self.problem_at = None;
                let before = self.shown();
                self.config = self.saved.clone();
                self.renames.clear();
                self.settle_config(&before);
                self.message = None;
            }
            Message::Tour(step) => return self.move_tour(step),
            Message::Undo
            | Message::ShowProblem
            | Message::CloseRequested
            | Message::SaveAndQuit
            | Message::DiscardAndQuit
            | Message::KeepEditing => {
                return self.update_session(&message);
            }
            other => return self.update_games(other),
        }
        Task::none()
    }

    /// Shows the profile and assignment in `place`, open for editing.
    fn go_to(&mut self, place: checks::Place) -> Task<Message> {
        self.dialog = None;
        self.picker = None;
        self.show_game(place.game);
        self.editing = place.profile;
        self.game_tab = GameTab::Profiles;
        self.expanded.insert(place.target);
        match place.target {
            Target::Button(b) | Target::Gesture(b, _) => self.jump_to(b),
            Target::Combo(_) => {
                self.profile_tab = ProfileTab::Combos;
                Task::none()
            }
            Target::Trigger(_) | Target::Zone(..) | Target::RingSector(..) => {
                self.profile_tab = ProfileTab::Sticks;
                Task::none()
            }
            Target::Touch(_) => {
                self.profile_tab = ProfileTab::Touchpad;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// Undo and closing the window.
    fn update_session(&mut self, message: &Message) -> Task<Message> {
        match message {
            Message::ShowProblem => {
                if let Some(place) = self.problem_at.clone() {
                    return self.go_to(place);
                }
            }
            Message::Undo => {
                // A popup edits what it was opened from, which undoing could move or remove under it.
                let popup = self.picker.is_some()
                    || self.token_picker.is_some()
                    || self.sound_picker.is_some()
                    || self.char_picker.is_some()
                    || self.dialog.is_some();
                if !popup
                    && let Some(previous) = self.undo.pop()
                {
                    let before = self.shown();
                    self.config = previous;
                    self.last_edit = None;
                    self.settle_config(&before);
                }
            }
            Message::CloseRequested => {
                if self.config == self.saved {
                    return iced::exit();
                }
                if !matches!(self.dialog, Some(Dialog::ConfirmQuit)) {
                    return self.ask_to_save();
                }
            }
            Message::SaveAndQuit => {
                self.quit_after_save = true;
                return Task::done(Message::Save);
            }
            Message::DiscardAndQuit => return iced::exit(),
            Message::KeepEditing => {
                self.dialog = None;
                return Task::future(call(Request::CancelStop)).discard();
            }
            _ => {}
        }
        Task::none()
    }

    /// Asks about this window's unsaved edits before it can close, or the daemon can end: the question
    /// comes up over whatever is open, and the window is brought to the front.
    fn ask_to_save(&mut self) -> Task<Message> {
        self.picker = None;
        self.token_picker = None;
        self.sound_picker = None;
        self.char_picker = None;
        self.dialog = Some(Dialog::ConfirmQuit);
        iced::window::latest().then(|id| id.map_or(Task::none(), iced::window::gain_focus))
    }

    /// Whether saving must wait because the config file couldn't be read (see `unreadable`); says
    /// so if it must.
    fn refuse_unreadable(&mut self) -> bool {
        let Some(e) = &self.unreadable else { return false };
        self.message = Some((format!("Not saved: the config file couldn't be read ({e}). Fix or move it, then open Padwight again."), true));
        true
    }

    /// Sends a config to the daemon, or writes the file if the daemon is not running.
    fn push(&self, config: Config) -> Task<Message> {
        let daemon_up = self.status.is_some();
        let sent = Box::new(config.clone());
        Task::perform(
            async move {
                if daemon_up {
                    call(Request::SetConfig(Box::new(config))).await.map(|_| ())
                } else {
                    tokio::task::spawn_blocking(move || config.save())
                        .await
                        .map_err(|e| e.to_string())?
                        .map_err(|e| format!("{e:#}"))
                }
            },
            move |result| Message::Saved(result.map(|()| sent)),
        )
    }

    fn view(&self) -> Element<'_, Message> {
        let page: Element<'_, Message> = match &self.page {
            Page::Overview => column![
                self.view_live(self.config.active().map(|p| self.with_active_layers(p)).as_ref(), false),
                rule::horizontal(1),
                self.view_activity(),
                rule::horizontal(1),
                self.view_devices()
            ]
                .spacing(16)
                .into(),
            Page::Settings => self.view_settings(),
            Page::Manual => self.view_manual(),
            Page::Game(_) => self.view_game(),
        };
        // The Manual is for reading: the status header would only push it down.
        let content = if self.page == Page::Manual {
            column![page]
        } else {
            column![self.view_header(), rule::horizontal(1), page]
        }
        .spacing(16)
        .padding(20);
        // Nothing is edited on the Manual, so its footer shows only while there's something to save
        // from another page, or a message.
        let footer = self.page != Page::Manual || self.config != self.saved || self.message.is_some();
        let main = column![scrollable(content).id("main").height(Length::Fill)].push(footer.then(|| self.view_footer())).width(Length::Fill);
        // Always a stack with the page first, popups or not: iced keeps widget state (such as
        // the page's scroll position) by place in the tree, so the shape must not change when
        // a popup opens or closes.
        let base = row![self.view_sidebar(), rule::vertical(1), main];
        let popup = if let Some(picker) = &self.picker {
            Some(view_picker(picker))
        } else if let Some(picker) = &self.token_picker {
            Some(self.view_token_picker(picker))
        } else if let Some(picker) = &self.sound_picker {
            Some(self.view_sound_picker(picker))
        } else if let Some(picker) = &self.char_picker {
            Some(self.view_char_picker(picker))
        } else {
            self.dialog.as_ref().map(|dialog| self.view_dialog(dialog))
        };
        let mut layers = stack![base];
        if let Some(popup) = popup {
            layers = layers.push(popup);
        }
        if let Some(step) = self.tour {
            layers = layers.push(self.view_tour(step));
        }
        layers.into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let profiles: Vec<ProfileRef> = self
            .saved
            .all_games()
            .flat_map(|(key, g)| g.profiles.iter().map(move |p| ProfileRef::new(key, &p.name)))
            .collect();
        let running = self.status.is_some();
        let remapping = running && self.config.enabled;

        // One row on every page: the window title already names the app. Daemon status sits
        // beside the switch it explains, and the longer note only shows on the Overview.
        let mut controls = row![
            // Without the daemon nothing is remapped, so the switch reads off, whatever the
            // config says, and can't be turned on.
            toggler(remapping)
                .label(if remapping { "Remapping on" } else { "Remapping off" })
                .on_toggle_maybe(running.then_some(Message::SetEnabled)),
        ]
        .spacing(16)
        .align_y(Alignment::Center);
        if !running {
            controls = controls.push(text("○ Daemon not running").color(ERROR_COLOR));
        }
        // Locked, the active profile stays put while rules and the default profile would switch it.
        let locked = self.status.as_ref().is_some_and(|s| s.profile_locked);
        let lock = button(tab_icons::lock(locked))
            .style(style::secondary)
            .on_press_maybe(running.then_some(Message::SetProfileLock(!locked)));
        let lock = tooltip(
            lock,
            container(text(if locked { "Locked: rules won't switch this profile. Click to unlock." } else { "Lock this profile against automatic switching." }).size(13))
                .padding(8)
                .style(style::tooltip),
            tooltip::Position::Bottom,
        );
        controls = controls
            .push(space::horizontal())
            .push(text("Active profile"))
            .push(dropdown(profiles, Some(self.saved.active.clone()), Message::ActivateProfile).width(SETTING_WIDTH))
            .push(lock);
        let mut header = column![controls].spacing(8);

        if !running && self.page == Page::Overview {
            let code = |s: &'static str| -> text::Span<'static> { span(s).font(iced::Font::MONOSPACE) };
            #[cfg(target_os = "linux")]
            let how = rich_text![
                "The background service isn't running. Start it with ",
                code("systemctl --user start padwight"),
                " or ",
                code("padwight daemon"),
                ". You can still edit: your changes are saved and take effect once it starts.",
            ];
            #[cfg(windows)]
            let how = rich_text![
                "The background service isn't running. Start it with ",
                code("padwight daemon"),
                ". You can still edit: your changes are saved and take effect once it starts.",
            ];
            header = header.push(how.size(13).color(MUTED_COLOR));
        }
        if self.page == Page::Overview
            && let Some(banner) = self.view_system_check_banner()
        {
            header = header.push(banner);
        }
        header.into()
    }

    fn view_footer(&self) -> Element<'_, Message> {
        let dirty = self.config != self.saved;
        let msg: Element<'_, Message> = match &self.message {
            Some((m, is_err)) => text(m).color(if *is_err { ERROR_COLOR } else { MUTED_COLOR }).into(),
            None if dirty => text("Unsaved changes").color(MUTED_COLOR).into(),
            None => space().into(),
        };
        let mut actions = row![msg, space::horizontal()].spacing(8).align_y(Alignment::Center);
        if self.problem_at.is_some() {
            actions = actions.push(button(text("Go to problem")).style(style::secondary).on_press(Message::ShowProblem));
        }
        actions = actions
            .push(button(text("Revert")).style(style::secondary).on_press_maybe(dirty.then_some(Message::Revert)))
            .push(button(text("Save & apply")).style(style::primary).on_press_maybe(dirty.then_some(Message::Save)));
        // Only a rule on top: a box would add a second line beside the sidebar's divider.
        column![
            rule::horizontal(1),
            container(actions).padding(12),
        ]
        .into()
    }
}

/// Ctrl+Z, without Shift, undoes the last edit.
fn undo_shortcut(event: iced::Event, _: iced::event::Status, _: iced::window::Id) -> Option<Message> {
    match event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Character(c),
            modifiers,
            ..
        }) if modifiers.command() && !modifiers.shift() && c.eq_ignore_ascii_case("z") => Some(Message::Undo),
        _ => None,
    }
}

/// The config as the daemon has it, or read from disk when the daemon isn't running.
fn load_config() -> Task<Message> {
    Task::perform(
        async {
            match call(Request::GetConfig).await {
                Ok(Response::Config(c)) => (c, None),
                _ => match tokio::task::spawn_blocking(Config::load).await {
                    Ok(Ok(c)) => (Box::new(c), None),
                    Ok(Err(e)) => (Box::default(), Some(format!("{e:#}"))),
                    Err(e) => (Box::default(), Some(e.to_string())),
                },
            }
        },
        |(c, err)| Message::ConfigLoaded(c, err),
    )
}

/// Streams live input from the daemon, reconnecting every second while it is unavailable.
fn watch_input() -> impl iced::futures::Stream<Item = Message> {
    watch(Request::WatchInput, Message::LiveInput)
}

/// Asks to close when the process is sent SIGTERM or SIGINT (a terminal's Ctrl+C, `kill`), as the
/// close button would, so unsaved edits are asked about rather than lost.
fn close_on_signals() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::{SinkExt, StreamExt, channel::mpsc};

    iced::stream::channel(1, async move |mut output: mpsc::Sender<Message>| {
        let (signals, mut received) = mpsc::unbounded::<()>();
        if let Err(e) = crate::daemon::stop_on_signals(move || signals.unbounded_send(()).is_ok()) {
            crate::monitor::log!("cannot handle SIGTERM and SIGINT: {e:#}");
            return;
        }
        while received.next().await.is_some() {
            let _ = output.send(Message::CloseRequested).await;
        }
    })
}

/// Streams the latest presses and what they did, like [`watch_input`].
fn watch_feed() -> impl iced::futures::Stream<Item = Message> {
    watch(Request::WatchFeed, |feeds: Option<Feeds>| Message::LiveFeed(feeds.map(Box::new)))
}

/// Streams the values a watch request sends, each wrapped by `wrap`; `None` when the daemon
/// isn't available.
fn watch<T: serde::de::DeserializeOwned + Send + 'static>(
    request: Request,
    wrap: fn(Option<T>) -> Message,
) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::{SinkExt, StreamExt, channel::mpsc};

    iced::stream::channel(16, async move |mut output: mpsc::Sender<Message>| {
        loop {
            if let Ok(mut lines) = ipc::watch(&request) {
                while let Some(line) = lines.next().await {
                    if let Ok(value) = serde_json::from_str::<Option<T>>(&line) {
                        let _ = output.send(wrap(value)).await;
                    }
                }
            }
            let _ = output.send(wrap(None)).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

#[cfg(test)]
mod tests;
