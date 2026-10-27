//! Controller input while an overlay is up: it drives the keyboard, a menu, the media controls or
//! the offer instead of the mappings, and repeats and timeouts advance them.

use super::*;

impl Daemon {
    /// Redraws info overlays and advances the keyboard or menu that is up.
    pub(super) fn tick_overlays(&mut self, now: Instant) {
        if self.info_refresh.is_some_and(|t| t <= now) {
            self.sampler.sample();
            self.broadcast_overlay();
        } else if self.info_fade.is_some_and(|t| t <= now) || self.log_redraw.is_some_and(|t| t <= now) {
            self.broadcast_overlay();
        }
        let (mut keyboard_actions, mut changed) = (Vec::new(), false);
        // A held direction that repeats moves the cursor, and sounds like any other step.
        let mut repeated_step = None;
        match &mut self.active {
            Some(Active::Keyboard(k)) if k.next_deadline().is_some_and(|d| d <= now) => {
                let (overlay, before) = (keyboard_sound(k.layout()), k.cursor());
                (keyboard_actions, changed) = k.tick(now);
                repeated_step = (k.cursor() != before).then_some(overlay);
            }
            Some(Active::Menu { session, .. }) if session.expired(now) => return self.close_overlay(),
            Some(Active::Menu { session, .. }) if session.next_deadline().is_some_and(|d| d <= now) => {
                let before = session.cursor();
                changed = session.tick(&self.scope.menus, now);
                repeated_step = (session.cursor() != before).then_some(sound::SoundOverlay::Menu);
            }
            Some(Active::Media(m)) if m.next_deadline().is_some_and(|d| d <= now) => {
                m.tick(now);
                if m.expired(now) {
                    return self.close_overlay();
                }
            }
            Some(Active::Offer(o)) if o.expired(now) => return self.answer_offer(OfferOutcome::No),
            Some(Active::Offer(o)) if o.next_deadline().is_some_and(|d| d <= now) => {
                if o.tick(now) {
                    repeated_step = Some(sound::SoundOverlay::Offer);
                    changed = true;
                }
            }
            Some(Active::NewPad(..)) => match self.tick_new_pad(now) {
                Some(moved) => (changed, repeated_step) = (moved, moved.then_some(sound::SoundOverlay::Offer)),
                None => return,
            },
            Some(Active::Settings(m, _)) if m.next_deadline().is_some_and(|d| d <= now) => {
                if !self.settings_step(|menu, config| menu.tick(now, config)) {
                    self.broadcast_overlay();
                }
                return;
            }
            _ => {}
        }
        if let Some(overlay) = repeated_step {
            self.overlay_sound(overlay, Some(sound::Feedback::Step));
        }
        self.apply_keyboard(keyboard_actions);
        if changed {
            self.broadcast_overlay();
        }
    }

    /// Called when a device stops being managed; watchers fall back to "no controller". An
    /// overlay it drove can't be driven any more, so every overlay closes: `None` when it drove
    /// none, else whether the in-game menu's unsaved edits were put aside.
    pub(super) fn forget_active(&mut self, id: u64) -> Option<bool> {
        if self.last_active == Some(id) {
            self.last_active = None;
            self.broadcast(None);
        }
        self.close_overlays_of(id)
    }

    /// The controller the overlay on screen belongs to, if it has one yet.
    pub(super) fn overlay_driver(&self) -> Option<u64> {
        match &self.active {
            Some(Active::Menu { device, .. } | Active::Settings(_, device) | Active::Tools(_, device)) => Some(*device),
            _ => self.driving.owner,
        }
    }

    /// Whether controller `id` drives the overlay on screen. The keyboard, numpad, media controls
    /// and offer go to the first controller that sends them input; the others keep playing.
    pub(super) fn drives_overlay(&mut self, id: u64) -> bool {
        match &self.active {
            Some(Active::Menu { device, .. } | Active::Settings(_, device) | Active::Tools(_, device)) => *device == id,
            Some(_) => *self.driving.owner.get_or_insert(id) == id,
            None => false,
        }
    }

    /// Feeds an input to the on-screen keyboard or numpad, and sounds its step or key press.
    pub(super) fn keyboard_input(&mut self, id: u64, ev: InputEvent, now: Instant) {
        let Some(Active::Keyboard(k)) = &mut self.active else { return };
        let (overlay, before) = (keyboard_sound(k.layout()), k.cursor());
        let actions = k.handle(ev, now);
        let picked = actions.iter().any(|a| matches!(a, OverlayAction::Key(_, true)));
        let feedback = sound::Feedback::of(picked, k.cursor() != before);
        self.overlay_sound(overlay, feedback);
        self.log_keyboard(id, ev, &actions);
        self.apply_keyboard(actions);
    }

    /// Feeds an input to the media controls, and sounds it. Returns whether they close.
    pub(super) fn media_input(&mut self, ev: InputEvent, now: Instant) -> bool {
        let Some(Active::Media(m)) = &mut self.active else { return false };
        let outcome = m.handle(ev, now);
        if outcome.is_none() {
            self.overlay_sound(sound::SoundOverlay::Media, media_feedback(ev));
        }
        outcome == Some(MediaOutcome::Close)
    }

