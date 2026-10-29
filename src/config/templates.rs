//! The profiles a new setup can start from.

use std::collections::BTreeMap;

use super::*;

impl Profile {
    /// 1:1 virtual gamepad. Guide holds the Guide layer.
    pub fn passthrough(name: &str) -> Self {
        let buttons = Button::ALL
            .iter()
            .map(|&b| {
                let action = if b == Button::Guide {
                    ButtonAction::guide_hold()
                } else {
                    ButtonAction::Gamepad(b)
                };
                (b, action)
            })
            .collect();
        let mut p = Profile {
            name: name.into(),
            buttons,
            left_stick: StickConfig::new(StickAction::Gamepad { stick: Stick::Left, invert_y: false }, 0.05, 1.0),
            right_stick: StickConfig::new(StickAction::Gamepad { stick: Stick::Right, invert_y: false }, 0.05, 1.0),
            left_trigger: TriggerAction::Gamepad(Trigger::Left).into(),
            right_trigger: TriggerAction::Gamepad(Trigger::Right).into(),
            combos: Vec::new(),
            combo_window_ms: default_combo_window_ms(),
            gestures: BTreeMap::new(),
            tap_window_ms: default_tap_window_ms(),
            long_press_ms: default_long_press_ms(),
            gyro: GyroConfig::default(),
            requires: Vec::new(),
            touchpad: TouchpadConfig::default(),
            guide: GuideSettings::default(),
        };
        p.take_guide();
        p
    }

    /// Keyboard-and-mouse action games (shooters, third-person action): WASD movement with
    /// sprint at full push, mouse look, Mouse 1/2 on the triggers.
    pub fn pc_action(name: &str) -> Self {
        use ButtonAction::*;
        let key = |k: &str| Keys(vec![k.into()]);
        let mut left_stick = StickConfig::new(StickAction::wasd().starting_at(0.25), 0.12, 1.0);
        left_stick.key_threshold = 0.25;
        left_stick.zones.push(Zone { min: 0.9, max: 1.0, action: key("KEY_LEFTSHIFT") });
        Profile {
            buttons: BTreeMap::from([
                (Button::South, key("KEY_E")),
                (Button::East, key("KEY_SPACE")),
                (Button::West, key("KEY_R")),
                (Button::North, key("KEY_F")),
                (Button::LeftBumper, key("KEY_Q")),
                (Button::RightBumper, key("KEY_G")),
                (Button::LeftStick, key("KEY_LEFTCTRL")),
                (Button::RightStick, key("KEY_V")),
                (Button::DpadUp, key("KEY_1")),
                (Button::DpadRight, key("KEY_2")),
                (Button::DpadDown, key("KEY_3")),
                (Button::DpadLeft, key("KEY_4")),
                (Button::Start, key("KEY_ESC")),
                (Button::Select, key("KEY_TAB")),
                (Button::Guide, ButtonAction::guide_hold()),
            ]),
            left_stick,
            right_stick: StickConfig::new(StickAction::mouse(1600.0), 0.1, 2.0),
            left_trigger: TriggerAction::Button { action: Multi(vec![Mouse(MouseButton::Right), GyroHold]), threshold: 0.3 }.into(),
            right_trigger: TriggerAction::Button { action: Mouse(MouseButton::Left), threshold: 0.3 }.into(),
            // Gyro aiming while aiming down sights (LT), on pads that have a gyro.
            gyro: GyroConfig { enabled: true, starts_active: false, mode: GyroMode::Mouse { sensitivity: 15.0 }, ..GyroConfig::default() },
            ..Profile::passthrough(name)
        }
    }

