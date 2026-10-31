//! Which games are installed (Steam, Heroic, Lutris) or running, so the library picker can
//! put the user's own games first. Games are matched through their auto-switch rules.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::{Rule, RuleKind},
    ipc::WindowInfo,
};

/// How deep to look inside a game's install folder for its executables, and how many
/// entries to look at per game at most.
const SEARCH_DEPTH: usize = 4;
const SEARCH_BUDGET: usize = 20_000;

#[derive(Debug, Clone, Default)]
pub struct Installed {
    /// Steam App IDs with an install manifest.
    pub steam: HashSet<String>,
    /// File names (lowercase) found in installed games' folders, e.g. `eldenring.exe`.
    pub files: HashSet<String>,
    /// Full paths of the programs among them (`.exe` files, and on Linux files marked
    /// executable), by lowercase file name, for rules that name a program's folder.
    pub programs: HashMap<String, Vec<String>>,
    pub running: Vec<WindowInfo>,
}

impl Installed {
    /// True if any of a game's rules points at something installed or running.
    pub fn has(&self, rules: &[Rule]) -> bool {
        rules.iter().any(|r| {
            let value = r.value.trim();
            !value.is_empty()
                && match r.kind {
                    RuleKind::SteamAppId => self.steam.contains(value),
                    RuleKind::Executable if crate::focus::names_folder(value) => {
                        let paths = self.programs.get(&file_name(value));
                        paths.is_some_and(|paths| paths.iter().any(|p| crate::focus::path_ends_with(p, value)))
                    }
                    RuleKind::Executable => self.files.contains(&value.to_lowercase()),
                    RuleKind::WindowClass => false,
                }
        }) || rules.iter().any(|r| crate::focus::rule_matches_any(r, &self.running))
    }

    fn add_program(&mut self, path: String) {
        self.programs.entry(file_name(&path)).or_default().push(path);
    }
}

/// Looks through the launchers' files and the running processes. Reads the disk: run it off
/// the UI thread.
pub fn scan() -> Installed {
    let mut found = Installed { running: crate::focus::running_processes(), ..Installed::default() };
    let Some(home) = dirs::home_dir() else { return found };
    let mut folders = Vec::new();

    for root in steam_roots(&home) {
        for library in steam_libraries(&root) {
            let apps = library.join("steamapps");
            let Ok(entries) = fs::read_dir(&apps) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                    continue;
                }
                let Ok(text) = fs::read_to_string(entry.path()) else { continue };
                let (id, dir) = parse_manifest(&text);
                if let Some(id) = id {
                    found.steam.insert(id);
                }
                if let Some(dir) = dir {
                    folders.push(apps.join("common").join(dir));
                }
            }
        }
    }

    for heroic in heroic_configs(&home) {
        let legendary = heroic.join("legendaryConfig/legendary/installed.json");
        let gog = heroic.join("gog_store/installed.json");
        for file in [legendary, gog] {
            if let Ok(text) = fs::read_to_string(file) {
                let (dirs, exes) = parse_heroic(&text);
                folders.extend(dirs);
                found.files.extend(exes);
            }
        }
    }

    for lutris in [
        home.join(".local/share/lutris/games"),
        home.join(".config/lutris/games"),
        home.join(".var/app/net.lutris.Lutris/data/lutris/games"),
    ] {
        let Ok(entries) = fs::read_dir(lutris) else { continue };
        for entry in entries.flatten() {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                for exe in parse_lutris(&text) {
                    found.files.insert(file_name(&exe));
                    found.add_program(exe);
                }
            }
        }
    }

    for folder in folders {
        let mut budget = SEARCH_BUDGET;
        collect_files(&folder, SEARCH_DEPTH, &mut budget, &mut found);
    }
    found
}

pub(crate) fn steam_roots(home: &Path) -> Vec<PathBuf> {
    let candidates = steam_candidates(home);
    // ~/.steam/steam is usually a link to the first; look at each folder once.
    let mut seen = HashSet::new();
    candidates.into_iter().filter(|p| p.is_dir() && seen.insert(fs::canonicalize(p).unwrap_or(p.clone()))).collect()
}

