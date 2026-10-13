//! Packs: a game written to a `.padpack` file (TOML, in the config's own shapes), and what
//! importing, updating and exporting one does to the config.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{motion::MotionSet, pad_identity::ControllerSupport};
use crate::sound::SoundSet;
use crate::config::{
    RumbleSet,
    Button, ButtonAction, Combo, Config, Game, Gestures, GyroMode, Indicator, InfoOverlay, ItemKind, Layer, LogOverlay, Macro, Mode,
    Menu, MacroStep, Origin, OverlayStyle, PackAssets, PackInfo, PackRef, Profile, Rule, free_name,
};

/// The pack format this app writes, and the newest it reads. 2 added layers; 3, toggles
/// that start on; 4, the keyboard and numpad styles; 5, the overlay font; 6, grid menus; 7,
/// the Guide shift, screenshot, recording and force-quit actions and layer indicators
/// with generated bindings, extra info overlays and a delay; 8, profiles stating what
/// controller features they need (replacing the pack-wide list); 9, the media controls and
/// in-game menu looks; 10, menu sounds, radial arcs and corner rounding; 11, how each overlay moves;
/// 12, the sounds of each overlay (replacing the menu sounds); 13, back paddles (their button
/// names and the `back_paddles` feature); 14, overlay decorations (outlines, shadows, gradients,
/// glows and text colors); 15, gyro acceleration curves, axes, player and world space, the
/// position mode, smoothing and pausing, and the touchpad's curve; 16, the touchpad's joystick,
/// menu and swipe modes, tapping, scrolling and gliding, the split pad, and layers' touchpads;
/// 17, modes (which replace toggled layers: older packs' toggled layers are read as modes);
/// 18, menu item icons, text typography, ornaments, background pictures, looping effects and
/// rumble cues; 19, `{icon:…}` tokens and inline text formatting in place of menu item icons,
/// the `{gyro:…}` and `{trackpad}` tokens, icons and sounds carried in the pack (`[assets]`), and the
/// `swift`, `dreamy` and `retro` motion styles; 20, `native_controller` in the header.
pub const FORMAT: u32 = 20;
pub const EXTENSION: &str = "padpack";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub format: u32,
    pub pack: Header,
    #[serde(default)]
    pub rules: Vec<Rule>,
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub macros: Vec<Macro>,
    #[serde(default)]
    pub menus: Vec<Menu>,
    #[serde(default)]
    pub info_overlays: Vec<InfoOverlay>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub log_overlays: Vec<LogOverlay>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<Mode>,
    /// How the on-screen keyboard and numpad look in this game, if the author set that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_style: Option<OverlayStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numpad_style: Option<OverlayStyle>,
    /// The look of the media controls and the in-game menu, if the author set those.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_style: Option<OverlayStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_style: Option<OverlayStyle>,
    /// The font of this game's overlays, menus and keyboards, if the author set one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_font: Option<String>,
    /// Whether this game's button glyphs use the Nintendo layout, if the author set that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nintendo_layout: Option<bool>,
    /// The overlay sounds, if the author set them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sounds: Option<SoundSet>,
    /// The controller's buzzes for each overlay, if the author set them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rumble: Option<RumbleSet>,
    /// How each overlay moves, if the author set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<MotionSet>,
    /// The icons and sounds the pack carries.
    #[serde(default, skip_serializing_if = "PackAssets::is_empty")]
    pub assets: PackAssets,
    /// Other controllers the game supports, if the author ticked any (see `Game::controllers`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controllers: Option<ControllerSupport>,
}

/// What a pack says about itself.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    /// Stable across versions of the same pack.
    pub id: String,
    /// Becomes the game's name.
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// The controller it was made with (free text).
    #[serde(default)]
    pub made_with: String,
    /// Read from packs made before profiles declared their own needs (format 7 and older),
    /// which `parse` moves onto the profiles. Never written.
    #[serde(default, skip_serializing)]
    pub requires: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub based_on: Option<PackRef>,
    /// The game plays with a controller by itself, so the pack is never offered when Steam
    /// starts the game: Steam Input looks after it then.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub native_controller: bool,
}

