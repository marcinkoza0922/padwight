//! Where saving's problems show in the window: which section has one, where the first one is,
//! and the menu warnings saving doesn't refuse. The rules themselves are `config::problems`.

use super::*;
pub(super) use crate::config::problems::*;

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
        ProfileTab::Combos => combos_problem(&p.combos).is_some() || p.combos.iter().any(|c| bad(&c.action)),
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
    let action = all.into_iter().find(|(_, a)| action_problem(a, names).is_some()).map(|(t, _)| t);
    action.or_else(|| (0..p.combos.len()).find(|&i| combo_problem(&p.combos, i).is_some()).map(Target::Combo))
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


impl App {
    pub(super) fn validate(&self) -> Option<String> {
        self.config.problem()
    }
}

#[cfg(test)]
mod tests;
