//! Forwards rumble a game sends to our virtual pad to the physical controller, on Windows.
//!
//! ViGEmBus tells us each time a game sets the virtual pad's motors; we set the physical
//! controller's XInput slot to the same, at the configured strength.

use std::{
    path::PathBuf,
    pin::pin,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread,
    time::Duration,
};

use crate::{
    config::rumble_cues::RumbleCue,
    input::xinput::{self, set_rumble},
    monitor::log,
    output::Target,
};

/// How strongly rumble plays, in hundredths of what the game asks for.
static STRENGTH: AtomicU32 = AtomicU32::new(100);

/// How often the forwarding thread checks for a new request when none has come.
const POLL: Duration = Duration::from_millis(4);

/// Sets how strongly rumble plays, from 0 (none) to 1 (as the game asks).
pub fn set_strength(strength: f32) {
    STRENGTH.store((strength.clamp(0.0, 1.0) * 100.0).round() as u32, Ordering::Relaxed);
}

/// A motor speed at the set strength.
fn at_strength(speed: u16) -> u16 {
    (u32::from(speed) * STRENGTH.load(Ordering::Relaxed) / 100) as u16
}

/// Mirrors rumble from `pad` onto the controller at `physical` until `stop` is set, then stops
/// its motors.
pub fn spawn(pad: Option<Arc<Mutex<Target>>>, physical: PathBuf, stop: Arc<AtomicBool>) {
    let (Some(pad), Some(slot)) = (pad, xinput::slot_of(&physical)) else { return };
    thread::spawn(move || {
        let request = pad.lock().unwrap_or_else(PoisonError::into_inner).request_notification();
        let request = match request {
            Ok(request) => request,
            Err(e) => {
                log!("rumble forwarding for {} stopped: {e}", physical.display());
                return;
            }
        };
        let mut request = pin!(request);
        'requests: while !stop.load(Ordering::Relaxed) {
            request.as_mut().request();
            loop {
                match request.as_mut().poll(false) {
                    Ok(Some(motors)) => {
                        // XInput's motors are 0..65535; the notification's 0..255.
                        let speed = |m: u8| at_strength(u16::from(m) * 257);
                        set_rumble(slot, speed(motors.large_motor), speed(motors.small_motor));
                        continue 'requests;
                    }
                    Ok(None) if !stop.load(Ordering::Relaxed) => thread::sleep(POLL),
                    // Stopped, or the virtual pad went away.
                    Ok(None) | Err(_) => break 'requests,
                }
            }
        }
        set_rumble(slot, 0, 0);
    });
}

/// Plays `pattern` (strong motor, weak motor, how long) on the controller at `path` in the
/// background, then stops its motors.
fn play(path: PathBuf, pattern: Vec<(u16, u16, Duration)>) {
    let Some(slot) = xinput::slot_of(&path) else { return };
    thread::spawn(move || {
        for (strong, weak, time) in pattern {
            if !set_rumble(slot, strong, weak) {
                log!("rumble on {} failed: the controller is gone", path.display());
                return;
            }
            thread::sleep(time);
        }
        set_rumble(slot, 0, 0);
    });
}

const PULSE: Duration = Duration::from_millis(300);
const PAUSE: Duration = Duration::from_millis(150);
const TICK: Duration = Duration::from_millis(40);
/// How long a pick's pulse lasts.
const PICK: Duration = Duration::from_millis(25);

/// Plays a short test pattern: a strong-motor pulse, then a weak-motor pulse, so both motors can
/// be felt.
pub fn test(path: PathBuf) {
    play(path, vec![(0xc000, 0, PULSE), (0, 0, PAUSE), (0, 0xffff, PULSE)]);
}

/// Plays a short tick as gyro is switched on (one firm tick) or off (two light ones).
pub fn tick(path: PathBuf, on: bool) {
    let pattern = if on {
        vec![(0x8000, 0, TICK)]
    } else {
        vec![(0, 0x9000, TICK), (0, 0, TICK), (0, 0x9000, TICK)]
    };
    play(path, pattern);
}

/// Plays one of an overlay's buzzes (a rumble cue, see `config::RumbleCue`) on the controller
/// at `path`, in the background. It follows the rumble strength. The weak motor carries the
/// cue's strength, and the strong one half of it.
pub fn cue(path: PathBuf, cue: RumbleCue) {
    let weak = (cue.strength.clamp(0.0, 1.0) * f32::from(u16::MAX)) as u16;
    play(path, vec![(at_strength(weak / 2), at_strength(weak), Duration::from_millis(u64::from(cue.length_ms)))]);
}

/// Plays a faint pulse as an overlay's item is picked. It follows the rumble strength.
pub fn pick(path: PathBuf) {
    play(path, vec![(0, at_strength(0x6000), PICK)]);
}
