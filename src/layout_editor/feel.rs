//! The game's sounds, rumble and motion in Edit Controls. Each is the app's until the game has
//! its own, which starts from the app's (or, for rumble, from silence).

use crate::{
    config::{Config, RumbleSet, rumble_cues::{OverlayRumble, RumbleCue}},
    motion::{MotionStyle, OverlayKind},
    sound::{Feedback, OverlaySounds, SoundKind, SoundOverlay, SoundSpec},
};

use super::{
    owner::game_mut,
    pages::Page,
    rows::{Number, Row, cycle, set},
};

const FEEDBACKS: [(Feedback, &str); 5] = [(Feedback::Step, "Cursor step"), (Feedback::Pick, "Pick"), (Feedback::Open, "Open"), (Feedback::Close, "Close"), (Feedback::Mode, "Mode switch")];

fn feedback_name(f: Feedback) -> &'static str {
    FEEDBACKS.iter().find(|(x, _)| *x == f).map_or("", |(_, name)| name)
}

/// A switch between the app's and the game's own, which starts as `start` makes it.
fn own_row<T: PartialEq + 'static>(what: &str, own: bool, slot: fn(&mut crate::config::Game) -> &mut Option<T>, start: fn(&Config) -> T) -> Row {
    Row::toggle(&format!("Its own {what}"), own, move |c, on| {
        let started = on.then(|| start(c));
        game_mut(c).is_some_and(|g| set(slot(g), started))
    })
}

/// The game's sounds: its own or the app's, and each overlay's.
pub(super) fn sounds_rows(config: &Config) -> Vec<Row> {
    let own = config.active_game().sounds.is_some();
    let mut rows = vec![own_row("sounds", own, |g| &mut g.sounds, |c| c.sounds)];
    if own {
        rows.extend(SoundOverlay::ALL.into_iter().map(|o| Row::open(o.label(), Page::SoundsOf(o))));
    }
    rows
}

fn sounds_of(config: &Config, o: SoundOverlay) -> Option<OverlaySounds> {
    config.active_game().sounds.as_ref().map(|s| s.get(o))
}

/// Changes overlay `o`'s sounds in the game's own set.
fn edit_sounds(config: &mut Config, o: SoundOverlay, change: impl FnOnce(&mut OverlaySounds) -> bool) -> bool {
    let Some(set) = game_mut(config).and_then(|g| g.sounds.as_mut()) else { return false };
    let mut sounds = set.get(o);
    let changed = change(&mut sounds);
    set.set(o, sounds);
    changed
}

fn cue_mut(sounds: &mut OverlaySounds, f: Feedback) -> &mut SoundSpec {
    match f {
        Feedback::Step => &mut sounds.step,
        Feedback::Pick => &mut sounds.pick,
        Feedback::Open => &mut sounds.open,
        Feedback::Close => &mut sounds.close,
        Feedback::Mode => &mut sounds.mode,
    }
}

/// One overlay's sounds: on or off, and each cue.
pub(super) fn overlay_sounds_rows(config: &Config, o: SoundOverlay) -> Vec<Row> {
    let Some(mut sounds) = sounds_of(config, o) else { return Vec::new() };
    let mut rows = vec![Row::toggle("Sounds on", sounds.enabled, move |c, on| edit_sounds(c, o, |s| set(&mut s.enabled, on)))];
    if sounds.enabled {
        rows.extend(FEEDBACKS.iter().map(|&(f, name)| Row::open(format!("{name}: {}", cue_mut(&mut sounds, f).kind), Page::Cue(o, f))));
    }
    rows
}

const VOLUME: Number = Number::new(0.05, (0.0, 1.0), 2, "");
const FACTOR: Number = Number::new(0.05, (0.5, 2.0), 2, "×");
const TONE: Number = Number::new(1.0, (-12.0, 12.0), 0, " dB");
const AMOUNT: Number = Number::new(0.05, (0.0, 1.0), 2, "");

/// A number of a cue, changed by `field`.
fn cue_num((o, f): (SoundOverlay, Feedback), label: &str, value: f32, n: &Number, field: fn(&mut SoundSpec, f32) -> bool) -> Row {
    n.row(label, value, move |c, v| edit_sounds(c, o, |s| field(cue_mut(s, f), v)))
}

/// One cue: what it sounds like, how loud, high and long it is, and its effects.
pub(super) fn cue_rows(config: &Config, o: SoundOverlay, f: Feedback) -> Vec<Row> {
    let Some(mut sounds) = sounds_of(config, o) else { return Vec::new() };
    let cue = *cue_mut(&mut sounds, f);
    let kind = cue.kind;
    vec![
        Row::step(format!("Sound: {kind}"), move |c, dir| edit_sounds(c, o, |s| set(&mut cue_mut(s, f).kind, cycle(&SoundKind::ALL, kind, dir)))),
        cue_num((o, f), "Volume", cue.volume, &VOLUME, |s, v| set(&mut s.volume, v)),
        cue_num((o, f), "Pitch", cue.pitch, &FACTOR, |s, v| set(&mut s.pitch, v)),
        cue_num((o, f), "Length", cue.length, &FACTOR, |s, v| set(&mut s.length, v)),
        cue_num((o, f), "Bass", cue.bass, &TONE, |s, v| set(&mut s.bass, v)),
        cue_num((o, f), "Treble", cue.treble, &TONE, |s, v| set(&mut s.treble, v)),
        cue_num((o, f), "Drive", cue.drive, &AMOUNT, |s, v| set(&mut s.drive, v)),
        cue_num((o, f), "Echo", cue.echo, &AMOUNT, |s, v| set(&mut s.echo, v)),
        cue_num((o, f), "Reverb", cue.reverb, &AMOUNT, |s, v| set(&mut s.reverb, v)),
    ]
}

