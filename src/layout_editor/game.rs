//! The profile's and the game's own pages in Edit Controls. The profile: its name, what a
//! controller needs to play it, and its timings. The game: its name, the rules that pick it for
//! a window, its profiles, its button layout, font and looks, and (in `feel.rs`) its sounds,
//! rumble and motion.

use crate::{
    config::{Config, Feature, Rule, RuleKind, unique_name},
    menu::Tone,
    system_menu::active_profile_mut,
};

use super::{
    owner::game_mut,
    pages::Page,
    rows::{Act, Nav, Number, Row, cycle, set},
    style::{Look, StyleAt},
};

const TAP_WINDOW: Number = Number::new(10.0, (100.0, 600.0), 0, " ms");
const LONG_PRESS: Number = Number::new(50.0, (200.0, 1500.0), 0, " ms");
const COMBO_WINDOW: Number = Number::new(5.0, (20.0, 300.0), 0, " ms");

/// The profile being edited's game, as a rule or default names it.
fn game_key(config: &Config) -> Option<String> {
    config.active_ref().game
}

/// The profile's page: its name, what it needs, and how long its gestures and combos wait.
pub(super) fn profile_rows(config: &Config) -> Vec<Row> {
    let Some(p) = config.active() else { return Vec::new() };
    let name = p.name.clone();
    let mut rows = vec![Row::text(format!("Name: {name}"), "Name of the profile", name.clone(), move |c, typed| {
        let game = game_key(c);
        c.rename_profile(game.as_deref(), &name, typed.trim())
    })];
    for f in Feature::ALL {
        rows.push(Row::toggle(&format!("Needs {}", f.label()), p.requires.contains(&f), move |c, on| {
            let Some(p) = active_profile_mut(c) else { return false };
            let had = p.requires.contains(&f);
            match (had, on) {
                (false, true) => p.requires.push(f),
                (true, false) => p.requires.retain(|x| *x != f),
                _ => return false,
            }
            true
        }));
    }
    rows.push(TAP_WINDOW.row_u("Taps count together within", p.tap_window_ms, |c, v| active_profile_mut(c).is_some_and(|p| set(&mut p.tap_window_ms, v))));
    rows.push(LONG_PRESS.row_u("Long press after", p.long_press_ms, |c, v| active_profile_mut(c).is_some_and(|p| set(&mut p.long_press_ms, v))));
    rows.push(COMBO_WINDOW.row_u("Combo buttons wait for each other", p.combo_window_ms, |c, v| active_profile_mut(c).is_some_and(|p| set(&mut p.combo_window_ms, v))));
    rows
}

/// The game's page. General has no name or rules of its own.
pub(super) fn game_rows(config: &Config) -> Vec<Row> {
    let game = config.active_game();
    let key = game_key(config);
    let mut rows = Vec::new();
    if let Some(old) = key.clone() {
        rows.push(Row::text(format!("Name: {old}"), "Name of the setup", old.clone(), move |c, typed| c.rename_game(&old, typed.trim())));
        rows.push(Row::open(format!("Window rules ({})", game.rules.len()), Page::Rules));
    }
    rows.push(Row::open(format!("Profiles ({})", game.profiles.len()), Page::GameProfiles));
    let own = game.nintendo_layout;
    let label = match own {
        None => format!("as the app's ({})", if config.nintendo_layout { "on" } else { "off" }),
        Some(true) => "on".into(),
        Some(false) => "off".into(),
    };
    rows.push(Row::step(format!("Nintendo button layout: {label}"), move |c, dir| {
        game_mut(c).is_some_and(|g| set(&mut g.nintendo_layout, cycle(&[None, Some(true), Some(false)], own, dir)))
    }));
    rows.push(Row::open(format!("Font: {}", game.overlay_font.as_deref().unwrap_or("the app's")), Page::GameFont));
    for look in Look::ALL {
        let own = match look {
            Look::Keyboard => game.keyboard_style.is_some(),
            Look::Numpad => game.numpad_style.is_some(),
            Look::Media => game.media_style.is_some(),
            Look::Menu => game.menu_style.is_some(),
        };
        rows.push(Row::open(format!("{} look: {}", look.name(), if own { "its own" } else { "the app's" }), Page::Style(StyleAt::Game(look))));
    }
    rows.push(Row::open("Sounds…", Page::Sounds));
    rows.push(Row::open("Rumble…", Page::Rumble));
    rows.push(Row::open("Motion…", Page::Motion));
    rows
}

/// The fonts a game's overlays can be set in: the app's, or any font the app knows.
pub(super) fn font_rows(config: &Config) -> Vec<Row> {
    let now = config.active_game().overlay_font.clone();
    let pick = |font: Option<String>| move |c: &mut Config| Act::changed_if(game_mut(c).is_some_and(|g| set(&mut g.overlay_font, font.clone()))).then(Nav::Back(1));
    let tick = |on: bool| if on { "✓ " } else { "" };
    let mut rows = vec![Row::apply(format!("{}The app's", tick(now.is_none())), pick(None))];
    rows.extend(crate::font::names().into_iter().map(|name| Row::apply(format!("{}{name}", tick(now.as_deref() == Some(name.as_str()))), pick(Some(name)))));
    rows
}

fn rule_mut(config: &mut Config, i: usize) -> Option<&mut Rule> {
    game_mut(config)?.rules.get_mut(i)
}

