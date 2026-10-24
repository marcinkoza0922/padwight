//! The editor for an overlay's sounds, on the App settings page and a game's page.

use iced::widget::{column, row};

use super::*;
use super::sound_picker::SoundTarget;
use crate::sound::{Feedback, VOLUME_FLOOR_DB, decibels, volume_db};

/// One overlay's sounds: on or off, and (when on and opened) its step, pick, open and close cues.
pub(super) fn overlay_sounds<'a>(
    set: SoundSet,
    overlay: SoundOverlay,
    open_overlays: &HashSet<SoundOverlay>,
    open_edits: &HashSet<(SoundOverlay, Feedback)>,
    game: bool,
) -> Element<'a, Message> {
    let change: fn(SoundSet) -> Message = if game { Message::SetGameSounds } else { Message::SetSounds };
    let open = open_overlays.contains(&overlay);
    let current = set.get(overlay);
    let put = move |sounds: OverlaySounds| {
        let mut next = set;
        next.set(overlay, sounds);
        change(next)
    };
    let toggle = toggler(current.enabled).label(overlay.label()).on_toggle(move |on| put(OverlaySounds { enabled: on, ..current }));
    let mut head = row![container(toggle).width(220)].align_y(Alignment::Center);
    if current.enabled {
        head = head.push(disclosure("Adjust", open, Message::ToggleSoundCues(overlay)));
    }
    let mut block = column![head].spacing(6);
    if current.enabled && open {
        let cue = |label, feedback, spec, set: fn(OverlaySounds, SoundSpec) -> OverlaySounds| {
            let editing = open_edits.contains(&(overlay, feedback));
            let target = SoundTarget { game, overlay, feedback };
            cue_row(label, spec, editing, target, move |spec| put(set(current, spec)))
        };
        block = block.push(cue("Step", Feedback::Step, current.step, |s, step| OverlaySounds { step, ..s }));
        block = block.push(cue("Pick", Feedback::Pick, current.pick, |s, pick| OverlaySounds { pick, ..s }));
        block = block.push(cue("Open", Feedback::Open, current.open, |s, open| OverlaySounds { open, ..s }));
        block = block.push(cue("Close", Feedback::Close, current.close, |s, close| OverlaySounds { close, ..s }));
        if overlay == SoundOverlay::Guide {
            block = block.push(cue("Mode", Feedback::Mode, current.mode, |s, mode| OverlaySounds { mode, ..s }));
        }
    }
    block.into()
}

/// What a cue plays: one of the built-in sounds, or a sound file the setup's pack carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SoundChoice {
    Kind(SoundKind),
    File(crate::sound::SoundFile),
}

impl std::fmt::Display for SoundChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoundChoice::Kind(kind) => kind.fmt(f),
            SoundChoice::File(file) => file.fmt(f),
        }
    }
}

pub(super) fn chosen(spec: &SoundSpec) -> SoundChoice {
    spec.file.map_or(SoundChoice::Kind(spec.kind), SoundChoice::File)
}

/// How a knob shows its value.
enum Unit {
    Percent,
    Decibels,
    /// A volume in dB below full, whose bottom is silence: heard loudness follows dB, so a linear
    /// slider would crowd every quiet cue into its first few steps.
    Level,
}

/// One cue: its label, kind and preview, and a fold that holds every way to shape it: volume,
/// pitch and length, and under them bass, treble, drive, echo and reverb.
fn cue_row<'a>(label: &'static str, spec: SoundSpec, editing: bool, target: SoundTarget, put: impl Fn(SoundSpec) -> Message + Copy + 'a) -> Element<'a, Message> {
    let knob = |name: &'static str, value: f32, range: std::ops::RangeInclusive<f32>, unit: Unit, set: fn(&mut SoundSpec, f32)| {
        let (step, shown) = match unit {
            Unit::Percent => (5.0_f32, format!("{value:.0}%")),
            Unit::Decibels => (1.0, format!("{value:+.0} dB")),
            Unit::Level if value <= VOLUME_FLOOR_DB => (1.0, "Off".to_string()),
            Unit::Level => (1.0, format!("{value:.0} dB")),
        };
        row![
            text(name).width(56),
            slider(range, value, move |v| {
                let mut next = spec;
                set(&mut next, v);
                put(next)
            })
            .step(step)
            .width(90),
            text(shown).size(13).color(MUTED_COLOR).width(48),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    };
    let line = row![
        text(label).width(48),
        button(text(chosen(&spec).to_string()).size(14).width(Length::Fill)).style(style::secondary).width(120).on_press(Message::OpenSoundPicker(target)),
        button(text("Play")).style(style::secondary).on_press(Message::PreviewSound(spec)),
        space().width(8),
        disclosure("Edit sound", editing, Message::ToggleSoundEdit(target.overlay, target.feedback)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let inset = |rows| container(rows).padding(iced::Padding::ZERO.left(28));
    if !editing {
        return inset(column![line]).into();
    }
    let shape = row![
        knob("Volume", volume_db(spec.volume), VOLUME_FLOOR_DB..=0.0, Unit::Level, |s, v| s.volume = decibels(v)),
        knob("Pitch", spec.pitch * 100.0, 50.0..=200.0, Unit::Percent, |s, v| s.pitch = v / 100.0),
        knob("Length", spec.length * 100.0, 50.0..=200.0, Unit::Percent, |s, v| s.length = v / 100.0),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    let effects = row![
        knob("Bass", spec.bass, -12.0..=12.0, Unit::Decibels, |s, v| s.bass = v),
        knob("Treble", spec.treble, -12.0..=12.0, Unit::Decibels, |s, v| s.treble = v),
        knob("Drive", spec.drive * 100.0, 0.0..=100.0, Unit::Percent, |s, v| s.drive = v / 100.0),
        knob("Echo", spec.echo * 100.0, 0.0..=100.0, Unit::Percent, |s, v| s.echo = v / 100.0),
        knob("Reverb", spec.reverb * 100.0, 0.0..=100.0, Unit::Percent, |s, v| s.reverb = v / 100.0),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    // The sliders start under the kind, past the label.
    let under = |knobs| container(knobs).padding(iced::Padding::ZERO.left(48.0 + 8.0));
    inset(column![line, under(shape), under(effects)].spacing(4)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn a_cue_opens_its_sliders_on_its_own() {
        let mut app = with_game();
        let _ = app.update(Message::ToggleSoundEdit(SoundOverlay::Menu, Feedback::Pick));
        assert!(app.open_cue_edits.contains(&(SoundOverlay::Menu, Feedback::Pick)));
        assert!(!app.open_cue_edits.contains(&(SoundOverlay::Menu, Feedback::Step)));
        assert!(!app.open_cue_edits.contains(&(SoundOverlay::Keyboard, Feedback::Pick)));
        let _ = app.update(Message::ToggleSoundEdit(SoundOverlay::Menu, Feedback::Pick));
        assert!(app.open_cue_edits.is_empty());
    }
}
