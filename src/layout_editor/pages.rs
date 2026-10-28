//! Every page of Edit Controls, the module that builds each one's rows, and its title (which
//! the pages after it also show as a breadcrumb).

use crate::{
    config::{Analog, Button, Config, GestureKind, Stick, TouchSide, Trigger},
    sound::{Feedback, SoundOverlay},
};

use super::{
    Cx,
    action::{ActionAt, Slot},
    action_page::{self, Picker},
    decor::{self, Decor},
    feel, game, guide_page, infos, inputs, layers,
    style::{self, PaintOf, StyleAt},
    logs::{self, Tracked},
    gyro, macros, menus,
    owner::{Owner, Shift, layer, owner_name},
    rows::Row,
    sticks, top, touchpad, triggers,
};

/// Where the editor is. Each page's parent is the one it was opened from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Top,
    /// An owner's buttons.
    Buttons(Owner),
    /// What an action is, and the pickers it opens.
    Action(ActionAt),
    Pad(ActionAt),
    KeyGroups(ActionAt),
    /// One row of the keyboard, for the key picker.
    Keys(ActionAt, usize),
    Pick(ActionAt, Picker),
    Multi(ActionAt),
    Gestures(Owner, Button),
    /// A button's own tap window and long press time.
    GestureTiming(Owner, Button),
    /// A combo's own window.
    ComboTiming(Owner, usize),
    Combos(Owner),
    Combo(Owner, usize),
    Sticks(Owner),
    Stick(Owner, Stick),
    /// A stick's ring: its sectors.
    Ring(Owner, Stick),
    Triggers(Owner),
    Trigger(Owner, Trigger),
    Zones(Owner, Analog),
    Zone(Owner, Analog, usize),
    Gyro(Owner),
    /// The touchpad, or its right half when it is split.
    Touch(Owner, TouchSide),
    /// The game's modes or layers, and one of them.
    Layers(Shift),
    Layer(Shift, usize),
    /// The profile's combos a mode or layer turns off.
    OffCombos(Shift, usize),
    Macros,
    Macro(usize),
    /// A step of a macro.
    Step(usize, usize),
    /// What a new step of a macro is.
    AddStep(usize),
    Menus,
    Menu(usize),
    MenuItems(usize),
    MenuItem(usize, usize),
    /// Picking a menu's close button.
    MenuClose(usize),
    /// Picking the button a button menu's item is chosen with.
    ItemButton(usize, usize),
    Infos,
    Info(usize),
    /// An info overlay's rows of cells, and one row.
    InfoCells(usize),
    InfoRow(usize, usize),
    Logs,
    Log(usize),
    /// The Guide overlay: its notes, line by line, and its Mappings rows.
    Guide,
    GuideNotes,
    GuideRows,
    GuideRow(usize),
    /// The inputs a Mappings row can share with.
    GuideShare(usize),
    /// The profile's own settings, and the game's.
    Profile,
    Game,
    GameFont,
    Rules,
    Rule(usize),
    GameProfiles,
    GameProfile(usize),
    /// A look, a color of it, its text, its decorations, and one decoration.
    Style(StyleAt),
    Paint(StyleAt, PaintOf),
    StyleText(StyleAt),
    Decorations(StyleAt),
    Decor(StyleAt, Decor),
    /// The game's sounds, one overlay's, and one cue.
    Sounds,
    SoundsOf(SoundOverlay),
    Cue(SoundOverlay, Feedback),
    Rumble,
    RumbleOf(SoundOverlay),
    Motion,
    /// What a log overlay, or an info overlay's current-input cells, follow.
    Tracking(Tracked),
    /// The inputs of one group a display follows.
    TrackGroup(Tracked, usize),
}

/// The rows of `page`.
pub(super) fn rows(page: Page, cx: &Cx) -> Vec<Row> {
    input_rows(page, cx).unwrap_or_else(|| game_rows(page, cx.config))
}

