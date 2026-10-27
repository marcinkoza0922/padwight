//! When to draw, apart from Wayland so the rules can be tested: frames go out on the
//! compositor's frame callbacks while anything moves, and not at all while everything is still.
//!
//! Every frame drawn asks for a callback in its own commit. Until that callback is done, nothing
//! more is drawn: a daemon frame or an animation step waits for it, and is drawn as it fires,
//! with the animation clock read then. With no callback outstanding (the overlay was still, or
//! the frame before was skipped because every buffer was held), a change is drawn at once: the
//! compositor has already shown the last commit, so there is nothing to throttle against, and
//! asking for a callback first would cost a refresh of latency (and a commit without a buffer may
//! not even get one, as the compositor may not repaint for it). The first frame after a configure
//! is drawn at once too (by the caller), since nothing is on at that size yet.
//!
//! A callback can stop coming: the output is off, or the surface is hidden behind something.
//! After `FALLBACK` it is taken as lost, and the next frame goes out without it, so fades go on,
//! what has closed is pruned, and the surface still shrinks to 1×1 after the last one.
//!
//! A frame can also come out the same as the one on screen (a fade waiting out its delay): it
//! isn't committed, so no callback comes for it. While something moves, the next look is then
//! `RETRY` later, about a refresh, rather than at once (a busy loop) or after the fallback (a
//! stall).

use std::time::{Duration, Instant};

/// How long an unanswered callback is waited for before drawing without it.
pub(super) const FALLBACK: Duration = Duration::from_millis(100);
/// How long after a frame that showed nothing new the next one is looked at, while anything
/// moves: about a refresh at 60 Hz.
pub(super) const RETRY: Duration = Duration::from_millis(16);
/// The callback asked for with the last frame drawn.
#[derive(Debug, Clone, Copy)]
struct Pending {
    id: u64,
    asked: Instant,
}

#[derive(Debug, Default)]
pub(super) struct Pacer {
    pending: Option<Pending>,
    /// The id the last callback asked for got: a callback done for an older one (taken as lost,
    /// or asked before a configure's frame) is passed over.
    asked: u64,
    /// Something changed that isn't on screen yet: a daemon frame, a frame skipped because
    /// every buffer was held, or an animation that has just come to rest.
    dirty: bool,
    /// Whether something was moving at the last look.
    moving: bool,
    /// When the last look found nothing new to show, since the last frame committed.
    quiet: Option<Instant>,
}

impl Pacer {
    /// The daemon sent a frame.
    pub(super) fn changed(&mut self) {
        self.dirty = true;
    }

    /// What moves at the loop's latest look. When everything has just come to rest, one more
    /// frame shows where it rests: the last one drawn while moving was a step before that.
    pub(super) fn moving(&mut self, busy: bool) {
        if self.moving && !busy {
            self.dirty = true;
        }
        self.moving = busy;
    }

    /// Whether a frame is due at `now`: something changed or moves, and no callback is
    /// outstanding, or the one that is has been waited for too long.
    /// After a look that found nothing new, what moves waits `RETRY` for the next.
    pub(super) fn due(&self, now: Instant) -> bool {
        let free = self.pending.is_none_or(|p| now.duration_since(p.asked) >= FALLBACK);
        let rested = self.quiet.is_none_or(|t| now.duration_since(t) >= RETRY);
        free && (self.dirty || (self.moving && rested))
    }

    /// A frame has been committed at `now`: returns the id of the callback to ask for in the
    /// same commit.
    pub(super) fn drawn(&mut self, now: Instant) -> u64 {
        self.dirty = false;
        self.quiet = None;
        self.asked += 1;
        self.pending = Some(Pending { id: self.asked, asked: now });
        self.asked
    }

    /// A frame skipped because every buffer was held: drawn once one is released (or the
    /// fallback runs out).
    pub(super) fn skipped(&mut self) {
        self.dirty = true;
    }

    /// A frame drawn at `now` came out the same as the one on screen, so it wasn't committed and
    /// no callback comes for it.
    pub(super) fn unchanged(&mut self, now: Instant) {
        self.dirty = false;
        self.quiet = Some(now);
    }

    /// The callback `id` is done: returns whether it was the one waited for.
    pub(super) fn done(&mut self, id: u64) -> bool {
        let current = self.pending.is_some_and(|p| p.id == id);
        if current {
            self.pending = None;
        }
        current
    }

    /// The surface is going back to 1×1: no callbacks are waited for, and nothing is drawn
    /// until the next configure for the screen.
    pub(super) fn hidden(&mut self) {
        self.pending = None;
        self.dirty = false;
        self.moving = false;
        self.quiet = None;
    }

    /// How long the loop may sleep at `now` before it has to look again: while something waits
    /// to be drawn, no longer than until the fallback is due (a callback, a buffer release or a
    /// frame from the daemon wakes it sooner). Otherwise, and while hidden, nothing needs looking
    /// at: it sleeps until something comes.
    pub(super) fn timeout(&self, now: Instant, shown: bool) -> Option<Duration> {
        if shown { self.fallback_in(now) } else { None }
    }

