//! A profile's touchpad settings: what fingers on the pad do, and what the click does. The pad
//! can be split into a left and a right half that work on their own.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{ButtonAction, MouseButton, MouseRegion, SpeedCurve, Stick};

/// Default pointer speed: pixels the pointer moves when a finger crosses the whole pad.
pub const DEFAULT_TOUCHPAD_SPEED: f32 = 1200.0;

/// A finger's speed in pad widths/second is read by acceleration curves made for degrees/second
/// of gyro turning at this scale.
pub const TOUCHPAD_CURVE_SCALE: f32 = 0.025;

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_false(v: &bool) -> bool {
    !*v
}

fn is_disabled(a: &ButtonAction) -> bool {
    *a == ButtonAction::Disabled
}

fn disabled() -> ButtonAction {
    ButtonAction::Disabled
}

fn yes() -> bool {
    true
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_true(v: &bool) -> bool {
    *v
}

/// When a touch menu runs the item under the finger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TouchChoose {
    /// Clicking the pad presses the item, held until the click lets go.
    #[default]
    Click,
    /// Lifting the finger runs the item (a click still works too).
    Lift,
}

impl TouchChoose {
    pub const ALL: [TouchChoose; 2] = [TouchChoose::Click, TouchChoose::Lift];
}

impl fmt::Display for TouchChoose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TouchChoose::Click => "Click the pad",
            TouchChoose::Lift => "Lift the finger",
        })
    }
}

/// The four ways a swipe can go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

impl SwipeDirection {
    pub const ALL: [SwipeDirection; 4] = [SwipeDirection::Up, SwipeDirection::Down, SwipeDirection::Left, SwipeDirection::Right];
}

impl fmt::Display for SwipeDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SwipeDirection::Up => "Swipe up",
            SwipeDirection::Down => "Swipe down",
            SwipeDirection::Left => "Swipe left",
            SwipeDirection::Right => "Swipe right",
        })
    }
}

/// Default swipe length, in pad widths.
pub const DEFAULT_SWIPE_DISTANCE: f32 = 0.2;

