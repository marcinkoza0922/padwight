//! Where an action is kept, and how the editor reads and changes it. A [`Slot`] is a place that
//! holds an action (a button, a gesture, a zone, a menu item…); an [`ActionAt`] adds a path into
//! the actions nested in it (the parts of a Multi, what a Toggle or Turbo wraps).

use crate::config::{Analog, Button, ButtonAction, Config, GestureKind, Stick, TouchSlot, Trigger, TriggerAction};

use super::owner::{self, Owner, game_mut};

/// A place that holds an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Button(Owner, Button),
    Gesture(Owner, Button, GestureKind),
    Combo(Owner, usize),
    /// A trigger pulled past its press point.
    Trigger(Owner, Trigger),
    /// A button-acting trigger's On press or On release.
    TriggerGesture(Owner, Trigger, GestureKind),
    /// A stick's or trigger's zone.
    Zone(Owner, Analog, usize),
    /// A sector of a stick's ring.
    Ring(Owner, Stick, usize),
    /// The touchpad's click, or a swipe.
    Touch(Owner, TouchSlot),
    /// An item of one of the game's menus: the menu, then the item.
    MenuItem(usize, usize),
    /// What a tap, press or release step of a macro presses: the macro, then the step.
    MacroStep(usize, usize),
}

impl Slot {
    /// Whether it can be left unset (a gesture, or a button a mode or layer leaves alone),
    /// rather than doing nothing.
    pub fn unsettable(self) -> bool {
        matches!(self, Slot::Gesture(..) | Slot::TriggerGesture(..) | Slot::Button(Owner::Shift(..), _))
    }
}

/// The label a new menu item has until its action names it.
pub const NEW_ITEM: &str = "New item";

/// The deepest an action is followed into the actions nested in it.
const MAX_DEPTH: usize = 4;

/// An action: its slot, and the way down to it through the Multis, Toggles and Turbos above it.
/// Each step of the path is a part of a Multi; a Toggle or Turbo only has its one inner action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionAt {
    pub slot: Slot,
    path: [u8; MAX_DEPTH],
    depth: u8,
}

impl ActionAt {
    pub fn new(slot: Slot) -> Self {
        ActionAt { slot, path: [0; MAX_DEPTH], depth: 0 }
    }

    /// The action nested in this one at `part` (a Multi's part; 0 for what a Toggle or Turbo wraps).
    pub fn child(self, part: usize) -> Option<Self> {
        let depth = usize::from(self.depth);
        if depth >= MAX_DEPTH {
            return None;
        }
        let mut path = self.path;
        path[depth] = u8::try_from(part).ok()?;
        Some(ActionAt { path, depth: self.depth + 1, ..self })
    }

    /// Whether it is the slot's own action, not one nested in it.
    pub fn is_top(self) -> bool {
        self.depth == 0
    }

    fn path(&self) -> &[u8] {
        &self.path[..usize::from(self.depth)]
    }
}

/// The action a slot holds now; `None` when it is unset.
pub fn slot_action(config: &Config, slot: Slot) -> Option<ButtonAction> {
    match slot {
        Slot::Button(o, b) => owner::button(config, o, b),
        Slot::Gesture(o, b, kind) => owner::gestures(config, o)?.get(&b)?.get(kind).cloned(),
        Slot::Combo(o, i) => owner::combos(config, o).get(i).map(|c| c.action.clone()),
        Slot::Trigger(o, t) => match &owner::trigger(config, o, t)?.action {
            TriggerAction::Button { action, .. } => Some(action.clone()),
            _ => None,
        },
        Slot::TriggerGesture(o, t, kind) => owner::trigger(config, o, t)?.gestures.get(kind).cloned(),
        Slot::Zone(o, a, i) => owner::zones(config, o, a).get(i).map(|z| z.action.clone()),
        Slot::Ring(o, s, i) => owner::stick(config, o, s)?.action.ring_actions().get(i).cloned(),
        Slot::Touch(o, t) => owner::touchpad(config, o)?.slot(t).cloned(),
        Slot::MenuItem(m, i) => config.active_game().menus.get(m)?.items.get(i).map(|it| it.action.clone()),
        Slot::MacroStep(m, i) => config.active_game().macros.get(m)?.steps.get(i)?.action().cloned(),
    }
}

/// Sets what a slot holds. `None` unsets a slot that can be unset; others become Disabled.
pub fn set_slot_action(config: &mut Config, slot: Slot, action: Option<ButtonAction>) -> bool {
    let before = slot_action(config, slot);
    let action = if slot.unsettable() { action } else { Some(action.unwrap_or(ButtonAction::Disabled)) };
    if before == action {
        return false;
    }
    match slot {
        Slot::Button(o, b) => return owner::set_button(config, o, b, action),
        Slot::Gesture(o, b, kind) => {
            let Some(gestures) = owner::gestures_mut(config, o) else { return false };
            let entry = gestures.entry(b).or_default();
            *entry.slot(kind) = action;
            if entry.is_empty() {
                gestures.remove(&b);
            }
        }
        Slot::TriggerGesture(o, t, kind) => {
            let Some(cfg) = owner::trigger_mut(config, o, t) else { return false };
            *cfg.gestures.slot(kind) = action;
        }
        Slot::MenuItem(m, i) => {
            let Some(item) = game_mut(config).and_then(|g| g.menus.get_mut(m)).and_then(|menu| menu.items.get_mut(i)) else { return false };
            let action = action.unwrap_or(ButtonAction::Disabled);
            // An item not named yet, or still named for its old action, is named for the new one.
            if item.label.is_empty() || item.label == NEW_ITEM || item.label == item.action.summary() {
                item.label = action.summary();
            }
            item.action = action;
        }
        _ => {
            let Some(place) = slot_mut(config, slot) else { return false };
            *place = action.unwrap_or(ButtonAction::Disabled);
        }
    }
    true
}

