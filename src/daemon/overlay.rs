//! The overlay window and what it shows: opening and closing the keyboard, the media controls and
//! menus, the frames sent to the window, and its sounds. Input to them is in `overlay_input.rs`.

use super::*;

impl Daemon {
    /// Opens or closes the on-screen keyboard or numpad (one replaces the other), for the
    /// controller `by` (`None`: whichever sends it input first).
    pub(super) fn toggle_overlay(&mut self, layout: Layout, by: Option<u64>) {
        match &self.active {
            Some(Active::Keyboard(k)) if k.layout() == layout => return self.close_overlay(),
            Some(Active::Keyboard(_)) => self.close_overlay(),
            Some(Active::Menu { .. } | Active::Media(_) | Active::Offer(_) | Active::Settings(..)) => return,
            None => {}
        }
        self.driving.owner = by;
        if !self.ensure_overlay_process() {
            return;
        }
        self.release_mappings();
        log!("on-screen {} open", layout_name(layout));
        let board = match layout {
            Layout::Keyboard if self.config.full_keyboard => crate::keyboard::Board::Full,
            _ => layout.into(),
        };
        let cursor = self.driving.cursors.get(&board).copied().unwrap_or(board.home());
        let mut controller = OverlayController::new(board, cursor);
        let guide = self.guide_chord_button();
        let held = guide.is_some_and(|g| self.devices.values().any(|d| d.view.buttons().any(|b| b == g)));
        let shortcut = if layout == Layout::Numpad { Shortcut::Numpad } else { Shortcut::Keyboard };
        controller.set_guide(held, guide, self.shortcut_button(shortcut));
        self.active = Some(Active::Keyboard(controller));
        self.broadcast_overlay();
        self.overlay_sound(keyboard_sound(layout), Some(sound::Feedback::Open));
    }

