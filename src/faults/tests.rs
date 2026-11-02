use std::{
    fs,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::Command,
};

use super::*;
use crate::config::{Config, Game, Profile};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("padwight-faults-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// The older config, and the newer one a save is trying to put in its place.
fn older() -> Config {
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Gamepad")]));
    config
}

fn newer(base: &Config) -> Config {
    let mut config = base.clone();
    config.general.profiles.push(Profile::passthrough("Extra"));
    config.games.push(Game::new("Hades", vec![Profile::passthrough("Gamepad")]));
    config
}

/// What loading gives for `config` saved cleanly, to compare a fault's aftermath with.
fn as_loaded(config: &Config, name: &str) -> Config {
    let dir = scratch(name);
    config.save_to(&dir.join("config.toml")).unwrap();
    let loaded = Config::load_from(&dir.join("config.toml")).unwrap();
    let _ = fs::remove_dir_all(&dir);
    loaded
}

/// The save points, each with how many hits to let through first. `save.write` is hit once per
/// file, so the second entry fails a write well into the tree.
const STAGES: [(&str, u64); 8] = [
    ("save.write", 0),
    ("save.write", 4),
    ("save.fsync", 0),
    ("save.fsync", 1),
    ("save.set_aside", 0),
    ("save.set_aside", 1),
    ("save.rename", 0),
    ("save.rename", 1),
];

/// Every part, whole: either the older one or the newer one, never a mix inside a part.
fn assert_whole(loaded: &Config, old: &Config, new: &Config, what: &str) {
    assert!(loaded.general == old.general || loaded.general == new.general, "{what}: General is neither version");
    assert!(loaded.games == old.games || loaded.games == new.games, "{what}: the setups are neither version");
}

fn leftovers(dir: &Path) -> Vec<String> {
    let names = fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned());
    names.filter(|n| n.starts_with('.')).collect()
}

#[test]
fn a_failed_save_leaves_the_old_setups_and_reports_an_error() {
    let base = older();
    let (old, new) = (as_loaded(&base, "old"), as_loaded(&newer(&base), "new"));
    for (point, skip) in STAGES {
        let what = format!("{point}+{skip}");
        let dir = scratch("failed");
        let path = dir.join("config.toml");
        base.save_to(&path).unwrap();
        {
            let _faults = set(&format!("{point}=fail+{skip}"));
            assert!(newer(&base).save_to(&path).is_err(), "{what}: the save reported no error");
        }
        let loaded = Config::load_from(&path).unwrap();
        // Before the first swap the old setups are untouched; after it, each part is whole.
        if point == "save.write" || (point == "save.fsync" && skip == 0) || (point == "save.set_aside" && skip == 0) {
            assert_eq!(loaded, old, "{what}: the old setups are intact");
        } else {
            assert_whole(&loaded, &old, &new, &what);
        }
        // The next save, with the disk fine again, finishes the job and leaves nothing behind.
        newer(&base).save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap(), new, "{what}: the next save carries on");
        assert_eq!(leftovers(&dir), Vec::<String>::new(), "{what}");
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_failed_save_of_the_app_settings_keeps_the_old_file() {
    let dir = scratch("settings");
    let path = dir.join("config.toml");
    let config = older();
    config.save_to(&path).unwrap();
    let before = fs::read_to_string(&path).unwrap();
    let _faults = set("save.settings_rename=fail");
    assert!(config.save_settings_to(&path).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_full_disk_on_every_write_fails_the_save_cleanly() {
    let dir = scratch("full");
    let path = dir.join("config.toml");
    let base = older();
    base.save_to(&path).unwrap();
    let old = Config::load_from(&path).unwrap();
    let _faults = set("save.*=fail");
    assert!(newer(&base).save_to(&path).is_err());
    assert_eq!(Config::load_from(&path).unwrap(), old);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_spec_has_actions_skips_and_ratios() {
    let rules = parse("a=fail, b=hang+2, c=close:1/50,d.*=crash").unwrap();
    assert_eq!(rules[0], ("a".into(), Action::Fail, 0, 1, 1));
    assert_eq!(rules[1], ("b".into(), Action::Hang, 2, 1, 1));
    assert_eq!(rules[2], ("c".into(), Action::Close, 0, 1, 50));
    assert_eq!(rules[3].1, Action::Crash);
    for bad in ["a", "a=melt", "a=fail:x/2", "a=fail:1", "a=fail:1/0"] {
        assert!(parse(bad).is_err(), "{bad}");
    }
    let _faults = set("p=fail+1:1/3");
    let fired: Vec<bool> = (0..8).map(|_| hit("p").is_some()).collect();
    assert_eq!(fired, [false, true, false, false, true, false, false, true]);
}

/// The child half of the crash test: saves the newer config with `PADWIGHT_FAULTS` armed, which
/// aborts it part way. It does nothing when run by itself.
#[test]
fn crash_child() {
    let Ok(dir) = std::env::var("PADWIGHT_CRASH_DIR") else { return };
    let path = Path::new(&dir).join("config.toml");
    arm();
    let base = Config::load_from(&path).unwrap();
    newer(&base).save_to(&path).unwrap();
    panic!("the save finished without crashing");
}

#[test]
fn a_crash_at_any_point_of_a_save_leaves_setups_that_load_whole() {
    let base = older();
    let (old, new) = (as_loaded(&base, "crash-old"), as_loaded(&newer(&base), "crash-new"));
    let exe = std::env::current_exe().unwrap();
    for (point, skip) in STAGES {
        let what = format!("{point}+{skip}");
        let dir = scratch("crash");
        let path = dir.join("config.toml");
        base.save_to(&path).unwrap();
        let status = Command::new(&exe)
            .args(["--exact", "faults::tests::crash_child", "--test-threads", "1", "--nocapture"])
            .env("PADWIGHT_CRASH_DIR", &dir)
            .env("PADWIGHT_FAULTS", format!("{point}=crash+{skip}"))
            .output()
            .unwrap()
            .status;
        assert_eq!(status.signal(), Some(libc::SIGABRT), "{what}: the child should have crashed, got {status:?}");
        // On restart: the folder loads, every part is one whole version, and a save finishes it.
        let loaded = Config::load_from(&path).unwrap();
        assert_whole(&loaded, &old, &new, &what);
        newer(&base).save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap(), new, "{what}: the next save carries on");
        assert_eq!(leftovers(&dir), Vec::<String>::new(), "{what}");
        let _ = fs::remove_dir_all(&dir);
    }
}
