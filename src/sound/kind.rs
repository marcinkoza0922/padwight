//! The kinds of cue: what each sounds like, its name, and how long it lasts.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{RATE, library};

/// What a cue sounds like. Most kinds are synthesized; the recorded ones (`Typewriter` and the last
/// thirty) play a bundled sample, resampled for pitch and stretched for length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundKind {
    /// A dry tick.
    #[default]
    Click,
    /// A small, bright wooden tick, like a metronome.
    Tick,
    /// A soft bubble that drops in pitch.
    Pop,
    /// A short square-ish bleep that rises. Retro and electronic.
    Blip,
    /// A bright two-note bell.
    Chime,
    /// A high metal ping with a rattle at the start.
    Clink,
    /// A low, short thump.
    Thud,
    /// A knuckle on wood: a dull knock with a little rap.
    Knock,
    /// A few crisp bursts of noise.
    Crunch,
    /// A breath of air that brightens as it passes.
    Swish,
    /// A key going down on a mechanical keyboard: a sharp click with a little thock under it.
    Keys,
    /// A typewriter keystroke, recorded: the typebar's clack on the platen.
    Typewriter,
    /// A single piano key struck: a warm, bright note that dies away.
    Piano,
    /// A clean, flat electronic beep, like a device acknowledging something.
    Beep,
    /// A drop falling into water: a round note that leaps up.
    Drop,
    /// A wooden bar struck with a soft mallet: mellow and woody.
    Marimba,
    /// A crystal glass tapped: a pure, long note that shimmers.
    Glass,
    /// A soft electronic tone with a glassy edge that melts away. Calm and techy.
    Pulse,
    /// A soft tone gliding up: something arriving.
    Rise,
    /// A soft tone gliding down: something leaving.
    Fall,
    /// A retro pickup: two quick square notes, the second ringing.
    Coin,
    /// A round, soft bong, like a muted gong.
    Bong,
    /// A tiny, crisp tap.
    Select,
    /// A soft switch flicked over.
    Toggle,
    /// A plucked string, short and bright.
    Pluck,
    /// A bright, friendly two-tone yes.
    Confirm,
    /// A low, short buzz of refusal.
    Error,
    /// A tiny electronic stutter.
    Glitch,
    /// A light hover, like a finger sliding over a button.
    Rollover,
    /// A chunky switch thrown.
    Switch,
    /// A metal latch clicking home.
    Latch,
    /// A page of a book turned.
    Page,
    /// A light knock on a wooden surface.
    Wood,
    /// A metal plate struck, ringing out.
    Plate,
    /// A tin can tapped: hollow and rattly.
    Tin,
    /// A heavy bell struck, with a long ring.
    Bell,
    /// A Geiger counter's crackle: a few uneven ticks.
    Geiger,
    /// Two quick electronic shimmers, like nanites at work.
    Nano,
    /// A burst of radio static.
    Radio,
    /// A heartbeat: two low, soft thumps.
    Heartbeat,
    /// A spent brass casing bouncing on a hard floor.
    Casing,
    /// Two small, dry metal ticks: a lock's pin giving way.
    Lockpick,
    /// A blade drawn from its sheath: a bright metal ring.
    Shing,
    /// A harsh square-wave stutter, like a failing terminal.
    Fault,
    /// A soft hiss of grains pouring: an hourglass turned.
    Sand,
    /// A cartoon spring: a wobbling tone that bends up and settles.
    Boing,
    /// A camera shutter: two crisp clicks.
    Shutter,
    /// An electric zap: a buzzing tone diving down, with crackle.
    Zap,
    /// A cat's purr: a low, soft, pulsing rumble.
    Purr,
    /// A machine part landing on steel: a thump with a short metal ring.
    Clank,
    /// A playing card flicked down on a table: a papery snap.
    Card,
    /// A typewriter carriage thrown back: a quick ratchet and a clunk at the stop.
    Carriage,
}

