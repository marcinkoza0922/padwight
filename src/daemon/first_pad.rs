//! Keeping a virtual pad ahead of the real controllers in Steam Input.
//!
//! Steam gives each controller it finds the lowest free slot, and lists them to games in slot
//! order. Some games read only the first one: Slay the Spire 2 takes
//! `GetConnectedControllers()[0]` and nothing else. A real controller that is there before its
//! virtual pad takes the slot ahead of it, and as Padwight grabs it, such a game reads a controller
//! that never moves. So while remapping is on, the daemon keeps a virtual pad even with no
//! controller to feed it: the first controller to connect takes it over, and as that controller
//! leaves, a fresh one takes its place.
//!
//! A pad being rebuilt (for another identity, or the controller's rumble) is dropped before its
//! replacement is made, so the replacement gets the slot it freed: Steam notices a pad leaving at
//! once and a new one about a second later. A controller taking the kept pad over doesn't rebuild
//! it right away though, as it would hand the freed slot to that same controller arriving in
//! Steam. It is rebuilt a little later, once Steam has placed the controller.

//!
//! A controller pinned to a player number (`ControllerEntry::player`) gets its pad in that slot the
//! same way: pads made as placeholders fill the slots ahead of it, and a controller without a pin
//! takes the lowest of them over before a new pad is made.

use std::time::{Duration, Instant};

use super::{Daemon, devices::FfSpec, devices::virtual_pad_for};
use crate::{monitor::log, output::VirtualPad, pad_identity::PadIdentity};

/// How long after taking the kept pad over a controller waits to have it rebuilt to fit.
const REFIT_DELAY: Duration = Duration::from_secs(2);

/// The pad a controller that just connected feeds.
pub(super) struct NewPad {
    pub pad: VirtualPad,
    /// It is the kept pad, the one ahead of the real controllers.
    pub first: bool,
    /// When to rebuild it to fit the controller, if it doesn't yet.
    pub refit_at: Option<Instant>,
}

/// How many placeholder pads must exist before a controller pinned to `player` makes its own, when
/// `ahead` pads are already there. Player 1 and no pin need none.
fn fillers_needed(player: Option<u8>, ahead: usize) -> usize {
    player.map_or(0, |n| usize::from(n.saturating_sub(1)).saturating_sub(ahead))
}

impl Daemon {
    /// Makes sure a virtual pad holds the first slot while remapping is on, and none is kept
    /// while it's off. Called before each scan, and after a controller lets its pad go.
    pub(super) fn keep_first_pad(&mut self) {
        if !cfg!(target_os = "linux") || !self.config.enabled {
            self.spare_pad = None;
            self.filler_pads.clear();
            return;
        }
        if self.spare_pad.is_some() || self.devices.values().any(|d| d.first) {
            return;
        }
        let identity = PadIdentity::resolve(self.identity_override, None, self.active_support());
        self.spare_pad = virtual_pad_for(None, identity);
    }

    /// The virtual pad for a controller that just connected: one held ready for it if there is,
    /// otherwise a new one. `player` is the number it is pinned to, 1 for the first slot.
    pub(super) fn pad_for_new(&mut self, ff: Option<&FfSpec>, identity: PadIdentity, player: Option<u8>) -> Option<NewPad> {
        let pinned_behind = cfg!(target_os = "linux") && player.is_some_and(|n| n > 1);
        if pinned_behind {
            let ahead = self.devices.values().filter(|d| d.pad.is_some()).count()
                + usize::from(self.spare_pad.is_some())
                + self.filler_pads.len();
            for _ in 0..fillers_needed(player, ahead) {
                self.filler_pads.extend(virtual_pad_for(None, identity));
            }
        }
        let held = if pinned_behind {
            None
        } else {
            self.spare_pad.take().map(|pad| (pad, true)).or_else(|| (!self.filler_pads.is_empty()).then(|| (self.filler_pads.remove(0), false)))
        };
        let Some((pad, first)) = held else {
            return virtual_pad_for(ff, identity).map(|pad| NewPad { pad, first: false, refit_at: None });
        };
        let fits = ff.is_none() && pad.identity() == identity;
        let refit_at = (!fits).then(|| Instant::now() + REFIT_DELAY);
        Some(NewPad { pad, first, refit_at })
    }

    /// Rebuilds the pads of controllers that took the kept pad over and are due.
    pub(super) fn refit_pads(&mut self) {
        let now = Instant::now();
        let (forced, support) = (self.identity_override, self.active_support());
        for dev in self.devices.values_mut().filter(|d| d.refit_at.is_some_and(|at| at <= now)) {
            log!("fitting the first virtual pad to {}", dev.name);
            dev.rebuild_pad(PadIdentity::resolve(forced, dev.model, support));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fillers_needed;

    #[test]
    fn a_pin_fills_the_slots_ahead_of_it() {
        assert_eq!(fillers_needed(None, 0), 0);
        assert_eq!(fillers_needed(Some(1), 0), 0);
        assert_eq!(fillers_needed(Some(3), 0), 2);
        assert_eq!(fillers_needed(Some(3), 1), 1);
        assert_eq!(fillers_needed(Some(2), 4), 0);
    }
}