/// The action-holding place of a slot that always has one.
fn slot_mut(config: &mut Config, slot: Slot) -> Option<&mut ButtonAction> {
    match slot {
        Slot::Button(..) | Slot::Gesture(..) | Slot::TriggerGesture(..) => None,
        Slot::Combo(o, i) => owner::combos_mut(config, o)?.get_mut(i).map(|c| &mut c.action),
        Slot::Trigger(o, t) => match &mut owner::trigger_mut(config, o, t)?.action {
            TriggerAction::Button { action, .. } => Some(action),
            _ => None,
        },
        Slot::Zone(o, a, i) => owner::zones_mut(config, o, a)?.get_mut(i).map(|z| &mut z.action),
        Slot::Ring(o, s, i) => owner::stick_mut(config, o, s)?.action.ring_actions_mut().get_mut(i),
        Slot::Touch(o, t) => owner::touchpad_mut(config, o)?.slot_mut(t),
        Slot::MenuItem(m, i) => game_mut(config)?.menus.get_mut(m)?.items.get_mut(i).map(|it| &mut it.action),
        Slot::MacroStep(m, i) => game_mut(config)?.macros.get_mut(m)?.steps.get_mut(i)?.action_mut(),
    }
}

/// The action at `at` now; `None` when its slot is unset or the path leads nowhere.
pub fn action(config: &Config, at: ActionAt) -> Option<ButtonAction> {
    let mut action = slot_action(config, at.slot)?;
    for &part in at.path() {
        action = nested(&action, usize::from(part))?.clone();
    }
    Some(action)
}

/// Sets the action at `at`. `None` unsets the slot itself; a nested action can't be unset (a
/// Multi's part is removed on the Multi's page instead).
pub fn set_action(config: &mut Config, at: ActionAt, new: Option<ButtonAction>) -> bool {
    if at.is_top() {
        return set_slot_action(config, at.slot, new);
    }
    let (Some(mut root), Some(new)) = (slot_action(config, at.slot), new) else { return false };
    let mut place = &mut root;
    for &part in at.path() {
        let Some(inner) = nested_mut(place, usize::from(part)) else { return false };
        place = inner;
    }
    if *place == new {
        return false;
    }
    *place = new;
    set_slot_action(config, at.slot, Some(root))
}

/// The action nested in `action` at `part`.
fn nested(action: &ButtonAction, part: usize) -> Option<&ButtonAction> {
    match action {
        ButtonAction::Multi(parts) => parts.get(part),
        ButtonAction::Toggle(t) => Some(&t.action),
        ButtonAction::Turbo { action, .. } => Some(action),
        _ => None,
    }
}

fn nested_mut(action: &mut ButtonAction, part: usize) -> Option<&mut ButtonAction> {
    match action {
        ButtonAction::Multi(parts) => parts.get_mut(part),
        ButtonAction::Toggle(t) => Some(&mut t.action),
        ButtonAction::Turbo { action, .. } => Some(action),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Toggled;

    #[test]
    fn a_nested_action_is_read_and_changed_in_place() {
        let mut config = Config::default();
        let slot = Slot::Button(Owner::Profile, Button::South);
        let multi = ButtonAction::Multi(vec![ButtonAction::Screenshot, ButtonAction::toggle(ButtonAction::ToggleMedia)]);
        assert!(set_slot_action(&mut config, slot, Some(multi)));
        let inner = ActionAt::new(slot).child(1).unwrap().child(0).unwrap();
        assert_eq!(action(&config, inner), Some(ButtonAction::ToggleMedia));
        assert!(set_action(&mut config, inner, Some(ButtonAction::ToggleNumpad)));
        let ButtonAction::Multi(parts) = slot_action(&config, slot).unwrap() else { panic!("not a multi") };
        assert_eq!(parts[1], ButtonAction::Toggle(Toggled { action: Box::new(ButtonAction::ToggleNumpad), start_on: false }));
        assert!(!set_action(&mut config, inner, None), "a nested action can't be unset");
    }

    #[test]
    fn only_gestures_and_layer_buttons_can_be_unset() {
        let mut config = Config::default();
        let gesture = Slot::Gesture(Owner::Profile, Button::South, GestureKind::LongPress);
        assert!(set_slot_action(&mut config, gesture, Some(ButtonAction::Screenshot)));
        assert!(set_slot_action(&mut config, gesture, None));
        assert!(config.active().unwrap().gestures.is_empty());
        let button = Slot::Button(Owner::Profile, Button::South);
        set_slot_action(&mut config, button, None);
        assert_eq!(slot_action(&config, button), Some(ButtonAction::Disabled));
    }
}
