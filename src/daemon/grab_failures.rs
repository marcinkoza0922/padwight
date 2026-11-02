//! Keeps the log quiet while a pad stays held by another program: the retry every scan goes on,
//! but only the first failure (and a changed one) is logged, with the holder when it can be found.

use std::{
    collections::{HashMap, HashSet},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// The last grab failure logged per device node.
#[derive(Default)]
pub(super) struct GrabFailures(HashMap<PathBuf, String>);

impl GrabFailures {
    /// Records a failure; true when it should be logged (new, or different from the last one).
    pub(super) fn note(&mut self, node: &Path, error: &str) -> bool {
        self.0.insert(node.to_path_buf(), error.to_string()).as_deref() != Some(error)
    }

    /// The grab worked: a later failure is news again.
    pub(super) fn recovered(&mut self, node: &Path) {
        self.0.remove(node);
    }

    /// Forgets nodes that are gone.
    pub(super) fn retain_present(&mut self, present: &HashSet<PathBuf>) {
        self.0.retain(|node, _| present.contains(node));
    }
}

/// The name of another program of ours (this user's) that has `node` open, if one is found.
pub(super) fn holder(node: &Path) -> Option<String> {
    let uid = std::fs::metadata("/proc/self").ok()?.uid();
    holder_in(Path::new("/proc"), node, uid, std::process::id())
}

fn holder_in(proc_root: &Path, node: &Path, uid: u32, own_pid: u32) -> Option<String> {
    for entry in std::fs::read_dir(proc_root).ok()?.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
        if pid == own_pid || entry.metadata().ok()?.uid() != uid {
            continue;
        }
        let Ok(fds) = std::fs::read_dir(entry.path().join("fd")) else { continue };
        if fds.flatten().any(|fd| std::fs::read_link(fd.path()).is_ok_and(|target| target == node)) {
            let comm = std::fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
            let comm = comm.trim();
            return Some(if comm.is_empty() { format!("pid {pid}") } else { format!("{comm} (pid {pid})") });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repeated_failure_is_logged_once_until_it_changes_or_recovers() {
        let mut f = GrabFailures::default();
        let node = Path::new("/dev/input/event5");
        assert!(f.note(node, "Device or resource busy"));
        assert!(!f.note(node, "Device or resource busy"));
        assert!(!f.note(node, "Device or resource busy"));
        assert!(f.note(node, "Permission denied"));
        f.recovered(node);
        assert!(f.note(node, "Permission denied"));
    }

    #[test]
    fn the_holder_is_found_by_its_open_file_and_only_among_our_own_processes() {
        let dir = std::env::temp_dir().join(format!("padwight-holder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let node = Path::new("/dev/input/event5");
        for (pid, comm, target) in [("100", "steam", "/dev/input/event5"), ("200", "other", "/dev/input/event6")] {
            std::fs::create_dir_all(dir.join(pid).join("fd")).unwrap();
            std::fs::write(dir.join(pid).join("comm"), format!("{comm}\n")).unwrap();
            std::os::unix::fs::symlink(target, dir.join(pid).join("fd/3")).unwrap();
        }
        let uid = std::fs::metadata(&dir).unwrap().uid();
        assert_eq!(holder_in(&dir, node, uid, 1).as_deref(), Some("steam (pid 100)"));
        assert_eq!(holder_in(&dir, node, uid, 100), None, "never ourselves");
        assert_eq!(holder_in(&dir, node, uid + 1, 1), None, "other users' processes are not looked at");
        assert_eq!(holder_in(&dir, Path::new("/dev/input/event9"), uid, 1), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
