//! What a config must follow to be saved, and so to be loaded: names, references and the rules
//! each game, profile and item must follow. The settings window checks the same before saving.

use std::str::FromStr;

use crate::keycode::KeyCode;

use super::*;

/// A key as problems name it, without the `KEY_` prefix.
fn short_key(k: &str) -> &str {
    k.strip_prefix("KEY_").unwrap_or(k)
}

/// What's wrong with a menu item's action: the usual checks, plus which menus it may open
/// (see [`crate::menu::can_open`]).
pub fn item_problem(menu: &Menu, action: &ButtonAction, menus: &[&Menu], names: &Names) -> Option<String> {
    if uses_layer(action) {
        return Some("a menu item can't hold a layer: switch to a mode instead".into());
    }
    action_problem(action, names).or_else(|| {
        let mut problem = None;
        action.walk(&mut |a| {
            if let ButtonAction::OpenMenu(name) = a
                && problem.is_none()
            {
                if matches!(menu.kind, MenuKind::Radial { .. }) {
                    problem = Some("radial menus can't open other menus".to_string());
                } else if menu.cancel_button().is_none() {
                    problem = Some("give this menu a close button to open other menus from it".to_string());
                } else if let Some(child) = menus.iter().find(|m| &m.name == name)
                    && child.cancel_button() != menu.cancel_button()
                {
                    problem = Some(format!("“{name}” needs the same close button as this menu"));
                }
            }
        });
        problem
    })
}

/// Macro, menu, info overlay and layer names, for the "Macro…"/"Open menu…"/"Show info
/// overlay…"/"Layer…" pickers and for checking references.
#[derive(Debug, Clone, Default)]
pub struct Names {
    pub macros: Vec<String>,
    pub menus: Vec<String>,
    /// Info overlays an action can show: not those set to always show.
    pub infos: Vec<String>,
    /// Info overlays always on screen, which actions can't show.
    pub always_infos: Vec<String>,
    /// Log overlays an action can show, and those always on screen, likewise.
    pub logs: Vec<String>,
    pub always_logs: Vec<String>,
    pub layers: Vec<String>,
    pub modes: Vec<String>,
    /// How long each macro takes to play, in milliseconds, by name.
    pub macro_ms: std::collections::HashMap<String, u64>,
    /// Inside a turbo, where a macro plays once per press and never loops.
    pub in_turbo: bool,
}

impl Names {
    pub fn of(scope: &ScopeRef) -> Self {
        let list = |kind| scope.names(kind).into_iter().map(str::to_string).collect();
        let infos = |always: bool| scope.info.iter().filter(|o| o.always == always).map(|o| o.name.clone()).collect();
        let logs = |always: bool| scope.logs.iter().filter(|o| o.always == always).map(|o| o.name.clone()).collect();
        Names {
            macro_ms: scope.macros.iter().map(|m| (m.name.clone(), m.duration_ms())).collect(),
            macros: list(ItemKind::Macro),
            menus: list(ItemKind::Menu),
            infos: infos(false),
            always_infos: infos(true),
            logs: logs(false),
            always_logs: logs(true),
            layers: list(ItemKind::Layer),
            modes: list(ItemKind::Mode),
            in_turbo: false,
        }
    }

    /// What a game's profiles and items can use: its own items.
    pub fn for_game(config: &Config, game: &Game) -> Self {
        Names::of(&config.scope_of(game))
    }

    pub fn list(&self, kind: ItemKind) -> &[String] {
        match kind {
            ItemKind::Macro => &self.macros,
            ItemKind::Menu => &self.menus,
            ItemKind::Info => &self.infos,
            ItemKind::Log => &self.logs,
            ItemKind::Layer => &self.layers,
            ItemKind::Mode => &self.modes,
        }
    }
}

/// Whether `action` holds a layer anywhere in it, which a tap (a menu item) can't do.
pub fn uses_layer(action: &ButtonAction) -> bool {
    let mut layer = false;
    action.walk(&mut |a| layer |= matches!(a, ButtonAction::Layer(_)));
    layer
}

/// Why a Toggle can't hold `inner`, if it can't.
fn toggle_problem(inner: &ButtonAction) -> Option<&'static str> {
    match inner {
        ButtonAction::OpenMenu(_) => Some("a menu can't be toggled: its close button puts it away"),
        ButtonAction::Layer(_) => Some("a layer can't be toggled: to keep one on, make it a mode and switch to it"),
        ButtonAction::Mode(_) => Some("a mode switch can't be toggled: pressing it again already switches back"),
        _ => None,
    }
}