    /// Mouse-driven strategy games: pointer on the left stick, arrow-key camera pan on the
    /// right, zoom on the stick clicks, held Ctrl/Shift modifiers on the bumpers and control
    /// groups on the D-pad.
    pub fn strategy(name: &str) -> Self {
        use ButtonAction::*;
        let key = |k: &str| Keys(vec![k.into()]);
        let mut right_stick = StickConfig::new(StickAction::arrows().starting_at(0.35), 0.15, 1.0);
        right_stick.key_threshold = 0.35;
        Profile {
            buttons: BTreeMap::from([
                (Button::South, Mouse(MouseButton::Left)),
                (Button::East, Mouse(MouseButton::Right)),
                (Button::West, Mouse(MouseButton::Middle)),
                (Button::North, key("KEY_SPACE")),
                (Button::LeftBumper, key("KEY_LEFTCTRL")),
                (Button::RightBumper, key("KEY_LEFTSHIFT")),
                // Camera zoom, which strategy games put on the wheel.
                (Button::LeftStick, Wheel(WheelDirection::Up)),
                (Button::RightStick, Wheel(WheelDirection::Down)),
                (Button::DpadUp, key("KEY_1")),
                (Button::DpadRight, key("KEY_2")),
                (Button::DpadDown, key("KEY_3")),
                (Button::DpadLeft, key("KEY_4")),
                (Button::Start, key("KEY_ESC")),
                (Button::Select, key("KEY_TAB")),
                (Button::Guide, ButtonAction::guide_hold()),
            ]),
            left_stick: StickConfig::new(StickAction::mouse(1400.0), 0.12, 2.2),
            right_stick,
            left_trigger: TriggerAction::Button { action: key("KEY_LEFTALT"), threshold: 0.4 }.into(),
            // Hold and move the pointer to drag a selection box.
            right_trigger: TriggerAction::Button { action: Mouse(MouseButton::Left), threshold: 0.4 }.into(),
            ..Profile::passthrough(name)
        }
    }

    /// Makes the gyro run all the time (not only while aiming down sights), with LB as a clutch
    /// beside its own action.
    fn always_on_with_clutch(&mut self) {
        self.left_trigger = TriggerAction::Button { action: ButtonAction::Mouse(MouseButton::Right), threshold: 0.3 }.into();
        let lb = self.button(Button::LeftBumper).clone();
        self.set_button(Button::LeftBumper, ButtonAction::Multi(vec![lb, ButtonAction::GyroClutch]));
    }

    /// PC action with gyro aiming: the gyro turns the view all the time, so it needs a gyro. LB
    /// is a clutch: hold it to reposition the controller without turning.
    pub fn gyro_aim(name: &str) -> Self {
        let mut p = Profile::pc_action(name);
        p.requires = vec![Feature::Gyro];
        p.gyro = GyroConfig {
            enabled: true,
            mode: GyroMode::Mouse { sensitivity: 15.0 },
            ..GyroConfig::default()
        };
        p.always_on_with_clutch();
        p
    }

    /// PC action with a flick stick: pushing the right stick turns the view to that direction,
    /// and gyro aims up and down and fine-tunes, so it needs a gyro. LB is the same clutch.
    pub fn flick_stick(name: &str) -> Self {
        let mut p = Profile::pc_action(name);
        p.requires = vec![Feature::Gyro];
        p.right_stick = StickConfig::new(StickAction::flick(), 0.1, 2.0);
        p.gyro = GyroConfig {
            enabled: true,
            mode: GyroMode::Mouse { sensitivity: 22.0 },
            ..GyroConfig::default()
        };
        p.always_on_with_clutch();
        p
    }

    /// Keyboard-only retro games, indie platformers and emulators: arrows plus Z/X/C/V.
    pub fn platformer(name: &str) -> Self {
        use ButtonAction::*;
        let key = |k: &str| Keys(vec![k.into()]);
        let mut left_stick = StickConfig::new(StickAction::arrows().starting_at(0.4), 0.15, 1.0);
        left_stick.key_threshold = 0.4;
        Profile {
            buttons: BTreeMap::from([
                (Button::South, key("KEY_Z")),
                (Button::East, key("KEY_X")),
                (Button::West, key("KEY_C")),
                (Button::North, key("KEY_V")),
                (Button::LeftBumper, key("KEY_A")),
                (Button::RightBumper, key("KEY_S")),
                (Button::LeftStick, Disabled),
                (Button::RightStick, Disabled),
                (Button::DpadUp, key("KEY_UP")),
                (Button::DpadDown, key("KEY_DOWN")),
                (Button::DpadLeft, key("KEY_LEFT")),
                (Button::DpadRight, key("KEY_RIGHT")),
                (Button::Start, key("KEY_ENTER")),
                (Button::Select, key("KEY_ESC")),
                (Button::Guide, ButtonAction::guide_hold()),
            ]),
            left_stick,
            right_stick: StickConfig::new(StickAction::Disabled, 0.15, 1.0),
            left_trigger: TriggerAction::Button { action: key("KEY_LEFTSHIFT"), threshold: 0.5 }.into(),
            right_trigger: TriggerAction::Button { action: key("KEY_SPACE"), threshold: 0.5 }.into(),
            ..Profile::passthrough(name)
        }
    }

