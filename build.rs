//! Embeds the built-in game library (`packs/*.padpack`) and the built-in icons
//! (`assets/icons/*.svg`) in the binary, with each library pack's listing read here, so the
//! program can list thousands of packs without parsing them.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let packs = files(&root.join("packs"), "padpack");
    let mut library = embed(&packs, "FILES", true);
    library.push_str(&index(&packs));
    fs::write(out.join("library.rs"), library).unwrap();
    fs::write(out.join("icons.rs"), embed(&files(&root.join("assets/icons"), "svg"), "BUILT_IN", false)).unwrap();
}

/// The `extension` files in `dir`, sorted.
fn files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == extension)).collect())
        .unwrap_or_default();
    files.sort();
    files
}

/// A static `name` listing each of `files` as (name, contents): the file name with its
/// extension when `keep_extension`, without it otherwise.
fn embed(files: &[PathBuf], name: &str, keep_extension: bool) -> String {
    let mut out = format!("pub static {name}: &[(&str, &str)] = &[\n");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let label = if keep_extension { path.file_name() } else { path.file_stem() }.unwrap().to_string_lossy();
        out.push_str(&format!("    ({label:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    out.push_str("];\n");
    out
}

/// `INDEX`: each pack's listing (`library::Indexed`), sorted by name. A pack this can't read is
/// left out with a warning; the library's tests say what's wrong with it.
fn index(packs: &[PathBuf]) -> String {
    let mut listed: Vec<(String, String)> = Vec::new();
    for (at, path) in packs.iter().enumerate() {
        let text = fs::read_to_string(path).unwrap();
        match listing(at, &text) {
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

/// The pack's lowercase name, to sort by, and its `Indexed` as code. `at` is its place in `FILES`.
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
    let code = format!(
        "    super::Indexed {{ at: {at}, id: {id:?}, name: {name:?}, version: {version:?}, rules: &[{rules}], requires: &[{requires}] }},\n"
    );
    Ok((name.to_lowercase(), code))
}

/// The enum variant a snake_case name in a pack stands for, if it's one of `known`.
fn variant(name: &str, known: &[&str]) -> Result<String, String> {
    if !known.contains(&name) {
        return Err(format!("unknown {name:?}"));
    }
    Ok(name.split('_').map(|word| word[..1].to_uppercase() + &word[1..]).collect())
}
