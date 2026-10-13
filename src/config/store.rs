//! The config folder on disk. `config.toml` holds the app settings. Every setup is a folder of its
//! own: `setup.toml` has its name, rules and looks, and each profile, layer, macro, menu, info
//! overlay and log overlay is a file in a subfolder for its kind. General sits beside the setups in
//! the same shape, so a folder (or one subfolder of it) can be copied to another machine as it is.
//!
//! Files are numbered so they load in the order they were saved in. Each save rewrites the whole
//! tree, so renamed or removed items leave nothing behind, but a file that doesn't load is left
//! out of the config and kept on disk (see [`unreadable`]).

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

use super::{Config, Game, InfoOverlay, ItemKind, Layer, LogOverlay, Macro, Menu, Profile, free_name};

mod unreadable;
pub use unreadable::Unreadable;

/// The file in a setup's folder that holds its name, rules and looks.
const SETUP_FILE: &str = "setup.toml";
const SETUPS: &str = "setups";
const GENERAL: &str = "general";
/// Where older versions kept items every setup could use. Read once, folded into the setups (see
/// [`fold_in_shared`]) and removed by the next save.
const SHARED: &str = "shared";
/// The keys of `config.toml` that live in the folders instead.
const APP_KEYS: [&str; 3] = ["general", "setups", "unreadable"];
/// The keys of a setup that live in its subfolders instead.
const SETUP_KEYS: [&str; 7] = ["profiles", "macros", "menus", "info_overlays", "log_overlays", "layers", "modes"];

/// An item that is saved as a file named after it.
trait Named {
    fn name(&self) -> &str;
}

macro_rules! named {
    ($($item:ty),*) => {
        $(impl Named for $item {
            fn name(&self) -> &str {
                &self.name
            }
        })*
    };
}

named!(Profile, Layer, Macro, Menu, InfoOverlay, LogOverlay);

/// The folder that holds `config.toml`, and with it the setups.
pub fn root_of(path: &Path) -> &Path {
    path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."))
}

/// Fills in the setups of `config` from the folder at `root`. A missing General folder gives the
/// default General. Files that don't load are left out and kept in `config.unreadable`: an item
/// file on its own, a setup whose `setup.toml` doesn't load with its whole folder. General's
/// `setup.toml` gives the default General's settings, with its items read as usual. A setup that
/// couldn't be saved as it is (see [`super::problems`]) is left out and kept with its folder too.
pub fn load(root: &Path, config: &mut Config) -> Result<()> {
    let mut reading = Reading { root, kept: Vec::new() };
    let general = saved(root, GENERAL);
    config.general = if general.join(SETUP_FILE).exists() {
        reading.setup(&general, true)?.unwrap_or_else(|| Config::default().general)
    } else {
        Config::default().general
    };
    config.games = Vec::new();
    let mut dirs = Vec::new();
    for dir in setup_dirs(&saved(root, SETUPS))? {
        if let Some(game) = reading.setup(&dir, false)? {
            config.games.push(game);
            dirs.push(dir);
        }
    }
    let shared = root.join(SHARED);
    if shared.exists() {
        let mut items = Game::new("Shared", Vec::new());
        items.macros = reading.items(&shared.join("macros"))?;
        items.menus = reading.items(&shared.join("menus"))?;
        items.info = reading.items(&shared.join("info"))?;
        items.logs = reading.items(&shared.join("logs"))?;
        fold_in_shared(config, &items);
    }
    // General needs a profile, for the desktop; its own may all have been left out.
    if config.general.profiles.is_empty() {
        config.general.profiles = Config::default().general.profiles;
    }
    // Checked once upgraded and the old shared items are in, as they may be what a setup refers to.
    config.upgrade_games();
    reading.leave_out_invalid(config, &general, dirs)?;
    config.unreadable = reading.kept;
    Ok(())
}

/// Gives every setup its own copy of the old shared items it uses (followed through menus and
/// macros), where it has none of that name. Those no setup uses go to General, renamed if General
/// has one of that name.
fn fold_in_shared(config: &mut Config, shared: &Game) {
    let mut used = Vec::new();
    for game in std::iter::once(&mut config.general).chain(&mut config.games) {
        loop {
            let missing: Vec<(ItemKind, String)> = crate::pack::game_references(game)
                .into_iter()
                .filter(|(kind, name)| !game.names(*kind).contains(&name.as_str()) && shared.names(*kind).contains(&name.as_str()))
                .collect();
            if missing.is_empty() {
                break;
            }
            for (kind, name) in missing {
                copy_item(game, shared, kind, &name, &name);
                used.push((kind, name));
            }
        }
    }
    for kind in [ItemKind::Macro, ItemKind::Menu, ItemKind::Info, ItemKind::Log] {
        for name in shared.names(kind) {
            if !used.iter().any(|(k, n)| *k == kind && n == name) {
                let new = free_name(name, |n| config.general.names(kind).contains(&n));
                copy_item(&mut config.general, shared, kind, name, &new);
            }
        }
    }
}

