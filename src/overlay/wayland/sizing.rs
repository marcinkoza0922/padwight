//! The layer surface's size, as a state machine apart from Wayland, so its rules can be tested:
//! the size only ever comes from a configure the compositor sent and we acked, and a buffer is
//! attached only at that size.
//!
//! The surface is 1×1 in a corner while hidden. To show, it asks for the whole screen (all four
//! edges anchored, size 0×0) and draws nothing until the configure with the screen's size has
//! come. To hide, it commits a clear screen with the request for 1×1, then attaches a 1×1 buffer
//! once that configure has come.

/// What the surface last asked the compositor to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    /// 1×1 in the top right corner.
    Corner,
    /// The whole screen: all edges anchored, size 0×0 so the compositor picks it.
    Screen,
}

impl Target {
    /// The size the surface asks for: 0 lets the compositor choose.
    pub(super) fn asked(self) -> (u32, u32) {
        match self {
            Target::Corner => (1, 1),
            Target::Screen => (0, 0),
        }
    }

    /// Whether a configure of `size` answers this target, and not an earlier request
    /// (configures sent before the compositor saw our last commit can still arrive after it).
    fn fits(self, size: (u32, u32)) -> bool {
        match self {
            Target::Corner => size == (1, 1),
            // Anchored to all four edges, the compositor has to give both sides.
            Target::Screen => size.0 > 1 && size.1 > 1,
        }
    }
}

/// What the caller has to commit after a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Commit {
    /// Nothing to do.
    Nothing,
    /// Set the anchors and size for `Target::Screen`, and commit without a new buffer.
    Grow,
    /// Set the anchors and size for `Target::Corner`. With `clear`, attach a clear buffer of
    /// that size (the size the surface has now) in the same commit, so whatever was last drawn
    /// doesn't linger until the compositor shrinks the surface.
    Shrink { clear: Option<(u32, u32)> },
    /// Attach a clear 1×1 buffer and commit: the surface is hidden.
    Blank,
    /// The screen has its size: draw it at this size and commit.
    Draw((u32, u32)),
}

#[derive(Debug)]
pub(super) struct Sizing {
    target: Target,
    /// The size from the last configure acked: the only size a buffer may be attached at
    /// (until the next configure is acked).
    acked: Option<(u32, u32)>,
    /// Whether `acked` answers `target`.
    fits: bool,
    /// The size of the buffer last attached, if any.
    attached: Option<(u32, u32)>,
}

impl Sizing {
    /// A new surface: it asked for the corner in its first commit, which had no buffer.
    pub(super) fn new() -> Sizing {
        Sizing { target: Target::Corner, acked: None, fits: false, attached: None }
    }

    pub(super) fn target(&self) -> Target {
        self.target
    }

    /// The size the screen can be drawn at, once its configure has come.
    pub(super) fn screen(&self) -> Option<(u32, u32)> {
        self.acked.filter(|_| self.target == Target::Screen && self.fits)
    }

    /// Whether a buffer of `size` may be attached now: only at the size last acked.
    pub(super) fn may_attach(&self, size: (u32, u32)) -> bool {
        self.acked == Some(size)
    }

    /// The overlay wants to show (or hide): asks for the screen (or the corner) when that
    /// differs from what was asked last.
    pub(super) fn want(&mut self, shown: bool) -> Commit {
        let target = if shown { Target::Screen } else { Target::Corner };
        if target == self.target {
            return Commit::Nothing;
        }
        self.target = target;
        self.fits = false;
        match target {
            Target::Screen => Commit::Grow,
            // Something bigger than the corner is on: clear it, at the size last acked, which
            // holds until the compositor answers this request.
            Target::Corner => Commit::Shrink { clear: self.acked.filter(|_| self.attached.is_some_and(|a| a != (1, 1))) },
        }
    }

    /// A configure has come and been acked: takes its size, and says what to commit when it
    /// answers what was asked. A zero side leaves that side to us: the size asked for.
    pub(super) fn configure(&mut self, width: u32, height: u32) -> Commit {
        let (asked_w, asked_h) = self.target.asked();
        let size = (if width == 0 { asked_w.max(1) } else { width }, if height == 0 { asked_h.max(1) } else { height });
        let repeat = self.fits && self.acked == Some(size);
        self.acked = Some(size);
        self.fits = self.target.fits(size);
        if !self.fits || (repeat && self.attached == Some(size)) || (self.target == Target::Corner && self.attached == Some(size)) {
            return Commit::Nothing;
        }
        match self.target {
            Target::Corner => Commit::Blank,
            Target::Screen => Commit::Draw(size),
        }
    }

