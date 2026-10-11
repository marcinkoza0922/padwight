//! Touchpad input. Each finger is followed from where it lands to where it lifts, on the side of
//! the pad it landed on (a split pad has two). What a side does with its fingers is its mode:
//! mouse (with tap to click, two-finger scrolling and gliding), a virtual stick, a menu laid out
//! on the pad, or swipes. The click runs the side's click action, or chooses the menu item under
//! the finger.

use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use crate::{
    config::{ButtonAction, Profile, SpeedCurve, SwipeDirection, TouchChoose, TouchSide, TouchpadConfig, TouchpadMotion},
    input::TouchpadEvent,
    output::OutEvent,
};

use super::{Engine, Source};

/// A finger movement report's usual interval (seconds), assumed for the first one of a stroke.
const REPORT_INTERVAL: f32 = 0.008;
/// A pause this long (seconds) starts the finger's speed afresh.
const STROKE_GAP: f32 = 0.1;
/// Time constant (seconds) smoothing the finger's speed, so one uneven report doesn't jolt it.
const SPEED_SMOOTHING: f32 = 0.03;
/// A tap is a touch this short that moves no further than `TAP_SLOP` pad widths.
const TAP_TIME: Duration = Duration::from_millis(200);
const TAP_SLOP: f32 = 0.03;
/// A swipe is a stroke this quick.
const SWIPE_TIME: Duration = Duration::from_millis(400);
/// A finger lifting this fast (pad widths/second) leaves the pointer gliding...
const GLIDE_FROM: f32 = 0.6;
/// ...unless it stopped this long before lifting.
const GLIDE_STALE: Duration = Duration::from_millis(60);

/// One finger on the pad. Positions are 0..1 across the whole pad.
#[derive(Debug, Clone, Copy)]
struct Finger {
    /// Its contact slot on the pad.
    id: u8,
    side: TouchSide,
    at: (f32, f32),
    landed: (f32, f32),
    landed_at: Instant,
    moved_at: Instant,
    /// Smoothed, in pad widths/second.
    velocity: (f32, f32),
    /// Pad widths moved in all.
    travelled: f32,
    /// The pad's height in widths.
    height: f32,
}

impl Finger {
    /// The finger moved to `at`, with its speed brought up to date, and how far it moved (pad widths).
    fn moved_to(mut self, at: (f32, f32), now: Instant) -> (Finger, (f32, f32)) {
        let delta = (at.0 - self.at.0, (at.1 - self.at.1) * self.height);
        let dt = Some(now.duration_since(self.moved_at).as_secs_f32()).filter(|dt| *dt < STROKE_GAP && self.travelled > 0.0);
        let step = dt.unwrap_or(REPORT_INTERVAL).max(1e-3);
        let speed = (delta.0 / step, delta.1 / step);
        self.velocity = match dt {
            Some(dt) => {
                let k = 1.0 - (-dt / SPEED_SMOOTHING).exp();
                (self.velocity.0 + (speed.0 - self.velocity.0) * k, self.velocity.1 + (speed.1 - self.velocity.1) * k)
            }
            None => speed,
        };
        self.at = at;
        self.moved_at = now;
        self.travelled += delta.0.hypot(delta.1);
        (self, delta)
    }

    /// From where it landed to where it is, in pad widths.
    fn stroke(&self) -> (f32, f32) {
        (self.at.0 - self.landed.0, (self.at.1 - self.landed.1) * self.height)
    }
}

/// The menu a finger on the pad is showing, for the overlay.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchMenu {
    pub menu: String,
    /// The item under the finger.
    pub item: Option<usize>,
    pub choose: TouchChoose,
    /// Items chosen from the pad so far.
    pub picks: u32,
}

#[derive(Debug, Default)]
pub(super) struct TouchState {
    fingers: BTreeMap<u8, Finger>,
    /// Sub-pixel pointer movement and sub-unit scrolling, not yet sent.
    pointer_acc: (f32, f32),
    scroll_acc: (f32, f32),
    /// What the click is holding down.
    click: Option<Source>,
    /// The menu item under the finger on each side in menu mode.
    picked: BTreeMap<TouchSide, usize>,
    /// Sides whose finger chose its item with a click, so lifting doesn't choose it again.
    clicked_item: Vec<TouchSide>,
    picks: u32,
    shown: Option<TouchMenu>,
    shown_changed: bool,
    /// Pointer speed (pixels/second) still gliding after a flick.
    glide: Option<(f32, f32)>,
    /// The stick each side's joystick pushes, and how far.
    pub(super) sticks: BTreeMap<TouchSide, (crate::config::Stick, (f32, f32))>,
}

