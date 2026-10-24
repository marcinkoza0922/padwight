//! Ways out of every overlay at once, for when the usual ones (B, the opener) don't get there:
//! `padwight close-overlays` (which a desktop shortcut can run), the tray, holding Guide, ten
//! minutes without input, and the controller driving them going away. The in-game menu's
//! unsaved edits are put aside rather than lost, and come back when it next opens.

use super::*;

/// How long overlays stay up with no input from any controller.
const IDLE_CLOSE: Duration = Duration::from_secs(10 * 60);
/// How long Guide is held on its own over an overlay to close them all, and how long before the
/// countdown shows (a tap of Guide that closes the Guide overlay shouldn't flash it).
const GUIDE_HOLD: Duration = Duration::from_secs(3);
const GUIDE_HOLD_SHOWN: Duration = Duration::from_secs(1);

/// What a toast says when the in-game menu's edits were put aside.
const PUT_ASIDE: &str = "Unsaved edits kept: open the in-game menu to carry on";

#[derive(Default)]
pub(super) struct Fallbacks {
    /// Since when no controller input has come while an overlay is up.
    idle_since: Option<Instant>,
    /// When Guide went down on its own over an overlay, and whether the countdown shows.
    guide_hold: Option<(Instant, bool)>,
    /// The in-game menu, closed with edits not saved, and the config with those edits.
    put_aside: Option<(SystemMenu, Config)>,
}

impl Daemon {
    /// Whether anything is up that closing every overlay closes: an overlay that has the
    /// controller, or a Guide overlay.
    fn overlays_up(&self) -> bool {
        self.active.is_some() || self.devices.values().any(|d| d.engine.guide_overlay_open())
    }

    /// Closes every overlay: the one that has the controller and every Guide overlay. The offer
    /// and the new-controller question are answered "not now". `None` when nothing was up, else
    /// whether the in-game menu's unsaved edits were put aside.
    pub(super) fn close_all_overlays(&mut self, why: &str) -> Option<bool> {
        if !self.overlays_up() {
            return None;
        }
        log!("closing every overlay: {why}");
        let put_aside = self.put_settings_aside();
        match &self.active {
            Some(Active::Offer(_)) => self.answer_offer(OfferOutcome::No),
            Some(Active::NewPad(..)) => self.answer_new_pad(NewPadOutcome::Later),
            Some(Active::Settings(..)) => {
                self.close_overlay();
                // Remapping turned off while the menu had changes kept its controller until now.
                if !self.config.enabled {
                    self.release_devices(|_, _| true);
                }
            }
            Some(_) => self.close_overlay(),
            None => {}
        }
        let guides: Vec<u64> = self.devices.iter().filter(|(_, d)| d.engine.guide_overlay_open()).map(|(id, _)| *id).collect();
        for id in guides {
            self.guide_close(id);
        }
        self.fallbacks.idle_since = None;
        Some(put_aside)
    }

    /// Closes every overlay for `padwight close-overlays` or the tray, and says so.
    pub(super) fn close_overlays_by_hand(&mut self, from: &str) {
        if let Some(put_aside) = self.close_all_overlays(from) {
            self.say(closed_lines("Overlays closed", put_aside));
        }
    }

    /// The controller `id` went away: every overlay it drove closes. `None` when it drove none,
    /// else whether the in-game menu's edits were put aside.
    pub(super) fn close_overlays_of(&mut self, id: u64) -> Option<bool> {
        (self.active.is_some() && self.overlay_driver() == Some(id)).then(|| self.close_all_overlays("its controller went away")).flatten()
    }

    /// Notes input from a controller: overlays left up only close once none has come for a while.
    pub(super) fn note_activity(&mut self, events: &[InputEvent]) {
        let active = events.iter().any(|ev| match *ev {
            InputEvent::Button(_, down) => down,
            InputEvent::Axis(_, v) => v.abs() > 0.5,
            InputEvent::Touchpad(_) => true,
        });
        if active {
            self.fallbacks.idle_since = None;
        }
    }

    /// Closes the overlays left up with no input for long enough, and counts down a hold of Guide.
    pub(super) fn check_fallbacks(&mut self, now: Instant) {
        if !self.overlays_up() {
            self.fallbacks.idle_since = None;
        } else if self.fallbacks.idle_since.is_none() {
            self.fallbacks.idle_since = Some(now);
        } else if self.fallbacks.idle_since.is_some_and(|since| now.duration_since(since) >= IDLE_CLOSE)
            && let Some(put_aside) = self.close_all_overlays("no input for 10 minutes")
        {
            self.say(closed_lines("Overlays closed after 10 minutes without input", put_aside));
        }
        self.check_guide_hold(now);
    }

