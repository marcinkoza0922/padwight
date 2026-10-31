//! The key and button codes mappings send: evdev's.

pub use evdev::KeyCode;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::KeyCode;

    #[test]
    fn names_round_trip() {
        for name in ["KEY_A", "KEY_LEFTCTRL", "KEY_F12", "BTN_SOUTH", "BTN_LEFT", "KEY_VOLUMEUP"] {
            let code = KeyCode::from_str(name).unwrap();
            assert_eq!(format!("{code:?}"), name);
        }
        assert_eq!(KeyCode::KEY_A.code(), 30);
        assert!(KeyCode::from_str("KEY_NOPE").is_err());
    }
}