pub use crate::config::Feature;

/// A feature and where it's used, e.g. "profile “Play”".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeatureUse {
    pub feature: Feature,
    pub place: String,
    /// Whether the author declared that the profile can't be played without it. Otherwise it
    /// only adds to a scheme that works on a plain pad.
    pub required: bool,
}

impl Pack {
    pub fn info(&self) -> PackInfo {
        let h = &self.pack;
        PackInfo {
            id: h.id.clone(),
            version: h.version.clone(),
            author: h.author.clone(),
            description: h.description.clone(),
            made_with: h.made_with.clone(),
            based_on: h.based_on.clone(),
        }
    }

    /// The game this pack makes, before any clash handling.
    pub fn to_game(&self) -> Game {
        Game {
            name: self.pack.name.clone(),
            pack: self.info(),
            origin: None,
            rules: self.rules.clone(),
            profiles: self.profiles.clone(),
            macros: self.macros.clone(),
            menus: self.menus.clone(),
            info: self.info_overlays.clone(),
            logs: self.log_overlays.clone(),
            layers: self.layers.clone(),
            modes: self.modes.clone(),
            keyboard_style: self.keyboard_style.clone(),
            numpad_style: self.numpad_style.clone(),
            media_style: self.media_style.clone(),
            menu_style: self.menu_style.clone(),
            overlay_font: self.overlay_font.clone(),
            nintendo_layout: self.nintendo_layout,
            sounds: self.sounds,
            rumble: self.rumble,
            motion: self.motion,
            assets: self.assets.clone(),
            controllers: self.controllers,
            over_steam: false,
        }
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}

/// Reads a pack, refusing one from a newer app and anything malformed (whole or nothing).
pub fn parse(text: &str) -> Result<Pack> {
    let text = crate::config::migrate_text(text);
    let text = text.as_str();
    let table: toml::Table = toml::from_str(text).context("not a valid pack file")?;
    crate::config::check_finite(&table)?;
    let format = table.get("format").and_then(toml::Value::as_integer).context("not a pack file (no format number)")?;
    if format > FORMAT as i64 {
        bail!("This pack was made with a newer version of the app. Update to import it.");
    }
    if format < 1 {
        bail!("unknown pack format {format}");
    }
    let mut pack: Pack = toml::from_str(text).context("reading the pack")?;
    // Before format 8 the pack listed what its gyro-aiming profiles needed, as a whole.
    for feature in std::mem::take(&mut pack.pack.requires) {
        for p in pack.profiles.iter_mut().filter(|p| feature == Feature::Gyro && p.gyro.mode != GyroMode::Off) {
            p.requires.push(feature);
        }
    }
    crate::config::untoggle_menus(&mut pack.profiles, &mut pack.menus, &mut pack.layers);
    crate::config::untoggle_menus(&mut [], &mut [], &mut pack.modes);
    crate::config::modes::toggled_layers_to_modes(&mut pack.profiles, &mut pack.menus, &mut pack.layers, &mut pack.modes);
    // The built-in modes are every game's, fixed: a pack can't bring its own (see `built_in_mode`).
    pack.layers.retain(|l| crate::config::built_in_mode(&l.name).is_none());
    pack.modes.retain(|l| crate::config::built_in_mode(&l.name).is_none());
    crate::config::tidy_combos(&mut pack.profiles, &mut pack.layers);
    crate::config::tidy_combos(&mut [], &mut pack.modes);
    check(&pack)?;
    Ok(pack)
}

/// What a pack must hold to make a usable game.
fn check(pack: &Pack) -> Result<()> {
    if pack.pack.id.trim().is_empty() {
        bail!("the pack has no id");
    }
    if pack.pack.name.trim().is_empty() {
        bail!("the pack has no name");
    }
    if pack.profiles.is_empty() {
        bail!("the pack has no profiles");
    }
    if let Err(e) = pack.assets.check() {
        bail!("{e}");
    }
    let game = pack.to_game();
    for kind in ItemKind::ALL {
        let names = game.names(kind);
        let unique: BTreeSet<&str> = names.iter().copied().collect();
        if unique.len() != names.len() {
            bail!("the pack has two {}s with the same name", kind.noun());
        }
    }
    let profiles: BTreeSet<&String> = pack.profiles.iter().map(|p| &p.name).collect();
    if profiles.len() != pack.profiles.len() {
        bail!("the pack has two profiles with the same name");
    }
    if let Some(r) = pack.rules.iter().find(|r| !profiles.contains(&r.profile)) {
        bail!("a rule points to missing profile {:?}", r.profile);
    }
    // What couldn't be saved can't come in. Built-in modes are every game's, whatever the shortcuts.
    if let Some(problem) = crate::config::problems::game_problem(&crate::config::Config::default(), &game) {
        bail!("{problem}");
    }
    Ok(())
}

/// A new random pack ID (a version 4 UUID).
pub fn new_id() -> String {
    use std::io::Read;
    let mut b = [0u8; 16];
    let read = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b));
    if read.is_err() {
        // Not cryptographic, only unique enough: the clock and this process.
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let seed = now.as_nanos() ^ ((std::process::id() as u128) << 64);
        b = seed.to_le_bytes();
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// Every (kind, name) reference in these profiles, menus, macros and layers (or modes), nested
/// ones included, and the info overlays layers use as their indicator.
fn references(profiles: &[Profile], menus: &[Menu], macros: &[Macro], layers: &[&Layer]) -> BTreeSet<(ItemKind, String)> {
    let mut refs = BTreeSet::new();
    let mut visit = |a: &ButtonAction| {
        a.walk(&mut |a| {
            for kind in ItemKind::ALL {
                if let Some(name) = kind.name_in(a) {
                    refs.insert((kind, name.clone()));
                }
            }
        })
    };
    for p in profiles {
        p.actions().into_iter().for_each(&mut visit);
    }
    for m in menus {
        m.items.iter().for_each(|i| visit(&i.action));
    }
    for m in macros {
        m.steps.iter().filter_map(MacroStep::action).for_each(&mut visit);
    }
    for l in layers {
        l.actions().into_iter().for_each(&mut visit);
    }
    for l in layers {
        if let Indicator::Info(name) = &l.indicator {
            refs.insert((ItemKind::Info, name.clone()));
        }
    }
    let pads = profiles.iter().map(|p| &p.touchpad).chain(layers.iter().filter_map(|l| l.touchpad.as_ref()));
    refs.extend(pads.flat_map(crate::config::TouchpadConfig::menus).map(|name| (ItemKind::Menu, name.clone())));
    refs
}

/// Every (kind, name) a game's own items refer to. Built-in modes always resolve, so they're left out.
pub(crate) fn game_references(g: &Game) -> BTreeSet<(ItemKind, String)> {
    let shifts: Vec<&Layer> = g.layers.iter().chain(&g.modes).collect();
    references(&g.profiles, &g.menus, &g.macros, &shifts)
        .into_iter()
        .filter(|(kind, name)| !(*kind == ItemKind::Mode && crate::config::built_in_mode(name).is_some()))
        .collect()
}

/// What copying the `kind` item `name` out of `source` has to bring: the item itself (first)
/// and, followed through menu items and macro steps, every item of `source` it refers to
/// that `resolves` can't already find where it's going.
pub fn dependencies(source: &Game, kind: ItemKind, name: &str, resolves: impl Fn(ItemKind, &str) -> bool) -> Vec<(ItemKind, String)> {
    if !source.names(kind).contains(&name) {
        return Vec::new();
    }
    let mut needed = vec![(kind, name.to_string())];
    let mut i = 0;
    while i < needed.len() {
        let (k, n) = needed[i].clone();
        let refs = match k {
            ItemKind::Macro => source.macros.iter().find(|m| m.name == n).map(|m| references(&[], &[], std::slice::from_ref(m), &[])),
            ItemKind::Menu => source.menus.iter().find(|m| m.name == n).map(|m| references(&[], std::slice::from_ref(m), &[], &[])),
            ItemKind::Layer => source.layers.iter().find(|l| l.name == n).map(|l| references(&[], &[], &[], &[l])),
            ItemKind::Mode => source.mode_named(&n).map(|l| references(&[], &[], &[], &[l])),
            ItemKind::Info | ItemKind::Log => None,
        };
        for (rk, rn) in refs.unwrap_or_default() {
            let item = (rk, rn);
            if !needed.contains(&item) && !resolves(rk, &item.1) && source.names(rk).contains(&item.1.as_str()) {
                needed.push(item);
            }
        }
        i += 1;
    }
    needed
}

/// Inputs beyond the XInput baseline that a game's profiles and layers use or demand, and
/// where. What a profile demands is only what its author declared.
pub fn features(game: &Game) -> Vec<FeatureUse> {
    let profiles = game.profiles.iter().flat_map(|p| {
        let uses = [
            (Feature::Gyro, p.gyro.mode != GyroMode::Off),
            (Feature::BackPaddles, uses_paddles(&p.buttons, &p.gestures, &p.combos)),
            (Feature::Touchpad, !p.touchpad.is_default()),
        ];
        Feature::ALL.into_iter().filter_map(move |feature| {
            let required = p.requires.contains(&feature);
            let used = uses.iter().any(|(f, used)| *f == feature && *used);
            (required || used).then(|| FeatureUse { feature, place: format!("profile “{}”", p.name), required })
        })
    });
    let shifts = game.layers.iter().map(|l| (l, "layer")).chain(game.modes.iter().map(|l| (l, "mode")));
    let layers = shifts.flat_map(|(l, noun)| {
        let gyro = l.gyro.as_ref().is_some_and(|g| g.mode != GyroMode::Off);
        [(Feature::Gyro, gyro), (Feature::BackPaddles, uses_paddles(&l.buttons, &l.gestures, &l.combos)), (Feature::Touchpad, l.touchpad.is_some())]
            .into_iter()
            .filter(|(_, used)| *used)
            .map(move |(feature, _)| FeatureUse { feature, place: format!("{noun} “{}”", l.name), required: false })
    });
    profiles.chain(layers).collect()
}

/// Whether any buttons, gestures or combos use a back paddle (a set action, not "Disabled").
fn uses_paddles(buttons: &BTreeMap<Button, ButtonAction>, gestures: &BTreeMap<Button, Gestures>, combos: &[Combo]) -> bool {
    Button::PADDLES.iter().any(|b| {
        buttons.get(b).is_some_and(|a| *a != ButtonAction::Disabled) || gestures.contains_key(b)
    }) || combos.iter().any(|c| c.buttons.iter().any(|b| Button::PADDLES.contains(b)))
}

impl Pack {
    /// The features every profile of the pack needs, so a player without them gets nothing.
    pub fn required_by_all(&self) -> Vec<Feature> {
        Feature::ALL.into_iter().filter(|f| !self.profiles.is_empty() && self.profiles.iter().all(|p| p.requires.contains(f))).collect()
    }

    /// The names of the profiles that need `feature`.
    pub fn needing(&self, feature: Feature) -> Vec<&str> {
        self.profiles.iter().filter(|p| p.requires.contains(&feature)).map(|p| p.name.as_str()).collect()
    }
}

/// A pack ready to save, and what the export dialog should point out.
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    pub pack: Pack,
    /// References to items that exist nowhere, e.g. "macro \"Dodge\"".
    pub dangling: Vec<String>,
    pub features: Vec<FeatureUse>,
}

