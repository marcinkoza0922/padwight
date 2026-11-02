//! What the overlay shows, as JSON a script can ask about: `padwight debug overlay state`, and
//! `expect overlay <path> <value>` in a debug session. Both read the frame a running daemon
//! sends its overlay window (the first line of `WatchOverlay`), so they work against any
//! daemon, not only a `--debug` one, and need no display.
//!
//! The document ([`state`]) is a summary built to be asked about, not the wire frame:
//!
//! ```text
//! showing    ["guide", "info:Clock"]  what is up: the panel's name (a menu's title in lower
//!                                     case with dashes, `keyboard`, `numpad`, `media`,
//!                                     `offer`), then `info:<name>` for each info overlay
//!                                     and `log:<name>` for each input log
//! peek       bool                     the panel is held back so what's under it shows
//! menu       null | { title, kind, items: [{label, button, submenu, tone}], labels: [..],
//!                     selected, index, crumbs: [..], hint, about, picks, prompt }
//! <title>    the same object as `menu`, under the menu's title in lower case with spaces
//!            as dashes (`guide.selected`), unless that is another key's name
//! keyboard   null | { board, key, label, row, col, latched: [..], pressed, closing, caps,
//!                     typed, caret, field }
//! media      null | { player, title, artist, state, position_ms, length_ms, volume, muted }
//! offer      null | { title, detail, answers: [{button, label}], hint }
//! info       [{ name, title, rows: [[cell text]], text: [line] }]  after token expansion
//! logs       [{ name, heading, lines: [text] }]
//! backdrop   null | menu title        a menu being edited, drawn under the panel
//! ```
//!
//! `selected` is the highlighted row's label (null when none), `index` its position. A cell's
//! text writes a button glyph as `[A]`, an icon as `[icon:name]` and a bar as `[bar 0.50]`;
//! formatting tags leave nothing.

use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use crate::{
    info::{InfoView, Segment},
    ipc::{self, Request, UnixStream},
    overlay::{OverlayFrame, OverlayView},
};

/// The frame the daemon at `socket` is showing right now.
fn current_frame(socket: &Path, wait: Duration) -> Result<OverlayFrame> {
    let mut stream = UnixStream::connect(socket).with_context(|| format!("no daemon at {}", socket.display()))?;
    let mut request = serde_json::to_string(&Request::WatchOverlay)?;
    request.push('\n');
    stream.write_all(request.as_bytes())?;
    stream.set_read_timeout(Some(wait))?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).context("the daemon sent no overlay frame")?;
    serde_json::from_str(&line).context("the daemon sent something that isn't a frame")
}

/// [`state`] of what the daemon is showing right now.
pub fn current(wait: Duration) -> Result<Value> {
    Ok(state(&current_frame(&ipc::socket_path(), wait)?))
}

/// `padwight debug overlay state [--wait ms]`: prints the document.
#[expect(clippy::print_stdout, reason = "CLI output")]
pub(super) fn run(args: &[&str]) -> Result<()> {
    let wait = match args {
        [] => Duration::from_secs(3),
        ["--wait", ms] => Duration::from_millis(ms.parse().context("--wait takes milliseconds")?),
        _ => anyhow::bail!("usage: overlay state [--wait <ms>]"),
    };
    println!("{}", serde_json::to_string_pretty(&current(wait)?)?);
    Ok(())
}

fn text(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|s| match s {
            Segment::Text(t) => t.clone(),
            Segment::Glyph { label, .. } => format!("[{label}]"),
            Segment::Dpad(_) => "[dpad]".into(),
            Segment::Icon(i) => format!("[icon:{}]", format!("{i:?}").split(['(', ' ']).next().unwrap_or_default().to_lowercase()),
            Segment::Picture(name) => format!("[icon:{name}]"),
            Segment::Bar(share) => format!("[bar {share:.2}]"),
            Segment::Mark(_) => String::new(),
        })
        .collect()
}

fn info(view: &InfoView) -> Value {
    let rows: Vec<Vec<String>> = view.rows.iter().map(|row| row.iter().map(|cell| text(cell)).collect()).collect();
    let lines: Vec<String> = rows.iter().map(|row| row.join(" ")).collect();
    json!({ "name": view.name, "title": view.title.as_deref().map(text), "rows": rows, "text": lines })
}

