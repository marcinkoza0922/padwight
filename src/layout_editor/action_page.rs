//! The action page, where any slot's action is chosen, and the pickers it opens: a gamepad
//! button, keys, one of the game's modes, layers, macros, menus or overlays, and the actions of a
//! Multi. A Toggle or Turbo keeps what it wraps on a page of its own, reached from the top row.

use crate::{
    config::{Button, ButtonAction, Config, Keyword, MouseButton, Toggled, WheelDirection},
    keyboard::Board,
    menu::Tone,
};

use super::{
    Page,
    action::{ActionAt, Slot, action, set_action},
    rows::{Act, Nav, Number, Row, set},
};

/// What a picker lists from the game: its modes, layers, macros, menus or overlays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    Mode,
    Layer,
    Macro,
    Menu,
    Info,
    Log,
}

impl Picker {
    pub const ALL: [Picker; 6] = [Picker::Mode, Picker::Layer, Picker::Macro, Picker::Menu, Picker::Info, Picker::Log];

    fn label(self) -> &'static str {
        match self {
            Picker::Mode => "Switch mode",
            Picker::Layer => "Hold layer",
            Picker::Macro => "Macro",
            Picker::Menu => "Open menu",
            Picker::Info => "Info overlay",
            Picker::Log => "Log overlay",
        }
    }

    pub fn noun(self) -> &'static str {
        match self {
            Picker::Mode => "mode",
            Picker::Layer => "layer",
            Picker::Macro => "macro",
            Picker::Menu => "menu",
            Picker::Info => "info overlay",
            Picker::Log => "log overlay",
        }
    }

    fn keyword(self) -> Keyword {
        match self {
            Picker::Mode => Keyword::Mode,
            Picker::Layer => Keyword::Layer,
            Picker::Macro => Keyword::Macro,
            Picker::Menu => Keyword::Menu,
            Picker::Info => Keyword::Info,
            Picker::Log => Keyword::Log,
        }
    }

    fn action(self, name: &str) -> ButtonAction {
        match self {
            Picker::Mode => ButtonAction::Mode(name.into()),
            Picker::Layer => ButtonAction::Layer(name.into()),
            Picker::Macro => ButtonAction::Macro { name: name.into(), repeat: false },
            Picker::Menu => ButtonAction::OpenMenu(name.into()),
            Picker::Info => ButtonAction::ShowInfo(name.into()),
            Picker::Log => ButtonAction::ShowLog(name.into()),
        }
    }

    /// The names it lists from the active game.
    pub fn names(self, config: &Config) -> Vec<String> {
        let game = config.active_game();
        match self {
            Picker::Mode => game.modes.iter().map(|l| l.name.clone()).chain([crate::config::MOUSE_MODE.to_string()]).collect(),
            Picker::Layer => game.layers.iter().map(|l| l.name.clone()).collect(),
            Picker::Macro => game.macros.iter().map(|m| m.name.clone()).collect(),
            Picker::Menu => game.menus.iter().map(|m| m.name.clone()).collect(),
            Picker::Info => game.info.iter().map(|i| i.name.clone()).collect(),
            Picker::Log => game.logs.iter().map(|l| l.name.clone()).collect(),
        }
    }
}

