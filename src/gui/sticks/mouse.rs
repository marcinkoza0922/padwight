//! A mouse stick's rows: its speed, its response (a preset and the settings it sets, as for
//! gyro and touchpad acceleration), and the "Advanced" fold.

use iced::widget::Column;

use super::*;
use crate::config::{CurvePreset, MouseResponse};

/// A mouse stick's speed and response: a preset, and its settings when it isn't linear, as for
/// gyro and touchpad acceleration.
pub(super) fn mouse_rows<'a>(rows: Column<'a, Message>, s: Stick, cfg: &'a StickConfig, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let StickAction::Mouse { speed, response, invert_y } = cfg.action else {
        return rows;
    };
    // The curve is the stick's, not the action's, so the response's settings send both.
    let set = move |curve: f32, response: MouseResponse| {
        let mut c = cfg.clone();
        (c.curve, c.action) = (curve, StickAction::Mouse { speed, response, invert_y });
        Message::SetStick(s, c)
    };
    let curve = cfg.curve;
    let preset = CurvePreset::of_stick(curve, &response);
    let picker = curve_preset_picker(preset, move |p: CurvePreset| {
        let (curve, response) = p.stick_response(response).unwrap_or_else(|| custom_response(preset, curve, response));
        set(curve, response)
    });
    let rows = rows
        .push(box_row("Speed", slider_editor(100.0..=4000.0, speed, 50.0, "px/s", move |v| with(StickAction::Mouse { speed: v, response, invert_y }))))
        .push(box_row("Response", picker));
    if preset == CurvePreset::Linear {
        return rows;
    }
    let rows = rows
        .push(sub_row("Curve", slider_editor(1.0..=3.0, curve, 0.05, "(1 = straight, more = finer near center)", move |v| set(v, response))))
        .push(sub_row(
            "Acceleration",
            slider_editor(0.0..=100.0, response.accel * 100.0, 5.0, "% faster while held at full push", move |v| {
                set(curve, MouseResponse { accel: v / 100.0, ..response })
            }),
        ));
    let rows = if response.accel > 0.0 {
        rows.push(sub_row(
            "Ramp time",
            slider_editor(100.0..=2000.0, response.accel_ramp_ms as f32, 50.0, "ms to full acceleration", move |v| {
                set(curve, MouseResponse { accel_ramp_ms: v as u32, ..response })
            }),
        ))
    } else {
        rows
    };
    rows.push(sub_row(
        "Outer boost",
        fraction_editor(0.0..=1.0, response.outer_boost, 0.05, "more speed near the edge", move |v| set(curve, MouseResponse { outer_boost: v, ..response })),
    ))
}

/// Where Custom starts: the response in use with its curve changed, so it matches no preset and
/// its settings show. From Linear, which shows none, it starts from Gentle instead.
fn custom_response(from: CurvePreset, curve: f32, response: MouseResponse) -> (f32, MouseResponse) {
    let (curve, response) = if from == CurvePreset::Linear { CurvePreset::Gentle.stick_response(response).unwrap_or((curve, response)) } else { (curve, response) };
    (if curve + 0.25 <= 3.0 { curve + 0.25 } else { curve - 0.25 }, response)
}

/// What a mouse stick's "Advanced" fold holds besides the deadzone rows: vertical speed and
/// smoothing, which no preset sets.
pub(super) const ADVANCED_HOLDS: &str = "Deadzone shape, anti-deadzone, vertical speed, smoothing";

pub(super) fn mouse_advanced<'a>(rows: Column<'a, Message>, cfg: &StickConfig, with: impl Fn(StickAction) -> Message + Copy + 'a) -> Column<'a, Message> {
    let StickAction::Mouse { speed, response, invert_y } = cfg.action else {
        return rows;
    };
    let set = move |r: MouseResponse| with(StickAction::Mouse { speed, response: r, invert_y });
    rows.push(fold_row("Vertical speed", slider_editor(0.25..=2.0, response.y_scale, 0.05, "× horizontal", move |v| set(MouseResponse { y_scale: v, ..response }))))
        .push(fold_row("Smoothing", slider_editor(0.0..=200.0, response.smoothing_ms as f32, 5.0, "ms", move |v| set(MouseResponse { smoothing_ms: v as u32, ..response }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_starts_from_a_response_of_its_own() {
        let base = MouseResponse::default();
        for p in CurvePreset::NAMED {
            let (curve, response) = p.stick_response(base).unwrap();
            let (curve, response) = custom_response(p, curve, response);
            assert_eq!(CurvePreset::of_stick(curve, &response), CurvePreset::Custom, "from {p}");
            assert!((1.0..=3.0).contains(&curve), "from {p}: curve {curve} is off the slider");
        }
    }

    #[test]
    fn every_preset_builds() {
        for p in CurvePreset::NAMED.into_iter().chain([CurvePreset::Custom]) {
            let (curve, response) = p.stick_response(MouseResponse::default()).unwrap_or((1.4, MouseResponse { accel: 0.3, ..MouseResponse::default() }));
            let cfg = StickConfig::new(StickAction::Mouse { speed: 1200.0, response, invert_y: false }, 0.1, curve);
            let with = |a| Message::SetStick(Stick::Right, StickConfig::new(a, 0.1, 1.0));
            let _ = mouse_rows(Column::new(), Stick::Right, &cfg, with);
            let _ = mouse_advanced(Column::new(), &cfg, with);
        }
    }
}
