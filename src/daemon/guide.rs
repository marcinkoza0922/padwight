//! The Guide overlay's center menu and its Mappings: moving through them, and running the item
//! chosen.

use super::*;

impl Daemon {
    /// Makes the Guide menu when the Guide overlay goes up, and drops it when the overlay comes down.
    pub(super) fn sync_guide_menu(&mut self) {
        let open = self.devices.values().any(|d| d.engine.guide_overlay_open());
        match (open, self.guide_menu.as_ref().map(|menu| menu.chosen)) {
            (true, None) => {
                let mode = self.guide_current_mode();
                self.guide_menu = Some(GuideMenu { cursor: 0, steam: false, page: 0, mode, chosen: false });
                // Reading every process's name takes a few milliseconds: not on this thread.
                let tx = self.tx.clone();
                thread::spawn(move || {
                    let _ = tx.send(Msg::SteamRunning(crate::guide_menu::steam_running()));
                });
                self.overlay_sound(sound::SoundOverlay::Guide, Some(sound::Feedback::Open));
            }
            (false, Some(chosen)) => {
                self.guide_menu = None;
                self.overlay_sound(sound::SoundOverlay::Guide, (!chosen).then_some(sound::Feedback::Close));
            }
            _ => {}
        }
    }

    /// Lists the Steam overlay row in the Guide menu, or takes it away, once whether Steam runs
    /// is known. The cursor stays on an item that is still there.
    pub(super) fn guide_menu_steam(&mut self, running: bool) {
        let Some(menu) = self.guide_menu.as_mut().filter(|m| m.steam != running) else { return };
        menu.steam = running;
        menu.cursor = menu.cursor.min(crate::guide_menu::items(running).len() - 1);
        self.broadcast_overlay();
    }

    /// What the Guide overlay's Mappings show: the active profile in the mode and page the Guide
    /// menu is on.
    pub(super) fn guide_view(&self) -> Option<crate::info::GuideView<'_>> {
        let menu = self.guide_menu.as_ref();
        Some(crate::info::GuideView {
            profile: self.config.active()?,
            modes: self.scope.game_modes(),
            layers: &self.scope.layers,
            shown: menu.map_or_else(|| self.guide_current_mode(), |m| m.mode),
            current: self.guide_current_mode(),
            page: menu.map_or(0, |m| m.page),
        })
    }

    /// The mode on for the controller that opened the Guide overlay, counted as the Mappings
    /// count them (0 is the default).
    pub(super) fn guide_current_mode(&self) -> usize {
        let pad = self.devices.values().find(|d| d.engine.guide_overlay_open());
        let mode = pad.and_then(|d| d.engine.chosen_mode());
        mode.and_then(|m| self.scope.game_modes().iter().position(|x| x.name == m)).map_or(0, |i| i + 1)
    }

    /// Moves the Guide menu's cursor, or chooses or closes, for the presses the engine took while
    /// the overlay was up.
    pub(super) fn guide_keys(&mut self, device: u64, keys: &[crate::config::Button]) {
        use crate::config::Button;
        for &key in keys {
            let Some(menu) = self.guide_menu.as_mut() else { return };
            let len = crate::guide_menu::items(menu.steam).len();
            match key {
                Button::DpadUp | Button::LeftStickUp => menu.cursor = (menu.cursor + len - 1) % len,
                Button::DpadDown | Button::LeftStickDown => menu.cursor = (menu.cursor + 1) % len,
                // The right stick turns the Mappings' pages, and left and right their modes.
                Button::RightStickUp => menu.page = menu.page.saturating_sub(1),
                Button::RightStickDown => {
                    let pages = self.guide_view().map_or(1, |v| v.pages());
                    if let Some(menu) = self.guide_menu.as_mut() {
                        menu.page = (menu.page + 1).min(pages - 1);
                    }
                }
                Button::DpadLeft | Button::LeftStickLeft | Button::DpadRight | Button::LeftStickRight => {
                    let modes = self.scope.game_modes().len() + 1;
                    if modes > 1 {
                        let forward = matches!(key, Button::DpadRight | Button::LeftStickRight);
                        menu.mode = if forward { (menu.mode + 1) % modes } else { (menu.mode + modes - 1) % modes };
                        menu.page = 0;
                        self.overlay_sound(sound::SoundOverlay::Guide, Some(sound::Feedback::Mode));
                    }
                }
                Button::South => {
                    let item = crate::guide_menu::items(menu.steam)[menu.cursor];
                    menu.chosen = true;
                    self.overlay_sound(sound::SoundOverlay::Guide, Some(sound::Feedback::Pick));
                    self.guide_choose(device, item);
                }
                Button::East => self.guide_close(device),
                _ => {}
            }
            if matches!(key, Button::DpadUp | Button::LeftStickUp | Button::DpadDown | Button::LeftStickDown) {
                self.overlay_sound(sound::SoundOverlay::Guide, Some(sound::Feedback::Step));
            }
        }
        // The cursor is part of the drawn frame, so the overlay has to be sent again to move it.
        self.broadcast_overlay();
    }

    /// Runs a Guide menu item. The overlay goes away first. Quick Settings, Edit Controls and
    /// Switch Profile open the Guide + Start menu on their page.
    pub(super) fn guide_choose(&mut self, device: u64, item: crate::guide_menu::Item) {
        use crate::guide_menu::Item;
        match item {
            Item::QuickSettings => {
                self.guide_close(device);
                self.open_system_menu(device, Page::Quick);
            }
            Item::EditControls => {
                self.guide_close(device);
                self.open_system_menu(device, Page::EditControls);
                if self.edit_controls() {
                    self.apply_settings_change();
                }
            }
            Item::SwitchProfile => {
                self.guide_close(device);
                self.open_system_menu(device, Page::Profiles);
            }
            Item::Keyboard | Item::Numpad | Item::Media | Item::Steam => {
                self.guide_close(device);
                match item {
                    Item::Keyboard => self.toggle_overlay(Layout::Keyboard, Some(device)),
                    Item::Numpad => self.toggle_overlay(Layout::Numpad, Some(device)),
                    Item::Media => self.toggle_media(Some(device)),
                    _ => self.send_steam_overlay(device),
                }
            }
            Item::MouseMode => {
                if let Some(dev) = self.devices.get_mut(&device) {
                    dev.engine.enter_mouse_mode();
                }
                self.engine_changed(device);
            }
        }
    }

    /// Closes the Guide overlay on the controller that drives it.
    pub(super) fn guide_close(&mut self, device: u64) {
        if let Some(dev) = self.devices.get_mut(&device) {
            dev.engine.close_guide_overlay();
        }
        self.engine_changed(device);
    }

    /// Applies a layer change the engine made outside an event: the layered profile and the
    /// analog state follow it, as they do after an event.
    pub(super) fn engine_changed(&mut self, device: u64) {
        let Some(dev) = self.devices.get_mut(&device) else { return };
        let mut out = Vec::new();
        if dev.refresh_layers(&self.config)
            && let Some(base) = self.config.profile_for(&dev.id)
        {
            dev.engine.resync(layered(&dev.layered, base), &mut out);
        }
        dev.dispatch(&mut self.kbm, out);
        self.check_info_changes();
    }

    /// Sends the real Guide button to the virtual pad, which opens Steam's overlay.
    pub(super) fn send_steam_overlay(&mut self, device: u64) {
        let Some(dev) = self.devices.get_mut(&device) else { return };
        let out = vec![
            OutEvent::PadButton(crate::config::Button::Guide, true),
            OutEvent::PadButton(crate::config::Button::Guide, false),
        ];
        dev.dispatch(&mut self.kbm, out);
    }
}