impl TouchState {
    /// The finger first down on `side`, which is the one that counts when one is enough.
    fn primary(&self, side: TouchSide) -> Option<u8> {
        self.fingers.iter().find(|(_, f)| f.side == side).map(|(n, _)| *n)
    }

    /// How far touch joysticks push the virtual stick `target`.
    pub(super) fn stick_push(&self, target: crate::config::Stick) -> (f32, f32) {
        self.sticks.values().filter(|(s, _)| *s == target).fold((0.0, 0.0), |(x, y), (_, (dx, dy))| (x + dx, y + dy))
    }

    pub(super) fn gliding(&self) -> bool {
        self.glide.is_some()
    }

    /// Forgets the fingers and lets the glide stop; the held click is released with the rest of
    /// the engine's held inputs.
    pub(super) fn reset(&mut self) {
        let shown = self.shown.take();
        *self = TouchState { picks: self.picks, shown_changed: self.shown_changed || shown.is_some(), ..TouchState::default() };
    }
}

/// The side of `cfg`'s pad that `x` (0..1 across) is on.
fn side_at(cfg: &TouchpadConfig, x: f32) -> TouchSide {
    if cfg.right.is_some() && x >= 0.5 { TouchSide::Right } else { TouchSide::Left }
}

/// Where `at` is within `side` (each 0..1 across it), and the side's height in its widths.
fn local(cfg: &TouchpadConfig, side: TouchSide, (x, y): (f32, f32), height: f32) -> ((f32, f32), f32) {
    match (cfg.right.is_some(), side) {
        (false, _) => ((x, y), height),
        (true, TouchSide::Left) => (((x * 2.0).clamp(0.0, 1.0), y), height * 2.0),
        (true, TouchSide::Right) => (((x * 2.0 - 1.0).clamp(0.0, 1.0), y), height * 2.0),
    }
}

impl Engine {
    /// Handles one touchpad event.
    pub(super) fn touchpad(&mut self, profile: &Profile, ev: TouchpadEvent, now: Instant, out: &mut Vec<OutEvent>) {
        let cfg = &profile.touchpad;
        match ev {
            TouchpadEvent::Click(pressed) => self.touch_click(cfg, pressed, out),
            TouchpadEvent::Touch { finger, x, y, height } => match self.touch.fingers.get(&finger).copied() {
                Some(f) => {
                    let (moved, delta) = f.moved_to((x, y), now);
                    self.finger_moved(cfg, moved, delta, out);
                }
                None => {
                    let side = side_at(cfg, x);
                    let f = Finger { id: finger, side, at: (x, y), landed: (x, y), landed_at: now, moved_at: now, velocity: (0.0, 0.0), travelled: 0.0, height };
                    self.finger_landed(cfg, f, out);
                }
            },
            TouchpadEvent::Lift { finger } => self.finger_lifted(cfg, finger, now, out),
        }
        self.refresh_touch_menu(cfg);
    }

    /// The menu a finger on the pad shows right now, if any.
    pub fn touch_menu(&self) -> Option<&TouchMenu> {
        self.touch.shown.as_ref()
    }

    /// Whether the touch menu (or the item picked in it) changed since the last call.
    pub fn take_touch_menu_changed(&mut self) -> bool {
        std::mem::take(&mut self.touch.shown_changed)
    }

    fn finger_landed(&mut self, cfg: &TouchpadConfig, f: Finger, out: &mut Vec<OutEvent>) {
        self.touch.glide = None;
        self.touch.fingers.insert(f.id, f);
        if self.touch.primary(f.side) == Some(f.id) {
            self.follow_primary(cfg, f.side, f, out);
        }
    }

