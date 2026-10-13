//! Files in the config folder that don't load: hand-edited ones with a mistake, or ones copied
//! from a newer version. Loading leaves them out, so the rest still loads, and keeps their bytes;
//! saving writes them back into the new tree, so they stay on disk to be fixed or read again by
//! the version that wrote them.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::{GENERAL, SETUP_FILE, SETUPS, SHARED, previous, write_private};

/// A file, or a setup's whole folder when its `setup.toml` doesn't load, that loading left out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unreadable {
    /// Where the next save puts it back, from the config folder: `setups/003-Doom/profiles/002-Aim.toml`,
    /// or `setups/003-Doom` for a whole setup.
    pub path: PathBuf,
    /// Why it didn't load, with the file it was read from.
    pub error: String,
    /// Its files, by path within `path` (an empty path for a single file), as they were read.
    files: Vec<(PathBuf, Vec<u8>)>,
}

impl Unreadable {
    /// Keeps the file at `file` under `root`, which didn't load for `error`.
    pub(super) fn file(root: &Path, file: &Path, error: &anyhow::Error) -> Result<Self> {
        let bytes = fs::read(file).with_context(|| format!("reading {}", file.display()))?;
        Ok(Self { path: kept_at(root, file), error: format!("{error:#}"), files: vec![(PathBuf::new(), bytes)] })
    }

    /// Keeps the whole setup folder `dir` under `root`, whose `setup.toml` didn't load for `error`.
    pub(super) fn setup(root: &Path, dir: &Path, error: &anyhow::Error) -> Result<Self> {
        let mut files = Vec::new();
        gather(dir, Path::new(""), &mut files)?;
        Ok(Self { path: kept_at(root, dir), error: format!("{error:#}"), files })
    }

    /// General's `setup.toml`: a save writes a new one, which would replace it, so saving waits
    /// until it loads.
    fn blocks_save(&self) -> bool {
        self.path == Path::new(GENERAL).join(SETUP_FILE)
    }
}

/// Every file under `dir`, by its path within `at`.
fn gather(dir: &Path, at: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            gather(&path, &at.join(entry.file_name()), files)?;
        } else {
            files.push((at.join(entry.file_name()), fs::read(&path).with_context(|| format!("reading {}", path.display()))?));
        }
    }
    Ok(())
}

/// Where `path` under `root` goes back: the same place, with a folder a save set aside read as
/// the folder itself, and the old shared items' folder as General, which they're folded into.
pub(super) fn kept_at(root: &Path, path: &Path) -> PathBuf {
    let within = path.strip_prefix(root).unwrap_or(path);
    let mut parts = within.components();
    let first = parts.next().map(Component::as_os_str).unwrap_or_default();
    let part = [GENERAL, SETUPS, SHARED]
        .into_iter()
        .find(|part| first == *part || first == previous(Path::new(""), part).as_os_str())
        .map_or(first, |part| if part == SHARED { GENERAL.as_ref() } else { part.as_ref() });
    Path::new(part).join(parts.as_path())
}

/// Refuses a save that would write over a file loading left out (see [`Unreadable::blocks_save`]).
pub(super) fn check_save(kept: &[Unreadable]) -> Result<()> {
    if let Some(u) = kept.iter().find(|u| u.blocks_save()) {
        bail!("not saved, as it would replace General's settings, which didn't load ({}). Fix or move the file, then load the config again", u.error);
    }
    Ok(())
}

/// Writes the files loading left out into the tree being saved at `staging`. One whose place a
/// saved item or setup took goes beside it, under a name with a number.
pub(super) fn put_back(staging: &Path, kept: &[Unreadable]) -> Result<()> {
    for u in kept {
        let at = free_place(&staging.join(&u.path));
        for (file, bytes) in &u.files {
            let path = if file.as_os_str().is_empty() { at.clone() } else { at.join(file) };
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            write_private(&path, bytes).with_context(|| format!("keeping {}", path.display()))?;
        }
    }
    Ok(())
}

/// `path`, or `name (2).toml`, `name (3).toml` and so on, whichever is free.
fn free_place(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    std::iter::once(path.to_path_buf())
        .chain((2..).map(|n| path.with_file_name(format!("{stem} ({n}){ext}"))))
        .find(|p| !p.exists())
        .unwrap_or_else(|| path.to_path_buf())
}
