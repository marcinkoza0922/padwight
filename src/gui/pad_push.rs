//! Editors for the two actions that push the virtual pad's analog inputs: pulling a trigger and
//! pushing a stick toward an angle, each by an amount that defaults to all the way.

use iced::widget::{column, row};

use super::sliders::{fraction_editor, slider_editor};
use super::*;
use crate::config::{COMPASS, angle_name};

/// Arrows for the eight compass directions, in the order of [`COMPASS`].
const ARROWS: [&str; 8] = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];

/// A trigger to pull and how far.
pub(super) fn trigger_editor<'a>(trigger: Trigger, amount: f32, on_change: OnAction<'a>) -> Element<'a, Message> {
    let which = {
        let on_change = on_change.clone();
        dropdown([Trigger::Left, Trigger::Right], Some(trigger), move |t| on_change(ButtonAction::PadTrigger { trigger: t, amount })).width(VALUE_WIDTH)
    };
    let how_far = fraction_editor(0.0..=1.0, amount, 0.01, "pulled", move |a| on_change(ButtonAction::PadTrigger { trigger, amount: a }));
    column![which, how_far].spacing(6).into()
}

/// A stick to push, the direction (one of the eight compass points, or any angle) and how far.
pub(super) fn stick_editor<'a>(stick: Stick, angle: f32, amount: f32, on_change: OnAction<'a>) -> Element<'a, Message> {
    let set = move |on_change: &OnAction<'a>, stick, angle, amount| on_change(ButtonAction::PadStick { stick, angle, amount });
    let which = {
        let on_change = on_change.clone();
        dropdown([Stick::Left, Stick::Right], Some(stick), move |s| set(&on_change, s, angle, amount)).width(VALUE_WIDTH)
    };
    let compass = row(COMPASS.iter().zip(ARROWS).map(|(&(_, at), arrow)| {
        let chosen = (angle.rem_euclid(360.0) - at).abs() < 0.05;
        let pick = set(&on_change, stick, at, amount);
        button(text(arrow).size(16).center())
            .style(move |t, status| if chosen { style::segment(true)(t, status) } else { style::secondary(t, status) })
            .width(34)
            .padding([4, 0])
            .on_press(pick)
            .into()
    }))
    .spacing(4);
    let aim = {
        let on_change = on_change.clone();
        slider_editor(0.0..=359.0, angle.rem_euclid(360.0).round(), 1.0, "°", move |a| set(&on_change, stick, a, amount))
    };
    let how_far = fraction_editor(0.0..=1.0, amount, 0.01, "pushed", move |a| set(&on_change, stick, angle, a));
    let toward = text(format!("Toward {}", angle_name(angle))).size(12).color(MUTED_COLOR);
    column![which, compass, aim, toward, how_far].spacing(6).into()
}