    /// Opens or closes the media controls, for the controller `by`.
    pub(super) fn toggle_media(&mut self, by: Option<u64>) {
        match &self.active {
            Some(Active::Media(_)) => return self.close_overlay(),
            Some(_) => return,
            None => {}
        }
        self.driving.owner = by;
        if !self.ensure_overlay_process() {
            return;
        }
        self.release_mappings();
        log!("media controls open");
        let (commands, requests) = std::sync::mpsc::channel();
        let tx = self.tx.clone();
        thread::spawn(move || crate::media::worker(&requests, |state| tx.send(Msg::Media(state)).is_ok()));
        let guide = self.guide_chord_button();
        let guide_held = guide.is_some_and(|g| self.devices.values().any(|d| d.view.buttons().any(|b| b == g)));
        let session = MediaSession::new(commands, guide_held, Instant::now()).with_chord(guide, self.shortcut_button(Shortcut::Media));
        self.active = Some(Active::Media(session));
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Media, Some(sound::Feedback::Open));
    }

    pub(super) fn update_media(&mut self, state: MediaState) {
        if let Some(Active::Media(m)) = &mut self.active {
            m.update(state);
            self.broadcast_overlay();
        }
    }

    pub(super) fn open_menu(&mut self, device: u64, name: &str, opener: Opener) {
        if self.active.is_some() {
            return;
        }
        let Some(mut session) = MenuSession::open(&self.scope.menus, name, opener) else {
            log!("no menu named {name:?}, or it has no items");
            return;
        };
        if !self.ensure_overlay_process() {
            return;
        }
        if let Some(dev) = self.devices.get(&device)
            && !session.prime(&self.scope.menus, dev.view.buttons(), dev.view.axes())
        {
            log!("menu {name:?} not shown: its button was already let go (use a Toggle to open it with a tap)");
            return;
        }
        self.release_mappings();
        self.active = Some(Active::Menu { session, device });
        self.broadcast_overlay();
        self.overlay_sound(sound::SoundOverlay::Menu, Some(sound::Feedback::Open));
    }

    /// Makes sure the (normally resident) overlay window process is running.
    pub(super) fn ensure_overlay_process(&mut self) -> bool {
        if let Some(child) = &mut self.overlay_process
            && child.try_wait().ok().flatten().is_some()
        {
            self.overlay_process = None;
        }
        if self.overlay_process.is_some() {
            return true;
        }
        match std::env::current_exe().and_then(|exe| os::windowless(std::process::Command::new(exe).arg("overlay")).spawn()) {
            Ok(child) => {
                self.overlay_process = Some(child);
                self.overlay_started = Some(Instant::now());
                true
            }
            Err(e) => {
                log!("cannot start the overlay window: {e}");
                false
            }
        }
    }

    /// Lets go of everything the mappings hold; the controller now drives the overlay.
    /// The mode stays on, so a menu item that switches to one isn't undone by the menu.
    pub(super) fn release_mappings(&mut self) {
        for dev in self.devices.values_mut() {
            let mut out = Vec::new();
            dev.engine.release_all(true, &mut out);
            dev.refresh_layers(&self.config);
            dev.dispatch(&mut self.kbm, out);
        }
    }

    pub(super) fn close_overlay(&mut self) {
        self.driving.owner = None;
        let closing = self.active.as_ref().map(active_sound);
        match self.active.take() {
            Some(Active::Keyboard(mut k)) => {
                self.driving.cursors.insert(k.board(), k.cursor());
                let actions = k.release_all();
                self.apply_keyboard(actions);
                log!("on-screen {} closed", layout_name(k.layout()));
            }
            Some(Active::Menu { .. }) => {}
            Some(Active::Media(_)) => log!("media controls closed"),
            Some(Active::Offer(_) | Active::Settings(..)) => {}
            None => return,
        }
        // The overlay window goes back to idle (it stays running for next time).
        self.broadcast_overlay();
        for dev in self.devices.values_mut() {
            let down = dev.view.buttons().collect();
            dev.engine.forget_released(&down);
        }
        self.resync_all();
        if let Some(overlay) = closing {
            self.overlay_sound(overlay, Some(sound::Feedback::Close));
        }
    }

    pub(super) fn apply_keyboard(&mut self, actions: Vec<OverlayAction>) {
        for action in actions {
            match action {
                OverlayAction::Key(k, pressed) => {
                    if let Err(e) = self.kbm.key(k, pressed) {
                        log!("output error: {e:#}");
                    }
                }
                OverlayAction::Close => self.close_overlay(),
            }
        }
    }

    /// Runs a chosen menu item on the controller that opened the menu.
    pub(super) fn run_menu_item(&mut self, device: u64, menu: &str, item: usize, action: &crate::config::ButtonAction) {
        let Some(dev) = self.devices.get_mut(&device) else { return };
        let mut out = Vec::new();
        dev.engine.tap_menu_item(menu, item, action, &mut out);
        if dev.refresh_layers(&self.config)
            && let Some(base) = self.config.active()
        {
            dev.engine.resync(layered(&dev.layered, base), &mut out);
        }
        dev.dispatch(&mut self.kbm, out);
        let asked = Requests::take(&mut dev.engine);
        // Opening the keyboard closes a menu that is still up.
        if asked.overlay.is_some() && matches!(self.active, Some(Active::Menu { .. })) {
            self.close_overlay();
        }
        self.run_requests(device, asked);
        self.check_info_changes();
    }

    pub(super) fn overlay_view(&self) -> Option<OverlayView> {
        match &self.active {
            Some(Active::Keyboard(k)) => {
                let mut view = k.view(Instant::now());
                view.style = match k.layout() {
                    Layout::Keyboard => self.config.active_keyboard_style().clone(),
                    Layout::Numpad => self.config.active_numpad_style().clone(),
                };
                Some(OverlayView::Keyboard(view))
            }
            Some(Active::Menu { session, .. }) => session.view(&self.scope.menus).map(OverlayView::Menu),
            Some(Active::Media(m)) => {
                let mut view = m.view();
                view.style = self.config.active_media_style().clone();
                Some(OverlayView::Media(view))
            }
            Some(Active::Offer(o)) => Some(o.view(self.config.active_menu_style())),
            Some(Active::Settings(m, _)) => Some(m.overlay_view(&self.config, &self.menu_live)),
            None => match &self.guide_menu {
                Some(menu) => {
                    let profile = self.config.active().map(|p| p.name.as_str()).unwrap_or_default();
                    let style = self.config.active_menu_style().clone();
                    Some(OverlayView::Menu(crate::guide_menu::view(menu.steam, menu.cursor, profile, style)))
                }
                None => self.touch_menu_view(),
            },
        }
    }

    /// The menu a finger on a controller's touchpad is showing, if any. It never takes the
    /// controller: the game keeps getting input while it shows.
    pub(super) fn touch_menu_view(&self) -> Option<OverlayView> {
        let shown = self.devices.values().find_map(|d| d.engine.touch_menu())?;
        let menu = self.scope.menus.iter().find(|m| m.name == shown.menu)?;
        Some(OverlayView::Menu(crate::menu::touch_view(menu, shown.item, shown.choose, shown.picks)))
    }

    pub(super) fn broadcast_overlay(&mut self) {
        let frame = self.overlay_frame();
        if (!frame.info.is_empty() || !frame.logs.is_empty())
            && self.overlay_process.is_none()
            && self.overlay_started.is_none_or(|t| t.elapsed() > Duration::from_secs(10))
        {
            self.ensure_overlay_process();
        }
        if self.last_frame.as_ref() == Some(&frame) {
            return;
        }
        self.overlay_watchers.retain(|w| w.send(frame.clone()).is_ok());
        self.last_frame = Some(frame);
    }

    pub(super) fn overlay_frame(&mut self) -> OverlayFrame {
        let now = Instant::now();
        let shown = self.timed_info(now);
        // Live values (clock, CPU, …) refresh every second while shown.
        let live = shown.iter().any(|(o, _)| crate::info::is_live(o));
        let fast = shown.iter().any(|(o, _)| crate::info::is_fast(o));
        self.info_refresh = live.then(|| Instant::now() + if fast { FAST_INFO_REFRESH } else { INFO_REFRESH });
        if live && self.sampler.sampled_at.is_none() {
            self.sampler.sample();
        }
        let mut values = self.live_values();
        let with_input: Vec<&crate::config::InfoOverlay> =
            shown.iter().map(|(o, _)| o).filter(|o| !crate::info::input_tokens(o).is_empty()).collect();
        if !with_input.is_empty() {
            values.inputs = self.inputs(now);
        }
        let token_redraw = self.token_redraw(&with_input, &values.inputs, now);
        let (logs, logs_up) = self.log_frame(now, values.family, values.nintendo_layout);
        self.shows_input = !with_input.is_empty() || logs_up;
        self.log_redraw = self.log_redraw.into_iter().chain(token_redraw).min();
        let mut info: Vec<_> = shown
            .iter()
            .map(|(o, opacity)| crate::info::InfoView { opacity: *opacity, ..crate::info::resolve(o, &values) })
            .collect();
        if self.recording.is_some() {
            info.push(crate::info::recording_view());
        }
        let (mut info, logs) = self.with_preview(info, logs, &values, now);
        let now = Instant::now();
        let toast = self.toast.as_ref().and_then(|t| Some((t.view(now)?, t.next_redraw(now, FADE_FRAME))));
        match toast {
            Some((view, redraw)) => {
                info.push(view);
                self.info_fade = Some(self.info_fade.map_or(redraw, |t| t.min(redraw)));
            }
            None => self.toast = None,
        }
        OverlayFrame {
            info,
            logs,
            active: self.overlay_view(),
            font: self.config.active_font().map(str::to_string),
            colorblind: self.config.colorblind_tones,
            opener: crate::info::OpenerLabel::of(&self.config.shortcuts),
            menu_keys: self.config.shortcuts.menu_keys(),
            panel_keys: self.config.shortcuts.clone(),
            pack_icons: Some(self.config.active_game().assets.icons.clone()),
            pack_fonts: self.pack_fonts.clone(),
            family: values.family,
            nintendo_layout: values.nintendo_layout,
            motion: self.config.active_motion(),
            lite: self.config.performance.lite(),
        }
    }

    /// If the overlay window died (crashed, or no compositor yet), leave overlay mode so the
    /// controller isn't stuck driving something invisible. It restarts on next use.
    pub(super) fn check_overlay_process(&mut self) {
        let exited = self.overlay_process.as_mut().is_some_and(|c| c.try_wait().ok().flatten().is_some());
        if exited {
            self.overlay_process = None;
            if self.active.is_some() {
                log!("overlay window went away");
                // Nothing can answer the in-game menu's question now.
                if matches!(self.active, Some(Active::Settings(..))) {
                    self.discard_settings();
                } else {
                    self.close_overlay();
                }
            }
        }
    }

    /// Plays the active game's cue for an input in `overlay`, if it has one, and the buzz for it
    /// on the controller driving the overlay. A pick buzzes the default way unless the setup
    /// gives it a buzz of its own.
    pub(super) fn overlay_sound(&self, overlay: sound::SoundOverlay, feedback: Option<sound::Feedback>) {
        if let Some((f, spec)) = feedback.and_then(|f| Some((f, self.scope.sounds.cue(overlay, f)?))) {
            if self.injection.enabled() {
                log!(
                    "sound: {} {f:?} -> {} volume={} pitch={} length={} file={:?}",
                    overlay.key(),
                    spec.kind.label().to_lowercase(),
                    spec.volume,
                    spec.pitch,
                    spec.length,
                    spec.file
                );
            }
            self.sounds.play(&spec);
        }
        let Some(dev) = self.overlay_driver().and_then(|id| self.devices.get(&id)).filter(|dev| dev.ff.is_some()) else { return };
        match feedback.and_then(|f| self.scope.rumble.cue(overlay, f)) {
            Some(cue) => rumble::cue(dev.path.clone(), cue),
            // A pick can be felt too, on the controller driving the overlay.
            None if feedback == Some(sound::Feedback::Pick) && self.config.pick_rumble => rumble::pick(dev.path.clone()),
            None => {}
        }
    }
}
