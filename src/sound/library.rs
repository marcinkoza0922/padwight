//! Cues that are samples rather than synthesis: short CC0 files bundled in `assets/sounds`
//! (see the README there for where each came from), decoded once on first use.

use std::sync::OnceLock;

use super::{SoundKind, wav::decode_wav};

/// The WAV file a kind plays, if it is a recording.
fn bytes(kind: SoundKind) -> Option<&'static [u8]> {
    macro_rules! wav {
        ($name:literal) => {
            Some(include_bytes!(concat!("../../assets/sounds/", $name, ".wav")).as_slice())
        };
    }
    match kind {
        SoundKind::Typewriter => wav!("typewriter"),
        SoundKind::Bong => wav!("bong"),
        SoundKind::Select => wav!("select"),
        SoundKind::Toggle => wav!("toggle"),
        SoundKind::Pluck => wav!("pluck"),
        SoundKind::Confirm => wav!("confirm"),
        SoundKind::Error => wav!("error"),
        SoundKind::Glitch => wav!("glitch"),
        SoundKind::Rollover => wav!("rollover"),
        SoundKind::Switch => wav!("switch"),
        SoundKind::Latch => wav!("latch"),
        SoundKind::Page => wav!("page"),
        SoundKind::Wood => wav!("wood"),
        SoundKind::Plate => wav!("plate"),
        SoundKind::Tin => wav!("tin"),
        SoundKind::Bell => wav!("bell"),
        SoundKind::Geiger => wav!("geiger"),
        SoundKind::Nano => wav!("nano"),
        SoundKind::Radio => wav!("radio"),
        SoundKind::Heartbeat => wav!("heartbeat"),
        SoundKind::Casing => wav!("casing"),
        SoundKind::Lockpick => wav!("lockpick"),
        SoundKind::Shing => wav!("shing"),
        SoundKind::Fault => wav!("fault"),
        SoundKind::Sand => wav!("sand"),
        SoundKind::Boing => wav!("boing"),
        SoundKind::Shutter => wav!("shutter"),
        SoundKind::Zap => wav!("zap"),
        SoundKind::Purr => wav!("purr"),
        SoundKind::Clank => wav!("clank"),
        SoundKind::Card => wav!("card"),
        _ => None,
    }
}

/// The samples of a recorded kind, mono at the cue rate and at the file's own level.
pub(super) fn samples(kind: SoundKind) -> Option<&'static [f32]> {
    static DECODED: OnceLock<Vec<(SoundKind, Vec<f32>)>> = OnceLock::new();
    let all = DECODED.get_or_init(|| SoundKind::ALL.into_iter().filter_map(|k| Some((k, decode_wav(bytes(k)?).expect("bundled sounds are valid WAV files")))).collect());
    all.iter().find(|(k, _)| *k == kind).map(|(_, s)| s.as_slice())
}
