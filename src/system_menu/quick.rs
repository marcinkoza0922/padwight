//! Quick Settings: a few values of the active profile, and of the overlays and rumble, changed
//! with the D-pad over the game. Which rows show depends on what the profile uses.

use crate::config::{Config, GyroConfig, GyroMode, Layer, Profile, ProfileRef, Stick, StickAction, StickConfig};

use super::{Outcome, active_profile_mut};

/// One step multiplies or divides a speed or sensitivity by this...
const STEP: f32 = 1.1;
/// ...or by this with RB held.
const BIG_STEP: f32 = 1.5;
/// Stick speed in pixels per second, and gyro sensitivity in pixels per degree.
const SPEED_RANGE: (f32, f32) = (10.0, 10_000.0);
const SENSITIVITY_RANGE: (f32, f32) = (0.1, 1_000.0);
/// A dead zone's steps (small and big), and the most it can be.
const DEADZONE_STEPS: (f32, f32) = (0.01, 0.05);
const DEADZONE_MAX: f32 = 0.5;
/// Rumble strength's steps, as shares of full.
const RUMBLE_STEPS: (f32, f32) = (0.1, 0.5);
/// The overlays' size steps, and its range.
const SIZE_STEPS: (f32, f32) = (0.1, 0.25);
const SIZE_RANGE: (f32, f32) = (0.5, 2.0);

/// A layer or mode of the active game (by its place in the game's list) that sets a speed of its
/// own, which the profile's row doesn't change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Layer(usize),
    Mode(usize),
}

impl Scope {
    fn get(self, config: &Config) -> Option<&Layer> {
        let game = config.active_game();
        match self {
            Scope::Layer(i) => game.layers.get(i),
            Scope::Mode(i) => game.modes.get(i),
        }
    }

    fn get_mut(self, config: &mut Config) -> Option<&mut Layer> {
        let at = config.active_ref();
        let game = config.game_mut(at.game.as_deref())?;
        match self {
            Scope::Layer(i) => game.layers.get_mut(i),
            Scope::Mode(i) => game.modes.get_mut(i),
        }
    }

    /// "Aim layer" or "Driving mode".
    fn name(self, config: &Config) -> String {
        let kind = if matches!(self, Scope::Layer(_)) { "layer" } else { "mode" };
        format!("{} {kind}", self.get(config).map_or("", |l| l.name.as_str()))
    }
}

/// One line of Quick Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Row {
    /// Speed of a stick that moves the mouse.
    StickSpeed(Stick),
    GyroSensitivity,
    /// The speed of a stick that moves the mouse in a layer or mode, which sets that stick itself,
    /// so the profile's speed doesn't reach it.
    ScopedStickSpeed(Scope, Stick),
    /// The same for a layer or mode's gyro sensitivity.
    ScopedGyroSensitivity(Scope),
    /// Gyro off, or back on as it was when the menu opened.
    Gyro,
    /// Flips every Y inversion in the profile at once.
    InvertY,
    /// A stick's dead zone.
    Deadzone(Stick),
    /// How strongly the controller rumbles, for every game.
    Rumble,
    /// The size of the keyboard, numpad, media controls and menus.
    OverlaySize,
    /// Button labels swapped to the Nintendo layout, for the active game.
    NintendoLayout,
    /// Next or previous profile of the active game.
    Profile,
    /// Stops remapping altogether: the controller goes back to the game as it is.
    Remapping,
}

/// The rows Quick Settings shows for the active profile. `saved` is the config as the menu
/// opened, whose gyro mode the Gyro row turns back on.
pub(super) fn rows(config: &Config, saved: &Config) -> Vec<Row> {
    let mut rows = Vec::new();
    if let Some(profile) = config.active() {
        for (stick, cfg) in [(Stick::Left, &profile.left_stick), (Stick::Right, &profile.right_stick)] {
            if matches!(cfg.action, StickAction::Mouse { .. }) {
                rows.push(Row::StickSpeed(stick));
            }
        }
        if matches!(profile.gyro.mode, GyroMode::Mouse { .. }) {
            rows.push(Row::GyroSensitivity);
        }
        let had_gyro = |c: &Config| c.active().is_some_and(|p| !matches!(p.gyro.mode, GyroMode::Off));
        if had_gyro(config) || had_gyro(saved) {
            rows.push(Row::Gyro);
        }
        rows.extend(scoped_rows(config));
        rows.push(Row::InvertY);
        for (stick, cfg) in [(Stick::Left, &profile.left_stick), (Stick::Right, &profile.right_stick)] {
            if !matches!(cfg.action, StickAction::Disabled) {
                rows.push(Row::Deadzone(stick));
            }
        }
    }
    rows.extend([Row::Rumble, Row::OverlaySize, Row::NintendoLayout]);
    if config.active_game().profiles.len() > 1 {
        rows.push(Row::Profile);
    }
    rows.push(Row::Remapping);
    rows
}