    /// Guide was pressed over the overlay: closes it. `true` when it closed.
    pub(super) fn guide_closes_overlay(&mut self) -> bool {
        match &mut self.active {
            Some(Active::Offer(_)) => self.answer_offer(OfferOutcome::No),
            Some(Active::NewPad(..)) => self.answer_new_pad(NewPadOutcome::Later),
            Some(Active::Settings(menu, _)) => {
                if menu.guide_pressed(&self.config) != Some(SystemOutcome::Close) {
                    self.broadcast_overlay();
                    return false;
                }
                self.close_overlay();
            }
            Some(_) => self.close_overlay(),
            None => {}
        }
        true
    }

    /// Runs one event on the open menu, and the item it chooses. `true` when the menu is no
    /// longer up.
    pub(super) fn menu_input(&mut self, id: u64, ev: InputEvent, now: Instant) -> bool {
        let Some(Active::Menu { session, device }) = &mut self.active else { return true };
        let device = *device;
        let before = session.cursor();
        session.set_keys(self.config.shortcuts.menu_keys());
        let outcome = session.handle(&self.scope.menus, ev, now);
        let picked = matches!(outcome, Some(MenuOutcome::Choose { .. }));
        let moved = outcome.is_none() && session.cursor() != before;
        self.overlay_sound(sound::SoundOverlay::Menu, sound::Feedback::of(picked, moved));
        match outcome {
            Some(MenuOutcome::Choose { menu, item, action, close }) => {
                self.log_menu_choice(id, ev, &action);
                if close {
                    // The picked item, lit, is the last the overlay draws of the menu.
                    self.broadcast_overlay();
                    self.close_overlay();
                }
                // A menu set to stay open stays up for more picks, unless the item put it away.
                self.run_menu_item(device, &menu, item, &action);
                !matches!(self.active, Some(Active::Menu { .. }))
            }
            Some(MenuOutcome::Close) => {
                self.close_overlay();
                true
            }
            None => false,
        }
    }

    /// What a press of `b` does in the media controls or the numpad, when either is up.
    pub(super) fn panel_press(&self, b: Button) -> PanelPress {
        let group = match &self.active {
            Some(Active::Media(_)) => Group::Media,
            Some(Active::Keyboard(k)) if k.layout() == Layout::Numpad => Group::Numpad,
            Some(Active::Keyboard(_)) => Group::Keyboard,
            _ => return PanelPress::Other,
        };
        self.config.shortcuts.panel_press(group, b)
    }

    /// What the overlay that's up hears of `ev`: the panels take A and B where their keys sit, and
    /// a press they ignore is none. Menus do it themselves, since some read the face buttons as
    /// they are.
    fn overlay_hears(&self, ev: InputEvent) -> Option<InputEvent> {
        let InputEvent::Button(b, down) = ev else { return Some(ev) };
        Some(match self.panel_press(b) {
            PanelPress::Key(key) => InputEvent::Button(key, down),
            PanelPress::Ignored => return None,
            PanelPress::Other if matches!(self.active, Some(Active::Menu { .. })) => InputEvent::Button(b, down),
            PanelPress::Other => InputEvent::Button(self.config.shortcuts.menu_keys().logical(b, true), down),
        })
    }

    /// While the overlay shows something, controller input drives it instead of the mappings.
    /// (Every way out redraws, so the input log's view of it shows too.)
    pub(super) fn overlay_input(&mut self, id: u64, events: Vec<InputEvent>, fires: &[bool]) {
        let now = Instant::now();
        if let Some(dev) = self.devices.get_mut(&id) {
            for ev in &events {
                dev.view.apply(ev);
                if let InputEvent::Axis(axis, value) = *ev {
                    dev.engine.note_axis(axis, value);
                }
            }
            dev.draw_status();
        }
        // The overlay's glyphs show the labelled buttons, so it hears them too.
        let swapped = self.faces_swapped(id);
        for (ev, fired) in events.into_iter().zip(fires) {
            let ev = match ev {
                InputEvent::Button(b, down) if swapped => InputEvent::Button(crate::info::nintendo_face(b), down),
                other => other,
            };
            // What opens the Guide overlay (Guide, unless moved) closes whatever is up, as well as
            // its own close inputs: the offer as "not now", the in-game menu once any unsaved
            // changes are answered. The tap it ends isn't a tap, so it doesn't offer a library
            // game on release.
            if *fired {
                self.offers.guide_tap = None;
                if self.guide_closes_overlay() {
                    return;
                }
                continue;
            }
            let Some(ev) = self.overlay_hears(ev) else { continue };
            match &mut self.active {
                Some(Active::Keyboard(_)) => self.keyboard_input(id, ev, now),
                Some(Active::Menu { .. }) => {
                    if self.menu_input(id, ev, now) {
                        return;
                    }
                }
                Some(Active::Settings(..)) => {
                    if self.settings_input(ev) {
                        return;
                    }
                }
                Some(Active::Tools(..)) => {
                    if self.tools_input(ev) {
                        return;
                    }
                }
                Some(Active::Media(_)) => {
                    if self.media_input(ev, now) {
                        return self.close_overlay();
                    }
                }
                Some(Active::Offer(_)) => {
                    if let Some(outcome) = self.offer_input(ev, now) {
                        return self.answer_offer(outcome);
                    }
                }
                Some(Active::NewPad(..)) => {
                    if let Some(outcome) = self.new_pad_input(ev, now) {
                        return self.answer_new_pad(outcome);
                    }
                }
                None => return,
            }
        }
        self.broadcast_overlay();
    }
}
