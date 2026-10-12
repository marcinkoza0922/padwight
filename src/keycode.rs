//! The key and button codes mappings send: evdev's on Linux, and the same names and numbers
//! elsewhere, so a config or pack means the same keys on every platform.

#[cfg(target_os = "linux")]
pub use evdev::KeyCode;

#[cfg(not(target_os = "linux"))]
pub use portable::KeyCode;

#[cfg(not(target_os = "linux"))]
mod portable {
    /// A key or button, numbered as in Linux's `input-event-codes.h`.
    #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[repr(transparent)]
    pub struct KeyCode(pub u16);

    impl KeyCode {
        pub const fn code(self) -> u16 {
            self.0
        }
    }

    /// A name that isn't a key.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ParseError;

    impl std::fmt::Display for ParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("unknown key name")
        }
    }

    impl std::error::Error for ParseError {}

    impl std::str::FromStr for KeyCode {
        type Err = ParseError;

        fn from_str(s: &str) -> Result<Self, ParseError> {
            Self::NAMES.iter().find(|(name, _)| *name == s).map(|(_, code)| *code).ok_or(ParseError)
        }
    }

    impl std::fmt::Debug for KeyCode {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match Self::NAMES.iter().find(|(_, code)| code == self) {
                Some((name, _)) => f.pad(name),
                None => write!(f, "unknown key: {}", self.0),
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod table;

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
