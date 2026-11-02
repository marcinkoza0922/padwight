//! `padwight debug overlay baseline|diff` and `padwight debug gui baseline|diff`: picture
//! regression. `baseline <dir>` draws every overlay scene, the glyph sheet, a motion strip per
//! style and the settings window's tabs (light and dark) into `<dir>` with a manifest;
//! `diff <dir>` draws them again and lists what moved, worst first, with difference pictures.
//! See `docs/development/qa-tooling.md`, T-8.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use iced::Color;
use serde::{Deserialize, Serialize};

use super::{
    overlay_compare::{self as compare, Outcome},
    overlay_shot as shot, scenes,
};
use crate::{overlay, snapshot::Image};

pub const USAGE: &str = "
Picture regression (does a change move any picture it wasn't meant to?):
  overlay baseline <dir> [--only overlay,motion,gui]     (also `gui baseline <dir>`: gui only)
     writes every overlay scene and the glyph sheet (<dir>/overlay), a frame strip per motion
     style (<dir>/motion) and the settings window's tabs, light and dark (<dir>/gui), plus
     <dir>/manifest.json: the tool version, renderer, font hashes and every picture's size and hash
  overlay diff <dir> [--only ...] [--out <dir>] [--min-over <percent>] [--exact]
     draws them again (into <out>/new, default screenshots/regression) and compares with the
     baseline: pictures added or removed, changed fonts, and the pictures that moved, worst first,
     as baseline | new | difference in <out>/diff. A picture moved when some of its pixels differ
     by more than 8 levels (more than --min-over percent of them, default 0); --exact counts any
     difference at all. Exit status 1 when anything moved, was added or removed.
";

/// The groups of pictures, in the order they are drawn.
const SETS: [&str; 3] = ["overlay", "motion", "gui"];
/// Menu scenes (and the keyboard) played in every motion style.
const MOTION_SCENES: [&str; 2] = ["menu-list", "keyboard"];
const SIZE: (u32, u32) = (1920, 1080);
const BACKGROUND: [u8; 3] = [0x3c, 0x46, 0x50];

#[derive(Serialize, Deserialize)]
struct Entry {
    file: String,
    width: u32,
    height: u32,
    hash: String,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    tool: String,
    renderer: String,
    sets: Vec<String>,
    /// Bundled font name to a hash of its bytes.
    fonts: BTreeMap<String, String>,
    entries: Vec<Entry>,
}

struct Options {
    dir: PathBuf,
    out: PathBuf,
    only: Option<Vec<String>>,
    min_over: f32,
    exact: bool,
}

/// FNV-1a, 64 bits: stable across Rust versions, which `DefaultHasher` is not promised to be.
fn fnv(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    format!("{hash:016x}")
}

fn fonts() -> BTreeMap<String, String> {
    crate::font::BUNDLED.iter().map(|f| (f.name.to_string(), fnv(f.bytes))).collect()
}

/// `mode` is `baseline` or `diff`; `args` are what follows it. `only` limits the groups
/// (`gui baseline` passes `gui`).
pub fn run(mode: &str, args: &[&str], only: Option<&str>) -> Result<()> {
    let mut options = Options { dir: PathBuf::new(), out: PathBuf::from("screenshots/regression"), only: only.map(|o| vec![o.into()]), min_over: 0.0, exact: false };
    let mut dir = None;
    let mut it = args.iter().copied();
    while let Some(arg) = it.next() {
        let mut value = || it.next().with_context(|| format!("{arg} needs a value"));
        match arg {
            "--only" => options.only = Some(value()?.split(',').map(str::to_string).collect()),
            "--out" => options.out = PathBuf::from(value()?),
            "--min-over" => options.min_over = value()?.parse::<f32>()? / 100.0,
            "--exact" => options.exact = true,
            _ if arg.starts_with("--") => bail!("unknown option {arg}{USAGE}"),
            _ => dir = Some(arg),
        }
    }
    options.dir = PathBuf::from(dir.with_context(|| format!("{mode} needs a folder{USAGE}"))?);
    if let Some(bad) = options.only.iter().flatten().find(|o| !SETS.contains(&o.as_str())) {
        bail!("no group {bad:?} (overlay, motion, gui)");
    }
    if mode == "baseline" { baseline(&options) } else { diff(&options) }
}

#[expect(clippy::print_stdout, reason = "CLI output")]
fn baseline(options: &Options) -> Result<()> {
    let sets: Vec<String> = SETS.iter().filter(|s| wanted(options, s)).map(ToString::to_string).collect();
    let files = produce(&sets, &options.dir)?;
    let manifest = manifest_of(&sets, &options.dir, &files)?;
    std::fs::write(options.dir.join("manifest.json"), serde_json::to_string_pretty(&manifest)?)?;
    println!("baseline: {} pictures in {} ({})", manifest.entries.len(), options.dir.display(), sets.join(", "));
    Ok(())
}

fn wanted(options: &Options, set: &str) -> bool {
    options.only.as_ref().is_none_or(|o| o.iter().any(|s| s == set))
}

fn manifest_of(sets: &[String], dir: &Path, files: &[String]) -> Result<Manifest> {
    let mut entries = Vec::new();
    for file in files {
        let image = load(&dir.join(file))?;
        entries.push(Entry { file: file.clone(), width: image.width, height: image.height, hash: fnv(&image.rgba) });
    }
    Ok(Manifest { tool: format!("padwight {}", env!("CARGO_PKG_VERSION")), renderer: "iced tiny-skia (CPU)".into(), sets: sets.to_vec(), fonts: fonts(), entries })
}

/// Draws the pictures of `sets` into `dir`; the files written, relative to it.
fn produce(sets: &[String], dir: &Path) -> Result<Vec<String>> {
    // The clock readings in info overlays: a fixed moment (2026-01-02 03:04:05 UTC).
    crate::info::pin_time(1_767_323_045);
    let mut files = Vec::new();
    for set in sets {
        match set.as_str() {
            "overlay" => files.extend(overlay_pictures(dir)?),
            "motion" => files.extend(motion_pictures(dir)?),
            _ => files.extend(gui_pictures(dir)?),
        }
    }
    Ok(files)
}

fn save(dir: &Path, file: String, image: &Image) -> Result<String> {
    image.save(&dir.join(&file))?;
    Ok(file)
}

fn overlay_pictures(dir: &Path) -> Result<Vec<String>> {
    let (_, options) = shot::parse_in(&["all"], "")?;
    let [r, g, b] = BACKGROUND;
    let mut files = Vec::new();
    for (name, _) in scenes::SCENES {
        let image = overlay::snapshot(scenes::frame(name)?, iced::Size::new(SIZE.0 as f32, SIZE.1 as f32), 1.0, Color::from_rgb8(r, g, b))?;
        files.push(save(dir, format!("overlay/{name}.png"), &image)?);
    }
    files.push(save(dir, "overlay/glyphs.png".into(), &shot::glyph_sheets(&options)?)?);
    Ok(files)
}

fn motion_pictures(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for scene in MOTION_SCENES {
        for (name, image) in super::motion_shot::strips(scene)? {
            files.push(save(dir, format!("motion/{name}.png"), &image)?);
        }
    }
    Ok(files)
}

/// The window's pages and tabs on a library game with the manual's demo macro and info overlay.
const GUI_STEPS: &[&str] = &[
    "library", "Deus Ex", "game", "Deus Ex", "demo", "size", "1280x1500",
    "page", "overview", "shot", "overview", "game", "Deus Ex",
    "picture", "hide", "tab", "details", "shot", "details",
    "dialog", "add", "shot", "add-setup", "close",
    "tab", "profiles", "sub", "buttons", "shot", "buttons", "open", "South", "shot", "buttons-open",
    "sub", "sticks", "shot", "sticks", "sub", "combos", "shot", "combos", "sub", "gyro", "shot", "gyro",
    "sub", "touchpad", "shot", "touchpad", "sub", "guide", "shot", "guide",
    "tab", "modes", "shot", "modes", "tab", "layers", "shot", "layers",
    "tab", "macros", "macro", "0", "shot", "macros", "tab", "cycles", "shot", "cycles",
    "tab", "menus", "menu", "0", "shot", "menus", "tab", "info", "info", "0", "shot", "info", "tab", "logs", "shot", "logs",
    "page", "general", "shot", "general", "page", "manual", "shot", "manual",
    "page", "settings", "settings", "general", "shot", "settings-general", "settings", "shortcuts", "shot", "settings-shortcuts",
    "dialog", "system-demo", "shot", "system-check",
];

fn gui_pictures(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for theme in ["light", "dark"] {
        let out = dir.join("gui").join(theme);
        let out_arg = out.to_string_lossy().into_owned();
        let mut args = vec!["--out", out_arg.as_str()];
        if theme == "dark" {
            args.push("--dark");
        }
        args.extend_from_slice(GUI_STEPS);
        crate::gui::shoot::run(&args)?;
        let mut names: Vec<String> = std::fs::read_dir(&out)?.filter_map(|e| Some(e.ok()?.file_name().to_string_lossy().into_owned())).collect();
        names.sort();
        files.extend(names.into_iter().map(|n| format!("gui/{theme}/{n}")));
    }
    Ok(files)
}

fn load(path: &Path) -> Result<Image> {
    let file = std::fs::File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file)).read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut rgba)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        bail!("{} is not an 8-bit RGBA picture", path.display());
    }
    rgba.truncate(info.buffer_size());
    Ok(Image { width: info.width, height: info.height, rgba })
}

