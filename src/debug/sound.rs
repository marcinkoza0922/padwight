//! `padwight debug sound`: plays overlay cues from the command line, to hear them while tuning
//! them. It needs no session or daemon, only an audio device.

use std::{thread::sleep, time::Duration};

use anyhow::{Context, Result, bail};

use crate::sound::{Feedback, SoundKind, SoundOverlay, SoundSet, SoundSpec, Sounds};

pub const USAGE: &str = "  sound [what]... [volume=v] [pitch=p] [length=l] [bass=db] [treble=db] [drive=d] [echo=e] [reverb=r]
                                 play every kind of cue in turn, or each kind or overlay named:
                                 an overlay's default cues play as a visit to it does (open, a
                                 few steps, a pick, close); the settings change the kinds only
  sound list                     list the kinds and the overlays
";

/// Silence between two kinds, so each is heard on its own.
const GAP: Duration = Duration::from_millis(350);
/// Between steps in an overlay, as a held direction repeats.
const STEP: Duration = Duration::from_millis(150);
/// How loud a kind plays when no volume is given: louder than any overlay's default, to hear it clearly.
const VOLUME: f32 = 0.5;

/// One thing to play: a kind as it is set, or an overlay's cues.
enum What {
    Kind(SoundKind),
    Overlay(SoundOverlay),
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    if args == ["list"] {
        println!("kinds: {}", SoundKind::ALL.map(name).join(" "));
        println!("overlays: {}", SoundOverlay::ALL.map(SoundOverlay::key).join(" "));
        return Ok(());
    }
    let mut spec = SoundSpec::new(SoundKind::default(), VOLUME, 1.0);
    let mut whats = Vec::new();
    for arg in args {
        match arg.split_once('=') {
            Some((key, value)) => {
                let value: f32 = value.parse().with_context(|| format!("{key} takes a number"))?;
                match key {
                    "volume" => spec.volume = value,
                    "pitch" => spec.pitch = value,
                    "length" => spec.length = value,
                    "bass" => spec.bass = value,
                    "treble" => spec.treble = value,
                    "drive" => spec.drive = value,
                    "echo" => spec.echo = value,
                    "reverb" => spec.reverb = value,
                    _ => bail!("unknown setting {key} (volume, pitch, length, bass, treble, drive, echo or reverb)"),
                }
            }
            None => whats.push(parse(arg)?),
        }
    }
    if whats.is_empty() {
        whats = SoundKind::ALL.map(What::Kind).into();
    }
    let sounds = Sounds::start();
    for what in whats {
        match what {
            What::Kind(kind) => play_kind(&sounds, SoundSpec { kind, ..spec }),
            What::Overlay(overlay) => visit(&sounds, overlay),
        }
    }
    Ok(())
}

fn parse(word: &str) -> Result<What> {
    if let Some(kind) = SoundKind::ALL.into_iter().find(|k| name(*k) == word) {
        return Ok(What::Kind(kind));
    }
    if let Some(overlay) = SoundOverlay::ALL.into_iter().find(|o| o.key() == word) {
        return Ok(What::Overlay(overlay));
    }
    bail!("no kind or overlay named {word} (see `padwight debug sound list`)")
}

/// A kind as a pack file names it.
fn name(kind: SoundKind) -> String {
    kind.label().to_lowercase()
}

/// Names a kind and plays it, and waits for it to finish.
#[expect(clippy::print_stdout, reason = "CLI output")]
pub(super) fn play_kind(sounds: &Sounds, spec: SoundSpec) {
    println!("{}", name(spec.kind));
    play(sounds, spec);
}

/// Plays one cue and waits for it to finish, and a little more.
fn play(sounds: &Sounds, spec: SoundSpec) {
    sounds.play(&spec);
    sleep(spec.duration() + GAP);
}

/// Plays an overlay's cues the way a visit does: it opens, the cursor steps a few times,
/// something is picked, and it closes.
#[expect(clippy::print_stdout, reason = "CLI output")]
pub(super) fn visit(sounds: &Sounds, overlay: SoundOverlay) {
    let set = SoundSet::default();
    let cue = |feedback: Feedback| {
        let spec = set.cue(overlay, feedback).expect("the default sounds are on");
        println!("{} {feedback:?}: {} volume={} pitch={} length={}", overlay.key(), name(spec.kind), spec.volume, spec.pitch, spec.length);
        spec
    };
    let open = cue(Feedback::Open);
    sounds.play(&open);
    sleep(open.duration().max(STEP * 2));
    let step = cue(Feedback::Step);
    for _ in 0..4 {
        sounds.play(&step);
        sleep(STEP);
    }
    sleep(STEP);
    let pick = cue(Feedback::Pick);
    sounds.play(&pick);
    sleep(pick.duration().max(STEP * 3));
    play(sounds, cue(Feedback::Close));
}
