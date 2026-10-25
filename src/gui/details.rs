//! A game's Details tab: its name, rules, controllers, button layout, sounds, motion and pack.

use iced::widget::{column, row};

use super::*;
use super::overlays::{motion_guide, motion_rows, silent_overlays_note};

impl App {
    /// The setup's overlay sounds: each overlay's cues, on or off, and what its steps and picks sound like.
    /// Its own sounds start as a copy of the App settings' ones.
    fn view_game_sounds(&self) -> Element<'_, Message> {
        let own = self.game().sounds;
        let global = self.config.sounds;
        let mut rows = own_rows("sounds", "the sounds set on the App settings page", own.is_some(), move |on| if on { Message::SetGameSounds(global) } else { Message::ClearGameSounds });
        if let Some(set) = own {
            rows.extend(self.sounds_off_note());
            rows.extend(SoundOverlay::ALL.iter().map(|&overlay| overlay_sounds(set, overlay, &self.open_sounds, &self.open_cue_edits, true)));
            rows.push(silent_overlays_note());
            rows.push(button(text("Turn them all off").size(13)).style(style::secondary).on_press(Message::SetGameSounds(set.silenced())).into());
        }
        section(
            "Overlay sounds",
            Some("Faint sounds as the cursor moves in an overlay, when something is picked, and as it opens and closes. A setup's own sounds start as a copy of App settings'. Any overlay can be silenced.".into()),
            rows,
        )
    }

    /// The setup's own overlay motion: each kind of overlay, starting as a copy of the global set.
    fn view_game_motion(&self) -> Element<'_, Message> {
        let own = self.game().motion;
        let mut rows = own_rows("motion", "the motion set on the App settings page", own.is_some(), Message::OwnGameMotion);
        if let Some(set) = own {
            rows.extend(motion_rows(set, Message::SetGameMotion));
            rows.push(self.view_motion_demo());
        }
        section(
            "Overlay motion",
            Some(format!("How this setup's overlays move while it's active. Each kind of overlay has its own style, starting as a copy of App settings'. {}", motion_guide())),
            rows,
        )
    }

    /// Which other controllers a setup supports: a physical one of a ticked kind makes the pad
    /// present itself as that kind while the setup is active.
    fn view_game_controllers(&self) -> Element<'_, Message> {
        let support = self.game().controllers.unwrap_or_default();
        let toggle = |label: &'static str, on: bool, set: fn(&mut crate::pad_identity::ControllerSupport, bool)| {
            checkbox(on).label(label).on_toggle(move |value| {
                let mut next = support;
                set(&mut next, value);
                Message::SetGameControllers(next)
            })
        };
        section(
            "Controllers this setup supports",
            Some("While this setup is active, the game sees a ticked kind as itself, with its own buttons and glyphs. Everything else appears as an Xbox controller.".into()),
            vec![
                text("Tick only kinds the game supports itself.").size(13).color(MUTED_COLOR).into(),
                toggle("DualShock 4", support.dualshock, |s, on| s.dualshock = on).into(),
                toggle("DualSense", support.dualsense, |s, on| s.dualsense = on).into(),
                toggle("Switch Pro Controller", support.switch_pro, |s, on| s.switch_pro = on).into(),
            ],
        )
    }

    /// A game's own choice of the Nintendo layout, or the one App settings gives.
    fn view_game_nintendo_layout(&self) -> Element<'_, Message> {
        let own = self.game().nintendo_layout;
        let mut rows = own_rows("button layout", "each controller's setting on the Profiles tab", own.is_some(), move |on| Message::SetGameNintendoLayout(on.then_some(false)));
        if let Some(on) = own {
            rows.push(toggler(on).label("Nintendo button layout").on_toggle(|on| Message::SetGameNintendoLayout(Some(on))).into());
        }
        section(
            "Button layout",
            Some("Nintendo layout for this setup's glyphs. Otherwise each controller's setting on the Profiles tab applies.".into()),
            rows,
        )
    }

    /// A game's name, rules, pack details, export and deletion.
    #[expect(clippy::too_many_lines, clippy::cognitive_complexity, reason = "predates the size lints")]
    pub(super) fn view_details(&self) -> Element<'_, Message> {
        let game = self.game();
        let names: Vec<String> = game.profiles.iter().map(|p| p.name.clone()).collect();
        let mut rules = column![].spacing(8);
        for (i, r) in game.rules.iter().enumerate() {
            let placeholder = match r.kind {
                RuleKind::Executable => "e.g. eldenring.exe or Folder/game.exe",
                RuleKind::SteamAppId => "e.g. 1245620",
                RuleKind::WindowClass => "e.g. steam_app_1245620",
            };
            let mut line = row![
                checkbox(r.enabled).on_toggle(move |on| Message::SetRuleEnabled(i, on)),
                text("When"),
                dropdown(RuleKind::ALL, Some(r.kind), move |k| Message::SetRuleKind(i, k)).width(150),
                text("is"),
                field(placeholder, &r.value).on_input(move |v| Message::SetRuleValue(i, v)).width(200),
                text("use"),
                dropdown(names.clone(), Some(r.profile.clone()), move |p| Message::SetRuleProfile(i, p)).width(160),
                button(text("✕").size(13)).style(style::secondary).on_press(Message::RemoveRule(i)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if !names.contains(&r.profile) {
                line = line.push(text("missing profile").size(12).color(ERROR_COLOR));
            }
            if let Some(other) = self.config.games.iter().find(|g| g.name != game.name && g.rules.iter().any(|o| o.enabled && o.same_match(r))) {
                line = line.push(text(format!("also in {}", other.name)).size(12).color(MUTED_COLOR));
            }
            // In a narrow window the end of a rule goes onto a second line rather than out of view.
            rules = rules.push(line.wrap().vertical_spacing(8));
        }
        if game.rules.is_empty() {
            rules = rules.push(text("No rules yet. This setup only starts when you pick it yourself.").size(13).color(MUTED_COLOR));
        }
        rules = rules.push(
            row![
                button(text("+ Add rule").size(13)).style(style::secondary).on_press(Message::AddRule(None)),
                space::horizontal(),
                tutorial_link("tutorial-first-setup.md"),
            ]
                .spacing(8)
                .align_y(Alignment::Center),
        );
        // This app's own window is never a game.
        let recent: Vec<_> =
            self.status.iter().flat_map(|s| &s.recent_windows).filter(|w| w.exe != env!("CARGO_PKG_NAME")).collect();
        if !recent.is_empty() {
            let mut list = column![
                text("Recently focused windows. \"+ Rule\" adds a rule for the profile open on the Profiles tab.").size(13).color(MUTED_COLOR)
            ]
            .spacing(6);
            for w in recent {
                let mut details = Vec::new();
                if !w.exe.is_empty() {
                    details.push(w.exe.clone());
                }
                if let Some(id) = &w.steam_app_id {
                    details.push(format!("Steam {id}"));
                }
                if !w.class.is_empty() {
                    details.push(format!("class {}", w.class));
                }
                // Untitled windows (e.g. plasmashell) go by their executable instead.
                let name = [&w.title, &w.exe, &w.class].into_iter().find(|s| !s.is_empty()).map_or("Untitled window", |s| s.as_str());
                let title: String = name.chars().take(60).collect();
                list = list.push(
                    row![
                        column![text(title).size(14), text(details.join(" · ")).size(12).color(MUTED_COLOR)].width(Length::Fill),
                        button(text("+ Rule").size(13)).style(style::secondary).on_press(Message::AddRule(Some(w.clone()))),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            rules = rules.push(list);
        }

        let mut pack_rows: Vec<Element<'_, Message>> = Vec::new();
        if let Some(origin) = &game.origin {
            let source = if origin.library { "the built-in library" } else { "a pack file" };
            let by = if game.pack.author.is_empty() { String::new() } else { format!(" by {}", game.pack.author) };
            pack_rows.push(text(format!("Added from {source}: version {}{by}.", origin.version)).into());
            if !self.edited.is_empty() {
                pack_rows.push(text(format!("Changed since: {}.", self.edited.join(", "))).size(13).color(MUTED_COLOR).into());
            }
            let update = library::updates(&self.config, self.library).into_iter().find(|(n, _)| *n == game.name);
            if let Some(entry) = update.map(|(_, i)| &self.library[i]) {
                pack_rows.push(
                    row![
                        text(format!("The library has version {}.", entry.version)).color(style_accent()),
                        button(text("Update…").size(13)).on_press(Message::UpdateFromLibrary(game.name.clone())),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .into(),
                );
            }
        } else if !game.pack.id.is_empty() {
            pack_rows.push(text(format!("You've exported this setup (version {}).", game.pack.version)).into());
        } else {
            pack_rows.push(text("Export this setup to share it as a .padpack file.").size(13).color(MUTED_COLOR).into());
        }
        if let Some(b) = &game.pack.based_on {
            pack_rows.push(text(format!("Based on {} {} by {}.", b.name, b.version, b.author)).size(13).color(MUTED_COLOR).into());
        }
        if !game.pack.description.is_empty() {
            pack_rows.push(text(game.pack.description.clone()).size(13).into());
        }
        pack_rows.push(row![button(text("Export…")).on_press(Message::OpenExport), tutorial_link("tutorial-sharing.md")].spacing(8).align_y(Alignment::Center).into());

        // The cards follow App settings' order, so a card is found in the same place on both.
        column![
            section(
                "Setup",
                Some("The name shown in the sidebar. Deleting the setup removes its profiles, items and rules on Save & apply.".into()),
                vec![
                    row![
                        labeled("Name", field("Setup name", &game.name).on_input(Message::RenameGame).width(260).into()),
                        space::horizontal(),
                        button(text("Delete setup")).style(style::quiet_danger).on_press(Message::AskDeleteGame),
                    ]
                    .align_y(Alignment::Center)
                    .into(),
                ],
            ),
            section(
                "Auto-switch rules",
                Some("A rule switches to its profile when a matching window has focus or a matching program is running. Untick a rule to turn it off.".into()),
                vec![rules.into()],
            ),
            self.view_game_nintendo_layout(),
            self.view_game_controllers(),
            self.view_game_font(),
            self.view_game_motion(),
            self.view_game_sounds(),
            self.view_game_overlays(),
            self.view_pack_files(),
            section(
                "Pack",
                Some("A pack is a setup saved as a .padpack file, to share or keep.".into()),
                pack_rows,
            ),
        ]
        .spacing(16)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn rules_from_windows_prefer_the_most_specific_identifier() {
        let w = |class: &str, exe: &str, steam: Option<&str>| WindowInfo {
            class: class.into(),
            exe: exe.into(),
            steam_app_id: steam.map(Into::into),
            ..Default::default()
        };
        let pick = |w: WindowInfo| {
            let r = crate::focus::rule_for_window(&Config::default(), &w, "P".into());
            (r.kind, r.value)
        };
        assert_eq!(pick(w("steam_app_1", "game.exe", Some("1"))), (RuleKind::SteamAppId, "1".into()));
        assert_eq!(pick(w("wine", "game.exe", None)), (RuleKind::Executable, "game.exe".into()));
        // Native exe names can be generic (java, python), so the class is preferred.
        assert_eq!(pick(w("Minecraft", "java", None)), (RuleKind::WindowClass, "Minecraft".into()));
        assert_eq!(pick(w("", "factorio", None)), (RuleKind::Executable, "factorio".into()));
    }

    #[test]
    fn rules_must_point_at_existing_profiles() {
        let mut app = with_game();
        let _ = app.update(Message::AddRule(None));
        assert!(app.validate().unwrap().contains("no Executable to match"), "{:?}", app.validate());
        let _ = app.update(Message::SetRuleValue(1, "x".into()));
        let _ = app.update(Message::SetRuleProfile(1, "Gone".into()));
        let err = app.validate().unwrap();
        assert!(err.starts_with("Doom: ") && err.contains("doesn't exist"), "{err}");
        let _ = app.update(Message::RemoveRule(1));
        let _ = app.update(Message::SetRuleEnabled(0, false));
        assert!(!app.game().rules[0].enabled);
        assert_eq!(app.validate(), None);
    }
}