/// The pack metadata the export dialog starts from. Exporting a game that came from someone
/// else's pack (or the library) makes a fork: a new ID, crediting the original. `library`
/// keeps a library pack's ID instead, for the maintainer publishing a new version.
pub fn draft(game: &Game, library: bool) -> PackInfo {
    let mut info = game.pack.clone();
    if let Some(origin) = &game.origin
        && !library
        && (info.id.is_empty() || info.id == origin.id)
    {
        let name = if origin.name.is_empty() { &game.name } else { &origin.name };
        info.based_on = Some(PackRef {
            id: origin.id.clone(),
            name: name.clone(),
            author: info.author.clone(),
            version: origin.version.clone(),
        });
        info.id = new_id();
        info.author = String::new();
        info.version = String::new();
    }
    if info.id.is_empty() {
        info.id = new_id();
    }
    if info.version.is_empty() {
        info.version = "1.0".into();
    }
    info
}

/// Writes `game` as a pack with this metadata.
pub fn export(game: &Game, info: &PackInfo) -> Export {
    let pack_game = game.clone();
    let dangling: Vec<String> = game_references(game)
        .into_iter()
        .filter(|(kind, name)| !game.names(*kind).contains(&name.as_str()))
        .map(|(kind, name)| format!("{} {name:?}", kind.noun()))
        .collect();
    let features = features(&pack_game);
    let pack = Pack {
        format: FORMAT,
        pack: Header {
            id: info.id.clone(),
            name: game.name.clone(),
            version: info.version.clone(),
            author: info.author.clone(),
            description: info.description.clone(),
            made_with: info.made_with.clone(),
            requires: Vec::new(),
            based_on: info.based_on.clone(),
            native_controller: false,
        },
        rules: pack_game.rules.iter().map(|r| Rule { enabled: true, ..r.clone() }).collect(),
        profiles: pack_game.profiles,
        macros: pack_game.macros,
        menus: pack_game.menus,
        info_overlays: pack_game.info,
        log_overlays: pack_game.logs,
        layers: pack_game.layers,
        modes: pack_game.modes,
        keyboard_style: pack_game.keyboard_style,
        numpad_style: pack_game.numpad_style,
        media_style: pack_game.media_style,
        menu_style: pack_game.menu_style,
        overlay_font: pack_game.overlay_font,
        nintendo_layout: pack_game.nintendo_layout,
        sounds: pack_game.sounds,
        rumble: pack_game.rumble,
        motion: pack_game.motion,
        assets: pack_game.assets,
        controllers: pack_game.controllers,
    };
    Export { pack, dangling, features }
}

