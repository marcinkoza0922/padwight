//! Gyro: turns motion-sensor samples into mouse movement or virtual-pad stick deflection.
//!
//! Each sample is read in the chosen space (the controller's own axes, or player or world space
//! from gravity), has drift and jitter taken out, is smoothed when slow, and is shaped by the
//! acceleration curve before the mode turns it into output.

use crate::{
    config::{GyroActivation, GyroConfig, GyroHorizontal, GyroInput, GyroMode, Profile, Stick},
    input::{Axis, MotionSample},
    ipc::GyroOut,
    output::OutEvent,
};

use super::{Engine, angle_diff, stick_axes, take_whole};

/// Time constant (seconds) for smoothing the accelerometer tilt used by gyro steering.
const TILT_SMOOTHING: f32 = 0.05;
/// Time constant (seconds) for the gravity direction used by player and world space.
const GRAVITY_SMOOTHING: f32 = 0.2;
/// Player space leans on the yaw found from gravity this much more than its share, as
/// JoyShockMapper does, so a tilted grip still turns at full speed.
const PLAYER_SPACE_RELAX: f32 = 1.41;
/// Auto-calibration counts the controller as set down when it turns slower than this
/// (degrees/second) and its acceleration stays within `STILL_ACCEL` g of gravity...
const STILL_RATE: f32 = 4.0;
const STILL_ACCEL: f32 = 0.015;
/// ...for this long (seconds), and then learns the drift with this time constant (seconds).
const STILL_TIME: f32 = 0.5;
const DRIFT_LEARNING: f32 = 1.0;
/// A stick counts as pushed, for pausing the gyro, past its deadzone or this, whichever is more.
const PAUSE_PUSH: f32 = 0.15;

#[derive(Default)]
pub(super) struct GyroState {
    /// For `GyroActivation::Toggle`.
    pub(super) toggled_on: bool,
    /// Virtual-pad stick the gyro is deflecting, and by how much.
    pub(super) stick: Option<(Stick, (f32, f32))>,
    pub(super) mouse_acc: (f32, f32),
    /// Low-passed tilt from the accelerometer (degrees), and the tilt counted as straight.
    roll: Option<f32>,
    roll_center: f32,
    /// Low-passed acceleration (g), pointing up when the controller is still.
    gravity: Option<[f32; 3]>,
    /// Drift learned by auto-calibration (degrees/second), and how long the controller has
    /// been still.
    drift: [f32; 3],
    still_for: f32,
    /// The slow part of the aim being smoothed (degrees/second).
    smoothed: (f32, f32),
    /// How far the controller has turned since Position mode started (degrees).
    position: (f32, f32),
    /// The last sample's turning speed and shaped aim while gyro was live, for the settings
    /// page's live view (see [`GyroOut`]).
    pub(super) aim: Option<(f32, (f32, f32))>,
    /// Set when a toggle switches gyro on (true) or off with rumble asked for; the daemon takes it.
    rumble: Option<bool>,
}

impl GyroState {
    /// Follows tilt and gravity from the accelerometer, all the time, so steering is centered
    /// and the spaces are right as soon as gyro is switched on.
    fn track_gravity(&mut self, sample: &MotionSample) {
        let accel = sample.accel;
        if accel == [0.0; 3] {
            return;
        }
        let [ax, ay, _] = accel;
        if ax != 0.0 || ay != 0.0 {
            let tilt = ax.atan2(ay).to_degrees();
            let blend = blend(sample.dt, TILT_SMOOTHING);
            self.roll = Some(match self.roll {
                Some(r) => r + angle_diff(tilt, r) * blend,
                None => tilt,
            });
        }
        let blend = blend(sample.dt, GRAVITY_SMOOTHING);
        self.gravity = Some(match self.gravity {
            Some(g) => [0, 1, 2].map(|i| g[i] + (accel[i] - g[i]) * blend),
            None => accel,
        });
    }

    /// Unit vector pointing up, in the controller's frame; straight up when unknown.
    fn up(&self) -> [f32; 3] {
        self.gravity.and_then(normalized).unwrap_or([0.0, 1.0, 0.0])
    }