    /// A finger moved by `delta` pad widths, to where `f` now is.
    fn finger_moved(&mut self, cfg: &TouchpadConfig, f: Finger, delta: (f32, f32), out: &mut Vec<OutEvent>) {
        self.touch.fingers.insert(f.id, f);
        let primary = self.touch.primary(f.side) == Some(f.id);
        if let TouchpadMotion::Mouse { speed, curve, scroll, .. } = cfg.motion(f.side) {
            let fingers = self.touch.fingers.values().filter(|o| o.side == f.side).count();
            if *scroll && fingers > 1 {
                self.scroll_with(delta, fingers, out);
            } else if primary {
                self.point_with(delta, *speed * finger_gain(curve, f.velocity), out);
            }
        } else if primary {
            self.follow_primary(cfg, f.side, f, out);
        }
    }

    fn finger_lifted(&mut self, cfg: &TouchpadConfig, n: u8, now: Instant, out: &mut Vec<OutEvent>) {
        let primary = self.touch.fingers.get(&n).is_some_and(|f| self.touch.primary(f.side) == Some(n));
        let Some(f) = self.touch.fingers.remove(&n) else { return };
        let side = f.side;
        let alone = self.touch.primary(side).is_none();
        match cfg.motion(side) {
            TouchpadMotion::Mouse { speed, curve, tap, inertia, .. } => {
                let quick = now.duration_since(f.landed_at) <= TAP_TIME && f.travelled <= TAP_SLOP;
                if *tap && quick && alone && self.touch.click.is_none() {
                    self.tap(&Source::TouchClick(side), cfg.click(side), out);
                }
                let (vx, vy) = f.velocity;
                let fresh = now.duration_since(f.moved_at) < GLIDE_STALE;
                if *inertia && alone && primary && fresh && vx.hypot(vy) >= GLIDE_FROM {
                    let px = *speed * finger_gain(curve, f.velocity);
                    self.touch.glide = Some((vx * px, vy * px));
                }
            }
            TouchpadMotion::Menu { menu, choose, .. } if primary => {
                if let Some(i) = self.touch.picked.remove(&side) {
                    let clicked = self.touch.clicked_item.contains(&side);
                    if *choose == TouchChoose::Lift && !clicked {
                        self.choose_touch_item(menu, i, true, out);
                    }
                }
                self.touch.clicked_item.retain(|s| *s != side);
            }
            TouchpadMotion::Swipe { distance, .. } => {
                let (dx, dy) = f.stroke();
                let dir = match (dx.abs() > dy.abs(), dx > 0.0, dy > 0.0) {
                    (true, true, _) => SwipeDirection::Right,
                    (true, false, _) => SwipeDirection::Left,
                    (false, _, true) => SwipeDirection::Down,
                    (false, _, false) => SwipeDirection::Up,
                };
                if now.duration_since(f.landed_at) <= SWIPE_TIME && dx.hypot(dy) >= *distance
                    && let Some(action) = cfg.motion(side).swipe(dir)
                {
                    self.tap(&Source::TouchSwipe(side, dir), action, out);
                }
            }
            TouchpadMotion::Joystick { .. } | TouchpadMotion::Menu { .. } | TouchpadMotion::Off => {}
        }
        // The next finger on the side takes over, or the side lets go.
        match self.touch.primary(side).and_then(|n| self.touch.fingers.get(&n).copied()) {
            Some(next) if primary => self.follow_primary(cfg, side, next, out),
            Some(_) => {}
            None => {
                if let Some((stick, _)) = self.touch.sticks.remove(&side) {
                    self.emit_pad_stick(stick, out);
                }
                self.touch.picked.remove(&side);
            }
        }
    }