/// The rows for the speeds the active game's layers and modes set themselves, so a slower aim
/// layer can be tuned from here without changing the base sensitivity.
fn scoped_rows(config: &Config) -> Vec<Row> {
    let game = config.active_game();
    let mut rows = Vec::new();
    let scopes = (0..game.layers.len()).map(Scope::Layer).chain((0..game.modes.len()).map(Scope::Mode));
    for scope in scopes {
        let Some(layer) = scope.get(config) else { continue };
        for (stick, cfg) in [(Stick::Left, &layer.left_stick), (Stick::Right, &layer.right_stick)] {
            if cfg.as_ref().is_some_and(|c| matches!(c.action, StickAction::Mouse { .. })) {
                rows.push(Row::ScopedStickSpeed(scope, stick));
            }
        }
        if layer.gyro.as_ref().is_some_and(|g| matches!(g.mode, GyroMode::Mouse { .. })) {
            rows.push(Row::ScopedGyroSensitivity(scope));
        }
    }
    rows
}

fn percent(share: f32) -> String {
    format!("{:.0}%", share * 100.0)
}

fn side(stick: Stick) -> &'static str {
    if stick == Stick::Left { "Left" } else { "Right" }
}

/// A row's label in `config`. A number changed since the menu opened says what it was in
/// `saved`, so it's easy to get back to.
pub(super) fn label(row: Row, config: &Config, saved: &Config) -> String {
    let was = |value: &dyn Fn(&Config) -> Option<String>| {
        let now = value(config).unwrap_or_default();
        match value(saved) {
            Some(then) if then != now => format!("{now} (was {then})"),
            _ => now,
        }
    };
    let on = |yes: bool| if yes { "on" } else { "off" };
    match row {
        Row::StickSpeed(stick) => format!(
            "{} stick speed: {}",
            side(stick),
            was(&|c| c.active().and_then(|p| match &stick_config(p, stick).action {
                StickAction::Mouse { speed, .. } => Some(format!("{speed:.0} px/s")),
                _ => None,
            }))
        ),
        Row::GyroSensitivity => format!("Gyro sensitivity: {}", was(&|c| c.active().and_then(|p| p.gyro.sensitivity_label()))),
        Row::ScopedStickSpeed(scope, stick) => format!(
            "{} stick speed, {}: {}",
            side(stick),
            scope.name(config),
            was(&|c| scope.get(c).and_then(|l| match &(if stick == Stick::Left { &l.left_stick } else { &l.right_stick }).as_ref()?.action {
                StickAction::Mouse { speed, .. } => Some(format!("{speed:.0} px/s")),
                _ => None,
            }))
        ),
        Row::ScopedGyroSensitivity(scope) => {
            format!("Gyro sensitivity, {}: {}", scope.name(config), was(&|c| scope.get(c).and_then(|l| l.gyro.as_ref()?.sensitivity_label())))
        }
        Row::Gyro => format!("Gyro: {}", on(config.active().is_some_and(|p| !matches!(p.gyro.mode, GyroMode::Off)))),
        Row::InvertY => format!("Invert Y: {}", on(config.active().is_some_and(inverted_y))),
        Row::Deadzone(stick) => format!("{} stick dead zone: {}", side(stick), was(&|c| c.active().map(|p| percent(stick_config(p, stick).deadzone)))),
        Row::Rumble => format!("Rumble: {}", was(&|c| Some(percent(c.rumble_strength)))),
        Row::OverlaySize => format!("Overlay size: {}", was(&|c| Some(percent(c.active_menu_style().scale)))),
        Row::NintendoLayout => format!("Nintendo button layout: {}", on(config.active_nintendo_layout())),
        Row::Profile => format!("Profile: {}", config.active_ref().profile),
        Row::Remapping => "Turn remapping off".into(),
    }
}

fn stick_config(profile: &Profile, stick: Stick) -> &StickConfig {
    if stick == Stick::Left { &profile.left_stick } else { &profile.right_stick }
}

