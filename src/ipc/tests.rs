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
