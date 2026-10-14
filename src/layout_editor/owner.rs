//! Whose inputs a page edits: the active profile's, or one of its game's modes or layers. A mode
//! or layer leaves what it doesn't set to the profile: a button it doesn't bind, and a stick,
//! trigger, gyro or touchpad it doesn't override.

use std::collections::BTreeMap;

use crate::{
    config::{Analog, Button, ButtonAction, Combo, Config, Game, Gestures, GyroConfig, Layer, Stick, StickConfig, TouchpadConfig, Trigger, TriggerConfig, Zone},
    system_menu::active_profile_mut,
};

/// Which of a game's lists of overrides a page is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    Mode,
    Layer,
}

impl Shift {
    /// "Mode" or "Layer".
    pub fn title(self) -> &'static str {
        match self {
            Shift::Mode => "Mode",
            Shift::Layer => "Layer",
        }
    }

    /// The game's modes or layers.
    pub fn list(self, game: &Game) -> &[Layer] {
        match self {
            Shift::Mode => &game.modes,
            Shift::Layer => &game.layers,
        }
    }

    pub fn list_mut(self, game: &mut Game) -> &mut Vec<Layer> {
        match self {
            Shift::Mode => &mut game.modes,
            Shift::Layer => &mut game.layers,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Profile,
    /// A mode or layer of the game, by its place in the list.
    Shift(Shift, usize),
}

impl Owner {
    /// Whether it is a mode or layer, which may leave things to the profile.
    pub fn is_shift(self) -> bool {
        matches!(self, Owner::Shift(..))
    }
}

/// The game the active profile is in, or General.
pub(super) fn game_mut(config: &mut Config) -> Option<&mut Game> {
    let at = config.active_ref();
    config.game_mut(at.game.as_deref())
}

pub(super) fn layer(config: &Config, shift: Shift, i: usize) -> Option<&Layer> {
    shift.list(config.active_game()).get(i)
}

pub(super) fn layer_mut(config: &mut Config, shift: Shift, i: usize) -> Option<&mut Layer> {
    shift.list_mut(game_mut(config)?).get_mut(i)
}

/// A mode's or layer's name, or the profile's.
pub(super) fn owner_name(config: &Config, owner: Owner) -> String {
    match owner {
        Owner::Profile => config.active().map_or_else(String::new, |p| p.name.clone()),
        Owner::Shift(shift, i) => layer(config, shift, i).map_or_else(|| shift.title().into(), |l| l.name.clone()),
    }
}

/// What `owner` sends for button `b`. A mode or layer that doesn't bind it gives `None`; the
/// profile gives Disabled.
pub(super) fn button(config: &Config, owner: Owner, b: Button) -> Option<ButtonAction> {
    match owner {
        Owner::Profile => Some(config.active().map_or(ButtonAction::Disabled, |p| p.button(b).clone())),
        Owner::Shift(shift, i) => layer(config, shift, i)?.buttons.get(&b).cloned(),
    }
}

/// Sets what `owner` sends for `b`. `None` unbinds a mode's or layer's button; the profile's is
/// Disabled instead.
pub(super) fn set_button(config: &mut Config, owner: Owner, b: Button, action: Option<ButtonAction>) -> bool {
    let Some(buttons) = buttons_mut(config, owner) else { return false };
    let before = buttons.get(&b).cloned();
    match (owner, action) {
        (_, Some(action)) => drop(buttons.insert(b, action)),
        (Owner::Profile, None) => drop(buttons.insert(b, ButtonAction::Disabled)),
        (Owner::Shift(..), None) => drop(buttons.remove(&b)),
    }
    buttons.get(&b).cloned() != before
}

fn buttons_mut(config: &mut Config, owner: Owner) -> Option<&mut BTreeMap<Button, ButtonAction>> {
    match owner {
        Owner::Profile => Some(&mut active_profile_mut(config)?.buttons),
        Owner::Shift(shift, i) => Some(&mut layer_mut(config, shift, i)?.buttons),
    }
}

pub(super) fn gestures(config: &Config, owner: Owner) -> Option<&BTreeMap<Button, Gestures>> {
    match owner {
        Owner::Profile => Some(&config.active()?.gestures),
        Owner::Shift(shift, i) => Some(&layer(config, shift, i)?.gestures),
    }
}

pub(super) fn gestures_mut(config: &mut Config, owner: Owner) -> Option<&mut BTreeMap<Button, Gestures>> {
    match owner {
        Owner::Profile => Some(&mut active_profile_mut(config)?.gestures),
        Owner::Shift(shift, i) => Some(&mut layer_mut(config, shift, i)?.gestures),
    }
}

pub(super) fn combos(config: &Config, owner: Owner) -> &[Combo] {
    let combos = match owner {
        Owner::Profile => config.active().map(|p| p.combos.as_slice()),
        Owner::Shift(shift, i) => layer(config, shift, i).map(|l| l.combos.as_slice()),
    };
    combos.unwrap_or(&[])
}

pub(super) fn combos_mut(config: &mut Config, owner: Owner) -> Option<&mut Vec<Combo>> {
    match owner {
        Owner::Profile => Some(&mut active_profile_mut(config)?.combos),
        Owner::Shift(shift, i) => Some(&mut layer_mut(config, shift, i)?.combos),
    }
}

/// A stick as `owner` sets it: always the profile's; a mode's or layer's only when it overrides it.
pub(super) fn stick(config: &Config, owner: Owner, s: Stick) -> Option<&StickConfig> {
    match owner {
        Owner::Profile => Some(config.active()?.stick(s)),
        Owner::Shift(shift, i) => {
            let l = layer(config, shift, i)?;
            if s == Stick::Left { l.left_stick.as_ref() } else { l.right_stick.as_ref() }
        }
    }
}

pub(super) fn stick_mut(config: &mut Config, owner: Owner, s: Stick) -> Option<&mut StickConfig> {
    match owner {
        Owner::Profile => Some(active_profile_mut(config)?.stick_mut(s)),
        Owner::Shift(shift, i) => {
            let l = layer_mut(config, shift, i)?;
            if s == Stick::Left { l.left_stick.as_mut() } else { l.right_stick.as_mut() }
        }
    }
}

pub(super) fn trigger(config: &Config, owner: Owner, t: Trigger) -> Option<&TriggerConfig> {
    match owner {
        Owner::Profile => {
            let p = config.active()?;
            Some(if t == Trigger::Left { &p.left_trigger } else { &p.right_trigger })
        }
        Owner::Shift(shift, i) => {
            let l = layer(config, shift, i)?;
            if t == Trigger::Left { l.left_trigger.as_ref() } else { l.right_trigger.as_ref() }
        }
    }
}

pub(super) fn trigger_mut(config: &mut Config, owner: Owner, t: Trigger) -> Option<&mut TriggerConfig> {
    match owner {
        Owner::Profile => {
            let p = active_profile_mut(config)?;
            Some(if t == Trigger::Left { &mut p.left_trigger } else { &mut p.right_trigger })
        }
        Owner::Shift(shift, i) => {
            let l = layer_mut(config, shift, i)?;
            if t == Trigger::Left { l.left_trigger.as_mut() } else { l.right_trigger.as_mut() }
        }
    }
}

pub(super) fn zones(config: &Config, owner: Owner, a: Analog) -> &[Zone] {
    let zones = match a {
        Analog::Stick(s) => stick(config, owner, s).map(|c| c.zones.as_slice()),
        Analog::Trigger(t) => trigger(config, owner, t).map(|c| c.zones.as_slice()),
    };
    zones.unwrap_or(&[])
}

pub(super) fn zones_mut(config: &mut Config, owner: Owner, a: Analog) -> Option<&mut Vec<Zone>> {
    match a {
        Analog::Stick(s) => Some(&mut stick_mut(config, owner, s)?.zones),
        Analog::Trigger(t) => Some(&mut trigger_mut(config, owner, t)?.zones),
    }
}

pub(super) fn gyro(config: &Config, owner: Owner) -> Option<&GyroConfig> {
    match owner {
        Owner::Profile => Some(&config.active()?.gyro),
        Owner::Shift(shift, i) => layer(config, shift, i)?.gyro.as_ref(),
    }
}

pub(super) fn gyro_mut(config: &mut Config, owner: Owner) -> Option<&mut GyroConfig> {
    match owner {
        Owner::Profile => Some(&mut active_profile_mut(config)?.gyro),
        Owner::Shift(shift, i) => layer_mut(config, shift, i)?.gyro.as_mut(),
    }
}

pub(super) fn touchpad(config: &Config, owner: Owner) -> Option<&TouchpadConfig> {
    match owner {
        Owner::Profile => Some(&config.active()?.touchpad),
        Owner::Shift(shift, i) => layer(config, shift, i)?.touchpad.as_ref(),
    }
}

pub(super) fn touchpad_mut(config: &mut Config, owner: Owner) -> Option<&mut TouchpadConfig> {
    match owner {
        Owner::Profile => Some(&mut active_profile_mut(config)?.touchpad),
        Owner::Shift(shift, i) => layer_mut(config, shift, i)?.touchpad.as_mut(),
    }
}

/// What a mode or layer can override: a stick, a trigger, the gyro or the touchpad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Stick(Stick),
    Trigger(Trigger),
    Gyro,
    Touchpad,
}