/// Whether any of the profile's Y inversions is on.
pub(super) fn inverted_y(profile: &Profile) -> bool {
    [&profile.left_stick, &profile.right_stick].into_iter().any(|s| match s.action {
        StickAction::Gamepad { invert_y, .. } | StickAction::Mouse { invert_y, .. } | StickAction::Scroll { invert_y, .. } => invert_y,
        _ => false,
    }) || profile.gyro.invert_y
}

/// Every Y inversion in the profile, so one switch can set them all.
fn y_flags(profile: &mut Profile) -> Vec<&mut bool> {
    let mut flags = Vec::new();
    for stick in [&mut profile.left_stick, &mut profile.right_stick] {
        match &mut stick.action {
            StickAction::Gamepad { invert_y, .. } | StickAction::Mouse { invert_y, .. } | StickAction::Scroll { invert_y, .. } => {
                flags.push(invert_y);
            }
            _ => {}
        }
    }
    flags.push(&mut profile.gyro.invert_y);
    flags
}

/// Changes a row's value by one step (`dir` is -1 or 1; `big` with RB held), or moves to the
/// next profile, or turns remapping off. `saved` is the config as the menu opened.
pub(super) fn adjust(row: Row, dir: i32, big: bool, config: &mut Config, saved: &Config) -> Option<Outcome> {
    let pick = |(small, large): (f32, f32)| if big { large } else { small } * dir as f32;
    match row {
        Row::Profile => return next_profile(dir, config).map(Outcome::Switch),
        Row::Remapping => return Some(Outcome::RemappingOff),
        Row::NintendoLayout => {
            // A game's own choice, so it's set on the game the menu is over, not on the profile.
            let on = !config.active_nintendo_layout();
            let at = config.active_ref();
            config.game_mut(at.game.as_deref())?.nintendo_layout = Some(on);
        }
        Row::Rumble => config.rumble_strength = (config.rumble_strength + pick(RUMBLE_STEPS)).clamp(0.0, 1.0),
        Row::OverlaySize => {
            let size = (config.active_menu_style().scale + pick(SIZE_STEPS)).clamp(SIZE_RANGE.0, SIZE_RANGE.1);
            set_panel_size(config, size);
        }
        Row::Gyro => {
            let restored = saved.active().map(|p| p.gyro.mode.clone()).filter(|m| !matches!(m, GyroMode::Off));
            let gyro = &mut active_profile_mut(config)?.gyro;
            gyro.mode = match gyro.mode {
                GyroMode::Off => restored?,
                _ => GyroMode::Off,
            };
        }
        Row::ScopedStickSpeed(scope, stick) => {
            let cfg = if stick == Stick::Left { &mut scope.get_mut(config)?.left_stick } else { &mut scope.get_mut(config)?.right_stick };
            scale_speed(cfg.as_mut()?, dir, if big { BIG_STEP } else { STEP })?;
        }
        Row::ScopedGyroSensitivity(scope) => scale_sensitivity(scope.get_mut(config)?.gyro.as_mut()?, dir, if big { BIG_STEP } else { STEP })?,
        _ => adjust_profile(row, dir, if big { BIG_STEP } else { STEP }, pick(DEADZONE_STEPS), active_profile_mut(config)?)?,
    }
    Some(Outcome::Changed)
}

/// Changes one of the profile's own rows: a speed or sensitivity by `step`, a dead zone by
/// `deadzone_step`, or flips Invert Y.
fn adjust_profile(row: Row, dir: i32, step: f32, deadzone_step: f32, profile: &mut Profile) -> Option<()> {
    match row {
        Row::StickSpeed(stick) => {
            let cfg = if stick == Stick::Left { &mut profile.left_stick } else { &mut profile.right_stick };
            scale_speed(cfg, dir, step)?;
        }
        Row::GyroSensitivity => scale_sensitivity(&mut profile.gyro, dir, step)?,
        Row::InvertY => {
            let flags = y_flags(profile);
            let on = flags.iter().any(|f| **f);
            for flag in flags {
                *flag = !on;
            }
        }
        Row::Deadzone(stick) => {
            let cfg = if stick == Stick::Left { &mut profile.left_stick } else { &mut profile.right_stick };
            cfg.deadzone = ((cfg.deadzone + deadzone_step) * 100.0).round().clamp(0.0, DEADZONE_MAX * 100.0) / 100.0;
        }
        _ => return None,
    }
    Some(())
}