    /// Learns drift while the controller lies still, and returns the rates with it removed.
    fn calibrate(&mut self, sample: &MotionSample) -> [f32; 3] {
        let rates = [0, 1, 2].map(|i| sample.gyro[i] - self.drift[i]);
        let steady = self.gravity.is_some_and(|g| (0..3).map(|i| (sample.accel[i] - g[i]).powi(2)).sum::<f32>().sqrt() < STILL_ACCEL);
        if steady && sample.accel != [0.0; 3] && length(rates) < STILL_RATE {
            self.still_for += sample.dt;
        } else {
            self.still_for = 0.0;
        }
        if self.still_for >= STILL_TIME {
            let blend = blend(sample.dt, DRIFT_LEARNING);
            for (d, g) in self.drift.iter_mut().zip(sample.gyro) {
                *d += (g - *d) * blend;
            }
        }
        rates
    }

    /// The rotation as horizontal and vertical turning (degrees/second, positive = left/up
    /// before inverting), in the configured space.
    fn read_space(&self, horizontal: GyroHorizontal, [pitch, yaw, roll]: [f32; 3]) -> (f32, f32) {
        match horizontal {
            GyroHorizontal::Yaw => (yaw, pitch),
            GyroHorizontal::Roll => (roll, pitch),
            GyroHorizontal::YawAndRoll => (yaw + roll, pitch),
            GyroHorizontal::PlayerSpace => {
                let up = self.up();
                let world_yaw = yaw * up[1] + roll * up[2];
                (world_yaw.signum() * (world_yaw.abs() * PLAYER_SPACE_RELAX).min(yaw.hypot(roll)), pitch)
            }
            GyroHorizontal::WorldSpace => {
                let up = self.up();
                let omega = [pitch, yaw, roll];
                // Pitch about the horizon, faded out when the controller is on its side and the
                // horizon is ambiguous.
                let side = ((up[1].abs().max(up[2].abs()) - 0.125) / 0.125).clamp(0.0, 1.0);
                let axis = normalized([1.0 - up[0] * up[0], -up[1] * up[0], -up[2] * up[0]]);
                (dot(omega, up), axis.map_or(0.0, |a| dot(omega, a) * side))
            }
        }
    }

    /// Smooths the slow part of the aim over `smoothing_ms`; faster turning passes straight
    /// through, so smoothing steadies slow aim without delaying flicks.
    fn smooth(&mut self, cfg: &GyroConfig, (x, y): (f32, f32), dt: f32) -> (f32, f32) {
        if cfg.smoothing_threshold <= 0.0 {
            return (x, y);
        }
        let low = cfg.smoothing_threshold / 2.0;
        let direct = ((x.hypot(y) - low) / (cfg.smoothing_threshold - low)).clamp(0.0, 1.0);
        let blend = blend(dt, cfg.smoothing_ms as f32 / 2000.0);
        self.smoothed.0 += (x * (1.0 - direct) - self.smoothed.0) * blend;
        self.smoothed.1 += (y * (1.0 - direct) - self.smoothed.1) * blend;
        (x * direct + self.smoothed.0, y * direct + self.smoothed.1)
    }

    /// The aim from one sample's rates: in the configured space, jitter tightened, inverted,
    /// limited to the chosen axes and smoothed. Degrees/second, positive = right/down.
    fn aim(&mut self, cfg: &GyroConfig, rates: [f32; 3], dt: f32) -> (f32, f32) {
        let (mut h, mut v) = self.read_space(cfg.horizontal, rates);
        if !cfg.axes.horizontal() {
            h = 0.0;
        }
        if !cfg.axes.vertical() {
            v = 0.0;
        }
        // Below the threshold, scale down smoothly so jitter and drift don't move the aim.
        let speed = h.hypot(v);
        let tighten = if cfg.noise_threshold > 0.0 { (speed / cfg.noise_threshold).min(1.0) } else { 1.0 };
        // Turning left or tilting the front up should aim left/up (negative x/y).
        let x = (if cfg.invert_x { h } else { -h }) * tighten;
        let y = (if cfg.invert_y { v } else { -v }) * tighten;
        self.smooth(cfg, (x, y), dt)
    }

    /// Forgets motion that was building up, for recentering and when gyro goes off.
    fn settle(&mut self) {
        self.mouse_acc = (0.0, 0.0);
        self.smoothed = (0.0, 0.0);
        self.position = (0.0, 0.0);
    }

