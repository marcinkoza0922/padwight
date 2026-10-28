//! Per-device mapping state: turns normalized input into output events for the active profile.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::keycode::KeyCode;
use crate::config::{Deadzone, DeadzoneShape};

use crate::{
    config::{
        Analog, Button, ButtonAction, Cycle, GUIDE_MODE, GestureKind, MAGNIFIER_MODE, MOUSE_MODE, GuideOpener, OPEN_GUIDE, Macro, MacroStep, Menu, Profile, Stick, StickAction, SwipeDirection, Toggled, TouchSide,
        Trigger, TriggerAction,
    },
    input::{Axis, InputEvent},
    output::OutEvent,
};

mod cycles;
mod flick;
mod guide;
mod macros;
use macros::{MacroOp, MacroRun, TurboState};
mod gyro;
pub(crate) mod stick;
mod touchpad;
#[cfg(test)]
pub(crate) mod release_tests;
#[cfg(test)]
mod bench;

/// A stick-direction button releases this far below its press threshold.
const STICK_DIRECTION_HYSTERESIS: f32 = 0.05;
/// A held zone stays active this far past its edges, so it doesn't flicker on a boundary.
const ZONE_HYSTERESIS: f32 = 0.02;
/// A trigger mapped to a button releases this far below its press threshold.
const TRIGGER_HYSTERESIS: f32 = 0.05;
const WHEEL_UNITS_PER_NOTCH: f32 = 120.0;
/// A held wheel action scrolls one notch at once, then continuously after this delay...
const WHEEL_REPEAT_DELAY: f32 = 0.35;
/// ...at this many notches per second.
const WHEEL_REPEAT_RATE: f32 = 10.0;
/// Holding a Guide menu direction repeats it: after this delay, then at this interval.
const GUIDE_REPEAT_DELAY: Duration = crate::nav::REPEAT_DELAY;
const GUIDE_REPEAT_RATE: Duration = crate::nav::REPEAT_EVERY;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Source {
    Button(Button),
    Trigger(Trigger),
    /// The touchpad's click, with the first finger down on this side of the pad.
    TouchClick(TouchSide),
    /// A swipe across this side of the pad.
    TouchSwipe(TouchSide, SwipeDirection),
    /// An active combo, identified by its sorted member buttons.
    Combo(Vec<Button>),
    /// A held double/triple-tap or long-press action.
    Gesture(Button),
    /// An input's On press or On release (the input is a `Button` or `Trigger` source).
    Edge(Box<Source>, GestureKind),
    /// A ring sector of a stick that is active.
    RingSector(Stick, usize),
    /// An analog zone (index into the stick's or trigger's zone list) that is active.
    Zone(Analog, usize),
    /// A menu item chosen on screen (menu name, item index).
    MenuItem(String, usize),
}

/// The physical input that opened a menu, which holds the menu up until it is let go.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Opener {
    /// Released when any of these is (stick directions included).
    pub buttons: Vec<Button>,
    /// Released when this trigger drops below the menu's release point.
    pub trigger: Option<Trigger>,
}

impl Source {
    /// The input this came from, as the input log knows it; `None` for a menu item.
    fn fired_from(&self) -> Option<crate::inputlog::FiredFrom> {
        use crate::inputlog::FiredFrom;
        Some(match self {
            Source::Button(b) | Source::Gesture(b) => FiredFrom::Button(*b),
            Source::TouchClick(_) | Source::TouchSwipe(..) => FiredFrom::Touchpad,
            Source::Combo(members) => FiredFrom::Combo(members.clone()),
            Source::Trigger(t) | Source::Zone(Analog::Trigger(t), _) => FiredFrom::Trigger(*t),
            Source::Zone(Analog::Stick(s), _) | Source::RingSector(s, _) => FiredFrom::Stick(*s),
            Source::Edge(input, _) => return input.fired_from(),
            Source::MenuItem(..) => return None,
        })
    }

    fn opener(&self) -> Opener {
        match self {
            Source::Button(b) | Source::Gesture(b) => Opener { buttons: vec![*b], ..Opener::default() },
            Source::Combo(members) => Opener { buttons: members.clone(), ..Opener::default() },
            Source::Trigger(t) => Opener { trigger: Some(*t), ..Opener::default() },
            Source::Edge(input, _) => input.opener(),
            _ => Opener::default(),
        }
    }
}

/// Gesture detection for a button acting alone that has gestures configured.
#[derive(Debug, Clone, Copy, PartialEq)]
enum GestureState {
    /// Held, undecided. `taps` counts presses in this sequence including this one.
    /// `hold_deadline`: held this long, a first press is a plain hold, not a tap.
    Down { taps: u8, long_deadline: Option<Instant>, hold_deadline: Option<Instant> },
    /// Released, waiting to see whether another tap follows.
    Up { taps: u8, deadline: Instant },
    /// A gesture action fired and is held until release.
    Holding,
    /// Held past the tap window: the button's own action is pressed until release.
    Pressed,
}

/// State of a held button that belongs to at least one combo.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ComboMember {
    /// Waiting to see if the rest of a combo follows. `None` never times out: a member
    /// whose own action is Disabled works as a modifier.
    Pending { deadline: Option<Instant> },
    /// The window ran out; the button's own action is pressed.
    Fired,
    /// Used up by a combo; releasing it releases the combo, nothing else.
    Consumed,
}