fn menu(view: &crate::menu::MenuView) -> Value {
    let items: Vec<Value> = view
        .items
        .iter()
        .map(|i| json!({ "label": i.label, "button": i.button, "submenu": i.submenu, "tone": format!("{:?}", i.tone).to_lowercase() }))
        .collect();
    let labels: Vec<&str> = view.items.iter().map(|i| i.label.as_str()).collect();
    let selected = view.selected.and_then(|i| labels.get(i));
    json!({
        "title": view.title,
        "kind": view.kind.tag().short().to_lowercase(),
        "items": items,
        "labels": labels,
        "selected": selected,
        "index": view.selected,
        "crumbs": view.crumbs,
        "hint": view.hint,
        "about": view.about,
        "picks": view.picks,
        "prompt": view.prompt.as_deref().map(offer),
    })
}

fn offer(view: &crate::offer::OfferView) -> Value {
    let answers: Vec<Value> = view.answers.iter().map(|(button, label)| json!({ "button": button, "label": label })).collect();
    json!({ "title": view.title, "detail": view.detail, "answers": answers, "hint": view.hint })
}

fn keyboard(view: &crate::overlay::KeyboardView) -> Value {
    let key = crate::keyboard::key_at(view.board, view.cursor);
    json!({
        "board": format!("{:?}", view.board).to_lowercase(),
        "key": key.code,
        "label": key.label,
        "row": view.cursor.row,
        "col": view.cursor.col,
        "latched": view.latched,
        "pressed": view.pressed,
        "closing": view.closing,
        "caps": view.caps,
        "typed": view.typed,
        "caret": view.caret,
        "field": view.field,
    })
}

fn media(view: &crate::media::MediaView) -> Value {
    json!({
        "player": view.player,
        "title": view.title,
        "artist": view.artist,
        "state": format!("{:?}", view.state).to_lowercase(),
        "position_ms": view.position_ms,
        "length_ms": view.length_ms,
        "volume": view.volume,
        "muted": view.muted,
    })
}

/// The document for `frame` (see the module's docs for its shape).
pub fn state(frame: &OverlayFrame) -> Value {
    let mut doc = Map::new();
    let mut showing: Vec<String> = Vec::new();
    let (mut menu_v, mut keyboard_v, mut media_v, mut offer_v) = (Value::Null, Value::Null, Value::Null, Value::Null);
    let mut alias = None;
    match &frame.active {
        Some(OverlayView::Menu(m)) => {
            let name = m.title.to_lowercase().replace(' ', "-");
            showing.push(name.clone());
            alias = Some(name);
            menu_v = menu(m);
        }
        Some(OverlayView::Keyboard(k)) => {
            showing.push(if k.board == crate::keyboard::Board::Numpad { "numpad" } else { "keyboard" }.into());
            keyboard_v = keyboard(k);
        }
        Some(OverlayView::Media(m)) => {
            showing.push("media".into());
            media_v = media(m);
        }
        Some(OverlayView::Offer(o)) => {
            showing.push("offer".into());
            offer_v = offer(o);
        }
        None => {}
    }
    showing.extend(frame.info.iter().map(|v| format!("info:{}", v.name)));
    showing.extend(frame.logs.iter().map(|v| format!("log:{}", v.name)));
    doc.insert("showing".into(), json!(showing));
    doc.insert("peek".into(), json!(frame.peek));
    doc.insert("menu".into(), menu_v.clone());
    doc.insert("keyboard".into(), keyboard_v);
    doc.insert("media".into(), media_v);
    doc.insert("offer".into(), offer_v);
    doc.insert("info".into(), Value::Array(frame.info.iter().map(info).collect()));
    let logs: Vec<Value> = frame
        .logs
        .iter()
        .map(|v| json!({ "name": v.name, "heading": v.heading, "lines": v.lines.iter().map(|l| format!("{l:?}")).collect::<Vec<_>>() }))
        .collect();
    doc.insert("logs".into(), Value::Array(logs));
    doc.insert("backdrop".into(), json!(frame.backdrop.as_ref().map(|m| &m.title)));
    if let Some(name) = alias.filter(|name| !doc.contains_key(name)) {
        doc.insert(name, menu_v);
    }
    Value::Object(doc)
}