pub(super) fn cue_title(o: SoundOverlay, f: Feedback) -> String {
    format!("{} · {} sound", o.label(), feedback_name(f).to_lowercase())
}

/// The game's rumble: its own or none, and each overlay's buzzes.
pub(super) fn rumble_rows(config: &Config) -> Vec<Row> {
    let own = config.active_game().rumble.is_some();
    let mut rows = vec![own_row("rumble", own, |g| &mut g.rumble, |_| RumbleSet::default())];
    if own {
        rows.extend(SoundOverlay::ALL.into_iter().map(|o| Row::open(o.label(), Page::RumbleOf(o))));
    }
    rows
}

fn overlay_rumble(set: &mut RumbleSet, o: SoundOverlay) -> &mut OverlayRumble {
    match o {
        SoundOverlay::Keyboard => &mut set.keyboard,
        SoundOverlay::Numpad => &mut set.numpad,
        SoundOverlay::Menu => &mut set.menu,
        SoundOverlay::Guide => &mut set.guide,
        SoundOverlay::Media => &mut set.media,
        SoundOverlay::Offer => &mut set.offer,
    }
}

fn buzz_mut(r: &mut OverlayRumble, f: Feedback) -> &mut Option<RumbleCue> {
    match f {
        Feedback::Step => &mut r.step,
        Feedback::Pick => &mut r.pick,
        Feedback::Open => &mut r.open,
        Feedback::Close => &mut r.close,
        Feedback::Mode => &mut r.mode,
    }
}

fn edit_buzz(config: &mut Config, o: SoundOverlay, f: Feedback, change: impl FnOnce(&mut Option<RumbleCue>) -> bool) -> bool {
    let Some(rumble) = game_mut(config).and_then(|g| g.rumble.as_mut()) else { return false };
    change(buzz_mut(overlay_rumble(rumble, o), f))
}

const STRENGTH: Number = Number::new(0.05, (0.0, 1.0), 2, "");
const BUZZ_LENGTH: Number = Number::new(10.0, (10.0, 500.0), 0, " ms");

/// One overlay's buzzes: each on or off, with its strength and length.
pub(super) fn overlay_rumble_rows(config: &Config, o: SoundOverlay) -> Vec<Row> {
    let Some(mut rumble) = config.active_game().rumble else { return Vec::new() };
    let mut rows = Vec::new();
    for &(f, name) in &FEEDBACKS {
        let buzz = *buzz_mut(overlay_rumble(&mut rumble, o), f);
        rows.push(Row::toggle(name, buzz.is_some(), move |c, on| {
            edit_buzz(c, o, f, |b| set(b, on.then_some(RumbleCue { strength: 0.3, length_ms: 30 })))
        }));
        if let Some(cue) = buzz {
            rows.push(STRENGTH.row("    Strength", cue.strength, move |c, v| edit_buzz(c, o, f, |b| b.as_mut().is_some_and(|b| set(&mut b.strength, v)))));
            rows.push(BUZZ_LENGTH.row_u("    Length", u64::from(cue.length_ms), move |c, v| edit_buzz(c, o, f, |b| b.as_mut().is_some_and(|b| set(&mut b.length_ms, v as u32)))));
        }
    }
    rows
}

/// The game's motion: its own or the app's, and each overlay's style.
pub(super) fn motion_rows(config: &Config) -> Vec<Row> {
    let Some(motion) = config.active_game().motion else {
        return vec![own_row("motion", false, |g| &mut g.motion, |c| c.motion)];
    };
    let mut rows = vec![own_row("motion", true, |g| &mut g.motion, |c| c.motion)];
    rows.extend(OverlayKind::ALL.into_iter().map(|kind| {
        let now = motion.get(kind);
        Row::step(format!("{}: {now}", kind.label()), move |c, dir| {
            let Some(motion) = game_mut(c).and_then(|g| g.motion.as_mut()) else { return false };
            let next = cycle(&MotionStyle::ALL, now, dir);
            motion.set(kind, next);
            next != now
        })
    }));
    rows.push(Row::step("All of them at once", move |c, dir| {
        let Some(m) = game_mut(c).and_then(|g| g.motion.as_mut()) else { return false };
        let next = cycle(&MotionStyle::ALL, m.get(OverlayKind::Menu), dir);
        *m = crate::motion::MotionSet::all(next);
        true
    }));
    rows
}
