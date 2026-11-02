//! What a `--debug` daemon did, in order and with the time it did it: the transcript that
//! `padwight debug transcript` prints and scenario tests compare. Everything the daemon outputs
//! for an injected controller is in it, with the things around the output that a person would
//! watch: profile, mode and layer changes, overlays coming and going, menu selections, toasts,
//! sound and rumble cues. On a `--fake-clock` daemon the times are exact.

use std::{
    sync::{
        Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::overlay::{OverlayFrame, OverlayView};

/// Lines kept; the oldest go first.
const MAX_LINES: usize = 20_000;

static ON: AtomicBool = AtomicBool::new(false);
static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static START: Mutex<Option<Instant>> = Mutex::new(None);
/// The last overlay summary written, so only changes are.
static SHOWN: Mutex<String> = Mutex::new(String::new());

fn lock<T>(m: &'static Mutex<T>) -> MutexGuard<'static, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Starts keeping the transcript, with time zero now. A daemon without `--debug` never calls this,
/// so for it every [`note`] is a single flag check.
pub(super) fn start() {
    *lock(&START) = Some(crate::clock::now());
    *lock(&SHOWN) = "none".into();
    ON.store(true, Ordering::Relaxed);
}

/// Whether a transcript is being kept.
pub(crate) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Adds a line, stamped with the milliseconds since the transcript started.
pub(crate) fn note(text: &str) {
    if !on() {
        return;
    }
    let ms = lock(&START).map_or(0, |t0| crate::clock::since(t0).as_millis());
    let mut lines = lock(&LINES);
    lines.push(format!("{text} @{ms}"));
    if lines.len() > MAX_LINES {
        let extra = lines.len() - MAX_LINES;
        lines.drain(..extra);
    }
}

/// The last value written for each pad axis, so a resync that sends them all again isn't news.
static AXES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

/// Adds one line of an injected controller's output. A pad axis that is sent a value it already
/// has (every axis at rest, after a layer change resynced the pad) is left out.
pub(super) fn output(line: &str) {
    if !on() {
        return;
    }
    if let Some(rest) = line.strip_prefix("pad-axis ")
        && let Some((axis, value)) = rest.split_once(' ')
    {
        let mut axes = lock(&AXES);
        match axes.iter_mut().find(|(a, _)| a == axis) {
            Some((_, last)) if last == value => return,
            Some((_, last)) => *last = value.to_string(),
            None => {
                axes.push((axis.to_string(), value.to_string()));
                if value == "0.00" {
                    return;
                }
            }
        }
    }
    note(line);
}

/// Writes the mode and layers now on, as `shifts mode Alt layers Hotkeys, Extra` (`-` for none).
pub(super) fn shifts(mode: Option<&str>, layers: &[String]) {
    let layers = if layers.is_empty() { "-".to_string() } else { layers.join(", ") };
    note(&format!("shifts mode {} layers {layers}", mode.unwrap_or("-")));
}

/// The transcript so far; with `clear`, forgets it too.
pub(super) fn take(clear: bool) -> Vec<String> {
    let mut lines = lock(&LINES);
    if clear { std::mem::take(&mut *lines) } else { lines.clone() }
}

/// Writes what the overlay shows now, if it differs from what was last written.
pub(super) fn overlay(frame: &OverlayFrame) {
    if !on() {
        return;
    }
    let summary = summarize(frame);
    let mut shown = lock(&SHOWN);
    if *shown != summary {
        note(&format!("overlay {summary}"));
        *shown = summary;
    }
}

/// The overlay's state in a line: the panel in front with its selection, then any info overlays.
fn summarize(frame: &OverlayFrame) -> String {
    let mut parts = Vec::new();
    match &frame.active {
        None => parts.push("none".to_string()),
        Some(OverlayView::Menu(m)) => {
            let picked = m.selected.and_then(|i| m.items.get(i)).map(|item| item.label.as_str());
            let menu = format!("menu {:?}", m.title);
            parts.push(match picked {
                Some(label) => format!("{menu} selected {label:?}"),
                None => menu,
            });
        }
        Some(OverlayView::Keyboard(_)) => parts.push("keyboard".into()),
        Some(OverlayView::Media(_)) => parts.push("media".into()),
        Some(OverlayView::Offer(_)) => parts.push("offer".into()),
    }
    if frame.peek {
        parts.push("peek".into());
    }
    if frame.lens.is_some() {
        parts.push("lens".into());
    }
    if !frame.info.is_empty() {
        parts.push(format!("info {}", frame.info.len()));
    }
    parts.join(" + ")
}

/// How many lines there are, for [`forget_since`].
pub(super) fn mark() -> usize {
    lock(&LINES).len()
}

/// Forgets the lines after `mark`: what a controller output while settling in isn't news.
pub(super) fn forget_since(mark: usize) {
    lock(&LINES).truncate(mark);
}
