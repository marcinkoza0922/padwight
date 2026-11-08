//! Compresses the built-in game library (`packs/*.padpack`) and the built-in icons, fonts and
//! sounds (`assets/`) into archives the binary embeds (see `src/archive.rs`), with each library
//! pack's listing read here, so the program can list thousands of packs without unpacking them.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// How hard to compress: the packs are many and alike, and a dictionary does most of the work.
const LEVEL: i32 = 12;
/// The size of the dictionary the packs share, and how many packs it is trained on.
const DICT_BYTES: usize = 256 * 1024;
const DICT_SAMPLES: usize = 1500;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let packs = files(&root.join("packs"), "padpack");
    let texts: Vec<String> = packs.iter().map(|p| fs::read_to_string(p).unwrap()).collect();
    let named: Vec<(String, Vec<u8>)> = packs.iter().zip(&texts).map(|(p, t)| (name(p), t.clone().into_bytes())).collect();

    let dict = train(&named);
    let mut library = archive(&out, "library", &named, &dict);
    library.push_str(&index(&texts, &packs));
    fs::write(out.join("library.rs"), library).unwrap();

    let mut assets = Vec::new();
    for dir in ["icons", "fonts", "sounds"] {
        for path in files_in(&root.join("assets").join(dir)) {
            assets.push((format!("{dir}/{}", name(&path)), fs::read(&path).unwrap()));
        }
    }
    fs::write(out.join("assets.rs"), archive(&out, "assets", &assets, &[])).unwrap();
    commit(&root);
}

/// The file's name.
fn name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// A dictionary for the library packs: they share most of their text. Empty if there are too
/// few to train on.
fn train(packs: &[(String, Vec<u8>)]) -> Vec<u8> {
    let step = packs.len().div_ceil(DICT_SAMPLES).max(1);
    let samples: Vec<&[u8]> = packs.iter().step_by(step).map(|(_, bytes)| bytes.as_slice()).collect();
    zstd::dict::from_samples(&samples, DICT_BYTES).unwrap_or_default()
}

/// Packs each of `files` (name, contents) on its own, writes the blob and the dictionary to
/// `out/<label>.bin` and `out/<label>.dict`, and returns the code that names them and says where
/// each file is: `BLOB`, `DICT` and `ITEMS`.
fn archive(out: &Path, label: &str, files: &[(String, Vec<u8>)], dict: &[u8]) -> String {
    let mut compressor = if dict.is_empty() { zstd::bulk::Compressor::new(LEVEL) } else { zstd::bulk::Compressor::with_dictionary(LEVEL, dict) }.unwrap();
    let mut blob = Vec::new();
    let mut items = String::new();
    for (name, bytes) in files {
        let packed = compressor.compress(bytes).unwrap();
        items.push_str(&format!("    Item {{ name: {name:?}, at: {}, packed: {}, raw: {} }},\n", blob.len(), packed.len(), bytes.len()));
        blob.extend(packed);
    }
    let (blob_path, dict_path) = (out.join(format!("{label}.bin")), out.join(format!("{label}.dict")));
    fs::write(&blob_path, blob).unwrap();
    fs::write(&dict_path, dict).unwrap();
    format!(
        "pub static BLOB: &[u8] = include_bytes!({:?});\npub static DICT: &[u8] = include_bytes!({:?});\npub static ITEMS: &[Item] = &[\n{items}];\n",
        blob_path.display().to_string(),
        dict_path.display().to_string()
    )
}

/// Hands the commit this build is of to the program as `PADWIGHT_COMMIT`, for bug reports. A build
/// outside a checkout (a source tarball) has none.
fn commit(root: &Path) {
    let run = |args: &[&str]| {
        let out = std::process::Command::new("git").args(args).current_dir(root).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned()).filter(|s| !s.is_empty())
    };
    if let Some(head) = run(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={}", root.join(head).display());
    }
    let hash = run(&["rev-parse", "--short=10", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=PADWIGHT_COMMIT={hash}");
}

/// The `extension` files in `dir`, sorted.
fn files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    files_in(dir).into_iter().filter(|p| p.extension().is_some_and(|x| x == extension)).collect()
}

/// Every file in `dir`, sorted. Each is watched, so changing one rebuilds.
fn files_in(dir: &Path) -> Vec<PathBuf> {
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = fs::read_dir(dir).map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()).unwrap_or_default();
    files.sort();
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    files
}