/// What's wrong with an action, if anything (the same things saving checks).
pub fn action_problem(action: &ButtonAction, names: &Names) -> Option<String> {
    let mut problem = None;
    action.walk(&mut |a| {
        if problem.is_some() {
            return;
        }
        match a {
            ButtonAction::Keys(keys) => {
                if let Some(bad) = keys.iter().find(|k| KeyCode::from_str(k).is_err()) {
                    problem = Some(format!("unknown key “{}”", short_key(bad)));
                }
            }
            ButtonAction::Macro { name, .. } if !names.macros.contains(name) => {
                problem = Some(format!("missing macro “{name}”"));
            }
            // A macro that grew longer than the turbo's gap would overlap its own presses.
            ButtonAction::Turbo { action, every_ms, .. } => {
                if let ButtonAction::Macro { name, .. } = &**action
                    && let Some(&macro_ms) = names.macro_ms.get(name)
                    && macro_ms > *every_ms
                {
                    problem = Some(format!(
                        "turbo of macro “{name}” presses every {every_ms} ms, but the macro takes {macro_ms} ms to play: raise the turbo's gap to at least {macro_ms} ms"
                    ));
                }
            }
            ButtonAction::Toggle(t) => problem = toggle_problem(&t.action).map(str::to_string),
            ButtonAction::OpenMenu(name) if !names.menus.contains(name) => {
                problem = Some(format!("missing menu “{name}”"));
            }
            ButtonAction::ShowInfo(name) if names.always_infos.contains(name) => {
                problem = Some(format!("info overlay “{name}” is always shown, so an action can't show it"));
            }
            ButtonAction::ShowInfo(name) if !names.infos.contains(name) => {
                problem = Some(format!("missing info overlay “{name}”"));
            }
            ButtonAction::ShowLog(name) if names.always_logs.contains(name) => {
                problem = Some(format!("log overlay “{name}” is always shown, so an action can't show it"));
            }
            ButtonAction::ShowLog(name) if !names.logs.contains(name) => {
                problem = Some(format!("missing log overlay “{name}”"));
            }
            ButtonAction::Layer(name) if !names.layers.contains(name) => {
                problem = Some(format!("missing layer “{name}”"));
            }
            ButtonAction::Mode(name) if !names.modes.contains(name) => {
                problem = Some(format!("missing mode “{name}”"));
            }
            _ => {}
        }
    });
    problem
}

/// Why combo `i` can't be saved, apart from its action, as its row says it.
pub fn combo_problem(combos: &[Combo], i: usize) -> Option<String> {
    let combo = combos.get(i)?;
    if combo.buttons.len() < 2 {
        return Some("needs at least two buttons".into());
    }
    let key = combo_key(&combo.buttons);
    combos[..i]
        .iter()
        .any(|c| combo_key(&c.buttons) == key)
        .then(|| "another combo has these buttons: give that one “Several at once…” instead".into())
}

/// The first thing saving would reject in a list of combos, apart from their actions.
pub fn combos_problem(combos: &[Combo]) -> Option<String> {
    if let Some(c) = combos.iter().find(|c| c.buttons.len() < 2) {
        return Some(format!("a combo needs at least two buttons (has {}).", c.buttons.len()));
    }
    let i = duplicate_combo(combos)?;
    let buttons = combos[i].buttons.iter().map(|b| b.short_name()).collect::<Vec<_>>().join(" + ");
    Some(format!("two combos are {buttons}. Give one of them “Several at once…” to do both things."))
}

