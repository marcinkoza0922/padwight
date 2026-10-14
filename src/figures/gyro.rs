//! The gyro in pictures: the acceleration curve (sensitivity against turning speed) with the live
//! speed on it, where the aim has been moving lately, and in the stick modes, the virtual stick.

use std::{collections::VecDeque, fmt::Write, time::Instant};

use crate::config::SpeedCurve;
use crate::ipc::GyroOut;

/// The curve's size, and the side of the square pictures, in logical pixels; `PAD` is kept clear
/// inside them.
pub const W: f32 = 220.0;
pub const H: f32 = 120.0;
pub const SIDE: f32 = 120.0;
pub const PAD: f32 = 10.0;
const PANEL: &str = "#1d2026";
const GRID: &str = "#30343c";
const LINEAR: &str = "#5d6470";
const CURVE: &str = "#4ea1ff";
const MARK: &str = "#ffffff";

/// How far (degrees) the aim picture reaches from its middle, and how quickly (seconds) the
/// aim drifts back to the middle so it stays in view.
const AIM_REACH: f32 = 30.0;
const AIM_RETURN: f32 = 1.5;
/// Points kept in the aim's trail.
const TRAIL: usize = 90;

/// Where the gyro's aim has been lately, built from the live input: the turning integrated into
/// degrees, slowly pulled back to the middle.
#[derive(Default)]
pub struct Trail {
    at: (f32, f32),
    points: VecDeque<(f32, f32)>,
    last: Option<Instant>,
    live: bool,
}

impl Trail {
    /// Where the aim has been (oldest first) and where it is now, while gyro is on.
    pub fn points(&self) -> (Vec<(f32, f32)>, (f32, f32)) {
        (if self.live { self.points.iter().copied().collect() } else { Vec::new() }, self.at)
    }

    /// Whether the gyro was on at the last reading.
    pub fn is_live(&self) -> bool {
        self.live
    }

    /// Adds one live reading; `None` is gyro being off.
    pub fn push(&mut self, out: Option<GyroOut>, now: Instant) {
        let dt = self.last.map_or(0.0, |t| now.duration_since(t).as_secs_f32().min(0.05));
        self.last = Some(now);
        self.live = out.is_some();
        let Some(out) = out else {
            self.at = (0.0, 0.0);
            self.points.clear();
            return;
        };
        let keep = (-dt / AIM_RETURN).exp();
        self.at.0 = ((self.at.0 + out.aim.0 * dt) * keep).clamp(-AIM_REACH, AIM_REACH);
        self.at.1 = ((self.at.1 + out.aim.1 * dt) * keep).clamp(-AIM_REACH, AIM_REACH);
        if self.points.len() == TRAIL {
            self.points.pop_front();
        }
        self.points.push_back(self.at);
    }
}

fn polyline(points: impl Iterator<Item = (f32, f32)>, stroke: &str, dash: &str, width: f32) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.enumerate() {
        let _ = write!(d, "{} {x:.1} {y:.1} ", if i == 0 { "M" } else { "L" });
    }
    format!(r#"<path d="{d}" fill="none" stroke="{stroke}" stroke-width="{width}" stroke-dasharray="{dash}" stroke-linejoin="round"/>"#)
}

fn dot(at: (f32, f32), color: &str) -> String {
    format!(r##"<circle cx="{:.1}" cy="{:.1}" r="4.5" fill="{color}" stroke="#000" stroke-width="1"/>"##, at.0, at.1)
}

fn open(w: f32, h: f32) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" rx="6" fill="{PANEL}"/>"#)
}

/// The fastest turning the curve picture shows: past the curve's fast end, and at least 60°/s.
pub fn top_speed(curve: &SpeedCurve) -> f32 {
    (curve.fast_speed * 1.25).max(60.0)
}

