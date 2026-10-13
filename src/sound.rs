//! Overlay sounds: short cues, synthesized in code, played as the cursor moves in an overlay and
//! when something is picked there. A setup sets them for each kind of overlay, and any kind can
//! be silenced. Playback runs on its own thread so a slow audio device never holds up input.

mod effects;
mod kind;
mod library;
mod player;
mod synth;
mod wav;

use serde::{Deserialize, Serialize};

pub use kind::SoundKind;
pub use player::Sounds;
pub use wav::{SoundFile, decode_wav, pack_sound_names, set_pack_sounds};
use synth::synthesize;

/// Sample rate of every cue.
const RATE: u32 = 44_100;
/// Peak level of a cue before its volume is applied.
const PEAK: f32 = 0.9;


/// One cue: its kind, how loud it is, how high, and how long, and the effects on it. Volume,
/// pitch and length are factors on the default, so 1.0 is what the kind sounds like on its own;
/// with every effect at 0 the cue is as the kind makes it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SoundSpec {
    #[serde(default)]
    pub kind: SoundKind,
    /// A sound file the game's pack carries, played in place of the kind's own sound (which
    /// plays if the pack isn't in use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<SoundFile>,
    /// 0..=1.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// 0.5..=2 (an octave either way).
    #[serde(default = "default_pitch")]
    pub pitch: f32,
    /// 0.5..=2 (half as long, or twice as long).
    #[serde(default = "default_length")]
    pub length: f32,
    /// -12..=12 dB: the lows turned down or up.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bass: f32,
    /// -12..=12 dB: the highs turned down or up.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub treble: f32,
    /// 0..=1: how hard it's pushed into soft clipping, gritty and thick.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub drive: f32,
    /// 0..=1: how loud and how many the echoes after it are.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub echo: f32,
    /// 0..=1: how much of a room it rings in, and how big.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reverb: f32,
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes a reference")]
fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

fn default_volume() -> f32 {
    0.25
}

fn default_pitch() -> f32 {
    1.0
}

fn default_length() -> f32 {
    1.0
}

impl SoundSpec {
    pub fn new(kind: SoundKind, volume: f32, pitch: f32) -> Self {
        SoundSpec { kind, file: None, volume, pitch, length: default_length(), bass: 0.0, treble: 0.0, drive: 0.0, echo: 0.0, reverb: 0.0 }
    }

    /// How long the cue plays, with any echo and reverb after it.
    pub fn duration(&self) -> std::time::Duration {
        let tail = effects::tail(self) as f32 / RATE as f32;
        let length = self.length.clamp(0.5, 2.0);
        let body = match self.file.and_then(wav::pack_samples) {
            Some(file) => file.len() as f32 / RATE as f32 / self.pitch.clamp(0.5, 2.0) * length,
            None if self.kind.is_recording() => self.kind.seconds() / self.pitch.clamp(0.5, 2.0) * length,
            None => self.kind.seconds() * length,
        };
        std::time::Duration::from_secs_f32(body + tail)
    }

    /// The cue's samples, mono at [`RATE`], with volume applied.
    pub fn samples(&self) -> Vec<f32> {
        let pitch = self.pitch.clamp(0.5, 2.0);
        let length = self.length.clamp(0.5, 2.0);
        let volume = self.volume.clamp(0.0, 1.0);
        let mut out = match self.file.and_then(wav::pack_samples) {
            // A file is played faster or slower for its pitch, then stretched to its length.
            Some(file) => wav::stretch(&wav::resample(&file, pitch), length),
            None => synthesize(self.kind, pitch, length),
        };
        effects::apply(&mut out, self);
        // The cue peaks at 1 before its effects. Echoes piling up, or boosted bass, can push it
        // past that, so it comes back down; a cut leaves it quieter, as a cut should.
        let peak = out.iter().fold(0.0_f32, |p, s| p.max(s.abs())).max(1.0);
        let gain = PEAK / peak * volume;
        for s in &mut out {
            *s *= gain;
        }
        out
    }
}

/// The sounds of one kind of overlay: whether it makes any, a cue for each step of the cursor,
/// one for a pick, and one as it pops in and out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySounds {
    /// Off silences all of them.
    pub enabled: bool,
    pub step: SoundSpec,
    pub pick: SoundSpec,
    pub open: SoundSpec,
    pub close: SoundSpec,
    /// A mode switched to (the Guide overlay's Left and Right).
    pub mode: SoundSpec,
}

impl Default for OverlaySounds {
    fn default() -> Self {
        OverlaySounds {
            enabled: true,
            step: SoundSpec::new(SoundKind::Click, 0.12, 1.2),
            pick: SoundSpec::new(SoundKind::Pop, 0.3, 1.0),
            // Quieter than a step, so an overlay coming or going is noticed without pulling focus.
            open: SoundSpec::new(SoundKind::Pop, 0.05, 1.3),
            close: SoundSpec::new(SoundKind::Pop, 0.04, 0.8),
            mode: SoundSpec::new(SoundKind::Swish, 0.08, 1.0),
        }
    }
}

