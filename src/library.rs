//! The built-in game library: packs embedded from `packs/` at build time (see `build.rs`).

use std::sync::LazyLock;

use crate::{
    config::{Config, Feature, Rule},
    pack::{self, Pack},
};

mod files {
    include!(concat!(env!("OUT_DIR"), "/library.rs"));
}

/// Damaged copies of `text` for robustness tests: cut at every fourth line, and with a character
/// removed or replaced every `step` characters. Most won't parse; none may panic.
#[cfg(test)]
pub(crate) fn mutants(text: &str, step: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = text.match_indices('\n').step_by(4).map(|(i, _)| text[..i].to_string()).collect();
    for i in (0..chars.len()).step_by(step) {
        let mut removed = chars.clone();
        removed.remove(i);
        out.push(removed.iter().collect());
        for junk in ['"', '[', '\u{0}'] {
            let mut replaced = chars.clone();
            replaced[i] = junk;
            out.push(replaced.iter().collect());
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub file: &'static str,
    pub pack: Pack,
}

/// Every library pack, by name. Packs that don't parse are left out (tests keep that from
/// happening). Parsed once: that takes about 70 ms, and they hold about 13 MB from then on, so
/// the daemon keeps only [`listings`].
pub fn entries() -> &'static [Entry] {
    static ENTRIES: LazyLock<Vec<Entry>> = LazyLock::new(|| {
        let mut all: Vec<Entry> = files::FILES
            .iter()
            .filter_map(|(file, text)| pack::parse(text).ok().map(|pack| Entry { file, pack }))
            .collect();
        all.sort_by_key(|e| e.pack.pack.name.to_lowercase());
        all
    });
    &ENTRIES
}

/// What finding a library pack for a window takes, without the rest of the pack.
#[derive(Debug, Clone, PartialEq)]
pub struct Listing {
    pub file: &'static str,
    pub id: String,
    pub rules: Vec<Rule>,
    /// What each of its profiles needs of the controller.
    pub requires: Vec<Vec<Feature>>,
}

impl Listing {
    pub fn of(entry: &Entry) -> Self {
        let pack = &entry.pack;
        Listing { file: entry.file, id: pack.pack.id.clone(), rules: pack.rules.clone(), requires: pack.profiles.iter().map(|p| p.requires.clone()).collect() }
    }

    /// Whether a controller with `have` can play one of its profiles.
    pub fn usable_with(&self, have: &[Feature]) -> bool {
        self.requires.iter().any(|needs| needs.iter().all(|f| have.contains(f)))
    }

    /// The whole pack, parsed again from the build.
    pub fn entry(&self) -> Option<Entry> {
        let (file, text) = files::FILES.iter().find(|(file, _)| *file == self.file)?;
        pack::parse(text).ok().map(|pack| Entry { file, pack })
    }
}

/// Every library pack's [`Listing`], in the order of [`entries`]. Each pack is parsed and dropped
/// in turn, so only the listings stay (a few hundred kB, against the whole library's 13 MB).
pub fn listings() -> &'static [Listing] {
    static LISTINGS: LazyLock<Vec<Listing>> = LazyLock::new(|| {
        let mut all: Vec<(String, Listing)> = files::FILES
            .iter()
            .filter_map(|(file, text)| {
                let entry = Entry { file, pack: pack::parse(text).ok()? };
                Some((entry.pack.pack.name.to_lowercase(), Listing::of(&entry)))
            })
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        let listings = all.into_iter().map(|(_, listing)| listing).collect();
        // Parsing left megabytes freed but kept by the allocator, scattered under what stays.
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        // SAFETY: malloc_trim only hands free memory back to the system.
        unsafe {
            libc::malloc_trim(0)
        };
        listings
    });
    &LISTINGS
}