#[derive(Default)]
pub struct Engine {
    axes: HashMap<Axis, f32>,
    /// Action each held input pressed, so release always matches press even across profile edits.
    held: HashMap<Source, ButtonAction>,
    members: HashMap<Button, ComboMember>,
    gestures: HashMap<Button, GestureState>,
    /// The On release of each input held down, as it was when pressed, tapped when let go.
    on_release: HashMap<Source, ButtonAction>,
    /// Virtual-pad stick each physical stick feeds in gamepad mode, so it can be recentered
    /// when a layer gives the stick another mode.
    pad_feeds: HashMap<Stick, Stick>,
    /// Virtual-pad trigger each physical trigger feeds, likewise.
    trigger_feeds: HashMap<Trigger, Trigger>,
    /// Press threshold of each trigger whose button action is held: it releases below that,
    /// whatever the trigger is set to by then.
    trigger_release: HashMap<Trigger, f32>,
    /// Bounds of each active zone, which it releases by (the zone list may have changed).
    zone_bounds: HashMap<(Analog, usize), (f32, f32)>,
    /// Layers held, oldest first, with how many inputs hold each.
    layers: Vec<(String, u32)>,
    /// The mode switched to, under the layers; `None` is the default mode (the profile itself).
    mode: Option<String>,
    /// Set when `layers` or `mode` changes; the daemon takes it.
    layers_changed: bool,
    /// Set while the Guide button is held: whether another button went down during the hold,
    /// which makes it a chord rather than a tap that toggles the Guide overlay.
    guide_held: Option<bool>,
    /// Whether the Guide overlay is up. Its layer stays on until the overlay closes.
    guide_open: bool,
    /// Whether mouse mode is on. It stays on until the opener is used again.
    mouse_mode: bool,
    /// What opens the Guide overlay (see [`GuideOpener`]).
    opener: GuideOpener,
    /// Which buttons confirm and go back in the Guide overlay's menu.
    menu_keys: crate::config::MenuKeys,
    /// Buttons pressed while the Guide overlay is up that drive its menu (see [`guide_nav`]),
    /// in order; the daemon takes them.
    guide_nav: Vec<Button>,
    /// The menu direction held down, and when it next repeats.
    guide_repeat: Option<(Button, Instant)>,
    mouse_acc: (f32, f32),
    scroll_acc: (f32, f32),
    /// Fingers on the touchpad and what they are doing.
    touch: touchpad::TouchState,
    /// The menus the touchpad can lay out (see [`Engine::set_menus`]).
    menus: Arc<Vec<Menu>>,
    /// Smoothed position of each mouse stick whose response smooths it, until it settles at rest.
    stick_smooth: HashMap<Stick, (f32, f32)>,
    /// Seconds each mouse stick has been held at full deflection, for acceleration.
    stick_ramp: HashMap<Stick, f32>,
    /// Where each stick in mouse-region mode last put the pointer, while it is pushed.
    stick_region: HashMap<Stick, (f32, f32)>,
    /// The ring sector each stick in ring mode is pointing into.
    ring_sector: HashMap<Stick, usize>,
    /// Flick progress of each stick in flick mode.
    flick: HashMap<Stick, flick::FlickState>,
    /// Seconds each held action containing a wheel direction has been held.
    wheel_held: HashMap<Source, f32>,
    wheel_acc: (f32, f32),
    /// Toggles that are on, keyed by input and position in the action tree.
    toggled: HashMap<StateId, ButtonAction>,
    turbo: HashMap<StateId, TurboState>,
    /// Whether the face buttons send the letters on their Nintendo labels (see
    /// [`set_face_swap`](Self::set_face_swap)).
    face_swap: bool,
    gyro: gyro::GyroState,
    /// What physical sticks currently send to each virtual-pad stick, so gyro can add to it.
    pad_sticks: HashMap<Stick, (f32, f32)>,
    /// Macro definitions by name, compiled to operations (see [`Engine::set_macros`]).
    macro_defs: HashMap<String, Arc<Vec<MacroOp>>>,
    /// How long each macro takes to play, in milliseconds, by name.
    macro_ms: HashMap<String, u64>,
    macros_running: HashMap<StateId, MacroRun>,
    /// Each cycle's steps, by name (see [`Engine::set_cycles`]).
    cycle_defs: HashMap<String, Vec<ButtonAction>>,
    /// The step of each cycle pressed last, by name; none until its first press.
    cycle_at: HashMap<String, usize>,
    /// The step each held Cycle action pressed, released with it.
    cycle_held: HashMap<StateId, ButtonAction>,
    /// Virtual-pad stick positions set by macro steps, added to physical and gyro input.
    macro_sticks: HashMap<Stick, (f32, f32)>,
    /// Stick-direction virtual buttons currently pressed by stick movement.
    stick_buttons: HashSet<Button>,
    /// Stick pushes held by actions (one entry per hold), each moving the virtual stick.
    pushed_sticks: Vec<(Stick, (f32, f32))>,
    /// Trigger pulls held by actions (one entry per hold), each pulling the virtual trigger.
    pushed_triggers: Vec<(Trigger, f32)>,
    /// What each physical trigger last fed its virtual trigger.
    trigger_fed: HashMap<Trigger, f32>,
    /// Set when a ToggleOverlay/ToggleNumpad action fires; the daemon takes it.
    overlay_toggled: Option<crate::keyboard::Layout>,
    /// Set when a Screenshot action fires; the daemon takes it.
    screenshot_requested: bool,
    /// Set when a QuickSettings action fires; the daemon takes it.
    settings_requested: bool,
    /// Set when a ToggleRecording action fires; the daemon takes it.
    recording_toggled: bool,
    /// Set when a ToggleMedia action fires; the daemon takes it.
    media_toggled: bool,
    /// Set when a Tools action fires; the daemon takes it.
    tools_requested: bool,
    /// How many inputs hold a ForceQuit action down.
    force_quit_holds: u32,
    /// Whether the magnifying glass mode is on.
    magnifier_mode: bool,
    /// How far the lens has been zoomed and resized from where it starts, in steps.
    lens_zoom: i32,
    lens_size: i32,
    /// Where the lens is, across and down the screen (0..1): the right stick moves it like a
    /// cursor while it isn't pinned.
    lens_at: (f32, f32),
    /// Where the lens is pinned, if it is.
    lens_pinned: Option<(f32, f32)>,
    /// Set when an OpenMenu action fires; the daemon takes it.
    menu_request: Option<(String, Opener)>,
    /// Info overlays shown by held (or toggled) ShowInfo actions, with how many hold each.
    info_holds: HashMap<String, u32>,
    /// Log overlays shown by held (or toggled) ShowLog actions, likewise.
    log_holds: HashMap<String, u32>,
    /// Set when `info_holds` or `log_holds` changes; the daemon takes it.
    info_changed: bool,
    /// Actions pressed since the daemon last took them, with the input that pressed each,
    /// for the input log.
    fired: Vec<(crate::inputlog::FiredFrom, ButtonAction)>,
}

/// Identifies a Toggle/Turbo node: the input it belongs to and its pre-order position among
/// the stateful nodes of that input's action, so press and release find the same state.
type StateId = (Source, usize);

/// How fast the right stick moves the magnifying glass at full push, in screens per second across
/// and down.
const LENS_SPEED: (f32, f32) = (0.6, 0.9);

impl Engine {
    /// Actions pressed since last taken, and the input behind each.
    pub fn take_fired(&mut self) -> Vec<(crate::inputlog::FiredFrom, ButtonAction)> {
        std::mem::take(&mut self.fired)
    }

    /// A menu an OpenMenu action asked for since the last call, and what opened it.
    pub fn take_menu_request(&mut self) -> Option<(String, Opener)> {
        self.menu_request.take()
    }

    /// Runs a chosen menu item's action as a quick press and release. Toggles and the like
    /// keep their state per item.
    pub fn tap_menu_item(&mut self, menu: &str, item: usize, action: &ButtonAction, out: &mut Vec<OutEvent>) {
        let src = Source::MenuItem(menu.to_string(), item);
        self.digital(&src, action, true, out);
        self.digital(&src, action, false, out);
    }

    /// The layers that apply, oldest first: the held ones, unless a built-in mode is on.
    pub fn layers(&self) -> Vec<String> {
        if self.built_in_mode().is_some() {
            return Vec::new();
        }
        self.layers.iter().map(|(name, _)| name.clone()).collect()
    }

    /// The mode on now (`None` is the default mode): the Guide mode while Guide is held or its
    /// overlay is up, else mouse mode while that's on, else the mode switched to.
    pub fn mode(&self) -> Option<&str> {
        self.built_in_mode().or(self.mode.as_deref())
    }

    /// The mode switched to, which comes back once a built-in mode ends.
    pub fn chosen_mode(&self) -> Option<&str> {
        self.mode.as_deref()
    }