/// Sets the size of the keyboard, numpad, media controls and menus: each the active game's own
/// style when it has one, else the global one.
fn set_panel_size(config: &mut Config, size: f32) {
    let at = config.active_ref();
    let Config { keyboard_style, numpad_style, media_style, menu_style, general, games, .. } = config;
    let game = match at.game.as_deref() {
        Some(name) => games.iter_mut().find(|g| g.name == name),
        None => Some(general),
    };
    let (keyboard, numpad, media, menu) = match game {
        Some(g) => (g.keyboard_style.as_mut(), g.numpad_style.as_mut(), g.media_style.as_mut(), g.menu_style.as_mut()),
        None => (None, None, None, None),
    };
    for style in [keyboard.unwrap_or(keyboard_style), numpad.unwrap_or(numpad_style), media.unwrap_or(media_style), menu.unwrap_or(menu_style)] {
        style.scale = size;
    }
}

/// Scales the speed of a stick that moves the mouse.
fn scale_speed(cfg: &mut StickConfig, dir: i32, step: f32) -> Option<()> {
    let StickAction::Mouse { speed, .. } = &mut cfg.action else { return None };
    *speed = scale(*speed, dir, step, SPEED_RANGE);
    Some(())
}

/// Scales the sensitivity of a gyro that moves the mouse.
fn scale_sensitivity(gyro: &mut GyroConfig, dir: i32, step: f32) -> Option<()> {
    let GyroMode::Mouse { sensitivity } = &mut gyro.mode else { return None };
    *sensitivity = scale(*sensitivity, dir, step, SENSITIVITY_RANGE);
    Some(())
}

fn scale(value: f32, dir: i32, step: f32, (low, high): (f32, f32)) -> f32 {
    let factor = if dir < 0 { 1.0 / step } else { step };
    (value * factor).clamp(low, high)
}

/// The profile after (or before) the active one in its game, wrapping around.
fn next_profile(dir: i32, config: &Config) -> Option<ProfileRef> {
    let at = config.active_ref();
    let profiles = &config.active_game().profiles;
    let i = profiles.iter().position(|p| p.name == at.profile)?;
    let next = (i as i32 + dir).rem_euclid(profiles.len() as i32) as usize;
    Some(ProfileRef::new(at.game.as_deref(), &profiles[next].name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MouseResponse;

    fn config() -> Config {
        let mut config = Config::default();
        config.general.profiles.truncate(1);
        let profile = &mut config.general.profiles[0];
        profile.left_stick.action = StickAction::Mouse { speed: 1000.0, response: MouseResponse::default(), invert_y: false };
        profile.gyro.mode = GyroMode::Mouse { sensitivity: 10.0 };
        config
    }

    #[test]
    fn gyro_goes_off_and_back_to_what_it_was() {
        let mut config = config();
        let saved = config.clone();
        assert!(rows(&config, &saved).contains(&Row::Gyro));
        adjust(Row::Gyro, 1, false, &mut config, &saved);
        assert!(matches!(config.active().unwrap().gyro.mode, GyroMode::Off));
        assert!(rows(&config, &saved).contains(&Row::Gyro), "still there to turn back on");
        assert!(!rows(&config, &saved).contains(&Row::GyroSensitivity));
        adjust(Row::Gyro, 1, false, &mut config, &saved);
        assert_eq!(config.active().unwrap().gyro.mode, saved.active().unwrap().gyro.mode);
    }

    #[test]
    fn dead_zones_rumble_and_overlay_size_step_and_stay_in_range() {
        let mut config = config();
        let saved = config.clone();
        let deadzone = config.active().unwrap().left_stick.deadzone;
        adjust(Row::Deadzone(Stick::Left), 1, false, &mut config, &saved);
        assert!((config.active().unwrap().left_stick.deadzone - (deadzone + 0.01)).abs() < 1e-4);
        assert!(label(Row::Deadzone(Stick::Left), &config, &saved).contains("(was"));
        for _ in 0..20 {
            adjust(Row::Rumble, -1, false, &mut config, &saved);
        }
        assert_eq!(config.rumble_strength, 0.0);
        assert_eq!(label(Row::Rumble, &config, &saved), "Rumble: 0% (was 100%)");
        adjust(Row::OverlaySize, 1, true, &mut config, &saved);
        assert!((config.active_menu_style().scale - 1.25).abs() < 1e-4);
        assert!((config.keyboard_style.scale - 1.25).abs() < 1e-4, "every panel takes the size");
        assert_eq!(adjust(Row::Remapping, 1, false, &mut config, &saved), Some(Outcome::RemappingOff));
    }
}
