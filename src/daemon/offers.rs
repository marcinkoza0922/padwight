//! Offering a library setup for the game in front: the hint when it comes to the front, the
//! offer a Guide tap opens, and its answer.

use super::*;

impl Daemon {
    /// The library packs a Guide tap over `window` would offer, parsed from the build: the daemon
    /// keeps only the library's listings.
    pub(super) fn library_candidates(&self, window: &WindowInfo) -> Vec<crate::library::Entry> {
        let library = crate::library::listings();
        let have = self.pad_features();
        let found = crate::offer::candidates(&self.config, library, window, have.as_deref());
        found
            .into_iter()
            .map(|i| &library[i])
            .filter(|listing| self.offers.declined.get(&listing.id) != Some(&window.pid))
            .filter_map(crate::library::Listing::entry)
            .collect()
    }

    /// Says, once per run of a game, that tapping Guide adds the library's setup for the game
    /// that just came to the front.
    pub(super) fn hint_library_game(&mut self, window: &WindowInfo) {
        // Without a controller there's no Guide to tap; the hint waits for the next focus.
        if !self.config.enabled || self.devices.is_empty() {
            return;
        }
        if !self.offers.hinted.insert((window.pid, window.class.clone(), window.exe.clone())) {
            return;
        }
        let found = self.library_candidates(window);
        let name = match found.as_slice() {
            [] => return,
            [one] => one.pack.pack.name.clone(),
            [first, ..] if window.title.trim().is_empty() => first.pack.pack.name.clone(),
            _ => window.title.trim().to_owned(),
        };
        log!("hinting that Guide adds the library setup for {}", describe(window));
        self.toast = Some(crate::offer::hint(&name, Instant::now()));
        self.broadcast_overlay();
    }

    /// Asks whether to add the focused game's profiles, when the library has some for it.
    pub(super) fn offer_library_game(&mut self, by: u64) {
        if self.active.is_some() || !self.config.enabled {
            return;
        }
        let Some(window) = self.focused.clone() else { return };
        let found = self.library_candidates(&window);
        let have = self.pad_features();
        if found.is_empty() || !self.ensure_overlay_process() {
            return;
        }
        log!("offering library profiles for {}", describe(&window));
        self.release_mappings();
        let all: Vec<usize> = (0..found.len()).collect();
        let session = OfferSession::new(&found, &all, have.as_deref(), Instant::now());
        self.offers.shown = found;
        self.offers.shown_pid = window.pid;
        self.driving.owner = Some(by);
        self.active = Some(Active::Offer(session));
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Offer, Some(sound::Feedback::Open));
    }

    /// Acts on the answer to the offer: adds the chosen pack, or remembers the refusal.
    pub(super) fn answer_offer(&mut self, outcome: OfferOutcome) {
        let Some(Active::Offer(session)) = &self.active else { return };
        let ids: Vec<String> = session.ids().map(str::to_owned).collect();
        let offered = std::mem::take(&mut self.offers.shown);
        self.close_overlay();
        match outcome {
            OfferOutcome::No => {
                let pid = self.offers.shown_pid;
                self.offers.declined.extend(ids.into_iter().map(|id| (id, pid)));
            }
            OfferOutcome::Never => {
                let mut config = self.config.clone();
                config.declined_packs.extend(ids);
                self.say(vec!["Won't ask again".into()]);
                self.replace_config(config);
            }
            OfferOutcome::Add(i, profile) => {
                let Some(entry) = offered.into_iter().nth(i) else { return };
                let mut config = self.config.clone();
                let profile = profile.and_then(|p| entry.pack.profiles.get(p)).map(|p| p.name.clone());
                let plan = crate::pack::plan(&config, entry.pack, true);
                let choices = crate::pack::Choices { replace: false, keep_mine: vec![true; plan.rule_clashes.len()] };
                let name = crate::pack::apply(&mut config, &plan, &choices);
                log!("added library setup {name}");
                // The chosen profile is the one this window's rules pick.
                if let (Some(profile), Some(window), Some(game)) =
                    (profile, &self.focused, config.games.iter_mut().find(|g| g.name == name))
                {
                    for rule in game.rules.iter_mut().filter(|r| crate::focus::rule_matches_any(r, std::slice::from_ref(window))) {
                        rule.profile.clone_from(&profile);
                    }
                }
                if let Response::Error(e) = self.replace_config(config) {
                    return self.say(vec!["Couldn't add the setup".into(), e]);
                }
                self.say(vec![format!("Added {name}")]);
                if let Some(window) = self.focused.clone() {
                    self.window_focused(&window);
                }
            }
        }
    }

    /// Feeds an input to the library offer, and sounds its step or pick.
    pub(super) fn offer_input(&mut self, ev: InputEvent, now: Instant) -> Option<OfferOutcome> {
        let Some(Active::Offer(o)) = &mut self.active else { return None };
        let before = o.position();
        let outcome = o.handle(ev, now);
        let picked = outcome.is_some() || o.position().0 != before.0;
        let moved = !picked && o.position() != before;
        let feedback = sound::Feedback::of(picked, moved);
        self.overlay_sound(sound::SoundOverlay::Offer, feedback);
        outcome
    }
}
