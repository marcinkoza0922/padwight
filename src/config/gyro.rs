//! A profile's gyro settings: what turning the controller does, when it does it, and how the
//! turning is read, filtered and shaped on the way.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{SpeedCurve, Stick, is_one, is_true, is_zero, yes};

/// Which rotation drives horizontal aim. Pitch drives vertical, except in world space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GyroHorizontal {
    /// Turning the controller left/right like a flashlight.
    Yaw,
    /// Tilting it like a steering wheel.
    Roll,
    /// Both added together, so either motion works.
    YawAndRoll,
    /// Turning about the player's up, found from gravity, so turning left aims left however
    /// the controller is held; up and down stay the controller's own.
    PlayerSpace,
    /// Turning about the world's up and tilting about the horizon, both found from gravity.
    WorldSpace,
}

impl GyroHorizontal {
    pub const ALL: [GyroHorizontal; 5] =
        [GyroHorizontal::PlayerSpace, GyroHorizontal::WorldSpace, GyroHorizontal::Yaw, GyroHorizontal::Roll, GyroHorizontal::YawAndRoll];
}

impl fmt::Display for GyroHorizontal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GyroHorizontal::Yaw => "Yaw (turn)",
            GyroHorizontal::Roll => "Roll (tilt)",
            GyroHorizontal::YawAndRoll => "Yaw + roll",
            GyroHorizontal::PlayerSpace => "Player space (any grip)",
            GyroHorizontal::WorldSpace => "World space",
        })
    }
}

/// Which axes the gyro moves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GyroAxes {
    #[default]
    Both,
    /// Left and right only, such as turning with the gyro and looking up and down with a stick.
    Horizontal,
    /// Up and down only, such as turning with a stick and aiming up and down with the gyro.
    Vertical,
}

impl GyroAxes {
    pub const ALL: [GyroAxes; 3] = [GyroAxes::Both, GyroAxes::Horizontal, GyroAxes::Vertical];

    pub fn horizontal(self) -> bool {
        self != GyroAxes::Vertical
    }

    pub fn vertical(self) -> bool {
        self != GyroAxes::Horizontal
    }
}

impl fmt::Display for GyroAxes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            GyroAxes::Both => "Both",
            GyroAxes::Horizontal => "Horizontal only",
            GyroAxes::Vertical => "Vertical only",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GyroMode {
    /// Rotation moves the mouse; `sensitivity` is pixels per degree turned (or, with
    /// [`GyroConfig::full_turn_px`], a multiple of the real-world turn).
    Mouse { sensitivity: f32 },
    /// Rotation speed deflects a virtual-pad stick (added to the physical stick). Turning at
    /// `full_rate` degrees/second is full deflection; `anti_deadzone` is the smallest
    /// deflection sent, to get past a game's own stick deadzone. `game_curve` is the exponent of
    /// the game's own stick response (1 for linear): the deflection undoes it, so the game turns
    /// at the rate the controller does, as a mouse would make it.
    Stick {
        stick: Stick,
        full_rate: f32,
        anti_deadzone: f32,
        #[serde(default = "one")]
        game_curve: f32,
    },
    /// Turning the controller away from where gyro started (or was recentered) deflects a
    /// virtual-pad stick, `max_angle` degrees being full deflection. With `lock_at_edge`,
    /// turning further than that is let go, so turning back moves off the edge at once.
    Position {
        stick: Stick,
        max_angle: f32,
        #[serde(default)]
        anti_deadzone: f32,
        #[serde(default)]
        lock_at_edge: bool,
    },
    /// Tilting like a steering wheel moves a stick left/right; `max_angle` degrees from
    /// center is full lock.
    Steering { stick: Stick, max_angle: f32 },
}

impl GyroMode {
    /// Whether turning speed is shaped by the acceleration curve: the modes that follow how fast
    /// the controller turns.
    pub fn uses_curve(&self) -> bool {
        matches!(self, GyroMode::Mouse { .. } | GyroMode::Stick { .. })
    }
}

fn default_mode() -> GyroMode {
    GyroMode::Mouse { sensitivity: 15.0 }
}

fn one() -> f32 {
    1.0
}