/// The simple actions offered, each with its label.
type Choice = (&'static str, fn() -> ButtonAction);

/// What a press sends: the ones a macro step may press too.
const PRESSES: [Choice; 10] = [
    ("Left click", || ButtonAction::Mouse(MouseButton::Left)),
    ("Right click", || ButtonAction::Mouse(MouseButton::Right)),
    ("Middle click", || ButtonAction::Mouse(MouseButton::Middle)),
    ("Back click", || ButtonAction::Mouse(MouseButton::Back)),
    ("Forward click", || ButtonAction::Mouse(MouseButton::Forward)),
    ("Wheel up", || ButtonAction::Wheel(WheelDirection::Up)),
    ("Wheel down", || ButtonAction::Wheel(WheelDirection::Down)),
    ("Wheel left", || ButtonAction::Wheel(WheelDirection::Left)),
    ("Wheel right", || ButtonAction::Wheel(WheelDirection::Right)),
    ("Disabled", || ButtonAction::Disabled),
];

/// The on-screen tools, the gyro's actions and the system's, after everything that presses or
/// opens something, in the order of the window's action picker.
const TOOLS: [Choice; 11] = [
    ("On-screen keyboard", || ButtonAction::ToggleOverlay),
    ("On-screen numpad", || ButtonAction::ToggleNumpad),
    ("Quick Settings", || ButtonAction::QuickSettings),
    ("Media controls", || ButtonAction::ToggleMedia),
    ("Gyro on / off", || ButtonAction::GyroToggle),
    ("Gyro clutch (off while held)", || ButtonAction::GyroClutch),
    ("Gyro on while held", || ButtonAction::GyroHold),
    ("Recenter gyro", || ButtonAction::GyroRecenter),
    ("Screenshot", || ButtonAction::Screenshot),
    ("Start or stop recording", || ButtonAction::ToggleRecording),
    ("Force quit (hold)", || ButtonAction::ForceQuit),
];

/// The rate a new turbo presses at, in presses a second.
const TURBO_RATE: f32 = 10.0;
const RATE: Number = Number::new(1.0, (1.0, 60.0), 0, " a second");
/// How often a turbo plays a macro, at the least (0: as often as the macro allows).
const EVERY: Number = Number::new(50.0, (0.0, 5_000.0), 0, " ms");

/// The action page of `at`.
pub(super) fn rows(config: &Config, at: ActionAt) -> Vec<Row> {
    let current = action(config, at);
    let mut rows = vec![Row::note(format!("Now: {}", current.as_ref().map_or("not set".into(), ButtonAction::summary))).keyword(current.as_ref().and_then(ButtonAction::keyword))];
    if let Some(current) = &current {
        rows.extend(wrapper_rows(at, current));
    }
    let step = matches!(at.slot, Slot::MacroStep(..));
    let choice = |&(label, make): &Choice| Row::apply(label, move |c| Act::changed_if(set_action(c, at, Some(make()))).then(Nav::Back(1)));
    // In the order of the window's action picker: what it presses, how it presses, what it
    // opens, then the tools, the gyro and the system.
    rows.push(Row::open("Keys", Page::KeyGroups(at)));
    rows.push(Row::open("Gamepad button", Page::Pad(at)));
    rows.extend(PRESSES.iter().map(choice));
    if let (Slot::Button(owner, b), true) = (at.slot, at.is_top()) {
        rows.push(Row::open("Gestures", Page::Gestures(owner, b)));
    }
    if !matches!(current, Some(ButtonAction::Multi(_))) {
        rows.push(Row::apply("Several at once", move |c| {
            let now = action(c, at).unwrap_or(ButtonAction::Disabled);
            Act::changed_if(set_action(c, at, Some(ButtonAction::Multi(vec![now])))).then(Nav::Open(Page::Multi(at)))
        }));
    }
    // A macro step presses what it holds, so it can't open anything. A menu item is a tap: it
    // can't hold a layer (it can switch modes).
    if !step {
        let item = matches!(at.slot, Slot::MenuItem(..));
        rows.extend(wrap_rows(at, current.as_ref(), item));
        for p in Picker::ALL.into_iter().filter(|p| !(item && *p == Picker::Layer)) {
            rows.push(Row::open(p.label(), Page::Pick(at, p)).keyword(Some(p.keyword())));
        }
        rows.extend(TOOLS.iter().map(choice));
    }
    if at.slot.unsettable() && at.is_top() {
        rows.push(Row::apply("Not set", move |c| Act::changed_if(set_action(c, at, None)).then(Nav::Back(1))));
    }
    rows
}

/// The settings of the wrapper or macro `current` is, first on its page.
fn wrapper_rows(at: ActionAt, current: &ButtonAction) -> Vec<Row> {
    let Some(inner) = at.child(0) else { return Vec::new() };
    match current {
        ButtonAction::Toggle(t) => vec![
            Row::open(format!("Toggles: {}", t.action.summary()), Page::Action(inner)),
            Row::toggle("Starts on", t.start_on, move |c, on| {
                edit(c, at, |a| match a {
                    ButtonAction::Toggle(t) => set(&mut t.start_on, on),
                    _ => false,
                })
            }),
            Row::apply("Stop toggling", move |c| unwrap(c, at)).tone(Tone::Remove),
        ],
        ButtonAction::Turbo { action: wrapped, rate, every_ms } => {
            let mut rows = vec![
                Row::open(format!("Turbo of: {}", wrapped.summary()), Page::Action(inner)),
                RATE.row("Rate", *rate, move |c, v| {
                    edit(c, at, |a| match a {
                        ButtonAction::Turbo { rate, .. } => set(rate, v),
                        _ => false,
                    })
                }),
            ];
            if matches!(**wrapped, ButtonAction::Macro { .. }) {
                rows.push(EVERY.row_u("Plays the macro every", *every_ms, move |c, v| {
                    edit(c, at, |a| match a {
                        ButtonAction::Turbo { every_ms, .. } => set(every_ms, v),
                        _ => false,
                    })
                }));
            }
            rows.push(Row::apply("Stop turbo", move |c| unwrap(c, at)).tone(Tone::Remove));
            rows
        }
        ButtonAction::Multi(parts) => vec![Row::open(format!("Several at once: {} actions", parts.len()), Page::Multi(at))],
        ButtonAction::Macro { repeat, .. } => vec![Row::toggle("Repeats while held", *repeat, move |c, on| {
            edit(c, at, |a| match a {
                ButtonAction::Macro { repeat, .. } => set(repeat, on),
                _ => false,
            })
        })],
        _ => Vec::new(),
    }
}

/// The rows that wrap the action in a Toggle or a Turbo, where it can be.
fn wrap_rows(at: ActionAt, current: Option<&ButtonAction>, item: bool) -> Vec<Row> {
    let mut rows = Vec::new();
    // A menu is put away with its close button, never toggled. A layer that stays on is a mode,
    // and a mode switch already switches back when pressed again.
    let toggleable = !matches!(current, None | Some(ButtonAction::Toggle(_) | ButtonAction::OpenMenu(_) | ButtonAction::Layer(_) | ButtonAction::Mode(_)));
    if toggleable {
        rows.push(Row::apply("Make it a toggle", move |c| wrap(c, at, |a| ButtonAction::Toggle(Toggled { action: Box::new(a), start_on: false }))).keyword(Some(Keyword::Toggle)));
    }
    // A turbo in a menu item would stop at once.
    if !item && !matches!(current, None | Some(ButtonAction::Turbo { .. })) {
        rows.push(Row::apply(format!("Make it turbo ({TURBO_RATE:.0} a second)"), move |c| wrap(c, at, |a| ButtonAction::Turbo { action: Box::new(a), rate: TURBO_RATE, every_ms: 0 })).keyword(Some(Keyword::Turbo)));
    }
    rows
}

/// Changes the action at `at` in place. True when `change` changed it.
fn edit(config: &mut Config, at: ActionAt, change: impl FnOnce(&mut ButtonAction) -> bool) -> bool {
    let Some(mut a) = action(config, at) else { return false };
    change(&mut a) && set_action(config, at, Some(a))
}

fn wrap(config: &mut Config, at: ActionAt, wrapper: impl FnOnce(ButtonAction) -> ButtonAction) -> Act {
    let Some(a) = action(config, at) else { return Act::stay() };
    Act::changed_if(set_action(config, at, Some(wrapper(a))))
}

/// Puts what a Toggle or Turbo wraps in its place.
fn unwrap(config: &mut Config, at: ActionAt) -> Act {
    let inner = at.child(0).and_then(|inner| action(config, inner));
    Act::changed_if(inner.is_some_and(|inner| set_action(config, at, Some(inner))))
}

/// The pad picker: each button, which the slot then sends. Back past the action page.
pub(super) fn pad_rows(at: ActionAt) -> Vec<Row> {
    Button::ALL
        .iter()
        .map(|&b| Row::apply(b.short_name(), move |c| Act::changed_if(set_action(c, at, Some(ButtonAction::Gamepad(b)))).then(Nav::Back(2))).buttons(vec![b]))
        .collect()
}

/// A picker of the game's modes, layers, macros, menus or overlays.
pub(super) fn pick_rows(config: &Config, at: ActionAt, p: Picker) -> Vec<Row> {
    let names = p.names(config);
    if names.is_empty() {
        return vec![Row::note(format!("The game has no {} yet", p.noun()))];
    }
    names
        .into_iter()
        .map(|name| {
            let made = p.action(&name);
            Row::apply(name, move |c| Act::changed_if(set_action(c, at, Some(made.clone()))).then(Nav::Back(2))).keyword(Some(p.keyword()))
        })
        .collect()
}

/// The parts of a Multi, each opening its own action page, then adding and removing parts.
pub(super) fn multi_rows(config: &Config, at: ActionAt) -> Vec<Row> {
    let parts = match action(config, at) {
        Some(ButtonAction::Multi(parts)) => parts,
        _ => Vec::new(),
    };
    let mut rows: Vec<Row> = parts
        .iter()
        .enumerate()
        .filter_map(|(i, part)| Some(Row::open(part.summary(), Page::Action(at.child(i)?)).keyword(part.keyword())))
        .collect();
    let len = parts.len();
    rows.push(Row::add("Add an action", move |c| {
        let added = edit(c, at, |a| match a {
            ButtonAction::Multi(parts) => {
                parts.push(ButtonAction::Disabled);
                true
            }
            _ => false,
        });
        match at.child(len) {
            Some(new) if added => Act::changed().then(Nav::Open(Page::Action(new))),
            _ => Act::changed_if(added),
        }
    }));
    if len > 1 {
        rows.push(Row::apply("Remove the last action", move |c| {
            Act::changed_if(edit(c, at, |a| match a {
                ButtonAction::Multi(parts) => parts.pop().is_some(),
                _ => false,
            }))
        }).tone(Tone::Remove));
    }
    rows.push(Row::apply("Keep only the first", move |c| {
        let first = match action(c, at) {
            Some(ButtonAction::Multi(parts)) => parts.into_iter().next().unwrap_or(ButtonAction::Disabled),
            _ => return Act::stay(),
        };
        Act::changed_if(set_action(c, at, Some(first))).then(Nav::Back(1))
    }).tone(Tone::Remove));
    rows
}

/// The rows of keys the key picker groups keys in: the full keyboard's rows.
fn key_groups() -> Vec<Vec<&'static crate::keyboard::Key>> {
    Board::Full.rows().iter().map(|row| row.iter().filter(|k| !k.code.is_empty()).collect::<Vec<_>>()).filter(|row| !row.is_empty()).collect()
}