/// Where Steam may be installed.
#[cfg(target_os = "linux")]
fn steam_candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/share/Steam"),
        home.join(".steam/steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ]
}

/// Heroic's config folders, where it lists the games it installed.
#[cfg(target_os = "linux")]
fn heroic_configs(home: &Path) -> Vec<PathBuf> {
    vec![home.join(".config/heroic"), home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic")]
}

/// A Steam install's library folders: its own, plus those in `libraryfolders.vdf`.
fn steam_libraries(root: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![root.to_path_buf()];
    if let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
        libraries.extend(vdf_values(&text, "path").into_iter().map(PathBuf::from));
    }
    let mut seen = HashSet::new();
    libraries.retain(|p| seen.insert(fs::canonicalize(p).unwrap_or(p.clone())));
    libraries
}

/// Every value of `key` in Valve's KeyValues text (`"key"  "value"` lines).
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('"').collect();
            // ["\t", key, "\t\t", value, ""]
            (parts.len() >= 5 && parts[1].eq_ignore_ascii_case(key)).then(|| parts[3].replace("\\\\", "\\"))
        })
        .collect()
}

/// A Steam app manifest's App ID and install folder name.
pub(crate) fn parse_manifest(text: &str) -> (Option<String>, Option<String>) {
    (vdf_values(text, "appid").into_iter().next(), vdf_values(text, "installdir").into_iter().next())
}

/// Heroic's installed-games lists (Epic via Legendary, and GOG): install folders, and
/// executable names where listed.
fn parse_heroic(text: &str) -> (Vec<PathBuf>, Vec<String>) {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return (Vec::new(), Vec::new()) };
    let games: Vec<&serde_json::Value> = match &json {
        // GOG: {"installed": [...]}; Legendary: {"AppName": {...}, ...}
        serde_json::Value::Object(map) => match map.get("installed") {
            Some(serde_json::Value::Array(list)) => list.iter().collect(),
            _ => map.values().collect(),
        },
        serde_json::Value::Array(list) => list.iter().collect(),
        _ => Vec::new(),
    };
    let (mut dirs, mut exes) = (Vec::new(), Vec::new());
    for game in games {
        if let Some(path) = game.get("install_path").and_then(|p| p.as_str()) {
            dirs.push(PathBuf::from(path));
        }
        if let Some(exe) = game.get("executable").and_then(|e| e.as_str()) {
            exes.push(file_name(exe));
        }
    }
    (dirs, exes)
}

/// The executable a Lutris game config (YAML) launches.
fn parse_lutris(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("exe:"))
        .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
        .filter(|path| !file_name(path).is_empty())
        .collect()
}

/// Lowercase file name of a Unix or Windows path.
fn file_name(path: &str) -> String {
    crate::focus::base_name(path).to_lowercase()
}

fn collect_files(dir: &Path, depth: usize, budget: &mut usize, out: &mut Installed) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if depth > 0 {
                collect_files(&entry.path(), depth - 1, budget, out);
            }
        } else if kind.is_file() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if is_program(&name, &entry) {
                out.add_program(entry.path().to_string_lossy().into_owned());
            }
            out.files.insert(name);
        }
    }
}

