//! A picture beside a page of the in-game editor, as the overlay process draws it: what it shows
//! is sent as data, with the live input already in it, and drawn as the settings window draws it.

use serde::{Deserialize, Serialize};

use crate::config::{Deadzone, Menu, MouseResponse, SpeedCurve, TouchpadConfig};

use super::{deadzone, gyro, response, ring, trigger};

/// What a mouse stick's response pictures show.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MouseShown {
    pub speed: f32,
    pub curve: f32,
    pub response: MouseResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Figure {
    /// A stick: its deadzone, its ring or mouse response when it has one, and where it is now.
    Stick { deadzone: Deadzone, ring: Option<(u8, f32, f32)>, mouse: Option<MouseShown>, live: Option<(f32, f32)> },
    /// A trigger: where it presses, its zones, and how far it's pulled now.
    Trigger { press_at: Option<f32>, zones: Vec<(f32, f32)>, live: Option<f32> },
    /// The gyro: its response curve when its mode has one, how fast the controller turns now, and
    /// where the turning has gone lately.
    Gyro { curve: Option<SpeedCurve>, speed: Option<f32>, trail: Vec<(f32, f32)>, at: (f32, f32) },
    /// The touchpad with what it does, its menu's items where they lie, and the fingers on it.
    Touchpad { pad: Box<TouchpadConfig>, menus: Vec<Menu>, touches: Vec<(f32, f32)>, click: bool, height: f32 },
}

/// One picture: its caption, its SVG, its size in logical pixels, and a note under it.
pub struct Picture {
    pub title: &'static str,
    pub svg: String,
    pub size: (f32, f32),
    pub note: Option<String>,
}

impl Figure {
    /// The figure's SVG pictures. The touchpad and a menu are drawn by the overlay itself.
    pub fn pictures(&self) -> Vec<Picture> {
        match self {
            Figure::Stick { deadzone: dz, ring: r, mouse, live } => stick_pictures(*dz, *r, mouse.as_ref(), *live),
            Figure::Trigger { press_at, zones, live } => {
                let note = live.map(|v| format!("Pulled {:.0}%", v * 100.0));
                vec![Picture { title: "Travel", svg: trigger::travel_svg(*press_at, zones, *live), size: (trigger::W, trigger::H), note }]
            }
            Figure::Gyro { curve, speed, trail, at } => {
                let mut pics = Vec::new();
                if let Some(curve) = curve {
                    let note = speed.map(|v| format!("Turning {v:.0}°/s: {:.2}× sensitivity", curve.gain(v)));
                    pics.push(Picture { title: "Sensitivity by speed", svg: gyro::curve_svg(curve, *speed), size: (gyro::W, gyro::H), note });
                }
                let note = trail.is_empty().then(|| "Turn the controller".to_string());
                pics.push(Picture { title: "Turning (live)", svg: gyro::aim_points_svg(trail, *at), size: (gyro::SIDE, gyro::SIDE), note });
                pics
            }
            Figure::Touchpad { .. } => Vec::new(),
        }
    }
}

fn stick_pictures(dz: Deadzone, r: Option<(u8, f32, f32)>, mouse: Option<&MouseShown>, live: Option<(f32, f32)>) -> Vec<Picture> {
    let mut pics = Vec::new();
    if let Some((sectors, start, inner)) = r {
        let active = live.and_then(|pos| crate::engine::stick::ring_sector_at(pos, sectors, start, inner));
        let note = active.map(|i| format!("Sector {}", i + 1));
        pics.push(Picture { title: "Ring", svg: ring::render(sectors, start, inner, active, live), size: (ring::SIZE, ring::SIZE), note });
    }
    let note = if dz.size <= 0.0 { "No deadzone".to_string() } else { format!("Flat for the first {:.0}%", dz.size * 100.0) };
    pics.push(Picture { title: "Stick travel", svg: deadzone::travel_svg(dz, live), size: (deadzone::SIDE, deadzone::SIDE), note: None });
    if let Some(m) = mouse {
        let push = live.map_or(1.0, |(x, y)| x.hypot(y).min(1.0));
        let shown = response::Shown { speed: m.speed, curve: m.curve, response: m.response, deadzone: dz, probe: push, live };
        pics.push(Picture { title: "Pointer speed", svg: response::curve_svg(&shown), size: (response::W, response::H), note: Some(response::curve_readout(&shown)) });
    } else {
        pics.push(Picture { title: "Response", svg: deadzone::response_svg(dz, live), size: (deadzone::SIDE, deadzone::SIDE), note: Some(note) });
    }
    pics
}

/// What the controller driving the in-game menu is doing now, for the pictures: both sticks,
/// both triggers, the fingers on its touchpad, and its turning.
#[derive(Default)]
pub struct Live {
    pub sticks: [(f32, f32); 2],
    pub triggers: [f32; 2],
    /// Fingers on the touchpad by number, each (x, y) 0..1 across it.
    pub touches: std::collections::BTreeMap<u8, (f32, f32)>,
    pub click: bool,
    /// The touchpad's height in widths.
    pub pad_height: f32,
    /// How fast the controller turns (degrees/second), and the path its turning has made.
    pub turning: Option<f32>,
    pub trail: gyro::Trail,
}

impl Live {
    /// Notes one input event of the controller.
    pub fn input(&mut self, ev: crate::input::InputEvent) {
        use crate::input::{Axis, InputEvent, TouchpadEvent};
        match ev {
            InputEvent::Axis(axis, v) => match axis {
                Axis::LeftX => self.sticks[0].0 = v,
                Axis::LeftY => self.sticks[0].1 = v,
                Axis::RightX => self.sticks[1].0 = v,
                Axis::RightY => self.sticks[1].1 = v,
                Axis::LeftTrigger => self.triggers[0] = v,
                Axis::RightTrigger => self.triggers[1] = v,
            },
            InputEvent::Touchpad(TouchpadEvent::Touch { finger, x, y, height }) => {
                self.touches.insert(finger, (x, y));
                self.pad_height = height;
            }
            InputEvent::Touchpad(TouchpadEvent::Lift { finger }) => drop(self.touches.remove(&finger)),
            InputEvent::Touchpad(TouchpadEvent::Click(down)) => self.click = down,
            InputEvent::Button(..) => {}
        }
    }

    /// Notes one gyro reading, in the standard frame (pitch, yaw, roll in degrees/second): its
    /// speed, and the turning on screen, left and right from yaw and up and down from pitch.
    pub fn turning(&mut self, gyro: [f32; 3], now: std::time::Instant) {
        let aim = (-gyro[1], -gyro[0]);
        let speed = aim.0.hypot(aim.1);
        self.turning = Some(speed);
        self.trail.push(Some(crate::ipc::GyroOut { speed, aim, stick: None }), now);
    }
}