    /// What the finger that counts on `side` does in the modes that follow its position.
    fn follow_primary(&mut self, cfg: &TouchpadConfig, side: TouchSide, f: Finger, out: &mut Vec<OutEvent>) {
        match cfg.motion(side) {
            TouchpadMotion::Joystick { stick, relative, radius, deadzone } => {
                let (x, y) = if *relative {
                    let (dx, dy) = f.stroke();
                    (dx / radius.max(0.01), dy / radius.max(0.01))
                } else {
                    let ((x, y), _) = local(cfg, side, f.at, f.height);
                    ((x - 0.5) * 2.0, (y - 0.5) * 2.0)
                };
                let push = past_deadzone((x, y), *deadzone);
                if let Some((old, _)) = self.touch.sticks.insert(side, (*stick, push))
                    && old != *stick
                {
                    self.emit_pad_stick(old, out);
                }
                self.emit_pad_stick(*stick, out);
            }
            TouchpadMotion::Menu { menu, .. } => {
                let (pos, height) = local(cfg, side, f.at, f.height);
                let item = self.menus.iter().find(|m| &m.name == menu).and_then(|m| crate::menu::item_at(m, pos, height));
                match item {
                    Some(i) => self.touch.picked.insert(side, i),
                    None => self.touch.picked.remove(&side),
                };
            }
            TouchpadMotion::Mouse { .. } | TouchpadMotion::Swipe { .. } | TouchpadMotion::Off => {}
        }
    }

    /// The click: chooses the menu item under the finger, or runs the click action of the side
    /// the first finger is on.
    fn touch_click(&mut self, cfg: &TouchpadConfig, pressed: bool, out: &mut Vec<OutEvent>) {
        if !pressed {
            if let Some(src) = self.touch.click.take() {
                self.digital(&src, &ButtonAction::Disabled, false, out);
            }
            return;
        }
        if self.touch.click.is_some() {
            return;
        }
        self.touch.glide = None;
        let side = self.touch.fingers.values().next().map_or(TouchSide::Left, |f| f.side);
        if let TouchpadMotion::Menu { menu, .. } = cfg.motion(side)
            && let Some(&i) = self.touch.picked.get(&side)
            && let Some(src) = self.choose_touch_item(menu, i, false, out)
        {
            self.touch.clicked_item.push(side);
            self.touch.click = Some(src);
            return;
        }
        let src = Source::TouchClick(side);
        self.digital(&src, cfg.click(side), true, out);
        self.touch.click = Some(src);
    }

    /// Presses item `i` of `menu` (and with `tap`, releases it again). Returns what holds it, or
    /// `None` when there's no such item or it does nothing.
    fn choose_touch_item(&mut self, menu: &str, i: usize, tap: bool, out: &mut Vec<OutEvent>) -> Option<Source> {
        let action = self.menus.iter().find(|m| m.name == menu)?.items.get(i)?.action.clone();
        if action == ButtonAction::Disabled {
            return None;
        }
        let src = Source::MenuItem(menu.to_string(), i);
        self.digital(&src, &action, true, out);
        if tap {
            self.digital(&src, &action, false, out);
        }
        self.touch.picks += 1;
        self.touch.shown_changed = true;
        Some(src)
    }

    /// Works out which menu the pad shows: the first side in menu mode with a finger on it and
    /// its overlay on.
    fn refresh_touch_menu(&mut self, cfg: &TouchpadConfig) {
        let shown = cfg.sides().iter().find_map(|&side| match cfg.motion(side) {
            TouchpadMotion::Menu { menu, choose, overlay: true } if self.touch.primary(side).is_some() => Some(TouchMenu {
                menu: menu.clone(),
                item: self.touch.picked.get(&side).copied(),
                choose: *choose,
                picks: self.touch.picks,
            }),
            _ => None,
        });
        if shown != self.touch.shown {
            self.touch.shown = shown;
            self.touch.shown_changed = true;
        }
    }
}

/// The touchpad curve's gain for a finger moving at `velocity` (pad widths/second).
fn finger_gain(curve: &SpeedCurve, (vx, vy): (f32, f32)) -> f32 {
    if curve.is_flat() { 1.0 } else { curve.gain(vx.hypot(vy)) }
}

/// A push held to the unit circle, with pushes inside `deadzone` dropped and the rest rescaled so
/// the edge of the deadzone is no push and the circle is full.
fn past_deadzone((x, y): (f32, f32), deadzone: f32) -> (f32, f32) {
    let mag = x.hypot(y);
    if mag <= deadzone || mag == 0.0 {
        return (0.0, 0.0);
    }
    let scaled = ((mag - deadzone) / (1.0 - deadzone).max(1e-3)).min(1.0);
    (x / mag * scaled, y / mag * scaled)
}

mod mouse;
#[cfg(test)]
mod tests;
