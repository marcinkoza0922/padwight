//! What each fuzz target feeds with its bytes (`fuzz/fuzz_targets/*.rs` call these). Each takes
//! anything and must neither panic nor hang; `engine_input` also asserts that releasing
//! everything leaves nothing held.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    config::{Button, ButtonAction, Config, Macro, MacroStep, MouseButton, Profile},
    engine::Engine,
    info::PadFamily,
    input::{Axis, InputEvent},
    ipc::{Request, Response},
    output::OutEvent,
    pack,
    tools::calculator::{Calculator, Press},
};

/// Lets panics inside the font parser pass: `font::family_of` catches them (a damaged font file
/// must not stop the app), but libfuzzer's own panic hook would abort on them first. Call from a
/// target's `init`, after libfuzzer has set its hook.
pub fn allow_caught_font_panics() {
    let libfuzzer_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !info.location().is_some_and(|at| at.file().contains("ttf-parser")) {
            libfuzzer_hook(info);
        }
    }));
}

/// A pack file: parse it, and if it reads, plan and apply the import and write it back out.
pub fn pack_parse(data: &[u8]) {
    let Ok(text) = std::str::from_utf8(data) else { return };
    let Ok(parsed) = pack::parse(text) else { return };
    let _ = parsed.to_toml();
    let _ = pack::features(&parsed.to_game());
    let mut config = Config::default();
    let plan = pack::plan(&config, parsed, false);
    let choices = pack::Choices { replace: false, keep_mine: vec![false; plan.rule_clashes.len()] };
    let _ = pack::apply(&mut config, &plan, &choices);
    // A second import of the same pack is an update or a reinstall.
    let again = pack::plan(&config, plan.pack.clone(), false);
    let choices = pack::Choices { replace: false, keep_mine: vec![false; again.rule_clashes.len()] };
    let _ = pack::apply(&mut config, &again, &choices);
}

/// A pack's icons, sounds and fonts: the first byte picks the loader.
pub fn pack_assets(data: &[u8]) {
    let Some((&kind, rest)) = data.split_first() else { return };
    let text = String::from_utf8_lossy(rest);
    match kind % 5 {
        0 => drop(crate::icons::check_svg(&text)),
        1 => drop(crate::sound::decode_wav(rest)),
        // The parser alone: `load_for_overlay` keeps every valid font for good, which would only
        // fill the fuzzer's memory.
        2 => drop(crate::font::family_of(rest)),
        3 => {
            // Whole assets as a pack holds them: base64 text under names, one per line.
            let mut assets = crate::config::PackAssets::default();
            for (i, line) in text.lines().enumerate() {
                let (name, body) = line.split_once(' ').unwrap_or(("n", line));
                let map = match i % 3 {
                    0 => &mut assets.icons,
                    1 => &mut assets.sounds,
                    _ => &mut assets.fonts,
                };
                map.insert(name.to_string(), body.to_string());
            }
            let _ = assets.check();
        }
        _ => drop(crate::config::decode_base64(&text)),
    }
}