/// What a rule matches, written out.
fn rule_label(r: &Rule) -> String {
    let off = if r.enabled { "" } else { " (off)" };
    format!("{}: {} → {}{off}", r.kind, if r.value.is_empty() { "(nothing yet)" } else { &r.value }, r.profile)
}

/// The game's window rules, and a row to add one for the profile being edited.
pub(super) fn rules_rows(config: &Config) -> Vec<Row> {
    let game = config.active_game();
    let mut rows: Vec<Row> = game.rules.iter().enumerate().map(|(i, r)| Row::open(rule_label(r), Page::Rule(i))).collect();
    let profile = config.active_ref().profile;
    rows.push(
        Row::text("Add a rule for a program", "The program's file name, such as game.exe", "", move |c, typed| {
            let typed = typed.trim();
            !typed.is_empty() && game_mut(c).is_some_and(|g| {
                g.rules.push(Rule::new(RuleKind::Executable, typed, profile.as_str()));
                true
            })
        })
        .tone(Tone::Add),
    );
    rows
}

/// A rule's page: what kind of thing it matches, the value, the profile it picks, on or off.
pub(super) fn rule_rows(config: &Config, i: usize) -> Vec<Row> {
    let game = config.active_game();
    let Some(r) = game.rules.get(i) else { return Vec::new() };
    let (kind, profile) = (r.kind, r.profile.clone());
    let names: Vec<String> = game.profiles.iter().map(|p| p.name.clone()).collect();
    vec![
        Row::step(format!("Matches: {kind}"), move |c, dir| rule_mut(c, i).is_some_and(|r| set(&mut r.kind, cycle(&RuleKind::ALL, kind, dir)))),
        Row::text(format!("Value: {}", r.value), format!("{kind} to match"), r.value.clone(), move |c, typed| {
            let typed = typed.trim();
            !typed.is_empty() && rule_mut(c, i).is_some_and(|r| set(&mut r.value, typed.to_string()))
        }),
        Row::step(format!("Switches to: {profile}"), move |c, dir| {
            let at = names.iter().position(|n| *n == profile).map_or(0, |p| (p as i32 + dir).rem_euclid(names.len() as i32) as usize);
            let Some(next) = names.get(at).cloned() else { return false };
            rule_mut(c, i).is_some_and(|r| set(&mut r.profile, next))
        }),
        Row::toggle("On", r.enabled, move |c, on| rule_mut(c, i).is_some_and(|r| set(&mut r.enabled, on))),
        Row::delete("Delete rule", move |c| {
            let Some(game) = game_mut(c).filter(|g| i < g.rules.len()) else { return false };
            game.rules.remove(i);
            true
        }),
    ]
}

/// The game's profiles, the one being edited marked.
pub(super) fn profiles_rows(config: &Config) -> Vec<Row> {
    let at = config.active_ref().profile;
    config
        .active_game()
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| Row::open(format!("{}{}", p.name, if p.name == at { " (current)" } else { "" }), Page::GameProfile(i)))
        .collect()
}

/// One of the game's profiles: renaming, copying, moving or deleting it. The last one can't go.
pub(super) fn game_profile_rows(config: &Config, i: usize) -> Vec<Row> {
    let game = config.active_game();
    let Some(p) = game.profiles.get(i) else { return Vec::new() };
    let name = p.name.clone();
    let len = game.profiles.len();
    let old = name.clone();
    let mut rows = vec![
        Row::text(format!("Name: {name}"), "Name of the profile", name.clone(), move |c, typed| {
            let game = game_key(c);
            c.rename_profile(game.as_deref(), &old, typed.trim())
        }),
        Row::add("Make a copy", move |c| {
            let Some(game) = game_mut(c) else { return Act::stay() };
            let Some(mut copy) = game.profiles.get(i).cloned() else { return Act::stay() };
            copy.name = unique_name(&copy.name, |n| game.profiles.iter().any(|p| p.name == n));
            game.profiles.insert(i + 1, copy);
            Act::changed().then(Nav::Follow(Page::GameProfile(i + 1), i + 1))
        }),
    ];
    for (label, to) in [("Move up", i.checked_sub(1)), ("Move down", (i + 1 < len).then_some(i + 1))] {
        let Some(to) = to else { continue };
        rows.push(Row::apply(label, move |c| {
            let Some(game) = game_mut(c) else { return Act::stay() };
            game.profiles.swap(i, to);
            Act::changed().then(Nav::Follow(Page::GameProfile(to), to))
        }));
    }
    if len > 1 {
        rows.push(Row::delete("Delete profile", move |c| delete_profile(c, i)));
    }
    rows
}

/// Deletes profile `i` of the game; the next one (or the one before) takes its place in use and
/// in its rules.
fn delete_profile(config: &mut Config, i: usize) -> bool {
    let at = config.active_ref();
    let Some(game) = game_mut(config).filter(|g| g.profiles.len() > 1 && i < g.profiles.len()) else { return false };
    let gone = game.profiles.remove(i).name;
    let next = game.profiles[i.min(game.profiles.len() - 1)].name.clone();
    // Its rules pick the profile that takes its place.
    for rule in game.rules.iter_mut().filter(|r| r.profile == gone) {
        rule.profile = next.clone();
    }
    if at.profile == gone {
        config.active.profile = next;
    }
    true
}
