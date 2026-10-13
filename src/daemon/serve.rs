//! The control socket's server side: one thread per client, passing requests to the event loop
//! and streaming what the watch requests ask for.

use std::{
    io::{BufRead, BufReader, Write},
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

use anyhow::Result;

use super::Msg;
use crate::{
    ipc::{self, Request, Response, UnixListener, UnixStream},
    monitor::log,
    overlay::OverlayFrame,
};

/// Minimum gap between snapshots streamed to one watcher (~60 fps).
const WATCH_INTERVAL: Duration = Duration::from_millis(16);
/// A watcher that takes nothing for this long (a stopped or hung GUI) is let go, so its updates
/// don't pile up in the daemon. The GUI connects again when it can; the overlay is restarted.
const WATCH_STALL: Duration = Duration::from_secs(10);

pub(super) fn ipc_server(listener: &UnixListener, tx: &Sender<Msg>) {
    for conn in listener.incoming() {
        let Ok(conn) = conn else { continue };
        let tx = tx.clone();
        // One thread per client: input watchers hold their connection open.
        thread::spawn(move || {
            if let Err(e) = serve_client(&conn, &tx) {
                log!("ipc: {e:#}");
            }
        });
    }
}

fn serve_client(mut conn: &UnixStream, tx: &Sender<Msg>) -> Result<()> {
    conn.set_read_timeout(Some(Duration::from_secs(2)))?;
    conn.set_write_timeout(Some(WATCH_STALL))?;
    let mut line = String::new();
    BufReader::new(conn).read_line(&mut line)?;
    let response = match serde_json::from_str::<Request>(&line) {
        Ok(Request::WatchInput) => return watch_input(conn, tx),
        Ok(Request::WatchOverlay) => return watch_overlay(conn, tx),
        Ok(Request::WatchFeed) => return watch_feed(conn, tx),
        Ok(req) => {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(Msg::Ipc { req, reply: reply_tx })?;
            // The request stays queued, so it may still be carried out; the client hears why it
            // got no answer rather than nothing.
            reply_rx.recv_timeout(ipc::REPLY_WAIT).unwrap_or_else(|_| {
                Response::Error("the daemon is busy and didn't answer in time; the change may still be made".into())
            })
        }
        Err(e) => Response::Error(format!("bad request: {e}")),
    };
    let mut out = serde_json::to_string(&response)?;
    out.push('\n');
    conn.write_all(out.as_bytes())?;
    Ok(())
}

/// Streams overlay state to the (resident) overlay window until it goes away. An empty frame
/// means nothing is shown; the window idles and keeps listening.
fn watch_overlay(mut conn: &UnixStream, tx: &Sender<Msg>) -> Result<()> {
    let (view_tx, view_rx) = mpsc::channel::<OverlayFrame>();
    tx.send(Msg::WatchOverlay(view_tx))?;
    let mut icons_sent = None;
    while let Ok(first) = view_rx.recv() {
        // Frames that came while the last was written: those a newer one stands in for are
        // skipped, so a burst doesn't pile up, but each opening, closing and pick is sent.
        let mut pending: Vec<OverlayFrame> = std::iter::once(first).chain(view_rx.try_iter()).collect();
        for i in 0..pending.len() {
            if pending.get(i + 1).is_some_and(|newer| newer.stands_in_for(&pending[i])) {
                continue;
            }
            let view = &mut pending[i];
            leave_out_icons_sent(view, &mut icons_sent);
            let mut line = serde_json::to_string(view)?;
            line.push('\n');
            if conn.write_all(line.as_bytes()).is_err() {
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Leaves the pack's icons out of `frame` when they are the ones last sent on this stream
/// (`sent`), which the overlay then keeps.
fn leave_out_icons_sent(frame: &mut OverlayFrame, sent: &mut Option<std::collections::BTreeMap<String, String>>) {
    if frame.pack_icons.is_none() {
        return;
    }
    if frame.pack_icons == *sent {
        frame.pack_icons = None;
    } else {
        sent.clone_from(&frame.pack_icons);
    }
}

/// Streams snapshots to a client until it disconnects. Bursts are coalesced to the latest
/// snapshot, so a fast-polling pad never floods the GUI.
fn watch_input(conn: &UnixStream, tx: &Sender<Msg>) -> Result<()> {
    let (snap_tx, snap_rx) = mpsc::channel();
    tx.send(Msg::Watch(snap_tx))?;
    stream_latest(conn, &snap_rx)
}

/// Streams the latest presses and what they did, like [`watch_input`].
fn watch_feed(conn: &UnixStream, tx: &Sender<Msg>) -> Result<()> {
    let (feed_tx, feed_rx) = mpsc::channel();
    tx.send(Msg::WatchFeed(feed_tx))?;
    stream_latest(conn, &feed_rx)
}

/// Writes each value as a JSON line until the client goes away, stops reading (the write times
/// out), or the daemon drops the sender.
fn stream_latest<T: serde::Serialize>(mut conn: &UnixStream, rx: &mpsc::Receiver<T>) -> Result<()> {
    // Blocks until the next update; ends when the daemon drops this watcher.
    while let Ok(mut snapshot) = rx.recv() {
        while let Ok(newer) = rx.try_recv() {
            snapshot = newer;
        }
        let mut line = serde_json::to_string(&snapshot)?;
        line.push('\n');
        if conn.write_all(line.as_bytes()).is_err() {
            // Client went away; the daemon drops our sender on its next send.
            return Ok(());
        }
        thread::sleep(WATCH_INTERVAL);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    fn reply(conn: &mut UnixStream) -> String {
        let mut text = String::new();
        conn.read_to_string(&mut text).unwrap();
        text
    }

    #[test]
    fn a_request_the_event_loop_doesn_t_answer_gets_an_error_not_silence() {
        let (mut client, server) = UnixStream::pair().unwrap();
        let (tx, rx) = mpsc::channel();
        client.write_all(b"\"Status\"\n").unwrap();
        // The event loop takes the request but never answers it.
        let held = thread::spawn(move || rx.recv().ok());
        serve_client(&server, &tx).unwrap();
        assert_eq!(server.write_timeout().unwrap(), Some(WATCH_STALL), "a client that stops reading can't hold a thread");
        drop(server);
        let text = reply(&mut client);
        assert!(text.contains("didn't answer in time"), "{text:?}");
        drop(held.join());
    }

    #[test]
    fn the_overlay_is_sent_a_pack_s_icons_only_when_they_change() {
        let (client, server) = UnixStream::pair().unwrap();
        let (tx, rx) = mpsc::channel();
        let streaming = thread::spawn(move || watch_overlay(&server, &tx).is_ok());
        let Ok(Msg::WatchOverlay(frames)) = rx.recv() else { panic!("no watcher") };
        let with = |icons: &[(&str, &str)]| OverlayFrame {
            pack_icons: Some(icons.iter().map(|(n, svg)| ((*n).to_string(), (*svg).to_string())).collect()),
            ..OverlayFrame::default()
        };
        let mut lines = BufReader::new(client).lines();
        // One at a time: frames queued together would stand in for each other.
        let mut got = Vec::new();
        for frame in [with(&[("flame", "<svg/>")]), with(&[("flame", "<svg/>")]), with(&[]), with(&[])] {
            frames.send(frame).unwrap();
            let line = lines.next().unwrap().unwrap();
            got.push(serde_json::from_str::<OverlayFrame>(&line).unwrap().pack_icons.map(|icons| icons.len()));
        }
        drop(frames);
        assert!(streaming.join().unwrap());
        assert_eq!(got, [Some(1), None, Some(0), None], "sent the first time and when they change, to none too");
    }

    #[test]
    fn a_watcher_that_stops_reading_is_let_go() {
        let (client, server) = UnixStream::pair().unwrap();
        server.set_write_timeout(Some(Duration::from_millis(200))).unwrap();
        let (tx, rx) = mpsc::channel();
        let streaming = thread::spawn(move || stream_latest(&server, &rx).is_ok());
        // The client never reads; big updates fill the socket's buffer within a few writes.
        let big = "x".repeat(256 * 1024);
        let mut sent = 0;
        while tx.send(big.clone()).is_ok() && sent < 1000 {
            sent += 1;
            thread::sleep(WATCH_INTERVAL);
        }
        assert!(sent < 1000, "the watcher kept taking updates nobody read");
        assert!(streaming.join().unwrap());
        drop(client);
    }
}
