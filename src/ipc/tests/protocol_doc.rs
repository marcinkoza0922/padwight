//! `docs/protocol.md` against the daemon's types (T-15): every JSON example in the document parses
//! with the real [`Request`], [`Response`] and friends and serializes back to the same value, and
//! every request, reply and field in the code is described in the document.
//!
//! Examples are found two ways. A fenced block marked `json request`, `json reply` or
//! `json <TypeName>` is one value. An inline code span under "## Requests" (and any starting with
//! `{"Error"`) that is valid JSON must be a request or a reply; spans that are not JSON, such as
//! `{"SetConfig": `[Config](#config)`}`, are skipped.

use std::{collections::BTreeSet, fs, path::Path};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::super::*;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// What an example claims to be.
#[derive(Debug, Clone, PartialEq)]
enum Kind {
    Request,
    Reply,
    Type(String),
    /// An inline span: a request or a reply, whichever reads it.
    Either,
}

struct Example {
    kind: Kind,
    line: usize,
    json: String,
}

/// The fenced blocks marked `json request`, `json reply` or `json <Type>`.
fn fenced(doc: &str) -> Vec<Example> {
    let mut found = Vec::new();
    // The block being read: what it claims to be (None: not ours), where it starts, its text.
    let mut open: Option<(Option<Kind>, usize, String)> = None;
    for (n, line) in doc.lines().enumerate() {
        if let Some((kind, start, mut body)) = open.take() {
            if line.starts_with("```") {
                if let Some(kind) = kind {
                    found.push(Example { kind, line: start, json: body });
                }
            } else {
                body.push_str(line);
                body.push('\n');
                open = Some((kind, start, body));
            }
        } else if let Some(info) = line.strip_prefix("```") {
            let kind = match info.strip_prefix("json").map(str::trim) {
                None | Some("") => None,
                Some("request") => Some(Kind::Request),
                Some("reply") => Some(Kind::Reply),
                Some(name) => Some(Kind::Type(name.to_owned())),
            };
            open = Some((kind, n + 1, String::new()));
        }
    }
    found
}

