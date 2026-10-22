//! The App settings page, and the controller list on the Overview.

use iced::widget::{column, row};

use super::*;
use crate::config::Performance;

/// An entry in the auto-switch "default profile" list.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DefaultChoice(pub(super) Option<ProfileRef>);

impl fmt::Display for DefaultChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(at) => at.fmt(f),
            None => f.write_str("(keep current profile)"),
        }
    }
}

impl App {
    /// Light, dark or the desktop's choice. Only the settings window follows it; the overlays keep
    /// their own colors.
    fn view_appearance(&self) -> Element<'_, Message> {
        let choice = |label: &'static str, appearance: Appearance| {
            button(text(label).size(14))
                .style(style::segment(self.config.appearance == appearance))
                .padding([5, 14])
                .on_press(Message::SetAppearance(appearance))
        };
        section(
            "Appearance",
            Some("Auto follows the desktop's light or dark setting.".into()),
            vec![
                container(row![choice("Auto", Appearance::Auto), choice("Light", Appearance::Light), choice("Dark", Appearance::Dark)].spacing(2))
                    .padding(3)
                    .style(style::segments)
                    .into(),
            ],
        )
    }

    /// The App settings page's sounds: the set every setup without its own set uses.
    fn view_default_sounds(&self) -> Element<'_, Message> {
        let set = self.config.sounds;
        // Silencing everything is Performance's Sounds switch, so this card has no button for it.
        let mut rows: Vec<Element<'_, Message>> = self.sounds_off_note().into_iter().collect();
        rows.extend(SoundOverlay::ALL.iter().map(|&overlay| overlay_sounds(set, overlay, &self.open_sounds, &self.open_cue_edits, false)));
        rows.push(overlays::silent_overlays_note());
        section(
            "Overlay sounds",
            Some("A faint sound as the cursor moves in an overlay, another when something is picked, and a softer one as it pops in and out. These apply to every setup that doesn't set its own on its Details tab. Any overlay can be turned off, and turning off Sounds under Performance silences them all.".into()),
            rows,
        )
    }

    /// While Performance has every sound off, a line over a sounds card's toggles saying that
    /// none of them plays.
    pub(super) fn sounds_off_note(&self) -> Option<Element<'_, Message>> {
        self.config.performance.quiet.then(|| {
            text("Sounds are off under App settings → Performance, so none of these play.").size(13).color(WARNING_COLOR).into()
        })
    }

    /// What every overlay can go without, on machines where drawing them costs too much. Each
    /// switch is on while the overlays have that thing, so "off" means off everywhere on the page.
    fn view_performance(&self) -> Element<'_, Message> {
        let p = self.config.performance;
        let switch = |has: bool, label: &'static str, what: &'static str, set: fn(&mut Performance, bool)| -> Element<'_, Message> {
            column![
                toggler(has).label(label).on_toggle(move |v| {
                    let mut p = p;
                    set(&mut p, v);
                    Message::SetPerformance(p)
                }),
                note(what),
            ]
            .spacing(2)
            .into()
        };
        let all = if p == Performance::lightest() {
            button(text("Turn them all back on").size(13)).style(style::secondary).on_press(Message::SetPerformance(Performance::default()))
        } else {
            button(text("Turn them all off").size(13)).style(style::secondary).on_press(Message::SetPerformance(Performance::lightest()))
        };
        section(
            "Performance",
            Some(
                "These apply to every overlay in every setup, whatever its own settings say. With a working \
                 graphics card, motion is the main cost: overlays redraw every frame while something moves. \
                 Without one (in a virtual machine, say), shadows and the other decorations cost the most."
                    .into(),
            ),
            vec![
                switch(!p.still, "Motion", "Overlays move as they open and close. Off, they appear and close at once, and redraw only when something changes.", |p, v| p.still = !v),
                switch(!p.plain, "Decorations", "Shadows, outlines, gradients and glows, the default ones included.", |p, v| p.plain = !v),
                switch(!p.quiet, "Sounds", "Overlay sounds. Off, every overlay is silent and the audio device is never opened.", |p, v| p.quiet = !v),
                switch(!p.no_loops, "Looping animations", "Glows pulse, and gradients and scanlines drift. Off, they stand still while motion styles still play.", |p, v| p.no_loops = !v),
                all.into(),
            ],
        )
    }

    pub(super) fn view_settings(&self) -> Element<'_, Message> {
        use shortcuts_tab::SettingsTab;
        let warned = !self.config.shortcuts.all_warnings().is_empty() || !self.opener_conflicts().is_empty();
        let mut tabs = row![].spacing(2);
        for tab in [SettingsTab::General, SettingsTab::Shortcuts] {
            let label = if tab == SettingsTab::Shortcuts && warned { format!("{tab}  ⚠") } else { tab.to_string() };
            tabs = tabs.push(
                button(text(label).size(15)).style(style::segment(self.shortcuts_ui.tab == tab)).padding([6, 16]).on_press(Message::SelectSettingsTab(tab)),
            );
        }
        let content = match self.shortcuts_ui.tab {
            SettingsTab::General => self.view_general_settings(),
            SettingsTab::Shortcuts => self.view_shortcuts(),
        };
        // A title like every setup page's, so pages start the same way.
        column![text("App settings").size(26), container(tabs).padding(3).style(style::segments), content].spacing(16).into()
    }

    fn view_general_settings(&self) -> Element<'_, Message> {
        // The ⓘ by the title, as on every card.
        let glyphs = section(
            "Info overlays",
            Some(
                "Glyphs follow the controller in use: Xbox, PlayStation or Nintendo labels. For a controller that \
                 can't be recognized, they're drawn like the fallback kind instead. Also used for previews."
                    .into(),
            ),
            vec![labeled(
                "Fallback glyphs",
                dropdown(PadFamily::ALL, Some(self.config.info_glyphs), Message::SetInfoGlyphs).width(SETTING_WIDTH).into(),
            )],
        );
        let query = self.settings_query.as_str();
        // Each card with the words it can be found by, besides the ones it shows.
        let cards: Vec<(&str, Element<'_, Message>)> = vec![
            ("appearance theme light dark auto desktop colors", self.view_appearance()),
            ("per-setup profiles switch automatically window rules no setup matches default profile", self.view_auto_switch()),
            ("overlay font typeface", self.view_font_card()),
            ("menu colors color-blind colorblind tint blue orange red", self.view_color_card()),
            ("motion animation overlay style", self.view_motion_card()),
            ("overlay sounds audio cue volume", self.view_default_sounds()),
            ("performance still plain quiet motion sounds decorations shadows looping animations graphics", self.view_performance()),
            ("on-screen keyboard full keyboard basic layout position", self.view_keyboard_card()),
            ("on-screen numpad number pad digits position", self.view_numpad_card()),
            ("media controls player panel music volume", self.view_media_card()),
            ("in-game menu guide button", self.view_in_game_menu_card()),
            ("info overlays fallback glyphs xbox playstation nintendo labels", glyphs),
            ("help tour guided welcome introduction getting started system check permissions uinput udev rules gyro doctor access", self.view_help_card()),
        ];
        let shown: Vec<Element<'_, Message>> = cards.into_iter().filter(|(words, _)| matches_query(query, words)).map(|(_, card)| card).collect();
        let body: Element<'_, Message> = if shown.is_empty() {
            text("No settings match.").color(MUTED_COLOR).into()
        } else {
            column(shown).spacing(16).into()
        };
        column![search_field("Search settings…", query, Message::SetSettingsSearch), body].spacing(16).into()
    }

    /// How strongly every controller rumbles, as a share of what games ask for.
    fn rumble_strength_row(&self) -> Element<'_, Message> {
        let strength = self.config.rumble_strength;
        row![
            text("Rumble strength").width(140),
            iced::widget::slider(0.0..=1.0, strength, Message::SetRumbleStrength).step(0.05_f32).width(240),
            text(format!("{:.0}%", strength * 100.0)).size(13).color(MUTED_COLOR),
            checkbox(self.config.pick_rumble).label("Pulse when picking an overlay item").on_toggle(Message::SetPickRumble),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap()
        .into()
    }

    pub(super) fn view_devices(&self) -> Element<'_, Message> {
        let mut list = column![text("Controllers").size(20)].spacing(8);
        match &self.status {
            Some(status) if !status.devices.is_empty() => {
                for d in &status.devices {
                    let state = if d.managed {
                        "remapping"
                    } else if d.ignored {
                        "ignored"
                    } else if !status.enabled {
                        "paused (remapping is off)"
                    } else {
                        "not remapped"
                    };
                    let state = if d.analog_triggers { state.to_string() } else { format!("{state} · on/off triggers") };
                    let state = if d.rumble { state } else { format!("{state} · no rumble") };
                    let state = if d.gyro { format!("{state} · gyro") } else { state };
                    let state = if d.paddles { format!("{state} · back paddles") } else { state };
                    let calibrate: Element<'_, Message> = if d.gyro {
                        button(text("Calibrate gyro").size(13))
                            .style(style::secondary)
                            .on_press(Message::CalibrateGyro(d.path.clone()))
                            .into()
                    } else {
                        space().into()
                    };
                    let id = d.id.clone();
                    let rumble: Element<'_, Message> = if d.rumble {
                        button(text("Test rumble").size(13))
                            .style(style::secondary)
                            .on_press(Message::TestRumble(d.path.clone()))
                            .into()
                    } else {
                        space().into()
                    };
                    list = list.push(
                        row![
                            column![text(&d.name), text(if d.path.is_empty() { state } else { format!("{} · {state}", d.path) }).size(12).color(MUTED_COLOR)]
                                .width(Length::Fill),
                            calibrate,
                            rumble,
                            checkbox(!d.ignored)
                                .label("Manage")
                                .on_toggle(move |on| Message::SetIgnored(id.clone(), !on)),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                    );
                }
            }
            Some(_) => list = list.push(text("No controllers detected.").color(MUTED_COLOR)),
            None => list = list.push(text(NEEDS_DAEMON).size(13).color(MUTED_COLOR)),
        }
        list = list.push(self.rumble_strength_row());
        list.into()
    }

    /// What the system check can fix about how Padwight reaches the controllers.
    pub(super) fn access_warnings(&self) -> Vec<String> {
        let Some(status) = &self.status else { return Vec::new() };
        let gyro = (!status.motion_access_denied.is_empty()).then(|| {
            let names = status.motion_access_denied.join(", ");
            format!("Can't use the gyro on {names}: Padwight isn't allowed to read its motion sensors.")
        });
        gyro.into_iter().chain(status.raw_readers.iter().map(crate::ipc::RawReaders::warning)).collect()
    }

    /// The access warnings, across the top of every page so they aren't missed. With none it is
    /// an empty space, so the page under it keeps its place in the widget tree (see `view`).
    pub(super) fn view_access_banner(&self) -> Element<'_, Message> {
        let warnings = self.access_warnings();
        if warnings.is_empty() {
            return space().height(0).into();
        }
        let lines = warnings.into_iter().fold(column![].spacing(4), |lines, w| lines.push(text(w).size(14)));
        let fix = button(text("Fix…").size(14)).style(style::primary).on_press(Message::OpenSystemCheck);
        container(row![text("⚠").size(22), lines.width(Length::Fill), fix].spacing(12).align_y(Alignment::Center))
            .padding([10, 20])
            .width(Length::Fill)
            .style(style::warning_banner)
            .into()
    }

    pub(super) fn view_auto_switch(&self) -> Element<'_, Message> {
        let auto = &self.config.auto_switch;
        let how = match self.status.as_ref().map(|s| s.focus_backend) {
            Some(backend) => match backend.desktop() {
                Some(desktop) => format!("Profiles switch as the focused window changes ({desktop})."),
                None => "Window tracking isn't available on this desktop, so rules apply while a matching \
                         process is running."
                    .to_string(),
            },
            None => NEEDS_DAEMON.to_string(),
        };
        let mut defaults = vec![DefaultChoice(None)];
        defaults.extend(
            self.config.all_games().flat_map(|(key, g)| g.profiles.iter().map(move |p| DefaultChoice(Some(ProfileRef::new(key, &p.name))))),
        );
        let default = DefaultChoice(auto.default_profile.clone());
        section(
            "Per-setup profiles",
            Some("On its Details tab, each setup's rules decide which windows switch to which of its profiles. When no setup's rule matches, the default profile is used.".into()),
            // The dropdown sits in the label column's layout, as every other setting's does.
            vec![
                toggler(auto.enabled).label("Switch profiles automatically").on_toggle(Message::SetAutoSwitch).into(),
                text(how).size(13).color(MUTED_COLOR).into(),
                labeled("Default profile", dropdown(defaults, Some(default), Message::SetDefaultProfile).width(SETTING_WIDTH).into()),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gui::tests::with_game, motion::{MotionSet, MotionStyle}};

    #[test]
    fn performance_overrides_every_setups_motion_and_sounds() {
        let mut app = with_game();
        let _ = app.update(Message::SelectPage(Page::Game(Some("Doom".into()))));
        let _ = app.update(Message::OwnGameMotion(true));
        let _ = app.update(Message::SetPerformance(Performance { still: true, quiet: true, ..Performance::default() }));
        assert_eq!(app.config.active_motion(), MotionSet::all(MotionStyle::Off));
        assert_eq!(app.config.scope().sounds, app.config.sounds.silenced());
        assert!(app.game().motion.is_some(), "the setup keeps its own motion for when performance allows it");
    }
}
