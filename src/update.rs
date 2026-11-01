//! Finding out whether a newer release exists on GitHub and, for a copy the release installer put in
//! `~/.local/bin`, replacing it in place. A copy from a package manager or a source build is never
//! touched: the update then only says where to get the new version.
//!
//! Downloads go through `curl`, `sha256sum` and `tar`, as `dist/get.sh` does, so no TLS stack is built in.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = "marcinkoza0922/padwight";

/// How this copy of padwight got onto the machine, which decides who may update it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `dist/install.sh` put it in the user's bin directory: `padwight update` can replace it.
    Installer,
    /// It sits in a system location, where a package manager (or an admin) owns it.
    Package,
    /// Anything else: a `cargo build`, `cargo install` or a copy somebody made.
    Other,
}

impl Source {
    pub fn detect() -> Self {
        match std::env::current_exe() {
            Ok(exe) => classify(&exe, &installed_path()),
            Err(_) => Self::Other,
        }
    }

    /// Whether `padwight update` may replace this copy.
    pub fn self_updating(self) -> bool {
        self == Self::Installer
    }

    /// Where to get the new version, for a copy that can't update itself.
    pub fn advice(self) -> &'static str {
        match self {
            Self::Installer => "run `padwight update`",
            Self::Package => "update it with your package manager",
            Self::Other => "rebuild it from source, or install a release with the one-line installer from the README",
        }
    }
}

/// Where `dist/install.sh` puts the binary.
fn installed_path() -> PathBuf {
    let bin = std::env::var_os("XDG_BIN_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/bin")))
        .unwrap_or_default();
    bin.join("padwight")
}

fn classify(exe: &Path, installed: &Path) -> Source {
    // A binary replaced while it runs reads back with " (deleted)" on the end.
    let exe = exe.to_string_lossy();
    let exe = Path::new(exe.strip_suffix(" (deleted)").unwrap_or(&exe));
    let same = |a: &Path, b: &Path| a == b || a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b);
    if same(exe, installed) {
        Source::Installer
    } else if ["/usr/", "/opt/", "/app/", "/nix/store/", "/snap/"].iter().any(|root| exe.starts_with(root)) {
        Source::Package
    } else {
        Source::Other
    }
}

