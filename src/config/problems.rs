//! What a config must follow to be saved, and so to be loaded: names, references and the rules
//! each game, profile and item must follow. The settings window checks the same before saving.

use std::borrow::Cow;
use std::collections::BTreeSet;
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
    pub cycles: Vec<String>,
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
    /// Inside a toggle, where a macro always loops until the toggle turns it off.
    pub in_toggle: bool,
    /// The glyphs of the controller games see, for pickers that show controller buttons. Left at
    /// the default (Xbox) outside the settings window.
    pub glyphs: crate::info::Glyphs,
    /// Set where gyro is disabled: the input being checked can't hold a gyro action, nor open
    /// anything in `gyro_items`.
    pub no_gyro: bool,
    /// The macros, cycles, menus, layers and modes that use gyro, directly or through others they
    /// use. A layer or mode that sets its own gyro supplies it, so it isn't one.
    pub gyro_items: BTreeSet<(ItemKind, String)>,
}

impl Names {
    pub fn of(scope: &ScopeRef) -> Self {
        let list = |kind| scope.names(kind).into_iter().map(str::to_string).collect();
        let infos = |always: bool| scope.info.iter().filter(|o| o.always == always).map(|o| o.name.clone()).collect();
        let logs = |always: bool| scope.logs.iter().filter(|o| o.always == always).map(|o| o.name.clone()).collect();
        Names {
            macro_ms: scope.macros.iter().map(|m| (m.name.clone(), m.duration_ms())).collect(),
            macros: list(ItemKind::Macro),
            cycles: list(ItemKind::Cycle),
            menus: list(ItemKind::Menu),
            infos: infos(false),
            always_infos: infos(true),
            logs: logs(false),
            always_logs: logs(true),
            layers: list(ItemKind::Layer),
            modes: list(ItemKind::Mode),
            in_turbo: false,
            in_toggle: false,
            glyphs: crate::info::Glyphs::default(),
            no_gyro: false,
            gyro_items: gyro_items(scope),
        }
    }

    /// These names for inputs whose gyro is `enabled` or not.
    pub fn for_gyro(&self, enabled: bool) -> Cow<'_, Names> {
        if self.no_gyro == !enabled {
            return Cow::Borrowed(self);
        }
        Cow::Owned(Names { no_gyro: !enabled, ..self.clone() })
    }

    /// Whether the item `kind` called `name` uses gyro.
    pub fn uses_gyro(&self, kind: ItemKind, name: &str) -> bool {
        self.gyro_items.contains(&(kind, name.to_string()))
    }

    /// What a game's profiles and items can use: its own items.
    pub fn for_game(config: &Config, game: &Game) -> Self {
        Names::of(&config.scope_of(game))
    }

    pub fn list(&self, kind: ItemKind) -> &[String] {
        match kind {
            ItemKind::Macro => &self.macros,
            ItemKind::Cycle => &self.cycles,
            ItemKind::Menu => &self.menus,
            ItemKind::Info => &self.infos,
            ItemKind::Log => &self.logs,
            ItemKind::Layer => &self.layers,
            ItemKind::Mode => &self.modes,
        }
    }
}

/// The items of a scope that use gyro: those with a gyro action, and those using one that does
/// (a menu item opening a menu with one, a macro playing one, and so on).
fn gyro_items(scope: &ScopeRef) -> BTreeSet<(ItemKind, String)> {
    let mut items: Vec<((ItemKind, String), Vec<&ButtonAction>)> = Vec::new();
    items.extend(scope.macros.iter().map(|m| ((ItemKind::Macro, m.name.clone()), m.steps.iter().filter_map(MacroStep::action).collect())));
    items.extend(scope.cycles.iter().map(|c| ((ItemKind::Cycle, c.name.clone()), c.steps.iter().collect())));
    items.extend(scope.menus.iter().map(|m| ((ItemKind::Menu, m.name.clone()), m.items.iter().map(|i| &i.action).collect())));
    items.extend(scope.layers.iter().filter(|l| l.gyro.is_none()).map(|l| ((ItemKind::Layer, l.name.clone()), l.actions())));
    items.extend(scope.modes.iter().filter(|l| l.gyro.is_none()).map(|l| ((ItemKind::Mode, l.name.clone()), l.actions())));
    let mut found = BTreeSet::new();
    loop {
        let before = found.len();
        for (item, actions) in &items {
            let mut hit = false;
            actions.iter().for_each(|a| {
                a.walk(&mut |a| {
                    hit |= a.is_gyro() || ItemKind::ALL.into_iter().any(|k| k.name_in(a).is_some_and(|n| found.contains(&(k, n.clone()))));
                })
            });
            if hit {
                found.insert(item.clone());
            }
        }
        if found.len() == before {
            return found;
        }
    }
}

