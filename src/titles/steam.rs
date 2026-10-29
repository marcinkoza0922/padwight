//! Steam's games: the installed apps, and the user's non-Steam shortcuts.

use std::{fs, path::{Path, PathBuf}};

use crate::{
    config::Store,
    launchers::{parse_manifest, steam_libraries, steam_roots, vdf_values},
};

use super::Found;

/// SteamID64 of an account minus its id in `userdata/`.
const STEAM_ID_BASE: u64 = 76_561_197_960_265_728;

pub(super) fn scan(home: &Path) -> Vec<Found> {
    let mut found = Vec::new();
    for root in steam_roots(home) {
        for library in steam_libraries(&root) {
            let apps = library.join("steamapps");
            let Ok(entries) = fs::read_dir(&apps) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                    continue;
                }
                let Ok(text) = fs::read_to_string(entry.path()) else { continue };
                let (Some(id), dir) = parse_manifest(&text) else { continue };
                let title = vdf_values(&text, "name").into_iter().next().unwrap_or_default();
                if title.is_empty() || is_tool(&title) {
                    continue;
                }
                found.push(Found {
                    store: Store::Steam,
                    store_id: id.clone(),
                    runner: String::new(),
                    name: title,
                    steam: Some(id),
                    folder: dir.map(|d| apps.join("common").join(d)),
                    programs: Vec::new(),
                });
            }
        }
        found.extend(shortcuts(&root));
    }
    found
}

/// Proton, the Steam Linux Runtime and the like: installed as apps, but not games.
fn is_tool(name: &str) -> bool {
    ["Proton ", "Proton-", "Steam Linux Runtime", "Steamworks Common", "Steam Controller Configs"].iter().any(|p| name.starts_with(p))
}

/// The newest-logged-in account's non-Steam shortcuts, apart from those that start another
/// store's game (those are that store's titles).
fn shortcuts(root: &Path) -> Vec<Found> {
    let Some(file) = account_folder(root).map(|d| d.join("config/shortcuts.vdf")) else { return Vec::new() };
    let Ok(data) = fs::read(file) else { return Vec::new() };
    parse_shortcuts(&data)
        .into_iter()
        .filter(|s| !starts_other_store(&s.exe) && !starts_other_store(&s.launch_options))
        .map(|s| Found {
            store: Store::Steam,
            store_id: (u64::from(s.appid) << 32 | 0x0200_0000).to_string(),
            runner: String::new(),
            name: s.name,
            steam: None,
            folder: None,
            programs: vec![s.exe.trim_matches('"').to_string()],
        })
        .collect()
}

fn starts_other_store(text: &str) -> bool {
    let text = text.to_lowercase();
    text.contains("heroic://") || text.contains("lutris:")
}

/// `userdata/<account>` of the account `loginusers.vdf` lists with the newest timestamp.
fn account_folder(root: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(root.join("config/loginusers.vdf")).ok()?;
    let (mut account, mut best) = (None::<u64>, (0u64, None::<u64>));
    for line in text.lines() {
        let parts: Vec<&str> = line.split('"').collect();
        match parts.as_slice() {
            [_, id, _] if id.len() >= 15 && id.bytes().all(|b| b.is_ascii_digit()) => account = id.parse().ok(),
            [_, key, _, value, _] if key.eq_ignore_ascii_case("timestamp") => {
                let time: u64 = value.parse().unwrap_or(0);
                if account.is_some() && (best.1.is_none() || time > best.0) {
                    best = (time, account);
                }
            }
            _ => {}
        }
    }
    let id = best.1.or(account)?.checked_sub(STEAM_ID_BASE)?;
    Some(root.join("userdata").join(id.to_string()))
}

#[derive(Debug, PartialEq)]
pub(super) struct Shortcut {
    pub appid: u32,
    pub name: String,
    pub exe: String,
    pub launch_options: String,
}

enum Node {
    Map(Vec<(String, Node)>),
    Text(String),
    Int(u32),
}