impl SoundKind {
    pub const ALL: [SoundKind; 52] = [
        SoundKind::Click,
        SoundKind::Tick,
        SoundKind::Pop,
        SoundKind::Blip,
        SoundKind::Chime,
        SoundKind::Clink,
        SoundKind::Thud,
        SoundKind::Knock,
        SoundKind::Crunch,
        SoundKind::Swish,
        SoundKind::Keys,
        SoundKind::Typewriter,
        SoundKind::Piano,
        SoundKind::Beep,
        SoundKind::Drop,
        SoundKind::Marimba,
        SoundKind::Glass,
        SoundKind::Pulse,
        SoundKind::Rise,
        SoundKind::Fall,
        SoundKind::Coin,
        SoundKind::Bong,
        SoundKind::Select,
        SoundKind::Toggle,
        SoundKind::Pluck,
        SoundKind::Confirm,
        SoundKind::Error,
        SoundKind::Glitch,
        SoundKind::Rollover,
        SoundKind::Switch,
        SoundKind::Latch,
        SoundKind::Page,
        SoundKind::Wood,
        SoundKind::Plate,
        SoundKind::Tin,
        SoundKind::Bell,
        SoundKind::Geiger,
        SoundKind::Nano,
        SoundKind::Radio,
        SoundKind::Heartbeat,
        SoundKind::Casing,
        SoundKind::Lockpick,
        SoundKind::Shing,
        SoundKind::Fault,
        SoundKind::Sand,
        SoundKind::Boing,
        SoundKind::Shutter,
        SoundKind::Zap,
        SoundKind::Purr,
        SoundKind::Clank,
        SoundKind::Card,
        SoundKind::Carriage,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SoundKind::Click => "Click",
            SoundKind::Tick => "Tick",
            SoundKind::Pop => "Pop",
            SoundKind::Blip => "Blip",
            SoundKind::Chime => "Chime",
            SoundKind::Clink => "Clink",
            SoundKind::Thud => "Thud",
            SoundKind::Knock => "Knock",
            SoundKind::Crunch => "Crunch",
            SoundKind::Swish => "Swish",
            SoundKind::Keys => "Keys",
            SoundKind::Typewriter => "Typewriter",
            SoundKind::Piano => "Piano",
            SoundKind::Beep => "Beep",
            SoundKind::Drop => "Drop",
            SoundKind::Marimba => "Marimba",
            SoundKind::Glass => "Glass",
            SoundKind::Pulse => "Pulse",
            SoundKind::Rise => "Rise",
            SoundKind::Fall => "Fall",
            SoundKind::Coin => "Coin",
            SoundKind::Bong => "Bong",
            SoundKind::Select => "Select",
            SoundKind::Toggle => "Toggle",
            SoundKind::Pluck => "Pluck",
            SoundKind::Confirm => "Confirm",
            SoundKind::Error => "Error",
            SoundKind::Glitch => "Glitch",
            SoundKind::Rollover => "Rollover",
            SoundKind::Switch => "Switch",
            SoundKind::Latch => "Latch",
            SoundKind::Page => "Page",
            SoundKind::Wood => "Wood",
            SoundKind::Plate => "Plate",
            SoundKind::Tin => "Tin",
            SoundKind::Bell => "Bell",
            SoundKind::Geiger => "Geiger",
            SoundKind::Nano => "Nano",
            SoundKind::Radio => "Radio",
            SoundKind::Heartbeat => "Heartbeat",
            SoundKind::Casing => "Casing",
            SoundKind::Lockpick => "Lockpick",
            SoundKind::Shing => "Shing",
            SoundKind::Fault => "Fault",
            SoundKind::Sand => "Sand",
            SoundKind::Boing => "Boing",
            SoundKind::Shutter => "Shutter",
            SoundKind::Zap => "Zap",
            SoundKind::Purr => "Purr",
            SoundKind::Clank => "Clank",
            SoundKind::Card => "Card",
            SoundKind::Carriage => "Carriage",
        }
    }

    /// Whether the kind is a bundled recording rather than synthesized.
    pub fn is_recording(self) -> bool {
        library::samples(self).is_some()
    }

    /// How long the cue lasts, in seconds, at normal length.
    pub(super) fn seconds(self) -> f32 {
        if let Some(recording) = library::samples(self) {
            return recording.len() as f32 / RATE as f32;
        }
        match self {
            SoundKind::Click => 0.03,
            SoundKind::Tick => 0.03,
            SoundKind::Pop => 0.09,
            SoundKind::Blip => 0.08,
            SoundKind::Chime => 0.6,
            SoundKind::Clink => 0.18,
            SoundKind::Thud => 0.22,
            SoundKind::Knock => 0.14,
            SoundKind::Crunch => 0.16,
            SoundKind::Swish => 0.22,
            SoundKind::Keys => 0.07,
            SoundKind::Piano => 0.7,
            SoundKind::Beep => 0.12,
            SoundKind::Drop => 0.14,
            SoundKind::Marimba => 0.4,
            SoundKind::Glass => 0.6,
            SoundKind::Pulse => 0.16,
            SoundKind::Rise | SoundKind::Fall => 0.26,
            SoundKind::Coin => 0.35,
            _ => unreachable!("recordings are timed by their samples"),
        }
    }
}

impl fmt::Display for SoundKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
