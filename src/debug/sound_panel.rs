//! The debug window's sounds: any kind of cue with any volume, pitch and length, every kind in
//! turn, or an overlay's cues as a visit to it plays them, the same as `padwight debug sound`.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use iced::{
    Alignment, Element,
    widget::{button, column, pick_list, row, slider, text},
};

use super::sound::{play_kind, visit};
use crate::sound::{SoundKind, SoundOverlay, SoundSpec, Sounds};

pub struct SoundPanel {
    spec: SoundSpec,
    sounds: Sounds,
    /// Set while a run of cues plays on its own thread; another waits for it to end.
    busy: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy)]
pub enum SoundMessage {
    Kind(SoundKind),
    Volume(f32),
    Pitch(f32),
    Length(f32),
    Play,
    /// Every kind in turn, with these settings.
    Every,
    Visit(SoundOverlay),
}

impl SoundPanel {
    pub fn new() -> Self {
        SoundPanel { spec: SoundSpec::new(SoundKind::default(), 0.5, 1.0), sounds: Sounds::start(), busy: Arc::default() }
    }

    pub fn update(&mut self, message: SoundMessage) {
        match message {
            SoundMessage::Kind(kind) => {
                self.spec.kind = kind;
                self.sounds.play(&self.spec);
            }
            SoundMessage::Volume(v) => self.spec.volume = v,
            SoundMessage::Pitch(v) => self.spec.pitch = v,
            SoundMessage::Length(v) => self.spec.length = v,
            SoundMessage::Play => self.sounds.play(&self.spec),
            SoundMessage::Every => {
                let spec = self.spec;
                self.in_turn(move |sounds| SoundKind::ALL.into_iter().for_each(|kind| play_kind(sounds, SoundSpec { kind, ..spec })));
            }
            SoundMessage::Visit(overlay) => self.in_turn(move |sounds| visit(sounds, overlay)),
        }
    }

    /// Plays a run of cues on its own thread, so the window keeps drawing while it waits between them.
    fn in_turn(&self, run: impl FnOnce(&Sounds) + Send + 'static) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let busy = self.busy.clone();
        std::thread::spawn(move || {
            run(&Sounds::start());
            busy.store(false, Ordering::SeqCst);
        });
    }

    pub fn view(&self) -> Element<'_, SoundMessage> {
        let spec = self.spec;
        let knob = |name: &'static str, value: f32, range: std::ops::RangeInclusive<f32>, set: fn(f32) -> SoundMessage| {
            row![text(name).size(13), slider(range, value, set).step(0.05_f32).width(120), text(format!("{value:.2}")).size(13).width(36)]
                .spacing(6)
                .align_y(Alignment::Center)
        };
        let cue = row![
            pick_list(SoundKind::ALL, Some(spec.kind), SoundMessage::Kind),
            button("Play").on_press(SoundMessage::Play),
            knob("Volume", spec.volume, 0.0..=1.0, SoundMessage::Volume),
            knob("Pitch", spec.pitch, 0.5..=2.0, SoundMessage::Pitch),
            knob("Length", spec.length, 0.5..=2.0, SoundMessage::Length),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap();
        let idle = !self.busy.load(Ordering::SeqCst);
        let runs = std::iter::once(button("Every kind").on_press_maybe(idle.then_some(SoundMessage::Every)).into()).chain(
            SoundOverlay::ALL.map(|overlay| button(text(overlay.label())).on_press_maybe(idle.then_some(SoundMessage::Visit(overlay))).into()),
        );
        column![
            text("Sounds").size(20),
            cue,
            row(runs).spacing(8).wrap(),
            text("Picking a kind plays it. Every kind plays them in turn with these settings; an overlay plays its default cues as a visit does (open, steps, a pick, close).").size(12),
        ]
        .spacing(8)
        .into()
    }
}
