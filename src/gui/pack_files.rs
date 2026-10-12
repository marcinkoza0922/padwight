//! A setup's own files: the icons `{icon:name}` can draw and the sounds the overlays' cues can
//! play, which travel in its pack. They work only in this setup.

use iced::widget::{button, container, row, space, text};

use super::*;

/// Which kind of file is being added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileKind {
    Icon,
    Sound,
    Font,
}

impl App {
    pub(super) fn update_pack_files(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddPackFile(kind) => return Task::perform(pick_file(kind), move |picked| Message::PackFilePicked(kind, picked)),
            Message::PackFilePicked(kind, picked) => match picked {
                None => {}
                Some(Err(e)) => self.message = Some((format!("Can't add that file: {e}"), true)),
                Some(Ok((name, bytes))) => {
                    let assets = &mut self.game_mut().assets;
                    let added = match kind {
                        FileKind::Icon => String::from_utf8(bytes).map_err(|_| "not an SVG".to_string()).and_then(|svg| assets.add_icon(&name, &svg)),
                        FileKind::Sound => assets.add_sound(&name, &bytes),
                        FileKind::Font => assets.add_font(&name, &bytes),
                    };
                    self.message = Some(match added {
                        Ok(()) => (format!("Added “{name}”. Use it with {}.", usage(kind, &name)), false),
                        Err(e) => (format!("Can't add “{name}”: {e}"), true),
                    });
                }
            },
            Message::RemovePackIcon(name) => {
                self.game_mut().assets.icons.remove(&name);
            }
            Message::RemovePackFont(name) => {
                self.game_mut().assets.fonts.remove(&name);
            }
            Message::RemovePackSound(name) => {
                self.game_mut().assets.sounds.remove(&name);
            }
            Message::PlayPackSound(name) => {
                let spec = SoundSpec { file: Some(crate::sound::SoundFile::new(&name)), ..SoundSpec::new(SoundKind::Click, 0.5, 1.0) };
                self.sounds.play(&spec);
            }
            other => return self.update_tokens(other),
        }
        Task::none()
    }

    /// The card listing the setup's own icons and sounds, with buttons to add more.
    pub(super) fn view_pack_files(&self) -> Element<'_, Message> {
        let assets = &self.game().assets;
        let font = self.preview_font();
        let mut rows: Vec<Element<'_, Message>> = vec![
            text("Icons, sounds and fonts go in the pack when you export it, and work only in this setup. Use an icon as {icon:name} in any text, a font as {font:name}…{/font} or as the overlay font, and pick a sound for an overlay's cues under Sounds.")
                .size(13)
                .color(MUTED_COLOR)
                .into(),
        ];
        for name in assets.icons.keys() {
            rows.push(
                row![
                    container(crate::overlay::draw::chip(&crate::info::preview(&format!("{{icon:{name}}}"), PadFamily::Xbox), font)).width(60),
                    text(format!("{{icon:{name}}}")).font(iced::Font::MONOSPACE).size(13).width(Length::Fill),
                    button(text("Remove").size(13)).style(style::quiet_danger).on_press(Message::RemovePackIcon(name.clone())),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            );
        }
        for name in assets.fonts.keys() {
            rows.push(
                row![
                    container(crate::overlay::draw::chip(&crate::info::preview(&format!("{{font:{name}}}Aa Bb 123{{/font}}"), PadFamily::Xbox), font)).width(150),
                    text(format!("{{font:{name}}}")).font(iced::Font::MONOSPACE).size(13).width(Length::Fill),
                    button(text("Remove").size(13)).style(style::quiet_danger).on_press(Message::RemovePackFont(name.clone())),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            );
        }
        for name in assets.sounds.keys() {
            rows.push(
                row![
                    button(text("▶ Play").size(13)).style(style::secondary).on_press(Message::PlayPackSound(name.clone())),
                    text(format!("♪ {name}")).size(13).width(Length::Fill),
                    button(text("Remove").size(13)).style(style::quiet_danger).on_press(Message::RemovePackSound(name.clone())),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            );
        }
        if assets.is_empty() {
            rows.push(text("None yet.").size(13).color(MUTED_COLOR).into());
        }
        rows.push(
            row![
                button(text("Add icon…").size(13)).style(style::secondary).on_press(Message::AddPackFile(FileKind::Icon)),
                button(text("Add sound…").size(13)).style(style::secondary).on_press(Message::AddPackFile(FileKind::Sound)),
                button(text("Add font…").size(13)).style(style::secondary).on_press(Message::AddPackFile(FileKind::Font)),
                space::horizontal(),
                text("An SVG icon, a short WAV, or a TTF/OTF font").size(12).color(MUTED_COLOR),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        );
        section("Pack files", Some("Your own icons, sound cues and fonts. Draw an icon that fits each menu item, and add a sound that suits the game, when none of the built-in ones do.".into()), rows)
    }
}

fn usage(kind: FileKind, name: &str) -> String {
    match kind {
        FileKind::Icon => format!("{{icon:{name}}}"),
        FileKind::Sound => "the sound pickers under Sounds".to_string(),
        FileKind::Font => format!("{{font:{name}}}…{{/font}}, or as an overlay font"),
    }
}

/// Asks for an icon or a sound file and reads it, named by its file name without the extension;
/// `None` if the user cancelled.
async fn pick_file(kind: FileKind) -> Option<Result<(String, Vec<u8>), String>> {
    tokio::task::spawn_blocking(move || {
        let (title, label, extensions): (&str, &str, &[&str]) = match kind {
            FileKind::Icon => ("Add an icon", "SVG icon", &["svg"]),
            FileKind::Sound => ("Add a sound", "WAV sound", &["wav"]),
            FileKind::Font => ("Add a font", "Font", &["ttf", "otf"]),
        };
        let path = rfd::FileDialog::new().set_title(title).add_filter(label, extensions).pick_file()?;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().replace(' ', "-");
        Some(std::fs::read(&path).map(|bytes| (name, bytes)).map_err(|e| format!("{}: {e}", path.display())))
    })
    .await
    .unwrap_or_else(|e| Some(Err(e.to_string())))
}
