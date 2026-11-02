//! The window states that are checked: every page, every tab of a rich library setup, dialogs.

use std::path::Path;

use anyhow::Result;
use iced::{Rectangle, Size};

use super::{Finding, scan::scan};
use crate::gui::shoot;

/// A name and the `debug gui` steps that get the window there.
pub struct Scene {
    pub name: String,
    pub steps: Vec<String>,
}

/// A library setup with menus, layers and macros, shown for the game tabs.
const GAME: &str = "Age of Empires II: Definitive Edition";

fn scene(name: &str, steps: &[&str]) -> Scene {
    Scene { name: name.into(), steps: steps.iter().map(|s| s.to_string()).collect() }
}

/// The steps that show `GAME` with a macro and an info overlay added, then `more`.
fn game(more: &[&str]) -> Vec<String> {
    ["library", GAME, "game", GAME, "demo"].iter().chain(more).map(|s| s.to_string()).collect()
}

pub fn all() -> Vec<Scene> {
    let mut scenes = vec![
        scene("overview", &["page", "overview"]),
        scene("overview-steam-warning", &["readers-demo", "page", "overview"]),
        scene("app-settings-general", &["page", "settings", "settings", "general"]),
        scene("app-settings-shortcuts", &["shortcuts-demo", "page", "settings", "settings", "shortcuts"]),
        scene("manual", &["page", "manual"]),
        scene("general", &["page", "general"]),
        scene("dialog-add-game", &["page", "overview", "dialog", "add"]),
        scene("dialog-export", &["page", "general", "dialog", "export"]),
        scene("dialog-system-check", &["dialog", "system-demo"]),
    ];
    for sub in ["buttons", "sticks", "combos", "gyro", "touchpad", "guide"] {
        let more = ["tab", "profiles", "sub", sub];
        scenes.push(Scene { name: format!("profile-{sub}"), steps: game(&more) });
    }
    scenes.push(Scene { name: "profile-buttons-expanded".into(), steps: game(&["tab", "profiles", "sub", "buttons", "expand"]) });
    for tab in ["modes", "layers", "macros", "cycles", "menus", "info", "logs", "details"] {
        scenes.push(Scene { name: format!("tab-{tab}"), steps: game(&["tab", tab]) });
    }
    let opened: [(&str, &[&str]); 5] = [
        ("macro-open", &["tab", "macros", "macro", "0"]),
        ("menu-open", &["tab", "menus", "menu", "0"]),
        ("menu-appearance", &["tab", "menus", "menu", "0", "appearance", "0"]),
        ("menu-filters", &["tab", "menus", "menu", "0", "appearance", "0", "filters-demo", "0"]),
        ("info-open", &["tab", "info", "info", "0"]),
    ];
    for (name, more) in opened {
        scenes.push(Scene { name: name.into(), steps: game(more) });
    }
    for (name, more) in [("action-picker", ["actions", "South"]), ("output-picker", ["outputs", "South"])] {
        let mut steps = game(&["tab", "profiles", "sub", "buttons"]);
        steps.extend(more.map(String::from));
        scenes.push(Scene { name: name.into(), steps });
    }
    for target in ["menu:0", "info:0", "keyboard", "numpad", "media"] {
        scenes.push(Scene { name: format!("stage-{}", target.replace(':', "-")), steps: game(&["stage", target]) });
    }
    scenes
}

/// Runs `scene` in a fresh window, checks it, and saves a picture with the findings boxed when
/// there are any.
pub fn checked(scene: &Scene, dark: bool, theme: &'static str, out: &Path, size: Size) -> Result<Vec<Finding>> {
    let mut app = shoot::new_app(dark);
    let mut options = shoot::Options { out: out.to_path_buf(), size, dark };
    let mut steps = scene.steps.iter().map(String::as_str);
    while let Some(step) = steps.next() {
        shoot::run_step(&mut app, &mut options, step, &mut steps)?;
    }
    app.saved = app.config.clone();
    let (found, mut image) = scan(&app, &scene.name, theme, size)?;
    if !found.is_empty() {
        for f in &found {
            outline(&mut image, f.bounds);
        }
        image.save(&out.join(format!("{}-{theme}.png", scene.name)))?;
    }
    Ok(found)
}

/// A red box around `r`.
fn outline(image: &mut crate::snapshot::Image, r: Rectangle) {
    let (x0, y0) = ((r.x as i64 - 2).max(0) as u32, (r.y as i64 - 2).max(0) as u32);
    let (x1, y1) = (((r.x + r.width) as u32 + 2).min(image.width - 1), ((r.y + r.height) as u32 + 2).min(image.height - 1));
    for x in x0..=x1 {
        for y in [y0, y1] {
            set(image, x, y);
        }
    }
    for y in y0..=y1 {
        for x in [x0, x1] {
            set(image, x, y);
        }
    }
}

fn set(image: &mut crate::snapshot::Image, x: u32, y: u32) {
    let i = 4 * (y * image.width + x) as usize;
    if let Some(p) = image.rgba.get_mut(i..i + 4) {
        p.copy_from_slice(&[0xff, 0x00, 0x30, 0xff]);
    }
}