    /// Desktop navigation: left stick = mouse, right stick = scroll.
    pub fn desktop(name: &str) -> Self {
        use ButtonAction::*;
        let key = |k: &str| Keys(vec![k.into()]);
        let buttons = BTreeMap::from([
            (Button::South, Mouse(MouseButton::Left)),
            (Button::East, Mouse(MouseButton::Right)),
            (Button::North, key("KEY_ESC")),
            (Button::West, Mouse(MouseButton::Middle)),
            (Button::LeftBumper, Mouse(MouseButton::Back)),
            (Button::RightBumper, Mouse(MouseButton::Forward)),
            (Button::Select, Keys(vec!["KEY_LEFTMETA".into()])),
            (Button::Start, key("KEY_ENTER")),
            (Button::Guide, ButtonAction::guide_hold()),
            (Button::LeftStick, Disabled),
            (Button::RightStick, Disabled),
            (Button::DpadUp, key("KEY_UP")),
            (Button::DpadDown, key("KEY_DOWN")),
            (Button::DpadLeft, key("KEY_LEFT")),
            (Button::DpadRight, key("KEY_RIGHT")),
        ]);
        let mut p = Profile {
            name: name.into(),
            buttons,
            left_stick: StickConfig::new(StickAction::mouse(1200.0), 0.12, 2.0),
            right_stick: StickConfig::new(StickAction::Scroll { speed: 15.0, invert_y: false }, 0.15, 2.0),
            left_trigger: TriggerAction::Button {
                action: Keys(vec!["KEY_LEFTSHIFT".into()]),
                threshold: 0.5,
            }
            .into(),
            right_trigger: TriggerAction::Button {
                action: Mouse(MouseButton::Left),
                threshold: 0.5,
            }
            .into(),
            combos: vec![Combo {
                buttons: vec![Button::LeftBumper, Button::RightBumper],
                action: Keys(vec!["KEY_LEFTALT".into(), "KEY_TAB".into()]),
                window_ms: None,
            }],
            combo_window_ms: default_combo_window_ms(),
            gestures: BTreeMap::new(),
            tap_window_ms: default_tap_window_ms(),
            long_press_ms: default_long_press_ms(),
            gyro: GyroConfig::default(),
            requires: Vec::new(),
            touchpad: TouchpadConfig::default(),
            guide: GuideSettings::default(),
        };
        p.take_guide();
        p
    }
}

/// What a new profile starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Gamepad,
    Desktop,
    Action,
    GyroAim,
    FlickStick,
    Strategy,
    Platformer,
    Duplicate,
}

impl Template {
    /// A new profile from this template (Duplicate makes a passthrough one; copy instead).
    pub fn make(self, name: &str) -> Profile {
        match self {
            Template::Gamepad | Template::Duplicate => Profile::passthrough(name),
            Template::Desktop => Profile::desktop(name),
            Template::Action => Profile::pc_action(name),
            Template::GyroAim => Profile::gyro_aim(name),
            Template::FlickStick => Profile::flick_stick(name),
            Template::Strategy => Profile::strategy(name),
            Template::Platformer => Profile::platformer(name),
        }
    }

    /// Offered in the "+ New profile" list (Duplicate has its own button).
    pub const NEW: [Template; 7] = [
        Template::Gamepad,
        Template::Action,
        Template::GyroAim,
        Template::FlickStick,
        Template::Strategy,
        Template::Platformer,
        Template::Desktop,
    ];

    pub fn base_name(self) -> &'static str {
        match self {
            Template::Gamepad => "Gamepad",
            Template::Desktop => "Desktop",
            Template::Action => "PC Action",
            Template::GyroAim => "Gyro Aim",
            Template::FlickStick => "Flick Stick",
            Template::Strategy => "Strategy",
            Template::Platformer => "Platformer",
            Template::Duplicate => "Copy",
        }
    }
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Template::Gamepad => "Gamepad pass-through",
            Template::Desktop => "Desktop navigation",
            Template::Action => "PC action (WASD + mouse look)",
            Template::GyroAim => "PC action with gyro aiming (needs a gyro)",
            Template::FlickStick => "PC action with a flick stick (needs a gyro)",
            Template::Strategy => "Strategy (mouse pointer + hotkeys)",
            Template::Platformer => "Retro / platformer (arrows + Z/X/C)",
            Template::Duplicate => "Copy of this profile",
        })
    }
}

/// `base`, or `base 2`, `base 3`, … whichever isn't taken.
pub fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..)
        .map(|i| if i == 1 { base.to_string() } else { format!("{base} {i}") })
        .find(|n| !taken(n))
        .unwrap()
}