fn default_smoothing_ms() -> u32 {
    125
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_default_smoothing(v: &u32) -> bool {
    *v == default_smoothing_ms()
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GyroConfig {
    /// Whether the profile uses gyro at all. While it doesn't, gyro does nothing, no input may
    /// hold a gyro action, and the aim settings are kept for when it is turned on. Left out of a
    /// save when it is, so a profile that doesn't say uses it.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub enabled: bool,
    /// Whether gyro is active from the start, once enabled. When it isn't, an action turns it
    /// on: Gyro active / inactive, or Gyro while held. Left out of a save when it is.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub starts_active: bool,
    #[serde(default = "default_mode")]
    pub mode: GyroMode,
    pub horizontal: GyroHorizontal,
    #[serde(default)]
    pub invert_x: bool,
    #[serde(default)]
    pub invert_y: bool,
    /// The axes the gyro moves; the others are ignored.
    #[serde(default, skip_serializing_if = "is_both")]
    pub axes: GyroAxes,
    /// Horizontal and vertical sensitivity, as multiples of the mode's own.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub x_scale: f32,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub y_scale: f32,
    /// How turning speed scales the sensitivity (Mouse and Stick modes), in degrees/second.
    #[serde(default, skip_serializing_if = "SpeedCurve::is_flat")]
    pub curve: SpeedCurve,
    /// Mouse pixels that turn the game a full circle. When set, Mouse sensitivity is real-world:
    /// 1.0 turns the game as far as the controller turned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_turn_px: Option<f32>,
    /// Rotation slower than this (degrees/second) is scaled down, hiding jitter and drift.
    pub noise_threshold: f32,
    /// Rotation slower than this (degrees/second) is smoothed over `smoothing_ms`, steadying
    /// slow aim without delaying fast turns; 0 is off.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub smoothing_threshold: f32,
    #[serde(default = "default_smoothing_ms", skip_serializing_if = "is_default_smoothing")]
    pub smoothing_ms: u32,
    /// Gyro pauses while this physical stick is pushed, so stick and gyro don't fight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_with_stick: Option<Stick>,
    /// Re-measures drift whenever the controller is set down still.
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_calibrate: bool,
    /// A short rumble each time a toggle switches gyro on or off.
    #[serde(default, skip_serializing_if = "is_false")]
    pub toggle_rumble: bool,
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde's skip_serializing_if passes a reference")]
fn is_both(v: &GyroAxes) -> bool {
    *v == GyroAxes::Both
}

impl Default for GyroConfig {
    fn default() -> Self {
        GyroConfig {
            enabled: false,
            starts_active: true,
            mode: default_mode(),
            horizontal: GyroHorizontal::Yaw,
            invert_x: false,
            invert_y: false,
            axes: GyroAxes::Both,
            x_scale: 1.0,
            y_scale: 1.0,
            curve: SpeedCurve::default(),
            full_turn_px: None,
            noise_threshold: 1.0,
            smoothing_threshold: 0.0,
            smoothing_ms: default_smoothing_ms(),
            pause_with_stick: None,
            auto_calibrate: false,
            toggle_rumble: false,
        }
    }
}

impl GyroConfig {
    /// Mouse pixels per degree the controller turns, at a curve gain of 1.
    pub fn pixels_per_degree(&self, sensitivity: f32) -> f32 {
        match self.full_turn_px {
            Some(turn) => sensitivity * turn / 360.0,
            None => sensitivity,
        }
    }

    /// Mouse mode's sensitivity in its units ("15.0 px/°", or "1.00× real world"); None in
    /// other modes.
    pub fn sensitivity_label(&self) -> Option<String> {
        let GyroMode::Mouse { sensitivity } = self.mode else { return None };
        Some(match self.full_turn_px {
            Some(_) => format!("{sensitivity:.2}× real world"),
            None => format!("{sensitivity:.1} px/°"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_gyro_settings_still_load_and_new_defaults_stay_out_of_saves() {
        let old = "horizontal = \"yaw\"\nnoise_threshold = 1.0\n[mode.mouse]\nsensitivity = 15.0\n";
        let g: GyroConfig = toml::from_str(old).unwrap();
        assert_eq!(g, GyroConfig { enabled: true, mode: GyroMode::Mouse { sensitivity: 15.0 }, ..GyroConfig::default() });
        let saved = toml::to_string(&g).unwrap();
        for new in ["axes", "x_scale", "curve", "full_turn_px", "smoothing", "pause_with_stick", "auto_calibrate", "toggle_rumble", "enabled", "starts_active"] {
            assert!(!saved.contains(new), "{new} in {saved}");
        }
    }

    #[test]
    fn disabled_gyro_keeps_its_aim_settings() {
        let off = GyroConfig { mode: GyroMode::Mouse { sensitivity: 9.0 }, starts_active: false, ..GyroConfig::default() };
        let saved = toml::to_string(&off).unwrap();
        assert!(saved.contains("enabled = false") && saved.contains("starts_active = false"), "{saved}");
        let back: GyroConfig = toml::from_str(&saved).unwrap();
        assert_eq!(back, off);
        let bare: GyroConfig = toml::from_str("enabled = false\nhorizontal = \"yaw\"\nnoise_threshold = 1.0\n").unwrap();
        assert!(!bare.enabled && bare.starts_active && bare.mode == default_mode(), "a pack can just say gyro is off");
    }

    #[test]
    fn real_world_sensitivity_turns_the_game_as_far_as_the_controller() {
        let g = GyroConfig { full_turn_px: Some(7200.0), ..GyroConfig::default() };
        assert_eq!(g.pixels_per_degree(1.0), 20.0);
        assert_eq!(GyroConfig::default().pixels_per_degree(15.0), 15.0);
        let mouse = GyroConfig { mode: GyroMode::Mouse { sensitivity: 1.0 }, ..g };
        assert_eq!(mouse.sensitivity_label().unwrap(), "1.00× real world");
    }
}