/// Copies the `kind` item `name` of `from` into `to`, as `new`.
fn copy_item(to: &mut Game, from: &Game, kind: ItemKind, name: &str, new: &str) {
    match kind {
        ItemKind::Macro => to.macros.extend(from.macros.iter().find(|m| m.name == name).map(|m| Macro { name: new.into(), ..m.clone() })),
        ItemKind::Menu => to.menus.extend(from.menus.iter().find(|m| m.name == name).map(|m| Menu { name: new.into(), ..m.clone() })),
        ItemKind::Info => to.info.extend(from.info.iter().find(|o| o.name == name).map(|o| InfoOverlay { name: new.into(), ..o.clone() })),
        ItemKind::Log => to.logs.extend(from.logs.iter().find(|o| o.name == name).map(|o| LogOverlay { name: new.into(), ..o.clone() })),
        ItemKind::Layer | ItemKind::Mode => {}
    }
}

/// Writes every setup under `root`, replacing what is there. The new tree is built
/// beside the old one and then swapped in, so a save that fails part way leaves the old setups.
pub fn save(root: &Path, config: &Config) -> Result<()> {
    unreadable::check_save(&config.unreadable)?;
    let staging = root.join(".saving");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    write_setup(&staging.join(GENERAL), &config.general)?;
    fs::create_dir_all(staging.join(SETUPS))?;
    for (i, game) in config.games.iter().enumerate() {
        write_setup(&staging.join(SETUPS).join(numbered(i, &game.name)), game)?;
    }
    unreadable::put_back(&staging, &config.unreadable)?;
    for part in [GENERAL, SETUPS] {
        // The old folder is set aside, not deleted, until the new one is in place: at every
        // moment one of them is whole, and `saved` reads whichever is there.
        let (target, old) = (root.join(part), previous(root, part));
        if target.exists() {
            if old.exists() {
                fs::remove_dir_all(&old).with_context(|| format!("removing {}", old.display()))?;
            }
            fs::rename(&target, &old).with_context(|| format!("replacing {}", target.display()))?;
        }
        fs::rename(staging.join(part), &target).with_context(|| format!("writing {}", target.display()))?;
        if old.exists() {
            fs::remove_dir_all(&old).with_context(|| format!("removing {}", old.display()))?;
        }
    }
    fs::remove_dir_all(&staging)?;
    // Folded into the setups when it was loaded.
    let shared = root.join(SHARED);
    if shared.exists() {
        fs::remove_dir_all(&shared).with_context(|| format!("removing {}", shared.display()))?;
    }
    Ok(())
}

/// Where a save sets `part` (General or the setups) aside while it puts the new one in place.
fn previous(root: &Path, part: &str) -> PathBuf {
    root.join(format!(".{part}.old"))
}

/// The folder `part` was last saved in whole: itself, or, when a save stopped (or is) between
/// setting the old one aside and putting the new one in place, the old one.
fn saved(root: &Path, part: &str) -> PathBuf {
    let (target, old) = (root.join(part), previous(root, part));
    if !target.exists() && old.exists() { old } else { target }
}

/// `config.toml`: the app settings, without the setups, which have folders of their own.
pub fn app_settings(config: &Config) -> Result<String> {
    Ok(toml::to_string_pretty(&without(config, &APP_KEYS)?)?)
}

/// Writes `text` to `path`, readable only by its owner.
pub fn write_private(path: &Path, text: impl AsRef<[u8]>) -> Result<()> {
    private_file(path)?.write_all(text.as_ref())?;
    Ok(())
}

/// Creates (or empties) `path` for writing, readable only by its owner. The mode is set when the
/// file is created, and set again in case an earlier run left one with a looser mode.
#[cfg(target_os = "linux")]
fn private_file(path: &Path) -> Result<fs::File> {
    let file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

/// Creates (or empties) `path` for writing. The config folder is in the user's profile, which
/// only they can read.
#[cfg(windows)]
fn private_file(path: &Path) -> Result<fs::File> {
    Ok(fs::File::create(path)?)
}

/// A setup's folder: `setup.toml`, then one subfolder per kind of item.
fn write_setup(dir: &Path, game: &Game) -> Result<()> {
    fs::create_dir_all(dir)?;
    write_private(&dir.join(SETUP_FILE), &toml::to_string_pretty(&without(game, &SETUP_KEYS)?)?)?;
    write_items(&dir.join("profiles"), &game.profiles)?;
    write_items(&dir.join("layers"), &game.layers)?;
    write_items(&dir.join("modes"), &game.modes)?;
    write_items(&dir.join("macros"), &game.macros)?;
    write_items(&dir.join("menus"), &game.menus)?;
    write_items(&dir.join("info"), &game.info)?;
    write_items(&dir.join("logs"), &game.logs)?;
    Ok(())
}

/// A load of the config folder at `root`, keeping what doesn't load.
struct Reading<'a> {
    root: &'a Path,
    kept: Vec<Unreadable>,
}

