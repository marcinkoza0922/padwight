//! Heroic's games: what it installed from Epic, GOG and Amazon, and the games added by hand.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{config::Store, launchers::heroic_configs};

use super::Found;

pub(super) fn scan(home: &Path) -> Vec<Found> {
    let mut found = Vec::new();
    for config in heroic_configs(home) {
        // (runner, installed list, library cache that has the names)
        for (runner, installed, library) in [
            ("legendary", "legendaryConfig/legendary/installed.json", "store_cache/legendary_library.json"),
            ("gog", "gog_store/installed.json", "store_cache/gog_library.json"),
            ("nile", "nile_config/nile/installed.json", "store_cache/nile_library.json"),
        ] {
            let Some(text) = read(&config, installed) else { continue };
            let names = names(&read(&config, library).unwrap_or_default());
            found.extend(parse_installed(&text, runner, &names));
        }
        if let Some(text) = read(&config, "sideload_apps/library.json") {
            found.extend(parse_sideload(&text));
        }
    }
    found
}

fn read(config: &Path, file: &str) -> Option<String> {
    fs::read_to_string(config.join(file)).ok()
}

/// App name to title, from a library cache (`games` or `library`, a list of `app_name`/`title`).
fn names(text: &str) -> HashMap<String, String> {
    let Ok(json) = serde_json::from_str::<Value>(text) else { return HashMap::new() };
    ["games", "library"]
        .iter()
        .filter_map(|key| json.get(key)?.as_array())
        .flatten()
        .filter_map(|game| Some((game.get("app_name")?.as_str()?.to_string(), game.get("title")?.as_str()?.to_string())))
        .collect()
}

/// A runner's installed list: `{"installed": [...]}` (GOG), `{app name: {...}}` (Legendary) or a
/// plain list (Nile). DLC and GOG's redistributables are left out.
fn parse_installed(text: &str, runner: &str, names: &HashMap<String, String>) -> Vec<Found> {
    let Ok(json) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    let entries: Vec<(Option<&str>, &Value)> = match &json {
        Value::Object(map) => match map.get("installed") {
            Some(Value::Array(list)) => list.iter().map(|g| (None, g)).collect(),
            _ => map.iter().map(|(k, g)| (Some(k.as_str()), g)).collect(),
        },
        Value::Array(list) => list.iter().map(|g| (None, g)).collect(),
        _ => Vec::new(),
    };
    entries
        .into_iter()
        .filter_map(|(key, game)| {
            let app = key.or_else(|| ["appName", "app_name", "id"].iter().find_map(|k| game.get(k)?.as_str()))?;
            if game.get("is_dlc").and_then(Value::as_bool) == Some(true) || app == "gog-redist" {
                return None;
            }
            let title = names.get(app).cloned().or_else(|| game.get("title").and_then(Value::as_str).map(str::to_string))?;
            let folder = ["install_path", "path"].iter().find_map(|k| game.get(k)?.as_str()).map(PathBuf::from);
            let program = game.get("executable").and_then(Value::as_str).filter(|e| !e.is_empty()).map(str::to_string);
            Some(Found { store: Store::Heroic, store_id: app.into(), runner: runner.into(), name: title, steam: None, folder, programs: program.into_iter().collect() })
        })
        .collect()
}

/// Games added by hand in Heroic ("Add game"): `{"games": [{app_name, title, install: {executable}}]}`.
fn parse_sideload(text: &str) -> Vec<Found> {
    let Ok(json) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    json.get("games")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|g| g.get("is_installed").and_then(Value::as_bool) != Some(false))
        .filter_map(|g| {
            let app = g.get("app_name")?.as_str()?;
            let title = g.get("title")?.as_str()?;
            let program = g.pointer("/install/executable").and_then(Value::as_str).filter(|e| !e.is_empty());
            Some(Found {
                store: Store::Heroic,
                store_id: app.into(),
                runner: "sideload".into(),
                name: title.into(),
                steam: None,
                folder: None,
                programs: program.map(str::to_string).into_iter().collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_installed_lists_with_names() {
        let names = names(r#"{"games": [{"app_name": "1460447406", "title": "Against the Storm"}], "__timestamp": 1}"#);
        let gog = r#"{"installed": [{"appName": "1460447406", "install_path": "/g/Storm", "is_dlc": false}, {"appName": "gog-redist", "install_path": "/x"}, {"appName": "9", "is_dlc": true}]}"#;
        let found = parse_installed(gog, "gog", &names);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].runner.as_str()), ("Against the Storm", "gog"));
        assert_eq!(found[0].folder, Some(PathBuf::from("/g/Storm")));
        // Legendary names its games itself and keys them by app name.
        let legendary = r#"{"Fortnite": {"title": "Fortnite", "install_path": "/g/F", "executable": "FortniteClient.exe"}}"#;
        let found = parse_installed(legendary, "legendary", &HashMap::new());
        assert_eq!((found[0].store_id.as_str(), found[0].programs.as_slice()), ("Fortnite", ["FortniteClient.exe".to_string()].as_slice()));
        assert!(parse_installed("{}", "nile", &names).is_empty() && parse_installed("nope", "gog", &names).is_empty());
    }

    #[test]
    fn reads_sideloaded_games() {
        let text = r#"{"games": [{"runner": "sideload", "app_name": "abc", "title": "GOG Galaxy", "install": {"executable": "/p/GalaxyClient.exe"}, "is_installed": true},
                                  {"app_name": "def", "title": "Gone", "is_installed": false}]}"#;
        let found = parse_sideload(text);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].runner.as_str(), found[0].programs.as_slice()), ("sideload", ["/p/GalaxyClient.exe".to_string()].as_slice()));
    }
}
