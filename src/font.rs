//! The font overlays are drawn in: a few free fonts bundled with the app, or any installed
//! family named by its name.

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{
        Mutex, OnceLock, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use iced::{Font, advanced::graphics::text::font_system};

/// A font shipped inside the binary (all under the SIL Open Font License; see `assets/fonts`).
pub struct Bundled {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub const BUNDLED: [Bundled; 19] = [
    Bundled { name: "Inter", bytes: include_bytes!("../assets/fonts/Inter-Regular.ttf") },
    Bundled { name: "Cantarell", bytes: include_bytes!("../assets/fonts/Cantarell-VF.otf") },
    Bundled { name: "Comfortaa", bytes: include_bytes!("../assets/fonts/Comfortaa-Regular.otf") },
    Bundled { name: "Source Code Pro", bytes: include_bytes!("../assets/fonts/SourceCodePro-Regular.otf") },
    Bundled { name: "Rajdhani", bytes: include_bytes!("../assets/fonts/Rajdhani-Medium.ttf") },
    Bundled { name: "Press Start 2P", bytes: include_bytes!("../assets/fonts/PressStart2P-Regular.ttf") },
    Bundled { name: "VT323", bytes: include_bytes!("../assets/fonts/VT323-Regular.ttf") },
    Bundled { name: "Silkscreen", bytes: include_bytes!("../assets/fonts/Silkscreen-Regular.ttf") },
    Bundled { name: "Monoton", bytes: include_bytes!("../assets/fonts/Monoton-Regular.ttf") },
    Bundled { name: "Creepster", bytes: include_bytes!("../assets/fonts/Creepster-Regular.ttf") },
    Bundled { name: "Nosifer", bytes: include_bytes!("../assets/fonts/Nosifer-Regular.ttf") },
    Bundled { name: "Eater", bytes: include_bytes!("../assets/fonts/Eater-Regular.ttf") },
    Bundled { name: "UnifrakturMaguntia", bytes: include_bytes!("../assets/fonts/UnifrakturMaguntia-Book.ttf") },
    Bundled { name: "Cinzel", bytes: include_bytes!("../assets/fonts/Cinzel-VF.ttf") },
    Bundled { name: "Bangers", bytes: include_bytes!("../assets/fonts/Bangers-Regular.ttf") },
    Bundled { name: "Kreon", bytes: include_bytes!("../assets/fonts/Kreon-Regular.ttf") },
    Bundled { name: "Orbitron", bytes: include_bytes!("../assets/fonts/Orbitron-VF.ttf") },
    Bundled { name: "Titillium Web", bytes: include_bytes!("../assets/fonts/TitilliumWeb-Regular.ttf") },
    Bundled { name: "Special Elite", bytes: include_bytes!("../assets/fonts/SpecialElite-Regular.ttf") },
];

/// Names offered in the font dropdowns: the bundled fonts, then the ones the game in use carries.
pub fn names() -> Vec<String> {
    BUNDLED.iter().map(|b| b.name.to_string()).chain(pack_names()).collect()
}

/// The fonts of the game in use, by the name the pack gives them, and the family each is
/// drawn as.
static PACK: RwLock<BTreeMap<String, &'static str>> = RwLock::new(BTreeMap::new());

/// What the game in use's font assets were last set from, so setting them again is free.
static PACK_SOURCE: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

/// Each font file loaded so far, by a hash of its bytes: its family and its bytes. Loading one
/// twice would only add the same faces again.
static LOADED: Mutex<Option<HashMap<u64, Loaded>>> = Mutex::new(None);

type Loaded = (&'static str, std::sync::Arc<[u8]>);

/// Goes up each time a file is added to [`LOADED`], so a renderer knows when to load [`files`]
/// again.
static VERSION: AtomicU64 = AtomicU64::new(0);

/// The most a font file may be, in bytes.
pub const MAX_FONT_BYTES: usize = 1024 * 1024;

/// A font name that can go in `{font:…}`: letters, digits, spaces and `- _ .`.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
}

fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Loads a font file into this process's fonts, iced's included, and returns the family it
/// draws as. A file that holds no font is an error.
pub fn load(bytes: &[u8]) -> Result<&'static str, String> {
    register(bytes, true)
}

/// Like [`load`], without iced's font system, which it would start: for the overlay process,
/// whose own renderer takes the file from [`files`].
pub fn load_for_overlay(bytes: &[u8]) -> Result<&'static str, String> {
    register(bytes, false)
}

