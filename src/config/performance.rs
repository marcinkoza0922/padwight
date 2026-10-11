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
}

impl Performance {
    pub fn is_default(&self) -> bool {
        *self == Performance::default()
    }

    /// Everything off: overlays at their barest.
    pub fn lightest() -> Self {
        Performance { plain: true, still: true, quiet: true }
    }

    /// What the overlay process is told to cut from what it draws.
    pub fn lite(&self) -> Lite {
        Lite { plain: self.plain }
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
    fn the_lightest_cuts_everything() {
        let p = Performance::lightest();
        assert_eq!(p.lite(), Lite { plain: true });
        assert_eq!(p.motion(MotionSet::default()), MotionSet::all(crate::motion::MotionStyle::Off));
        assert_eq!(p.sounds(SoundSet::default()), SoundSet::default().silenced());
        let back: Performance = toml::from_str(&toml::to_string(&p).unwrap()).unwrap();
        assert_eq!(back, p);
    }
}
