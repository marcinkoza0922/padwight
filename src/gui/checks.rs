//! What saving checks: names, references and the rules each game, profile and item must follow.

use super::*;

/// What's wrong with a menu item's action: the usual checks, plus which menus it may open
/// (see [`crate::menu::can_open`]).
pub(super) fn item_problem(menu: &Menu, action: &ButtonAction, menus: &[&Menu], names: &Names) -> Option<String> {
    if holds_layer(action) {
        return Some("a menu item can only toggle a layer (wrap it in Toggle)".into());
    }
    action_problem(action, names).or_else(|| {
        let mut problem = None;
        action.walk(&mut |a| {
            if let ButtonAction::OpenMenu(name) = a
                && problem.is_none()
            {
                if matches!(menu.kind, MenuKind::Radial { .. }) {
                    problem = Some("radial menus can't open other menus".to_string());
                } else if let Some(child) = menus.iter().find(|m| &m.name == name)
                    && child.kind.tag() != menu.kind.tag()
                {
                    problem = Some(format!("“{name}” isn't a {} menu", menu.kind.tag().short().to_lowercase()));
                }
            }
        });
        problem
    })
}

/// The buttons that open menu `name` in `game`, one list per binding: a button or its gesture
/// opens it alone, a combo with all its buttons. A submenu is opened by whatever opens the menus
/// it's in, since their opener still holds it up.
pub(super) fn menu_openers(game: &Game, name: &str) -> Vec<Vec<Button>> {
    let mut openers = Vec::new();
    let mut seen = vec![name.to_string()];
    let mut queue = vec![name.to_string()];
    let opens = |action: &ButtonAction, target: &str| {
        let mut found = false;
        action.walk(&mut |a| found |= matches!(a, ButtonAction::OpenMenu(n) if n == target));
        found
    };
    while let Some(target) = queue.pop() {
        let buttons = game.profiles.iter().flat_map(|p| p.buttons.iter()).chain(game.layers.iter().flat_map(|l| l.buttons.iter()));
        let gestures = game.profiles.iter().flat_map(|p| p.gestures.iter()).chain(game.layers.iter().flat_map(|l| l.gestures.iter()));
        let combos = game.profiles.iter().flat_map(|p| p.combos.iter()).chain(game.layers.iter().flat_map(|l| l.combos.iter()));
        openers.extend(buttons.filter(|(_, a)| opens(a, &target)).map(|(b, _)| vec![*b]));
        openers.extend(
            gestures.filter(|(_, g)| GestureKind::ALL.into_iter().filter_map(|k| g.get(k)).any(|a| opens(a, &target))).map(|(b, _)| vec![*b]),
        );
        openers.extend(combos.filter(|c| opens(&c.action, &target)).map(|c| c.buttons.clone()));
        for parent in game.menus.iter().filter(|m| m.items.iter().any(|item| opens(&item.action, &target))) {
            if !seen.contains(&parent.name) {
                seen.push(parent.name.clone());
                queue.push(parent.name.clone());
            }
        }
    }
    openers.sort();
    openers.dedup();
    openers
}

/// Why item `i` of a button menu can't be picked, if it can't: it has no button, an earlier item
/// has its button, or its button is part of what opens the menu (pressing it is holding the
/// menu up). `openers` are [`menu_openers`]' lists. Saving still works; this is a warning.
pub(super) fn button_item_warning(menu: &Menu, i: usize, openers: &[Vec<Button>]) -> Option<String> {
    if !matches!(menu.kind, MenuKind::Buttons) {
        return None;
    }
    let Some(b) = menu.items.get(i)?.button else {
        return Some("no button, so it can't be picked".into());
    };
    let badge = crate::menu::button_badge(b);
    if let Some(first) = menu.items[..i].iter().position(|item| item.button == Some(b)) {
        return Some(format!("item {} also uses {badge}; only the first is picked", first + 1));
    }
    if openers.iter().any(|o| o.contains(&b)) {
        return Some(format!("{badge} opens this menu, so pressing it can't pick this item"));
    }
    None
}

