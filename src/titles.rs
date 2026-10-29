//! Importing titles (games that can be launched) from Steam, Heroic and Lutris, and keeping the
//! user's list in step with what the stores list now.

mod heroic;
mod lutris;
mod open;
mod steam;

pub use open::open_link;

use std::{collections::HashSet, path::PathBuf};

use crate::{
    config::{Rule, Store, Title},
    launchers::Installed,
    library::Listing,
};

/// A game a store lists as installed.
#[derive(Debug, Clone)]
pub struct Found {
    pub store: Store,
    pub store_id: String,
    pub runner: String,
    pub name: String,
    /// Its Steam App ID, for the setups' `SteamAppId` rules.
    pub steam: Option<String>,
    /// Its install folder, searched for the programs the rules name.
    pub folder: Option<PathBuf>,
    /// Programs the store says it starts.
    pub programs: Vec<String>,
}

impl Found {
    fn id(&self) -> String {
        match self.store {
            Store::Steam => format!("steam-{}", self.store_id),
            Store::Heroic => format!("heroic-{}-{}", self.runner, self.store_id),
            Store::Lutris => format!("lutris-{}", self.store_id),
        }
    }
}

/// An imported game, with the setup whose rules match it and the library game it looks like.
#[derive(Debug, Clone)]
pub struct Scanned {
    pub title: Title,
    /// Name of the user's setup whose rules match it.
    pub setup: Option<String>,
    /// Index in the library of the built-in game whose rules match it, when no setup does.
    pub library: Option<usize>,
}

/// Everything the stores list as installed, matched to `setups` (name and rules) and the
/// library. Reads the disk: run it off the UI thread.
pub fn scan(setups: &[(String, Vec<Rule>)], library: &[Listing]) -> Vec<Scanned> {
    let Some(home) = dirs::home_dir() else { return Vec::new() };
    let mut seen = HashSet::new();
    let found = steam::scan(&home).into_iter().chain(heroic::scan(&home)).chain(lutris::scan(&home));
    found
        .filter(|f| seen.insert(f.id()))
        .map(|f| {
            let installed = Installed::of_title(f.steam.as_deref(), f.folder.as_deref(), &f.programs);
            let setup = setups.iter().find(|(_, rules)| installed.has(rules)).map(|(name, _)| name.clone());
            let library = if setup.is_some() { None } else { library.iter().position(|l| installed.has(&l.rules)) };
            let title = Title {
                id: f.id(),
                name: f.name,
                source: f.store,
                store_id: f.store_id,
                runner: f.runner,
                setup: String::new(),
                hidden: false,
                installed: true,
            };
            Scanned { title, setup, library }
        })
        .collect()
}

/// Folds an import into the user's titles. Titles found again keep their setup and whether
/// they're hidden; new ones get the setup whose rules match; the ones the stores no longer list
/// stay, marked not installed.
pub fn merge(titles: &mut Vec<Title>, scanned: &[Scanned]) {
    let found: HashSet<&str> = scanned.iter().map(|s| s.title.id.as_str()).collect();
    for title in titles.iter_mut().filter(|t| !found.contains(t.id.as_str())) {
        title.installed = false;
    }
    for item in scanned {
        match titles.iter_mut().find(|t| t.id == item.title.id) {
            Some(title) => {
                title.name.clone_from(&item.title.name);
                title.installed = true;
            }
            None => titles.push(Title { setup: item.setup.clone().unwrap_or_default(), ..item.title.clone() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanned(id: &str, name: &str, setup: Option<&str>) -> Scanned {
        let title = Title { id: id.into(), name: name.into(), source: Store::Steam, store_id: "1".into(), runner: String::new(), setup: String::new(), hidden: false, installed: true };
        Scanned { title, setup: setup.map(str::to_string), library: None }
    }

    #[test]
    fn merging_keeps_choices_and_marks_missing_titles() {
        let mut titles = vec![
            Title { setup: "Mine".into(), hidden: true, ..scanned("steam-1", "Old name", None).title },
            scanned("steam-2", "Gone", None).title,
        ];
        merge(&mut titles, &[scanned("steam-1", "New name", Some("Other")), scanned("steam-3", "Fresh", Some("Doom"))]);
        assert_eq!(titles.len(), 3);
        let (first, gone, fresh) = (&titles[0], &titles[1], &titles[2]);
        assert_eq!((first.name.as_str(), first.setup.as_str(), first.hidden, first.installed), ("New name", "Mine", true, true));
        assert!(!gone.installed);
        assert_eq!((fresh.setup.as_str(), fresh.installed), ("Doom", true));
        // Coming back installs it again.
        merge(&mut titles, &[scanned("steam-2", "Gone", None)]);
        assert!(titles[1].installed && !titles[0].installed);
    }

    #[test]
    fn ids_tell_stores_and_runners_apart() {
        let found = |store, runner: &str| Found { store, store_id: "7".into(), runner: runner.into(), name: String::new(), steam: None, folder: None, programs: Vec::new() };
        assert_eq!(found(Store::Steam, "").id(), "steam-7");
        assert_eq!(found(Store::Heroic, "gog").id(), "heroic-gog-7");
        assert_eq!(found(Store::Lutris, "").id(), "lutris-7");
    }

    #[expect(clippy::print_stdout, reason = "a manual check of the real stores")]
    /// Prints what the stores on this machine list: `cargo test titles::tests::this_machine -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the real stores"]
    fn this_machine() {
        let found = scan(&[], &[]);
        for s in &found {
            println!("{:<28} {:<9} {}", s.title.id, s.title.source.label(), s.title.name);
        }
        println!("{} titles", found.len());
    }
}