#[expect(clippy::print_stdout, reason = "CLI output")]
fn diff(options: &Options) -> Result<()> {
    let text = std::fs::read_to_string(options.dir.join("manifest.json")).with_context(|| format!("no baseline in {} (make one with `baseline`)", options.dir.display()))?;
    let old: Manifest = serde_json::from_str(&text).context("manifest.json is not a baseline manifest")?;
    let sets: Vec<String> = old.sets.iter().filter(|s| wanted(options, s)).cloned().collect();
    let fresh_dir = options.out.join("new");
    let diff_dir = options.out.join("diff");
    // Old pictures left from an earlier run would be mistaken for this one's.
    for dir in [&fresh_dir, &diff_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
    let files = produce(&sets, &fresh_dir)?;
    let new = manifest_of(&sets, &fresh_dir, &files)?;

    let in_sets = |file: &str| sets.iter().any(|s| file.starts_with(&format!("{s}/")));
    let mut problems = 0;
    if old.renderer != new.renderer || old.tool != new.tool {
        println!("tool: baseline {} / {}, now {} / {}", old.tool, old.renderer, new.tool, new.renderer);
    }
    for (name, hash) in &new.fonts {
        match old.fonts.get(name) {
            Some(h) if h == hash => {}
            Some(_) => println!("font changed: {name}"),
            None => println!("font added: {name}"),
        }
    }
    for name in old.fonts.keys().filter(|n| !new.fonts.contains_key(*n)) {
        println!("font removed: {name}");
    }
    let known: Vec<&str> = old.entries.iter().map(|e| e.file.as_str()).collect();
    let added: Vec<&str> = new.entries.iter().map(|e| e.file.as_str()).filter(|f| !known.contains(f)).collect();
    let removed: Vec<&str> = known.iter().copied().filter(|f| in_sets(f) && !files.iter().any(|n| n == f)).collect();

    let (mut results, mut same, mut faint) = (Vec::new(), 0, 0);
    for entry in old.entries.iter().filter(|e| files.contains(&e.file)) {
        let name = entry.file.trim_end_matches(".png").to_string();
        let new_entry = new.entries.iter().find(|e| e.file == entry.file).context("picture vanished")?;
        if new_entry.hash == entry.hash && (new_entry.width, new_entry.height) == (entry.width, entry.height) {
            same += 1;
            continue;
        }
        let (before, after) = (load(&options.dir.join(&entry.file))?, load(&fresh_dir.join(&entry.file))?);
        let path = diff_dir.join(&entry.file);
        match compare::score(&before, &after) {
            None => {
                compare::side_by_side(&[&before, &after]).save(&path)?;
                results.push((name, Outcome::SizeMismatch((before.width, before.height), (after.width, after.height), path)));
            }
            Some(s) if s.over > options.min_over || (options.exact && s.max > 0) => {
                compare::side_by_side(&[&before, &after, &compare::difference(&before, &after)]).save(&path)?;
                results.push((name, Outcome::Compared(s, path)));
            }
            Some(_) => faint += 1,
        }
    }
    compare::worst_first(&mut results);
    if !results.is_empty() {
        println!("{:<34}{:>8}{:>8}{:>6}", "picture", "over", "mean", "max");
    }
    for (name, outcome) in &results {
        match outcome {
            Outcome::Compared(s, path) => println!("{name:<34}{:>7.2}%{:>8.2}{:>6}  {}", s.over * 100.0, s.mean, s.max, path.display()),
            Outcome::SizeMismatch(a, b, path) => println!("{name:<34}size {}x{} -> {}x{}  {}", a.0, a.1, b.0, b.1, path.display()),
            Outcome::NotPorted => {}
        }
    }
    if !added.is_empty() {
        println!("added ({}): {}", added.len(), added.join(" "));
    }
    if !removed.is_empty() {
        println!("removed ({}): {}", removed.len(), removed.join(" "));
    }
    problems += results.len() + added.len() + removed.len();
    println!("{} moved, {} added, {} removed, {faint} differ below the threshold, {same} identical", results.len(), added.len(), removed.len());
    if problems > 0 {
        bail!("{problems} pictures differ from the baseline in {}", options.dir.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_matches_its_published_values() {
        assert_eq!(fnv(b""), "cbf29ce484222325");
        assert_eq!(fnv(b"a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn a_picture_survives_saving_and_loading() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("screenshots/regression-test");
        let image = Image { width: 2, height: 1, rgba: vec![1, 2, 3, 255, 4, 5, 6, 200] };
        image.save(&dir.join("x.png")).unwrap();
        let back = load(&dir.join("x.png")).unwrap();
        assert_eq!((back.width, back.height, back.rgba), (2, 1, image.rgba));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn options_need_a_folder_and_known_groups() {
        assert!(run("diff", &[], None).is_err());
        assert!(run("baseline", &["x", "--only", "nothing"], None).is_err());
    }
}