/// Why `action` can't be on an input whose gyro is disabled, if it can't: it is a gyro action, or
/// opens or plays something that has one.
pub fn gyro_problem(action: &ButtonAction, names: &Names) -> Option<String> {
    if !names.no_gyro {
        return None;
    }
    let mut problem = None;
    action.walk(&mut |a| {
        if problem.is_some() {
            return;
        }
        if a.is_gyro() {
            problem = Some("a gyro action needs gyro, which is off here: tick Enabled on the Gyro tab, or take the action off".to_string());
        }
        for kind in ItemKind::ALL {
            if let Some(name) = kind.name_in(a)
                && names.uses_gyro(kind, name)
            {
                problem = Some(format!("{} “{name}” uses gyro, which is off here: tick Enabled on the Gyro tab, or use something else", kind.noun()));
            }
        }
    });
    problem
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
        ButtonAction::Cycle { .. } => Some("a cycle can't be toggled: each press already moves it on"),
        _ => None,
    }
}

/// What's wrong with an action, if anything (the same things saving checks).
pub fn action_problem(action: &ButtonAction, names: &Names) -> Option<String> {
    if let Some(problem) = gyro_problem(action, names) {
        return Some(problem);
    }
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
            ButtonAction::Cycle { name, .. } if !names.cycles.contains(name) => {
                problem = Some(format!("missing cycle “{name}”"));
            }
            // A macro that grew longer than the turbo's gap would overlap its own presses.
            ButtonAction::Turbo { action, every_ms, .. } => {
                if let ButtonAction::Macro { name, .. } = &**action
                    && let Some(&macro_ms) = names.macro_ms.get(name)
                    && macro_ms > *every_ms
                {
                    problem = Some(format!(
                        "turbo of macro “{name}” presses every {every_ms} ms, but the macro takes {macro_ms} ms: raise the gap to at least {macro_ms} ms"
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

/// Why a touchpad can't be set up as it is where gyro is disabled: it lays out a menu that uses gyro.
pub fn touchpad_gyro_problem(touchpad: &TouchpadConfig, names: &Names) -> Option<String> {
    if !names.no_gyro {
        return None;
    }
    let menu = touchpad.menus().find(|m| names.uses_gyro(ItemKind::Menu, m))?;
    Some(format!("the touchpad's menu “{menu}” uses gyro, which is off here: tick Enabled on the Gyro tab, or use something else"))
}

/// A warning, not a refusal: the menu has gyro actions, but gyro is off in every profile of the
/// game, so they can never do anything.
pub fn menu_gyro_warning(game: &Game, names: &Names, menu: &Menu) -> Option<String> {
    (names.uses_gyro(ItemKind::Menu, &menu.name) && !game.profiles.iter().any(|p| p.gyro.enabled))
        .then(|| "uses gyro, but gyro is off in every profile here, so those items do nothing".to_string())
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

/// What's wrong with a cycle's step, if anything: one that isn't an input, a macro or several
/// of those at once, or what's wrong with its action.
pub fn cycle_step_problem(step: &ButtonAction, names: &Names) -> Option<String> {
    if !crate::config::fits_cycle(step) {
        return Some("a step can only press inputs or play macros".into());
    }
    action_problem(step, names)
}

/// The first thing saving would reject in a list of cycles: empty or repeated names, and steps
/// that can't be.
pub fn cycles_problem(cycles: &[Cycle], names: &Names) -> Option<String> {
    for (i, c) in cycles.iter().enumerate() {
        if c.name.trim().is_empty() {
            return Some("A cycle needs a name.".into());
        }
        if cycles[..i].iter().any(|o| o.name == c.name) {
            return Some(format!("Two cycles are named “{}”.", c.name));
        }
        if let Some(problem) = c.steps.iter().find_map(|s| cycle_step_problem(s, names)) {
            return Some(format!("Cycle “{}”: {problem}.", c.name));
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
    if let Some(problem) = logs_problem(&g.logs).or_else(|| cycles_problem(&g.cycles, &names)) {
        return Some(problem);
    }
    for (i, p) in g.profiles.iter().enumerate() {
        if p.name.trim().is_empty() {
            return Some("A profile needs a name.".into());
        }
        if g.profiles[..i].iter().any(|o| o.name == p.name) {
            return Some(format!("Two profiles are named “{}”.", p.name));
        }
        let names = &*names.for_gyro(p.gyro.enabled);
        if let Some(problem) = p.actions().into_iter().find_map(|a| {
            action_problem(a, names).filter(|problem| !problem.starts_with("unknown key"))
        }) {
            return Some(format!("Profile “{}”: {problem}.", p.name));
        }
        if let Some(problem) = touchpad_gyro_problem(&p.touchpad, names) {
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
        let names = &*names.for_gyro(l.gyro.as_ref().is_none_or(|g| g.enabled));
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
