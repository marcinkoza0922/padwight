//! Modes: a game's sets of overrides that are switched to, one at a time, rather than held like
//! layers. A mode is built like a layer (see [`Layer`]); with no mode on, the profile itself is
//! the default mode.

use super::{
    ButtonAction, Game, GuideInput, GuideSettings, Layer, Menu, Profile, RowId, Stick, Toggled, Trigger, built_in_mode, combo_key,
};

/// A mode is stored and applied exactly like a layer; only how it's turned on differs.
pub type Mode = Layer;

impl Layer {
    /// Whether it sets `input`: a button's action or gestures, a trigger, or a stick.
    pub fn sets(&self, input: GuideInput) -> bool {
        match input {
            GuideInput::Button(b) => self.buttons.contains_key(&b) || self.gestures.contains_key(&b),
            GuideInput::Trigger(Trigger::Left) => self.left_trigger.is_some(),
            GuideInput::Trigger(Trigger::Right) => self.right_trigger.is_some(),
            GuideInput::Stick(Stick::Left) => self.left_stick.is_some(),
            GuideInput::Stick(Stick::Right) => self.right_stick.is_some(),
        }
    }

    /// Whether it adds, replaces or switches off the combo of these inputs.
    pub fn sets_combo(&self, inputs: &[GuideInput]) -> bool {
        let buttons: Vec<_> = inputs.iter().filter_map(|i| if let GuideInput::Button(b) = i { Some(*b) } else { None }).collect();
        let key = combo_key(&buttons);
        self.combos.iter().any(|c| combo_key(&c.buttons) == key) || self.disabled_combos.iter().any(|d| combo_key(d) == key)
    }
}

impl GuideSettings {
    /// These settings without what they say about the inputs `mode` changes: their texts were
    /// written for what those inputs do outside it.
    pub fn without_changes_of(&self, mode: &Mode) -> GuideSettings {
        let mut settings = self.clone();
        settings.edits.retain(|id, _| match id {
            RowId::Input(input) | RowId::Gesture(input, _) => !mode.sets(*input),
            RowId::Combo(inputs) => !mode.sets_combo(inputs),
        });
        settings.merged.retain(|m| !m.inputs.iter().any(|i| mode.sets(*i)));
        settings.custom.retain(|c| !c.inputs.iter().any(|i| mode.sets(*i)));
        settings
    }
}

impl Game {
    /// Its mode called `name`, if it has one.
    pub fn mode_named(&self, name: &str) -> Option<&Mode> {
        self.modes.iter().find(|m| m.name == name)
    }

    /// Its mode called `name`, or the built-in one (see [`built_in_mode`]).
    pub fn mode_or_built_in(&self, name: &str) -> Option<&Mode> {
        self.mode_named(name).or_else(|| built_in_mode(name))
    }
}