/// `INDEX`: each pack's listing (`library::Indexed`), sorted by name. A pack this can't read is
/// left out with a warning; the library's tests say what's wrong with it.
fn index(texts: &[String], packs: &[PathBuf]) -> String {
    let mut listed: Vec<(String, String)> = Vec::new();
    for (at, (path, text)) in packs.iter().zip(texts).enumerate() {
        match listing(at, text) {
            Ok(found) => listed.push(found),
            Err(why) => println!("cargo:warning={}: {why}", path.display()),
        }
    }
    listed.sort();
    let mut out = String::from("use crate::config::{Feature, RuleKind};\n\npub static INDEX: &[super::Indexed] = &[\n");
    for (_, code) in listed {
        out.push_str(&code);
    }
    out.push_str("];\n");
    out
}

/// The pack's lowercase name, to sort by, and its `Indexed` as code. `at` is its place in `ITEMS`.
fn listing(at: usize, text: &str) -> Result<(String, String), String> {
    let table: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    let header = table.get("pack").and_then(toml::Value::as_table).ok_or("no [pack] table")?;
    let field = |key: &str| header.get(key).and_then(toml::Value::as_str);
    let id = field("id").ok_or("no pack id")?;
    let name = field("name").ok_or("no pack name")?;
    let version = field("version").unwrap_or_default();

    let tables = |key: &str| -> Vec<toml::Table> {
        table.get(key).and_then(toml::Value::as_array).map(|a| a.iter().filter_map(|v| v.as_table().cloned()).collect()).unwrap_or_default()
    };
    let mut rules = String::new();
    for rule in tables("rules") {
        let text = |key: &str| rule.get(key).and_then(toml::Value::as_str).ok_or(format!("a rule without {key}"));
        let kind = variant(text("kind")?, &["executable", "steam_app_id", "window_class"])?;
        let enabled = rule.get("enabled").and_then(toml::Value::as_bool).unwrap_or(true);
        rules.push_str(&format!("(RuleKind::{kind}, {:?}, {:?}, {enabled}), ", text("value")?, text("profile")?));
    }
    let mut requires = String::new();
    for profile in tables("profiles") {
        let needs = profile.get("requires").and_then(toml::Value::as_array).cloned().unwrap_or_default();
        let needs: Vec<String> = needs
            .iter()
            .map(|f| f.as_str().ok_or("a requires that isn't a name".to_string()).and_then(|f| variant(f, &["gyro", "back_paddles", "touchpad"])))
            .map(|f| f.map(|f| format!("Feature::{f}")))
            .collect::<Result<_, _>>()?;
        requires.push_str(&format!("&[{}], ", needs.join(", ")));
    }
    let tested = header.get("tested").and_then(toml::Value::as_bool).unwrap_or(false);
    let sends = ["profiles", "modes", "layers", "menus", "macros", "cycles"].iter().any(|key| tables(key).iter().any(sends_keys_or_mouse));
    let unverified = !tested && sends;
    let code = format!(
        "    super::Indexed {{ at: {at}, id: {id:?}, name: {name:?}, version: {version:?}, rules: &[{rules}], requires: &[{requires}], unverified: {unverified} }},\n"
    );
    Ok((name.to_lowercase(), code))
}

/// `Pack::sends_keys_or_mouse` on the TOML: a `keys`, `mouse` or `wheel` action anywhere but
/// gyro, or a stick that moves the mouse, scrolls, flicks or presses keys by region.
fn sends_keys_or_mouse(table: &toml::Table) -> bool {
    table.iter().any(|(key, value)| match key.as_str() {
        "gyro" => false,
        "keys" | "mouse" | "wheel" => true,
        "left_stick" | "right_stick" => {
            let action = value.get("action");
            let pc = |a: &toml::Table| a.keys().any(|k| ["mouse", "scroll", "flick", "region"].contains(&k.as_str()));
            action.and_then(toml::Value::as_table).is_some_and(pc) || value_sends(value)
        }
        _ => value_sends(value),
    })
}

fn value_sends(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(t) => sends_keys_or_mouse(t),
        toml::Value::Array(a) => a.iter().any(value_sends),
        _ => false,
    }
}

/// The enum variant a snake_case name in a pack stands for, if it's one of `known`.
fn variant(name: &str, known: &[&str]) -> Result<String, String> {
    if !known.contains(&name) {
        return Err(format!("unknown {name:?}"));
    }
    Ok(name.split('_').map(|word| word[..1].to_uppercase() + &word[1..]).collect())
}