/// The first thing saving would reject in a list of macros, menus and info overlays.
/// `names` is what they may refer to.
pub fn items_problem(macros: &[Macro], menus: &[Menu], info: &[InfoOverlay], names: &Names, reachable: &[&Menu]) -> Option<String> {
    for (i, m) in macros.iter().enumerate() {
        if m.name.trim().is_empty() {
            return Some("A macro needs a name.".into());
        }
        if macros[..i].iter().any(|o| o.name == m.name) {
            return Some(format!("Two macros are named “{}”.", m.name));
        }
        let keys = m.steps.iter().filter_map(MacroStep::action).flat_map(|a| a.key_names());
        if let Some(bad) = keys.into_iter().find(|k| KeyCode::from_str(k).is_err()) {
            return Some(format!("Macro “{}”: unknown key “{}”", m.name, short_key(bad)));
        }
        if m.steps.iter().filter_map(MacroStep::action).any(|a| {
            let mut shifts = false;
            a.walk(&mut |a| shifts |= matches!(a, ButtonAction::Layer(_) | ButtonAction::Mode(_)));
            shifts
        }) {
            return Some(format!("Macro “{}”: macros can't hold layers or switch modes.", m.name));
        }
        if let Some(problem) = m.steps.iter().filter_map(MacroStep::action).find_map(|a| action_problem(a, names)) {
            return Some(format!("Macro “{}”: {problem}", m.name));
        }
    }
    for (i, m) in menus.iter().enumerate() {
        if m.name.trim().is_empty() {
            return Some("A menu needs a name.".into());
        }
        if menus[..i].iter().any(|o| o.name == m.name) {
            return Some(format!("Two menus are named “{}”.", m.name));
        }
        if let Some(problem) = m.items.iter().find_map(|item| item_problem(m, &item.action, reachable, names)) {
            return Some(format!("Menu “{}”: {problem}", m.name));
        }
    }
    for (i, o) in info.iter().enumerate() {
        if o.name.trim().is_empty() {
            return Some("An info overlay needs a name.".into());
        }
        if info[..i].iter().any(|other| other.name == o.name) {
            return Some(format!("Two info overlays are named “{}”.", o.name));
        }
    }
    None
}

/// The first thing saving would reject in a list of log overlays: empty or repeated names.
pub fn logs_problem(logs: &[LogOverlay]) -> Option<String> {
    for (i, o) in logs.iter().enumerate() {
        if o.name.trim().is_empty() {
            return Some("A log overlay needs a name.".into());
        }
        if logs[..i].iter().any(|other| other.name == o.name) {
            return Some(format!("Two log overlays are named “{}”.", o.name));
        }
    }
    None
}

/// The first thing saving would reject in a game (or General).
pub fn game_problem(config: &Config, g: &Game) -> Option<String> {
    if g.profiles.is_empty() {
        return Some("needs at least one profile.".into());
    }
    let names = Names::for_game(config, g);
    let reachable = config.scope_of(g).menus;
    if let Some(problem) = items_problem(&g.macros, &g.menus, &g.info, &names, &reachable) {
        return Some(problem);
    }
    if let Some(problem) = logs_problem(&g.logs) {
        return Some(problem);
    }
    for (i, p) in g.profiles.iter().enumerate() {
        if p.name.trim().is_empty() {
            return Some("A profile needs a name.".into());
        }
        if g.profiles[..i].iter().any(|o| o.name == p.name) {
            return Some(format!("Two profiles are named “{}”.", p.name));
        }
        if let Some(problem) = p.actions().into_iter().find_map(|a| {
            action_problem(a, &names).filter(|problem| !problem.starts_with("unknown key"))
        }) {
            return Some(format!("Profile “{}”: {problem}.", p.name));
        }
        if let Some(problem) = combos_problem(&p.combos) {
            return Some(format!("Profile “{}”: {problem}", p.name));
        }
        let analogs = [
            Analog::Stick(Stick::Left),
            Analog::Stick(Stick::Right),
            Analog::Trigger(Trigger::Left),
            Analog::Trigger(Trigger::Right),
        ];
        if analogs.into_iter().any(|a| p.zones(a).iter().any(|z| z.min >= z.max)) {
            return Some(format!("Profile “{}”: a zone's range must start below where it ends.", p.name));
        }
        if [Stick::Left, Stick::Right].into_iter().all(|s| matches!(p.stick(s).action, StickAction::Flick { .. })) {
            return Some(format!("Profile “{}”: only one stick can be a flick stick.", p.name));
        }
        let mut keys: Vec<&String> = p.actions().into_iter().flat_map(|a| a.key_names()).collect();
        for s in [Stick::Left, Stick::Right] {
            keys.extend(p.stick(s).action.ring_actions().iter().flat_map(|a| a.key_names()));
        }
        if let Some(bad) = keys.iter().find(|k| KeyCode::from_str(k).is_err()) {
            return Some(format!("Profile “{}”: unknown key “{}”", p.name, short_key(bad)));
        }
    }
    modes_problem(g, &names).or_else(|| layers_problem(g, &names)).or_else(|| rules_problem(g))
}