/// Whether a file found in a game's folder is a program a rule could name.
fn is_program(name: &str, entry: &fs::DirEntry) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if entry.metadata().is_ok_and(|m| m.permissions().mode() & 0o111 != 0) {
            return true;
        }
    }
    #[cfg(not(unix))]
    let _ = entry;
    name.ends_with(".exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steam_library_folders_and_manifests() {
        let folders = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"/home/me/.local/share/Steam"
		"label"		""
	}
	"1"
	{
		"path"		"/mnt/games/SteamLibrary"
	}
}"#;
        assert_eq!(vdf_values(folders, "path"), ["/home/me/.local/share/Steam", "/mnt/games/SteamLibrary"]);
        let manifest = "\"AppState\"\n{\n\t\"appid\"\t\t\"1245620\"\n\t\"name\"\t\t\"ELDEN RING\"\n\t\"installdir\"\t\t\"ELDEN RING\"\n}";
        assert_eq!(parse_manifest(manifest), (Some("1245620".into()), Some("ELDEN RING".into())));
    }

    #[test]
    fn reads_heroic_and_lutris_lists() {
        let legendary = r#"{"Fortnite": {"install_path": "/games/Fortnite", "executable": "FortniteGame/Binaries/Win64/FortniteClient.exe"}}"#;
        assert_eq!(parse_heroic(legendary), (vec![PathBuf::from("/games/Fortnite")], vec!["fortniteclient.exe".to_string()]));
        let gog = r#"{"installed": [{"appName": "1", "install_path": "/games/Witcher 3"}]}"#;
        assert_eq!(parse_heroic(gog).0, [PathBuf::from("/games/Witcher 3")]);
        assert_eq!(parse_heroic("not json"), (Vec::new(), Vec::new()));
        let lutris = "game:\n  exe: /home/me/Games/diablo/drive_c/Diablo II/Game.exe\n  prefix: /x\nsystem: {}\n";
        assert_eq!(parse_lutris(lutris), ["/home/me/Games/diablo/drive_c/Diablo II/Game.exe"]);
    }

    #[test]
    fn games_match_through_their_rules() {
        let installed = Installed {
            steam: HashSet::from(["1245620".to_string()]),
            files: HashSet::from(["factorio".to_string()]),
            programs: HashMap::new(),
            running: vec![WindowInfo { exe: "Hades.exe".into(), ..WindowInfo::default() }],
        };
        let rule = |kind, value: &str| vec![Rule::new(kind, value, "P")];
        assert!(installed.has(&rule(RuleKind::SteamAppId, "1245620")));
        assert!(installed.has(&rule(RuleKind::Executable, "Factorio")));
        assert!(installed.has(&rule(RuleKind::Executable, "hades.exe")), "running counts");
        assert!(!installed.has(&rule(RuleKind::Executable, "doom.exe")));
        assert!(!installed.has(&[]));
    }

    /// Games sharing a program name (both Prince of Persia packs run `gpp.exe`) name its folder
    /// in their rules; installed is the one whose folder is there.
    #[test]
    fn a_rule_naming_the_folder_finds_the_game_in_that_folder() {
        let dir = std::env::temp_dir().join(format!("launchers-folder-test-{}", std::process::id()));
        let game = dir.join("common/Prince of Persia - The Sands of Time");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("GPP.exe"), "").unwrap();
        fs::write(game.join("readme.txt"), "").unwrap();
        let (mut found, mut budget) = (Installed::default(), SEARCH_BUDGET);
        collect_files(&dir, SEARCH_DEPTH, &mut budget, &mut found);
        fs::remove_dir_all(dir).unwrap();
        let rule = |value: &str| vec![Rule::new(RuleKind::Executable, value, "P")];
        assert!(found.has(&rule("Prince of Persia - The Sands of Time/gpp.exe")));
        assert!(found.has(&rule(r"prince of persia - the sands of time\GPP.EXE")), "either slash, any case");
        assert!(!found.has(&rule("Prince of Persia - Warrior Within/gpp.exe")));
        assert!(!found.has(&rule("Sands of Time/gpp.exe")), "whole folder names only");
        assert!(!found.has(&rule("Prince of Persia - The Sands of Time/readme.txt")), "only programs");
    }

    #[test]
    fn finds_files_in_install_folders_within_limits() {
        let dir = std::env::temp_dir().join(format!("launchers-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("bin/x64")).unwrap();
        fs::write(dir.join("bin/x64/Game.EXE"), "").unwrap();
        let mut found = Installed::default();
        let mut budget = 100;
        collect_files(&dir, 4, &mut budget, &mut found);
        assert!(found.files.contains("game.exe"));
        let mut shallow = Installed::default();
        collect_files(&dir, 0, &mut 100, &mut shallow);
        assert!(shallow.files.is_empty(), "only one level, which holds just a folder");
        fs::remove_dir_all(dir).unwrap();
    }
}