    /// How long until the fallback is due, while something waits to be drawn. Without a callback
    /// outstanding, what is waited for is a released buffer or the configure.
    /// After a frame that showed nothing new, what moves is looked at again after `RETRY`.
    fn fallback_in(&self, now: Instant) -> Option<Duration> {
        if !(self.dirty || self.moving) {
            return None;
        }
        Some(match (self.pending, self.quiet) {
            (Some(p), _) => (p.asked + FALLBACK).saturating_duration_since(now),
            (None, Some(t)) if !self.dirty => (t + RETRY).saturating_duration_since(now),
            (None, _) => FALLBACK,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn a_change_while_still_is_drawn_at_once() {
        let mut p = Pacer::default();
        let t = Instant::now();
        assert!(!p.due(t), "nothing changed");
        p.changed();
        assert!(p.due(t), "no callback outstanding: nothing to wait for");
        assert_eq!(p.drawn(t), 1);
        assert!(!p.due(t), "drawn");
    }

    #[test]
    fn while_moving_one_frame_per_callback() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.moving(true);
        assert!(p.due(t));
        let id = p.drawn(t);
        p.moving(true);
        assert!(!p.due(t + 5 * MS), "waits for the callback");
        p.changed();
        assert!(!p.due(t + 8 * MS), "a daemon frame waits for it too");
        assert!(p.done(id));
        assert!(p.due(t + 16 * MS), "drawn as the callback fires");
        let next = p.drawn(t + 16 * MS);
        assert!(!p.due(t + 17 * MS));
        assert!(p.done(next));
        p.moving(true);
        assert!(p.due(t + 33 * MS), "still moving: the next one");
    }

    #[test]
    fn nothing_is_drawn_while_still() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.changed();
        let id = p.drawn(t);
        assert!(p.done(id));
        p.moving(false);
        assert!(!p.due(t + 16 * MS), "the callback came, but nothing changed");
        assert!(!p.due(t + 10_000 * MS), "not even after a long while");
        assert_eq!(p.fallback_in(t + 16 * MS), None, "nothing to wake for");
        assert_eq!(p.timeout(t + 16 * MS, true), None, "a still overlay sleeps until something comes");
    }

    #[test]
    fn coming_to_rest_draws_one_more_frame() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.moving(true);
        let id = p.drawn(t);
        p.done(id);
        p.moving(false);
        assert!(p.due(t + 16 * MS), "the frame where it rests");
        let id = p.drawn(t + 16 * MS);
        p.done(id);
        p.moving(false);
        assert!(!p.due(t + 33 * MS));
    }

    #[test]
    fn a_lost_callback_is_given_up_on_after_the_fallback() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.moving(true);
        let lost = p.drawn(t);
        assert_eq!(p.fallback_in(t + 40 * MS), Some(60 * MS));
        assert_eq!(p.timeout(t + 40 * MS, true), Some(60 * MS), "no longer than until the fallback");
        assert!(!p.due(t + FALLBACK - MS));
        assert!(p.due(t + FALLBACK), "fades and closing go on without it");
        let next = p.drawn(t + FALLBACK);
        assert!(!p.done(lost), "a late answer to the lost one is passed over");
        assert!(!p.due(t + FALLBACK + 5 * MS), "still waiting for the new one");
        assert!(p.done(next));
    }

    #[test]
    fn a_skipped_frame_goes_out_when_a_buffer_comes_back() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.changed();
        let id = p.drawn(t);
        p.done(id);
        p.changed();
        // Every buffer held: the caller skipped it, so no callback was asked for.
        p.skipped();
        assert!(p.due(t + 16 * MS), "drawn on the next look that finds a buffer free");
        assert_eq!(p.fallback_in(t + 16 * MS), Some(FALLBACK), "the release, or the fallback, wakes the loop");
    }

    #[test]
    fn a_frame_with_nothing_new_waits_a_refresh_and_expects_no_callback() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.moving(true);
        p.changed();
        assert!(p.due(t));
        // Painted, but the same as what is on: not committed, no callback asked for.
        p.unchanged(t);
        p.moving(true);
        assert!(!p.due(t + MS), "not again at once: that would spin");
        assert_eq!(p.timeout(t + MS, true), Some(RETRY - MS), "woken a refresh on, not after the fallback");
        assert!(p.due(t + RETRY), "looked at again a refresh on");
        p.changed();
        assert!(p.due(t + 2 * MS), "a daemon frame goes at once all the same");
        let id = p.drawn(t + 2 * MS);
        assert!(p.done(id));
        p.moving(true);
        assert!(p.due(t + 3 * MS), "a committed frame ends the wait");
        // At rest, an unchanged frame leaves nothing to wake for.
        p.moving(false);
        p.unchanged(t + 4 * MS);
        p.moving(false);
        assert!(!p.due(t + 100 * MS));
        assert_eq!(p.timeout(t + 5 * MS, true), None);
    }

    #[test]
    fn hidden_waits_for_nothing() {
        let mut p = Pacer::default();
        let t = Instant::now();
        p.moving(true);
        p.drawn(t);
        p.hidden();
        assert!(!p.due(t + FALLBACK));
        assert_eq!(p.fallback_in(t), None);
        assert_eq!(p.timeout(t, false), None);
        // A frame from the daemon while hidden draws nothing and wakes nothing either: it sleeps.
        p.changed();
        assert_eq!(p.timeout(t, false), None);
    }
}