    fn built_in_mode(&self) -> Option<&'static str> {
        if self.guide_held.is_some() || self.guide_open {
            Some(GUIDE_MODE)
        } else if self.mouse_mode {
            Some(MOUSE_MODE)
        } else if self.magnifier_mode {
            Some(MAGNIFIER_MODE)
        } else {
            None
        }
    }

    /// Switches to `name`, or back to the default mode if it's already on. Mouse mode is switched
    /// on and off over the chosen mode; switching to any other mode ends it. The Guide mode
    /// belongs to the Guide button, so switching to it does nothing.
    fn switch_mode(&mut self, name: &str) {
        match name {
            GUIDE_MODE => {}
            // The opener's gesture or combo: it opens and closes the overlay, and ends mouse mode.
            OPEN_GUIDE if self.mouse_mode || self.magnifier_mode => {
                self.set_mouse_mode(false);
                self.set_magnifier_mode(false);
            }
            OPEN_GUIDE => self.set_guide_open(!self.guide_open),
            MOUSE_MODE => {
                if self.guide_open {
                    self.set_guide_open(false);
                }
                self.set_magnifier_mode(false);
                self.set_mouse_mode(!self.mouse_mode);
            }
            // Out in the game, Guide + the trigger takes a pinned lens away; from the Guide menu it
            // unpins the lens and starts it over.
            MAGNIFIER_MODE if !self.guide_open && !self.magnifier_mode && self.lens_pinned.is_some() => {
                self.lens_pinned = None;
                self.layers_changed = true;
            }
            MAGNIFIER_MODE => {
                if self.guide_open {
                    self.set_guide_open(false);
                    self.lens_pinned = None;
                }
                self.set_mouse_mode(false);
                self.set_magnifier_mode(!self.magnifier_mode);
            }
            _ => {
                self.mode = if self.mode.as_deref() == Some(name) && !self.mouse_mode { None } else { Some(name.to_string()) };
                self.set_mouse_mode(false);
                self.set_magnifier_mode(false);
            }
        }
    }

    /// Whether the active layers or mode changed since the last call.
    pub fn take_layers_changed(&mut self) -> bool {
        std::mem::take(&mut self.layers_changed)
    }

    /// Whether the Guide overlay is up (on any controller of this engine).
    pub fn guide_overlay_open(&self) -> bool {
        self.guide_open
    }

    /// Whether the shown info overlays changed since the last call.
    pub fn take_info_changed(&mut self) -> bool {
        std::mem::take(&mut self.info_changed)
    }

    /// Info overlays held up by ShowInfo actions right now.
    pub fn shown_info(&self) -> impl Iterator<Item = &String> {
        self.info_holds.keys()
    }

    /// Log overlays held up by ShowLog actions right now.
    pub fn shown_logs(&self) -> impl Iterator<Item = &String> {
        self.log_holds.keys()
    }

    /// Whether a ToggleRecording action fired since the last call.
    pub fn take_recording_toggle(&mut self) -> bool {
        std::mem::take(&mut self.recording_toggled)
    }

    /// Whether a ToggleMedia action fired since the last call.
    pub fn take_media_toggle(&mut self) -> bool {
        std::mem::take(&mut self.media_toggled)
    }

    /// Whether a Tools action fired since the last call.
    pub fn take_tools_request(&mut self) -> bool {
        std::mem::take(&mut self.tools_requested)
    }

    /// Whether a ForceQuit action is being held down.
    pub fn force_quit_held(&self) -> bool {
        self.force_quit_holds > 0
    }

    /// The magnifying glass while its mode is on or it is pinned: where it is on the screen
    /// (0..1 across and down; the right stick moves it, the middle with the stick at rest), how
    /// much it magnifies and its radius as a share of the screen's shorter side.
    pub fn magnifier(&self) -> Option<(f32, f32, f32, f32)> {
        if !self.magnifier_mode && self.lens_pinned.is_none() {
            return None;
        }
        let (x, y) = self.lens_pinned.unwrap_or(self.lens_at);
        let zoom = (2.5 * 1.25_f32.powi(self.lens_zoom)).clamp(1.5, 10.0);
        let size = (0.17 + 0.03 * self.lens_size as f32).clamp(0.06, 0.4);
        Some((x, y, zoom, size))
    }

    /// Whether the lens is pinned.
    pub fn lens_pinned(&self) -> bool {
        self.lens_pinned.is_some()
    }

    /// Whether the magnifying glass mode is on.
    pub fn magnifier_mode(&self) -> bool {
        self.magnifier_mode
    }

    /// How far the right stick pushes the lens, each way from -1 to 1.
    fn lens_push(&self) -> (f32, f32) {
        let axis = |a: Axis| self.axes.get(&a).copied().unwrap_or(0.0);
        let (x, y) = apply_deadzone(axis(Axis::RightX), axis(Axis::RightY), 0.12_f32);
        (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0))
    }

    /// Whether the right stick is moving the lens.
    fn lens_moving(&self) -> bool {
        self.magnifier_mode && self.lens_pinned.is_none() && self.lens_push() != (0.0, 0.0)
    }

    /// Moves the lens by the stick's push, in whole screens per second at full push.
    fn tick_lens(&mut self, dt: f32) {
        if !self.lens_moving() {
            return;
        }
        let (x, y) = self.lens_push();
        self.lens_at = ((self.lens_at.0 + x * LENS_SPEED.0 * dt).clamp(0.0, 1.0), (self.lens_at.1 + y * LENS_SPEED.1 * dt).clamp(0.0, 1.0));
    }

    fn lens_control(&mut self, control: crate::config::LensControl) {
        use crate::config::LensControl;
        match control {
            LensControl::ZoomIn => self.lens_zoom = (self.lens_zoom + 1).min(7),
            LensControl::ZoomOut => self.lens_zoom = (self.lens_zoom - 1).max(-3),
            LensControl::Larger => self.lens_size = (self.lens_size + 1).min(7),
            LensControl::Smaller => self.lens_size = (self.lens_size - 1).max(-3),
            LensControl::Pin => {
                // Unpinned, the lens carries on from where it was held.
                if let Some(at) = self.lens_pinned.take() {
                    self.lens_at = at;
                } else {
                    self.lens_pinned = Some(self.lens_at);
                }
            }
        }
    }

    /// Whether a QuickSettings action fired since the last call.
    pub fn take_settings_request(&mut self) -> bool {
        std::mem::take(&mut self.settings_requested)
    }

    /// Whether a Screenshot action fired since the last call.
    pub fn take_screenshot(&mut self) -> bool {
        std::mem::take(&mut self.screenshot_requested)
    }

    /// Which on-screen keyboard (or numpad) a toggle action asked for since the last call.
    pub fn take_overlay_toggle(&mut self) -> Option<crate::keyboard::Layout> {
        self.overlay_toggled.take()
    }

    /// Whether a face button's plain press sends the letter on its Nintendo label rather than its
    /// position, for the pad now sending input. Set by the daemon before each batch of events.
    pub fn set_face_swap(&mut self, on: bool) {
        self.face_swap = on;
    }

    /// What a plain press of `b` does: the profile's action, except that with the face swap on,
    /// a face button that sends itself sends its Nintendo counterpart instead.
    fn plain_action(&self, profile: &Profile, b: Button) -> ButtonAction {
        match profile.button(b) {
            ButtonAction::Gamepad(out) if self.face_swap && *out == b => ButtonAction::Gamepad(crate::info::nintendo_face(b)),
            action => action.clone(),
        }
    }

    /// Replaces the menus a touchpad in menu mode can lay out (the active game's, from the config).
    pub fn set_menus(&mut self, menus: &[Menu]) {
        self.menus = Arc::new(menus.to_vec());
    }

    /// Processes one input.
    pub fn handle(&mut self, profile: &Profile, ev: InputEvent, now: Instant, out: &mut Vec<OutEvent>) {
        self.handle_mapped(profile, ev, now, out);
        self.gyro_after_input(profile, out);
        // After the gyro edge checks above: the direction buttons run their own.
        if let InputEvent::Axis(axis, _) = ev
            && let Some(s) = axis_stick(axis)
        {
            self.update_stick_buttons(profile, s, now, out);
        }
    }

    /// Presses or releases a stick's direction buttons (e.g. `Button::LeftStickUp`) as it
    /// moves past its press threshold, feeding them through the normal button path so they
    /// get actions, gestures and combos.
    fn update_stick_buttons(&mut self, profile: &Profile, s: Stick, now: Instant, out: &mut Vec<OutEvent>) {
        let cfg = profile.stick(s);
        let (x, y) = self.stick_pos(s, cfg);
        let t = cfg.key_threshold;
        let [up, down, left, right] = Button::stick_directions(s);
        for (b, v) in [(up, -y), (down, y), (left, -x), (right, x)] {
            let pressed = self.stick_buttons.contains(&b);
            let now_pressed = analog_pressed(v, t, pressed, STICK_DIRECTION_HYSTERESIS);
            if !pressed && now_pressed {
                self.stick_buttons.insert(b);
                self.handle(profile, InputEvent::Button(b, true), now, out);
            } else if pressed && !now_pressed {
                self.stick_buttons.remove(&b);
                self.handle(profile, InputEvent::Button(b, false), now, out);
            }
        }
    }

    fn handle_mapped(&mut self, profile: &Profile, ev: InputEvent, now: Instant, out: &mut Vec<OutEvent>) {
        if let InputEvent::Button(b, pressed) = ev
            && self.guide_routes(b, pressed, now)
        {
            return;
        }
        match ev {
            InputEvent::Button(b, true) if profile.in_combo(b) => {
                let deadline = match profile.button(b) {
                    ButtonAction::Disabled => None,
                    _ => Some(now + Duration::from_millis(profile.combo_window_ms)),
                };
                self.members.insert(b, ComboMember::Pending { deadline });
                self.try_combo(profile, out);
            }
            InputEvent::Button(b, false) if self.members.contains_key(&b) => {
                match self.members.remove(&b) {
                    // Released inside the window: a quick tap of the button on its own.
                    Some(ComboMember::Pending { .. }) => {
                        self.solo(profile, b, true, now, out);
                        self.solo(profile, b, false, now, out);
                    }
                    Some(ComboMember::Fired) => self.solo(profile, b, false, now, out),
                    Some(ComboMember::Consumed) => {
                        let combos: Vec<Source> = self
                            .held
                            .keys()
                            .filter(|s| matches!(s, Source::Combo(m) if m.contains(&b)))
                            .cloned()
                            .collect();
                        for src in combos {
                            self.digital(&src, &ButtonAction::Disabled, false, out);
                        }
                    }
                    None => {}
                }
            }
            InputEvent::Button(b, pressed) => self.solo(profile, b, pressed, now, out),
            InputEvent::Touchpad(ev) => self.touchpad(profile, ev, now, out),
            InputEvent::Axis(axis, value) => {
                self.axes.insert(axis, value);
                match axis {
                    Axis::LeftTrigger => self.trigger(profile, Trigger::Left, value, out),
                    Axis::RightTrigger => self.trigger(profile, Trigger::Right, value, out),
                    Axis::LeftX | Axis::LeftY => self.stick(profile, Stick::Left, out),
                    Axis::RightX | Axis::RightY => self.stick(profile, Stick::Right, out),
                }
            }
        }
    }

    /// Fires the largest combo whose members are all pending.
    fn try_combo(&mut self, profile: &Profile, out: &mut Vec<OutEvent>) {
        let ready = profile
            .combos
            .iter()
            .filter(|c| c.buttons.len() >= 2)
            .filter(|c| {
                c.buttons
                    .iter()
                    .all(|b| matches!(self.members.get(b), Some(ComboMember::Pending { .. })))
            })
            .max_by_key(|c| c.buttons.len());
        let Some(combo) = ready else { return };
        for b in &combo.buttons {
            self.members.insert(*b, ComboMember::Consumed);
        }
        let mut id = combo.buttons.clone();
        id.sort();
        id.dedup();
        self.digital(&Source::Combo(id), &combo.action, true, out);
    }

    /// Fires combo members whose window has run out.
    pub fn timers(&mut self, profile: &Profile, now: Instant, out: &mut Vec<OutEvent>) {
        self.repeat_guide_nav(now);
        let due: Vec<Button> = self
            .members
            .iter()
            .filter(|(_, m)| matches!(m, ComboMember::Pending { deadline: Some(d) } if *d <= now))
            .map(|(b, _)| *b)
            .collect();
        for b in due {
            self.members.insert(b, ComboMember::Fired);
            self.solo(profile, b, true, now, out);
        }

        let due: Vec<(Button, GestureState)> = self
            .gestures
            .iter()
            .filter(|(_, g)| match g {
                GestureState::Down { long_deadline, hold_deadline, .. } => {
                    long_deadline.is_some_and(|d| d <= now) || hold_deadline.is_some_and(|d| d <= now)
                }
                GestureState::Up { deadline, .. } => *deadline <= now,
                _ => false,
            })
            .map(|(b, g)| (*b, *g))
            .collect();
        for (b, state) in due {
            let Some(gestures) = profile.gestures(b) else {
                self.gestures.remove(&b);
                continue;
            };
            match state {
                // Held long enough: the long press fires and stays held until release.
                GestureState::Down { long_deadline: Some(_), .. } => {
                    self.gestures.insert(b, GestureState::Holding);
                    if let Some(action) = &gestures.long_press {
                        self.digital(&Source::Gesture(b), action, true, out);
                    }
                }
                // Held past the tap window with no long press set: a plain press, held (so
                // e.g. a menu on it stays up while held).
                GestureState::Down { .. } => {
                    self.gestures.insert(b, GestureState::Pressed);
                    let action = self.plain_action(profile, b);
                    self.digital(&Source::Button(b), &action, true, out);
                }
                // No further tap came: the sequence so far is final.
                GestureState::Up { taps, .. } => {
                    self.gestures.remove(&b);
                    self.tap_sequence(profile, b, taps, out);
                }
                GestureState::Holding | GestureState::Pressed => {}
            }
        }
    }

    /// Earliest moment `timers` has work to do.
    pub fn next_deadline(&self) -> Option<Instant> {
        let combos = self.members.values().filter_map(|m| match m {
            ComboMember::Pending { deadline } => *deadline,
            _ => None,
        });
        let gestures = self.gestures.values().filter_map(|g| match g {
            GestureState::Down { long_deadline, hold_deadline, .. } => long_deadline.or(*hold_deadline),
            GestureState::Up { deadline, .. } => Some(*deadline),
            GestureState::Holding | GestureState::Pressed => None,
        });
        let guide = self.guide_repeat.map(|(_, due)| due);
        combos.chain(gestures).chain(guide).min()
    }

    /// A button acting on its own (not as part of a combo). Runs gesture detection if the
    /// button has gestures, otherwise presses/releases its action directly.
    #[expect(clippy::too_many_arguments, reason = "predates the size lints")]
    fn solo(&mut self, profile: &Profile, b: Button, pressed: bool, now: Instant, out: &mut Vec<OutEvent>) {
        self.edges(&Source::Button(b), profile.gestures(b), pressed, out);
        let Some(gestures) = profile.gestures(b).filter(|g| g.timed()) else {
            // Gestures may have been removed mid-sequence; a release still has to land.
            if !pressed {
                match self.gestures.remove(&b) {
                    Some(GestureState::Holding) => {
                        self.digital(&Source::Gesture(b), &ButtonAction::Disabled, false, out);
                        return;
                    }
                    Some(GestureState::Pressed) => {
                        self.digital(&Source::Button(b), &ButtonAction::Disabled, false, out);
                        return;
                    }
                    _ => {}
                }
            }
            self.digital(&Source::Button(b), &self.plain_action(profile, b), pressed, out);
            return;
        };
        let max_taps = gestures.max_taps();
        if pressed {
            let taps = match self.gestures.get(&b) {
                Some(GestureState::Up { taps, .. }) => taps + 1,
                _ => 1,
            };
            if taps > 1 && taps == max_taps {
                // Final tap of the longest sequence: fire now and hold until release.
                self.gestures.insert(b, GestureState::Holding);
                let action = gestures.for_taps(taps).cloned().unwrap_or_else(|| self.plain_action(profile, b));
                self.digital(&Source::Gesture(b), &action, true, out);
                return;
            }
            let long_deadline = (taps == 1 && gestures.long_press.is_some())
                .then(|| now + Duration::from_millis(profile.long_press_ms));
            let hold_deadline = (taps == 1 && gestures.long_press.is_none())
                .then(|| now + Duration::from_millis(profile.tap_window_ms));
            self.gestures.insert(b, GestureState::Down { taps, long_deadline, hold_deadline });
        } else {
            match self.gestures.remove(&b) {
                Some(GestureState::Holding) => {
                    self.digital(&Source::Gesture(b), &ButtonAction::Disabled, false, out);
                }
                Some(GestureState::Pressed) => {
                    self.digital(&Source::Button(b), &ButtonAction::Disabled, false, out);
                }
                Some(GestureState::Down { taps, .. }) if taps < max_taps => {
                    let deadline = now + Duration::from_millis(profile.tap_window_ms);
                    self.gestures.insert(b, GestureState::Up { taps, deadline });
                }
                // Nothing more can follow (e.g. only a long press is set): act as a tap now.
                Some(GestureState::Down { taps, .. }) => self.tap_sequence(profile, b, taps, out),
                _ => {}
            }
        }
    }

    /// Taps an input's On press as it goes down, and as it's let go the On release it had then.
    /// Neither waits on the input's other gestures.
    fn edges(&mut self, input: &Source, gestures: Option<&crate::config::Gestures>, pressed: bool, out: &mut Vec<OutEvent>) {
        if !pressed {
            if let Some(action) = self.on_release.remove(input) {
                self.tap(&Source::Edge(Box::new(input.clone()), GestureKind::OnRelease), &action, out);
            }
            return;
        }
        let Some(g) = gestures else { return };
        if let Some(action) = &g.on_release {
            self.on_release.insert(input.clone(), action.clone());
        }
        if let Some(action) = &g.on_press {
            self.tap(&Source::Edge(Box::new(input.clone()), GestureKind::OnPress), action, out);
        }
    }

    /// Emits the result of a finished tap sequence as a quick press and release.
    fn tap_sequence(&mut self, profile: &Profile, b: Button, taps: u8, out: &mut Vec<OutEvent>) {
        let gestures = profile.gestures(b);
        match gestures.and_then(|g| g.for_taps(taps)) {
            Some(action) => self.tap(&Source::Gesture(b), action, out),
            // e.g. a double tap when only a triple tap is set: that many normal taps.
            None => {
                for _ in 0..taps {
                    self.tap(&Source::Button(b), &self.plain_action(profile, b), out);
                }
            }
        }
    }

    fn tap(&mut self, src: &Source, action: &ButtonAction, out: &mut Vec<OutEvent>) {
        self.digital(src, action, true, out);
        self.digital(src, action, false, out);
    }

    fn digital(&mut self, src: &Source, action: &ButtonAction, pressed: bool, out: &mut Vec<OutEvent>) {
        if pressed {
            if self.held.contains_key(src) {
                return;
            }
            self.held.insert(src.clone(), action.clone());
            if let Some(from) = src.fired_from() {
                self.fired.push((from, action.clone()));
            }
            self.emit(src, action, true, 0, out);
        } else if let Some(action) = self.held.remove(src) {
            self.emit(src, &action, false, 0, out);
        }
    }

    /// One more (or one fewer) input holding the layer `name`.
    fn hold_layer(&mut self, name: &str, pressed: bool) {
        let at = self.layers.iter().position(|(n, _)| n == name);
        match (pressed, at) {
            (true, Some(i)) => self.layers[i].1 += 1,
            (true, None) => self.layers.push((name.to_string(), 1)),
            (false, Some(i)) => {
                self.layers[i].1 -= 1;
                if self.layers[i].1 == 0 {
                    self.layers.remove(i);
                }
            }
            (false, None) => {}
        }
        self.layers_changed = true;
    }

    /// Emits press/release for an action. `slot` is the position of this node among the
    /// Toggle/Turbo nodes of the input's action (see [`StateId`]).
    #[expect(clippy::too_many_lines, clippy::too_many_arguments, clippy::cognitive_complexity, reason = "predates the size lints")]
    fn emit(&mut self, src: &Source, action: &ButtonAction, pressed: bool, slot: usize, out: &mut Vec<OutEvent>) {
        match action {
            ButtonAction::Disabled => {}
            ButtonAction::Gamepad(b) => out.push(OutEvent::PadButton(*b, pressed)),
            ButtonAction::PadStick { stick, .. } => {
                let push = (*stick, action.stick_push().map_or((0.0, 0.0), |(_, v)| v));
                if pressed {
                    self.pushed_sticks.push(push);
                } else if let Some(i) = self.pushed_sticks.iter().position(|p| *p == push) {
                    self.pushed_sticks.remove(i);
                }
                self.emit_pad_stick(*stick, out);
            }
            ButtonAction::PadTrigger { trigger, amount } => {
                let push = (*trigger, amount.clamp(0.0, 1.0));
                if pressed {
                    self.pushed_triggers.push(push);
                } else if let Some(i) = self.pushed_triggers.iter().position(|p| *p == push) {
                    self.pushed_triggers.remove(i);
                }
                self.emit_pad_trigger(*trigger, out);
            }
            ButtonAction::Mouse(m) => out.push(OutEvent::MouseButton(*m, pressed)),
            // One notch right away; `tick_wheel` continues while held.
            ButtonAction::Wheel(d) => {
                if pressed {
                    let (h, v) = d.vector();
                    out.push(OutEvent::Wheel {
                        vertical: (v * WHEEL_UNITS_PER_NOTCH) as i32,
                        horizontal: (h * WHEEL_UNITS_PER_NOTCH) as i32,
                    });
                }
            }
            ButtonAction::Keys(keys) => {
                let codes = keys.iter().filter_map(|k| parse_key(k));
                // Press modifiers first, release them last.
                if pressed {
                    out.extend(codes.map(|k| OutEvent::Key(k, true)));
                } else {
                    let codes: Vec<_> = codes.collect();
                    out.extend(codes.into_iter().rev().map(|k| OutEvent::Key(k, false)));
                }
            }
        ButtonAction::ToggleOverlay => {
            if pressed {
                self.overlay_toggled = Some(crate::keyboard::Layout::Keyboard);
            }
        }
        ButtonAction::ToggleNumpad => {
            if pressed {
                self.overlay_toggled = Some(crate::keyboard::Layout::Numpad);
            }
        }
        ButtonAction::Screenshot => {
            if pressed {
                self.screenshot_requested = true;
            }
        }
        ButtonAction::ToggleRecording => {
            if pressed {
                self.recording_toggled = true;
            }
        }
        ButtonAction::ToggleMedia => {
            if pressed {
                self.media_toggled = true;
            }
        }
        ButtonAction::Tools => {
            if pressed {
                self.tools_requested = true;
            }
        }
        ButtonAction::QuickSettings => {
            if pressed {
                self.settings_requested = true;
                self.close_guide_overlay();
            }
        }
        ButtonAction::GyroToggle => self.gyro.toggle_pending |= pressed,
        ButtonAction::GyroRecenter => self.gyro.recenter_pending |= pressed,
        ButtonAction::GyroClutch => {
            if pressed {
                self.gyro.clutch_holds += 1;
            } else {
                self.gyro.clutch_holds = self.gyro.clutch_holds.saturating_sub(1);
            }
        }
        ButtonAction::GyroHold => {
            if pressed {
                self.gyro.on_holds += 1;
            } else {
                self.gyro.on_holds = self.gyro.on_holds.saturating_sub(1);
            }
        }
        ButtonAction::Magnifier(control) => {
            if pressed {
                self.lens_control(*control);
            }
        }
        ButtonAction::ForceQuit => {
            if pressed {
                self.force_quit_holds += 1;
            } else {
                self.force_quit_holds = self.force_quit_holds.saturating_sub(1);
            }
        }
        ButtonAction::ShowInfo(name) => {
            count_hold(&mut self.info_holds, name, pressed);
            self.info_changed = true;
        }
        ButtonAction::ShowLog(name) => {
            count_hold(&mut self.log_holds, name, pressed);
            self.info_changed = true;
        }
        ButtonAction::OpenMenu(name) => {
            if pressed {
                self.menu_request = Some((name.clone(), src.opener()));
            }
        }
        ButtonAction::Layer(name) => self.hold_layer(name, pressed),
        ButtonAction::Mode(name) => {
            if pressed {
                self.switch_mode(name);
            }
        }
            ButtonAction::Multi(actions) => {
                // Child slots depend only on position, so reverse-order release matches.
                let mut slots = Vec::with_capacity(actions.len());
                let mut next = slot;
                for a in actions {
                    slots.push(next);
                    next += stateful_nodes(a);
                }
                if pressed {
                    for (a, s) in actions.iter().zip(&slots) {
                        self.emit(src, a, true, *s, out);
                    }
                } else {
                    for (a, s) in actions.iter().zip(&slots).rev() {
                        self.emit(src, a, false, *s, out);
                    }
                }
            }
            // Only presses matter: each one flips the inner action on or off.
            ButtonAction::Toggle(Toggled { action: inner, .. }) => {
                if !pressed {
                    return;
                }
                let id = (src.clone(), slot);
                if let Some(inner) = self.toggled.remove(&id) {
                    self.emit(src, &inner, false, slot + 1, out);
                    return;
                }
                self.toggle_on(src, inner, slot, out);
            }
            ButtonAction::Turbo { action: inner, rate, every_ms } => {
                let id = (src.clone(), slot);
                if pressed {
                    if self.turbo.contains_key(&id) {
                        return;
                    }
                    let (inner, half_period) = self.turbo_pace(inner, *rate, *every_ms);
                    self.turbo.insert(id, TurboState { action: inner.clone(), half_period, elapsed: 0.0, down: true });
                    self.emit(src, &inner, true, slot + 1, out);
                    return;
                }
                if let Some(state) = self.turbo.remove(&id)
                    && state.down
                {
                    self.emit(src, &state.action, false, slot + 1, out);
                }
            }
            // Play once per press (release doesn't stop it), or loop while held.
            ButtonAction::Macro { name, repeat } => {
                let id = (src.clone(), slot);
                if !pressed {
                    if *repeat {
                        self.stop_macro(&id, out);
                    }
                    return;
                }
                if self.macros_running.contains_key(&id) {
                    return;
                }
                let Some(ops) = self.macro_defs.get(name).cloned() else {
                    crate::monitor::log!("no macro named {name:?}");
                    return;
                };
                let run = MacroRun::new(ops, *repeat);
                self.macros_running.insert(id.clone(), run);
                self.advance_macro(&id, 0.0, out);
            }
            ButtonAction::Cycle { name, back } => self.cycle((src.clone(), slot), (name, *back), pressed, out),
        }
    }

    /// Turns a Toggle (at `slot` of `src`'s action) on, pressing its inner action.
    fn toggle_on(&mut self, src: &Source, inner: &ButtonAction, slot: usize, out: &mut Vec<OutEvent>) {
        self.toggled.insert((src.clone(), slot), inner.clone());
        self.emit(src, inner, true, slot + 1, out);
    }

    /// Switches on every Toggle set to start on, in the profile's mappings and the game's menu
    /// items, as if pressed (one that's on already stays on). For when the game starts.
    pub fn start_toggles(&mut self, profile: &Profile, menus: &[Menu], out: &mut Vec<OutEvent>) {
        let mut inputs: Vec<(Source, &ButtonAction)> = Vec::new();
        inputs.extend(profile.buttons.iter().map(|(b, a)| (Source::Button(*b), a)));
        for (b, g) in &profile.gestures {
            inputs.extend(GestureKind::TIMED.into_iter().filter_map(|k| g.get(k)).map(|a| (Source::Gesture(*b), a)));
        }
        fn edges<'a>(input: &Source, g: &'a crate::config::Gestures) -> Vec<(Source, &'a ButtonAction)> {
            GestureKind::EDGES.into_iter().filter_map(|k| g.get(k).map(|a| (Source::Edge(Box::new(input.clone()), k), a))).collect()
        }
        for (b, g) in &profile.gestures {
            inputs.extend(edges(&Source::Button(*b), g));
        }
        for c in &profile.combos {
            let mut id = c.buttons.clone();
            id.sort();
            id.dedup();
            inputs.push((Source::Combo(id), &c.action));
        }
        for t in [Trigger::Left, Trigger::Right] {
            if let TriggerAction::Button { action, .. } = profile.trigger(t) {
                inputs.push((Source::Trigger(t), action));
                inputs.extend(edges(&Source::Trigger(t), &profile.trigger_config(t).gestures));
            }
        }
        for &side in profile.touchpad.sides() {
            inputs.push((Source::TouchClick(side), profile.touchpad.click(side)));
            let swipes = SwipeDirection::ALL.into_iter().filter_map(|d| profile.touchpad.motion(side).swipe(d).map(|a| (Source::TouchSwipe(side, d), a)));
            inputs.extend(swipes);
        }
        for a in [Analog::Stick(Stick::Left), Analog::Stick(Stick::Right), Analog::Trigger(Trigger::Left), Analog::Trigger(Trigger::Right)] {
            inputs.extend(profile.zones(a).iter().enumerate().map(|(i, z)| (Source::Zone(a, i), &z.action)));
        }
        for m in menus {
            inputs.extend(m.items.iter().enumerate().map(|(i, item)| (Source::MenuItem(m.name.clone(), i), &item.action)));
        }
        for (src, action) in inputs {
            let mut starting = Vec::new();
            start_on_toggles(action, 0, &mut starting);
            for (slot, inner) in starting {
                if !self.toggled.contains_key(&(src.clone(), slot)) {
                    self.toggle_on(&src, inner, slot, out);
                }
            }
        }
    }

    /// Sends a virtual-pad trigger: the physical trigger fed to it, or the deepest pull an action
    /// holds, whichever is further.
    fn emit_pad_trigger(&self, t: Trigger, out: &mut Vec<OutEvent>) {
        let fed = self.trigger_fed.get(&t).copied().unwrap_or(0.0);
        let pulled = self.pushed_triggers.iter().filter(|(p, _)| *p == t).map(|(_, a)| *a).fold(0.0, f32::max);
        out.push(OutEvent::PadAxis(trigger_axis(t), fed.max(pulled)));
    }

    fn trigger(&mut self, profile: &Profile, t: Trigger, value: f32, out: &mut Vec<OutEvent>) {
        let action = profile.trigger(t);
        // A virtual trigger no longer fed (a layer changed this one's mode) goes back to rest.
        let target = match action {
            TriggerAction::Gamepad(target) => Some(*target),
            _ => None,
        };
        if let Some(old) = self.trigger_feeds.get(&t).copied()
            && Some(old) != target
        {
            self.trigger_feeds.remove(&t);
            self.trigger_fed.remove(&old);
            self.emit_pad_trigger(old, out);
        }
        let src = Source::Trigger(t);
        // A pull keeps its action until let go past where it pressed.
        if let Some(threshold) = self.trigger_release.get(&t).copied()
            && !analog_pressed(value, threshold, true, TRIGGER_HYSTERESIS)
        {
            self.trigger_release.remove(&t);
            self.digital(&src, &ButtonAction::Disabled, false, out);
            self.edges(&src, None, false, out);
        }
        match action {
            TriggerAction::Disabled => {}
            TriggerAction::Gamepad(target) => {
                self.trigger_feeds.insert(t, *target);
                self.trigger_fed.insert(*target, value);
                self.emit_pad_trigger(*target, out);
            }
            TriggerAction::Button { action, threshold } => {
                if !self.trigger_release.contains_key(&t) && analog_pressed(value, *threshold, false, TRIGGER_HYSTERESIS) {
                    self.trigger_release.insert(t, *threshold);
                    // A pull with Guide held is a chord: letting go of Guide opens nothing.
                    self.forget_guide_tap();
                    self.digital(&src, action, true, out);
                    self.edges(&src, profile.trigger_gestures(t), true, out);
                }
            }
        }
        self.zones(profile, Analog::Trigger(t), value, out);
    }

    /// Presses/releases zone actions for an analog input at `value` (0.0..1.0). Zones are
    /// inactive at rest (value 0), so a zone starting at 0 means "as soon as it moves".
    fn zones(&mut self, profile: &Profile, analog: Analog, value: f32, out: &mut Vec<OutEvent>) {
        let inside = |(min, max): (f32, f32), margin: f32| {
            // The top zone includes full deflection itself.
            let below_max = max >= 1.0 || value < max + margin;
            value > 0.0 && value >= min - margin && below_max
        };
        // Release first, by the bounds each active zone pressed with (a layer may have
        // changed the zones since), so moving between adjacent zones never holds both.
        let leaving: Vec<usize> = self
            .zone_bounds
            .iter()
            .filter(|((a, _), bounds)| *a == analog && !inside(**bounds, ZONE_HYSTERESIS))
            .map(|((_, i), _)| *i)
            .collect();
        for i in leaving {
            self.zone_bounds.remove(&(analog, i));
            self.digital(&Source::Zone(analog, i), &ButtonAction::Disabled, false, out);
        }
        for (i, zone) in profile.zones(analog).iter().enumerate() {
            let bounds = (zone.min, zone.max);
            if !self.zone_bounds.contains_key(&(analog, i)) && inside(bounds, 0.0) {
                self.zone_bounds.insert((analog, i), bounds);
                self.digital(&Source::Zone(analog, i), &zone.action, true, out);
            }
        }
    }

    /// Whether `tick` currently has anything to do: a mouse/scroll stick is deflected or a
    /// wheel action is held.
    pub fn needs_tick(&self, profile: &Profile) -> bool {
        self.sticks_need_tick(profile)
            || !self.turbo.is_empty()
            || !self.macros_running.is_empty()
            || self.touch.gliding()
            || self.lens_moving()
            || self.held.values().any(|a| !a.wheel_directions().is_empty())
    }

    /// Advances continuous outputs (mouse motion, scrolling) by `dt` seconds.
    pub fn tick(&mut self, profile: &Profile, dt: f32, out: &mut Vec<OutEvent>) {
        self.tick_sticks(profile, dt, out);
        self.tick_wheel(dt, out);
        self.tick_turbo(dt, out);
        self.tick_macros(dt, out);
        self.tick_touch(dt, out);
        self.tick_lens(dt);
    }

    /// Keeps scrolling for wheel actions held past the repeat delay.
    fn tick_wheel(&mut self, dt: f32, out: &mut Vec<OutEvent>) {
        self.wheel_held.retain(|src, _| self.held.contains_key(src));
        let (mut h, mut v) = (0.0, 0.0);
        for (src, action) in &self.held {
            let directions = action.wheel_directions();
            if directions.is_empty() {
                continue;
            }
            let held_for = self.wheel_held.entry(src.clone()).or_insert(0.0);
            *held_for += dt;
            if *held_for > WHEEL_REPEAT_DELAY {
                for d in directions {
                    let (dh, dv) = d.vector();
                    h += dh;
                    v += dv;
                }
            }
        }
        let units = WHEEL_REPEAT_RATE * WHEEL_UNITS_PER_NOTCH * dt;
        let (h, v) = take_whole(&mut self.wheel_acc, h * units, v * units);
        if h != 0 || v != 0 {
            out.push(OutEvent::Wheel { vertical: v, horizontal: h });
        }
    }

    /// Releases everything this engine holds down and centers the virtual pad. Used before a
    /// profile switch, when an on-screen overlay takes the controller, or when the device goes
    /// away. With `keep_mode`, the mode stays (held layers end either way).
    pub fn release_all(&mut self, keep_mode: bool, out: &mut Vec<OutEvent>) {
        if !keep_mode && self.mode.take().is_some() {
            self.layers_changed = true;
        }
        if self.guide_held.take().is_some() {
            self.layers_changed = true;
        }
        if self.guide_open {
            self.set_guide_open(false);
        }
        let held: Vec<_> = self.held.drain().collect();
        for (src, action) in held {
            self.emit(&src, &action, false, 0, out);
        }
        // Inputs still down get their On release now, so a game's toggle their press turned on
        // goes back off; letting go of them later does nothing.
        let releases: Vec<_> = self.on_release.drain().collect();
        for (input, action) in releases {
            self.tap(&Source::Edge(Box::new(input), GestureKind::OnRelease), &action, out);
        }
        // Toggled-on actions are released too (this also stops toggled turbos).
        let toggled: Vec<_> = self.toggled.drain().collect();
        for ((src, slot), inner) in toggled {
            self.emit(&src, &inner, false, slot + 1, out);
        }
        self.release_cycles(out);
        for id in self.macros_running.keys().cloned().collect::<Vec<_>>() {
            self.stop_macro(&id, out);
        }
        // Anything still repeating (normally nothing by now) is stopped as well.
        for ((src, slot), state) in self.turbo.drain().collect::<Vec<_>>() {
            if state.down {
                self.emit(&src, &state.action, false, slot + 1, out);
            }
        }
        // Held combo members stay silent until released rather than firing under a new profile.
        for m in self.members.values_mut() {
            *m = ComboMember::Consumed;
        }
        // Unfinished tap sequences are dropped.
        self.gestures.clear();
        self.pad_feeds.clear();
        self.trigger_feeds.clear();
        self.trigger_release.clear();
        self.zone_bounds.clear();
        for axis in [Axis::LeftX, Axis::LeftY, Axis::RightX, Axis::RightY, Axis::LeftTrigger, Axis::RightTrigger] {
            out.push(OutEvent::PadAxis(axis, 0.0));
        }
        self.mouse_acc = (0.0, 0.0);
        self.scroll_acc = (0.0, 0.0);
        self.stick_smooth.clear();
        self.stick_ramp.clear();
        self.ring_sector.clear();
        self.stick_region.clear();
        self.flick.clear();
        self.wheel_held.clear();
        self.wheel_acc = (0.0, 0.0);
        // Pad axes were centered above; forget what fed them. Tilt tracking carries on.
        self.pad_sticks.clear();
        self.gyro.reset();
        self.touch.reset();
    }

    /// Notes where a stick or trigger is while an overlay has the controller and the engine
    /// hears nothing else. Without it, a trigger let go of under the overlay stays pulled here,
    /// and the next layer change (Guide held) re-applies it: Guide + right trigger, which opened
    /// the Tools menu, would open it again on the next Guide press.
    pub fn note_axis(&mut self, axis: Axis, value: f32) {
        self.axes.insert(axis, value);
    }

    /// Re-applies current analog state under a (new) profile, e.g. after switching.
    pub fn resync(&mut self, profile: &Profile, out: &mut Vec<OutEvent>) {
        self.stick(profile, Stick::Left, out);
        self.stick(profile, Stick::Right, out);
        for (t, axis) in [(Trigger::Left, Axis::LeftTrigger), (Trigger::Right, Axis::RightTrigger)] {
            let value = self.axes.get(&axis).copied().unwrap_or(0.0);
            self.trigger(profile, t, value, out);
        }
    }
}

