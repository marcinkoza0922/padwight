//! The sidebar, and a game's page (its Details tab is in `details.rs`).

use iced::widget::{column, row};

use super::*;

/// "from <pack> 1.2 by <author>" for a game made from a pack.
pub(super) fn origin_note(game: &Game) -> Option<String> {
    let origin = game.origin.as_ref()?;
    let by = if game.pack.author.is_empty() { String::new() } else { format!(" by {}", game.pack.author) };
    let source = if origin.library { "library" } else { "pack" };
    Some(format!("from {source}, version {}{by}", origin.version))
}

impl App {
    #[expect(clippy::too_many_lines, reason = "predates the size lints")]
    pub(super) fn update_games(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TestRumble(path) => return call_ok(Request::TestRumble(path)),
            Message::TestTurn(px) => {
                self.message = Some(("Switch to your game: the mouse turns right in 3 seconds.".into(), false));
                return call_ok(Request::TestTurn(px));
            }
            Message::ToggleOverlay => return call_ok(Request::ToggleOverlay),
            Message::ToggleNumpad => return call_ok(Request::ToggleNumpad),
            Message::ToggleNumpadAppearance => self.numpad_appearance = !self.numpad_appearance,
            Message::ToggleSoundCues(overlay) => {
                if !self.open_sounds.remove(&overlay) {
                    self.open_sounds.insert(overlay);
                }
            }
            Message::ToggleSoundEdit(overlay, feedback) => {
                if !self.open_cue_edits.remove(&(overlay, feedback)) {
                    self.open_cue_edits.insert((overlay, feedback));
                }
            }
            Message::SetNumpadStyle(style) => self.config.numpad_style = style,
            Message::ToggleMediaAppearance => self.media_appearance = !self.media_appearance,
            Message::SetMediaStyle(style) => self.config.media_style = style,
            Message::ToggleInGameMenuAppearance => self.menu_appearance = !self.menu_appearance,
            Message::SetInGameMenuStyle(style) => self.config.menu_style = style,
            Message::CalibrateGyro(path) => {
                self.message = Some(("Calibrating gyro: keep the controller still for 2 seconds.".into(), false));
                return call_ok(Request::CalibrateGyro(path));
            }
            Message::OpenSystemCheck | Message::RecheckSystem | Message::CopyFix(_) => return self.update_system_check(&message),
            Message::SelectPage(page) => {
                // Also how a dialog's "Open it" leaves it.
                self.dialog = None;
                match page {
                    Page::Game(key) => self.show_game(key),
                    other => {
                        self.page = other;
                        self.reset_page_state();
                    }
                }
                self.sync_motion_demo();
            }
            Message::SetGameSearch(search) => self.game_search = search,
            Message::SelectGameTab(tab) => {
                self.game_tab = tab;
                self.found = None;
                self.sync_motion_demo();
            }
            Message::GoToItem(kind, name) => return self.go_to_item(kind, &name),
            Message::SelectChapter(chapter) => self.manual_chapter = chapter,
            Message::OpenTutorial(file) => {
                self.manual_chapter = manual::chapter_of(file).unwrap_or(self.manual_chapter);
                self.manual_query.clear();
                self.page = Page::Manual;
            }
            Message::SetSettingsSearch(query) => self.settings_query = query,
            Message::SelectSettingsTab(tab) => {
                self.shortcuts_ui.tab = tab;
                self.shortcuts_ui.listening = None;
            }
            Message::SetShortcut(s, input) => {
                self.shortcuts_ui.listening = None;
                self.set_shortcut(s, input);
            }
            Message::ResetShortcut(s) => self.config.shortcuts.reset(s),
            Message::ResetAllShortcuts => self.config.shortcuts = Default::default(),
            Message::SetMouseModeExtras(extras) => self.config.mouse_mode = extras,
            Message::ListenShortcut(at) => self.shortcuts_ui.listening = at,
            Message::SetManualSearch(query) => self.manual_query = query,
            Message::SetButtonSearch(query) => self.button_query = query,
            Message::ManualLink(link) => {
                if let Some(chapter) = manual::chapter_of(&link) {
                    self.manual_chapter = chapter;
                }
            }
            Message::SystemMode(mode) => self.theme_mode = mode,
            Message::RenameGame(name) => {
                let Some(old) = self.game_key().map(str::to_string) else { return Task::none() };
                let taken = self.config.games.iter().any(|g| g.name == name && g.name != old);
                if !taken {
                    self.game_mut().name = name.clone();
                    if let Some(d) = self.config.auto_switch.default_profile.as_mut().filter(|d| d.game.as_ref() == Some(&old)) {
                        d.game = Some(name.clone());
                    }
                    let rename = Rename::Game { old, new: name.clone() };
                    self.config.active = follow_renames(std::slice::from_ref(&rename), self.config.active.clone());
                    self.renames.push(rename);
                    self.page = Page::Game(Some(name));
                }
            }
            Message::SetAutoSwitch(on) => self.config.auto_switch.enabled = on,
            Message::SetDefaultProfile(choice) => self.config.auto_switch.default_profile = choice.0,
            Message::AddRule(window) => {
                let profile = self.profile().map(|p| p.name.clone()).unwrap_or_default();
                let rule = match window {
                    Some(w) => crate::focus::rule_for_window(&self.config, &w, profile),
                    None => Rule::new(RuleKind::Executable, "", profile),
                };
                self.game_mut().rules.push(rule);
            }
            Message::RemoveRule(i) => {
                if i < self.game().rules.len() {
                    self.game_mut().rules.remove(i);
                }
            }
            Message::SetRuleKind(i, kind) => {
                if let Some(r) = self.game_mut().rules.get_mut(i) {
                    r.kind = kind;
                }
            }
            Message::SetRuleValue(i, value) => {
                if let Some(r) = self.game_mut().rules.get_mut(i) {
                    r.value = value;
                }
            }
            Message::SetRuleProfile(i, profile) => {
                if let Some(r) = self.game_mut().rules.get_mut(i) {
                    r.profile = profile;
                }
            }
            Message::SetRuleEnabled(i, on) => {
                if let Some(r) = self.game_mut().rules.get_mut(i) {
                    r.enabled = on;
                }
            }
            Message::SetInfoGlyphs(family) => self.config.info_glyphs = family,
            Message::SetAppearance(appearance) => self.config.appearance = appearance,
            Message::SetNintendoLayout(on) => {
                let Some(id) = self.layout_controller().map(|c| c.id.clone()) else { return Task::none() };
                return Task::batch([call_ok(Request::SetNintendoLayout { id, on }), Task::done(Message::Poll)]);
            }
            Message::SetGameNintendoLayout(choice) => self.game_mut().nintendo_layout = choice,
            Message::SetGameControllers(support) => self.game_mut().controllers = Some(support),
            Message::SetGameSounds(sounds) => self.game_mut().sounds = Some(sounds),
            Message::SetSounds(sounds) => self.config.sounds = sounds,
            Message::ClearGameSounds => self.game_mut().sounds = None,
            Message::PreviewSound(spec) => self.sounds.play(&spec),
            Message::SetKeyboardStyle(style) => self.config.keyboard_style = style,
            Message::SetFullKeyboard(full) => self.config.full_keyboard = full,
            Message::SetRumbleStrength(strength) => self.config.rumble_strength = strength,
            Message::SetPickRumble(on) => self.config.pick_rumble = on,
            Message::SetRealtimeInput(on) => self.config.realtime_input = on,
            Message::SetFastTick(on) => self.config.fast_tick = on,
            Message::SetPeekTrigger(t) => self.config.peek_trigger = t,
            Message::SetOverlayFont(font) => {
                self.config.overlay_font = font;
                self.font_menu = None;
            }
            Message::SetColorblindTones(on) => self.config.colorblind_tones = on,
            Message::SetPerformance(performance) => self.config.performance = performance,
            Message::SetMotion(kind, style) => {
                self.config.motion.set(kind, style);
                self.motion_demo.show(style);
            }
            Message::MotionDemoStyle(style) => self.motion_demo.show(style),
            Message::MotionTick(now) => self.motion_demo.tick(now),
            Message::SetGameMotion(kind, style) => {
                let mut set = self.game().motion.unwrap_or(self.config.motion);
                set.set(kind, style);
                self.game_mut().motion = Some(set);
                self.motion_demo.show(style);
            }
            Message::OwnGameMotion(on) => {
                self.game_mut().motion = on.then_some(self.config.motion);
                self.sync_motion_demo();
            }
            Message::SetGameOverlayFont(font) => {
                self.game_mut().overlay_font = font;
                self.font_menu = None;
            }
            Message::ToggleFontMenu(slot) => self.font_menu = if self.font_menu == Some(slot) { None } else { Some(slot) },
            Message::SetGameOverlayStyle(layout, style) => self.set_game_overlay_style(layout, style),
            Message::SetGameMediaStyle(style) => self.game_mut().media_style = style,
            Message::SetGameMenuStyle(style) => self.game_mut().menu_style = style,
            Message::SetIgnored(name, ignored) => {
                // Saving the defaults shown in place of an unreadable file would write over it.
                if self.refuse_unreadable() {
                    return Task::none();
                }
                // Applies immediately, independent of unsaved profile edits.
                for c in [&mut self.config, &mut self.saved] {
                    c.ignored_devices.retain(|n| n != &name);
                    if ignored {
                        c.ignored_devices.push(name.clone());
                    }
                }
                return self.push(self.saved.clone());
            }
            other => return self.update_items(other),
        }
        Task::none()
    }

    /// False only when every controller in use has on/off triggers (e.g. Switch pads), in
    /// which case pull-depth settings are hidden. With nothing detected, assume analog.
    pub(super) fn analog_triggers(&self) -> bool {
        let Some(status) = &self.status else { return true };
        let mut in_use = status.devices.iter().filter(|d| !d.ignored).peekable();
        in_use.peek().is_none() || in_use.any(|d| d.analog_triggers)
    }

    /// Whether any managed controller has a gyro (only affects hints, not what can be set).
    pub(super) fn any_gyro(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.devices.iter().any(|d| d.gyro))
    }

    /// Whether any managed controller has a touchpad (only affects hints).
    pub(super) fn any_touchpad(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.devices.iter().any(|d| d.touchpad))
    }

    /// Whether any managed controller has back paddles (only affects hints).
    pub(super) fn any_paddles(&self) -> bool {
        self.status.as_ref().is_some_and(|s| s.devices.iter().any(|d| d.paddles))
    }

    /// `p` with the mode and layers the daemon says are on right now (they belong to the active
    /// game).
    pub(super) fn with_active_layers(&self, p: &Profile) -> Profile {
        let layers = self.status.as_ref().map(|s| s.active_layers.as_slice()).unwrap_or_default();
        let game = self.config.active_game();
        let mode = self.status.as_ref().and_then(|s| s.active_mode.as_deref()).and_then(|m| self.config.mode_for(game, m));
        p.with_layers(mode.iter().map(|m| &**m).chain(game.layers_named(layers)))
    }

    /// Overview and Settings, then General and the games, with a search over the games.
    pub(super) fn view_sidebar(&self) -> Element<'_, Message> {
        let entry = |label: String, page: Page, note: Option<String>| -> Element<'_, Message> {
            let selected = self.page == page;
            let mut line = row![text(label).size(15).width(Length::Fill)].spacing(6).align_y(Alignment::Center);
            if let Some(note) = note {
                // The dot alone doesn't say what it means, so it says so on hover.
                let why = if note.contains('●') { "In use: the active profile is one of this setup's." } else if note.contains('⚠') { "One of its profiles uses what opens the Guide overlay." } else { "A newer version is in the library." };
                line = line.push(tooltip(
                    text(note).size(12).color(style_accent()),
                    container(text(why).size(13)).padding(8).style(style::tooltip),
                    tooltip::Position::Right,
                ));
            }
            button(line)
                .width(Length::Fill)
                .padding([6, 10])
                .style(style::nav(selected))
                .on_press(Message::SelectPage(page))
                .into()
        };
        let active_game = self.config.active_ref().game;
        let playing = |key: &Option<String>| (&active_game == key && self.status.is_some()).then_some("●");
        // A setup whose profile uses what opens the Guide overlay gets a warning, with what else it shows.
        let warned = |game: &Game, note: Option<&str>| -> Option<String> {
            let conflicts = !self.config.shortcut_conflicts(game).is_empty();
            match (conflicts, note) {
                (true, Some(note)) => Some(format!("⚠ {note}")),
                (true, None) => Some("⚠".into()),
                (false, note) => note.map(str::to_string),
            }
        };
        let updates: Vec<&str> = library::updates(&self.config, self.library).into_iter().map(|(name, _)| name).collect();

        let mut col = column![
            entry("Overview".into(), Page::Overview, None),
            entry("App settings".into(), Page::Settings, None),
            entry("Manual".into(), Page::Manual, None),
            rule::horizontal(1),
            text("Setups").size(13).color(MUTED_COLOR),
            entry("General".into(), Page::Game(None), warned(&self.config.general, playing(&None))),
        ]
        .spacing(6);
        // General is always listed, so the search box and the empty note are about the games.
        if self.config.games.is_empty() {
            col = col.push(text("No game setups yet.").size(13).color(MUTED_COLOR));
        } else {
            col = col.push(field("Search setups…", &self.game_search).on_input(Message::SetGameSearch).size(14));
        }
        let search = self.game_search.trim().to_lowercase();
        let mut games: Vec<&Game> = self.config.games.iter().filter(|g| g.name.to_lowercase().contains(&search)).collect();
        games.sort_by_key(|g| g.name.to_lowercase());
        for g in games {
            let key = Some(g.name.clone());
            let note = if updates.contains(&g.name.as_str()) { Some("update") } else { playing(&key) };
            let label = if g.name.is_empty() { "(unnamed)".to_string() } else { g.name.clone() };
            col = col.push(entry(label, Page::Game(key), warned(g, note)));
        }
        // A sidebar entry in the accent color rather than a filled button, which would look like
        // the search field above it.
        col = col.push(
            button(text("+ Add setup").color(style_accent()))
                .width(Length::Fill)
                .padding([6, 10])
                .style(style::nav(false))
                .on_press(Message::OpenAddGame),
        );
        container(scrollable(col.padding(12)).height(Length::Fill)).width(SIDEBAR_WIDTH).into()
    }

    /// A game's (or General's) page: its name and sub-tabs.
    pub(super) fn view_game(&self) -> Element<'_, Message> {
        let general = self.game_key().is_none();
        let game = self.game();
        let names = self.names();
        let mut title = row![text(if general { "General" } else { game.name.as_str() }).size(26)]
            .spacing(12)
            .align_y(Alignment::Center);
        if let Some(note) = origin_note(game) {
            title = title.push(text(note).size(13).color(MUTED_COLOR));
        }
        // Every setup's title has an ⓘ saying what the page holds.
        title = title.push(help(if general {
            "Profiles for no particular game, such as the desktop or a plain gamepad.".into()
        } else {
            "This game's profiles and the modes, layers, macros, cycles, menus and overlays they share. Rules on the \
             Details tab switch to it when the game has focus."
                .into()
        }));

        let profile_issue = self.profile().is_some_and(|p| ProfileTab::ALL.iter().any(|t| section_has_problem(p, *t, &names)));
        let reachable = reachable_menus(&self.config, game);
        let issue = |tab: GameTab| match tab {
            GameTab::Profiles => profile_issue,
            GameTab::Modes => modes_problem(game, &names).is_some(),
            GameTab::Layers => layers_problem(game, &names).is_some(),
            GameTab::Macros => macros_have_problem(self.macros(), &names),
            GameTab::Cycles => self.cycles_have_problem(&names),
            GameTab::Menus => menus_have_problem(self.menus(), &reachable, &names),
            GameTab::Info => info_has_problem(self.infos()),
            GameTab::Logs => self.logs_have_problem(),
            GameTab::Details => !general && (rules_problem(game).is_some() || game.name.trim().is_empty()),
        };
        let family = self.glyph_family();
        let segments: Vec<_> = GameTab::ALL
            .into_iter()
            .filter(|t| !(general && *t == GameTab::Details))
            .map(|tab| {
                let label = if issue(tab) { format!("{tab}  ⚠") } else { tab.to_string() };
                segment(tab.glyph(family), label, 15.0, self.game_tab == tab, Message::SelectGameTab(tab))
            })
            .collect();
        let tab = if general && self.game_tab == GameTab::Details { GameTab::Profiles } else { self.game_tab };
        let body = match tab {
            GameTab::Profiles => self.view_profile_tab(&names),
            GameTab::Modes => self.view_layers(layers::Shift::Mode, &names),
            GameTab::Layers => self.view_layers(layers::Shift::Layer, &names),
            GameTab::Macros => self.view_macros(&names),
            GameTab::Cycles => self.view_cycles(&names),
            GameTab::Menus => self.view_menus(&names, &reachable),
            GameTab::Info => self.view_infos(),
            GameTab::Logs => self.view_logs(),
            GameTab::Details => self.view_details(),
        };
        column![title, segment_bar(segments), body].spacing(16).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn games_are_renamed_deleted_and_added_empty() {
        let mut app = with_game();
        app.config.auto_switch.default_profile = Some(ProfileRef::new(Some("Doom"), "Menus"));
        let _ = app.update(Message::RenameGame("Doom II".into()));
        assert_eq!(app.page, Page::Game(Some("Doom II".into())));
        assert_eq!(app.config.auto_switch.default_profile, Some(ProfileRef::new(Some("Doom II"), "Menus")));
        assert_eq!(app.game().name, "Doom II");

        let _ = app.update(Message::AskDeleteGame);
        assert!(matches!(app.dialog, Some(Dialog::DeleteGame(ref n)) if n == "Doom II"));
        let _ = app.update(Message::ConfirmDeleteGame);
        assert!(app.config.games.is_empty() && app.dialog.is_none());
        assert_eq!(app.page, Page::Game(None));
        assert_eq!(app.config.auto_switch.default_profile, None);

        let _ = app.update(Message::AddEmptyGame(Template::Strategy));
        assert_eq!(app.page, Page::Game(Some("New setup".into())));
        assert_eq!(app.game().profiles[0].name, "Strategy");
        assert_eq!(app.game_tab, GameTab::Details);
        let _ = app.update(Message::AddEmptyGame(Template::Gamepad));
        assert_eq!(app.config.games[1].name, "New setup 2");
    }
}
