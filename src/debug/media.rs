//! `padwight debug media`: the volume and mute the media actions change, through `wpctl` or `pactl`,
//! without a controller or a daemon. For checking a sound server setup (PipeWire, PulseAudio, none).

use anyhow::{Result, bail};

use crate::media::mixer;

pub const USAGE: &str = "  media [volume up|down|mute]    the default output's volume and mute; with an argument, change it
                                 as the D-pad volume actions and the mute action do (exit 1 if no tool could)
";

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    let done = match args {
        [] => true,
        ["volume", "up"] => mixer::change(0.05),
        ["volume", "down"] => mixer::change(-0.05),
        ["mute"] => mixer::toggle_mute(),
        _ => bail!("usage: padwight debug media [volume up|down|mute]"),
    };
    match mixer::read() {
        Some(level) => println!("volume {:.0}% {}", level.volume * 100.0, if level.muted { "muted" } else { "unmuted" }),
        None => println!("no output (neither wpctl nor pactl could read one)"),
    }
    if !done {
        bail!("no tool could change it (wpctl and pactl are missing, or there is no output)");
    }
    Ok(())
}