/// The rows of the pages about inputs and actions; `None` for the game's.
fn input_rows(page: Page, cx: &Cx) -> Option<Vec<Row>> {
    let config = cx.config;
    Some(match page {
        Page::Top => top::rows(cx),
        Page::Buttons(owner) => inputs::button_rows(cx, owner),
        Page::Action(at) => action_page::rows(config, at),
        Page::Pad(at) => action_page::pad_rows(at),
        Page::KeyGroups(at) => action_page::key_group_rows(config, at),
        Page::Keys(at, group) => action_page::key_rows(config, at, group),
        Page::Pick(at, p) => action_page::pick_rows(config, at, p),
        Page::Multi(at) => action_page::multi_rows(config, at),
        Page::Gestures(owner, b) => inputs::gesture_rows(config, owner, b),
        Page::GestureTiming(owner, b) => inputs::gesture_timing_rows(config, owner, b),
        Page::ComboTiming(owner, i) => inputs::combo_timing_rows(config, owner, i),
        Page::Combos(owner) => inputs::combo_rows(config, owner),
        Page::Combo(owner, i) => inputs::combo_page(cx, owner, i),
        Page::Sticks(owner) => sticks::sticks_rows(config, owner),
        Page::Stick(owner, s) => sticks::stick_rows(config, owner, s),
        Page::Ring(owner, s) => sticks::ring_rows(config, owner, s),
        Page::Triggers(owner) => triggers::triggers_rows(config, owner),
        Page::Trigger(owner, t) => triggers::trigger_rows(config, owner, t),
        Page::Zones(owner, a) => triggers::zones_rows(config, owner, a),
        Page::Zone(owner, a, i) => triggers::zone_rows(config, owner, a, i),
        Page::Gyro(owner) => gyro::gyro_rows(config, owner),
        Page::Touch(owner, side) => touchpad::touch_rows(config, owner, side),
        _ => return None,
    })
}

/// The rows of the pages about the game's items, looks and details.
fn game_rows(page: Page, config: &Config) -> Vec<Row> {
    match page {
        Page::Layers(shift) => layers::list_rows(config, shift),
        Page::Layer(shift, i) => layers::layer_rows(config, shift, i),
        Page::OffCombos(shift, i) => layers::off_combo_rows(config, shift, i),
        Page::Macros => macros::list_rows(config),
        Page::Macro(m) => macros::macro_rows(config, m),
        Page::Step(m, i) => macros::step_rows(config, m, i),
        Page::AddStep(m) => macros::add_rows(m),
        Page::Menus => menus::list_rows(config),
        Page::Menu(m) => menus::menu_rows(config, m),
        Page::MenuItems(m) => menus::items_rows(config, m),
        Page::MenuItem(m, i) => menus::item_rows(config, m, i),
        Page::MenuClose(m) => menus::close_rows(config, m),
        Page::ItemButton(m, i) => menus::item_button_rows(config, m, i),
        Page::Infos => infos::list_rows(config),
        Page::Info(i) => infos::info_rows(config, i),
        Page::InfoCells(i) => infos::cells_rows(config, i),
        Page::InfoRow(i, r) => infos::row_rows(config, i, r),
        Page::Guide => guide_page::guide_rows(config),
        Page::GuideNotes => guide_page::notes_rows(config),
        Page::GuideRows => guide_page::mapping_rows(config),
        Page::GuideRow(i) => guide_page::row_rows(config, i),
        Page::GuideShare(i) => guide_page::share_rows(config, i),
        Page::Profile => game::profile_rows(config),
        Page::Game => game::game_rows(config),
        Page::GameFont => game::font_rows(config),
        Page::Rules => game::rules_rows(config),
        Page::Rule(i) => game::rule_rows(config, i),
        Page::GameProfiles => game::profiles_rows(config),
        Page::GameProfile(i) => game::game_profile_rows(config, i),
        Page::Style(at) => style::style_rows(config, at),
        Page::Paint(at, of) => style::paint_rows(config, at, of),
        Page::StyleText(at) => style::text_rows(config, at),
        Page::Decorations(at) => decor::list_rows(config, at),
        Page::Decor(at, d) => decor::decor_rows(config, at, d),
        Page::Sounds => feel::sounds_rows(config),
        Page::SoundsOf(o) => feel::overlay_sounds_rows(config, o),
        Page::Cue(o, f) => feel::cue_rows(config, o, f),
        Page::Rumble => feel::rumble_rows(config),
        Page::RumbleOf(o) => feel::overlay_rumble_rows(config, o),
        Page::Motion => feel::motion_rows(config),
        Page::Logs => logs::list_rows(config),
        Page::Log(i) => logs::log_rows(config, i),
        Page::Tracking(of) => logs::tracking_rows(config, of),
        Page::TrackGroup(of, g) => logs::group_rows(config, of, g),
        _ => Vec::new(),
    }
}

