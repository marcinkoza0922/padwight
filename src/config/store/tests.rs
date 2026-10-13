use super::*;
use crate::config::Profile;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("padwight-store-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_save_cut_short_loses_nothing() {
    let dir = scratch("cut-short");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Gamepad")]));
    config.save_to(&path).unwrap();
    // A save stopped (a crash, a power cut) after setting the old folders aside, before the
    // new ones were in place.
    for part in [GENERAL, SETUPS] {
        fs::rename(dir.join(part), previous(&dir, part)).unwrap();
    }
    assert_eq!(Config::load_from(&path).unwrap(), config, "the old folders are read");
    config.games.push(Game::new("Hades", vec![Profile::passthrough("Gamepad")]));
    config.save_to(&path).unwrap();
    assert_eq!(Config::load_from(&path).unwrap(), config, "the next save carries on");
    assert!(!previous(&dir, SETUPS).exists() && !dir.join(".saving").exists(), "nothing is left aside");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_config_is_saved_as_folders_and_loads_back_unchanged() {
    let dir = scratch("roundtrip");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Gamepad"), Profile::desktop("Desktop")]));
    config.games.push(Game::new("Hades", vec![Profile::passthrough("Gamepad")]));
    config.save_to(&path).unwrap();

    // App settings stay in one file, with no setups in it.
    let app = fs::read_to_string(&path).unwrap();
    assert!(!app.contains("[general") && !app.contains("[[setups"));
    // Each setup is a folder: setup.toml, then a subfolder per kind of item, numbered.
    assert!(dir.join("general/setup.toml").is_file());
    assert!(dir.join("general/profiles/001-Gamepad.toml").is_file());
    assert!(dir.join("setups/001-Doom/setup.toml").is_file());
    assert!(dir.join("setups/001-Doom/profiles/002-Desktop.toml").is_file());
    assert!(dir.join("setups/002-Hades/profiles/001-Gamepad.toml").is_file());

    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded, config);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_removed_setup_leaves_no_folder_behind() {
    let dir = scratch("removed");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Gamepad")]));
    config.save_to(&path).unwrap();
    config.games.clear();
    config.save_to(&path).unwrap();
    assert!(!dir.join("setups/001-Doom").exists());
    assert_eq!(Config::load_from(&path).unwrap().games.len(), 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_general_folder_gives_the_default_general() {
    let dir = scratch("missing-general");
    let path = dir.join("config.toml");
    Config::default().save_to(&path).unwrap();
    fs::remove_dir_all(dir.join("general")).unwrap();
    assert_eq!(Config::load_from(&path).unwrap().general, Config::default().general);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn old_shared_items_go_to_the_setups_that_use_them() {
    use crate::config::{Button, ButtonAction, MacroStep, MenuItem, MenuKind, OverlayStyle};
    let dir = scratch("shared");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    let mut doom = Game::new("Doom", vec![Profile::passthrough("Gamepad")]);
    doom.profiles[0].set_button(Button::Select, ButtonAction::OpenMenu("Wheel".into()));
    config.games.push(doom);
    config.games.push(Game::new("Hades", vec![Profile::passthrough("Gamepad")]));
    config.general.macros.push(Macro { name: "Spare".into(), steps: vec![] });
    config.save_to(&path).unwrap();
    // What an older version saved: a menu that plays a macro, and two unused macros.
    let heal = ButtonAction::Macro { name: "Heal".into(), repeat: false };
    let item = MenuItem::new("Heal", heal);
    let wheel = Menu { name: "Wheel".into(), kind: MenuKind::List, items: vec![item], cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() };
    let mac = |name: &str| Macro { name: name.into(), steps: vec![MacroStep::Wait(1)] };
    write_items(&dir.join("shared/menus"), &[wheel]).unwrap();
    write_items(&dir.join("shared/macros"), &[mac("Heal"), mac("Spare"), mac("Other")]).unwrap();

    let loaded = Config::load_from(&path).unwrap();
    let names = |g: &Game, kind| g.names(kind).iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(names(&loaded.games[0], ItemKind::Menu), ["Wheel"]);
    assert_eq!(names(&loaded.games[0], ItemKind::Macro), ["Heal"], "followed through the menu");
    assert!(names(&loaded.games[1], ItemKind::Macro).is_empty(), "Hades uses none");
    assert_eq!(names(&loaded.general, ItemKind::Macro), ["Spare", "Spare (2)", "Other"], "unused ones go to General");
    loaded.save_to(&path).unwrap();
    assert!(!dir.join("shared").exists());
    assert_eq!(Config::load_from(&path).unwrap(), loaded);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn names_with_slashes_make_safe_file_names() {
    assert_eq!(slug("Dark Souls: Remastered/2"), "Dark Souls_ Remastered_2");
    assert_eq!(slug("  "), "item");
    assert_eq!(numbered(0, "Doom"), "001-Doom");
}

#[test]
fn a_file_that_doesn_t_load_is_left_out_and_kept_on_disk() {
    let dir = scratch("unreadable");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Gamepad"), Profile::desktop("Desktop")]));
    config.games.push(Game::new("Hades", vec![Profile::passthrough("Gamepad")]));
    config.save_to(&path).unwrap();
    // A hand edit gone wrong, and a setup from a newer version.
    let profile = dir.join("setups/001-Doom/profiles/002-Desktop.toml");
    fs::write(&profile, "name = \"Desktop\"\nbuttons = [").unwrap();
    let hades = dir.join("setups/002-Hades/setup.toml");
    fs::write(&hades, "name = \"Hades\"\nrules = 3\n").unwrap();

    let mut loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.games.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Doom"], "the rest loads");
    let profiles = |c: &Config| c.games[0].profiles.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    assert_eq!(profiles(&loaded), ["Gamepad"]);
    let kept: Vec<_> = loaded.unreadable.iter().map(|u| u.path.clone()).collect();
    assert_eq!(kept, [PathBuf::from("setups/001-Doom/profiles/002-Desktop.toml"), PathBuf::from("setups/002-Hades")]);
    assert!(loaded.unreadable[0].error.contains("002-Desktop.toml"), "{}", loaded.unreadable[0].error);

    // A save puts both back. A new profile saved in the left-out one's place keeps it, and the
    // left-out one goes beside it.
    loaded.games[0].profiles.push(Profile::desktop("Desktop"));
    loaded.save_to(&path).unwrap();
    assert!(Config::load_from(&path).unwrap().games[0].profiles.len() == 2);
    let beside = dir.join("setups/001-Doom/profiles/002-Desktop (2).toml");
    assert_eq!(fs::read_to_string(&beside).unwrap(), "name = \"Desktop\"\nbuttons = [");
    assert_eq!(fs::read_to_string(&hades).unwrap(), "name = \"Hades\"\nrules = 3\n");
    assert!(dir.join("setups/002-Hades/profiles/001-Gamepad.toml").is_file(), "with the rest of its folder");
    let again = Config::load_from(&path).unwrap();
    assert_eq!(profiles(&again), ["Gamepad", "Desktop"]);
    assert_eq!(again.unreadable.len(), 2, "and they're tried again");

    // Fixed, it loads again.
    fs::write(&hades, toml::to_string(&without(&Game::new("Hades", Vec::new()), &SETUP_KEYS).unwrap()).unwrap()).unwrap();
    assert_eq!(Config::load_from(&path).unwrap().games.len(), 2);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generals_settings_that_don_t_load_keep_its_items_and_stop_saves() {
    let dir = scratch("unreadable-general");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.general.macros.push(Macro { name: "Spare".into(), steps: vec![] });
    config.save_to(&path).unwrap();
    let setup = dir.join("general/setup.toml");
    fs::write(&setup, "name = [").unwrap();

    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.general.names(ItemKind::Macro), ["Spare"]);
    assert_eq!(loaded.unreadable[0].path, Path::new("general/setup.toml"));
    let err = loaded.save_to(&path).unwrap_err();
    assert!(format!("{err:#}").starts_with("not saved"), "{err:#}");
    assert_eq!(fs::read_to_string(&setup).unwrap(), "name = [", "left as it was");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn general_keeps_a_profile_when_its_own_are_left_out() {
    let dir = scratch("unreadable-general-profiles");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.general.profiles.truncate(1);
    config.save_to(&path).unwrap();
    fs::write(dir.join("general/profiles/001-Gamepad.toml"), "?").unwrap();
    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.general.profiles, Config::default().general.profiles);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn left_out_files_go_back_where_a_save_reads_them() {
    let root = Path::new("/c");
    assert_eq!(unreadable::kept_at(root, Path::new("/c/.setups.old/001-Doom")), Path::new("setups/001-Doom"));
    assert_eq!(unreadable::kept_at(root, Path::new("/c/shared/macros/001-Heal.toml")), Path::new("general/macros/001-Heal.toml"));
    assert_eq!(unreadable::kept_at(root, Path::new("/c/general/setup.toml")), Path::new("general/setup.toml"));
}
