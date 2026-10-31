//! Cinnamon (Muffin): a small Cinnamon extension reports every focus change to our D-Bus
//! service, as the GNOME Shell one does. We install it into the user's extensions directory and
//! add it to the `enabled-extensions` setting, which Cinnamon watches.

use std::{fs, path::{Path, PathBuf}, process::Command};

use anyhow::{Context, Result, bail};
use zbus::blocking::Connection;

use super::dbus::call;
use crate::monitor::log;

const UUID: &str = "padwight-focus@io.github.marcinkoza0922";
const SHELL: &str = "org.Cinnamon";
/// Owned by the extension once Cinnamon has loaded it (it serves the overlay).
const LOADED_NAME: &str = "io.github.marcinkoza0922.Padwight.Shell";

/// The extension's files, compiled in so the app can install them anywhere.
const FILES: [(&str, &str); 2] = [
    ("metadata.json", include_str!("../../cinnamon-extension/padwight-focus@io.github.marcinkoza0922/metadata.json")),
    ("extension.js", include_str!("../../cinnamon-extension/padwight-focus@io.github.marcinkoza0922/extension.js")),
];

fn name_has_owner(conn: &Connection, name: &str) -> bool {
    call(conn, ("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus"), "NameHasOwner", &(name,))
        .and_then(|reply| Ok(reply.body().deserialize::<bool>()?))
        .unwrap_or(false)
}

/// Whether Cinnamon is on the session bus.
pub fn is_running(conn: &Connection) -> bool {
    name_has_owner(conn, SHELL)
}

/// Installs the extension if needed and makes sure Cinnamon has it running. Errors when it
/// isn't loaded yet, such as right after a first install, and the caller tries again later.
/// A running extension keeps its old code until the next log in, so an update is only reported.
pub fn ensure_extension(conn: &Connection) -> Result<()> {
    let updated = write_files(&extension_dir()?)?;
    enable()?;
    if name_has_owner(conn, LOADED_NAME) {
        if updated {
            log!("the focus extension was updated: log out and back in to load it");
        }
        Ok(())
    } else {
        bail!("Cinnamon hasn't loaded {UUID} yet")
    }
}

/// Adds the extension to `enabled-extensions`, unless it is already there.
fn enable() -> Result<()> {
    let read = Command::new("gsettings").args(["get", "org.cinnamon", "enabled-extensions"]).output().context("running gsettings")?;
    if !read.status.success() {
        bail!("gsettings can't read enabled-extensions");
    }
    let mut list = parse_list(&String::from_utf8_lossy(&read.stdout));
    if list.iter().any(|uuid| uuid == UUID) {
        return Ok(());
    }
    list.push(UUID.to_string());
    let status = Command::new("gsettings").args(["set", "org.cinnamon", "enabled-extensions", &format_list(&list)]).status().context("running gsettings")?;
    if !status.success() {
        bail!("gsettings can't write enabled-extensions");
    }
    Ok(())
}

/// The strings of a GVariant `as` as `gsettings get` prints it: `['a', 'b']` or `@as []`.
fn parse_list(text: &str) -> Vec<String> {
    text.split('\'').skip(1).step_by(2).map(str::to_string).collect()
}

fn format_list(list: &[String]) -> String {
    format!("[{}]", list.iter().map(|uuid| format!("'{uuid}'")).collect::<Vec<_>>().join(", "))
}

/// Writes the extension's files, leaving alone any that already match. Returns whether any were
/// written.
fn write_files(dir: &Path) -> Result<bool> {
    fs::create_dir_all(dir)?;
    let mut written = false;
    for (name, content) in FILES {
        let path = dir.join(name);
        if fs::read_to_string(&path).ok().as_deref() != Some(content) {
            fs::write(&path, content)?;
            written = true;
        }
    }
    Ok(written)
}

fn extension_dir() -> Result<PathBuf> {
    Ok(dirs::data_dir().context("no data directory")?.join("cinnamon/extensions").join(UUID))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_writes_only_what_changed() {
        let dir = std::env::temp_dir().join(format!("padwight-cinnamon-install-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(write_files(&dir).unwrap(), "a new install writes the files");
        assert!(!write_files(&dir).unwrap(), "a current install writes nothing");
        let (name, content) = FILES[1];
        fs::write(dir.join(name), "an older extension").unwrap();
        assert!(write_files(&dir).unwrap(), "an out-of-date file is rewritten");
        assert_eq!(fs::read_to_string(dir.join(name)).unwrap(), content);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn enabled_extensions_round_trip() {
        assert!(parse_list("@as []\n").is_empty());
        let list = parse_list("['a@b', 'c@d']\n");
        assert_eq!(list, ["a@b", "c@d"]);
        assert_eq!(format_list(&list), "['a@b', 'c@d']");
        assert_eq!(format_list(&[]), "[]");
    }
}