/// The inline code spans that are JSON, in the "Requests" and "JSON shapes" sections and anywhere an `{"Error"` is shown.
fn inline(doc: &str) -> Vec<Example> {
    let mut found = Vec::new();
    let (mut section, mut in_fence) = (String::new(), false);
    for (n, line) in doc.lines().enumerate() {
        if line.starts_with("```") {
            in_fence = !in_fence;
        }
        if in_fence {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            section = heading.to_owned();
        }
        for (i, span) in line.split('`').enumerate() {
            // Odd pieces are inside backticks.
            if i % 2 == 1 && (matches!(section.as_str(), "Requests" | "JSON shapes") || span.starts_with(r#"{"Error""#))
                && serde_json::from_str::<Value>(span).is_ok_and(|v| v.is_string() || v.is_object())
            {
                found.push(Example { kind: Kind::Either, line: n + 1, json: span.to_owned() });
            }
        }
    }
    found
}

/// Parses `json` as `T` and returns what `T` writes back, for comparing with the original.
fn round_trip<T: DeserializeOwned + Serialize>(json: &str) -> Result<(Value, Value), String> {
    let original: Value = serde_json::from_str(json).map_err(|e| format!("not JSON: {e}"))?;
    let parsed: T = serde_json::from_value(original.clone()).map_err(|e| e.to_string())?;
    Ok((original, serde_json::to_value(parsed).unwrap()))
}

/// The name of the request or reply variant: the string itself, or the one key.
fn tag(value: &Value) -> String {
    match value {
        Value::String(name) => name.clone(),
        Value::Object(map) if map.len() == 1 => map.keys().next().unwrap().clone(),
        other => other.to_string(),
    }
}

/// What an example turned out to be, and its variant's name; or why it is wrong.
fn check(example: &Example) -> Result<(Kind, String), String> {
    let attempt = |kind: &Kind| match kind {
        Kind::Request => round_trip::<Request>(&example.json),
        Kind::Reply => round_trip::<Response>(&example.json),
        Kind::Type(name) => match name.as_str() {
            "ControllerInfo" => round_trip::<ControllerInfo>(&example.json),
            "InputSnapshot" => round_trip::<Option<InputSnapshot>>(&example.json),
            other => Err(format!("the test doesn't know a type called {other}")),
        },
        Kind::Either => unreachable!("an inline span is tried as a request, then as a reply"),
    };
    let kinds = if example.kind == Kind::Either { vec![Kind::Request, Kind::Reply] } else { vec![example.kind.clone()] };
    let mut errors = Vec::new();
    for kind in kinds {
        match attempt(&kind) {
            Ok((doc, code)) if doc == code => return Ok((kind, tag(&doc))),
            Ok((doc, code)) => errors.push(format!("{kind:?}: the daemon writes it back as\n  {code}\nbut the document has\n  {doc}")),
            Err(e) => errors.push(format!("{kind:?}: {e}")),
        }
    }
    Err(errors.join("\n"))
}

fn examples() -> Vec<Example> {
    let doc = read("docs/protocol.md");
    let mut all = fenced(&doc);
    all.extend(inline(&doc));
    all
}

#[test]
fn every_example_in_the_protocol_doc_round_trips_through_the_real_types() {
    let all = examples();
    let failures: Vec<String> = all
        .iter()
        .filter_map(|e| check(e).err().map(|why| format!("docs/protocol.md:{}: {}\n{why}", e.line, e.json.lines().next().unwrap_or(""))))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    assert!(all.iter().any(|e| e.kind == Kind::Reply) && all.len() > 25, "found only {} examples: has the document's layout changed?", all.len());
}

/// The text between a `pub enum Name {` or `pub struct Name {` header in src/ipc.rs and its closing brace.
fn body(source: &str, header: &str) -> String {
    let start = source.find(header).unwrap_or_else(|| panic!("src/ipc.rs has no `{header}`")) + header.len();
    let end = source[start..].find("\n}").unwrap();
    source[start..start + end].to_owned()
}

/// Variant names: the lines at the enum's indent that start with a capital.
fn variants(source: &str, name: &str) -> Vec<String> {
    body(source, &format!("pub enum {name} {{"))
        .lines()
        .filter(|l| l.starts_with("    ") && !l.starts_with("     ") && l.trim_start().starts_with(|c: char| c.is_ascii_uppercase()))
        .map(|l| l.trim_start().chars().take_while(|c| c.is_alphanumeric()).collect())
        .collect()
}

fn fields(source: &str, name: &str) -> Vec<String> {
    body(source, &format!("pub struct {name} {{"))
        .lines()
        .filter_map(|l| l.strip_prefix("    pub "))
        .map(|l| l.split(':').next().unwrap().to_owned())
        .collect()
}

#[test]
fn every_request_and_reply_in_the_code_is_in_the_protocol_doc() {
    let (doc, source) = (read("docs/protocol.md"), read("src/ipc.rs"));
    let shown: Vec<(Kind, String)> = examples().iter().filter_map(|e| check(e).ok()).collect();
    let has_example = |kind: Kind, name: &str| shown.iter().any(|(k, t)| *k == kind && t == name);
    let mut missing = Vec::new();
    for name in variants(&source, "Request") {
        // The debug requests are only named: they are not covered by the version.
        if name.starts_with("Debug") {
            if !doc.contains(&name) {
                missing.push(format!("request {name} is not mentioned"));
            }
        } else if name == "SetConfig" {
            // Its example would be the whole configuration: it is shown as a link to Config.
            if !doc.contains(r#"{"SetConfig":"#) {
                missing.push("request SetConfig is not shown".into());
            }
        } else if !has_example(Kind::Request, &name) {
            missing.push(format!("request {name} has no example"));
        }
    }
    for name in variants(&source, "Response") {
        if name == "Config" {
            if !doc.contains(r#"{"Config":"#) {
                missing.push("reply Config is not shown".into());
            }
        } else if matches!(name.as_str(), "Attached" | "Lines") {
            if !doc.contains(&format!("\"{name}\"")) {
                missing.push(format!("reply {name} is not mentioned"));
            }
        } else if !has_example(Kind::Reply, &name) {
            missing.push(format!("reply {name} has no example"));
        }
    }
    assert!(missing.is_empty(), "docs/protocol.md lacks:\n{}", missing.join("\n"));
}

#[test]
fn every_field_of_a_documented_reply_is_in_the_protocol_doc() {
    let (doc, source) = (read("docs/protocol.md"), read("src/ipc.rs"));
    let mut missing = BTreeSet::new();
    for name in ["VersionInfo", "Status", "UpdateInfo", "SwitchEvent", "WindowInfo", "ControllerInfo", "RawReaders", "DeviceInfo", "InputSnapshot", "GyroOut"] {
        for field in fields(&source, name) {
            if !doc.contains(&format!("`{field}`")) && !doc.contains(&format!("\"{field}\":")) {
                missing.insert(format!("{name}.{field}"));
            }
        }
    }
    for name in variants(&source, "FocusBackend") {
        if !doc.contains(&format!("`{name}`")) {
            missing.insert(format!("FocusBackend::{name}"));
        }
    }
    assert!(missing.is_empty(), "docs/protocol.md doesn't mention: {missing:?}");
}

/// The configuration is too large to print in the document, but the top-level fields it names must exist.
#[test]
fn the_configuration_fields_the_protocol_doc_names_exist() {
    let config = serde_json::to_value(Response::Config(Box::default())).unwrap();
    let config = &config["Config"];
    // `GetConfig` leaves out what has its default value, so only the always-present ones are checked.
    for field in ["general", "setups"] {
        assert!(config.get(field).is_some(), "the configuration has no `{field}`, which docs/protocol.md names");
    }
    let doc = read("docs/protocol.md");
    for field in config.as_object().unwrap().keys().filter(|k| ["enabled", "active", "general", "setups", "ignored_devices", "auto_switch", "controllers"].contains(&k.as_str())) {
        assert!(doc.contains(&format!("`{field}`")), "docs/protocol.md doesn't mention Config.{field}");
    }
}