impl OverlaySounds {
    /// The Guide menus' own: soft electronic tones, calm enough to hear over any game, and set
    /// apart from a game's menus so the controller's own menus sound like the system.
    pub fn guide() -> Self {
        OverlaySounds {
            enabled: true,
            step: SoundSpec::new(SoundKind::Pulse, 0.1, 1.0),
            pick: SoundSpec { length: 0.8, ..SoundSpec::new(SoundKind::Glass, 0.2, 0.8) },
            open: SoundSpec::new(SoundKind::Rise, 0.07, 1.0),
            close: SoundSpec::new(SoundKind::Fall, 0.06, 1.0),
            // A mode switch is a change of state rather than a cursor step, so it's a breath, not a tick.
            mode: SoundSpec { length: 0.8, ..SoundSpec::new(SoundKind::Swish, 0.08, 1.0) },
        }
    }
}

/// The overlays that have a cursor to sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundOverlay {
    Keyboard,
    Numpad,
    /// The menus a setup defines.
    Menu,
    /// The Guide overlay's menu, and the Guide + Start menu it leads to.
    Guide,
    Media,
    Offer,
}

impl SoundOverlay {
    pub const ALL: [SoundOverlay; 6] = [
        SoundOverlay::Keyboard,
        SoundOverlay::Numpad,
        SoundOverlay::Menu,
        SoundOverlay::Guide,
        SoundOverlay::Media,
        SoundOverlay::Offer,
    ];

    /// Its table in a set of sounds: `[sounds.<key>]`.
    pub fn key(self) -> &'static str {
        match self {
            SoundOverlay::Keyboard => "keyboard",
            SoundOverlay::Numpad => "numpad",
            SoundOverlay::Menu => "menu",
            SoundOverlay::Guide => "guide",
            SoundOverlay::Media => "media",
            SoundOverlay::Offer => "offer",
        }
    }

    /// What the settings call this overlay.
    pub fn label(self) -> &'static str {
        match self {
            SoundOverlay::Keyboard => "On-screen keyboard",
            SoundOverlay::Numpad => "On-screen numpad",
            SoundOverlay::Menu => "Menus",
            SoundOverlay::Guide => "Guide menus",
            SoundOverlay::Media => "Media controls",
            SoundOverlay::Offer => "Library offers",
        }
    }
}

/// The sounds of every overlay. A game's own set replaces the defaults.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundSet {
    pub keyboard: OverlaySounds,
    pub numpad: OverlaySounds,
    pub menu: OverlaySounds,
    pub guide: OverlaySounds,
    pub media: OverlaySounds,
    pub offer: OverlaySounds,
}

impl Default for SoundSet {
    fn default() -> Self {
        let common = OverlaySounds::default();
        SoundSet { keyboard: common, numpad: common, menu: common, guide: OverlaySounds::guide(), media: common, offer: common }
    }
}

impl SoundSet {
    pub fn get(&self, overlay: SoundOverlay) -> OverlaySounds {
        match overlay {
            SoundOverlay::Keyboard => self.keyboard,
            SoundOverlay::Numpad => self.numpad,
            SoundOverlay::Menu => self.menu,
            SoundOverlay::Guide => self.guide,
            SoundOverlay::Media => self.media,
            SoundOverlay::Offer => self.offer,
        }
    }

    pub fn set(&mut self, overlay: SoundOverlay, sounds: OverlaySounds) {
        let slot = match overlay {
            SoundOverlay::Keyboard => &mut self.keyboard,
            SoundOverlay::Numpad => &mut self.numpad,
            SoundOverlay::Menu => &mut self.menu,
            SoundOverlay::Guide => &mut self.guide,
            SoundOverlay::Media => &mut self.media,
            SoundOverlay::Offer => &mut self.offer,
        };
        *slot = sounds;
    }

    /// The cue for `feedback` in `overlay`, or `None` when that overlay is silenced.
    pub fn cue(&self, overlay: SoundOverlay, feedback: Feedback) -> Option<SoundSpec> {
        let sounds = self.get(overlay);
        if !sounds.enabled {
            return None;
        }
        Some(match feedback {
            Feedback::Step => sounds.step,
            Feedback::Pick => sounds.pick,
            Feedback::Open => sounds.open,
            Feedback::Close => sounds.close,
            Feedback::Mode => sounds.mode,
        })
    }

    pub fn is_default(&self) -> bool {
        *self == SoundSet::default()
    }

    /// The same set with every overlay silenced (their cues kept, for turning them back on).
    pub fn silenced(mut self) -> Self {
        for overlay in SoundOverlay::ALL {
            self.set(overlay, OverlaySounds { enabled: false, ..self.get(overlay) });
        }
        self
    }
}

/// What happened in an overlay: a pick, a step of the cursor, it popping in or out, or a mode
/// switched to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feedback {
    Step,
    Pick,
    Open,
    Close,
    Mode,
}

impl Feedback {
    /// A pick if something was chosen, else a step if the cursor moved, else nothing.
    pub fn of(picked: bool, moved: bool) -> Option<Self> {
        if picked {
            Some(Feedback::Pick)
        } else if moved {
            Some(Feedback::Step)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