/// `shortcuts.vdf`, Valve's binary KeyValues: each shortcut's `appid`, `AppName`, `Exe` and
/// `LaunchOptions` (keys in any case).
pub(super) fn parse_shortcuts(data: &[u8]) -> Vec<Shortcut> {
    let mut at = 0;
    let Node::Map(top) = read_map(data, &mut at) else { return Vec::new() };
    let Some((_, Node::Map(entries))) = top.into_iter().find(|(k, _)| k.eq_ignore_ascii_case("shortcuts")) else { return Vec::new() };
    entries
        .into_iter()
        .filter_map(|(_, node)| {
            let Node::Map(fields) = node else { return None };
            let text = |key: &str| fields.iter().find_map(|(k, v)| match v {
                Node::Text(t) if k.eq_ignore_ascii_case(key) => Some(t.clone()),
                _ => None,
            });
            let appid = fields.iter().find_map(|(k, v)| match v {
                Node::Int(i) if k.eq_ignore_ascii_case("appid") => Some(*i),
                _ => None,
            })?;
            Some(Shortcut { appid, name: text("AppName")?, exe: text("Exe").unwrap_or_default(), launch_options: text("LaunchOptions").unwrap_or_default() })
        })
        .collect()
}

/// Reads entries until the map's end marker (or the data's).
fn read_map(data: &[u8], at: &mut usize) -> Node {
    let mut items = Vec::new();
    while let Some(&kind) = data.get(*at) {
        *at += 1;
        if kind == 8 {
            break;
        }
        let key = read_cstr(data, at);
        let node = match kind {
            0 => read_map(data, at),
            1 => Node::Text(read_cstr(data, at)),
            2 => Node::Int(read_u32(data, at)),
            7 => {
                *at += 8;
                Node::Int(0)
            }
            _ => {
                *at += 4;
                Node::Int(0)
            }
        };
        items.push((key, node));
    }
    Node::Map(items)
}

fn read_cstr(data: &[u8], at: &mut usize) -> String {
    let rest = data.get(*at..).unwrap_or_default();
    let len = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
    *at += len + 1;
    String::from_utf8_lossy(&rest[..len]).into_owned()
}

fn read_u32(data: &[u8], at: &mut usize) -> u32 {
    let bytes = data.get(*at..*at + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()).unwrap_or_default();
    *at += 4;
    u32::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: &str, appid: u32, name: &str, exe: &str, options: &str) -> Vec<u8> {
        let mut v = vec![0];
        v.extend(format!("{index}\0").bytes());
        v.push(2);
        v.extend(b"appid\0");
        v.extend(appid.to_le_bytes());
        for (key, value) in [("AppName", name), ("Exe", exe), ("LaunchOptions", options)] {
            v.push(1);
            v.extend(format!("{key}\0{value}\0").bytes());
        }
        v.push(8);
        v
    }

    #[test]
    fn reads_shortcuts() {
        let mut data = b"\0shortcuts\0".to_vec();
        data.extend(entry("0", 3_135_921_720, "Cuphead", "\"flatpak\"", "run x \"heroic://launch?appName=1&runner=gog\""));
        data.extend(entry("1", 7, "Tool", "\"/usr/bin/tool\"", ""));
        data.extend([8, 8]);
        let shortcuts = parse_shortcuts(&data);
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts[0].appid, 3_135_921_720);
        assert!(starts_other_store(&shortcuts[0].launch_options) && !starts_other_store(&shortcuts[1].exe));
        // The link id Steam starts it by (checked against a real shortcut).
        assert_eq!(u64::from(shortcuts[0].appid) << 32 | 0x0200_0000, 13_468_681_230_249_623_552);
    }

    #[test]
    fn skips_tools_and_cut_off_data() {
        assert!(is_tool("Proton 9.0") && is_tool("Steam Linux Runtime 3.0 (sniper)") && !is_tool("Portal"));
        assert!(parse_shortcuts(b"\0shortcuts\0\0").is_empty());
        assert!(parse_shortcuts(&[]).is_empty());
    }
}