/// Whether a mode or layer overrides `part`. The profile has every part of its own.
pub(super) fn overrides(config: &Config, owner: Owner, part: Part) -> bool {
    match part {
        Part::Stick(s) => stick(config, owner, s).is_some(),
        Part::Trigger(t) => trigger(config, owner, t).is_some(),
        Part::Gyro => gyro(config, owner).is_some(),
        Part::Touchpad => touchpad(config, owner).is_some(),
    }
}

/// Makes a mode or layer override `part`, starting from the profile's, or go back to the
/// profile's. True when that changed something.
pub(super) fn set_override(config: &mut Config, owner: Owner, part: Part, on: bool) -> bool {
    let Owner::Shift(shift, i) = owner else { return false };
    let Some(profile) = config.active().cloned() else { return false };
    let Some(l) = layer_mut(config, shift, i) else { return false };
    fn put<T: Clone>(slot: &mut Option<T>, on: bool, from: &T) -> bool {
        if slot.is_some() == on {
            return false;
        }
        *slot = on.then(|| from.clone());
        true
    }
    match part {
        Part::Stick(Stick::Left) => put(&mut l.left_stick, on, &profile.left_stick),
        Part::Stick(Stick::Right) => put(&mut l.right_stick, on, &profile.right_stick),
        Part::Trigger(Trigger::Left) => put(&mut l.left_trigger, on, &profile.left_trigger),
        Part::Trigger(Trigger::Right) => put(&mut l.right_trigger, on, &profile.right_trigger),
        Part::Gyro => put(&mut l.gyro, on, &profile.gyro),
        Part::Touchpad => put(&mut l.touchpad, on, &profile.touchpad),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_layer() -> Config {
        let mut config = Config::default();
        config.general.layers.push(Layer::new("Hotkeys"));
        config
    }

    #[test]
    fn a_layer_leaves_unbound_buttons_and_parts_to_the_profile() {
        let mut config = with_layer();
        let owner = Owner::Shift(Shift::Layer, 0);
        assert_eq!(button(&config, owner, Button::South), None);
        assert!(set_button(&mut config, owner, Button::South, Some(ButtonAction::Screenshot)));
        assert!(set_button(&mut config, owner, Button::South, None));
        assert!(layer(&config, Shift::Layer, 0).unwrap().buttons.is_empty());
        assert!(stick(&config, owner, Stick::Left).is_none());
        assert!(set_override(&mut config, owner, Part::Stick(Stick::Left), true));
        assert_eq!(stick(&config, owner, Stick::Left), Some(config.active().unwrap().stick(Stick::Left)));
        assert!(!set_override(&mut config, owner, Part::Stick(Stick::Left), true), "already overridden");
        assert!(set_override(&mut config, owner, Part::Stick(Stick::Left), false));
    }

    #[test]
    fn the_profiles_buttons_are_disabled_rather_than_unbound() {
        let mut config = Config::default();
        set_button(&mut config, Owner::Profile, Button::South, None);
        assert_eq!(button(&config, Owner::Profile, Button::South), Some(ButtonAction::Disabled));
        assert!(!set_override(&mut config, Owner::Profile, Part::Gyro, false), "the profile has its own of everything");
    }
}
