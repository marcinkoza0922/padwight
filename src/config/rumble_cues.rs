//! Rumble cues: short buzzes on the controller that drives an overlay, as its cursor moves and
//! as something is picked, opened or closed. A setup sets them per overlay, the way it sets the
//! sounds; with none set, nothing buzzes.

use serde::{Deserialize, Serialize};

use crate::sound::{Feedback, SoundOverlay};

/// One buzz: how hard it is and how long it lasts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RumbleCue {
    /// 0..=1: how hard the controller buzzes. The App settings' rumble strength scales it.
    pub strength: f32,
    /// 10..=500: how long the buzz lasts, in milliseconds.
    pub length_ms: u32,
}

impl RumbleCue {
    /// The cue with its strength and length kept in the ranges the controller can play.
    pub fn clamped(self) -> Self {
        RumbleCue { strength: self.strength.clamp(0.0, 1.0), length_ms: self.length_ms.clamp(10, 500) }
    }
}

/// The buzzes of one overlay: one for each step of the cursor, a pick, an open, a close and a
/// mode switch. An unset one is silent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayRumble {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<RumbleCue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick: Option<RumbleCue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<RumbleCue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close: Option<RumbleCue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RumbleCue>,
}

impl OverlayRumble {
    pub fn is_default(&self) -> bool {
        *self == OverlayRumble::default()
    }

    /// The buzz for `feedback`, if this overlay has one.
    pub fn cue(&self, feedback: Feedback) -> Option<RumbleCue> {
        match feedback {
            Feedback::Step => self.step,
            Feedback::Pick => self.pick,
            Feedback::Open => self.open,
            Feedback::Close => self.close,
            Feedback::Mode => self.mode,
        }
        .map(RumbleCue::clamped)
    }
}

/// The rumble cues of every overlay. A game's own set replaces the default, which is silent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RumbleSet {
    #[serde(default, skip_serializing_if = "OverlayRumble::is_default")]
    pub keyboard: OverlayRumble,
    #[serde(default, skip_serializing_if = "OverlayRumble::is_default")]
    pub numpad: OverlayRumble,
    #[serde(default, skip_serializing_if = "OverlayRumble::is_default")]
    pub menu: OverlayRumble,
    #[serde(default, skip_serializing_if = "OverlayRumble::is_default")]
    pub guide: OverlayRumble,
    #[serde(default, skip_serializing_if = "OverlayRumble::is_default")]
    pub media: OverlayRumble,
    #[serde(default, skip_serializing_if = "OverlayRumble::is_default")]
    pub offer: OverlayRumble,
}

impl RumbleSet {
    /// The buzz for `feedback` in `overlay`, if that overlay buzzes for it.
    pub fn cue(&self, overlay: SoundOverlay, feedback: Feedback) -> Option<RumbleCue> {
        let rumble = match overlay {
            SoundOverlay::Keyboard => &self.keyboard,
            SoundOverlay::Numpad => &self.numpad,
            SoundOverlay::Menu => &self.menu,
            SoundOverlay::Guide => &self.guide,
            SoundOverlay::Media => &self.media,
            SoundOverlay::Offer => &self.offer,
        };
        rumble.cue(feedback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_unless_a_cue_is_set() {
        let set: RumbleSet = toml::from_str("[menu]\nstep = { strength = 0.3, length_ms = 25 }\n").unwrap();
        assert_eq!(set.cue(SoundOverlay::Menu, Feedback::Step), Some(RumbleCue { strength: 0.3, length_ms: 25 }));
        assert_eq!(set.cue(SoundOverlay::Menu, Feedback::Pick), None);
        assert_eq!(set.cue(SoundOverlay::Guide, Feedback::Step), None);
        assert!(toml::to_string(&RumbleSet::default()).unwrap().is_empty());
    }

    #[test]
    fn a_cue_is_kept_playable() {
        let loud: RumbleCue = toml::from_str("strength = 4.0\nlength_ms = 1").unwrap();
        assert_eq!(loud.clamped(), RumbleCue { strength: 1.0, length_ms: 10 });
    }
}
