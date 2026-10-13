//! What Steam's app cache (`appcache/appinfo.vdf`) says about a game's controller support.
//! Games Steam lists with full controller support are left to Steam Input when Steam starts them
//! (see the daemon's Steam hand-off).

use std::{collections::HashMap, fs, sync::Mutex};

/// Steam's own words for how well a game plays with a controller: `"full"`, `"partial"`, or
/// `None` when it says nothing (or can't be read).
pub fn controller_support(app_id: &str) -> Option<String> {
    static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.get_or_insert_with(HashMap::new).entry(app_id.to_string()).or_insert_with(|| read(app_id)).clone()
}

/// Whether Steam lists the game as playable with a controller throughout.
pub fn full_support(app_id: &str) -> bool {
    controller_support(app_id).as_deref() == Some("full")
}

fn read(app_id: &str) -> Option<String> {
    let id: u32 = app_id.parse().ok()?;
    let home = dirs::home_dir()?;
    crate::launchers::steam_roots(&home).into_iter().find_map(|root| {
        let data = fs::read(root.join("appcache/appinfo.vdf")).ok()?;
        find(&data, id)
    })
}

const V40: u32 = 0x0756_4428;
/// Version 41 moved keys into a table of strings at the end of the file.
const V41: u32 = 0x0756_4429;
/// What comes before an app's data: its ID and size, then 60 bytes of state, times and hashes.
const APP_HEADER: usize = 8;
const APP_STATE: usize = 60;

/// `common/controller_support` of app `id` in an appinfo.vdf file.
fn find(data: &[u8], id: u32) -> Option<String> {
    let magic = u32_at(data, 0)?;
    let (strings, mut pos, end) = match magic {
        V41 => {
            let table = usize::try_from(u64_at(data, 8)?).ok()?;
            (Some(string_table(data, table)?), 16, table)
        }
        V40 => (None, 8, data.len()),
        _ => return None,
    };
    while pos + APP_HEADER <= end {
        let (app, size) = (u32_at(data, pos)?, u32_at(data, pos + 4)? as usize);
        if app == 0 {
            return None;
        }
        let body = pos + APP_HEADER;
        if app == id {
            let reader = Reader { data, strings: strings.as_deref() };
            return reader.lookup(body + APP_STATE, &["appinfo", "common", "controller_support"]);
        }
        pos = body + size;
    }
    None
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

    #[test]
    fn reads_each_apps_controller_support() {
        let data = file(&[(10, Some("partial")), (2_868_840, Some("full")), (13_600, None)]);
        assert_eq!(find(&data, 2_868_840).as_deref(), Some("full"));
        assert_eq!(find(&data, 10).as_deref(), Some("partial"));
        assert_eq!(find(&data, 13_600), None, "an app that says nothing");
        assert_eq!(find(&data, 99), None, "an app that isn't there");
    }

    #[test]
    fn other_files_are_not_read() {
        assert_eq!(find(b"not an appinfo file", 1), None);
        assert_eq!(find(&[], 1), None);
    }
}
