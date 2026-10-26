//! The rows of the Mappings overlay and the Guide tab, built from what the profile maps and
//! the Guide settings' edits.

use super::super::summary::{Piece, guide_text, plain_text};
use super::super::{Button, ButtonAction, GestureKind, Gestures, Profile, Stick, StickAction, Trigger, TriggerAction, combo_key};
use super::{GuideInput, GuideSettings, RowId, RowShape};

/// One line of the Mappings overlay: the inputs it names, and the text after them.
#[derive(Debug, Clone, PartialEq)]
pub struct MappingRow {
    pub inputs: Vec<GuideInput>,
    pub text: String,
    pub shape: RowShape,
}

/// Which row of the Guide tab something refers to: a row of the list, or a merged input row by
/// its index in [`GuideSettings::merged`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKey {
    Row(RowId),
    Merged(usize),
    Custom(usize),
}

/// A row of the Guide tab: the mapping it stands for, the text it shows, and whether it's shown.
#[derive(Debug, Clone, PartialEq)]
pub struct EditableRow {
    pub key: RowKey,
    pub inputs: Vec<GuideInput>,
    /// The text the overlay shows: the override, or the default.
    pub text: String,
    /// The default text, which the text is replaced from.
    pub default_text: String,
    /// The default text as plain words, for showing where its glyph tokens can't be drawn (a text
    /// field): `“'”` rather than `{keyboard:apostrophe}`.
    pub default_label: String,
    pub shape: RowShape,
    pub overridden: bool,
    pub hidden: bool,
}

/// The action an On press and an On release both tap, which makes a game's toggle a hold: it
/// reads as one thing the input does, so it has no rows of its own.
fn held_toggle(gestures: Option<&Gestures>) -> Option<&ButtonAction> {
    let g = gestures?;
    g.on_press.as_ref().filter(|a| g.on_release.as_ref() == Some(*a) && **a != ButtonAction::Disabled)
}

/// Every input the profile maps, with its default text in pieces, in order: buttons, then
/// triggers, then sticks. Guide itself, disabled inputs and plain pass-through buttons, triggers and
/// sticks aren't listed. An input that does nothing itself but hold a toggle (see [`held_toggle`])
/// says what the toggle does.
fn mapped_inputs(profile: &Profile) -> Vec<(GuideInput, Vec<Piece>)> {
    let mut inputs = Vec::new();
    let own = |action: &ButtonAction, gestures: Option<&Gestures>| match held_toggle(gestures) {
        Some(toggle) if *action == ButtonAction::Disabled => toggle.pieces(),
        _ => action.pieces(),
    };
    let buttons = profile.buttons.keys().chain(profile.gestures.keys().filter(|b| !profile.buttons.contains_key(b)));
    let mut buttons: Vec<Button> = buttons.copied().collect();
    buttons.sort();
    for b in buttons {
        let (action, gestures) = (profile.button(b), profile.gestures(b));
        let shown = *action != ButtonAction::Disabled || held_toggle(gestures).is_some();
        if b != Button::Guide && shown && *action != ButtonAction::Gamepad(b) {
            inputs.push((GuideInput::Button(b), own(action, gestures)));
        }
    }
    for t in [Trigger::Left, Trigger::Right] {
        if let TriggerAction::Button { action, .. } = profile.trigger(t) {
            inputs.push((GuideInput::Trigger(t), own(action, Some(&profile.trigger_config(t).gestures))));
        }
    }
    for s in [Stick::Left, Stick::Right] {
        let text = match &profile.stick(s).action {
            StickAction::Disabled | StickAction::Gamepad { .. } => continue,
            StickAction::Mouse { .. } => "Move the mouse",
            StickAction::Scroll { .. } => "Scroll",
            StickAction::Region { .. } => "Point",
            _ => "Remapped",
        };
        inputs.push((GuideInput::Stick(s), vec![Piece::Text(text.to_string())]));
    }
    inputs
}

