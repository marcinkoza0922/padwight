//! Layers and the built-in modes: overrides put on top of a profile while they're on.

use std::{collections::BTreeMap, sync::LazyLock};

use serde::{Deserialize, Serialize};

use super::*;

/// What a layer shows on screen while it's active.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Indicator {
    /// A small label with the layer's name.
    #[default]
    Name,
    /// One of the game's info overlays.
    Info(String),
    /// A cheat sheet built from the layer's own bindings, with button glyphs.
    Bindings,
    Off,
}

/// A set of overrides on top of whichever of its game's profiles is active, while an input
/// holds it (or a toggle keeps it on). Whatever it doesn't set falls through to the layers
/// below it and then the profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    #[serde(default)]
    pub indicator: Indicator,
    /// How the name label looks, for `Indicator::Name`.
    #[serde(default = "OverlayStyle::indicator")]
    pub indicator_style: OverlayStyle,
    /// The heading of the generated bindings sheet (may hold `{tokens}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicator_title: Option<String>,
    /// How long the layer must be on before its indicator appears, so a quick
    /// tap of the input holding it doesn't flash them.
    #[serde(default, skip_serializing_if = "is_zero_ms")]
    pub indicator_delay_ms: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub buttons: BTreeMap<Button, ButtonAction>,
    /// Per button, its gestures replace the profile's (empty: none while the layer is on).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gestures: BTreeMap<Button, Gestures>,
    /// Added to the profile's combos; one with the same buttons replaces the profile's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combos: Vec<Combo>,
    /// The profile's combos (by their buttons) switched off while the layer is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_combos: Vec<Vec<Button>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_stick: Option<StickConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_stick: Option<StickConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_trigger: Option<TriggerConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_trigger: Option<TriggerConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gyro: Option<GyroConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub touchpad: Option<TouchpadConfig>,
    /// While on, buttons the layer doesn't set do nothing (instead of keeping the profile's
    /// action), paddles and stick directions included, and so do the profile's combos, except
    /// ones that hold a layer or switch modes themselves.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub swallow_unbound: bool,
}

/// The name of the built-in mode the Guide button turns on while it's held or its overlay is up.
pub const GUIDE_MODE: &str = "Guide";

/// The name of the built-in mouse mode (see the engine).
pub const MOUSE_MODE: &str = "Mouse mode";

static GUIDE_BUILT_IN: LazyLock<Layer> = LazyLock::new(Layer::guide);
static MOUSE_BUILT_IN: LazyLock<Layer> = LazyLock::new(Layer::mouse);

/// The built-in mode called `name`, if there is one. These are fixed: every game has them, and
/// they're never stored in a game, so they can't be edited or deleted. While one is on, it stands
/// in for the game's mode, and held layers don't apply.
pub fn built_in_mode(name: &str) -> Option<&'static Layer> {
    match name {
        GUIDE_MODE => Some(&GUIDE_BUILT_IN),
        MOUSE_MODE => Some(&MOUSE_BUILT_IN),
        _ => None,
    }
}

/// Every built-in mode.
pub fn built_in_modes() -> [&'static Layer; 2] {
    [&GUIDE_BUILT_IN, &MOUSE_BUILT_IN]
}

impl Layer {
    /// The Guide mode: the system shortcuts on the Guide chords, and everything else
    /// swallowed. Mouse mode (see [`Layer::mouse`]) has the mouse and the D-pad keys.
    pub fn guide() -> Self {
        Self::guide_with(&Shortcuts::default())
    }

    /// The Guide mode without its shortcuts, which [`Layer::guide_with`] puts on.
    pub(super) fn guide_base() -> Self {
        let mut layer = Self::new(GUIDE_MODE);
        layer.indicator = Indicator::Bindings;
        layer.indicator_style = OverlayStyle::sheet();
        layer.indicator_title = Some("Guide shortcuts".into());
        layer.indicator_delay_ms = 100;
        layer.swallow_unbound = true;
        layer
    }

    /// Mouse mode: the controller acts as a mouse and a keyboard for clicking, typing and
    /// scrolling. Only the inputs set here do anything, so the game underneath gets nothing.
    pub fn mouse() -> Self {
        Self::mouse_with(&Shortcuts::default())
    }

    /// Mouse mode without its keys, which [`Layer::mouse_with`] puts on.
    pub(super) fn mouse_base() -> Self {
        use ButtonAction::*;
        let mut layer = Self::new(MOUSE_MODE);
        layer.indicator = Indicator::Bindings;
        layer.indicator_style = OverlayStyle::sheet();
        layer.indicator_title = Some("Mouse mode: press {guide} to exit".into());
        layer.swallow_unbound = true;
        layer.left_trigger = Some(TriggerAction::Button { action: Mouse(MouseButton::Right), threshold: 0.3 }.into());
        layer.right_trigger = Some(TriggerAction::Button { action: Mouse(MouseButton::Left), threshold: 0.3 }.into());
        layer.left_stick = Some(StickConfig::new(StickAction::Scroll { speed: 15.0, invert_y: false }, 0.15, 2.0));
        layer.right_stick = Some(StickConfig::new(StickAction::mouse(1600.0), 0.1, 2.0));
        layer
    }
}

/// Drops buttons a combo lists twice (`[A, A]` would fire on one press of A), in profiles and
/// layers, and in the combos layers switch off. Only a hand-edited file or a pack can have them.
pub fn tidy_combos(profiles: &mut [Profile], layers: &mut [Layer]) {
    let once = |buttons: &mut Vec<Button>| {
        let mut seen = Vec::new();
        buttons.retain(|b| {
            let new = !seen.contains(b);
            seen.push(*b);
            new
        });
    };
    let combos = profiles.iter_mut().flat_map(|p| &mut p.combos).chain(layers.iter_mut().flat_map(|l| &mut l.combos));
    combos.for_each(|c| once(&mut c.buttons));
    layers.iter_mut().flat_map(|l| &mut l.disabled_combos).for_each(once);
}

