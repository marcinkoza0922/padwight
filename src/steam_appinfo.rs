//! What Steam's app cache (`appcache/appinfo.vdf`) says about a game's controller support.
//! Games Steam lists with full controller support are left to Steam Input when Steam starts them
//! (see the daemon's Steam hand-off).

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Mutex,
    time::SystemTime,
};

/// Every app's controller support from Steam's app caches, and the files' modification times
/// when they were read. Steam rewrites the cache as it learns about games, so a changed time
/// means reading it again: a game installed after the daemon started is then found.
struct Index {
    stamps: Vec<(PathBuf, Option<SystemTime>)>,
    apps: HashMap<u32, String>,
}

/// Steam's own words for how well a game plays with a controller: `"full"`, `"partial"`, or
/// `None` when it says nothing (or can't be read).
pub fn controller_support(app_id: &str) -> Option<String> {
    static INDEX: Mutex<Option<Index>> = Mutex::new(None);
    let id: u32 = app_id.parse().ok()?;
    let mut index = INDEX.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    support_in(&mut index, &appinfo_files(), id)
}

/// App `id`'s controller support in `files`, through `index`, which is read again when a file
/// changed since.
fn support_in(index: &mut Option<Index>, files: &[PathBuf], id: u32) -> Option<String> {
    let stamps = stamps(files);
    if index.as_ref().is_none_or(|i| i.stamps != stamps) {
        *index = Some(Index { apps: read(&stamps), stamps });
    }
    index.as_ref()?.apps.get(&id).cloned()
}

/// Whether Steam lists the game as playable with a controller throughout.
pub fn full_support(app_id: &str) -> bool {
    controller_support(app_id).as_deref() == Some("full")
}

/// Each Steam install's `appinfo.vdf`, the first install first.
pub fn appinfo_files() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else { return Vec::new() };
    crate::launchers::steam_roots(&home).into_iter().map(|root| root.join("appcache/appinfo.vdf")).collect()
}

fn stamps(files: &[PathBuf]) -> Vec<(PathBuf, Option<SystemTime>)> {
    files.iter().map(|f| (f.clone(), fs::metadata(f).and_then(|m| m.modified()).ok())).collect()
}

/// The apps in these files; an app in an earlier file wins over the same app in a later one.
fn read(stamps: &[(PathBuf, Option<SystemTime>)]) -> HashMap<u32, String> {
    stamps.iter().rev().filter_map(|(file, _)| fs::read(file).ok()).flat_map(|data| index(&data)).collect()
}

const V40: u32 = 0x0756_4428;
/// Version 41 moved keys into a table of strings at the end of the file.
const V41: u32 = 0x0756_4429;
/// What comes before an app's data: its ID and size, then 60 bytes of state, times and hashes.
const APP_HEADER: usize = 8;
const APP_STATE: usize = 60;

/// `common/controller_support` of every app in an appinfo.vdf file that has one. A file that
/// isn't one, or is cut short, gives the apps before the damage.
pub fn index(data: &[u8]) -> HashMap<u32, String> {
    let mut apps = HashMap::new();
    let Some(magic) = u32_at(data, 0) else { return apps };
    let (strings, mut pos, end) = match magic {
        V41 => {
            let Some((table, strings)) = u64_at(data, 8)
                .and_then(|t| usize::try_from(t).ok())
                .and_then(|t| Some((t, string_table(data, t)?)))
            else {
                return apps;
            };
            (Some(strings), 16, table)
        }
        V40 => (None, 8, data.len()),
        _ => return apps,
    };
    let reader = Reader { data, strings: strings.as_deref() };
    while pos + APP_HEADER <= end {
        let (Some(app), Some(size)) = (u32_at(data, pos), u32_at(data, pos + 4)) else { break };
        if app == 0 {
            break;
        }
        let body = pos + APP_HEADER;
        if let Some(support) = reader.lookup(body + APP_STATE, &["appinfo", "common", "controller_support"]) {
            apps.insert(app, support);
        }
        pos = body + size as usize;
    }
    apps
}

fn u32_at(data: &[u8], pos: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(pos..pos + 4)?.try_into().ok()?))
}

fn u64_at(data: &[u8], pos: usize) -> Option<u64> {
    Some(u64::from_le_bytes(data.get(pos..pos + 8)?.try_into().ok()?))
}

fn cstr(data: &[u8], pos: usize) -> Option<(&[u8], usize)> {
    let len = data.get(pos..)?.iter().position(|b| *b == 0)?;
    Some((&data[pos..pos + len], pos + len + 1))
}

fn string_table(data: &[u8], mut pos: usize) -> Option<Vec<&[u8]>> {
    let count = u32_at(data, pos)?;
    pos += 4;
    let mut strings = Vec::new();
    for _ in 0..count {
        let (s, next) = cstr(data, pos)?;
        strings.push(s);
        pos = next;
    }
    Some(strings)
}

/// Steam's binary key-values: a type byte, a key, then the value; a section ends with 8.
struct Reader<'a> {
    data: &'a [u8],
    /// Version 41's key table (keys are indexes into it); version 40 writes keys in place.
    strings: Option<&'a [&'a [u8]]>,
}

