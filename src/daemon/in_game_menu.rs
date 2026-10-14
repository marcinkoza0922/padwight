//! The in-game menu (Guide + Start): Quick Settings and Edit Controls over the game, and what its
//! choices do to the running config. Nothing it changes is saved until its Save.

use super::*;

impl Daemon {
    /// Opens the Guide + Start menu over the game, on `page`, for the controller `device`. That
    /// controller drives it from now on.
    pub(super) fn open_system_menu(&mut self, device: u64, page: Page) {
        if !self.ensure_overlay_process() {
            return;
        }
        log!("opening the settings menu");
        self.release_mappings();
        let paddles = self.devices.get(&device).is_some_and(|d| d.paddles);
        let menu = SystemMenu::new(&self.config).with_paddles(paddles).with_features(self.pad_features()).on_page(page);
        self.active = Some(Active::Settings(menu, device));
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Guide, Some(sound::Feedback::Open));
    }

    /// Runs one event on the Guide + Start menu. `true` when the menu closed.
    pub(super) fn settings_input(&mut self, ev: InputEvent) -> bool {
        let now = Instant::now();
        self.settings_step(|menu, config| menu.handle(ev, now, config))
    }

    /// Runs one step of the Guide + Start menu (an event, or a held direction's repeat) and does
    /// what it asks. `true` when the menu closed.
    pub(super) fn settings_step(&mut self, step: impl FnOnce(&mut SystemMenu, &mut Config) -> Option<SystemOutcome>) -> bool {
        // Outside any game, General is what a change would land in: keep a copy so the change
        // can move into a new pack instead.
        let general = self.config.active_ref().game.is_none().then(|| self.config.general.clone());
        let (outcome, moved, device) = match &mut self.active {
            Some(Active::Settings(menu, device)) => {
                let before = menu.cursor();
                let outcome = step(menu, &mut self.config);
                (outcome, menu.cursor() != before, *device)
            }
            _ => (None, false, 0),
        };
        let picked = matches!(outcome, Some(SystemOutcome::Changed | SystemOutcome::EditControls | SystemOutcome::Switch(_) | SystemOutcome::Added(_)));
        self.overlay_sound(sound::SoundOverlay::Guide, sound::Feedback::of(picked, moved && outcome.is_none()));
        match outcome {
            Some(SystemOutcome::Close) => {
                self.close_overlay();
                return true;
            }
            Some(SystemOutcome::Changed) => {
                if let Some(general) = general {
                    self.pack_from_general(general);
                }
                self.apply_settings_change();
            }
            Some(SystemOutcome::EditControls) => {
                if self.edit_controls() {
                    self.apply_settings_change();
                }
            }
            Some(SystemOutcome::Save { back }) => {
                if let Err(e) = self.config.save() {
                    self.say(vec!["Couldn't save the changes".into(), format!("{e:#}")]);
                    return false;
                }
                self.leave_settings(device, back);
                return true;
            }
            Some(SystemOutcome::Discard { saved, back }) => {
                self.config = *saved;
                // It may go back to another profile too.
                self.apply_settings_change();
                self.leave_settings(device, back);
                return true;
            }
            Some(SystemOutcome::Switch(target)) => return self.settings_switch(target),
            Some(SystemOutcome::Added(target)) => self.settings_added(target),
            Some(SystemOutcome::ReturnToGuide) => {
                self.return_to_guide(device);
                return true;
            }
            Some(SystemOutcome::RemappingOff) => {
                self.remapping_off();
                return true;
            }
            None => {}
        }
        false
    }

    /// Switches to a profile the in-game menu chose. Switch Profile then leaves: `true` when the
    /// menu closed.
    fn settings_switch(&mut self, target: ProfileRef) -> bool {
        self.switch_profile(target, "Chosen in the in-game menu");
        let then = match &mut self.active {
            Some(Active::Settings(menu, _)) => menu.profile_switched(&self.config),
            _ => None,
        };
        if then == Some(SystemOutcome::Close) {
            self.close_overlay();
            return true;
        }
        self.broadcast_overlay();
        false
    }

    /// Switches to the profile the in-game menu just made, and shows it in the list.
    fn settings_added(&mut self, target: ProfileRef) {
        self.apply_settings_change();
        self.switch_profile(target, "Made in the in-game menu");
        if let Some(Active::Settings(menu, _)) = &mut self.active {
            menu.profile_added(&self.config);
        }
        self.broadcast_overlay();
    }

    /// Closes the Guide + Start menu after its question about unsaved changes: back to the Guide
    /// overlay when it was left with Back, else out.
    pub(super) fn leave_settings(&mut self, device: u64, back: bool) {
        if back {
            self.return_to_guide(device);
        } else {
            self.close_overlay();
        }
        // Remapping turned off while the menu had changes kept its controller until now.
        if !self.config.enabled {
            self.release_devices(|_, _| true);
        }
    }

    /// The controller driving the in-game menu went away. The menu waits on hold for another
    /// controller; with remapping off none can come, so it closes and its unsaved changes are
    /// discarded. `false` when the menu isn't up.
    pub(super) fn settings_lost_controller(&mut self) -> bool {
        let Some(Active::Settings(menu, _)) = &mut self.active else { return false };
        if !self.config.enabled {
            self.discard_settings();
            return true;
        }
        log!("the in-game menu's controller went away: it waits for another");
        menu.hold(Hold::NoController);
        self.broadcast_overlay();
        true
    }

    /// A button pressed while the in-game menu waits for a controller makes that controller the
    /// menu's. The press only takes it over. `true` when it did.
    pub(super) fn take_over_settings(&mut self, id: u64, events: &[InputEvent]) -> bool {
        let Some(Active::Settings(menu, device)) = &mut self.active else { return false };
        if menu.on_hold() != Some(Hold::NoController) || !events.iter().any(|ev| matches!(ev, InputEvent::Button(_, true))) {
            return false;
        }
        menu.carry_on();
        *device = id;
        // What its mappings hold is let go of, as for the controller that opened the menu.
        if let Some(dev) = self.devices.get_mut(&id) {
            log!("{} took over the in-game menu", dev.name);
            let mut out = Vec::new();
            dev.engine.release_all(true, &mut out);
            dev.refresh_layers(&self.config);
            dev.dispatch(&mut self.kbm, out);
        }
        self.broadcast_overlay();
        true
    }

    /// Remapping was turned on or off from the tray (or the command line), which is saved at
    /// once. The in-game menu, if up, closes, unless it has changes not saved: then it keeps its
    /// controller and asks to save them, or carries on if remapping comes back on. Returns the
    /// controller kept.
    pub(super) fn settings_remapping_set(&mut self, enabled: bool) -> Option<u64> {
        let Some(Active::Settings(menu, device)) = &mut self.active else { return None };
        menu.remapping_set(enabled);
        let (hold, unsaved, device) = (menu.on_hold(), menu.unsaved(&self.config), *device);
        match hold {
            Some(Hold::RemappingOff) if enabled => menu.carry_on(),
            _ if enabled => return None,
            // No controller drives it, and with remapping off none can come.
            Some(Hold::NoController) => {
                self.discard_settings();
                return None;
            }
            _ if unsaved => menu.hold(Hold::RemappingOff),
            _ => {
                self.close_overlay();
                return None;
            }
        }
        self.broadcast_overlay();
        (!enabled).then_some(device)
    }

    /// Closes the in-game menu without its question, as when the overlay window dies: the config
    /// goes back as it was when the menu opened (but for whether remapping is on), and with
    /// remapping off the controller it kept is let go of.
    pub(super) fn discard_settings(&mut self) {
        let Some(Active::Settings(menu, _)) = &self.active else { return };
        if menu.unsaved(&self.config) {
            let mut saved = menu.saved().clone();
            saved.enabled = self.config.enabled;
            self.config = saved;
            self.refresh_scope();
            log!("the in-game menu closed without asking: its unsaved changes were discarded");
            self.say(vec!["The in-game menu closed".into(), "Changes not saved were discarded".into()]);
        }
        self.close_overlay();
        if !self.config.enabled {
            self.release_devices(|_, _| true);
        }
    }

    /// Quick Settings' Turn remapping off: closes the menu and lets go of every controller. It
    /// isn't saved, like the menu's other changes until Save, so a restart turns it back on.
    pub(super) fn remapping_off(&mut self) {
        self.close_overlay();
        self.config.enabled = false;
        self.release_devices(|_, _| true);
        self.say(vec!["Remapping is off".into(), "Turn it back on from the tray or the settings window".into()]);
    }

    /// Closes the Guide + Start menu and puts the Guide overlay it was opened from back up.
    pub(super) fn return_to_guide(&mut self, device: u64) {
        self.close_overlay();
        if let Some(dev) = self.devices.get_mut(&device) {
            dev.engine.open_guide_overlay();
        }
        self.engine_changed(device);
    }

    /// Edit Controls was chosen. Outside a game it makes the window's pack first. `false` when
    /// there is nothing to edit: the menu goes back to where it was opened from.
    pub(super) fn edit_controls(&mut self) -> bool {
        // Outside any game there is no game to edit, and no window to name a pack after.
        // Refuse, and go back to the menu's main page, or to the Guide overlay.
        if self.config.active_ref().game.is_none() && self.focused.is_none() {
            let back = match &mut self.active {
                Some(Active::Settings(menu, device)) if menu.opened_from_guide() => Some(*device),
                Some(Active::Settings(menu, _)) => {
                    menu.back_to_main();
                    None
                }
                _ => None,
            };
            if let Some(device) = back {
                self.return_to_guide(device);
            }
            self.say(vec!["Edit Controls needs a setup".into(), "Focus a game window first".into()]);
            return false;
        }
        if self.config.active_ref().game.is_none() {
            let general = self.config.general.clone();
            self.pack_from_general(general);
        }
        true
    }

    /// Moves what General's active profile is into a new pack for the window in front, and puts
    /// General back as `general` was. The pack keeps the game-wide choices General had, the
    /// Nintendo layout included, as they were edited. Returns false, changing nothing, without a
    /// window: there is nothing to name the pack after, so the change stays in General.
    pub(super) fn pack_from_general(&mut self, general: Game) -> bool {
        let Some(window) = self.focused.clone() else { return false };
        let Some(edited) = self.config.active().cloned() else { return false };
        let nintendo_layout = self.config.general.nintendo_layout;
        self.config.general = general;
        let at = self.config.add_game_for_window(&window, edited);
        if let Some(game) = self.config.game_mut(at.game.as_deref()) {
            game.nintendo_layout = nintendo_layout;
        }
        log!("made a pack for {} from General", describe(&window));
        self.say(vec![format!("Made a pack: {}", at.game.as_deref().unwrap_or_default())]);
        self.set_profile(at, &format!("Made a pack for {}", describe(&window)));
        true
    }

    /// Applies a Quick Settings or Edit Controls change to the running mappings, and to the
    /// macros, menus and layers the game has (which Edit Controls can change too). It reaches the
    /// config file only when the menu's Save is chosen.
    pub(super) fn apply_settings_change(&mut self) {
        self.refresh_scope();
        self.resync_all();
    }
}
