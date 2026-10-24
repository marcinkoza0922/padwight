//! Keyboard and mouse input as `SendInput` takes it: the input records, and the Windows key for
//! each Linux key code.

use anyhow::{Result, bail};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    SendInput, VIRTUAL_KEY,
};

use crate::{config::MouseButton, keycode::KeyCode};

pub(super) fn send(inputs: &[INPUT]) -> Result<()> {
    // SAFETY: `inputs` is a valid slice of initialized INPUT structures.
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        bail!("SendInput refused the input: {}", std::io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn mouse_input(dx: i32, dy: i32, data: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

pub(super) fn mouse_button_input(b: MouseButton, down: bool) -> INPUT {
    const XBUTTON1: i32 = 1;
    const XBUTTON2: i32 = 2;
    let pick = |on, off| if down { on } else { off };
    let (flags, data) = match b {
        MouseButton::Left => (pick(MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP), 0),
        MouseButton::Right => (pick(MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP), 0),
        MouseButton::Middle => (pick(MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP), 0),
        MouseButton::Back => (pick(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP), XBUTTON1),
        MouseButton::Forward => (pick(MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP), XBUTTON2),
    };
    mouse_input(0, 0, data, flags)
}

/// How a key reaches Windows: by its scancode (with the E0 prefix, if `extended`), or by virtual
/// key for the media and browser keys, which have no scancode every layout agrees on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WinKey {
    Scan { code: u16, extended: bool },
    Virtual(u16),
}

pub(super) fn key_input(code: KeyCode, down: bool) -> Option<INPUT> {
    let up = if down { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP };
    let ki = match win_key(code)? {
        WinKey::Scan { code, extended } => {
            let ext = if extended { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) };
            KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: code, dwFlags: KEYEVENTF_SCANCODE | ext | up, time: 0, dwExtraInfo: 0 }
        }
        WinKey::Virtual(vk) => KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: up, time: 0, dwExtraInfo: 0 },
    };
    Some(INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki } })
}

/// The Windows key for a Linux key code. Linux numbers the main block (Esc to F12) by the PC's
/// set 1 scancodes, so those pass straight through; the rest are looked up.
fn win_key(code: KeyCode) -> Option<WinKey> {
    let scan = |code| Some(WinKey::Scan { code, extended: false });
    let ext = |code| Some(WinKey::Scan { code, extended: true });
    let vk = |vk| Some(WinKey::Virtual(vk));
    match code.code() {
        n @ 1..=88 if n != 84 => scan(n),
        96 => ext(0x1c),  // KEY_KPENTER
        97 => ext(0x1d),  // KEY_RIGHTCTRL
        98 => ext(0x35),  // KEY_KPSLASH
        99 => vk(0x2c),   // KEY_SYSRQ: VK_SNAPSHOT
        100 => ext(0x38), // KEY_RIGHTALT
        102 => ext(0x47), // KEY_HOME
        103 => ext(0x48), // KEY_UP
        104 => ext(0x49), // KEY_PAGEUP
        105 => ext(0x4b), // KEY_LEFT
        106 => ext(0x4d), // KEY_RIGHT
        107 => ext(0x4f), // KEY_END
        108 => ext(0x50), // KEY_DOWN
        109 => ext(0x51), // KEY_PAGEDOWN
        110 => ext(0x52), // KEY_INSERT
        111 => ext(0x53), // KEY_DELETE
        113 => vk(0xad),  // KEY_MUTE: VK_VOLUME_MUTE
        114 => vk(0xae),  // KEY_VOLUMEDOWN
        115 => vk(0xaf),  // KEY_VOLUMEUP
        117 => scan(0x59), // KEY_KPEQUAL
        119 => vk(0x13),  // KEY_PAUSE: VK_PAUSE
        121 => scan(0x7e), // KEY_KPCOMMA
        124 => scan(0x7d), // KEY_YEN
        125 => ext(0x5b), // KEY_LEFTMETA
        126 => ext(0x5c), // KEY_RIGHTMETA
        127 => ext(0x5d), // KEY_COMPOSE: the menu key
        140 => vk(0xb7),  // KEY_CALC: VK_LAUNCH_APP2
        142 => vk(0x5f),  // KEY_SLEEP
        155 => vk(0xb4),  // KEY_MAIL
        158 => vk(0xa6),  // KEY_BACK: VK_BROWSER_BACK
        159 => vk(0xa7),  // KEY_FORWARD
        163 => vk(0xb0),  // KEY_NEXTSONG
        164 => vk(0xb3),  // KEY_PLAYPAUSE
        165 => vk(0xb1),  // KEY_PREVIOUSSONG
        166 => vk(0xb2),  // KEY_STOPCD
        172 => vk(0xac),  // KEY_HOMEPAGE
        173 => vk(0xa8),  // KEY_REFRESH
        n @ 183..=193 => scan(0x64 + (n - 183)), // KEY_F13..KEY_F23
        194 => scan(0x76), // KEY_F24
        207 => vk(0xfa),  // KEY_PLAY
        217 => vk(0xaa),  // KEY_SEARCH: VK_BROWSER_SEARCH
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_block_keys_are_their_scancodes() {
        assert_eq!(win_key(KeyCode::KEY_A), Some(WinKey::Scan { code: 0x1e, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_ESC), Some(WinKey::Scan { code: 0x01, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_F12), Some(WinKey::Scan { code: 0x58, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_UP), Some(WinKey::Scan { code: 0x48, extended: true }));
        assert_eq!(win_key(KeyCode::KEY_RIGHTCTRL), Some(WinKey::Scan { code: 0x1d, extended: true }));
        assert_eq!(win_key(KeyCode::KEY_F13), Some(WinKey::Scan { code: 0x64, extended: false }));
        assert_eq!(win_key(KeyCode::KEY_PLAYPAUSE), Some(WinKey::Virtual(0xb3)));
        assert_eq!(win_key(KeyCode::BTN_SOUTH), None);
    }
}