/// The title of a page.
pub(super) fn title(page: Page, config: &Config) -> String {
    input_title(page, config).unwrap_or_else(|| game_title(page, config))
}

fn input_title(page: Page, config: &Config) -> Option<String> {
    Some(match page {
        Page::Top => "Edit Controls".into(),
        Page::Buttons(owner) => in_owner(config, owner, "Buttons"),
        Page::Action(at) | Page::Multi(at) => slot_title(config, at.slot),
        Page::Pad(at) => format!("{} · gamepad button", slot_title(config, at.slot)),
        Page::KeyGroups(at) | Page::Keys(at, _) => format!("{} · keys", slot_title(config, at.slot)),
        Page::Pick(at, p) => format!("{} · pick a {}", slot_title(config, at.slot), p.noun()),
        Page::Gestures(owner, b) => in_owner(config, owner, &format!("{} gestures", b.short_name())),
        Page::Combos(owner) => in_owner(config, owner, "Combos"),
        Page::Combo(owner, i) => in_owner(config, owner, &format!("Combo {}", i + 1)),
        Page::GestureTiming(owner, b) => in_owner(config, owner, &format!("{} timing", b.short_name())),
        Page::ComboTiming(owner, i) => in_owner(config, owner, &format!("Combo {} timing", i + 1)),
        Page::Sticks(owner) => in_owner(config, owner, "Sticks"),
        Page::Stick(owner, s) => in_owner(config, owner, &stick_name(s)),
        Page::Ring(owner, s) => in_owner(config, owner, &format!("{} ring", stick_name(s))),
        Page::Triggers(owner) => in_owner(config, owner, "Triggers"),
        Page::Trigger(owner, t) => in_owner(config, owner, &trigger_name(t)),
        Page::Zones(owner, a) => in_owner(config, owner, &format!("{} zones", analog_name(a))),
        Page::Zone(owner, a, i) => in_owner(config, owner, &format!("{} zone {}", analog_name(a), i + 1)),
        Page::Gyro(owner) => in_owner(config, owner, "Gyro"),
        Page::Touch(owner, side) => in_owner(config, owner, if side == TouchSide::Left { "Touchpad" } else { "Touchpad · right half" }),
        _ => return None,
    })
}

fn game_title(page: Page, config: &Config) -> String {
    match page {
        Page::Layers(shift) => format!("{}s", shift.title()),
        Page::Layer(shift, i) => layer(config, shift, i).map_or_else(|| shift.title().into(), |l| l.name.clone()),
        Page::OffCombos(shift, i) => in_owner(config, Owner::Shift(shift, i), "Profile combos turned off"),
        Page::Macros => "Macros".into(),
        Page::Macro(m) => macro_name(config, m),
        Page::Step(m, i) => format!("{} · step {}", macro_name(config, m), i + 1),
        Page::AddStep(m) => format!("{} · new step", macro_name(config, m)),
        Page::Menus => "Menus".into(),
        Page::Menu(m) => menu_name(config, m),
        Page::MenuItems(m) => format!("{} · items", menu_name(config, m)),
        Page::MenuItem(m, i) | Page::ItemButton(m, i) => format!("{} · item {}", menu_name(config, m), i + 1),
        Page::MenuClose(m) => format!("{} · closes with", menu_name(config, m)),
        Page::Infos => "Info overlays".into(),
        Page::Info(i) => info_name(config, i),
        Page::InfoCells(i) => format!("{} · cells", info_name(config, i)),
        Page::InfoRow(i, r) => format!("{} · row {}", info_name(config, i), r + 1),
        Page::Guide => "Guide overlay".into(),
        Page::GuideNotes => "Guide overlay · notes".into(),
        Page::GuideRows => "Guide overlay · mappings".into(),
        Page::GuideRow(i) | Page::GuideShare(i) => format!("Guide overlay · row {}", i + 1),
        Page::Profile => format!("Profile {}", config.active_ref().profile),
        Page::Game => config.active_ref().game.unwrap_or_else(|| "General".into()),
        Page::GameFont => "Font".into(),
        Page::Rules => "Window rules".into(),
        Page::Rule(i) => format!("Rule {}", i + 1),
        Page::GameProfiles => "Profiles".into(),
        Page::GameProfile(i) => config.active_game().profiles.get(i).map_or_else(|| "Profile".into(), |p| p.name.clone()),
        Page::Style(at) => style::style_title(config, at),
        Page::Paint(_, of) => paint_title(of).into(),
        Page::StyleText(_) => "Text".into(),
        Page::Decorations(_) => "Decorations".into(),
        Page::Decor(_, d) => d.name().into(),
        Page::Sounds => "Sounds".into(),
        Page::SoundsOf(o) => format!("{} sounds", o.label()),
        Page::Cue(o, f) => feel::cue_title(o, f),
        Page::Rumble => "Rumble".into(),
        Page::RumbleOf(o) => format!("{} rumble", o.label()),
        Page::Motion => "Motion".into(),
        Page::Logs => "Log overlays".into(),
        Page::Log(i) => logs::log_name(config, i),
        Page::Tracking(of) => logs::tracking_title(config, of),
        Page::TrackGroup(of, g) => format!("{} · {}", logs::tracking_title(config, of), crate::config::INPUT_GROUPS.get(g).map_or("", |(name, _)| name).to_lowercase()),
        _ => String::new(),
    }
}

