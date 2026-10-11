//! Acceleration curves for motion that moves the mouse: how fast the input moves scales its
//! sensitivity (gyro, touchpad), and named presets that also tune a mouse stick's response.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::MouseResponse;

/// Sensitivity by input speed: `slow_gain` times the base sensitivity up to `slow_speed`,
/// `fast_gain` times it from `fast_speed`, and a blend between shaped by `power` (1 is a
/// straight ramp, more stays slow longer). Speeds are in the input's own units, such as
/// degrees/second for a gyro. The default is flat: the base sensitivity at every speed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeedCurve {
    pub slow_gain: f32,
    pub fast_gain: f32,
    pub slow_speed: f32,
    pub fast_speed: f32,
    pub power: f32,
}

impl Default for SpeedCurve {
    fn default() -> Self {
        SpeedCurve { slow_gain: 1.0, fast_gain: 1.0, slow_speed: 0.0, fast_speed: 100.0, power: 1.0 }
    }
}

impl SpeedCurve {
    /// The sensitivity multiplier at input speed `speed`.
    pub fn gain(&self, speed: f32) -> f32 {
        if self.slow_gain == self.fast_gain {
            return self.slow_gain;
        }
        let span = (self.fast_speed - self.slow_speed).max(1e-3);
        let t = ((speed - self.slow_speed) / span).clamp(0.0, 1.0).powf(self.power.max(0.1));
        self.slow_gain + (self.fast_gain - self.slow_gain) * t
    }

    /// Left out of saved configs when flat, so they stay as they were.
    pub fn is_flat(&self) -> bool {
        *self == SpeedCurve::default()
    }

    /// The preset these settings match, or Custom, for speeds in units where `scale` is one
    /// degree/second of gyro turning (see [`CurvePreset::speed_curve`]).
    pub fn preset(&self, scale: f32) -> CurvePreset {
        CurvePreset::NAMED.into_iter().find(|p| p.speed_curve(scale).is_some_and(|c| close(&c, self))).unwrap_or(CurvePreset::Custom)
    }

    /// Where Custom starts: this curve with its shape changed, so it matches no preset. From a
    /// flat curve, which has nothing to change, it starts from Gentle.
    pub fn custom_from(&self, scale: f32) -> SpeedCurve {
        if self.is_flat() {
            SpeedCurve { power: 1.5, ..CurvePreset::Gentle.speed_curve(scale).unwrap_or_default() }
        } else {
            SpeedCurve { power: self.power + 0.25, ..*self }
        }
    }
}

/// Equal to within slider rounding.
fn close(a: &SpeedCurve, b: &SpeedCurve) -> bool {
    let near = |x: f32, y: f32| (x - y).abs() <= 1e-3 * x.abs().max(y.abs()).max(1.0);
    near(a.slow_gain, b.slow_gain)
        && near(a.fast_gain, b.fast_gain)
        && near(a.slow_speed, b.slow_speed)
        && near(a.fast_speed, b.fast_speed)
        && near(a.power, b.power)
}

/// A named acceleration curve, the same names for every mapping that moves the mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurvePreset {
    Linear,
    Gentle,
    Aggressive,
    Wide,
    ExtraWide,
    Precision,
    TurnBoost,
    /// Settings that match no preset.
    Custom,
}

impl CurvePreset {
    /// Every preset with settings of its own, for pickers (Custom is shown when nothing matches).
    pub const NAMED: [CurvePreset; 7] = [
        CurvePreset::Linear,
        CurvePreset::Gentle,
        CurvePreset::Aggressive,
        CurvePreset::Wide,
        CurvePreset::ExtraWide,
        CurvePreset::Precision,
        CurvePreset::TurnBoost,
    ];

    /// The preset as a speed curve. The speeds are written in degrees/second of gyro turning and
    /// multiplied by `scale`, so another input (a touchpad's pad widths/second) gets a curve of
    /// the same feel.
    pub fn speed_curve(self, scale: f32) -> Option<SpeedCurve> {
        let (slow_gain, fast_gain, slow_speed, fast_speed, power) = match self {
            CurvePreset::Linear => return Some(SpeedCurve::default()),
            CurvePreset::Gentle => (0.75, 1.25, 5.0, 60.0, 1.0),
            CurvePreset::Aggressive => (0.5, 2.0, 5.0, 40.0, 1.0),
            CurvePreset::Wide => (0.6, 1.6, 5.0, 150.0, 1.0),
            CurvePreset::ExtraWide => (0.5, 2.0, 0.0, 300.0, 1.0),
            CurvePreset::Precision => (0.35, 1.0, 2.0, 30.0, 2.0),
            CurvePreset::TurnBoost => (1.0, 2.5, 120.0, 360.0, 1.0),
            CurvePreset::Custom => return None,
        };
        Some(SpeedCurve { slow_gain, fast_gain, slow_speed: slow_speed * scale, fast_speed: fast_speed * scale, power })
    }