fn default_swipe_distance() -> f32 {
    DEFAULT_SWIPE_DISTANCE
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TouchpadMotion {
    /// Fingers do nothing. The pad is still taken from the desktop while the controller is
    /// managed, so the pointer doesn't move either.
    Off,
    /// Finger movement moves the mouse pointer. `speed` is the pixels across the whole pad;
    /// `curve` scales it by how fast the finger moves, in pad widths/second.
    Mouse {
        speed: f32,
        #[serde(default, skip_serializing_if = "SpeedCurve::is_flat")]
        curve: SpeedCurve,
        /// A quick tap runs the click action.
        #[serde(default, skip_serializing_if = "is_false")]
        tap: bool,
        /// Two fingers moving together scroll instead of moving the pointer.
        #[serde(default, skip_serializing_if = "is_false")]
        scroll: bool,
        /// A flick keeps the pointer gliding after the finger lifts, slowing down.
        #[serde(default, skip_serializing_if = "is_false")]
        inertia: bool,
    },
    /// The finger pushes a virtual-pad stick. `relative` puts the stick's center wherever the
    /// finger lands, and moving `radius` pad widths from there is full deflection; otherwise the
    /// middle of the pad (or half) is center and its edges are full deflection. Pushes smaller
    /// than `deadzone` (0..1) are ignored.
    Joystick { stick: Stick, relative: bool, radius: f32, deadzone: f32 },
    /// Mouse region: the pad (or half) is laid over `region`, and the pointer goes where the
    /// finger is. Lifting the finger leaves the pointer where it is.
    Region {
        #[serde(flatten)]
        region: MouseRegion,
    },
    /// The pad is divided into the items of `menu`, laid out as the menu is drawn: a radial menu
    /// by angle from the middle, a grid by cells, a list by rows. Touching shows the menu (unless
    /// `overlay` is off) with the item under the finger picked.
    Menu {
        menu: String,
        #[serde(default)]
        choose: TouchChoose,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        overlay: bool,
    },
    /// A quick stroke at least `distance` pad widths long runs the action for its direction.
    Swipe {
        #[serde(default = "disabled", skip_serializing_if = "is_disabled")]
        up: ButtonAction,
        #[serde(default = "disabled", skip_serializing_if = "is_disabled")]
        down: ButtonAction,
        #[serde(default = "disabled", skip_serializing_if = "is_disabled")]
        left: ButtonAction,
        #[serde(default = "disabled", skip_serializing_if = "is_disabled")]
        right: ButtonAction,
        #[serde(default = "default_swipe_distance")]
        distance: f32,
    },
}

impl TouchpadMotion {
    /// Mouse mode at `speed` with nothing else on.
    pub fn mouse(speed: f32) -> Self {
        TouchpadMotion::Mouse { speed, curve: SpeedCurve::default(), tap: false, scroll: false, inertia: false }
    }

    /// The swipe action for `dir`, in swipe mode.
    pub fn swipe(&self, dir: SwipeDirection) -> Option<&ButtonAction> {
        let TouchpadMotion::Swipe { up, down, left, right, .. } = self else { return None };
        Some(match dir {
            SwipeDirection::Up => up,
            SwipeDirection::Down => down,
            SwipeDirection::Left => left,
            SwipeDirection::Right => right,
        })
    }

    pub fn swipe_mut(&mut self, dir: SwipeDirection) -> Option<&mut ButtonAction> {
        let TouchpadMotion::Swipe { up, down, left, right, .. } = self else { return None };
        Some(match dir {
            SwipeDirection::Up => up,
            SwipeDirection::Down => down,
            SwipeDirection::Left => left,
            SwipeDirection::Right => right,
        })
    }

    /// The menu it lays out on the pad, in menu mode.
    pub fn menu(&self) -> Option<&String> {
        match self {
            TouchpadMotion::Menu { menu, .. } => Some(menu),
            _ => None,
        }
    }
}

/// Which part of the pad: the whole pad (or its left half, when it is split) or the right half.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TouchSide {
    Left,
    Right,
}

/// A place on the touchpad that holds an action, for editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchSlot {
    Click(TouchSide),
    Swipe(TouchSide, SwipeDirection),
}

/// One half of a split pad: what fingers on it do, and what the click does while a finger is on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TouchRegion {
    pub motion: TouchpadMotion,
    #[serde(default = "disabled")]
    pub click: ButtonAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchpadConfig {
    /// What fingers do on the whole pad, or on its left half when it is split.
    pub motion: TouchpadMotion,
    /// What pressing the pad does, like any button's action (with a split pad, while the
    /// first finger down is on the left half).
    pub click: ButtonAction,
    /// When set, the pad is split down the middle and this is its right half.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<TouchRegion>,
}

impl Default for TouchpadConfig {
    /// Works as a laptop trackpad does, so the pad keeps serving the desktop pointer.
    fn default() -> Self {
        TouchpadConfig { motion: TouchpadMotion::mouse(DEFAULT_TOUCHPAD_SPEED), click: ButtonAction::Mouse(MouseButton::Left), right: None }
    }
}