/// Whether an analog input at `value` (0 at rest) presses a button set to press at `threshold`,
/// given whether it's pressed now. It lets go `hysteresis` below the threshold, and always at
/// rest, so a threshold at or below the hysteresis can't hold the button forever.
fn analog_pressed(value: f32, threshold: f32, pressed: bool, hysteresis: f32) -> bool {
    let at = if pressed { threshold - hysteresis } else { threshold };
    value > 0.0 && value >= at
}

/// Counts one more (or one fewer) hold on a named overlay, forgetting it at none.
fn count_hold(holds: &mut HashMap<String, u32>, name: &str, pressed: bool) {
    if pressed {
        *holds.entry(name.to_string()).or_insert(0) += 1;
    } else if let Some(n) = holds.get_mut(name) {
        *n -= 1;
        if *n == 0 {
            holds.remove(name);
        }
    }
}

/// Difference between two angles in degrees, wrapped to -180..180.
fn angle_diff(a: f32, b: f32) -> f32 {
    (a - b + 540.0).rem_euclid(360.0) - 180.0
}

/// The Toggles set to start on in `action` (at `slot`), with their slots, numbered as
/// [`Engine::emit`] numbers them.
fn start_on_toggles<'a>(action: &'a ButtonAction, slot: usize, found: &mut Vec<(usize, &'a ButtonAction)>) {
    match action {
        ButtonAction::Multi(list) => {
            let mut next = slot;
            for a in list {
                start_on_toggles(a, next, found);
                next += stateful_nodes(a);
            }
        }
        ButtonAction::Toggle(t) => {
            if t.start_on {
                found.push((slot, &t.action));
            }
            start_on_toggles(&t.action, slot + 1, found);
        }
        ButtonAction::Turbo { action, .. } => start_on_toggles(action, slot + 1, found),
        _ => {}
    }
}

