//! The sound picker: a window that lists every sound a cue can play by group, plays the one
//! clicked, and sets the cue it was opened from to it.

use iced::widget::{button, center, column, container, mouse_area, opaque, row, scrollable, space, text};

use super::{sounds::{SoundChoice, chosen}, *};
use crate::sound::Feedback;

/// The cue a pick sets: one of an overlay's, in the App settings' sounds or the setup's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SoundTarget {
    pub game: bool,
    pub overlay: SoundOverlay,
    pub feedback: Feedback,
}

/// The open picker.
pub(super) struct SoundPicker {
    target: SoundTarget,
    query: String,
}

impl App {
    pub(super) fn update_sound_picker(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenSoundPicker(target) => self.sound_picker = Some(SoundPicker { target, query: String::new() }),
            Message::CloseSoundPicker => self.sound_picker = None,
            Message::SoundSearch(query) => {
                if let Some(p) = &mut self.sound_picker {
                    p.query = query;
                }
            }
            Message::PickSound(choice) => {
                if let Some(target) = self.sound_picker.as_ref().map(|p| p.target) {
                    let spec = self.cue_of(target);
                    let spec = match choice {
                        SoundChoice::Kind(kind) => SoundSpec { kind, file: None, ..spec },
                        SoundChoice::File(file) => SoundSpec { file: Some(file), ..spec },
                    };
                    self.sounds.play(&spec);
                    return self.set_cue(target, spec);
                }
            }
            other => return self.update_chars(other),
        }
        Task::none()
    }

    /// The set of sounds `target` is in.
    fn sound_set(&self, game: bool) -> SoundSet {
        if game { self.game().sounds.unwrap_or(self.config.sounds) } else { self.config.sounds }
    }

    fn cue_of(&self, target: SoundTarget) -> SoundSpec {
        let sounds = self.sound_set(target.game).get(target.overlay);
        match target.feedback {
            Feedback::Step => sounds.step,
            Feedback::Pick => sounds.pick,
            Feedback::Open => sounds.open,
            Feedback::Close => sounds.close,
            Feedback::Mode => sounds.mode,
        }
    }

    fn set_cue(&mut self, target: SoundTarget, spec: SoundSpec) -> Task<Message> {
        let mut set = self.sound_set(target.game);
        let sounds = set.get(target.overlay);
        let sounds = match target.feedback {
            Feedback::Step => OverlaySounds { step: spec, ..sounds },
            Feedback::Pick => OverlaySounds { pick: spec, ..sounds },
            Feedback::Open => OverlaySounds { open: spec, ..sounds },
            Feedback::Close => OverlaySounds { close: spec, ..sounds },
            Feedback::Mode => OverlaySounds { mode: spec, ..sounds },
        };
        set.set(target.overlay, sounds);
        self.update(if target.game { Message::SetGameSounds(set) } else { Message::SetSounds(set) })
    }

    pub(super) fn view_sound_picker<'a>(&'a self, picker: &'a SoundPicker) -> Element<'a, Message> {
        let now = chosen(&self.cue_of(picker.target));
        let query = picker.query.trim().to_lowercase();
        let matches = |choice: &SoundChoice| query.is_empty() || choice.to_string().to_lowercase().contains(&query);
        let card = |choice: SoundChoice| {
            let on = choice == now;
            button(text(choice.to_string()).size(14).width(Length::Fill).align_x(Alignment::Center))
                .style(if on { style::primary } else { style::secondary })
                .width(Length::Fixed(120.0))
                .on_press(Message::PickSound(choice))
        };
        let group = |title: &'static str, blurb: &'static str, choices: Vec<SoundChoice>| -> Option<Element<'a, Message>> {
            let choices: Vec<SoundChoice> = choices.into_iter().filter(matches).collect();
            if choices.is_empty() {
                return None;
            }
            let cards = iced::widget::row(choices.into_iter().map(|c| card(c).into())).spacing(6).wrap().vertical_spacing(6);
            let head = row![text(title).size(15), text(blurb).size(12).color(MUTED_COLOR)].spacing(10).align_y(Alignment::Center);
            Some(container(column![head, cards].spacing(8)).padding(10).width(Length::Fill).style(style::inset).into())
        };
        let (recorded, synthesized): (Vec<_>, Vec<_>) = SoundKind::ALL.into_iter().partition(|k| k.is_recording());
        let own: Vec<SoundChoice> = crate::sound::pack_sound_names().into_iter().map(SoundChoice::File).collect();
        let mut groups = column![].spacing(10);
        for g in [
            group("Recordings", "Real sounds, free to use (CC0). Their length is their own.", recorded.into_iter().map(SoundChoice::Kind).collect()),
            group("Synthesized", "Made in code: pitch and length reshape them.", synthesized.into_iter().map(SoundChoice::Kind).collect()),
            group("This game's sounds", "Carried by this game's pack: they work in this game only.", own),
        ]
        .into_iter()
        .flatten()
        {
            groups = groups.push(g);
        }
        let dialog = container(
            column![
                text("Choose a sound").size(20),
                row![text("Now").size(13).color(MUTED_COLOR), text(now.to_string()).size(14)].spacing(10).align_y(Alignment::Center),
                field("Search sounds", &picker.query).on_input(Message::SoundSearch).width(Length::Fill),
                scrollable(groups).height(Length::Fixed(430.0)),
                row![
                    text("Each click plays the sound and sets the cue to it.").size(12).color(MUTED_COLOR),
                    space::horizontal(),
                    button(text("Done")).on_press(Message::CloseSoundPicker),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(12),
        )
        .padding(20)
        .width(Length::Fixed(720.0))
        .style(style::dialog);
        let backdrop = container(center(opaque(dialog)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style { background: Some(Color { a: 0.6, ..Color::BLACK }.into()), ..container::Style::default() });
        opaque(mouse_area(backdrop).on_press(Message::CloseSoundPicker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn a_pick_sets_the_cue_it_was_opened_from_and_nothing_else() {
        let mut app = with_game();
        let target = SoundTarget { game: false, overlay: SoundOverlay::Menu, feedback: Feedback::Pick };
        let before = app.config.sounds.get(SoundOverlay::Menu);
        let _ = app.update(Message::OpenSoundPicker(target));
        let _ = app.view();
        let _ = app.update(Message::SoundSearch("bell".into()));
        let _ = app.view();
        let _ = app.update(Message::PickSound(SoundChoice::Kind(SoundKind::Bell)));
        let after = app.config.sounds.get(SoundOverlay::Menu);
        assert_eq!(after.pick.kind, SoundKind::Bell);
        assert_eq!(after.step, before.step);
        assert_eq!(after.pick.volume, before.pick.volume);
        let _ = app.update(Message::CloseSoundPicker);
        assert!(app.sound_picker.is_none());
    }

    #[test]
    fn a_setup_s_own_sounds_are_set_by_a_pick_in_its_picker() {
        let mut app = with_game();
        let global = app.config.sounds;
        let _ = app.update(Message::SetGameSounds(global));
        let target = SoundTarget { game: true, overlay: SoundOverlay::Keyboard, feedback: Feedback::Step };
        let _ = app.update(Message::OpenSoundPicker(target));
        let _ = app.update(Message::PickSound(SoundChoice::Kind(SoundKind::Typewriter)));
        assert_eq!(app.game().sounds.unwrap().get(SoundOverlay::Keyboard).step.kind, SoundKind::Typewriter);
        assert_eq!(app.config.sounds, global);
    }
}
