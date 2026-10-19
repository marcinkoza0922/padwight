//! The gyro in pictures: the acceleration curve (sensitivity against turning speed) with the live
//! speed on it, where the aim has been moving lately, and in the stick modes, the virtual stick.
//! The live parts come from the controller in use and show the active profile's saved gyro.

use iced::Element;
use iced::widget::{column, row, svg};

use super::*;
use crate::figures::gyro::{H, SIDE, W, aim_svg, curve_svg, stick_svg, top_speed};
pub(super) use crate::figures::gyro::Trail;
use crate::ipc::GyroOut;

fn picture<'a>(svg_text: String, w: f32, h: f32) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(svg_text.into_bytes())).width(w).height(h).into()
}

/// The gyro's pictures for its settings: the curve when the mode uses one, the aim, and the
/// virtual stick in the stick modes. `live` is the controller's gyro output, when there is one.
pub(super) fn figures<'a>(cfg: &GyroConfig, live: Option<GyroOut>, trail: &Trail) -> Vec<Element<'a, Message>> {
    let mut pics = Vec::new();
    if cfg.mode.uses_curve() {
        let readout = match live {
            Some(out) => format!("Turning {:.0}°/s: {:.2}× sensitivity", out.speed, cfg.curve.gain(out.speed)),
            None => format!("{:.2}× slow, {:.2}× fast", cfg.curve.slow_gain, cfg.curve.fast_gain),
        };
        let faint = |s: String| text(s).size(11).color(MUTED_COLOR);
        let ticks = row![faint("0°/s".into()), space().width(Length::Fill), faint("Turning speed".into()), space().width(Length::Fill), faint(format!("{:.0}°/s", top_speed(&cfg.curve)))].width(W);
        pics.push(figure("Sensitivity by speed", column![picture(curve_svg(&cfg.curve, live.map(|o| o.speed)), W, H), ticks].spacing(2), Some(readout), W));
    }
    let off = (!trail.is_live()).then(|| "Gyro is off".to_string());
    pics.push(figure("Aim (live)", picture(aim_svg(trail), SIDE, SIDE), off, SIDE + 40.0));
    if matches!(cfg.mode, GyroMode::Stick { .. } | GyroMode::Position { .. } | GyroMode::Steering { .. }) {
        let stick = live.and_then(|o| o.stick);
        let note = stick.map(|(x, y)| format!("{:.0}% pushed", x.hypot(y).min(1.0) * 100.0));
        pics.push(figure("Virtual stick (live)", picture(stick_svg(stick), SIDE, SIDE), note, SIDE + 40.0));
    }
    pics
}