impl Reading<'_> {
    /// The setup in `dir`, or none when its `setup.toml` doesn't load and the folder is kept.
    /// General's (`general`) is kept as a file, and its items are read.
    fn setup(&mut self, dir: &Path, general: bool) -> Result<Option<Game>> {
        let path = dir.join(SETUP_FILE);
        let mut game: Game = match read_toml(&path) {
            Ok(game) => game,
            Err(e) if general => {
                self.kept.push(Unreadable::file(self.root, &path, &e)?);
                Config::default().general
            }
            Err(e) => {
                self.kept.push(Unreadable::setup(self.root, dir, &e)?);
                return Ok(None);
            }
        };
        game.profiles = self.items(&dir.join("profiles"))?;
        game.layers = self.items(&dir.join("layers"))?;
        game.modes = self.items(&dir.join("modes"))?;
        game.macros = self.items(&dir.join("macros"))?;
        game.menus = self.items(&dir.join("menus"))?;
        game.info = self.items(&dir.join("info"))?;
        game.logs = self.items(&dir.join("logs"))?;
        Ok(Some(game))
    }

    /// Leaves out the setups that couldn't be saved as they are, keeping their folders (`dirs`,
    /// one per setup in `config`). General then gets the default's, and saving waits.
    fn leave_out_invalid(&mut self, config: &mut Config, general: &Path, dirs: Vec<PathBuf>) -> Result<()> {
        let kept = |dir: &Path, problem: String| anyhow::anyhow!("checking {}: {problem}", dir.display());
        // A General whose `setup.toml` didn't load already holds up saving.
        if !self.kept.iter().any(Unreadable::blocks_save)
            && let Some(problem) = super::problems::game_problem(config, &config.general)
        {
            self.kept.push(Unreadable::setup(self.root, general, &kept(general, problem))?);
            config.general = Config::default().general;
        }
        let games = std::mem::take(&mut config.games);
        for (game, dir) in games.into_iter().zip(dirs) {
            let problem = if game.name.trim().is_empty() {
                Some("A setup needs a name.".into())
            } else if config.games.iter().any(|g| g.name == game.name) {
                Some(format!("Two setups are named “{}”.", game.name))
            } else {
                super::problems::game_problem(config, &game)
            };
            match problem {
                Some(problem) => self.kept.push(Unreadable::setup(self.root, &dir, &kept(&dir, problem))?),
                None => config.games.push(game),
            }
        }
        Ok(())
    }

    /// The items saved in `dir`, in file name order, but for those kept as they don't load. A
    /// missing folder has none.
    fn items<T: DeserializeOwned>(&mut self, dir: &Path) -> Result<Vec<T>> {
        let mut items = Vec::new();
        for path in item_files(dir)? {
            match read_toml(&path) {
                Ok(item) => items.push(item),
                Err(e) => self.kept.push(Unreadable::file(self.root, &path, &e)?),
            }
        }
        Ok(items)
    }
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    super::finite::from_toml(&text).with_context(|| format!("parsing {}", path.display()))
}

/// The setup folders under `dir` (those with a `setup.toml`), in name order.
fn setup_dirs(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.join(SETUP_FILE).is_file())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// The item files in `dir`, in name order. A missing folder has none.
fn item_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    Ok(files)
}

/// One file per item, named by its position and name, so the files sort in the list's order.
fn write_items<T: Serialize + Named>(dir: &Path, items: &[T]) -> Result<()> {
    fs::create_dir_all(dir)?;
    for (i, item) in items.iter().enumerate() {
        write_private(&dir.join(format!("{}.toml", numbered(i, item.name()))), &toml::to_string_pretty(item)?)?;
    }
    Ok(())
}

/// A file or folder name for the item at `index`: its position, then its name.
fn numbered(index: usize, name: &str) -> String {
    format!("{:03}-{}", index + 1, slug(name))
}

/// `name` with anything that isn't a letter, digit, space, dash or underscore as an underscore.
fn slug(name: &str) -> String {
    let slug: String = name.chars().map(|c| if c.is_alphanumeric() || " -_".contains(c) { c } else { '_' }).collect();
    let slug = slug.trim();
    if slug.is_empty() { "item".into() } else { slug.chars().take(60).collect() }
}

/// `value` as a TOML table, without the `keys`.
fn without<T: Serialize>(value: &T, keys: &[&str]) -> Result<toml::Table> {
    let mut table: toml::Table = toml::from_str(&toml::to_string(value)?)?;
    for key in keys {
        table.remove(*key);
    }
    Ok(table)
}

#[cfg(test)]
mod tests;