impl TouchpadConfig {
    /// Left out of saved configs and packs when it is the default, so they stay as they were.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The sides the pad has: one, or two when split.
    pub fn sides(&self) -> &'static [TouchSide] {
        if self.right.is_some() { &[TouchSide::Left, TouchSide::Right] } else { &[TouchSide::Left] }
    }

    /// What fingers do on `side` (the right side of an unsplit pad is the whole pad).
    pub fn motion(&self, side: TouchSide) -> &TouchpadMotion {
        match (side, &self.right) {
            (TouchSide::Right, Some(r)) => &r.motion,
            _ => &self.motion,
        }
    }

    pub fn motion_mut(&mut self, side: TouchSide) -> &mut TouchpadMotion {
        match (side, &mut self.right) {
            (TouchSide::Right, Some(r)) => &mut r.motion,
            _ => &mut self.motion,
        }
    }

    /// What clicking does with a finger on `side`.
    pub fn click(&self, side: TouchSide) -> &ButtonAction {
        match (side, &self.right) {
            (TouchSide::Right, Some(r)) => &r.click,
            _ => &self.click,
        }
    }

    /// The action in `slot`, if the pad has it.
    pub fn slot(&self, slot: TouchSlot) -> Option<&ButtonAction> {
        match slot {
            TouchSlot::Click(side) => Some(self.click(side)),
            TouchSlot::Swipe(side, dir) => self.motion(side).swipe(dir),
        }
    }

    pub fn slot_mut(&mut self, slot: TouchSlot) -> Option<&mut ButtonAction> {
        match slot {
            TouchSlot::Click(TouchSide::Right) => self.right.as_mut().map(|r| &mut r.click),
            TouchSlot::Click(TouchSide::Left) => Some(&mut self.click),
            TouchSlot::Swipe(side, dir) => self.motion_mut(side).swipe_mut(dir),
        }
    }

    /// Every action slot it has: each side's click, and its swipes in swipe mode.
    pub fn slots(&self) -> Vec<TouchSlot> {
        let mut all = Vec::new();
        for &side in self.sides() {
            all.push(TouchSlot::Click(side));
            if matches!(self.motion(side), TouchpadMotion::Swipe { .. }) {
                all.extend(SwipeDirection::ALL.map(|d| TouchSlot::Swipe(side, d)));
            }
        }
        all
    }

    /// Every action it holds: the clicks and the swipes.
    pub fn actions(&self) -> Vec<&ButtonAction> {
        let mut all = vec![&self.click];
        let regions = std::iter::once(&self.motion).chain(self.right.as_ref().map(|r| &r.motion));
        if let Some(r) = &self.right {
            all.push(&r.click);
        }
        for motion in regions {
            all.extend(SwipeDirection::ALL.iter().filter_map(|d| motion.swipe(*d)));
        }
        all
    }

    pub fn actions_mut(&mut self) -> Vec<&mut ButtonAction> {
        let mut all = vec![&mut self.click];
        let mut motions = vec![&mut self.motion];
        if let Some(r) = &mut self.right {
            all.push(&mut r.click);
            motions.push(&mut r.motion);
        }
        for motion in motions {
            if let TouchpadMotion::Swipe { up, down, left, right, .. } = motion {
                all.extend([up, down, left, right]);
            }
        }
        all
    }

    /// The menus it lays out on the pad.
    pub fn menus(&self) -> impl Iterator<Item = &String> {
        std::iter::once(&self.motion).chain(self.right.as_ref().map(|r| &r.motion)).filter_map(TouchpadMotion::menu)
    }

    pub fn menus_mut(&mut self) -> impl Iterator<Item = &mut String> {
        std::iter::once(&mut self.motion).chain(self.right.as_mut().map(|r| &mut r.motion)).filter_map(|m| match m {
            TouchpadMotion::Menu { menu, .. } => Some(menu),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_section_is_the_default_and_round_trips() {
        #[derive(Deserialize)]
        struct Wrap {
            #[serde(default)]
            touchpad: TouchpadConfig,
        }
        let w: Wrap = toml::from_str("").unwrap();
        assert!(w.touchpad.is_default());

        let custom = TouchpadConfig { motion: TouchpadMotion::Off, click: ButtonAction::Disabled, right: None };
        let text = toml::to_string(&custom).unwrap();
        let back: TouchpadConfig = toml::from_str(&text).unwrap();
        assert_eq!(back, custom);
        assert!(!custom.is_default());
    }

    #[test]
    fn motion_reads_off_or_mouse_with_speed() {
        let off: TouchpadConfig = toml::from_str("motion = \"off\"").unwrap();
        assert_eq!(off.motion, TouchpadMotion::Off);
        let mouse: TouchpadConfig = toml::from_str("[motion.mouse]\nspeed = 800.0").unwrap();
        assert_eq!(mouse.motion, TouchpadMotion::mouse(800.0));
        assert_eq!(mouse.click, TouchpadConfig::default().click, "a missing click keeps the default");
        let saved = toml::to_string(&mouse).unwrap();
        for new in ["curve", "tap", "scroll", "inertia", "right"] {
            assert!(!saved.contains(new), "{new} in {saved}");
        }
    }

    #[test]
    fn a_split_pad_with_every_mode_round_trips() {
        let swipe = TouchpadMotion::Swipe {
            up: ButtonAction::Keys(vec!["KEY_W".into()]),
            down: ButtonAction::Disabled,
            left: ButtonAction::Disabled,
            right: ButtonAction::Keys(vec!["KEY_D".into()]),
            distance: 0.3,
        };
        for (left, right) in [
            (TouchpadMotion::Menu { menu: "Build".into(), choose: TouchChoose::Lift, overlay: false }, swipe.clone()),
            (TouchpadMotion::Joystick { stick: Stick::Left, relative: true, radius: 0.2, deadzone: 0.1 }, TouchpadMotion::mouse(900.0)),
            (TouchpadMotion::Region { region: MouseRegion { x: 0.25, ..MouseRegion::FULL } }, TouchpadMotion::Region { region: MouseRegion::default() }),
        ] {
            let cfg = TouchpadConfig { motion: left, click: ButtonAction::Disabled, right: Some(TouchRegion { motion: right, click: ButtonAction::Mouse(MouseButton::Right) }) };
            let back: TouchpadConfig = toml::from_str(&toml::to_string(&cfg).unwrap()).unwrap();
            assert_eq!(back, cfg);
        }
        let menu: TouchpadMotion = toml::from_str::<TouchpadConfig>("[motion.menu]\nmenu = \"M\"").unwrap().motion;
        assert_eq!(menu, TouchpadMotion::Menu { menu: "M".into(), choose: TouchChoose::Click, overlay: true });
        let region: TouchpadMotion = toml::from_str::<TouchpadConfig>("[motion.region]\nwidth = 1.0").unwrap().motion;
        assert_eq!(region, TouchpadMotion::Region { region: MouseRegion { width: 1.0, ..MouseRegion::default() } });
        let rect: TouchpadMotion = toml::from_str::<TouchpadConfig>("[motion.region]\nshape = \"rectangle\"").unwrap().motion;
        assert_eq!(rect, TouchpadMotion::Region { region: MouseRegion { shape: super::super::RegionShape::Rectangle, ..MouseRegion::default() } });
    }

    #[test]
    fn actions_cover_both_clicks_and_every_swipe() {
        let swipe = |k: &str| TouchpadMotion::Swipe {
            up: ButtonAction::Keys(vec![k.into()]),
            down: ButtonAction::Disabled,
            left: ButtonAction::Disabled,
            right: ButtonAction::Disabled,
            distance: 0.2,
        };
        let mut cfg = TouchpadConfig { motion: swipe("KEY_A"), click: ButtonAction::Disabled, right: Some(TouchRegion { motion: swipe("KEY_B"), click: ButtonAction::Disabled }) };
        assert_eq!(cfg.actions().len(), 10);
        assert_eq!(cfg.actions_mut().len(), 10);
        assert_eq!(cfg.slots().len(), 10);
        assert_eq!(cfg.slot(TouchSlot::Swipe(TouchSide::Right, SwipeDirection::Up)), Some(&ButtonAction::Keys(vec!["KEY_B".into()])));
        cfg.right = None;
        assert_eq!(cfg.slots().len(), 5);
        assert_eq!(cfg.motion(TouchSide::Right), &cfg.motion, "an unsplit pad is one side");
    }
}