/// A stable 64-bit FNV-1a hash, as hex.
fn fnv(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn hash<T: Serialize>(item: &T) -> String {
    fnv(&toml::to_string(item).unwrap_or_default())
}

/// A content hash per profile, macro, menu and info overlay, keyed "kind:name".
pub fn item_hashes(game: &Game) -> BTreeMap<String, String> {
    let mut hashes = BTreeMap::new();
    for p in &game.profiles {
        hashes.insert(format!("profile:{}", p.name), hash(p));
    }
    for m in &game.macros {
        hashes.insert(format!("macro:{}", m.name), hash(m));
    }
    for m in &game.menus {
        hashes.insert(format!("menu:{}", m.name), hash(m));
    }
    for o in &game.info {
        hashes.insert(format!("info overlay:{}", o.name), hash(o));
    }
    for o in &game.logs {
        hashes.insert(format!("log overlay:{}", o.name), hash(o));
    }
    for l in &game.layers {
        hashes.insert(format!("layer:{}", l.name), hash(l));
    }
    for l in &game.modes {
        hashes.insert(format!("mode:{}", l.name), hash(l));
    }
    hashes
}

/// Items edited, added or removed since the game was imported, as "macro “Dodge”".
pub fn edited_items(game: &Game) -> Vec<String> {
    let Some(origin) = &game.origin else { return Vec::new() };
    let now = item_hashes(game);
    let keys: BTreeSet<&String> = now.keys().chain(origin.hashes.keys()).collect();
    keys.into_iter()
        .filter(|k| now.get(*k) != origin.hashes.get(*k))
        .map(|k| {
            let (kind, name) = k.split_once(':').unwrap_or(("item", k));
            format!("{kind} “{name}”")
        })
        .collect()
}

/// How an incoming pack's version relates to the installed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateKind {
    Update,
    Reinstall,
    Downgrade,
}