const SECTION: u8 = 0;
const STRING: u8 = 1;
const INT: u8 = 2;
const FLOAT: u8 = 3;
const POINTER: u8 = 4;
const COLOR: u8 = 6;
const UINT64: u8 = 7;
const END: u8 = 8;
const INT64: u8 = 10;

impl Reader<'_> {
    fn key(&self, pos: usize) -> Option<(&[u8], usize)> {
        match self.strings {
            Some(table) => Some((table.get(u32_at(self.data, pos)? as usize)?, pos + 4)),
            None => cstr(self.data, pos),
        }
    }

    /// The string at `path` in the section starting at `pos`, skipping everything else.
    fn lookup(&self, mut pos: usize, path: &[&str]) -> Option<String> {
        let (want, rest) = path.split_first()?;
        loop {
            let kind = *self.data.get(pos)?;
            if kind == END {
                return None;
            }
            let (key, value) = self.key(pos + 1)?;
            let hit = key.eq_ignore_ascii_case(want.as_bytes());
            match kind {
                SECTION if hit => return self.lookup(value, rest),
                SECTION => pos = self.skip_section(value)?,
                STRING if hit && rest.is_empty() => return Some(String::from_utf8_lossy(cstr(self.data, value)?.0).into_owned()),
                STRING => pos = cstr(self.data, value)?.1,
                INT | FLOAT | POINTER | COLOR => pos = value + 4,
                UINT64 | INT64 => pos = value + 8,
                _ => return None,
            }
        }
    }

    /// Where the section whose entries start at `pos` ends.
    fn skip_section(&self, mut pos: usize) -> Option<usize> {
        loop {
            let kind = *self.data.get(pos)?;
            if kind == END {
                return Some(pos + 1);
            }
            let (_, value) = self.key(pos + 1)?;
            pos = match kind {
                SECTION => self.skip_section(value)?,
                STRING => cstr(self.data, value)?.1,
                INT | FLOAT | POINTER | COLOR => value + 4,
                UINT64 | INT64 => value + 8,
                _ => return None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version 41 file with these apps, each `(id, controller_support)`.
    fn file(apps: &[(u32, Option<&str>)]) -> Vec<u8> {
        let keys = ["appinfo", "common", "name", "controller_support", "type"];
        let key = |k: &str| (keys.iter().position(|x| *x == k).unwrap() as u32).to_le_bytes();
        let mut body = Vec::new();
        for (id, support) in apps {
            let mut kv = Vec::new();
            kv.push(SECTION);
            kv.extend(key("appinfo"));
            kv.push(INT);
            kv.extend(key("type"));
            kv.extend(7u32.to_le_bytes());
            kv.push(SECTION);
            kv.extend(key("common"));
            kv.push(STRING);
            kv.extend(key("name"));
            kv.extend(b"Game\0");
            if let Some(s) = support {
                kv.push(STRING);
                kv.extend(key("controller_support"));
                kv.extend(s.as_bytes());
                kv.push(0);
            }
            kv.extend([END, END, END]);
            body.extend(id.to_le_bytes());
            body.extend(((APP_STATE + kv.len()) as u32).to_le_bytes());
            body.extend([0; APP_STATE]);
            body.extend(kv);
        }
        body.extend(0u32.to_le_bytes());
        let mut out = Vec::new();
        out.extend(V41.to_le_bytes());
        out.extend(1u32.to_le_bytes());
        out.extend(((16 + body.len()) as u64).to_le_bytes());
        out.extend(body);
        out.extend((keys.len() as u32).to_le_bytes());
        for k in keys {
            out.extend(k.as_bytes());
            out.push(0);
        }
        out
    }

    fn find(data: &[u8], id: u32) -> Option<String> {
        index(data).remove(&id)
    }

    #[test]
    fn reads_each_apps_controller_support() {
        let data = file(&[(10, Some("partial")), (2_868_840, Some("full")), (13_600, None)]);
        assert_eq!(find(&data, 2_868_840).as_deref(), Some("full"));
        assert_eq!(find(&data, 10).as_deref(), Some("partial"));
        assert_eq!(find(&data, 13_600), None, "an app that says nothing");
        assert_eq!(find(&data, 99), None, "an app that isn't there");
    }

    #[test]
    fn a_game_steam_learns_about_later_is_found() {
        let dir = std::env::temp_dir().join(format!("padwight-appinfo-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("appinfo.vdf");
        let write = |apps: &[(u32, Option<&str>)], secs: u64| {
            fs::write(&path, file(apps)).unwrap();
            let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
            fs::File::options().write(true).open(&path).unwrap().set_modified(when).unwrap();
        };
        let mut index = None;
        let files = [path.clone()];
        write(&[(10, Some("partial"))], 1_000);
        assert_eq!(support_in(&mut index, &files, 2_868_840), None);
        write(&[(10, Some("partial")), (2_868_840, Some("full"))], 2_000);
        assert_eq!(support_in(&mut index, &files, 2_868_840).as_deref(), Some("full"), "a changed file is read again");
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(support_in(&mut index, &files, 2_868_840), None, "a removed file holds nothing");
    }

    #[test]
    fn other_files_are_not_read() {
        assert_eq!(find(b"not an appinfo file", 1), None);
        assert_eq!(find(&[], 1), None);
    }
}
