//! Hides Padwight's virtual controller from Steam. Steam reports every controller it sees to the
//! games it runs, so a game with Steam Input turned off (Slay the Spire 2) switches to reading
//! Steam's controller, which then sends no stick. Steam started with
//! `SDL_GAMECONTROLLER_IGNORE_DEVICES` leaves the listed controllers alone, and it hands its
//! games their own value of that variable, so they still see Padwight's controller.
//!
//! Steam reads the variable when it starts, so it's put where Steam is started from: a copy of
//! its menu entry in the user's applications folder, or a Flatpak override.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use anyhow::{Context, Result, bail};

use crate::pad_identity::PadIdentity;

const VAR: &str = "SDL_GAMECONTROLLER_IGNORE_DEVICES";
const FLATPAK_APP: &str = "com.valvesoftware.Steam";
/// The first line of a menu entry Padwight wrote, so it only ever removes its own.
const MARKER: &str = "# Written by Padwight to hide its virtual controller from Steam.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Steam starts with Padwight's controller hidden (once it's restarted).
    Hidden,
    Shown,
    /// Steam's menu entry can't be found, or the user has an entry of their own in the way.
    Unavailable(String),
}

/// The USB IDs Padwight's controller can present, as the variable lists them.
pub(crate) fn ignored_ids() -> String {
    [PadIdentity::Xbox360, PadIdentity::DualShock4, PadIdentity::DualSense, PadIdentity::SwitchPro]
        .map(|i| {
            let (vendor, product, _) = i.usb_id();
            format!("0x{vendor:04x}/0x{product:04x}")
        })
        .join(",")
}

/// Where Steam comes from: the Flatpak, or a menu entry installed with the system.
enum Install {
    Flatpak,
    Native { entry: PathBuf, user_entry: PathBuf },
}

fn install() -> Result<Install> {
    if std::env::var_os("FLATPAK_ID").is_some() {
        bail!("Padwight's Flatpak can't change how Steam starts. See Troubleshooting in the manual for doing it by hand.");
    }
    let data_home = dirs::data_dir().context("no home folder")?;
    let user_entry = data_home.join("applications/steam.desktop");
    let system_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    if let Some(entry) = system_dirs.split(':').map(|d| Path::new(d).join("applications/steam.desktop")).find(|p| p.exists()) {
        return Ok(Install::Native { entry, user_entry });
    }
    if flatpak_installed() {
        return Ok(Install::Flatpak);
    }
    bail!("Steam isn't installed, or its menu entry can't be found.")
}

fn flatpak_installed() -> bool {
    Command::new("flatpak").args(["info", FLATPAK_APP]).output().is_ok_and(|o| o.status.success())
}

fn flatpak_override_file() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("flatpak/overrides").join(FLATPAK_APP))
}

/// The last [`status`] read, so drawing the settings page doesn't look again every frame.
static STATUS: Mutex<Option<Status>> = Mutex::new(None);

/// Whether Steam starts with Padwight's controller hidden; read once, and again after [`set`].
pub fn status() -> Status {
    let mut cached = STATUS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    cached.get_or_insert_with(read_status).clone()
}

fn read_status() -> Status {
    match install() {
        Err(e) => Status::Unavailable(format!("{e:#}")),
        Ok(Install::Flatpak) => {
            let set = flatpak_override_file().and_then(|f| fs::read_to_string(f).ok()).is_some_and(|t| t.contains(&format!("{VAR}=")));
            if set { Status::Hidden } else { Status::Shown }
        }
        Ok(Install::Native { user_entry, .. }) => match fs::read_to_string(&user_entry) {
            Ok(text) if text.starts_with(MARKER) => Status::Hidden,
            Ok(_) => Status::Unavailable(format!("{} is your own copy of Steam's menu entry, so Padwight leaves it alone.", user_entry.display())),
            Err(_) => Status::Shown,
        },
    }
}

/// Hides Padwight's controller from Steam the next time Steam starts, or stops hiding it.
pub fn set(hidden: bool) -> Result<()> {
    let done = apply(hidden);
    *STATUS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    done
}

fn apply(hidden: bool) -> Result<()> {
    match install()? {
        Install::Flatpak => {
            let arg = if hidden { format!("--env={VAR}={}", ignored_ids()) } else { format!("--unset-env={VAR}") };
            let out = Command::new("flatpak").args(["override", "--user", &arg, FLATPAK_APP]).output().context("running flatpak")?;
            if !out.status.success() {
                bail!("flatpak override failed: {}", String::from_utf8_lossy(&out.stderr).trim());
            }
        }
        Install::Native { entry, user_entry } => {
            let current = fs::read_to_string(&user_entry).ok();
            if current.as_ref().is_some_and(|t| !t.starts_with(MARKER)) {
                bail!("{} is your own copy of Steam's menu entry, so Padwight leaves it alone.", user_entry.display());
            }
            if hidden {
                let text = fs::read_to_string(&entry).with_context(|| format!("reading {}", entry.display()))?;
                if let Some(dir) = user_entry.parent() {
                    fs::create_dir_all(dir)?;
                }
                fs::write(&user_entry, hiding_entry(&text, &ignored_ids())).with_context(|| format!("writing {}", user_entry.display()))?;
            } else if current.is_some() {
                fs::remove_file(&user_entry).with_context(|| format!("removing {}", user_entry.display()))?;
            }
        }
    }
    Ok(())
}

/// Steam's menu entry with every command started with the variable set to `ids`.
fn hiding_entry(entry: &str, ids: &str) -> String {
    let mut out = format!("{MARKER}\n");
    for line in entry.lines() {
        match line.strip_prefix("Exec=") {
            Some(cmd) => out.push_str(&format!("Exec=env {VAR}={ids} {cmd}\n")),
            None => out.push_str(&format!("{line}\n")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_steam_command_gets_the_variable() {
        let entry = "[Desktop Entry]\nName=Steam\nExec=/usr/bin/steam %U\n\n[Desktop Action Store]\nExec=/usr/bin/steam steam://store\n";
        let out = hiding_entry(entry, "0x045e/0x028e");
        assert!(out.starts_with(MARKER));
        assert!(out.contains("Exec=env SDL_GAMECONTROLLER_IGNORE_DEVICES=0x045e/0x028e /usr/bin/steam %U\n"));
        assert!(out.contains("Exec=env SDL_GAMECONTROLLER_IGNORE_DEVICES=0x045e/0x028e /usr/bin/steam steam://store\n"));
        assert!(out.contains("Name=Steam\n"));
    }

    #[test]
    fn hides_every_identity_padwight_presents() {
        assert_eq!(ignored_ids(), "0x045e/0x028e,0x054c/0x09cc,0x054c/0x0ce6,0x057e/0x2009");
    }
}
