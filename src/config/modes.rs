//! Modes: a game's sets of overrides that are switched to, one at a time, rather than held like
//! layers. A mode is built like a layer (see [`Layer`]); with no mode on, the profile itself is
//! the default mode.

use super::{Game, GuideInput, GuideSettings, Layer, RowId, Stick, Trigger, combo_key};

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
}