impl UpdateKind {
    pub fn verb(self) -> &'static str {
        match self {
            UpdateKind::Update => "Update",
            UpdateKind::Reinstall => "Reinstall",
            UpdateKind::Downgrade => "Downgrade",
        }
    }
}

/// Compares dotted versions number by number ("1.10" is newer than "1.9"); anything that
/// isn't a number compares as text. A pre-release ("1.0-beta") comes before its release.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let split = |v: &str| {
        let v = v.trim().trim_start_matches(['v', 'V']);
        // Build metadata ("+linux") doesn't order versions.
        let v = v.split('+').next().unwrap_or(v);
        match v.split_once('-') {
            Some((core, pre)) => (core.to_string(), Some(pre.to_string())),
            None => (v.to_string(), None),
        }
    };
    let ((a, a_pre), (b, b_pre)) = (split(a), split(b));
    let (a, b): (Vec<&str>, Vec<&str>) = (a.split('.').collect(), b.split('.').collect());
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or("0"), b.get(i).copied().unwrap_or("0"));
        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            _ => x.cmp(y),
        };
        if order.is_ne() {
            return order;
        }
    }
    match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) => compare_versions(&x, &y),
    }
}

/// An imported rule matching the same window as another game's (switched-on) rule.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleClash {
    /// Index in the pack's rules.
    pub rule: usize,
    pub other_game: String,
}

