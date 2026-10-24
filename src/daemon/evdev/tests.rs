use super::*;


fn node(name: &str, parent: Option<&str>, uniq: Option<&str>) -> PairedNode {
    PairedNode {
        path: PathBuf::from("/dev/input/event99"),
        name: name.into(),
        parent: parent.map(PathBuf::from),
        uniq: uniq.map(Into::into),
    }
}

#[test]
fn motion_sensors_pair_with_their_own_controller() {
    let hid = Some(PathBuf::from("/sys/devices/x/0005:054C:0CE6.0001"));
    let mac = Some("a0:5a:5c:00:00:01".to_string());
    let pad = "Sony Interactive Entertainment DualSense Wireless Controller";

    // Same HID parent (USB or Bluetooth), or same MAC.
    assert!(node("x", Some("/sys/devices/x/0005:054C:0CE6.0001"), None).matches(pad, &hid, &None));
    assert!(node("x", None, Some("a0:5a:5c:00:00:01")).matches(pad, &None, &mac));
    // Driver naming: "<pad> Motion Sensors" (hid-playstation), "<pad> IMU" (hid-nintendo).
    assert!(node(&format!("{pad} Motion Sensors"), None, None).matches(pad, &None, &None));
    assert!(node("Nintendo Switch Pro Controller IMU", None, None).matches("Nintendo Switch Pro Controller", &None, &None));

    // A second identical controller's sensors don't match by parent or MAC, even though
    // the name would.
    let other = node(&format!("{pad} Motion Sensors"), Some("/sys/devices/x/0005:054C:0CE6.0002"), Some("a0:5a:5c:00:00:02"));
    assert!(!other.matches(pad, &hid, &mac));
    // Empty unique IDs (common over USB) never count as a match.
    assert!(!node("x", None, Some("")).matches(pad, &None, &Some(String::new())));
}

#[test]
fn nodes_without_our_axes_are_not_opened() {
    // Bitmaps as sysfs writes them: a pad and motion sensors (this machine's pad), a touchpad
    // with its multi-touch axes, and a bitmap of two words with the axes in the lower one. Then
    // an RGB controller (ABS_MISC only), a mouse, and a sound card's jack.
    for abs in ["3003f", "3f", "260800000000003", "1 3"] {
        assert!(has_our_axes(abs), "{abs}");
    }
    for abs in ["10000000000", "0", "0\n"] {
        assert!(!has_our_axes(abs), "{abs}");
    }
    assert!(has_our_axes("unreadable"), "looked at when in doubt");
}