/// Follows `path` (keys and array positions joined by dots) into `doc`. The error names the keys
/// that were there when the path leaves the document.
fn lookup<'a>(doc: &'a Value, path: &str) -> Result<&'a Value, String> {
    let mut at = doc;
    for (i, key) in path.split('.').enumerate() {
        let next = match at {
            Value::Object(map) => map.get(key),
            Value::Array(list) => key.parse::<usize>().ok().and_then(|n| list.get(n)),
            _ => None,
        };
        at = next.ok_or_else(|| {
            let walked = path.split('.').take(i).collect::<Vec<_>>().join(".");
            let here = if walked.is_empty() { "the overlay".to_string() } else { walked };
            match at {
                Value::Object(map) => format!("{here} has no {key:?}; it has {}", map.keys().map(String::as_str).collect::<Vec<_>>().join(", ")),
                Value::Array(list) => format!("{here} has no item {key:?}; it has {} items", list.len()),
                _ => format!("{here} has no {key:?}"),
            }
        })?;
    }
    Ok(at)
}

/// A value as a script writes it: strings bare, `none` for null.
fn word(v: &Value) -> String {
    match v {
        Value::Null => "none".into(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Fails with a message unless the value at `path` is `want`. Strings match exactly (a script's
/// double quotes around `want` are dropped); numbers match within 0.01; `none` matches null and
/// an empty list; a list matches when one of its items does.
pub fn check(doc: &Value, path: &str, want: &str) -> Result<(), String> {
    let want = want.trim();
    let want = want.strip_prefix('"').and_then(|w| w.strip_suffix('"')).unwrap_or(want);
    let got = lookup(doc, path)?;
    let same = |v: &Value| match v {
        Value::Number(n) => n.as_f64().zip(want.parse::<f64>().ok()).is_some_and(|(a, b)| (a - b).abs() <= 0.01),
        other => word(other) == want,
    };
    let ok = match got {
        Value::Array(list) => (want == "none" && list.is_empty()) || list.iter().any(same),
        other => same(other),
    };
    if ok {
        return Ok(());
    }
    let shown = match got {
        Value::Array(list) => Value::Array(list.clone()).to_string(),
        other => format!("{:?}", word(other)),
    };
    Err(format!("overlay {path} is {shown}, not {want:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::scenes;

    fn doc(scene: &str) -> Value {
        state(&scenes::frame(scene).unwrap())
    }

    #[test]
    fn a_menu_names_its_rows_and_selection() {
        let name = scenes::SCENES.iter().map(|(n, _)| *n).find(|n| doc(n)["menu"]["index"].is_u64()).expect("a scene with a selected menu row");
        let d = doc(name);
        let title = d["menu"]["title"].as_str().unwrap().to_lowercase().replace(' ', "-");
        check(&d, "showing", &title).unwrap();
        let i = d["menu"]["index"].as_u64().unwrap() as usize;
        let label = d["menu"]["labels"][i].as_str().unwrap().to_string();
        check(&d, "menu.selected", &label).unwrap();
        check(&d, "menu.selected", &format!("\"{label}\"")).unwrap();
        assert!(check(&d, "menu.selected", "no such label").unwrap_err().contains("not \"no such label\""));
    }

    #[test]
    fn nothing_showing_is_none() {
        let d = state(&OverlayFrame::default());
        check(&d, "showing", "none").unwrap();
        check(&d, "menu", "none").unwrap();
        check(&d, "info", "none").unwrap();
        assert!(check(&d, "showing", "guide").is_err());
    }

    #[test]
    fn a_bad_path_lists_what_is_there() {
        let d = state(&OverlayFrame::default());
        let why = check(&d, "colour", "x").unwrap_err();
        assert!(why.contains("showing"), "{why}");
    }

    #[test]
    fn numbers_match_within_a_hundredth_and_lists_by_item() {
        let d = json!({ "a": { "n": 0.5, "list": ["x", "y"], "b": true } });
        check(&d, "a.n", "0.501").unwrap();
        check(&d, "a.list", "y").unwrap();
        check(&d, "a.list.1", "y").unwrap();
        check(&d, "a.b", "true").unwrap();
        assert!(check(&d, "a.n", "0.6").is_err());
    }

    #[test]
    fn cells_are_text_after_expansion() {
        let cell = [Segment::Text("Hold ".into()), Segment::Glyph { label: "A".into(), fill: None, round: true }, Segment::Mark(crate::info::Mark::Bold(true)), Segment::Bar(0.5)];
        assert_eq!(text(&cell), "Hold [A][bar 0.50]");
    }
}