fn register(bytes: &[u8], iced: bool) -> Result<&'static str, String> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(format!("a font is at most {} KB", MAX_FONT_BYTES / 1024));
    }
    let key = hash(bytes);
    let mut loaded = LOADED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let loaded = loaded.get_or_insert_with(HashMap::new);
    if let Some((family, _)) = loaded.get(&key) {
        return Ok(family);
    }
    let family = family_of(bytes).ok_or("not a font file (TrueType or OpenType)")?;
    let family: &'static str = Box::leak(family.into_boxed_str());
    if iced {
        font_system().write().unwrap_or_else(std::sync::PoisonError::into_inner).load_font(bytes.to_vec().into());
    }
    loaded.insert(key, (family, bytes.into()));
    VERSION.fetch_add(1, Ordering::Release);
    Ok(family)
}

/// The family name a font file draws as, read without any renderer.
fn family_of(bytes: &[u8]) -> Option<String> {
    let mut db = fontdb::Database::new();
    db.load_font_data(bytes.to_vec());
    db.faces().find_map(|f| f.families.first().map(|(name, _)| name.clone()))
}

/// Every font file a renderer should load to draw what [`family`] names: the bundled fonts, then
/// the other files loaded so far (a game's). Each renderer loads them into its own font system.
pub fn files() -> Vec<Cow<'static, [u8]>> {
    // A bundled font is hashed only when a loaded file is as long: hashing reads every byte, and
    // so keeps all the bundled fonts in memory where drawing reads only a few of their pages.
    let bundled = |key: u64, bytes: &[u8]| BUNDLED.iter().any(|b| b.bytes.len() == bytes.len() && hash(b.bytes) == key);
    let loaded = LOADED.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut others: Vec<_> = loaded.iter().flatten().filter(|(key, (_, bytes))| !bundled(**key, bytes)).collect();
    others.sort_by_key(|(key, _)| **key);
    let others = others.into_iter().map(|(_, (_, bytes))| Cow::Owned(bytes.to_vec()));
    BUNDLED.iter().map(|b| Cow::Borrowed(b.bytes)).chain(others).collect()
}

/// Changes whenever [`files`] would list more: read it before [`files`], and load them again
/// once it moves on.
pub fn version() -> u64 {
    VERSION.load(Ordering::Acquire)
}

/// Makes these the fonts `{font:name}` and the font settings can name besides the others: the
/// game in use's, as base64. A font that doesn't load is left out.
pub fn set_pack_fonts(fonts: &BTreeMap<String, String>) {
    let mut source = PACK_SOURCE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if source.as_ref() == Some(fonts) {
        return;
    }
    let ready = fonts
        .iter()
        .filter_map(|(name, data)| Some((name.clone(), load(&crate::config::decode_base64(data)?).ok()?)))
        .collect();
    set_pack(ready);
    *source = Some(fonts.clone());
}

fn set_pack(ready: BTreeMap<String, &'static str>) {
    if let Ok(mut pack) = PACK.write() {
        *pack = ready;
    }
}

