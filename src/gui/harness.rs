//! A test rig for the settings window that checks what Save & apply writes: it loads a config
//! into the app from a folder of its own, drives the app by its `Message`s, runs the real save
//! checks, writes the config the way the daemon-less save does, and diffs the files. A test then
//! reads as "do X, expect exactly this change".
//!
//! No window and no GPU: `App::update` is called directly and the tasks it returns are dropped.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::tests::{app, with_game};
use super::*;

/// The settings window on a config folder of its own (under `screenshots/`, which is untracked).
pub(super) struct Rig {
    pub(super) app: App,
    dir: PathBuf,
    /// The files as last saved, what [`Rig::save_diff`] compares with.
    before: BTreeMap<String, String>,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Rig {
    /// The app on General's page with the default config saved, so the folder holds a whole one.
    pub(super) fn new() -> Rig {
        Self::around(app())
    }

    /// The app with a game "Doom" (profiles "Play" and "Menus") shown, saved.
    pub(super) fn with_game() -> Rig {
        Self::around(with_game())
    }

    fn around(app: App) -> Rig {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let name = format!("{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed));
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("screenshots").join("gui-save-tests").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut rig = Rig { app, dir, before: BTreeMap::new() };
        rig.app.config.save_to(&rig.config_path()).unwrap();
        rig.app.saved = rig.app.config.clone();
        rig.app.undo.clear();
        rig.before = rig.files();
        rig
    }

    pub(super) fn config_path(&self) -> PathBuf {
        self.dir.join("config.toml")
    }

    pub(super) fn send(&mut self, message: Message) {
        let _ = self.app.update(message);
    }

    /// Save & apply as pressed with no daemon running: the checks first, then the files. `Err` is
    /// the refusal's text, shown to the player, with nothing written.
    pub(super) fn save(&mut self) -> Result<(), String> {
        self.app.message = None;
        self.send(Message::Save);
        if let Some((text, true)) = self.app.message.clone() {
            return Err(text);
        }
        self.app.config.save_to(&self.config_path()).map_err(|e| format!("{e:#}"))?;
        let sent = Box::new(self.app.config.clone());
        self.send(Message::Saved(Ok(sent)));
        Ok(())
    }

    /// Every file of the config folder, by path below it.
    pub(super) fn files(&self) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        collect(&self.dir, &self.dir, &mut found);
        found
    }

    /// What the next save changes in the files, as sorted lines: `path: + added line` and
    /// `path: - removed line`, a whole file as `path: + (new file)` or `path: - (removed file)`.
    /// The files are then what later saves are compared with. Panics if the save is refused.
    pub(super) fn save_diff(&mut self) -> Vec<String> {
        if let Err(e) = self.save() {
            panic!("the save was refused: {e}");
        }
        let after = self.files();
        let diff = diff_files(&self.before, &after);
        self.before = after;
        diff
    }

    /// The config as the files hold it now, read back the way the daemon would.
    pub(super) fn reloaded(&self) -> Config {
        Config::load_from(&self.config_path()).unwrap()
    }

    /// Saved, then read back: what the window shows is what the files hold.
    pub(super) fn assert_round_trip(&self) {
        assert_eq!(self.reloaded(), self.app.config, "the files hold something other than the window shows");
    }
}

fn collect(root: &Path, dir: &Path, found: &mut BTreeMap<String, String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    for e in entries {
        let path = e.path();
        if path.is_dir() {
            collect(root, &path, found);
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            found.insert(path.strip_prefix(root).unwrap().display().to_string(), text);
        }
    }
}

/// The lines that differ between two sets of files (see [`Rig::save_diff`]).
pub(super) fn diff_files(before: &BTreeMap<String, String>, after: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    for path in before.keys().chain(after.keys()).collect::<std::collections::BTreeSet<_>>() {
        match (before.get(path), after.get(path)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(_), None) => out.push(format!("{path}: - (removed file)")),
            (None, Some(_)) => out.push(format!("{path}: + (new file)")),
            (Some(a), Some(b)) => {
                let mut left: Vec<&str> = a.lines().collect();
                let mut added = Vec::new();
                for line in b.lines() {
                    match left.iter().position(|l| *l == line) {
                        Some(i) => {
                            left.remove(i);
                        }
                        None => added.push(line),
                    }
                }
                out.extend(left.iter().map(|l| format!("{path}: - {l}")));
                out.extend(added.iter().map(|l| format!("{path}: + {l}")));
            }
            (None, None) => {}
        }
    }
    out
}
