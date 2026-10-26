//! Which of the pool's buffers may be drawn into, apart from Wayland so the rules can be tested.
//!
//! A pool has room for `MOST` buffers of one size. A buffer is held from the commit that
//! attaches it until the compositor's `release` for it, and is never handed out for drawing
//! meanwhile. Each remembers the frame it last held, so a frame drawn into it knows its age:
//! how many frames behind the screen it is (step 4.3 repaints only what changed since then).
//!
//! When every buffer is held, the frame is skipped and drawn on the next release. Two buffers
//! cover a compositor that keeps the last one until the next is latched; a third covers one
//! that also holds the one it shows (scanout, or a frame still being copied). Beyond three the
//! compositor isn't taking frames at all (the output is off, or it is behind), so more buffers
//! would only cost memory. A buffer never drawn into costs nothing: the memfd's pages are made
//! only when touched, and the newest free buffer is always preferred, so the third is used only
//! when the first two are both held.

/// The most buffers a pool has.
pub(super) const MOST: usize = 3;

/// A free buffer, handed out to draw one frame into and commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Lease {
    /// Which buffer.
    pub(super) slot: usize,
    /// How many frames old its pixels are: 1 holds the last frame committed, 2 the one before,
    /// and so on. 0: nothing known (never drawn into), so everything has to be drawn.
    pub(super) age: u64,
}

/// The bytes of a pool: `MOST` buffers of `width`×`height`, four bytes a pixel, one after
/// another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) width: u32,
    pub(super) height: u32,
}

impl Layout {
    pub(super) fn stride(self) -> usize {
        self.width as usize * 4
    }

    /// One buffer's bytes.
    pub(super) fn len(self) -> usize {
        self.stride() * self.height as usize
    }

    /// Where buffer `slot` starts in the pool.
    pub(super) fn offset(self, slot: usize) -> usize {
        self.len() * slot
    }

    /// The whole pool's bytes.
    pub(super) fn pool_len(self) -> usize {
        self.len() * MOST
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Slot {
    held: bool,
    /// The number of the frame it last held.
    frame: Option<u64>,
}

#[derive(Debug, Default)]
pub(super) struct Slots {
    slots: [Slot; MOST],
    /// Frames committed from this pool.
    frames: u64,
}

impl Slots {
    /// The free buffer to draw the next frame into: the one holding the newest frame, so the
    /// least has to be repainted, else one never drawn into. None while all are held.
    pub(super) fn lease(&self) -> Option<Lease> {
        let free = self.slots.iter().enumerate().filter(|(_, s)| !s.held);
        let (slot, s) = free.max_by_key(|(i, s)| (s.frame, std::cmp::Reverse(*i)))?;
        let age = s.frame.map_or(0, |f| self.frames + 1 - f);
        Some(Lease { slot, age })
    }

    /// Whether a frame could be drawn now.
    pub(super) fn any_free(&self) -> bool {
        self.lease().is_some()
    }

    /// Checks that the leased buffer may be drawn into: the compositor doesn't hold it. A
    /// failure is a bug here, and drawing on would change what the compositor shows.
    pub(super) fn check_free(&self, lease: Lease) {
        assert!(!self.slots[lease.slot].held, "buffer {} is held by the compositor; it can't be drawn into", lease.slot);
    }

    /// The leased buffer has been attached and committed: the compositor holds it until it
    /// releases it.
    pub(super) fn committed(&mut self, lease: Lease) {
        self.check_free(lease);
        self.frames += 1;
        self.slots[lease.slot] = Slot { held: true, frame: Some(self.frames) };
    }

    /// The compositor released buffer `slot`.
    pub(super) fn released(&mut self, slot: usize) {
        if let Some(s) = self.slots.get_mut(slot) {
            s.held = false;
        }
    }

    /// How many buffers the compositor holds.
    pub(super) fn held(&self) -> usize {
        self.slots.iter().filter(|s| s.held).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_pool_hands_out_its_first_buffer_with_nothing_known() {
        let s = Slots::default();
        assert_eq!(s.lease(), Some(Lease { slot: 0, age: 0 }));
        assert_eq!(s.held(), 0);
    }

    #[test]
    fn a_held_buffer_is_never_handed_out_until_released() {
        let mut s = Slots::default();
        let a = s.lease().unwrap();
        s.committed(a);
        let b = s.lease().unwrap();
        assert_ne!(b.slot, a.slot);
        s.committed(b);
        let c = s.lease().unwrap();
        assert_eq!(c.slot, 2, "the third buffer only once the first two are held");
        s.committed(c);
        assert_eq!(s.lease(), None, "all held: the frame waits for a release");
        assert!(!s.any_free());
        s.released(b.slot);
        assert_eq!(s.lease().map(|l| l.slot), Some(b.slot));
    }

    #[test]
    fn two_buffers_take_turns_when_the_compositor_releases_promptly() {
        let mut s = Slots::default();
        let mut used = Vec::new();
        let mut last = None;
        for _ in 0..6 {
            let lease = s.lease().unwrap();
            s.committed(lease);
            // The compositor lets go of the one before once the new one is on.
            if let Some(prev) = last.replace(lease.slot) {
                s.released(prev);
            }
            used.push(lease.slot);
        }
        assert_eq!(used, [0, 1, 0, 1, 0, 1]);
    }

    #[test]
    fn the_age_counts_frames_since_the_buffer_was_drawn() {
        let mut s = Slots::default();
        let a = s.lease().unwrap();
        s.committed(a);
        s.released(a.slot);
        assert_eq!(s.lease(), Some(Lease { slot: a.slot, age: 1 }), "the newest free buffer holds the last frame");
        // Hold slot 0: the next frame goes into a fresh buffer, then they alternate.
        let a = s.lease().unwrap();
        s.committed(a);
        let b = s.lease().unwrap();
        assert_eq!(b, Lease { slot: 1, age: 0 });
        s.committed(b);
        s.released(a.slot);
        assert_eq!(s.lease(), Some(Lease { slot: a.slot, age: 2 }), "two frames behind");
        s.released(b.slot);
        assert_eq!(s.lease(), Some(Lease { slot: b.slot, age: 1 }), "the newest is preferred");
    }

    #[test]
    fn a_release_for_another_pool_is_ignored() {
        let mut s = Slots::default();
        s.released(MOST + 4);
        assert_eq!(s.held(), 0);
    }

    #[test]
    #[should_panic(expected = "held by the compositor")]
    fn committing_a_held_buffer_again_is_caught() {
        let mut s = Slots::default();
        let a = s.lease().unwrap();
        s.committed(a);
        s.check_free(a);
    }

    #[test]
    fn the_layout_puts_buffers_one_after_another() {
        let screen = Layout { width: 1920, height: 1080 };
        assert_eq!(screen.stride(), 7680);
        assert_eq!(screen.len(), 7680 * 1080);
        assert_eq!(screen.offset(2), 2 * 7680 * 1080);
        assert_eq!(screen.pool_len(), 3 * 7680 * 1080);
        // While hidden the pool is three pixels.
        assert_eq!(Layout { width: 1, height: 1 }.pool_len(), 12);
    }
}