/// What importing a pack would do, for the preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub pack: Pack,
    pub library: bool,
    /// The installed game with this pack's ID, which an import updates.
    pub update_of: Option<String>,
    pub update_kind: Option<UpdateKind>,
    /// For an update: the installed game's items edited since its import.
    pub edited: Vec<String>,
    /// Another game with the pack's name (not an update).
    pub name_clash: bool,
    /// The name the game gets if renamed.
    pub renamed: String,
    pub rule_clashes: Vec<RuleClash>,
}

/// What the user chose in the preview.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choices {
    /// Replace the game that has the pack's name instead of renaming the import.
    pub replace: bool,
    /// Per rule clash: keep the other game's rule (the imported one comes in switched off).
    pub keep_mine: Vec<bool>,
}

/// Works out what importing `pack` would do to `config`.
pub fn plan(config: &Config, pack: Pack, library: bool) -> Plan {
    let installed = config.games.iter().find(|g| g.origin.as_ref().is_some_and(|o| o.id == pack.pack.id));
    let update_kind = installed.and_then(|g| g.origin.as_ref()).map(|o| match compare_versions(&pack.pack.version, &o.version) {
        std::cmp::Ordering::Greater => UpdateKind::Update,
        std::cmp::Ordering::Equal => UpdateKind::Reinstall,
        std::cmp::Ordering::Less => UpdateKind::Downgrade,
    });
    let name = &pack.pack.name;
    let name_clash = installed.is_none() && config.games.iter().any(|g| &g.name == name);
    let renamed = free_name(name, |n| config.games.iter().any(|g| g.name == n));

    let target = installed.map(|g| g.name.as_str());
    let mut rule_clashes = Vec::new();
    for (i, rule) in pack.rules.iter().enumerate() {
        // An update keeps what was decided for rules the game already had.
        if installed.is_some_and(|g| g.rules.iter().any(|r| r.same_match(rule))) {
            continue;
        }
        let other = config
            .games
            .iter()
            .filter(|g| Some(g.name.as_str()) != target)
            .find(|g| g.rules.iter().any(|r| r.enabled && r.same_match(rule)));
        if let Some(other) = other {
            rule_clashes.push(RuleClash { rule: i, other_game: other.name.clone() });
        }
    }

    Plan {
        update_of: installed.map(|g| g.name.clone()),
        update_kind,
        edited: installed.map(edited_items).unwrap_or_default(),
        name_clash,
        renamed,
        rule_clashes,
        pack,
        library,
    }
}

