//! The surface's scale, apart from Wayland so its rules can be tested. Sizes elsewhere are
//! logical (what the layer surface is configured at); the buffer is that size in device pixels.
//!
//! With wp_fractional_scale_v1 and a viewport, the compositor's preferred scale (in 120ths)
//! gives a buffer of round(logical × scale / 120) pixels, which the viewport maps to the
//! logical size, with a buffer scale of 1. Without them, the whole-number scale from
//! `wl_surface.preferred_buffer_scale` is the buffer scale and the buffer is that many times
//! the logical size. Without either, the scale is 1.

/// The scale the compositor prefers, as far as it has said.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Scale {
    /// wp_fractional_scale_v1's preferred scale in 120ths, if it has sent one.
    fractional: Option<u32>,
    /// wl_surface's preferred buffer scale, if it has sent one.
    integer: Option<u32>,
    /// Whether the compositor has a viewporter, which the fractional scale needs.
    viewport: bool,
}

/// How a buffer goes onto the surface at a scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Buffer scale 1, and the viewport's destination is the logical size.
    Viewport,
    /// This buffer scale, no viewport destination.
    Buffer(u32),
}

impl Scale {
    pub(super) fn new(viewport: bool) -> Scale {
        Scale { viewport, ..Scale::default() }
    }

    /// A preferred scale from wp_fractional_scale_v1; whether it changed anything.
    pub(super) fn set_fractional(&mut self, scale120: u32) -> bool {
        let before = *self;
        self.fractional = Some(scale120.max(1));
        *self != before
    }

    /// A preferred buffer scale from wl_surface; whether it changed anything.
    pub(super) fn set_integer(&mut self, scale: i32) -> bool {
        let before = *self;
        self.integer = Some(u32::try_from(scale).unwrap_or(1).max(1));
        *self != before
    }

    pub(super) fn mode(self) -> Mode {
        match self.fractional {
            Some(_) if self.viewport => Mode::Viewport,
            _ => Mode::Buffer(self.integer.unwrap_or(1)),
        }
    }

    /// Device pixels to a logical one, for drawing.
    pub(super) fn factor(self) -> f32 {
        match self.mode() {
            Mode::Viewport => self.fractional.unwrap_or(120) as f32 / 120.0,
            Mode::Buffer(n) => n as f32,
        }
    }

    /// The buffer's size for a surface of `logical` size.
    pub(super) fn device(self, (w, h): (u32, u32)) -> (u32, u32) {
        match self.mode() {
            Mode::Viewport => {
                let s = self.fractional.unwrap_or(120);
                (scaled(w, s), scaled(h, s))
            }
            Mode::Buffer(n) => (w * n, h * n),
        }
    }
}

/// `logical` × `scale120` / 120, rounded half away from zero as wp_fractional_scale_v1 says,
/// and at least 1.
fn scaled(logical: u32, scale120: u32) -> u32 {
    u32::try_from((u64::from(logical) * u64::from(scale120) + 60) / 120).unwrap_or(u32::MAX).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_said_is_scale_1() {
        let s = Scale::new(true);
        assert_eq!((s.mode(), s.factor(), s.device((1920, 1080))), (Mode::Buffer(1), 1.0, (1920, 1080)));
    }

    #[test]
    fn fractional_scale_rounds_half_away_from_zero() {
        let mut s = Scale::new(true);
        assert!(s.set_fractional(180));
        assert_eq!(s.mode(), Mode::Viewport);
        assert_eq!(s.device((1920, 1080)), (2880, 1620));
        // The hidden 1×1 surface: 1.5 rounds up to 2.
        assert_eq!(s.device((1, 1)), (2, 2));
        // 3 × 1.25 = 3.75 up, 5 × 1.25 = 6.25 down.
        assert!(s.set_fractional(150));
        assert_eq!(s.device((3, 5)), (4, 6));
        assert_eq!(s.factor(), 1.25);
        // 5 × 1.1 = 5.5 up, 7 × 1.2 = 8.4 down.
        assert_eq!(scaled(5, 132), 6);
        assert_eq!(scaled(7, 144), 8);
        assert_eq!(scaled(1, 1), 1, "never empty");
    }

    #[test]
    fn a_repeat_changes_nothing() {
        let mut s = Scale::new(true);
        assert!(s.set_fractional(240));
        assert!(!s.set_fractional(240));
        assert!(s.set_integer(2));
        assert!(!s.set_integer(2));
    }

    #[test]
    fn without_a_viewport_the_buffer_scale_is_used() {
        let mut s = Scale::new(false);
        s.set_fractional(180);
        assert_eq!(s.mode(), Mode::Buffer(1));
        s.set_integer(2);
        assert_eq!((s.mode(), s.factor(), s.device((1920, 1080)), s.device((1, 1))), (Mode::Buffer(2), 2.0, (3840, 2160), (2, 2)));
    }

    #[test]
    fn fractional_scale_wins_over_the_buffer_scale() {
        let mut s = Scale::new(true);
        s.set_integer(2);
        assert_eq!(s.mode(), Mode::Buffer(2));
        s.set_fractional(180);
        assert_eq!(s.mode(), Mode::Viewport);
        assert_eq!(s.device((100, 100)), (150, 150));
    }
}
