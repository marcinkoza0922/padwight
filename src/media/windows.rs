//! The media controls' worker on Windows: reads and drives the players that report to the
//! system's media controls (Spotify, browsers, the Media Player app), as the volume flyout does.

use std::sync::mpsc::{Receiver, RecvTimeoutError};

use anyhow::{Context, Result};
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    },
    Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
};

use super::{Command, MediaState, POLL, PlayState, mixer, next_player};
use crate::monitor::log;

/// Windows counts media time in ticks of 100 ns.
const TICKS_PER_US: i64 = 10;

/// Reads and drives the players until `commands` is dropped. `publish` gets each fresh state
/// and says whether anyone is still listening.
pub fn worker(commands: &Receiver<Command>, publish: impl Fn(MediaState) -> bool) {
    // SAFETY: initializes COM for this thread, which the media and volume APIs run on.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let manager = match Manager::RequestAsync().and_then(|op| op.join()) {
        Ok(manager) => manager,
        Err(e) => {
            log!("media: cannot reach the system's media controls: {e}");
            return;
        }
    };
    let mut chosen: Option<String> = None;
    loop {
        let sessions = sessions(&manager);
        match commands.recv_timeout(POLL) {
            Ok(command) => {
                let names: Vec<String> = sessions.iter().map(|(name, _)| name.clone()).collect();
                if command == Command::CyclePlayer {
                    chosen = next_player(&names, chosen.as_deref());
                } else if command == Command::Mute {
                    mixer::toggle_mute();
                } else if matches!(command, Command::Volume(delta) if mixer::change(delta)) {
                    // The system's volume changed; players have none of their own here.
                } else if let Some((name, session)) = pick(&sessions, chosen.as_deref())
                    && let Err(e) = send(session, command)
                {
                    log!("media: {name}: {e:#}");
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let sessions = self::sessions(&manager);
        let state = MediaState { system: mixer::read(), ..read_all(&sessions, &mut chosen) };
        if !publish(state) {
            return;
        }
    }
}

/// The players, by app ID, in a steady order.
fn sessions(manager: &Manager) -> Vec<(String, Session)> {
    let Ok(list) = manager.GetSessions() else { return Vec::new() };
    let mut sessions: Vec<(String, Session)> = (0..list.Size().unwrap_or(0))
        .filter_map(|i| list.GetAt(i).ok())
        .filter_map(|s| Some((s.SourceAppUserModelId().ok()?.to_string(), s)))
        .collect();
    sessions.sort_by(|a, b| a.0.cmp(&b.0));
    sessions
}

/// The chosen player, or the first.
fn pick<'a>(sessions: &'a [(String, Session)], chosen: Option<&str>) -> Option<&'a (String, Session)> {
    chosen.and_then(|c| sessions.iter().find(|(name, _)| name == c)).or(sessions.first())
}

fn send(session: &Session, command: Command) -> Result<()> {
    let done = match command {
        Command::PlayPause => session.TryTogglePlayPauseAsync()?.join()?,
        Command::Next => session.TrySkipNextAsync()?.join()?,
        Command::Previous => session.TrySkipPreviousAsync()?.join()?,
        Command::Seek(us) => {
            let timeline = session.GetTimelineProperties()?;
            let to = (timeline.Position()?.Duration + us * TICKS_PER_US).max(timeline.StartTime()?.Duration);
            session.TryChangePlaybackPositionAsync(to)?.join()?
        }
        Command::Volume(_) => anyhow::bail!("this player has no volume of its own"),
        Command::CyclePlayer | Command::Mute => true,
    };
    done.then_some(()).context("the player didn't do it")
}

fn status(session: &Session) -> PlayState {
    match session.GetPlaybackInfo().and_then(|info| info.PlaybackStatus()) {
        Ok(Status::Playing) => PlayState::Playing,
        Ok(Status::Paused) => PlayState::Paused,
        _ => PlayState::Stopped,
    }
}

/// A player's name from its app ID: `Spotify.exe` is Spotify, and a Store app's
/// `Publisher.App_hash!App` its app's name.
fn display_name(app_id: &str) -> String {
    let name = app_id.rsplit('!').next().unwrap_or(app_id);
    let name = name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")).unwrap_or(name);
    name.rsplit('.').next().filter(|n| !n.is_empty()).unwrap_or(name).to_string()
}

/// Picks the player to show (keeping `chosen` unless another one is the only one playing) and
/// reads it.
fn read_all(sessions: &[(String, Session)], chosen: &mut Option<String>) -> MediaState {
    let playing = sessions.iter().find(|(_, s)| status(s) == PlayState::Playing).map(|(name, _)| name);
    let keep = chosen.as_ref().filter(|c| sessions.iter().any(|(name, _)| name == *c));
    let keep_is_playing = keep.is_some_and(|c| sessions.iter().any(|(name, s)| name == c && status(s) == PlayState::Playing));
    *chosen = if keep_is_playing {
        keep.cloned()
    } else {
        playing.cloned().or_else(|| keep.cloned()).or_else(|| sessions.first().map(|(name, _)| name.clone()))
    };
    let Some((name, session)) = pick(sessions, chosen.as_deref()) else {
        return MediaState { players: sessions.len(), ..MediaState::default() };
    };
    let properties = session.TryGetMediaPropertiesAsync().and_then(|op| op.join()).ok();
    let title = properties.as_ref().and_then(|p| p.Title().ok()).map(|s| s.to_string()).unwrap_or_default();
    let artist = properties.as_ref().and_then(|p| p.Artist().ok()).map(|s| s.to_string()).unwrap_or_default();
    let timeline = session.GetTimelineProperties().ok();
    let ticks = |span: windows::core::Result<windows::Foundation::TimeSpan>| span.map_or(0, |s| s.Duration);
    let (start, position, end) = match &timeline {
        Some(t) => (ticks(t.StartTime()), ticks(t.Position()), ticks(t.EndTime())),
        None => (0, 0, 0),
    };
    MediaState {
        player: Some(display_name(name)),
        title,
        artist,
        state: status(session),
        position_us: (position - start) / TICKS_PER_US,
        length_us: (end - start) / TICKS_PER_US,
        volume: None,
        players: sessions.len(),
        system: None,
    }
}

#[cfg(test)]
mod tests {
    use super::display_name;

    #[test]
    fn players_are_named_after_their_app() {
        assert_eq!(display_name("Spotify.exe"), "Spotify");
        assert_eq!(display_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "ZuneMusic");
        assert_eq!(display_name("MSEdge"), "MSEdge");
    }
}
