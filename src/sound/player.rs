//! Plays cues on their own thread, opening the audio device only while they sound.

use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::Duration,
};

use super::{RATE, SoundSpec};
use crate::monitor::log;

/// The audio device closes after this long without a cue. Longer than any cue, so none is cut off.
const CLOSE_AFTER: Duration = Duration::from_secs(30);

/// Plays cues on a background thread. While no audio device opens, cues are dropped.
pub struct Sounds {
    tx: Option<Sender<SoundSpec>>,
}

impl Sounds {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<SoundSpec>();
        let spawned = thread::Builder::new().name("padwight-sound".into()).spawn(move || {
            let open = || match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut sink) => {
                    // Closing it is deliberate, and every cue has finished by then.
                    sink.log_on_drop(false);
                    Ok(sink)
                }
                Err(e) => Err(e.to_string()),
            };
            serve(&rx, CLOSE_AFTER, open, |sink, spec| {
                let player = rodio::Player::connect_new(sink.mixer());
                player.append(rodio::buffer::SamplesBuffer::new(MONO, RATE_NZ, spec.samples()));
                player.detach();
            });
        });
        Sounds { tx: spawned.ok().map(|_| tx) }
    }

    /// Plays `spec` now, over any cue still sounding.
    pub fn play(&self, spec: &SoundSpec) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(*spec);
        }
    }
}

/// Plays each cue from `rx` on a device `open` gives, until every sender is gone. A cue opens the
/// device, and it closes again after `close_after` without one: an open output stream mixes
/// silence all the time (about 1% of a core; see tests/profile/sound.rs), and opening one takes
/// only tens of milliseconds. A cue that finds no device (none plugged in, the sound server
/// restarting) is dropped, and the next one tries again.
fn serve<D>(rx: &Receiver<SoundSpec>, close_after: Duration, mut open: impl FnMut() -> Result<D, String>, mut play: impl FnMut(&D, &SoundSpec)) {
    let mut reported = false;
    while let Ok(first) = rx.recv() {
        let device = match open() {
            Ok(device) => device,
            Err(e) => {
                if !reported {
                    log!("no audio device for overlay sounds: {e}");
                    reported = true;
                }
                continue;
            }
        };
        let mut next = Ok(first);
        while let Ok(spec) = next {
            play(&device, &spec);
            next = rx.recv_timeout(close_after);
        }
        if next == Err(RecvTimeoutError::Disconnected) {
            return;
        }
    }
}

/// [`RATE`] and mono as the types the audio library takes.
const RATE_NZ: std::num::NonZero<u32> = std::num::NonZero::<u32>::new(RATE).unwrap();
const MONO: std::num::NonZero<u16> = std::num::NonZero::<u16>::MIN;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::SoundKind;

    #[test]
    fn a_device_that_won_t_open_drops_that_cue_and_the_next_tries_again() {
        let (tx, rx) = mpsc::channel();
        let (click, pop) = (SoundSpec::new(SoundKind::Click, 0.5, 1.0), SoundSpec::new(SoundKind::Pop, 0.5, 1.0));
        tx.send(click).unwrap();
        tx.send(pop).unwrap();
        drop(tx);
        let mut opens = 0;
        let mut played = Vec::new();
        let open = || {
            opens += 1;
            if opens == 1 { Err("no device".to_string()) } else { Ok(()) }
        };
        serve(&rx, Duration::from_millis(10), open, |(), spec| played.push(spec.kind));
        assert_eq!(played, [SoundKind::Pop]);
    }
}