/// Makes every Toggle of a layer a switch to a mode of that name, and makes the layer a mode. A
/// toggled layer is what modes are for, and toggles of layers are no longer allowed. A layer that
/// is also held somewhere stays a layer too, so the held ones keep working. The built-in Guide and
/// Mouse mode, which were layers once, are switched to as modes wherever they're named.
pub fn toggled_layers_to_modes(profiles: &mut [Profile], menus: &mut [Menu], layers: &mut Vec<Layer>, modes: &mut Vec<Mode>) {
    let mut toggled: Vec<String> = Vec::new();
    let mut convert = |a: &mut ButtonAction| {
        a.walk_mut(&mut |a| {
            if let ButtonAction::Layer(name) = a
                && built_in_mode(name).is_some()
            {
                *a = ButtonAction::Mode(name.clone());
            } else if let ButtonAction::Toggle(Toggled { action, .. }) = a
                && let ButtonAction::Layer(name) | ButtonAction::Mode(name) = &**action
            {
                if built_in_mode(name).is_none() && !toggled.contains(name) {
                    toggled.push(name.clone());
                }
                *a = ButtonAction::Mode(name.clone());
            }
        });
    };
    let actions = profiles.iter_mut().flat_map(Profile::actions_mut);
    let items = menus.iter_mut().flat_map(|m| m.items.iter_mut().map(|item| &mut item.action));
    let shifts = layers.iter_mut().chain(modes.iter_mut()).flat_map(Layer::actions_mut);
    actions.chain(items).chain(shifts).for_each(&mut convert);

    for name in toggled {
        if modes.iter().any(|m| m.name == name) {
            continue;
        }
        let Some(at) = layers.iter().position(|l| l.name == name) else { continue };
        let mut still_held = false;
        let mut look = |a: &ButtonAction| a.walk(&mut |a| still_held |= matches!(a, ButtonAction::Layer(n) if *n == name));
        profiles.iter().flat_map(Profile::actions).for_each(&mut look);
        menus.iter().flat_map(|m| m.items.iter().map(|item| &item.action)).for_each(&mut look);
        layers.iter().chain(modes.iter()).flat_map(Layer::actions).for_each(&mut look);
        let mode = if still_held { layers[at].clone() } else { layers.remove(at) };
        modes.push(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Button, MenuItem, MenuKind, OverlayStyle};

    fn toggled_layer(name: &str) -> ButtonAction {
        ButtonAction::toggle(ButtonAction::Layer(name.into()))
    }

    #[test]
    fn toggled_layers_become_modes() {
        let mut game = Game::new("g", vec![Profile::passthrough("p")]);
        game.profiles[0].set_button(Button::North, toggled_layer("Driving"));
        game.profiles[0].set_button(Button::West, toggled_layer("Lean"));
        game.profiles[0].set_button(Button::LeftBumper, ButtonAction::Layer("Lean".into()));
        let item = MenuItem::new("Drive", ButtonAction::Multi(vec![toggled_layer("Driving")]));
        game.menus.push(Menu { name: "M".into(), kind: MenuKind::List, items: vec![item], cancel: None, no_cancel: false, start: 0, style: OverlayStyle::default() });
        let mut driving = Layer::new("Driving");
        driving.buttons.insert(Button::South, toggled_layer("Driving"));
        game.layers = vec![driving, Layer::new("Lean"), Layer::new("Other")];

        game.untoggle_menus();
        let mode = |n: &str| ButtonAction::Mode(n.into());
        assert_eq!(game.profiles[0].buttons[&Button::North], mode("Driving"));
        assert_eq!(game.profiles[0].buttons[&Button::West], mode("Lean"));
        assert_eq!(game.profiles[0].buttons[&Button::LeftBumper], ButtonAction::Layer("Lean".into()), "held layers stay");
        assert_eq!(game.menus[0].items[0].action, ButtonAction::Multi(vec![mode("Driving")]));
        let names = |v: &[Layer]| v.iter().map(|l| l.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&game.layers), ["Lean", "Other"], "a layer only toggled moves; one also held stays");
        assert_eq!(names(&game.modes), ["Driving", "Lean"]);
        assert_eq!(game.mode_named("Driving").unwrap().buttons[&Button::South], mode("Driving"), "inside a mode too");
    }

    #[test]
    fn the_built_in_layers_are_switched_to_as_the_built_in_modes() {
        use crate::config::{GUIDE_MODE, MOUSE_MODE};
        let mut game = Game::new("g", vec![Profile::passthrough("p")]);
        game.profiles[0].set_button(Button::North, toggled_layer(MOUSE_MODE));
        game.profiles[0].set_button(Button::Guide, ButtonAction::Layer(GUIDE_MODE.into()));
        game.untoggle_menus();
        assert_eq!(game.profiles[0].buttons[&Button::North], ButtonAction::Mode(MOUSE_MODE.into()));
        assert_eq!(game.profiles[0].buttons[&Button::Guide], ButtonAction::guide_hold());
        assert!(game.modes.is_empty(), "nothing to move: they're built in");
        assert!(game.mode_or_built_in(MOUSE_MODE).is_some() && game.mode_named(MOUSE_MODE).is_none());
    }
}
