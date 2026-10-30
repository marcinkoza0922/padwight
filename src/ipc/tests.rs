use std::io::Read;

use super::*;

#[test]
fn a_daemon_that_hangs_up_without_replying_is_reported_as_such() {
    let dir = std::env::temp_dir().join(format!("padwight-ipc-hangup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("padwight.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut conn, _) = listener.accept().unwrap();
        let mut request = [0u8; 64];
        let _ = conn.read(&mut request);
    });
    let error = request_at(&path, &Request::Status).unwrap_err();
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(format!("{error:#}").contains("without replying"), "{error:#}");
}

#[test]
fn names_who_reads_a_controller_past_padwight() {
    let readers = |programs: &[&str]| RawReaders { controller: "Pro Controller".into(), programs: programs.iter().map(ToString::to_string).collect() };
    assert_eq!(readers(&["steam"]).warning(), "Steam also reads Pro Controller directly, so it gets your presses as well as Padwight's virtual pad.");
    assert!(readers(&["brave", "steam"]).warning().starts_with("Brave, steam also read Pro Controller directly, so they get"));
}

/// The wire format is a public contract (`docs/protocol.md`): these fail when a rename or a
/// reshaped variant would break a client written in another language.
#[test]
fn requests_keep_their_documented_wire_format() {
    let wire = |req: Request| serde_json::to_string(&req).unwrap();
    assert_eq!(wire(Request::Version), r#""Version""#);
    assert_eq!(wire(Request::Status), r#""Status""#);
    assert_eq!(wire(Request::SetEnabled(true)), r#"{"SetEnabled":true}"#);
    assert_eq!(wire(Request::SetProfile("Doom".into())), r#"{"SetProfile":"Doom"}"#);
    assert_eq!(wire(Request::WatchFeed(None)), r#"{"WatchFeed":null}"#);
    assert_eq!(wire(Request::SetNintendoLayout { id: "a".into(), on: true }), r#"{"SetNintendoLayout":{"id":"a","on":true}}"#);
    assert_eq!(wire(Request::Activate(ProfileRef::new(Some("Doom"), "Default"))), r#"{"Activate":{"setup":"Doom","profile":"Default"}}"#);
    assert_eq!(wire(Request::Activate(ProfileRef::new(None, "Default"))), r#"{"Activate":{"profile":"Default"}}"#);
}

#[test]
fn responses_keep_their_documented_wire_format() {
    let wire = |res: Response| serde_json::to_string(&res).unwrap();
    assert_eq!(wire(Response::Ok), r#""Ok""#);
    assert_eq!(wire(Response::Error("no".into())), r#"{"Error":"no"}"#);
    assert_eq!(
        wire(Response::Version(VersionInfo { protocol_major: 1, protocol_minor: 0, app: "0.1.0".into() })),
        r#"{"Version":{"protocol_major":1,"protocol_minor":0,"app":"0.1.0"}}"#
    );
    assert_eq!(VersionInfo::current().protocol_major, 1, "a new major version is a deliberate act: update docs/protocol.md");
}

#[test]
fn a_status_keeps_its_documented_fields_and_ignores_later_ones() {
    let status = serde_json::to_value(Status::default()).unwrap();
    for field in [
        "enabled", "active_profile", "active_game", "active_layers", "devices", "controllers", "focus_backend", "focused",
        "recent_windows", "overlay_visible", "numpad_visible", "stop_pending", "profile_locked", "switches",
    ] {
        assert!(status.get(field).is_some(), "Status lost `{field}`");
    }
    let mut later = status;
    later["a_field_from_protocol_1_9"] = true.into();
    serde_json::from_value::<Status>(later).expect("a client ignores fields it doesn't know");
}

#[test]
fn an_input_snapshot_keeps_its_documented_fields() {
    let json = r#"{"device":"Pad","buttons":["South","LeftBumper"],"left_stick":[0.5,-1.0],"right_stick":[0.0,0.0],"left_trigger":0.25,"right_trigger":0.0}"#;
    let snapshot: InputSnapshot = serde_json::from_str(json).unwrap();
    assert_eq!(snapshot.buttons, [Button::South, Button::LeftBumper]);
    assert_eq!(snapshot.left_stick, (0.5, -1.0));
}
