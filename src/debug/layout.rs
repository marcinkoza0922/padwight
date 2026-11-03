//! `padwight debug layout`: what the on-screen keyboard, typed text and the calculator's Send make
//! of the keyboard layout, from the code the daemon runs. `PADWIGHT_KEYBOARD_LAYOUT=de` picks a
//! layout; without it the desktop's is detected. For distro-matrix runs on machines with no desktop.

use anyhow::Result;

use crate::keyboard;

pub const USAGE: &str = "  layout [text]                  the key (and whether Shift) that types each character of text in
                                 the keyboard layout, and what some keys of the letter row show
";

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    for key in ["KEY_Q", "KEY_W", "KEY_Y", "KEY_A", "KEY_Z", "KEY_1", "KEY_SEMICOLON"] {
        let show = |shift| keyboard::typed(key, shift, false).unwrap_or_else(|| "-".into());
        println!("label {key} {} {}", show(false), show(true));
    }
    for c in args.join(" ").chars() {
        match keyboard::key_for(c) {
            Some((key, shift)) => println!("type {c} {key} {}", if shift { "shift" } else { "plain" }),
            None => println!("type {c} none"),
        }
    }
    Ok(())
}