    /// Clears what gyro was doing when everything is released; tilt tracking and learned
    /// drift carry on.
    pub(super) fn reset(&mut self) {
        self.stick = None;
        self.toggled_on = false;
        self.aim = None;
        self.settle();
    }
}

fn blend(dt: f32, time_constant: f32) -> f32 {
    if time_constant <= 0.0 { 1.0 } else { 1.0 - (-dt / time_constant).exp() }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn normalized(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = length(v);
    (len > 1e-3).then(|| v.map(|c| c / len))
}

/// The deflection that makes a game whose stick turns at `deflection^game_curve` of its full rate
/// turn at `(x, y)` (shares of that full rate): the inverse of the game's response.
fn undo_game_curve(x: f32, y: f32, game_curve: f32) -> (f32, f32) {
    let mag = x.hypot(y);
    if mag <= 0.0 {
        return (0.0, 0.0);
    }
    let scaled = mag.min(1.0).powf(1.0 / game_curve.max(0.1)) / mag;
    (x * scaled, y * scaled)
}

/// A stick deflection, clamped to the stick's circle and lifted past a game's own deadzone.
fn deflection(x: f32, y: f32, anti_deadzone: f32) -> (f32, f32) {
    let mag = x.hypot(y);
    if mag <= 0.0 {
        return (0.0, 0.0);
    }
    let scaled = (anti_deadzone + (1.0 - anti_deadzone) * mag.min(1.0)) / mag;
    (x * scaled, y * scaled)
}

impl Engine {
    /// Changes the gyro's stick deflection, re-sending affected sticks.
    pub(super) fn set_gyro_stick(&mut self, value: Option<(Stick, (f32, f32))>, out: &mut Vec<OutEvent>) {
        let old = std::mem::replace(&mut self.gyro.stick, value);
        if old == value {
            return;
        }
        if let Some((s, _)) = old
            && value.is_none_or(|(n, _)| n != s)
        {
            self.emit_pad_stick(s, out);
        }
        if let Some((s, _)) = value {
            self.emit_pad_stick(s, out);
        }
    }

    fn gyro_input_held(&self, input: GyroInput) -> bool {
        let trigger = |axis| self.axes.get(&axis).copied().unwrap_or(0.0) > 0.5;
        match input {
            GyroInput::Button(b) => self.raw_buttons.contains(&b),
            GyroInput::LeftTrigger => trigger(Axis::LeftTrigger),
            GyroInput::RightTrigger => trigger(Axis::RightTrigger),
        }
    }

    /// (toggle input held, recenter input held), for edge detection around each event.
    pub(super) fn gyro_controls_held(&self, cfg: &GyroConfig) -> (bool, bool) {
        let toggle = match cfg.activation {
            GyroActivation::Toggle(i) => self.gyro_input_held(i),
            _ => false,
        };
        (toggle, cfg.recenter.is_some_and(|i| self.gyro_input_held(i)))
    }

    /// After an input: flips the toggle and recenters on a fresh press, and drops the gyro's
    /// stick deflection once gyro is off.
    pub(super) fn gyro_after_input(&mut self, profile: &Profile, before: (bool, bool), out: &mut Vec<OutEvent>) {
        let cfg = &profile.gyro;
        let after = self.gyro_controls_held(cfg);
        if matches!(cfg.activation, GyroActivation::Toggle(_)) && after.0 && !before.0 {
            self.gyro.toggled_on = !self.gyro.toggled_on;
            if cfg.toggle_rumble {
                self.gyro.rumble = Some(self.gyro.toggled_on);
            }
        }
        if after.1 && !before.1 {
            self.gyro.roll_center = self.gyro.roll.unwrap_or(0.0);
            self.gyro.settle();
        }
        if !self.gyro_live(profile) {
            self.gyro_off(out);
        }
    }

    fn gyro_active(&self, cfg: &GyroConfig) -> bool {
        match cfg.activation {
            GyroActivation::Always => true,
            GyroActivation::WhileHeld(i) => self.gyro_input_held(i),
            GyroActivation::UnlessHeld(i) => !self.gyro_input_held(i),
            GyroActivation::Toggle(_) => self.gyro.toggled_on,
        }
    }

    /// Whether the stick that pauses gyro is pushed.
    fn gyro_paused(&self, profile: &Profile) -> bool {
        profile.gyro.pause_with_stick.is_some_and(|s| {
            let (ax, ay) = stick_axes(s);
            let at = |a| self.axes.get(&a).copied().unwrap_or(0.0);
            at(ax).hypot(at(ay)) > profile.stick(s).deadzone.max(PAUSE_PUSH)
        })
    }

    /// Whether gyro output is live: on, activated and not paused.
    fn gyro_live(&self, profile: &Profile) -> bool {
        !matches!(profile.gyro.mode, GyroMode::Off) && self.gyro_active(&profile.gyro) && !self.gyro_paused(profile)
    }

    fn gyro_off(&mut self, out: &mut Vec<OutEvent>) {
        self.set_gyro_stick(None, out);
        if self.gyro.aim.take().is_some() {
            self.gyro.settle();
        }
    }

    /// Whether a toggle just switched gyro on (`Some(true)`) or off, with rumble asked for.
    pub fn take_gyro_rumble(&mut self) -> Option<bool> {
        self.gyro.rumble.take()
    }

    /// What gyro is sending, for the settings page; None while gyro is off.
    pub fn gyro_out(&self) -> Option<GyroOut> {
        self.gyro.aim.map(|(speed, aim)| GyroOut { speed, aim, stick: self.gyro.stick.map(|(_, d)| d) })
    }

    /// Applies one motion-sensor sample (with calibration bias already removed).
    pub fn motion(&mut self, profile: &Profile, sample: MotionSample, out: &mut Vec<OutEvent>) {
        let cfg = &profile.gyro;
        self.gyro.track_gravity(&sample);
        let rates = if cfg.auto_calibrate { self.gyro.calibrate(&sample) } else { sample.gyro };
        if !self.gyro_live(profile) {
            self.gyro_off(out);
            return;
        }
        let (x, y) = self.gyro.aim(cfg, rates, sample.dt);
        let speed = x.hypot(y);
        let gain = if cfg.mode.uses_curve() { cfg.curve.gain(speed) } else { 1.0 };
        let (x, y) = (x * cfg.x_scale * gain, y * cfg.y_scale * gain);
        self.gyro.aim = Some((speed, (x, y)));

        match cfg.mode {
            GyroMode::Off => {}
            GyroMode::Mouse { sensitivity } => {
                // A layer may have switched from stick mode: stop deflecting the stick.
                self.set_gyro_stick(None, out);
                let k = cfg.pixels_per_degree(sensitivity) * sample.dt;
                let (dx, dy) = take_whole(&mut self.gyro.mouse_acc, x * k, y * k);
                if dx != 0 || dy != 0 {
                    out.push(OutEvent::MouseMove(dx, dy));
                }
            }
            GyroMode::Stick { stick, full_rate, anti_deadzone, game_curve } => {
                let (dx, dy) = undo_game_curve(x / full_rate.max(1.0), y / full_rate.max(1.0), game_curve);
                let d = deflection(dx, dy, anti_deadzone);
                self.set_gyro_stick(Some((stick, d)), out);
            }
            GyroMode::Position { stick, max_angle, anti_deadzone, lock_at_edge } => {
                let max = max_angle.max(1.0);
                let p = &mut self.gyro.position;
                p.0 += x * sample.dt;
                p.1 += y * sample.dt;
                // Locked at the edge, turning past it is let go, so turning back moves off it at once.
                let reach = p.0.hypot(p.1) / max;
                if lock_at_edge && reach > 1.0 {
                    p.0 /= reach;
                    p.1 /= reach;
                }
                let d = deflection(p.0 / max, p.1 / max, anti_deadzone);
                self.set_gyro_stick(Some((stick, d)), out);
            }
            GyroMode::Steering { stick, max_angle } => {
                let angle = angle_diff(self.gyro.roll.unwrap_or(0.0), self.gyro.roll_center);
                let mut sx = (angle / max_angle.max(1.0)).clamp(-1.0, 1.0);
                if cfg.invert_x {
                    sx = -sx;
                }
                self.set_gyro_stick(Some((stick, (sx, 0.0))), out);
            }
        }
    }
}

#[cfg(test)]
#[path = "gyro_tests.rs"]
mod tests;