/// A combo's buttons in a canonical order, to compare combos.
pub fn combo_key(buttons: &[Button]) -> Vec<Button> {
    let mut key = buttons.to_vec();
    key.sort();
    key.dedup();
    key
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Layer {
            name: name.into(),
            indicator: Indicator::Name,
            indicator_style: OverlayStyle::indicator(),
            indicator_title: None,
            indicator_delay_ms: 0,
            buttons: BTreeMap::new(),
            gestures: BTreeMap::new(),
            combos: Vec::new(),
            disabled_combos: Vec::new(),
            left_stick: None,
            right_stick: None,
            left_trigger: None,
            right_trigger: None,
            gyro: None,
            touchpad: None,
            swallow_unbound: false,
        }
    }

    /// Puts this layer's overrides on top of `p`.
    pub fn apply(&self, p: &mut Profile) {
        if self.swallow_unbound {
            // Buttons that hold a layer or switch modes stay, so there's always a way back.
            let shifts = |a: &ButtonAction| {
                let mut found = false;
                a.walk(&mut |a| found |= matches!(a, ButtonAction::Layer(_) | ButtonAction::Mode(_)));
                found
            };
            // Every input that acts as a button: the paddles and the sticks' directions too.
            for b in Button::EVERY.into_iter().chain(Button::PADDLES) {
                let gestures = p.gestures.get(&b).is_some_and(|g| GestureKind::ALL.into_iter().any(|k| g.get(k).is_some_and(shifts)));
                let holds = shifts(p.button(b)) || gestures;
                if !holds && !self.buttons.contains_key(&b) {
                    p.buttons.insert(b, ButtonAction::Disabled);
                    p.gestures.remove(&b);
                }
            }
            p.combos.retain(|c| shifts(&c.action));
        }
        for (b, a) in &self.buttons {
            p.buttons.insert(*b, a.clone());
        }
        for (b, g) in &self.gestures {
            if g.is_empty() {
                p.gestures.remove(b);
            } else {
                p.gestures.insert(*b, g.clone());
            }
        }
        let replaced = |c: &Combo| {
            let key = combo_key(&c.buttons);
            self.disabled_combos.iter().any(|d| combo_key(d) == key) || self.combos.iter().any(|l| combo_key(&l.buttons) == key)
        };
        p.combos.retain(|c| !replaced(c));
        p.combos.extend(self.combos.iter().cloned());
        let overrides = [(&self.left_stick, &mut p.left_stick), (&self.right_stick, &mut p.right_stick)];
        for (layer, profile) in overrides {
            if let Some(s) = layer {
                *profile = s.clone();
            }
        }
        for (layer, profile) in [(&self.left_trigger, &mut p.left_trigger), (&self.right_trigger, &mut p.right_trigger)] {
            if let Some(t) = layer {
                *profile = t.clone();
            }
        }
        if let Some(g) = &self.gyro {
            p.gyro = g.clone();
        }
        if let Some(t) = &self.touchpad {
            p.touchpad = t.clone();
        }
    }

    /// Every action it sets: buttons, gestures, combos, triggers and zones.
    pub fn actions(&self) -> Vec<&ButtonAction> {
        let mut all: Vec<&ButtonAction> = self.buttons.values().collect();
        all.extend(self.gestures.values().flat_map(|g| GestureKind::ALL.into_iter().filter_map(|k| g.get(k))));
        all.extend(self.combos.iter().map(|c| &c.action));
        for t in [&self.left_trigger, &self.right_trigger].into_iter().flatten() {
            if let TriggerAction::Button { action, .. } = &t.action {
                all.push(action);
            }
            all.extend(t.zones.iter().map(|z| &z.action));
        }
        for s in [&self.left_stick, &self.right_stick].into_iter().flatten() {
            all.extend(s.zones.iter().map(|z| &z.action));
            all.extend(s.action.ring_actions());
        }
        all.extend(self.touchpad.iter().flat_map(TouchpadConfig::actions));
        all
    }

    pub fn actions_mut(&mut self) -> Vec<&mut ButtonAction> {
        let mut all: Vec<&mut ButtonAction> = self.buttons.values_mut().collect();
        for g in self.gestures.values_mut() {
            all.extend([&mut g.double_tap, &mut g.triple_tap, &mut g.long_press].into_iter().filter_map(|s| s.as_mut()));
        }
        all.extend(self.combos.iter_mut().map(|c| &mut c.action));
        for t in [&mut self.left_trigger, &mut self.right_trigger].into_iter().flatten() {
            if let TriggerAction::Button { action, .. } = &mut t.action {
                all.push(action);
            }
            all.extend(t.zones.iter_mut().map(|z| &mut z.action));
        }
        for s in [&mut self.left_stick, &mut self.right_stick].into_iter().flatten() {
            all.extend(s.zones.iter_mut().map(|z| &mut z.action));
            all.extend(s.action.ring_actions_mut());
        }
        all.extend(self.touchpad.iter_mut().flat_map(TouchpadConfig::actions_mut));
        all
    }

    /// How many things it overrides, for summaries.
    pub fn overrides(&self) -> usize {
        self.buttons.len()
            + self.gestures.len()
            + self.combos.len()
            + self.disabled_combos.len()
            + [self.left_stick.is_some(), self.right_stick.is_some(), self.left_trigger.is_some(), self.right_trigger.is_some(), self.gyro.is_some(), self.touchpad.is_some()]
                .into_iter()
                .filter(|o| *o)
                .count()
    }
}