/// A config folder, written out from `data` and loaded. The text is files, each started by a line
/// `#### relative/path`; text before the first such line is dropped.
pub fn config_load(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let root = work_dir("config");
    let mut files = 0;
    for part in text.split("#### ").skip(1) {
        let (path, body) = part.split_once('\n').unwrap_or((part, ""));
        let path = Path::new(path.trim());
        let plain = path.components().all(|c| matches!(c, std::path::Component::Normal(_)));
        files += 1;
        if !plain || files > 64 || path.as_os_str().is_empty() {
            continue;
        }
        let at = root.join(path);
        if let Some(parent) = at.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(at, body);
    }
    let file = root.join("config.toml");
    if let Ok(config) = Config::load_from(&file) {
        let again = root.join("again");
        let _ = std::fs::create_dir_all(&again);
        if config.save_to(&again.join("config.toml")).is_ok() {
            let _ = Config::load_from(&again.join("config.toml"));
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// The config folder `config_load` makes of `config`, as the text it reads.
pub fn config_tree_text(config: &Config) -> String {
    let root = work_dir("seed");
    let file = root.join("config.toml");
    let mut out = String::new();
    if config.save_to(&file).is_ok() {
        let mut paths = Vec::new();
        collect(&root, &mut paths);
        paths.sort();
        for path in paths {
            if let Ok(body) = std::fs::read_to_string(&path) {
                let name = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
                out.push_str(&format!("#### {name}\n{body}\n"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Writes the seed corpus that needs the program itself: config folders as `config_load` reads
/// them, one with the defaults and one with each of the first library packs imported.
pub fn write_config_seeds(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let mut config = Config::default();
    let _ = std::fs::write(dir.join("default"), config_tree_text(&config));
    for (i, entry) in crate::library::listings().iter().filter_map(|l| l.entry()).take(40).enumerate() {
        let plan = pack::plan(&config, entry.pack, true);
        let choices = pack::Choices { replace: false, keep_mine: vec![false; plan.rule_clashes.len()] };
        let _ = pack::apply(&mut config, &plan, &choices);
        if i % 8 == 7 {
            let _ = std::fs::write(dir.join(format!("with-{}-packs", i + 1)), config_tree_text(&config));
        }
    }
}

/// A folder of our own to write in, inside the fuzz folder (`PADWIGHT_FUZZ_WORK`, else the working directory).
fn work_dir(kind: &str) -> PathBuf {
    let base = std::env::var_os("PADWIGHT_FUZZ_WORK").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let dir = base.join(format!("{kind}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// A line of the daemon's socket, as a request and as a reply.
pub fn ipc_request(data: &[u8]) {
    if let Ok(req) = serde_json::from_slice::<Request>(data) {
        let _ = serde_json::to_string(&req);
    }
    if let Ok(reply) = serde_json::from_slice::<Response>(data) {
        let _ = serde_json::to_string(&reply);
    }
}

/// Text from a pack, in an info overlay or a hint: tokens and formatting tags.
pub fn tokens(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    let _ = crate::info::parse(&text);
    for family in PadFamily::ALL {
        let _ = crate::info::preview(&text, family);
        let _ = crate::info::glyph_text(&text, crate::info::Glyphs { family, nintendo_layout: false });
    }
}

/// Keys pressed on the calculator, one byte each.
pub fn calculator(data: &[u8]) {
    let mut calc = Calculator::default();
    for &b in data {
        let key = match b % 20 {
            n @ 0..=9 => Press::Digit(char::from(b'0' + n)),
            10 => Press::Op('+'),
            11 => Press::Op('-'),
            12 => Press::Op('*'),
            13 => Press::Op('/'),
            14 | 15 => Press::Dot,
            16 => Press::Backspace,
            17 => Press::Clear,
            _ => Press::Equals,
        };
        calc.press(key);
        let _ = calc.line();
        let _ = calc.answer();
    }
}

fn key(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

/// A profile that uses the kinds of action most likely to leave something held.
fn busy_profile() -> Profile {
    let mut p = Profile::pc_action("Busy");
    p.set_button(Button::South, ButtonAction::toggle(key("KEY_LEFTCTRL")));
    p.set_button(Button::East, ButtonAction::Turbo { action: Box::new(ButtonAction::Mouse(MouseButton::Left)), rate: 12.0, every_ms: 0 });
    p.set_button(Button::West, ButtonAction::toggle(ButtonAction::Turbo { action: Box::new(key("KEY_R")), rate: 8.0, every_ms: 0 }));
    p.set_button(Button::North, ButtonAction::Multi(vec![key("KEY_A"), ButtonAction::Mouse(MouseButton::Right)]));
    p.set_button(Button::LeftBumper, ButtonAction::Macro { name: "loop".into(), repeat: true });
    p.set_button(Button::RightBumper, ButtonAction::Macro { name: "once".into(), repeat: false });
    p
}

fn engine() -> Engine {
    let mut e = Engine::default();
    e.set_macros(&[
        Macro {
            name: "loop".into(),
            steps: vec![MacroStep::Press(key("KEY_LEFTSHIFT")), MacroStep::Tap { action: key("KEY_W"), hold_ms: 30 }, MacroStep::Wait(20)],
        },
        Macro {
            name: "once".into(),
            steps: vec![MacroStep::Press(key("KEY_LEFTALT")), MacroStep::Wait(200), MacroStep::Release(key("KEY_LEFTALT"))],
        },
    ]);
    e
}

fn identity(ev: &OutEvent) -> Option<(String, bool)> {
    match ev {
        OutEvent::Key(k, down) => Some((format!("key {k:?}"), *down)),
        OutEvent::PadButton(b, down) => Some((format!("pad {b:?}"), *down)),
        OutEvent::MouseButton(b, down) => Some((format!("mouse {b:?}"), *down)),
        _ => None,
    }
}

fn track(held: &mut BTreeSet<String>, out: &[OutEvent]) {
    for ev in out {
        if let Some((id, down)) = identity(ev) {
            if down {
                held.insert(id);
            } else {
                held.remove(&id);
            }
        }
    }
}

/// Input events and time steps into the engine, from the bytes: the first picks the profile,
/// then two per step. After the steps and `release_all`, nothing may be held.
pub fn engine_input(data: &[u8]) {
    let Some((&which, steps)) = data.split_first() else { return };
    let profiles = [Profile::passthrough("Pass"), Profile::desktop("Desk"), Profile::pc_action("Play"), busy_profile()];
    let profile = &profiles[usize::from(which) % profiles.len()];
    let mut e = engine();
    let mut held = BTreeSet::new();
    let mut now = Instant::now();
    for step in steps.chunks(2).take(2000) {
        let (op, arg) = (step[0], step.get(1).copied().unwrap_or(0));
        let unit = f32::from(arg) / 127.5 - 1.0;
        let mut out = Vec::new();
        match op % 4 {
            0 | 1 => {
                let button = Button::ALL[usize::from(arg >> 1) % Button::ALL.len()];
                e.handle(profile, InputEvent::Button(button, arg & 1 == 1), now, &mut out);
            }
            2 => {
                let axes = [Axis::LeftX, Axis::LeftY, Axis::RightX, Axis::RightY];
                e.handle(profile, InputEvent::Axis(axes[usize::from(op >> 2) % 4], unit), now, &mut out);
            }
            _ => {
                let axes = [Axis::LeftTrigger, Axis::RightTrigger];
                e.handle(profile, InputEvent::Axis(axes[usize::from(op >> 2) % 2], (unit + 1.0) / 2.0), now, &mut out);
            }
        }
        // Time passes between events: up to about 250 ms.
        now += Duration::from_millis(u64::from(op >> 3) * 8);
        e.timers(profile, now, &mut out);
        if e.needs_tick(profile) {
            e.tick(profile, 0.016, &mut out);
        }
        track(&mut held, &out);
        for ev in &out {
            if let OutEvent::PadAxis(axis, v) = ev {
                assert!(v.is_finite() && (-1.0..=1.0).contains(v), "{axis:?} at {v}");
            }
        }
    }
    let mut out = Vec::new();
    e.release_all(false, &mut out);
    track(&mut held, &out);
    assert!(held.is_empty(), "still held after release_all: {held:?}");
}