/// Rule clashes that still matter: not with a game the import replaces.
pub fn live_clashes<'a>(plan: &'a Plan, choices: &Choices) -> impl Iterator<Item = (usize, &'a RuleClash)> {
    let replaced = (plan.name_clash && choices.replace).then_some(plan.pack.pack.name.as_str());
    plan.rule_clashes.iter().enumerate().filter(move |(_, c)| Some(c.other_game.as_str()) != replaced)
}

/// Imports the planned pack into `config` and returns the new game's name.
pub fn apply(config: &mut Config, plan: &Plan, choices: &Choices) -> String {
    let mut game = plan.pack.to_game();

    // Rules: an update keeps each existing rule's on/off; clashes go as chosen.
    let previous = plan.update_of.as_deref().and_then(|n| config.games.iter().find(|g| g.name == n));
    // So does whether the player uses the setup even when Steam Input could have the game.
    game.over_steam = previous.is_some_and(|g| g.over_steam);
    for (i, rule) in game.rules.iter_mut().enumerate() {
        if let Some(old) = previous.and_then(|g| g.rules.iter().find(|r| r.same_match(rule))) {
            rule.enabled = old.enabled;
        }
        if let Some((c, _)) = live_clashes(plan, choices).find(|(_, c)| c.rule == i)
            && choices.keep_mine.get(c).copied().unwrap_or(false)
        {
            rule.enabled = false;
        }
    }
    for (c, clash) in live_clashes(plan, choices) {
        if choices.keep_mine.get(c).copied().unwrap_or(false) {
            continue;
        }
        let rule = &plan.pack.rules[clash.rule];
        if let Some(other) = config.games.iter_mut().find(|g| g.name == clash.other_game) {
            other.rules.iter_mut().filter(|r| r.same_match(rule)).for_each(|r| r.enabled = false);
        }
    }

    game.origin = Some(Origin {
        id: plan.pack.pack.id.clone(),
        name: plan.pack.pack.name.clone(),
        version: plan.pack.pack.version.clone(),
        library: plan.library,
        hashes: item_hashes(&game),
    });

    let replace = match &plan.update_of {
        Some(name) => {
            game.name = name.clone();
            Some(name.clone())
        }
        None if plan.name_clash && choices.replace => Some(game.name.clone()),
        None if plan.name_clash => {
            game.name = plan.renamed.clone();
            None
        }
        None => None,
    };
    let name = game.name.clone();
    match replace.and_then(|n| config.games.iter().position(|g| g.name == n)) {
        Some(i) => {
            let kept = keep_looks(game, &config.games[i]);
            config.games[i] = kept;
        }
        None => config.games.push(game),
    }
    name
}

/// A pack that sets no look leaves the player's own in place.
fn keep_looks(mut game: Game, old: &Game) -> Game {
    game.keyboard_style = game.keyboard_style.or_else(|| old.keyboard_style.clone());
    game.numpad_style = game.numpad_style.or_else(|| old.numpad_style.clone());
    game.media_style = game.media_style.or_else(|| old.media_style.clone());
    game.menu_style = game.menu_style.or_else(|| old.menu_style.clone());
    game.overlay_font = game.overlay_font.or_else(|| old.overlay_font.clone());
    game.nintendo_layout = game.nintendo_layout.or(old.nintendo_layout);
    game.sounds = game.sounds.or(old.sounds);
    game.rumble = game.rumble.or(old.rumble);
    game.motion = game.motion.or(old.motion);
    game.controllers = game.controllers.or(old.controllers);
    // What the player added to the game stays, unless the pack carries one of the same name.
    for (name, svg) in &old.assets.icons {
        game.assets.icons.entry(name.clone()).or_insert_with(|| svg.clone());
    }
    for (name, wav) in &old.assets.sounds {
        game.assets.sounds.entry(name.clone()).or_insert_with(|| wav.clone());
    }
    game
}

#[cfg(test)]
mod tests;
