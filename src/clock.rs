//! The time the daemon runs on. Normally the system's monotonic clock; a `--debug` daemon started
//! with `--fake-clock` runs on a manual one that only moves when a debug session says `advance`,
//! so a test sees every timer, tick and window fire at an exact time, with no real waiting.

use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// The manual clock: where it started, and how far it has been moved since, in nanoseconds.
struct Manual {
    start: Instant,
    moved: AtomicU64,
}

static MANUAL: OnceLock<Manual> = OnceLock::new();

/// Makes the clock manual, for the rest of the process. Time stands still until [`advance`].
pub fn use_manual() {
    let _ = MANUAL.set(Manual { start: Instant::now(), moved: AtomicU64::new(0) });
}

/// Whether the clock is manual.
pub fn is_manual() -> bool {
    MANUAL.get().is_some()
}

/// The current time: the system's, or the manual clock's.
pub fn now() -> Instant {
    match MANUAL.get() {
        Some(m) => m.start + Duration::from_nanos(m.moved.load(Ordering::Relaxed)),
        None => Instant::now(),
    }
}

/// How long ago `t` was.
pub fn since(t: Instant) -> Duration {
    now().saturating_duration_since(t)
}

/// Moves the manual clock to `to`, never backwards. Does nothing on the system clock.
pub fn set(to: Instant) {
    if let Some(m) = MANUAL.get() {
        m.moved.fetch_max(to.saturating_duration_since(m.start).as_nanos() as u64, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_runs_by_itself() {
        let a = now();
        assert!(now() >= a);
        assert!(!is_manual());
        set(a + Duration::from_secs(1));
        assert!(now() < a + Duration::from_secs(1), "setting does nothing on the system clock");
    }
}
