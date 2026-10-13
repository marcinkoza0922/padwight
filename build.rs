//! Embeds the built-in game library (`packs/*.padpack`) and the built-in icons
//! (`assets/icons/*.svg`) in the binary.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    embed(&root.join("packs"), "padpack", "FILES", &out.join("library.rs"), true);
    embed(&root.join("assets/icons"), "svg", "BUILT_IN", &out.join("icons.rs"), false);
}

/// Writes `file` with a static `name` listing each `extension` file in `dir`, sorted, as
/// (name, contents): the file name with its extension when `keep_extension`, without it otherwise.
fn embed(dir: &Path, extension: &str, name: &str, file: &Path, keep_extension: bool) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == extension)).collect())
        .unwrap_or_default();
    files.sort();
    let mut out = format!("pub static {name}: &[(&str, &str)] = &[\n");
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let label = if keep_extension { path.file_name() } else { path.file_stem() }.unwrap().to_string_lossy();
        out.push_str(&format!("    ({label:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    out.push_str("];\n");
    fs::write(file, out).unwrap();
}