    /// When the fallbacks next need a look: the idle timeout, or the Guide hold's next step.
    pub(super) fn fallback_deadline(&self) -> Option<Instant> {
        let hold = self.fallbacks.guide_hold.map(|(since, shown)| since + if shown { GUIDE_HOLD } else { GUIDE_HOLD_SHOWN });
        self.fallbacks.idle_since.map(|since| since + IDLE_CLOSE).into_iter().chain(hold).min()
    }

    /// Guide held on its own over an overlay: after a moment a countdown shows, and when it fills
    /// every overlay closes. Letting go, or pressing anything else, stops it.
    fn check_guide_hold(&mut self, now: Instant) {
        use crate::config::Button;
        let alone = |d: &Managed| {
            let mut held = d.view.buttons();
            held.next() == Some(Button::Guide) && held.next().is_none()
        };
        let holding = self.overlays_up() && self.devices.values().any(alone);
        match (holding, self.fallbacks.guide_hold) {
            (false, Some((_, shown))) => {
                self.fallbacks.guide_hold = None;
                if shown {
                    self.toast = None;
                    self.broadcast_overlay();
                }
            }
            (false, None) => {}
            (true, None) => self.fallbacks.guide_hold = Some((now, false)),
            (true, Some((since, _))) if now.duration_since(since) >= GUIDE_HOLD => {
                self.fallbacks.guide_hold = None;
                // Letting go of Guide afterwards isn't a tap that opens the Guide overlay again.
                for dev in self.devices.values_mut().filter(|d| alone(d)) {
                    dev.engine.forget_guide_tap();
                }
                if let Some(put_aside) = self.close_all_overlays("Guide held") {
                    self.say(closed_lines("Overlays closed", put_aside));
                }
            }
            (true, Some((since, false))) if now.duration_since(since) >= GUIDE_HOLD_SHOWN => {
                self.fallbacks.guide_hold = Some((since, true));
                let lines = vec!["Keep holding Guide to close every overlay…".into()];
                self.toast = Some(crate::info::Toast::filling(lines, now, GUIDE_HOLD - now.duration_since(since)));
                self.broadcast_overlay();
            }
            (true, Some(_)) => {}
        }
    }

    /// Puts the in-game menu's unsaved edits aside, if it is up and has any: the config goes back
    /// to how it was saved (but for whether remapping is on). `true` when it put some aside.
    fn put_settings_aside(&mut self) -> bool {
        let Some(Active::Settings(menu, _)) = &self.active else { return false };
        if !menu.unsaved(&self.config) {
            return false;
        }
        let mut menu = menu.clone();
        menu.put_aside();
        let mut saved = menu.saved().clone();
        (saved.enabled, saved.acting) = (self.config.enabled, self.config.acting.clone());
        let edited = std::mem::replace(&mut self.config, saved);
        self.refresh_scope();
        log!("the in-game menu closed with unsaved changes: they're put aside until it opens again");
        self.fallbacks.put_aside = Some((menu, edited));
        true
    }

    /// The in-game menu put aside with unsaved edits, with those edits back in the config, if
    /// there is one and the config is still as it was saved then. Edits made while another
    /// profile is active wait for it; edits the config has moved on from are dropped.
    pub(super) fn take_put_aside(&mut self) -> Option<SystemMenu> {
        let (mut menu, mut edited) = self.fallbacks.put_aside.take()?;
        let mut saved = menu.saved().clone();
        (saved.enabled, saved.acting) = (self.config.enabled, self.config.acting.clone());
        if saved != self.config {
            saved.active.clone_from(&self.config.active);
            if saved == self.config {
                self.fallbacks.put_aside = Some((menu, edited));
            } else {
                log!("the in-game menu's unsaved changes put aside were dropped: the config changed since");
                self.say(vec!["Unsaved edits were dropped".into(), "The settings changed after the in-game menu closed".into()]);
            }
            return None;
        }
        (edited.enabled, edited.acting) = (saved.enabled, saved.acting);
        menu.remapping_set(self.config.enabled);
        self.config = edited;
        self.apply_settings_change();
        log!("the in-game menu's unsaved changes put aside are back");
        self.say(vec!["Unsaved edits restored".into()]);
        Some(menu)
    }
}

/// A toast's lines: `head`, and a word about the edits put aside if there were any.
fn closed_lines(head: &str, put_aside: bool) -> Vec<String> {
    std::iter::once(head.to_string()).chain(put_aside.then(|| PUT_ASIDE.to_string())).collect()
}

/// What a toast says about a controller that disconnected, and the overlays that closed with it
/// (`closed`: whether edits were put aside).
pub(super) fn disconnected_lines(name: &str, closed: Option<bool>) -> Vec<String> {
    match closed {
        Some(put_aside) => closed_lines(&format!("{name} disconnected: overlays closed"), put_aside),
        None => vec![format!("{name} disconnected")],
    }
}
