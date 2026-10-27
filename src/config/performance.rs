//! The App settings' performance card: what every overlay goes without, whatever the setups say,
//! for machines where drawing them costs too much.

use serde::{Deserialize, Serialize};

use super::Lite;
use crate::{motion::MotionSet, sound::SoundSet};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Performance {
    /// Overlays without decorations: no shadows, gradients, glows or outlines, the default ones
    /// included.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub plain: bool,
    /// Nothing moves in any overlay.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub still: bool,
    /// No overlay sounds, and the audio device is never opened.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub quiet: bool,
    /// Looping animations stop (glows, drifting gradients and scanlines), though the motion
    /// styles still play.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_loops: bool,
    /// What share of the display's pixels overlays are drawn at, stretched to fill it.
    #[serde(skip_serializing_if = "Resolution::is_full")]
    pub resolution: Resolution,
}

/// What share of the display's pixels overlays are drawn at: fewer is less to draw, and softer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    #[default]
    Full,
    ThreeQuarters,
    Half,
}

impl Resolution {
    pub const ALL: [Resolution; 3] = [Resolution::Full, Resolution::ThreeQuarters, Resolution::Half];

    pub fn is_full(&self) -> bool {
        *self == Resolution::Full
    }

    /// The share of the display's pixels, along each side, in hundredths.
    pub fn percent(self) -> u32 {
        match self {
            Resolution::Full => 100,
            Resolution::ThreeQuarters => 75,
            Resolution::Half => 50,
        }
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resolution::Full => f.write_str("Full"),
            other => write!(f, "{}%", other.percent()),
        }
    }
}

impl Performance {
    pub fn is_default(&self) -> bool {
        *self == Performance::default()
    }

    /// Every switch off: overlays at their barest. The resolution is a choice of its own and
    /// stays as it was.
    pub fn lightest(self) -> Self {
        Performance { plain: true, still: true, quiet: true, no_loops: true, ..self }
    }

    /// Every switch on, the resolution as it was.
    pub fn fullest(self) -> Self {
        Performance { resolution: self.resolution, ..Performance::default() }
    }

    /// What the overlay process is told to cut from what it draws.
    pub fn lite(&self) -> Lite {
        Lite { plain: self.plain, calm: self.no_loops || self.still, resolution: self.resolution }
    }

    /// `motion`, or no motion at all when it's turned off here.
    pub fn motion(&self, motion: MotionSet) -> MotionSet {
        if self.still { MotionSet::all(crate::motion::MotionStyle::Off) } else { motion }
    }

    /// `sounds`, or every one silenced when they're turned off here.
    pub fn sounds(&self, sounds: SoundSet) -> SoundSet {
        if self.quiet { sounds.silenced() } else { sounds }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_cut_unless_asked() {
        let p: Performance = toml::from_str("").unwrap();
        assert!(p.is_default() && p.lite().is_off());
        assert_eq!(p.motion(MotionSet::default()), MotionSet::default());
        assert!(toml::to_string(&p).unwrap().is_empty());
    }

    #[test]
    fn looping_animations_can_stop_alone() {
        let p: Performance = toml::from_str("no_loops = true").unwrap();
        assert_eq!(p.lite(), Lite { plain: false, calm: true, ..Lite::default() });
        assert_eq!(p.motion(MotionSet::default()), MotionSet::default());
        assert_eq!(Performance::default().lite(), Lite::default());
    }

    #[test]
    fn the_lightest_cuts_everything() {
        let p = Performance::default().lightest();
        assert_eq!(p.lite(), Lite { plain: true, calm: true, ..Lite::default() });
        assert_eq!(p.motion(MotionSet::default()), MotionSet::all(crate::motion::MotionStyle::Off));
        assert_eq!(p.sounds(SoundSet::default()), SoundSet::default().silenced());
        let back: Performance = toml::from_str(&toml::to_string(&p).unwrap()).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn the_resolution_survives_the_switches_and_reaches_the_overlay() {
        let p: Performance = toml::from_str("resolution = \"three_quarters\"").unwrap();
        assert_eq!(p.lite().resolution, Resolution::ThreeQuarters);
        assert!(!p.lite().is_off());
        assert_eq!(p.lightest().resolution, Resolution::ThreeQuarters);
        assert_eq!(p.lightest().fullest(), p);
        assert_eq!(toml::to_string(&p).unwrap().trim(), "resolution = \"three_quarters\"");
    }
}
