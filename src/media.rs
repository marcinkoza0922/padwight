//! Media playback controls: an overlay at the top of the screen that drives whatever player is
//! running (Spotify, YouTube Music in a browser, a local player) over MPRIS, or Windows' media
//! controls. A worker thread talks to the players so a stuck one can't stall the daemon;
//! `MediaSession` turns controller input into commands for it, and the overlay process draws
//! `MediaView`.

use std::{
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::{Button, OverlayStyle},
    input::InputEvent,
};

#[cfg_attr(windows, path = "media/windows_mixer.rs")]
mod mixer;
#[cfg(target_os = "linux")]
mod mpris;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub use mpris::worker;
#[cfg(windows)]
pub use windows::worker;

/// How often the worker re-reads the player (the progress bar moves with it).
const POLL: Duration = Duration::from_millis(500);
/// The overlay closes by itself after this long without input, so it can't hold the
/// controller forever.
const IDLE_CLOSE: Duration = Duration::from_secs(10);
const REPEAT_DELAY: Duration = crate::nav::REPEAT_DELAY;
const REPEAT_EVERY: Duration = crate::nav::REPEAT_EVERY;
const VOLUME_STEP: f64 = 0.05;
const SEEK_STEP_US: i64 = 10_000_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayState {
    Playing,
    Paused,
    #[default]
    Stopped,
}

/// What the worker last read from the selected player.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaState {
    /// The player's display name; `None` when no player is running.
    pub player: Option<String>,
    pub title: String,
    pub artist: String,
    pub state: PlayState,
    pub position_us: i64,
    pub length_us: i64,
    pub volume: Option<f64>,
    /// How many players are running, to offer switching between them.
    pub players: usize,
    /// The system's sound, when there's a mixer to read it from.
    pub system: Option<mixer::Level>,
}

/// What the overlay draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaView {
    pub player: Option<String>,
    pub title: String,
    pub artist: String,
    pub state: PlayState,
    pub position_ms: i64,
    pub length_ms: i64,
    /// 0..1.
    pub volume: Option<f32>,
    pub players: usize,
    /// The system's volume (0..1) and whether it's muted, when there's a mixer.
    #[serde(default)]
    pub system_volume: Option<f32>,
    #[serde(default)]
    pub muted: bool,
    pub hint: String,
    pub style: OverlayStyle,
}

/// The controls, as the overlay reminds you, a line for each.
const HINT: &str = "{leftright} seek · {updown} volume · {mute} mute\n{previous} {next} track · {play} play / pause · {player} player · {media_close} close";

