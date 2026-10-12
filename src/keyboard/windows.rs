//! What each key types in the user's Windows keyboard layout (the one new programs start with),
//! for the on-screen keyboard's labels.

use std::{collections::HashMap, str::FromStr};

use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, HKL, MAPVK_VSC_TO_VK_EX, MapVirtualKeyExW, ToUnicodeEx, VK_SHIFT};

use super::{Board, labels::Typed};

/// Don't let a dead key change the keyboard's state as we ask about it (Windows 10 1607 and later).
const KEEP_STATE: u32 = 0x4;

/// What every key types in the user's layout.
pub(super) fn labels() -> HashMap<&'static str, Typed> {
    // SAFETY: GetKeyboardLayout has no preconditions.
    let layout = unsafe { GetKeyboardLayout(0) };
    let mut typed = HashMap::new();
    for key in [Board::Basic, Board::Full].into_iter().flat_map(Board::keys) {
        let Ok(code) = crate::keycode::KeyCode::from_str(key.code) else { continue };
        // Linux numbers the main block's keys by their scancodes, the keys that type characters.
        let scan = code.code();
        if !(1..=88).contains(&scan) {
            continue;
        }
        let Some(plain) = types(layout, scan, false) else { continue };
        let shifted = types(layout, scan, true).unwrap_or_else(|| plain.clone());
        typed.insert(key.code, Typed { plain, shifted });
    }
    typed
}

/// The character the key with `scan` types, with Shift or without, as a label. A dead key shows
/// its accent; keys that type nothing visible (space, Enter, Tab) have none.
fn types(layout: HKL, scan: u16, shift: bool) -> Option<String> {
    let mut keys = [0u8; 256];
    if shift {
        keys[usize::from(VK_SHIFT.0)] = 0x80;
    }
    let mut buf = [0u16; 8];
    // SAFETY: the key state is the 256 bytes the call reads, and the buffer is as long as said.
    let written = unsafe {
        let vk = MapVirtualKeyExW(u32::from(scan), MAPVK_VSC_TO_VK_EX, Some(layout));
        if vk == 0 {
            return None;
        }
        ToUnicodeEx(vk, u32::from(scan), &keys, &mut buf, KEEP_STATE, Some(layout))
    };
    // A dead key reports -1 and its accent.
    let len = match written {
        n if n < 0 => 1,
        0 => return None,
        n => n as usize,
    };
    let text = String::from_utf16_lossy(&buf[..len.min(buf.len())]);
    (!text.chars().any(|c| c.is_control() || c.is_whitespace())).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_digits_type_something() {
        let labels = labels();
        // Every layout types something on the A key and the 1 key.
        assert!(labels.get("KEY_A").is_some_and(|t| !t.plain.is_empty()));
        assert!(labels.get("KEY_1").is_some_and(|t| !t.shifted.is_empty()));
        assert!(!labels.contains_key("KEY_SPACE"), "space types nothing to draw");
        assert!(!labels.contains_key("KEY_ENTER"));
    }
}