    /// The preset for a mouse stick: its curve exponent and response. Acceleration on a stick
    /// builds while it is held at full push, so the same feel is made from the curve's shape,
    /// the hold-time acceleration and the outer boost.
    pub fn stick_response(self, base: MouseResponse) -> Option<(f32, MouseResponse)> {
        let (curve, accel, accel_ramp_ms, outer_boost) = match self {
            CurvePreset::Linear => (1.0, 0.0, 400, 0.0),
            CurvePreset::Gentle => (1.5, 0.2, 400, 0.0),
            CurvePreset::Aggressive => (1.2, 0.8, 250, 0.3),
            CurvePreset::Wide => (2.0, 0.4, 600, 0.0),
            CurvePreset::ExtraWide => (2.6, 0.6, 800, 0.2),
            CurvePreset::Precision => (3.0, 0.0, 400, 0.4),
            CurvePreset::TurnBoost => (1.5, 1.0, 300, 0.5),
            CurvePreset::Custom => return None,
        };
        Some((curve, MouseResponse { accel, accel_ramp_ms, outer_boost, ..base }))
    }

    /// The preset a mouse stick's curve and response match, or Custom.
    pub fn of_stick(curve: f32, response: &MouseResponse) -> CurvePreset {
        let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
        CurvePreset::NAMED
            .into_iter()
            .find(|p| {
                p.stick_response(*response).is_some_and(|(c, r)| {
                    near(c, curve) && near(r.accel, response.accel) && near(r.outer_boost, response.outer_boost) && (r.accel == 0.0 || r.accel_ramp_ms == response.accel_ramp_ms)
                })
            })
            .unwrap_or(CurvePreset::Custom)
    }

    /// What the preset is for, in a few words.
    pub fn about(self) -> &'static str {
        match self {
            CurvePreset::Linear => "The same sensitivity at every speed.",
            CurvePreset::Gentle => "A little slower when slow, a little faster when fast.",
            CurvePreset::Aggressive => "Slow for aiming, quickly much faster for turning.",
            CurvePreset::Wide => "Speeds up gradually over a wide range.",
            CurvePreset::ExtraWide => "Speeds up very gradually, from still to fast turns.",
            CurvePreset::Precision => "Much slower for small movements, for sniping.",
            CurvePreset::TurnBoost => "Unchanged when aiming, faster only for big quick turns.",
            CurvePreset::Custom => "Your own settings.",
        }
    }
}

impl fmt::Display for CurvePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CurvePreset::Linear => "Linear",
            CurvePreset::Gentle => "Gentle",
            CurvePreset::Aggressive => "Aggressive",
            CurvePreset::Wide => "Wide",
            CurvePreset::ExtraWide => "Extra wide",
            CurvePreset::Precision => "Precision",
            CurvePreset::TurnBoost => "Turn boost",
            CurvePreset::Custom => "Custom",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_curve_keeps_the_base_sensitivity() {
        assert_eq!(SpeedCurve::default().gain(0.0), 1.0);
        assert_eq!(SpeedCurve::default().gain(1000.0), 1.0);
    }

    #[test]
    fn the_gain_blends_from_slow_to_fast() {
        let c = CurvePreset::Aggressive.speed_curve(1.0).unwrap();
        assert_eq!(c.gain(0.0), 0.5);
        assert!((c.gain(22.5) - 1.25).abs() < 1e-4);
        assert_eq!(c.gain(500.0), 2.0);
    }

    #[test]
    fn presets_are_recognized_at_any_scale() {
        for p in CurvePreset::NAMED {
            assert_eq!(p.speed_curve(0.01).unwrap().preset(0.01), p);
            let (curve, response) = p.stick_response(MouseResponse::default()).unwrap();
            assert_eq!(CurvePreset::of_stick(curve, &response), p);
        }
        let tweaked = SpeedCurve { fast_gain: 1.7, ..CurvePreset::Gentle.speed_curve(1.0).unwrap() };
        assert_eq!(tweaked.preset(1.0), CurvePreset::Custom);
    }

    #[test]
    fn custom_starts_from_a_curve_of_its_own() {
        for scale in [1.0, 0.025] {
            for p in CurvePreset::NAMED {
                let from = p.speed_curve(scale).unwrap();
                assert_eq!(from.custom_from(scale).preset(scale), CurvePreset::Custom, "from {p}");
            }
        }
        // From a preset, the rest of its curve is kept.
        let wide = CurvePreset::Wide.speed_curve(1.0).unwrap();
        assert_eq!(wide.custom_from(1.0).fast_speed, wide.fast_speed);
    }
}