impl MediaView {
    /// A made-up track, drawn with `style`, for the settings previews.
    pub fn sample(style: OverlayStyle) -> Self {
        MediaView {
            player: Some("Music".into()),
            title: "Track title".into(),
            artist: "Artist".into(),
            state: PlayState::Playing,
            position_ms: 62_000,
            length_ms: 215_000,
            volume: Some(0.7),
            players: 1,
            system_volume: Some(0.45),
            muted: false,
            hint: HINT.into(),
            style,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    PlayPause,
    Next,
    Previous,
    /// Microseconds, negative to go back.
    Seek(i64),
    /// Added to the volume (0..1 scale): the system's, or the player's without a mixer.
    Volume(f64),
    /// Mutes the system's sound, or unmutes it.
    Mute,
    CyclePlayer,
}

/// Turns controller input into player commands while the media overlay is up.
pub struct MediaSession {
    commands: Sender<Command>,
    state: MediaState,
    guide_held: bool,
    /// The button held to open the Guide overlay, and the one of the media shortcut that closes
    /// the panel with it held.
    guide_button: Option<Button>,
    close_button: Option<Button>,
    last_input: Instant,
    /// A held button that repeats, and when it next fires.
    repeat: Option<(Button, Instant)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaOutcome {
    Close,
}

fn repeating(b: Button) -> Option<Command> {
    Some(match b {
        Button::DpadUp => Command::Volume(VOLUME_STEP),
        Button::DpadDown => Command::Volume(-VOLUME_STEP),
        Button::DpadLeft => Command::Seek(-SEEK_STEP_US),
        Button::DpadRight => Command::Seek(SEEK_STEP_US),
        _ => return None,
    })
}

impl MediaSession {
    pub fn new(commands: Sender<Command>, guide_held: bool, now: Instant) -> Self {
        MediaSession {
            commands,
            state: MediaState::default(),
            guide_held,
            guide_button: Some(Button::Guide),
            close_button: Some(Button::LeftBumper),
            last_input: now,
            repeat: None,
        }
    }

    /// Which buttons make the chord that closes it (see the fields), when they aren't Guide + LB.
    pub fn with_chord(mut self, guide_button: Option<Button>, close_button: Option<Button>) -> Self {
        self.guide_button = guide_button;
        self.close_button = close_button;
        self
    }

    pub fn update(&mut self, state: MediaState) {
        self.state = state;
    }

    pub fn view(&self) -> MediaView {
        let s = &self.state;
        MediaView {
            player: s.player.clone(),
            title: s.title.clone(),
            artist: s.artist.clone(),
            state: s.state,
            position_ms: s.position_us / 1000,
            length_ms: s.length_us / 1000,
            volume: s.volume.map(|v| v as f32),
            players: s.players,
            system_volume: s.system.map(|l| l.volume),
            muted: s.system.is_some_and(|l| l.muted),
            hint: HINT.into(),
            style: OverlayStyle::media(),
        }
    }

    pub fn handle(&mut self, ev: InputEvent, now: Instant) -> Option<MediaOutcome> {
        let InputEvent::Button(b, pressed) = ev else { return None };
        if Some(b) == self.guide_button {
            self.guide_held = pressed;
            return None;
        }
        if !pressed {
            if self.repeat.is_some_and(|(held, _)| held == b) {
                self.repeat = None;
            }
            return None;
        }
        self.last_input = now;
        let command = match b {
            // The opener and the media shortcut opened it, so both again put it away.
            b if Some(b) == self.close_button && self.guide_held => return Some(MediaOutcome::Close),
            Button::East => return Some(MediaOutcome::Close),
            Button::South => Command::PlayPause,
            Button::RightBumper => Command::Next,
            Button::LeftBumper => Command::Previous,
            Button::North => Command::CyclePlayer,
            Button::West => Command::Mute,
            other => {
                let command = repeating(other)?;
                self.repeat = Some((other, now + REPEAT_DELAY));
                command
            }
        };
        let _ = self.commands.send(command);
        None
    }

    /// Fires a held button's repeat.
    pub fn tick(&mut self, now: Instant) {
        let Some((b, due)) = self.repeat else { return };
        if due > now {
            return;
        }
        if let Some(command) = repeating(b) {
            let _ = self.commands.send(command);
        }
        self.last_input = now;
        self.repeat = Some((b, now + REPEAT_EVERY));
    }

    pub fn expired(&self, now: Instant) -> bool {
        self.last_input + IDLE_CLOSE <= now
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        let idle = self.last_input + IDLE_CLOSE;
        Some(self.repeat.map_or(idle, |(_, due)| due.min(idle)))
    }
}

fn next_player(names: &[String], current: Option<&str>) -> Option<String> {
    let at = current.and_then(|c| names.iter().position(|n| n == c));
    names.get(at.map_or(0, |i| (i + 1) % names.len().max(1))).cloned()
}

/// `1:05` or `1:02:03`.
pub fn clock(ms: i64) -> String {
    let secs = (ms / 1000).max(0);
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn session(guide: bool) -> (MediaSession, mpsc::Receiver<Command>, Instant) {
        let (tx, rx) = mpsc::channel();
        let now = Instant::now();
        (MediaSession::new(tx, guide, now), rx, now)
    }

    fn press(s: &mut MediaSession, b: Button, now: Instant) -> Option<MediaOutcome> {
        s.handle(InputEvent::Button(b, true), now)
    }

    #[test]
    fn buttons_send_commands() {
        let (mut s, rx, now) = session(false);
        for (b, c) in [
            (Button::South, Command::PlayPause),
            (Button::RightBumper, Command::Next),
            (Button::LeftBumper, Command::Previous),
            (Button::DpadUp, Command::Volume(VOLUME_STEP)),
            (Button::DpadRight, Command::Seek(SEEK_STEP_US)),
            (Button::North, Command::CyclePlayer),
            (Button::West, Command::Mute),
        ] {
            assert_eq!(press(&mut s, b, now), None);
            assert_eq!(rx.try_recv(), Ok(c));
        }
        assert_eq!(press(&mut s, Button::East, now), Some(MediaOutcome::Close));
    }

    #[test]
    fn guide_and_left_bumper_close_but_left_bumper_alone_goes_back_a_track() {
        let (mut s, rx, now) = session(true);
        assert_eq!(press(&mut s, Button::LeftBumper, now), Some(MediaOutcome::Close));
        s.handle(InputEvent::Button(Button::Guide, false), now);
        assert_eq!(press(&mut s, Button::LeftBumper, now), None);
        assert_eq!(rx.try_recv(), Ok(Command::Previous));
    }

    #[test]
    fn held_buttons_repeat_until_let_go() {
        let (mut s, rx, now) = session(false);
        press(&mut s, Button::DpadDown, now);
        assert_eq!(rx.try_recv(), Ok(Command::Volume(-VOLUME_STEP)));
        s.tick(now + Duration::from_millis(100));
        assert!(rx.try_recv().is_err(), "not before the delay");
        s.tick(now + REPEAT_DELAY);
        assert_eq!(rx.try_recv(), Ok(Command::Volume(-VOLUME_STEP)));
        s.handle(InputEvent::Button(Button::DpadDown, false), now);
        s.tick(now + REPEAT_DELAY * 4);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn closes_when_idle() {
        let (mut s, _rx, now) = session(false);
        assert!(!s.expired(now + IDLE_CLOSE - Duration::from_millis(1)));
        assert!(s.expired(now + IDLE_CLOSE));
        press(&mut s, Button::South, now + Duration::from_secs(5));
        assert!(!s.expired(now + IDLE_CLOSE));
    }

    #[test]
    fn cycling_wraps_around_the_players() {
        let names = vec!["a".to_string(), "b".to_string()];
        assert_eq!(next_player(&names, Some("a")), Some("b".into()));
        assert_eq!(next_player(&names, Some("b")), Some("a".into()));
        assert_eq!(next_player(&names, None), Some("a".into()));
        assert_eq!(next_player(&[], None), None);
    }

    #[test]
    fn clock_formats() {
        assert_eq!(clock(65_000), "1:05");
        assert_eq!(clock(3_723_000), "1:02:03");
        assert_eq!(clock(-5), "0:00");
    }
}