pub(super) fn info_has_problem(info: &[InfoOverlay]) -> bool {
    info.iter().enumerate().any(|(i, o)| o.name.trim().is_empty() || info[..i].iter().any(|other| other.name == o.name))
}

pub(super) fn menus_have_problem(menus: &[Menu], reachable: &[&Menu], names: &Names) -> bool {
    menus.iter().enumerate().any(|(i, m)| {
        m.name.trim().is_empty()
            || menus[..i].iter().any(|o| o.name == m.name)
            || m.items.iter().any(|item| item_problem(m, &item.action, reachable, names).is_some())
    })
}

/// Macro, menu, info overlay and layer names, for the "Macro…"/"Open menu…"/"Show info
/// overlay…"/"Layer…" pickers and for checking references.
#[derive(Debug, Clone, Default)]
pub(super) struct Names {
    pub(super) macros: Vec<String>,
    pub(super) menus: Vec<String>,
    /// Info overlays an action can show: not those set to always show.
    pub(super) infos: Vec<String>,
    /// Info overlays always on screen, which actions can't show.
    pub(super) always_infos: Vec<String>,
    /// Log overlays an action can show, and those always on screen, likewise.
    pub(super) logs: Vec<String>,
    pub(super) always_logs: Vec<String>,
    pub(super) layers: Vec<String>,
    /// How long each macro takes to play, in milliseconds, by name.
    pub(super) macro_ms: std::collections::HashMap<String, u64>,
    /// Inside a turbo, where a macro plays once per press and never loops.
    pub(super) in_turbo: bool,
}

impl Names {
    pub(super) fn of(scope: &ScopeRef) -> Self {
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
            in_turbo: false,
        }
    }

    /// What a game's profiles and items can use: its own items.
    pub(super) fn for_game(config: &Config, game: &Game) -> Self {
        Names::of(&config.scope_of(game))
    }

    pub(super) fn list(&self, kind: ItemKind) -> &[String] {
        match kind {
            ItemKind::Macro => &self.macros,
            ItemKind::Menu => &self.menus,
            ItemKind::Info => &self.infos,
            ItemKind::Log => &self.logs,
            ItemKind::Layer => &self.layers,
        }
    }
}

/// Whether `action` holds a layer outside any Toggle, which a tap (a menu item) can't do.
pub(super) fn holds_layer(action: &ButtonAction) -> bool {
    match action {
        ButtonAction::Layer(_) => true,
        ButtonAction::Multi(list) => list.iter().any(holds_layer),
        ButtonAction::Turbo { action, .. } => holds_layer(action),
        _ => false,
    }
}

/// What's wrong with an action, if anything (the same things saving checks).
pub(super) fn action_problem(action: &ButtonAction, names: &Names) -> Option<String> {
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
            _ => {}
        }
    });
    problem
}

/// Whether a profile section contains anything saving would reject.
pub(super) fn section_has_problem(p: &Profile, tab: ProfileTab, names: &Names) -> bool {
    let bad = |a: &ButtonAction| action_problem(a, names).is_some();
    let button_bad = |b: Button| {
        bad(p.button(b)) || p.gestures.get(&b).is_some_and(|g| GestureKind::ALL.iter().any(|k| g.get(*k).is_some_and(bad)))
    };
    match tab {
        ProfileTab::Buttons => Button::ALL.into_iter().chain(Button::PADDLES).any(button_bad),
        ProfileTab::Sticks => {
            let zones = [Analog::Stick(Stick::Left), Analog::Stick(Stick::Right), Analog::Trigger(Trigger::Left), Analog::Trigger(Trigger::Right)]
                .into_iter()
                .any(|a| p.zones(a).iter().any(|z| z.min >= z.max || bad(&z.action)));
            let triggers = [Trigger::Left, Trigger::Right]
                .into_iter()
                .any(|t| matches!(p.trigger(t), TriggerAction::Button { action, .. } if bad(action)));
            let rings = [Stick::Left, Stick::Right].into_iter().any(|s| p.stick(s).action.ring_actions().iter().any(bad));
            let two_flicks = [Stick::Left, Stick::Right].into_iter().all(|s| matches!(p.stick(s).action, StickAction::Flick { .. }));
            zones || triggers || rings || two_flicks
        }
        ProfileTab::Combos => p.combos.iter().any(|c| c.buttons.len() < 2 || bad(&c.action)),
        ProfileTab::Touchpad => p.touchpad.actions().into_iter().any(bad) || super::touchpad::touchpad_problem(&p.touchpad, names).is_some(),
        ProfileTab::Gyro | ProfileTab::Guide => false,
    }
}