/// Writes the game's fonts as files in a folder the overlay can read, so frames carry their
/// paths rather than the fonts. Empty if there is no such folder.
pub fn share_pack_fonts(fonts: &BTreeMap<String, String>) -> BTreeMap<String, PathBuf> {
    let Some(dir) = shared_font_dir() else { return BTreeMap::new() };
    if fonts.is_empty() || std::fs::create_dir_all(&dir).is_err() {
        return BTreeMap::new();
    }
    fonts
        .iter()
        .filter_map(|(name, data)| {
            let bytes = crate::config::decode_base64(data)?;
            let path = dir.join(format!("{:016x}.font", hash(&bytes)));
            // A file cut short (the daemon ended while writing it) is written again.
            if std::fs::metadata(&path).map(|m| m.len()).ok() != Some(bytes.len() as u64) {
                std::fs::write(&path, &bytes).ok()?;
            }
            Some((name.clone(), path))
        })
        .collect()
}

/// Where the shared font files go: the runtime folder, or on Windows, which has none, the user's
/// local app data, which only they can open.
fn shared_font_dir() -> Option<PathBuf> {
    let dir = crate::ipc::shared_runtime_dir();
    #[cfg(windows)]
    let dir = dir.or_else(|| dirs::data_local_dir().map(|d| d.join("padwight")));
    dir.map(|d| d.join("padwight-fonts"))
}

/// Like [`set_pack_fonts`], from the files [`share_pack_fonts`] wrote: for the overlay process.
/// Every frame names them, so the files are read only when they change.
pub fn set_pack_font_files(files: &BTreeMap<String, PathBuf>) {
    static SOURCE: Mutex<Option<BTreeMap<String, PathBuf>>> = Mutex::new(None);
    let mut source = SOURCE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if source.as_ref() == Some(files) {
        return;
    }
    let ready = files.iter().filter_map(|(name, path)| Some((name.clone(), load_for_overlay(&std::fs::read(path).ok()?).ok()?))).collect();
    set_pack(ready);
    *source = Some(files.clone());
}

/// The names of the fonts the game in use carries, sorted.
pub fn pack_names() -> Vec<String> {
    PACK.read().map(|p| p.keys().cloned().collect()).unwrap_or_default()
}

/// The family a font setting draws as; `None` is the system default. A game's font gives its
/// file's own family, a bundled one its name, and any other name is taken as an installed family.
pub fn family(name: Option<&str>) -> Option<&'static str> {
    let name = name.map(str::trim).filter(|n| !n.is_empty())?;
    if let Some(family) = PACK.read().ok().and_then(|p| p.get(name).copied()) {
        return Some(family);
    }
    if let Some(b) = BUNDLED.iter().find(|b| b.name == name) {
        return Some(b.name);
    }
    // A `'static` name is handier; a pack can name any family, so keep each one only once.
    static OTHER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut known = OTHER.get_or_init(Mutex::default).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    Some(*known.entry(name.to_string()).or_insert_with(|| Box::leak(name.to_string().into_boxed_str())))
}