/// Games added from the library that it now has a newer version of: (game name, index of
/// the entry in `library`).
pub fn updates<'a>(config: &'a Config, library: &[Entry]) -> Vec<(&'a str, usize)> {
    config
        .games
        .iter()
        .filter_map(|g| {
            let origin = g.origin.as_ref().filter(|o| o.library)?;
            let i = library.iter().position(|e| e.pack.pack.id == origin.id)?;
            pack::compare_versions(&library[i].pack.pack.version, &origin.version)
                .is_gt()
                .then_some((g.name.as_str(), i))
        })
        .collect()
}

/// What's wrong with a pack as a library pack, beyond parsing.
#[cfg(test)]
pub fn problems(pack: &Pack) -> Vec<String> {
    let mut problems = Vec::new();
    let out = pack::export(&pack.to_game(), &pack.info());
    problems.extend(out.dangling.iter().map(|d| format!("refers to missing {d}")));
    if pack.rules.is_empty() {
        problems.push("has no auto-switch rule".into());
    }
    // Every key it presses must be a real key name.
    let game = pack.to_game();
    let mut actions: Vec<&crate::config::ButtonAction> = game.profiles.iter().flat_map(|p| p.actions()).collect();
    actions.extend(game.layers.iter().flat_map(|l| l.actions()));
    actions.extend(game.menus.iter().flat_map(|m| m.items.iter().map(|i| &i.action)));
    actions.extend(game.macros.iter().flat_map(|m| m.steps.iter().filter_map(crate::config::MacroStep::action)));
    let mut keys: Vec<&String> = actions.into_iter().flat_map(|a| a.key_names()).collect();
    for s in game.profiles.iter().flat_map(|p| [&p.left_stick, &p.right_stick]) {
        keys.extend(s.action.ring_actions().iter().flat_map(|a| a.key_names()));
    }
    for k in keys {
        if <crate::keycode::KeyCode as std::str::FromStr>::from_str(k).is_err() {
            problems.push(format!("presses unknown key {k:?}"));
        }
    }
    // Players with a plain pad must get something.
    if !pack.profiles.iter().any(|p| p.usable_with(&[])) {
        problems.push("has no profile that works on a plain pad".into());
    }
    problems
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::config::{Button, ButtonAction, Game, Profile, Rule, RuleKind};

    #[test]
    fn every_library_pack_is_valid() {
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for (file, text) in files::FILES {
            let pack = pack::parse(text).unwrap_or_else(|e| panic!("{file}: {e:#}"));
            assert_eq!(pack.format, pack::FORMAT, "{file}: write library packs at the current format");
            assert!(ids.insert(pack.pack.id.clone()), "{file}: pack ID used twice");
            assert!(names.insert(pack.pack.name.to_lowercase()), "{file}: setup name used twice");
            assert_eq!(problems(&pack), Vec::<String>::new(), "{file}");
        }
        assert_eq!(entries().len(), files::FILES.len());
    }

    /// The app's own packs add their icons, sounds and fonts to the built-in set (`assets/`) for
    /// every game, rather than carrying them, so every icon they name must be built in.
    /// The settings window asks often: the packs are parsed once.
    #[test]
    fn the_library_is_parsed_once() {
        assert!(std::ptr::eq(entries(), entries()));
        assert!(std::ptr::eq(listings(), listings()));
    }

    /// The daemon finds packs by their listings, and parses the ones it offers again.
    #[test]
    fn each_listing_is_its_entry_and_parses_back_to_it() {
        let listed: Vec<Listing> = entries().iter().map(Listing::of).collect();
        assert_eq!(listings(), listed.as_slice());
        for (listing, entry) in listings().iter().zip(entries()).step_by(17) {
            assert_eq!(listing.entry().as_ref(), Some(entry));
        }
    }

    #[test]
    fn library_packs_use_only_built_in_assets() {
        for (file, text) in files::FILES {
            let pack = pack::parse(text).unwrap();
            assert!(pack.assets.is_empty(), "{file}: move its icons, sounds and fonts into assets/");
            for name in text.split("{icon:").skip(1).filter_map(|rest| rest.split_once('}').map(|(name, _)| name)) {
                assert!(crate::icons::svg(name, [0, 0, 0]).is_some(), "{file}: no built-in icon {name:?}");
            }
        }
    }

    #[test]
    fn library_packs_dont_share_executable_names() {
        // Two packs matching the same program would both be offered for either game: a shared
        // name (gpp.exe, game.exe) must name its folder in at least all but one pack.
        let mut seen: Vec<(String, &str)> = Vec::new();
        for (file, text) in files::FILES {
            let pack = pack::parse(text).unwrap();
            for r in pack.rules.iter().filter(|r| r.kind == crate::config::RuleKind::Executable && !crate::focus::names_folder(&r.value)) {
                let name = r.value.trim().to_ascii_lowercase();
                if let Some((_, other)) = seen.iter().find(|(n, f)| *n == name && f != file) {
                    panic!("{file} and {other} both match {name}; name the folder in one of them");
                }
                seen.push((name, file));
            }
        }
    }

    #[test]
    fn shooters_offer_plain_gyro_and_flick_stick_profiles() {
        use crate::config::{GyroMode, StickAction};
        for file in ["deus-ex", "fear", "max-payne", "max-payne-2", "thief-gold", "thief-2", "system-shock-2"] {
            let text = files::FILES.iter().find(|(f, _)| f.contains(file)).map(|(_, t)| t).unwrap();
            let pack = pack::parse(text).unwrap();
            let flick = |p: &Profile| [&p.left_stick, &p.right_stick].iter().any(|s| matches!(s.action, StickAction::Flick { .. }));
            let gyro = |p: &Profile| p.gyro.mode != GyroMode::Off;
            let plain: Vec<_> = pack.profiles.iter().filter(|p| !gyro(p) && !flick(p) && p.usable_with(&[])).collect();
            assert_eq!(plain.len(), 1, "{file}: one profile without gyro");
            assert!(pack.profiles.iter().any(|p| gyro(p) && !flick(p) && p.usable_with(&[])), "{file}: gyro as an extra, playable without");
            // Flick stick turns up and down with gyro only, so it can't be played without.
            assert!(pack.profiles.iter().any(|p| flick(p) && gyro(p) && !p.usable_with(&[])), "{file}: flick stick that declares gyro");
            assert!(pack.rules.iter().all(|r| plain[0].name == r.profile), "{file}: rules start on the plain profile");
        }
    }

    #[test]
    fn starcraft_has_a_gyro_pointer_profile_that_declares_gyro() {
        use crate::config::{Feature, GyroMode};
        let text = files::FILES.iter().find(|(f, _)| f.contains("starcraft")).map(|(_, t)| t).unwrap();
        let pack = pack::parse(text).unwrap();
        let p = pack.profiles.iter().find(|p| p.name == "Gameplay + Gyro").unwrap();
        assert!(matches!(p.gyro.mode, GyroMode::Mouse { .. }));
        assert!(p.usable_with(&[Feature::Gyro]) && !p.usable_with(&[]), "tilting needs gyro, so it says so");
        assert!(pack.modes.iter().any(|l| l.name == "Menus"), "menus are a mode, not a profile");
        assert!(pack.profiles.iter().all(|p| !p.name.contains("Menus")));
        // Guide stays for things that aren't about this game: a long press of Select switches to it.
        let menus = ButtonAction::Mode("Menus".into());
        let long_press = |g: Option<&crate::config::Gestures>| g.and_then(|g| g.get(crate::config::GestureKind::LongPress)).cloned();
        for p in &pack.profiles {
            assert_eq!(long_press(p.gestures.get(&Button::Select)), Some(menus.clone()), "{}", p.name);
        }
        let mode = pack.modes.iter().find(|l| l.name == "Menus").unwrap();
        assert_eq!(long_press(mode.gestures.get(&Button::Select)), Some(menus), "and back again");
    }

    #[test]
    fn the_checks_catch_broken_packs() {
        let mut game = Game::new("Doom", vec![Profile::passthrough("P")]);
        game.profiles[0].set_button(Button::North, ButtonAction::Macro { name: "Gone".into(), repeat: false });
        let mut pack = pack::export(&game, &pack::draft(&game, false)).pack;
        pack.profiles[0].requires = vec![crate::config::Feature::Gyro];
        let found = problems(&pack);
        assert_eq!(found.len(), 3, "{found:?}");
        pack.rules.push(Rule::new(RuleKind::Executable, "doom.exe", "P"));
        assert_eq!(problems(&pack).len(), 2);
        pack.profiles[0].set_button(Button::South, ButtonAction::Keys(vec!["KEY_NOPE".into()]));
        assert!(problems(&pack).iter().any(|p| p.contains("KEY_NOPE")));
    }

    #[test]
    fn newer_library_versions_are_offered_as_updates() {
        let game = Game::new("Doom", vec![Profile::passthrough("P")]);
        let mut entry = Entry { file: "doom.padpack", pack: pack::export(&game, &pack::draft(&game, false)).pack };
        let mut config = Config::default();
        let plan = pack::plan(&config, entry.pack.clone(), true);
        pack::apply(&mut config, &plan, &pack::Choices::default());
        assert!(updates(&config, std::slice::from_ref(&entry)).is_empty());
        entry.pack.pack.version = "1.1".into();
        assert_eq!(updates(&config, std::slice::from_ref(&entry)).len(), 1);
        config.games[0].origin.as_mut().unwrap().library = false;
        assert!(updates(&config, &[entry]).is_empty(), "only library setups follow the library");
    }

    #[test]
    fn every_library_pack_survives_a_save_and_load() {
        for (file, text) in files::FILES {
            let pack = pack::parse(text).unwrap();
            let again = pack::parse(&pack.to_toml().unwrap()).unwrap_or_else(|e| panic!("{file}: {e:#}"));
            assert_eq!(again, pack, "{file}");
        }
    }

    /// Every library game added to one config, saved as folders and loaded back, comes back the
    /// same, and saving what was loaded writes the same files.
    #[test]
    fn a_config_with_every_library_game_saves_and_loads_unchanged() {
        let mut config = Config::default();
        for (file, text) in files::FILES {
            let plan = pack::plan(&config, pack::parse(text).unwrap(), true);
            pack::apply(&mut config, &plan, &pack::Choices::default());
            assert!(config.games.last().is_some_and(|g| g.origin.is_some()), "{file} was added");
        }
        assert_eq!(config.games.len(), files::FILES.len());

        let dir = std::env::temp_dir().join(format!("padwight-library-roundtrip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (first, second) = (dir.join("first/config.toml"), dir.join("second/config.toml"));
        config.save_to(&first).unwrap();
        let loaded = Config::load_from(&first).unwrap();
        for (a, b) in loaded.games.iter().zip(&config.games) {
            assert_eq!(a, b, "{}", b.name);
        }
        assert_eq!(loaded, config);
        loaded.save_to(&second).unwrap();
        let saved = tree(&dir.join("first"));
        let setups = saved.keys().filter(|p| p.ends_with("setup.toml")).count();
        assert_eq!(setups, config.games.len() + 1, "a folder per game, and General");
        assert_eq!(saved, tree(&dir.join("second")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every file under `root`, by its path inside it, with its contents.
    fn tree(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
        let mut files = std::collections::BTreeMap::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    files.insert(path.strip_prefix(root).unwrap().to_path_buf(), std::fs::read(&path).unwrap());
                }
            }
        }
        files
    }

    #[test]
    fn damaged_packs_are_refused_or_handled_without_panicking() {
        let base = Config::default();
        let mut checked = 0;
        for (_, text) in files::FILES {
            for damaged in mutants(text, 151) {
                checked += 1;
                // Whatever parses must also survive the checks and an import.
                if let Ok(pack) = pack::parse(&damaged) {
                    let _ = problems(&pack);
                    let plan = pack::plan(&base, pack, false);
                    pack::apply(&mut base.clone(), &plan, &pack::Choices::default());
                }
            }
        }
        assert!(checked > 500, "the sweep covers the packs ({checked} variants)");
    }
}