/// The first thing saving would reject in a game's layers.
pub fn layers_problem(g: &Game, names: &Names) -> Option<String> {
    shifts_problem(&g.layers, ItemKind::Layer, names)
}

/// The first thing saving would reject in a game's modes.
pub fn modes_problem(g: &Game, names: &Names) -> Option<String> {
    shifts_problem(&g.modes, ItemKind::Mode, names)
}

/// The first thing saving would reject in a game's layers or modes (`kind` says which).
fn shifts_problem(list: &[Layer], kind: ItemKind, names: &Names) -> Option<String> {
    use crate::config::Indicator;
    let noun = kind.noun();
    for (i, l) in list.iter().enumerate() {
        if l.name.trim().is_empty() {
            return Some(format!("A {noun} needs a name."));
        }
        if list[..i].iter().any(|o| o.name == l.name) {
            return Some(format!("Two {noun}s are named “{}”.", l.name));
        }
        let mut label = format!("{noun} “{}”", l.name);
        label[..1].make_ascii_uppercase();
        if let Indicator::Info(info) = &l.indicator
            && !names.infos.contains(info)
        {
            return Some(if names.always_infos.contains(info) {
                format!("{label}: info overlay “{info}” is always shown, so it can't show that the {noun} is on.")
            } else {
                format!("{label}: missing info overlay “{info}” to show.")
            });
        }
        if let Some(problem) = l.actions().into_iter().find_map(|a| action_problem(a, names)) {
            return Some(format!("{label}: {problem}."));
        }
        if let Some(problem) = combos_problem(&l.combos) {
            return Some(format!("{label}: {problem}"));
        }
        let zones = [&l.left_stick, &l.right_stick].into_iter().flatten().flat_map(|s| &s.zones);
        let zones = zones.chain([&l.left_trigger, &l.right_trigger].into_iter().flatten().flat_map(|t| &t.zones));
        if zones.into_iter().any(|z| z.min >= z.max) {
            return Some(format!("{label}: a zone's range must start below where it ends."));
        }
        if [&l.left_stick, &l.right_stick].into_iter().all(|s| s.as_ref().is_some_and(|s| matches!(s.action, StickAction::Flick { .. }))) {
            return Some(format!("{label}: only one stick can be a flick stick."));
        }
        let stick_keys = [&l.left_stick, &l.right_stick]
            .into_iter()
            .flatten()
            .flat_map(|s| s.action.ring_actions().iter().flat_map(|a| a.key_names()));
        if let Some(bad) = stick_keys.into_iter().find(|k| KeyCode::from_str(k).is_err()) {
            return Some(format!("{label}: unknown key “{}”", short_key(bad)));
        }
    }
    None
}

/// What's wrong with a game's auto-switch rules, if anything.
pub fn rules_problem(g: &Game) -> Option<String> {
    if let Some(r) = g.rules.iter().find(|r| r.value.trim().is_empty()) {
        return Some(format!("A rule for profile “{}” has no {} to match.", r.profile, r.kind));
    }
    if let Some(r) = g.rules.iter().find(|r| g.profile(&r.profile).is_none()) {
        return Some(format!("A rule points to a profile that doesn't exist: “{}”.", r.profile));
    }
    None
}

impl Config {
    /// The first thing that keeps this config from being saved or loaded, if anything.
    pub fn problem(&self) -> Option<String> {
        for (i, g) in self.games.iter().enumerate() {
            if g.name.trim().is_empty() {
                return Some("A setup needs a name.".into());
            }
            if self.games[..i].iter().any(|o| o.name == g.name) {
                return Some(format!("Two setups are named “{}”.", g.name));
            }
        }
        for (key, g) in self.all_games() {
            let label = key.unwrap_or("General");
            if let Some(problem) = game_problem(self, g) {
                return Some(format!("{label}: {problem}"));
            }
        }
        if let Some(d) = self.auto_switch.default_profile.as_ref().filter(|d| self.profile(d).is_none()) {
            return Some(format!("The setup's default profile {d} no longer exists."));
        }
        None
    }
}