/// The keys the slot's action presses now.
fn keys_of(config: &Config, at: ActionAt) -> Vec<String> {
    match action(config, at) {
        Some(ButtonAction::Keys(keys)) => keys,
        _ => Vec::new(),
    }
}

/// The key picker's first page: the keys pressed now, each row of the keyboard, and Done.
pub(super) fn key_group_rows(config: &Config, at: ActionAt) -> Vec<Row> {
    let pressed = keys_of(config, at);
    let now = if pressed.is_empty() { "none yet".into() } else { pressed.iter().map(|k| crate::keyboard::label(k)).collect::<Vec<_>>().join(" + ") };
    let mut rows = vec![Row::note(format!("Keys: {now}"))];
    for (g, keys) in key_groups().into_iter().enumerate() {
        let shown: Vec<&str> = keys.iter().take(6).map(|k| k.label).collect();
        let more = if keys.len() > 6 { " …" } else { "" };
        rows.push(Row::open(format!("{}{more}", shown.join(" ")), Page::Keys(at, g)));
    }
    if !pressed.is_empty() {
        rows.push(Row::apply("Clear the keys", move |c| Act::changed_if(set_action(c, at, Some(ButtonAction::Keys(Vec::new()))))).tone(Tone::Remove));
    }
    rows.push(Row::apply("Done", |_| Act::stay().then(Nav::Back(2))));
    rows
}

/// One row of the keyboard, each key ticked when the action presses it. A ticks or unticks.
pub(super) fn key_rows(config: &Config, at: ActionAt, group: usize) -> Vec<Row> {
    let pressed = keys_of(config, at);
    let Some(keys) = key_groups().into_iter().nth(group) else { return Vec::new() };
    keys.into_iter()
        .map(|key| {
            let on = pressed.iter().any(|k| k == key.code);
            let code = key.code;
            Row::apply(format!("[{}] {}", if on { "x" } else { " " }, crate::keyboard::label(code)), move |c| {
                let mut keys = keys_of(c, at);
                match keys.iter().position(|k| k == code) {
                    Some(at) => drop(keys.remove(at)),
                    None => keys.push(code.into()),
                }
                Act::changed_if(set_action(c, at, Some(ButtonAction::Keys(keys))))
            })
        })
        .collect()
}
