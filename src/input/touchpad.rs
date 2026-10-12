//! Touchpads on controllers (DualShock 4, DualSense). Each finger is reported on its own, by its
//! contact slot.

use serde::{Deserialize, Serialize};

/// What a touchpad did.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TouchpadEvent {
    /// The click, pressed or released.
    Click(bool),
    /// A finger is down at (`x`, `y`), each 0..1 across the pad from its top left corner. Sent
    /// when it lands and each time it moves. `height` is the pad's height in pad widths, so
    /// movement can be measured the same way across and down.
    Touch { finger: u8, x: f32, y: f32, height: f32 },
    /// The finger left the pad.
    Lift { finger: u8 },
}

/// The pad's height in widths: a DualShock 4's pad is 1920 x 942, a DualSense's 1920 x 1080.
pub const DUALSHOCK4_HEIGHT: f32 = 942.0 / 1920.0;
pub const DUALSENSE_HEIGHT: f32 = 1080.0 / 1920.0;