/// A title for a page of a mode or layer names it first; the profile's pages don't.
fn in_owner(config: &Config, owner: Owner, what: &str) -> String {
    match owner {
        Owner::Profile => what.into(),
        Owner::Shift(..) => format!("{} · {what}", owner_name(config, owner)),
    }
}

/// The title of an action's page: what holds it.
pub(super) fn slot_title(config: &Config, slot: Slot) -> String {
    match slot {
        Slot::Button(owner, b) => in_owner(config, owner, &format!("{} button", b.short_name())),
        Slot::Gesture(owner, b, kind) => in_owner(config, owner, &format!("{} {}", b.short_name(), gesture_name(kind).to_lowercase())),
        Slot::Combo(owner, i) => in_owner(config, owner, &format!("Combo {}", i + 1)),
        Slot::Trigger(owner, t) => in_owner(config, owner, &format!("{} press", trigger_name(t))),
        Slot::TriggerGesture(owner, t, kind) => in_owner(config, owner, &format!("{} {}", trigger_name(t), gesture_name(kind).to_lowercase())),
        Slot::Zone(owner, a, i) => in_owner(config, owner, &format!("{} zone {}", analog_name(a), i + 1)),
        Slot::Ring(owner, s, i) => in_owner(config, owner, &format!("{} ring sector {}", stick_name(s), i + 1)),
        Slot::Touch(owner, t) => in_owner(config, owner, &inputs::touch_slot_name(t)),
        Slot::MenuItem(m, i) => format!("{} · item {}", menu_name(config, m), i + 1),
        Slot::MacroStep(m, i) => format!("{} · step {}", macro_name(config, m), i + 1),
    }
}

pub(super) fn gesture_name(kind: GestureKind) -> &'static str {
    match kind {
        GestureKind::DoubleTap => "Double tap",
        GestureKind::TripleTap => "Triple tap",
        GestureKind::LongPress => "Long press",
        GestureKind::OnPress => "On press",
        GestureKind::OnRelease => "On release",
    }
}

pub(super) fn stick_name(s: Stick) -> String {
    format!("{} Stick", side(s == Stick::Left))
}

pub(super) fn trigger_name(t: Trigger) -> String {
    format!("{} Trigger", side(t == Trigger::Left))
}

pub(super) fn analog_name(a: Analog) -> String {
    match a {
        Analog::Stick(s) => stick_name(s),
        Analog::Trigger(t) => trigger_name(t),
    }
}

pub(super) fn side(left: bool) -> &'static str {
    if left { "Left" } else { "Right" }
}

fn macro_name(config: &Config, m: usize) -> String {
    config.active_game().macros.get(m).map_or_else(|| "Macro".into(), |x| x.name.clone())
}

fn paint_title(of: PaintOf) -> &'static str {
    match of {
        PaintOf::Background => "Background color",
        PaintOf::Items => "Items' color",
        PaintOf::Selected => "Selected item's color",
        PaintOf::Decor(d) => d.name(),
    }
}

fn info_name(config: &Config, i: usize) -> String {
    config.active_game().info.get(i).map_or_else(|| "Info overlay".into(), |o| o.name.clone())
}

fn menu_name(config: &Config, m: usize) -> String {
    config.active_game().menus.get(m).map_or_else(|| "Menu".into(), |x| x.name.clone())
}