/// SVG of the curve: sensitivity (up, as a multiple of the base) against turning speed
/// (across). A dotted line marks the base sensitivity, and the dot the live speed.
pub fn curve_svg(curve: &SpeedCurve, live: Option<f32>) -> String {
    let top = top_speed(curve);
    let ceiling = curve.slow_gain.max(curve.fast_gain).max(2.0) * 1.1;
    let x = |v: f32| PAD + v / top * (W - 2.0 * PAD);
    let y = |g: f32| H - PAD - g / ceiling * (H - 2.0 * PAD);
    let mut out = open(W, H);
    out.push_str(&polyline([(PAD, H - PAD), (W - PAD, H - PAD)].into_iter(), GRID, "none", 1.0));
    out.push_str(&polyline([(PAD, y(1.0)), (W - PAD, y(1.0))].into_iter(), LINEAR, "3 3", 1.5));
    out.push_str(&polyline((0..=100).map(|i| top * i as f32 / 100.0).map(|v| (x(v), y(curve.gain(v)))), CURVE, "none", 2.0));
    if let Some(speed) = live {
        let v = speed.min(top);
        out.push_str(&dot((x(v), y(curve.gain(v))), MARK));
    }
    out.push_str("</svg>");
    out
}

/// SVG of the aim's recent path, with crosshairs; empty while gyro is off.
pub fn aim_svg(trail: &Trail) -> String {
    let points: Vec<(f32, f32)> = if trail.live { trail.points.iter().copied().collect() } else { Vec::new() };
    aim_points_svg(&points, trail.at)
}

/// SVG of an aim's path, `points` in degrees from the middle (the newest last, at `at`), with
/// crosshairs; no path when there are no points.
pub fn aim_points_svg(points: &[(f32, f32)], at: (f32, f32)) -> String {
    let c = SIDE / 2.0;
    let k = (c - PAD) / AIM_REACH;
    let mut out = open(SIDE, SIDE);
    out.push_str(&polyline([(c, PAD), (c, SIDE - PAD)].into_iter(), GRID, "3 3", 1.0));
    out.push_str(&polyline([(PAD, c), (SIDE - PAD, c)].into_iter(), GRID, "3 3", 1.0));
    if !points.is_empty() {
        let place = |(x, y): (f32, f32)| (c + x * k, c + y * k);
        out.push_str(&polyline(points.iter().copied().map(place), CURVE, "none", 1.5));
        out.push_str(&dot(place(at), MARK));
    }
    out.push_str("</svg>");
    out
}

/// SVG of the virtual stick the gyro deflects, `deflection` in -1..1 (y down).
pub fn stick_svg(deflection: Option<(f32, f32)>) -> String {
    let c = SIDE / 2.0;
    let r = c - PAD;
    let mut out = open(SIDE, SIDE);
    let _ = write!(out, r#"<circle cx="{c}" cy="{c}" r="{r}" fill="none" stroke="{GRID}" stroke-width="1.5"/>"#);
    if let Some((x, y)) = deflection {
        out.push_str(&polyline([(c, c), (c + x * r, c + y * r)].into_iter(), CURVE, "none", 2.0));
        out.push_str(&dot((c + x * r, c + y * r), MARK));
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CurvePreset;

    fn out(aim: (f32, f32)) -> Option<GyroOut> {
        Some(GyroOut { speed: aim.0.hypot(aim.1), aim, stick: None })
    }

    #[test]
    fn the_trail_follows_the_aim_and_clears_when_gyro_goes_off() {
        let mut t = Trail::default();
        let t0 = Instant::now();
        for i in 0..10 {
            t.push(out((100.0, 0.0)), t0 + std::time::Duration::from_millis(10 * i));
        }
        assert!(t.at.0 > 5.0 && t.at.1 == 0.0, "{:?}", t.at);
        assert!(aim_svg(&t).contains("<circle"));
        t.push(None, t0 + std::time::Duration::from_millis(200));
        assert!(t.points.is_empty() && !aim_svg(&t).contains("<circle"));
    }

    #[test]
    fn the_curve_marks_the_live_speed_only_with_a_controller() {
        let c = CurvePreset::Aggressive.speed_curve(1.0).unwrap();
        assert_eq!(curve_svg(&c, None).matches("<circle").count(), 0);
        assert_eq!(curve_svg(&c, Some(30.0)).matches("<circle").count(), 1);
    }

    #[test]
    fn the_stick_shows_its_deflection() {
        assert_eq!(stick_svg(None).matches("<circle").count(), 1);
        assert_eq!(stick_svg(Some((0.5, 0.0))).matches("<circle").count(), 2);
    }
}

