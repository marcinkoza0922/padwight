//! The system's sound: its volume and mute, through PipeWire's `wpctl`, or PulseAudio's `pactl`
//! where that's what runs. The media controls change it with the D-pad, since many players (a
//! browser's, for one) ignore a volume set over MPRIS.

use std::process::Command;

/// The default output's volume (0 to 1) and whether it's muted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub volume: f32,
    pub muted: bool,
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The default output's level, or `None` with neither tool, or no output.
pub fn read() -> Option<Level> {
    run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]).and_then(|out| parse_wpctl(&out)).or_else(|| {
        let volume = run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])?;
        let mute = run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])?;
        parse_pactl(&volume, &mute)
    })
}

/// Changes the default output's volume by `delta` (a share of full, negative to lower it), up to
/// full. False when there's no tool to do it with.
pub fn change(delta: f64) -> bool {
    let percent = format!("{:.0}%", delta.abs() * 100.0);
    let sign = if delta < 0.0 { "-" } else { "+" };
    run("wpctl", &["set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", &format!("{percent}{sign}")]).is_some()
        || read().is_some_and(|level| match pactl_volume(level.volume, delta) {
            Some(to) => run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &to]).is_some(),
            // Already at full: done, as `wpctl` would be, rather than the player's volume rising.
            None => true,
        })
}

/// The volume to give `pactl` for `delta` from `volume`: set outright, since `pactl`'s own steps
/// go past full. Raising stops at full (or wherever it was set above it); `None` when that
/// leaves nothing to change.
fn pactl_volume(volume: f32, delta: f64) -> Option<String> {
    let volume = f64::from(volume);
    let to = if delta > 0.0 { (volume + delta).min(volume.max(1.0)) } else { (volume + delta).max(0.0) };
    ((to - volume).abs() > 1e-6).then(|| format!("{:.0}%", to * 100.0))
}

/// Mutes the default output, or unmutes it. False when there's no tool to do it with.
pub fn toggle_mute() -> bool {
    run("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]).is_some() || run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"]).is_some()
}

/// `Volume: 0.45` or `Volume: 0.45 [MUTED]`.
fn parse_wpctl(out: &str) -> Option<Level> {
    let rest = out.trim().strip_prefix("Volume:")?.trim();
    let volume = rest.split_whitespace().next()?.parse().ok()?;
    Some(Level { volume, muted: rest.contains("[MUTED]") })
}

/// `Volume: front-left: 29491 /  45% / -20.81 dB, …` and `Mute: no`.
fn parse_pactl(volume: &str, mute: &str) -> Option<Level> {
    let percent = volume.split('/').nth(1)?.trim().trim_end_matches('%').parse::<f32>().ok()?;
    Some(Level { volume: percent / 100.0, muted: mute.trim() == "Mute: yes" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_read_from_either_tool() {
        assert_eq!(parse_wpctl("Volume: 0.45\n"), Some(Level { volume: 0.45, muted: false }));
        assert_eq!(parse_wpctl("Volume: 1.00 [MUTED]\n"), Some(Level { volume: 1.0, muted: true }));
        assert_eq!(parse_wpctl("nothing"), None);
        let pactl = "Volume: front-left: 29491 /  45% / -20.81 dB,   front-right: 29491 /  45% / -20.81 dB\n";
        assert_eq!(parse_pactl(pactl, "Mute: yes\n"), Some(Level { volume: 0.45, muted: true }));
    }

    #[test]
    fn pactl_raises_the_volume_no_further_than_full() {
        assert_eq!(pactl_volume(0.45, 0.05).as_deref(), Some("50%"));
        assert_eq!(pactl_volume(0.98, 0.05).as_deref(), Some("100%"), "not 103%");
        assert_eq!(pactl_volume(1.0, 0.05), None, "nothing to do, and that's done");
        assert_eq!(pactl_volume(1.2, 0.05), None, "a volume set above full is left there");
        assert_eq!(pactl_volume(1.2, -0.05).as_deref(), Some("115%"));
        assert_eq!(pactl_volume(0.02, -0.05).as_deref(), Some("0%"));
        assert_eq!(pactl_volume(0.0, -0.05), None);
    }
}