/// The iced font for a setting; `None` is the system default.
pub fn resolve(name: Option<&str>) -> Font {
    family(name).map_or(Font::DEFAULT, Font::with_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_is_the_default_and_names_resolve_to_themselves() {
        assert_eq!(resolve(None), Font::DEFAULT);
        assert_eq!(resolve(Some("  ")), Font::DEFAULT);
        assert_eq!(resolve(Some("Inter")), Font::with_name("Inter"));
        assert_eq!(resolve(Some("Fira Sans")), resolve(Some("Fira Sans")));
    }

    #[test]
    fn family_names_what_a_setting_draws_as() {
        assert_eq!(family(None), None);
        assert_eq!(family(Some("  ")), None);
        assert_eq!(family(Some(" Inter ")), Some("Inter"));
        assert_eq!(family(Some("Fira Sans")), Some("Fira Sans"));
        let bangers = BUNDLED.iter().find(|b| b.name == "Bangers").unwrap().bytes;
        let mut assets = crate::config::PackAssets::default();
        assets.add_font("Marquee", bangers).unwrap();
        assets.activate();
        assert_eq!(family(Some("Marquee")), Some("Bangers"));
    }

    #[test]
    fn the_loaded_files_are_the_bundled_ones_then_a_game_s() {
        assert_eq!(files().len(), BUNDLED.len());
        assert!(files().iter().zip(&BUNDLED).all(|(f, b)| f.as_ref() == b.bytes));
        load(BUNDLED[0].bytes).unwrap();
        assert_eq!(files().len(), BUNDLED.len(), "a bundled file loaded is not listed twice");
        let path = std::env::temp_dir().join(format!("padwight-font-list-{}.font", std::process::id()));
        let bytes = std::fs::read("assets/fonts/Kreon-Regular.ttf").unwrap();
        assert_eq!(family_of(&bytes).as_deref(), Some("Kreon"));
        assert!(family_of(b"hello").is_none());
        let mut odd = bytes.clone();
        odd.extend_from_slice(b"x");
        std::fs::write(&path, &odd).unwrap();
        set_pack_font_files(&BTreeMap::from([("Face".to_string(), path.clone())]));
        assert_eq!(files().len(), BUNDLED.len() + 1);
        assert_eq!(files().last().unwrap().as_ref(), odd.as_slice());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn every_bundled_font_draws_as_its_name() {
        for font in &BUNDLED {
            assert_eq!(load(font.bytes), Ok(font.name), "{}: use a file whose family is the name", font.name);
        }
    }

    #[test]
    fn a_pack_s_font_is_loaded_named_and_resolved() {
        let bangers = BUNDLED.iter().find(|b| b.name == "Bangers").unwrap().bytes;
        let mut assets = crate::config::PackAssets::default();
        assert!(assets.add_font("not a font", b"hello").is_err());
        assert!(assets.add_font("../x", bangers).is_err());
        assets.add_font("Marquee Face", bangers).unwrap();
        assets.check().unwrap();
        assert_eq!(resolve(Some("Marquee Face")), resolve(Some("Marquee Face")));
        assert!(!pack_names().contains(&"Marquee Face".to_string()), "not in use until activated");
        assets.activate();
        assert!(names().contains(&"Marquee Face".to_string()));
        assert_eq!(resolve(Some("Marquee Face")), Font::with_name("Bangers"), "drawn as the file's own family");
        assert!(valid_name("Press Start 2P") && !valid_name("a/b") && !valid_name(""));
    }

    #[test]
    fn the_overlay_reads_a_pack_s_font_files_once_not_every_frame() {
        let dir = std::env::temp_dir().join(format!("padwight-font-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("face.font");
        std::fs::write(&path, BUNDLED.iter().find(|b| b.name == "Kreon").unwrap().bytes).unwrap();
        let files = BTreeMap::from([("Face".to_string(), path.clone())]);
        set_pack_font_files(&files);
        assert_eq!(pack_names(), ["Face"]);
        // A frame naming the same files reads nothing: gone now, the font stays.
        std::fs::remove_file(&path).unwrap();
        set_pack_font_files(&files);
        assert_eq!(pack_names(), ["Face"]);
        set_pack_font_files(&BTreeMap::new());
        assert!(pack_names().is_empty());
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn a_shared_font_file_cut_short_is_written_again() {
        let runtime = std::env::temp_dir().join(format!("padwight-font-share-{}", std::process::id()));
        std::fs::create_dir_all(&runtime).unwrap();
        // SAFETY: nextest runs each test in a process of its own.
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", &runtime) };
        let bytes = BUNDLED.iter().find(|b| b.name == "Kreon").unwrap().bytes;
        let fonts = BTreeMap::from([("Face".to_string(), crate::config::encode_base64(bytes))]);
        let path = share_pack_fonts(&fonts)["Face"].clone();
        std::fs::write(&path, &bytes[..100]).unwrap();
        assert_eq!(share_pack_fonts(&fonts)["Face"], path);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let _ = std::fs::remove_dir_all(&runtime);
    }
}