/// The version of the newest release, from where GitHub's `releases/latest` redirects to. (No API,
/// so no rate limit and no JSON.)
pub fn latest_version() -> Result<String> {
    let url = format!("https://github.com/{REPO}/releases/latest");
    let out = Command::new("curl")
        .args(["-fsSLI", "--max-time", "20", "-o", "/dev/null", "-w", "%{url_effective}", &url])
        .output()
        .context("running curl (is it installed?)")?;
    if !out.status.success() {
        bail!("couldn't reach GitHub: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    parse_tag(&String::from_utf8_lossy(&out.stdout)).context("GitHub has no release yet")
}

/// `https://github.com/o/r/releases/tag/v0.2.0` gives `0.2.0`; the `latest` page itself (no
/// release) gives nothing.
fn parse_tag(url: &str) -> Option<String> {
    let tag = url.trim().rsplit_once("/releases/tag/")?.1;
    let version = tag.trim_start_matches('v');
    parse_version(version).map(|_| version.to_owned())
}

fn parse_version(v: &str) -> Option<[u64; 3]> {
    let mut parts = v.split('.').map(|p| p.parse::<u64>().ok());
    let version = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(version)
}

/// Whether `latest` is a newer version than `current`. Anything unreadable counts as not newer.
pub fn is_newer(latest: &str, current: &str) -> bool {
    matches!((parse_version(latest), parse_version(current)), (Some(l), Some(c)) if l > c)
}

/// Checks GitHub: the newer version, if there is one.
pub fn available() -> Result<Option<String>> {
    Ok(Some(latest_version()?).filter(|v| is_newer(v, CURRENT)))
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    let check_only = match args {
        [] => false,
        ["--check"] => true,
        _ => bail!("usage: padwight update [--check]"),
    };
    let source = Source::detect();
    let Some(latest) = available()? else {
        println!("padwight {CURRENT} is the latest version");
        return Ok(());
    };
    println!("padwight {latest} is available (this is {CURRENT})");
    if check_only {
        if !source.self_updating() {
            println!("To get it, {}.", source.advice());
        }
        return Ok(());
    }
    if !source.self_updating() {
        bail!("this copy wasn't put here by the release installer, so it isn't replaced: {}", source.advice());
    }
    install(&latest)?;
    println!("updated to {latest}");
    report_stale_system_files();
    restart_service();
    Ok(())
}

#[expect(clippy::print_stdout, reason = "CLI output")]
/// Downloads release `version`, checks its checksum and runs the installer inside it.
fn install(version: &str) -> Result<()> {
    let arch = std::env::consts::ARCH;
    if arch != "x86_64" {
        bail!("there is no release for {arch} yet; build it from source");
    }
    let name = format!("padwight-{version}-linux-{arch}");
    let base = format!("https://github.com/{REPO}/releases/download/v{version}");
    let work = Scratch::new()?;
    println!("downloading {name}.tar.gz");
    for file in [format!("{name}.tar.gz"), format!("{name}.tar.gz.sha256")] {
        run_tool(Command::new("curl").args(["-fsSL", "--max-time", "300", "-o"]).arg(work.0.join(&file)).arg(format!("{base}/{file}")), "download")?;
    }
    run_tool(Command::new("sha256sum").args(["-c", &format!("{name}.tar.gz.sha256")]).current_dir(&work.0), "checksum check: the download doesn't match")?;
    run_tool(Command::new("tar").arg("-C").arg(&work.0).arg("-xzf").arg(work.0.join(format!("{name}.tar.gz"))), "unpacking")?;
    // The installer refreshes the binary, the service file, the icon and the menu entry. It leaves
    // the system files and the restart to us.
    run_tool(Command::new(work.0.join(&name).join("install.sh")).arg("--update"), "install")
}

fn run_tool(cmd: &mut Command, what: &str) -> Result<()> {
    let status = cmd.stdout(std::process::Stdio::null()).status().with_context(|| format!("{what}: couldn't run it"))?;
    if !status.success() {
        bail!("{what} failed ({status})");
    }
    Ok(())
}

/// Asks the new binary which of the system files in /etc differ from what it ships, and prints
/// them with the commands to fix. An update never uses sudo itself, so this is the user's cue.
fn report_stale_system_files() {
    // Best effort: a failure here must not make a finished update look failed.
    let _ = Command::new(installed_path()).args(["doctor", "--files"]).status();
}

/// Restarts the background service onto the new binary, if systemd runs it. This is last: when the
/// update itself was started by the daemon, the restart ends it.
#[expect(clippy::print_stdout, reason = "CLI output")]
fn restart_service() {
    let restarted = Command::new("systemctl").args(["--user", "try-restart", "padwight"]).status().is_ok_and(|s| s.success());
    if !restarted {
        println!("restart padwight (or log out and in) to use the new version");
    }
}

/// A scratch directory that is removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self> {
        let dir = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("padwight").join(format!("update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).with_context(|| format!("making {}", dir.display()))?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_version_from_the_redirect() {
        assert_eq!(parse_tag("https://github.com/o/padwight/releases/tag/v0.2.1\n").as_deref(), Some("0.2.1"));
        assert_eq!(parse_tag("https://github.com/o/padwight/releases"), None);
        assert_eq!(parse_tag("https://github.com/o/padwight/releases/tag/nightly"), None);
    }

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("0.10.0", "0.9.3"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("junk", "0.1.0"));
    }

    #[test]
    fn tells_who_owns_the_binary() {
        let mine = Path::new("/home/u/.local/bin/padwight");
        assert_eq!(classify(mine, mine), Source::Installer);
        assert_eq!(classify(Path::new("/home/u/.local/bin/padwight (deleted)"), mine), Source::Installer);
        assert_eq!(classify(Path::new("/usr/bin/padwight"), mine), Source::Package);
        assert_eq!(classify(Path::new("/home/u/src/padwight/target/debug/padwight"), mine), Source::Other);
    }
}
