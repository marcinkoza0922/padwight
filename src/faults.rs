//! Fault injection: named points in the code that can be made to fail, hang, close a socket or
//! kill the process, so the error paths run in tests instead of waiting for a full disk.
//!
//! A point is one line at the call site, `faults::io("save.rename")?;` or
//! `if faults::hit("ipc.reply") == Some(Action::Close) { return Ok(()); }`. What a point does comes
//! from a spec, a comma-separated list of `point=action`:
//!
//! - `action` is `fail`, `hang` (blocks the thread for good), `close` (the call site drops its
//!   connection) or `crash` (aborts the process on the spot, with no cleanup).
//! - `+K` after the action skips the first K hits: `save.write=fail+2` lets two writes through.
//! - `:N/M` makes the first N hits of every M fire: `uinput.write=fail:1/50` refuses one write in 50.
//! - A point ending in `*` matches every point with that prefix: `save.*=fail`.
//!
//! The daemon reads the spec from `PADWIGHT_FAULTS`, only when started with `--debug`; tests set it
//! with [`set`]. Release builds compile the points out entirely (they are empty inline functions),
//! so a shipped daemon ignores the variable whatever the flag says. The gate is `debug_assertions`
//! rather than a cargo feature because `cargo nextest` and the dev build get it with no extra flags,
//! and a release build can't turn it on by accident; `--debug` is only the runtime switch.

#[cfg(any(test, debug_assertions))]
mod live;
#[cfg(any(test, debug_assertions))]
pub use live::*;

/// What a point does when it fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Fail,
    Hang,
    Close,
    Crash,
}

#[cfg(not(any(test, debug_assertions)))]
mod off {
    use std::io;

    /// Compiled out: nothing fires.
    #[inline(always)]
    pub fn hit(_point: &str) -> Option<super::Action> {
        None
    }

    #[inline(always)]
    pub fn io(_point: &str) -> io::Result<()> {
        Ok(())
    }

    #[inline(always)]
    pub fn arm() {}
}
#[cfg(not(any(test, debug_assertions)))]
pub use off::*;

#[cfg(test)]
mod tests;
