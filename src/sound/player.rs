//! Plays cues on their own thread, opening the audio device only while they sound.

use std::{
    sync::mpsc::{self, Sender},
    thread,
};

use super::{RATE, SoundSpec};

/// The audio device closes after this long without a cue. Longer than any cue, so none is cut off.
const CLOSE_AFTER: std::time::Duration = std::time::Duration::from_secs(30);

/// Plays cues on a background thread. If no audio device opens, cues are dropped silently.
pub struct Sounds {
    tx: Option<Sender<SoundSpec>>,
}

impl Sounds {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<SoundSpec>();
        let spawned = thread::Builder::new().name("padwight-sound".into()).spawn(move || {
            // A cue opens the device, and it closes again after a quiet spell: an open output
            // stream mixes silence all the time (about 1% of a core; see tests/profile/sound.rs),
            // and opening one takes only tens of milliseconds.
            while let Ok(first) = rx.recv() {
                let Ok(mut sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
                    return;
                };
                // Closing it is deliberate, and every cue has finished by then.
                sink.log_on_drop(false);
                let mut next = Ok(first);
                while let Ok(spec) = next {
                    let player = rodio::Player::connect_new(sink.mixer());
                    player.append(rodio::buffer::SamplesBuffer::new(MONO, RATE_NZ, spec.samples()));
                    player.detach();
                    next = rx.recv_timeout(CLOSE_AFTER);
                }
                if next == Err(mpsc::RecvTimeoutError::Disconnected) {
                    return;
                }
            }
        });
        Sounds { tx: spawned.ok().map(|_| tx) }
    }

    /// Plays `spec` now. Cues queue up behind each other rather than overlapping.
    pub fn play(&self, spec: &SoundSpec) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(*spec);
        }
    }
}

/// [`RATE`] and mono as the types the audio library takes.
const RATE_NZ: std::num::NonZero<u32> = std::num::NonZero::<u32>::new(RATE).unwrap();
const MONO: std::num::NonZero<u16> = std::num::NonZero::<u16>::MIN;