/// Number of Toggle/Turbo nodes in an action, each of which owns a [`StateId`] slot.
fn stateful_nodes(action: &ButtonAction) -> usize {
    match action {
        ButtonAction::Toggle(Toggled { action: inner, .. }) | ButtonAction::Turbo { action: inner, .. } => 1 + stateful_nodes(inner),
        ButtonAction::Macro { .. } | ButtonAction::Cycle { .. } => 1,
        ButtonAction::Multi(actions) => actions.iter().map(stateful_nodes).sum(),
        _ => 0,
    }
}

pub fn parse_key(name: &str) -> Option<KeyCode> {
    match KeyCode::from_str(name) {
        Ok(k) => Some(k),
        Err(_) => {
            crate::monitor::log!("unknown key name in config: {name:?}");
            None
        }
    }
}

fn stick_axes(s: Stick) -> (Axis, Axis) {
    match s {
        Stick::Left => (Axis::LeftX, Axis::LeftY),
        Stick::Right => (Axis::RightX, Axis::RightY),
    }
}

/// The stick an axis belongs to, if it is a stick axis.
fn axis_stick(axis: Axis) -> Option<Stick> {
    match axis {
        Axis::LeftX | Axis::LeftY => Some(Stick::Left),
        Axis::RightX | Axis::RightY => Some(Stick::Right),
        Axis::LeftTrigger | Axis::RightTrigger => None,
    }
}

