//! The stage's overlay over time: the Flash button's pulses, and its motion and looping effects
//! when animations play.

use super::*;
use crate::motion::{OverlayKind, Timing};

/// How long the overlay flashes, and how many times it pulses in that.
const FLASH_FOR: Duration = Duration::from_millis(1500);
const FLASH_PULSES: f32 = 3.0;

impl App {
    /// How the overlay on the stage moves now: still, or in the shown setup's motion style for
    /// its kind, opened when it was last played.
    pub(super) fn stage_anim(&self, stage: &Stage) -> Anim {
        if !stage.animate {
            return Anim::still();
        }
        let kind = match stage.target {
            StageTarget::Menu(_) | StageTarget::Panel(Panel::InGameMenu) | StageTarget::Setup(Panel::InGameMenu) => OverlayKind::Menu,
            StageTarget::Info(_) => OverlayKind::Info,
            StageTarget::Log(_) => OverlayKind::Log,
            StageTarget::Panel(Panel::Keyboard) | StageTarget::Setup(Panel::Keyboard) => OverlayKind::Keyboard,
            StageTarget::Panel(Panel::Numpad) | StageTarget::Setup(Panel::Numpad) => OverlayKind::Numpad,
            StageTarget::Panel(Panel::Media) | StageTarget::Setup(Panel::Media) => OverlayKind::Media,
        };
        let motion = self.game().motion.unwrap_or(self.config.motion);
        let timing = Timing { opened: stage.played.elapsed().as_secs_f32(), ..Timing::default() };
        Anim { feel: motion.feel(kind), timing, clock: stage.clock() }
    }
}

impl Stage {
    /// Whether it changes from frame to frame, so the window needs redrawing.
    pub(in crate::gui) fn is_ticking(&self) -> bool {
        self.flash.is_some() || self.animate
    }

    /// Lets a flash go once it has played out.
    pub(super) fn tick(&mut self, now: Instant) {
        self.flash = self.flash.filter(|since| now.saturating_duration_since(*since) < FLASH_FOR);
    }

    /// The clock looping effects run on: stopped while it's drawn still.
    pub(super) fn clock(&self) -> f32 {
        if self.animate { crate::motion::clock(Instant::now()) } else { 0.0 }
    }

    /// How bright the flash is now, from 0 (not flashing) to 1: pulses that fade out.
    pub(super) fn flashing(&self) -> f32 {
        let Some(since) = self.flash else { return 0.0 };
        let t = since.elapsed().as_secs_f32() / FLASH_FOR.as_secs_f32();
        if t >= 1.0 { return 0.0 }
        (t * FLASH_PULSES * std::f32::consts::TAU).cos().mul_add(-0.5, 0.5) * (1.0 - t * 0.5)
    }
}
