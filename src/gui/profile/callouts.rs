//! What the live controller picture says about a profile: the callouts beside its parts, and the
//! tooltips on the buttons that do more than one thing.

use super::*;

/// A stick's mode in a word or two for the picture, or `None` while it's the stick it is.
fn stick_mode(cfg: &StickConfig, s: Stick) -> Option<&'static str> {
    Some(match cfg.action {
        StickAction::Gamepad { stick, .. } if stick == s => return None,
        StickAction::Gamepad { stick: Stick::Left, .. } => "Pad LS",
        StickAction::Gamepad { stick: Stick::Right, .. } => "Pad RS",
        StickAction::Disabled => "Stick off",
        StickAction::Mouse { .. } => "Mouse",
        StickAction::Scroll { .. } => "Scroll",
        StickAction::Ring { .. } => "Ring",
        StickAction::Flick { .. } => "Flick",
    })
}

/// An action as its callout on the picture says it: the summary, shortened where the column is
/// too narrow for it. A menu is its name alone, as the list below the picture marks it a menu.
fn callout(action: &ButtonAction) -> String {
    match action {
        ButtonAction::GyroClutch => "Gyro clutch".into(),
        ButtonAction::OpenMenu(name) => format!("“{name}”"),
        _ => summarize(action),
    }
}

/// Labels for the controller drawing: every input that doesn't simply pass through. Guide's
/// usual mapping is left out too: the engine ignores it (Guide is built in).
pub(super) fn drawing_labels(p: &Profile) -> Vec<(pad_svg::Spot, String)> {
    let mut labels = Vec::new();
    for b in Button::ALL {
        let usual = if b == Button::Guide { ButtonAction::guide_hold() } else { ButtonAction::Gamepad(b) };
        let gestures = p.gestures(b).is_some();
        // A stick's own mode shares its click's label, since both meet the picture at the stick.
        let stick = match b {
            Button::LeftStick => stick_mode(p.stick(Stick::Left), Stick::Left),
            Button::RightStick => stick_mode(p.stick(Stick::Right), Stick::Right),
            _ => None,
        };
        let click = (*p.button(b) != usual || gestures).then(|| {
            let mut label = callout(p.button(b));
            if gestures {
                label.push_str(pad_svg::GESTURE_MARK);
            }
            label
        });
        let label = match (stick, click) {
            (Some(stick), Some(click)) => format!("{stick} · {click}"),
            (Some(stick), None) => stick.to_string(),
            (None, Some(click)) => click,
            (None, None) => continue,
        };
        labels.push((pad_svg::Spot::Button(b), label));
    }
    for t in [Trigger::Left, Trigger::Right] {
        let label = match p.trigger(t) {
            TriggerAction::Gamepad(out) if *out == t => continue,
            TriggerAction::Gamepad(out) => format!("Pad {}", if *out == Trigger::Left { "LT" } else { "RT" }),
            TriggerAction::Disabled => "Disabled".into(),
            TriggerAction::Button { action, .. } => callout(action),
        };
        labels.push((pad_svg::Spot::Trigger(t), label));
    }
    labels
}

/// The whole of what each button with a gesture or in a combo does, for the tooltip on its part
/// of the picture: its press, each gesture, then each combo it's part of.
pub(super) fn drawing_tips(p: &Profile) -> Vec<(pad_svg::Spot, String)> {
    let mut tips = Vec::new();
    for b in Button::ALL {
        let gestures: Vec<String> = GestureKind::ALL
            .into_iter()
            .filter_map(|k| p.gestures(b).and_then(|g| g.get(k)).map(|a| format!("{k}: {}", summarize(a))))
            .collect();
        let combos: Vec<String> = p
            .combos
            .iter()
            .filter(|c| c.buttons.contains(&b))
            .map(|c| {
                let name = c.buttons.iter().map(|b| short_button(*b)).collect::<Vec<_>>().join(" + ");
                format!("{name}: {}", summarize(&c.action))
            })
            .collect();
        if gestures.is_empty() && combos.is_empty() {
            continue;
        }
        let lines = std::iter::once(format!("Press: {}", summarize(p.button(b)))).chain(gestures).chain(combos);
        tips.push((pad_svg::Spot::Button(b), lines.collect::<Vec<_>>().join("\n")));
    }
    tips
}
