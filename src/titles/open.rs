//! Opening a store's link, which starts the game there.

/// Hands `link` to the desktop, as a browser does for a `steam://` link. Inside the Flatpak the
/// desktop portal opens it on the host.
pub fn open_link(link: &str) -> Result<(), String> {
    open(link).map_err(|e| format!("Couldn't open {link}: {e}"))
}

#[cfg(target_os = "linux")]
fn open(link: &str) -> Result<(), String> {
    use std::{collections::HashMap, process::Command};
    use zbus::{blocking::Connection, zvariant::Value};

    if std::path::Path::new("/.flatpak-info").exists() {
        let session = Connection::session().map_err(|e| e.to_string())?;
        let options: HashMap<&str, Value> = HashMap::new();
        session
            .call_method(Some("org.freedesktop.portal.Desktop"), "/org/freedesktop/portal/desktop", Some("org.freedesktop.portal.OpenURI"), "OpenURI", &("", link, options))
            .map(drop)
            .map_err(|e| e.to_string())
    } else {
        Command::new("xdg-open").arg(link).spawn().map(drop).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
fn open(link: &str) -> Result<(), String> {
    std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", link]).spawn().map(drop).map_err(|e| e.to_string())
}