fn trigger_axis(t: Trigger) -> Axis {
    match t {
        Trigger::Left => Axis::LeftTrigger,
        Trigger::Right => Axis::RightTrigger,
    }
}

/// A deadzone of the given settings, rescaled so output still spans the full 0..1 range.
pub(crate) fn apply_deadzone(x: f32, y: f32, dead: impl Into<Deadzone>) -> (f32, f32) {
    let dz = dead.into();
    match dz.shape {
        DeadzoneShape::Circle => radial(x, y, dz, x.hypot(y)),
        DeadzoneShape::Square => radial(x, y, dz, x.abs().max(y.abs())),
        DeadzoneShape::Cross => (axis(x, dz), axis(y, dz)),
    }
}

/// The output for a push of `reach` (0..1) past the deadzone: flat at the anti-deadzone through
/// the buffer, then rising to full.
fn dead_response(reach: f32, dz: Deadzone) -> f32 {
    if reach <= dz.size {
        return 0.0;
    }
    let start = (dz.size + dz.buffer).min(1.0);
    let rise = ((reach - start) / (1.0 - start).max(f32::EPSILON)).clamp(0.0, 1.0);
    dz.anti + (1.0 - dz.anti) * rise
}

/// A deadzone measured by `reach`, applied along the stick's direction.
fn radial(x: f32, y: f32, dz: Deadzone, reach: f32) -> (f32, f32) {
    let mag = x.hypot(y);
    if mag == 0.0 {
        return (0.0, 0.0);
    }
    let out = dead_response(reach, dz);
    // The output's size is measured the same way as the reach, so a square's corners reach 1.
    let k = if reach == 0.0 { 0.0 } else { out / reach };
    (x * k, y * k)
}

fn axis(v: f32, dz: Deadzone) -> f32 {
    dead_response(v.abs(), dz).copysign(v)
}

/// Adds a delta to a sub-unit accumulator and returns the whole units ready to emit.
fn take_whole(acc: &mut (f32, f32), dx: f32, dy: f32) -> (i32, i32) {
    acc.0 += dx;
    acc.1 += dy;
    let whole = (acc.0.trunc(), acc.1.trunc());
    acc.0 -= whole.0;
    acc.1 -= whole.1;
    (whole.0 as i32, whole.1 as i32)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod cycle_tests;