    /// A buffer of `size` has been attached and committed.
    pub(super) fn attached(&mut self, size: (u32, u32)) {
        debug_assert!(self.may_attach(size), "a {size:?} buffer attached, but the configure acked was {:?}", self.acked);
        self.attached = Some(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: (u32, u32) = (1920, 1080);

    /// A surface that has been shown once, and is now on the screen at `SCREEN`.
    fn shown() -> Sizing {
        let mut s = Sizing::new();
        assert_eq!(s.configure(1, 1), Commit::Blank);
        s.attached((1, 1));
        assert_eq!(s.want(true), Commit::Grow);
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Draw(SCREEN));
        s.attached(SCREEN);
        s
    }

    #[test]
    fn nothing_is_drawn_before_the_first_configure() {
        let mut s = Sizing::new();
        assert_eq!(s.screen(), None);
        assert!(!s.may_attach((1, 1)));
        assert_eq!(s.want(true), Commit::Grow);
        assert_eq!(s.screen(), None, "asked for the screen, but no configure yet");
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Draw(SCREEN));
        assert_eq!(s.screen(), Some(SCREEN));
    }

    #[test]
    fn the_first_configure_hides_with_a_1x1_buffer() {
        let mut s = Sizing::new();
        assert_eq!(s.configure(1, 1), Commit::Blank);
        assert!(s.may_attach((1, 1)));
        s.attached((1, 1));
        assert_eq!(s.configure(1, 1), Commit::Nothing, "already blank");
        // A compositor may leave a side to us: it is the size we asked for.
        let mut s = Sizing::new();
        assert_eq!(s.configure(0, 0), Commit::Blank);
    }

    #[test]
    fn showing_waits_for_the_screens_configure() {
        let mut s = Sizing::new();
        s.configure(1, 1);
        s.attached((1, 1));
        assert_eq!(s.want(true), Commit::Grow);
        assert_eq!(s.want(true), Commit::Nothing, "asked once");
        assert_eq!(s.screen(), None);
        // A configure that still answers the corner request draws nothing.
        assert_eq!(s.configure(1, 1), Commit::Nothing);
        assert_eq!(s.screen(), None);
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Draw(SCREEN));
        assert!(s.may_attach(SCREEN));
    }

    #[test]
    fn hiding_clears_the_screen_then_goes_to_1x1() {
        let mut s = shown();
        assert_eq!(s.want(false), Commit::Shrink { clear: Some(SCREEN) });
        assert_eq!(s.screen(), None);
        assert!(s.may_attach(SCREEN), "the clear buffer goes on at the size still acked");
        assert!(!s.may_attach((1, 1)), "not before the compositor answers");
        s.attached(SCREEN);
        // A late configure at the screen's size asks for nothing.
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Nothing);
        assert_eq!(s.screen(), None);
        assert_eq!(s.configure(1, 1), Commit::Blank);
        assert!(s.may_attach((1, 1)) && !s.may_attach(SCREEN));
    }

    #[test]
    fn hiding_before_the_screens_configure_has_nothing_to_clear() {
        let mut s = Sizing::new();
        s.configure(1, 1);
        s.attached((1, 1));
        s.want(true);
        assert_eq!(s.want(false), Commit::Shrink { clear: None });
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Nothing, "the screen came too late");
        assert_eq!(s.configure(1, 1), Commit::Nothing, "still blank from before");
    }

    #[test]
    fn showing_again_while_shrinking_waits_for_a_new_configure() {
        let mut s = shown();
        s.want(false);
        s.attached(SCREEN);
        assert_eq!(s.want(true), Commit::Grow);
        // The answer to the shrink comes first, and is passed over.
        assert_eq!(s.configure(1, 1), Commit::Nothing);
        assert_eq!(s.screen(), None);
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Draw(SCREEN), "drawn again: the clear buffer is on");
    }

    #[test]
    fn a_new_screen_size_redraws_and_a_repeat_does_not() {
        let mut s = shown();
        assert_eq!(s.configure(SCREEN.0, SCREEN.1), Commit::Nothing);
        assert_eq!(s.configure(2560, 1440), Commit::Draw((2560, 1440)));
        assert!(!s.may_attach(SCREEN));
    }

    #[test]
    #[should_panic(expected = "buffer attached")]
    fn attaching_at_another_size_is_caught() {
        let mut s = shown();
        s.attached((1, 1));
    }
}
