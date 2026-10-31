//! Normalized controller input: buttons, sticks in -1..1 and triggers in 0..1, whatever the
//! platform reads them from (evdev).

use crate::config::Button;

#[cfg(target_os = "linux")]
mod linux;
mod touchpad;

#[cfg(target_os = "linux")]
pub use linux::{
    MotionNormalizer, Normalizer, TouchNormalizer, has_analog_triggers, has_paddles, inaccessible_motion_sensor, is_gamepad,
    is_motion_sensor, is_touchpad, uses_xbox_labels,
};
pub use touchpad::{DUALSENSE_HEIGHT, DUALSHOCK4_HEIGHT, TouchpadEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Axis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftTrigger,
    RightTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Button(Button, bool),
    /// Sticks are -1.0..1.0 (Y positive = down), triggers 0.0..1.0.
    Axis(Axis, f32),
    Touchpad(TouchpadEvent),
}

/// One motion reading. Axes are the controller's own until [`MotionFrame::to_standard`]; in
/// the standard frame (X right, Y up, Z toward the player) gyro is pitch, yaw, roll.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MotionSample {
    /// Angular velocity in degrees/second.
    pub gyro: [f32; 3],
    /// Acceleration in g, including gravity.
    pub accel: [f32; 3],
    /// Seconds since the previous sample.
    pub dt: f32,
}

/// How a driver orients its motion axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionFrame {
    /// hid-playstation passes the controller's frame through: X right, Y up, Z toward the
    /// player. This is the standard frame the engine works in.
    #[default]
    PlayStation,
    /// hid-nintendo: X toward the triggers, Y left, Z up out of the face.
    #[cfg_attr(not(any(target_os = "linux", test)), expect(dead_code, reason = "only evdev reads Nintendo motion sensors"))]
    Nintendo,
}

impl MotionFrame {
    /// Re-expresses a sample in the standard frame.
    pub fn to_standard(self, sample: MotionSample) -> MotionSample {
        match self {
            MotionFrame::PlayStation => sample,
            MotionFrame::Nintendo => {
                // right = -left, up = up, toward the player = -toward the triggers
                let remap = |[x, y, z]: [f32; 3]| [-y, z, -x];
                MotionSample { gyro: remap(sample.gyro), accel: remap(sample.accel), dt: sample.dt }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nintendo_motion_is_remapped_to_the_standard_frame() {
        let n = |gyro: [f32; 3], accel: [f32; 3]| MotionFrame::Nintendo.to_standard(MotionSample { gyro, accel, dt: 0.0 });
        // Lying flat: gravity is up (+Z for Nintendo), which is +Y in the standard frame.
        assert_eq!(n([0.0; 3], [0.0, 0.0, 1.0]).accel, [0.0, 1.0, 0.0]);
        // Turning left (counter-clockwise from above, +Z for Nintendo) is positive yaw.
        assert_eq!(n([0.0, 0.0, 30.0], [0.0; 3]).gyro, [0.0, 30.0, 0.0]);
        // Lifting the trigger edge rotates about -Y (left) for Nintendo: positive pitch.
        assert_eq!(n([0.0, -30.0, 0.0], [0.0; 3]).gyro, [30.0, 0.0, 0.0]);
        // Rotating about the trigger axis (+X) is about the away-from-player axis: negative roll.
        assert_eq!(n([30.0, 0.0, 0.0], [0.0; 3]).gyro, [0.0, 0.0, -30.0]);
        // PlayStation passes through unchanged.
        let s = MotionSample { gyro: [1.0, 2.0, 3.0], accel: [4.0, 5.0, 6.0], dt: 0.1 };
        assert_eq!(MotionFrame::PlayStation.to_standard(s), s);
    }
}
