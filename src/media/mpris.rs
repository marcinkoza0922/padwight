//! The media controls' worker on Linux: reads and drives MPRIS players over the session bus.

use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, RecvTimeoutError},
};

use anyhow::{Context, Result};
use zbus::{blocking::Connection, zvariant::{OwnedValue, Value}};

use super::{Command, MediaState, POLL, PlayState, mixer, next_player};
use crate::monitor::log;

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const ROOT: &str = "org.mpris.MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";

/// Reads and drives the players until `commands` is dropped. `publish` gets each fresh state
/// and says whether anyone is still listening.
pub fn worker(commands: &Receiver<Command>, publish: impl Fn(MediaState) -> bool) {
    let conn = match Connection::session() {
        Ok(c) => c,
        Err(e) => {
            log!("media: cannot reach the session bus: {e}");
            return;
        }
    };
    let mut chosen: Option<String> = None;
    loop {
        let names = players(&conn).unwrap_or_default();
        match commands.recv_timeout(POLL) {
            Ok(command) => {
                if command == Command::CyclePlayer {
                    chosen = next_player(&names, chosen.as_deref());
                } else if command == Command::Mute {
                    mixer::toggle_mute();
                } else if matches!(command, Command::Volume(delta) if mixer::change(delta)) {
                    // The system's volume changed; the player's stays as it is.
                } else if let Some(name) = chosen.as_deref().or(names.first().map(String::as_str))
                    && let Err(e) = send(&conn, name, command)
                {
                    log!("media: {name}: {e:#}");
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let names = players(&conn).unwrap_or_default();
        let state = MediaState { system: mixer::read(), ..read_all(&conn, &names, &mut chosen) };
        if !publish(state) {
            return;
        }
    }
}

fn players(conn: &Connection) -> Result<Vec<String>> {
    let reply = conn.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "ListNames", &())?;
    let mut names: Vec<String> = reply.body().deserialize::<Vec<String>>()?.into_iter().filter(|n| n.starts_with(PREFIX)).collect();
    names.sort();
    Ok(names)
}

fn all_properties(conn: &Connection, name: &str, iface: &str) -> Result<HashMap<String, OwnedValue>> {
    let reply = conn.call_method(Some(name), PATH, Some(PROPERTIES), "GetAll", &(iface,))?;
    Ok(reply.body().deserialize()?)
}

fn send(conn: &Connection, name: &str, command: Command) -> Result<()> {
    let call = |method: &str| conn.call_method(Some(name), PATH, Some(PLAYER), method, &()).map(drop);
    match command {
        Command::PlayPause => call("PlayPause")?,
        Command::Next => call("Next")?,
        Command::Previous => call("Previous")?,
        Command::Seek(us) => {
            conn.call_method(Some(name), PATH, Some(PLAYER), "Seek", &(us,))?;
        }
        Command::Volume(delta) => {
            let props = all_properties(conn, name, PLAYER)?;
            let current = props.get("Volume").and_then(|v| v.downcast_ref::<f64>().ok()).context("this player has no volume")?;
            let volume = (current + delta).clamp(0.0, 1.0);
            conn.call_method(Some(name), PATH, Some(PROPERTIES), "Set", &(PLAYER, "Volume", Value::from(volume)))?;
        }
        Command::CyclePlayer | Command::Mute => {}
    }
    Ok(())
}

/// Picks the player to show (keeping `chosen` unless another one is the only one playing) and
/// reads it.
fn read_all(conn: &Connection, names: &[String], chosen: &mut Option<String>) -> MediaState {
    let mut playing = None;
    let mut read: HashMap<&str, HashMap<String, OwnedValue>> = HashMap::new();
    for name in names {
        if let Ok(props) = all_properties(conn, name, PLAYER) {
            if playing.is_none() && status(&props) == PlayState::Playing {
                playing = Some(name);
            }
            read.insert(name, props);
        }
    }
    let keep = chosen.as_ref().filter(|c| read.contains_key(c.as_str()));
    let keep_is_playing = keep.is_some_and(|c| status(&read[c.as_str()]) == PlayState::Playing);
    *chosen = if keep_is_playing { keep.cloned() } else { playing.cloned().or_else(|| keep.cloned()).or_else(|| names.first().cloned()) };
    let Some(name) = chosen.as_deref().filter(|c| read.contains_key(c)) else {
        return MediaState { players: names.len(), ..MediaState::default() };
    };
    let props = &read[name];
    let metadata = props.get("Metadata").and_then(|m| HashMap::<String, OwnedValue>::try_from(m.try_clone().ok()?).ok()).unwrap_or_default();
    let text = |key: &str| metadata.get(key).and_then(|v| v.downcast_ref::<String>().ok()).unwrap_or_default();
    let artist = metadata
        .get("xesam:artist")
        .and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok())
        .map(|a| a.join(", "))
        .unwrap_or_default();
    let identity = all_properties(conn, name, ROOT).ok().and_then(|r| r.get("Identity").and_then(|v| v.downcast_ref::<String>().ok()));
    MediaState {
        player: Some(identity.unwrap_or_else(|| name.trim_start_matches(PREFIX).to_string())),
        title: text("xesam:title"),
        artist,
        state: status(props),
        position_us: props.get("Position").and_then(integer).unwrap_or(0),
        length_us: metadata.get("mpris:length").and_then(integer).unwrap_or(0),
        volume: props.get("Volume").and_then(|v| v.downcast_ref::<f64>().ok()),
        players: names.len(),
        system: None,
    }
}

fn status(props: &HashMap<String, OwnedValue>) -> PlayState {
    match props.get("PlaybackStatus").and_then(|v| v.downcast_ref::<String>().ok()).as_deref() {
        Some("Playing") => PlayState::Playing,
        Some("Paused") => PlayState::Paused,
        _ => PlayState::Stopped,
    }
}

/// Players disagree on the integer type of positions and lengths.
fn integer(v: &OwnedValue) -> Option<i64> {
    v.downcast_ref::<i64>()
        .ok()
        .or_else(|| v.downcast_ref::<u64>().ok().and_then(|n| i64::try_from(n).ok()))
        .or_else(|| v.downcast_ref::<i32>().ok().map(i64::from))
        .or_else(|| v.downcast_ref::<u32>().ok().map(i64::from))
}