pub(super) fn macros_have_problem(macros: &[Macro], names: &Names) -> bool {
    macros.iter().enumerate().any(|(i, m)| {
        m.name.trim().is_empty()
            || macros[..i].iter().any(|o| o.name == m.name)
            || m.steps.iter().filter_map(MacroStep::action).any(|a| action_problem(a, names).is_some())
    })
}

/// The menus a game's items can open.
pub(super) fn reachable_menus<'a>(config: &'a Config, game: &'a Game) -> Vec<&'a Menu> {
    config.scope_of(game).menus
}

/// The first thing saving would reject in a list of macros, menus and info overlays.
/// `names` is what they may refer to.
pub(super) fn items_problem(macros: &[Macro], menus: &[Menu], info: &[InfoOverlay], names: &Names, reachable: &[&Menu]) -> Option<String> {
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
            let mut layer = false;
            a.walk(&mut |a| layer |= matches!(a, ButtonAction::Layer(_)));
            layer
        }) {
            return Some(format!("Macro “{}”: macros can't hold layers.", m.name));
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
pub(super) fn logs_problem(logs: &[LogOverlay]) -> Option<String> {
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
/// Where a saving problem is: a profile of a game, and the assignment in it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Place {
    /// The game (`None`: General).
    pub(super) game: Option<String>,
    pub(super) profile: usize,
    pub(super) target: Target,
}

/// The first assignment in a profile that saving rejects.
fn bad_target(p: &Profile, names: &Names) -> Option<Target> {
    let mut all: Vec<(Target, &ButtonAction)> = Button::ALL.into_iter().chain(Button::PADDLES).map(|b| (Target::Button(b), p.button(b))).collect();
    for (b, gestures) in &p.gestures {
        all.extend(GestureKind::ALL.into_iter().filter_map(|k| gestures.get(k).map(|a| (Target::Gesture(*b, k), a))));
    }
    for t in [Trigger::Left, Trigger::Right] {
        if let TriggerAction::Button { action, .. } = p.trigger(t) {
            all.push((Target::Trigger(t), action));
        }
    }
    all.extend(p.combos.iter().enumerate().map(|(i, c)| (Target::Combo(i), &c.action)));
    for a in [Analog::Stick(Stick::Left), Analog::Stick(Stick::Right), Analog::Trigger(Trigger::Left), Analog::Trigger(Trigger::Right)] {
        all.extend(p.zones(a).iter().enumerate().map(|(i, z)| (Target::Zone(a, i), &z.action)));
    }
    for s in [Stick::Left, Stick::Right] {
        all.extend(p.stick(s).action.ring_actions().iter().enumerate().map(|(i, a)| (Target::RingSector(s, i), a)));
    }
    all.extend(p.touchpad.slots().into_iter().filter_map(|slot| p.touchpad.slot(slot).map(|a| (Target::Touch(slot), a))));
    all.into_iter().find(|(_, a)| action_problem(a, names).is_some()).map(|(t, _)| t)
}

impl App {
    /// Where the first saving problem is, when it's an assignment in a profile.
    pub(super) fn problem_place(&self) -> Option<Place> {
        let config = &self.config;
        config.all_games().find_map(|(key, g)| {
            game_problem(config, g)?;
            let names = Names::for_game(config, g);
            g.profiles.iter().enumerate().find_map(|(i, p)| {
                bad_target(p, &names).map(|target| Place { game: key.map(str::to_string), profile: i, target })
            })
        })
    }
}

pub(super) fn game_problem(config: &Config, g: &Game) -> Option<String> {
    if g.profiles.is_empty() {
        return Some("needs at least one profile.".into());
    }
    let names = Names::for_game(config, g);
    let reachable = reachable_menus(config, g);
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
        if let Some(c) = p.combos.iter().find(|c| c.buttons.len() < 2) {
            return Some(format!("Profile “{}”: a combo needs at least two buttons (has {}).", p.name, c.buttons.len()));
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
    layers_problem(g, &names).or_else(|| rules_problem(g))
}

/// The first thing saving would reject in a game's layers.
pub(super) fn layers_problem(g: &Game, names: &Names) -> Option<String> {
    use crate::config::Indicator;
    for (i, l) in g.layers.iter().enumerate() {
        if l.name.trim().is_empty() {
            return Some("A layer needs a name.".into());
        }
        if g.layers[..i].iter().any(|o| o.name == l.name) {
            return Some(format!("Two layers are named “{}”.", l.name));
        }
        let label = format!("Layer “{}”", l.name);
        if let Indicator::Info(info) = &l.indicator
            && !names.infos.contains(info)
        {
            return Some(if names.always_infos.contains(info) {
                format!("{label}: info overlay “{info}” is always shown, so it can't show that the layer is on.")
            } else {
                format!("{label}: missing info overlay “{info}” to show.")
            });
        }
        if let Some(problem) = l.actions().into_iter().find_map(|a| action_problem(a, names)) {
            return Some(format!("{label}: {problem}."));
        }
        if let Some(c) = l.combos.iter().find(|c| c.buttons.len() < 2) {
            return Some(format!("{label}: a combo needs at least two buttons (has {}).", c.buttons.len()));
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
pub(super) fn rules_problem(g: &Game) -> Option<String> {
    if let Some(r) = g.rules.iter().find(|r| r.value.trim().is_empty()) {
        return Some(format!("A rule for profile “{}” has no {} to match.", r.profile, r.kind));
    }
    if let Some(r) = g.rules.iter().find(|r| g.profile(&r.profile).is_none()) {
        return Some(format!("A rule points to a profile that doesn't exist: “{}”.", r.profile));
    }
    None
}

impl App {
    pub(super) fn validate(&self) -> Option<String> {
        let config = &self.config;
        for (i, g) in config.games.iter().enumerate() {
            if g.name.trim().is_empty() {
                return Some("A setup needs a name.".into());
            }
            if config.games[..i].iter().any(|o| o.name == g.name) {
                return Some(format!("Two setups are named “{}”.", g.name));
            }
        }
        for (key, g) in config.all_games() {
            let label = key.unwrap_or("General");
            if let Some(problem) = game_problem(config, g) {
                return Some(format!("{label}: {problem}"));
            }
        }
        if let Some(d) = config.auto_switch.default_profile.as_ref().filter(|d| config.profile(d).is_none()) {
            return Some(format!("The setup's default profile {d} no longer exists."));
        }
        None
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_menu_items_nothing_can_pick_are_warned_about() {
        let item = |button| MenuItem { label: "x".into(), action: ButtonAction::Disabled, button, weight: 1.0 };
        let menu = |name: &str, items| Menu { name: name.into(), kind: MenuKind::Buttons, items, cancel: None, style: OverlayStyle::default() };
        let quick = menu("Quick", vec![item(Some(Button::LeftBumper)), item(Some(Button::South)), item(Some(Button::South)), item(None)]);
        let sub = menu("Sub", vec![item(Some(Button::RightBumper))]);
        let mut profile = Profile::passthrough("p");
        profile.set_button(Button::LeftBumper, ButtonAction::toggle(ButtonAction::OpenMenu("Quick".into())));
        profile.combos.push(Combo { buttons: vec![Button::Select, Button::RightBumper], action: ButtonAction::OpenMenu("Quick".into()) });
        let mut game = Game::new("g", vec![profile]);
        game.menus = vec![quick.clone(), sub.clone()];
        game.menus[0].items[1].action = ButtonAction::OpenMenu("Sub".into());

        let openers = menu_openers(&game, "Quick");
        assert_eq!(openers, [vec![Button::LeftBumper], vec![Button::Select, Button::RightBumper]]);
        assert!(button_item_warning(&quick, 0, &openers).is_some_and(|w| w.contains("opens this menu")));
        assert_eq!(button_item_warning(&quick, 1, &openers), None);
        assert!(button_item_warning(&quick, 2, &openers).is_some_and(|w| w.contains("item 2")));
        assert!(button_item_warning(&quick, 3, &openers).is_some_and(|w| w.contains("no button")));
        // A submenu is held up by its parent's openers too.
        assert!(button_item_warning(&sub, 0, &menu_openers(&game, "Sub")).is_some_and(|w| w.contains("RB opens")));
        // Other kinds of menu have no item buttons to warn about.
        let list = Menu { kind: MenuKind::List, ..quick };
        assert_eq!(button_item_warning(&list, 3, &openers), None);
    }

    #[test]
    fn problems_are_found_inside_nested_actions_and_flag_their_section() {
        let names = Names { macros: vec!["Known".to_string()], layers: vec!["Guide".to_string()], ..Names::default() };
        let nested = ButtonAction::Multi(vec![
            ButtonAction::Mouse(MouseButton::Left),
            ButtonAction::toggle(ButtonAction::Keys(vec!["KEY_NOPE".into()])),
        ]);
        assert_eq!(action_problem(&nested, &names).as_deref(), Some("unknown key \u{201c}NOPE\u{201d}"));
        let missing = ButtonAction::Macro { name: "Gone".into(), repeat: false };
        assert_eq!(action_problem(&missing, &names).as_deref(), Some("missing macro \u{201c}Gone\u{201d}"));
        assert_eq!(action_problem(&ButtonAction::Macro { name: "Known".into(), repeat: false }, &names), None);
        // A turbo's gap must cover its macro's length; a longer macro is caught here.
        let turbo = |every_ms| ButtonAction::Turbo {
            action: Box::new(ButtonAction::Macro { name: "Long".into(), repeat: false }),
            rate: 10.0,
            every_ms,
        };
        let long = Names { macros: vec!["Long".into()], macro_ms: [("Long".to_string(), 300)].into(), ..Names::default() };
        assert!(action_problem(&turbo(200), &long).is_some_and(|p| p.contains("raise the turbo's gap")));
        assert_eq!(action_problem(&turbo(300), &long), None);

        let mut p = Profile::passthrough("p");
        assert!(ProfileTab::ALL.iter().all(|t| !section_has_problem(&p, *t, &names)));
        p.set_button(Button::RightStickUp, missing.clone());
        assert!(!section_has_problem(&p, ProfileTab::Sticks, &names), "stick directions have no actions to check");
        assert!(!section_has_problem(&p, ProfileTab::Buttons, &names));
        // A button ring's sector does, and flags the Sticks tab.
        p.stick_mut(Stick::Right).action = StickAction::ring(8);
        let StickAction::Ring { actions, .. } = &mut p.stick_mut(Stick::Right).action else { unreachable!() };
        actions[0] = missing;
        assert!(section_has_problem(&p, ProfileTab::Sticks, &names), "ring sectors live on the Sticks tab");
        p.combos.push(Combo { buttons: vec![Button::South], action: ButtonAction::Disabled });
        assert!(section_has_problem(&p, ProfileTab::Combos, &names));
    }
}