/// Every row the profile's Mappings overlay can show, hidden ones included: the inputs (merged
/// inputs share one row, at the first of them), then the gestures, then the combos.
pub fn editable_rows(profile: &Profile) -> Vec<EditableRow> {
    let settings = &profile.guide;
    let mapped = mapped_inputs(profile);
    let mut rows = Vec::new();
    for (input, pieces) in &mapped {
        if let Some((index, merge)) = settings.merged.iter().enumerate().find(|(_, m)| m.inputs.contains(input)) {
            let first = mapped.iter().find(|(i, _)| merge.inputs.contains(i)).map(|(i, _)| i) == Some(input);
            if first {
                let inputs = mapped.iter().map(|(i, _)| *i).filter(|i| merge.inputs.contains(i)).collect();
                rows.push(EditableRow {
                    key: RowKey::Merged(index),
                    inputs,
                    text: merge.text.clone(),
                    default_text: String::new(),
                    default_label: String::new(),
                    shape: RowShape::Merged,
                    overridden: true,
                    hidden: merge.hidden,
                });
            }
            continue;
        }
        rows.push(row(settings, RowId::Input(*input), vec![*input], pieces, RowShape::Input));
    }
    let triggers = [Trigger::Left, Trigger::Right]
        .into_iter()
        .filter(|t| matches!(profile.trigger(*t), TriggerAction::Button { .. }))
        .map(|t| (GuideInput::Trigger(t), &profile.trigger_config(t).gestures));
    let buttons = profile.gestures.iter().filter(|(b, _)| **b != Button::Guide).map(|(b, g)| (GuideInput::Button(*b), g));
    for (input, gestures) in buttons.chain(triggers) {
        let folded = held_toggle(Some(gestures)).is_some();
        for kind in GestureKind::ALL {
            if folded && GestureKind::EDGES.contains(&kind) {
                continue;
            }
            if let Some(action) = gestures.get(kind).filter(|a| **a != ButtonAction::Disabled) {
                rows.push(row(settings, RowId::Gesture(input, kind), vec![input], &action.pieces(), RowShape::Gesture(kind)));
            }
        }
    }
    for combo in &profile.combos {
        if combo.buttons.len() < 2 || combo.buttons.contains(&Button::Guide) || combo.action == ButtonAction::Disabled {
            continue;
        }
        let inputs: Vec<GuideInput> = combo_key(&combo.buttons).into_iter().map(GuideInput::Button).collect();
        rows.push(row(settings, RowId::Combo(inputs.clone()), inputs, &combo.action.pieces(), RowShape::Combo));
    }
    for (index, custom) in settings.custom.iter().enumerate() {
        let shape = if custom.inputs.len() > 1 { RowShape::Combo } else { RowShape::Input };
        rows.push(EditableRow {
            key: RowKey::Custom(index),
            inputs: custom.inputs.clone(),
            text: custom.text.clone(),
            default_text: String::new(),
            default_label: String::new(),
            shape,
            overridden: true,
            hidden: false,
        });
    }
    rows
}

/// A row for `id`, with its edit applied. `pieces` is the default text.
fn row(settings: &GuideSettings, id: RowId, inputs: Vec<GuideInput>, pieces: &[Piece], shape: RowShape) -> EditableRow {
    let edit = settings.edits.get(&id).cloned().unwrap_or_default();
    let default_text = guide_text(pieces);
    EditableRow {
        key: RowKey::Row(id),
        inputs,
        text: edit.text.clone().unwrap_or_else(|| default_text.clone()),
        default_text,
        default_label: plain_label(pieces),
        shape,
        overridden: edit.text.is_some(),
        hidden: edit.hidden,
    }
}

/// The default text in plain words. A text that starts with a key says "Press", since a lone
/// key reads as nothing much in plain text (“'”).
fn plain_label(pieces: &[Piece]) -> String {
    let text = plain_text(pieces);
    match pieces.iter().find(|p| **p != Piece::Text(String::new())) {
        Some(Piece::Key(_)) => format!("Press {text}"),
        _ => text,
    }
}

/// The Mappings overlay's rows for `profile`: the editable rows that aren't hidden. They follow
/// the mappings, so a change to an action shows up here unless its text is overridden.
pub fn mapping_rows(profile: &Profile) -> Vec<MappingRow> {
    editable_rows(profile)
        .into_iter()
        .filter(|r| !r.hidden)
        .map(|r| MappingRow { inputs: r.inputs, text: r.text, shape: r.shape })
        .collect()
}

/// The text an input's row shows by default: its action's description, or what its stick or
/// trigger does.
pub fn default_text(profile: &Profile, input: GuideInput) -> String {
    mapped_inputs(profile).into_iter().find(|(i, _)| *i == input).map(|(_, pieces)| guide_text(&pieces)).unwrap_or_default()
}
