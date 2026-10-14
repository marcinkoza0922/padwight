//! Ready-made stick moves for macros: the stick's eight directions and its center, and the
//! motions of fighting games, as the settings window and Edit Controls both offer them.

use super::{MacroStep, Stick};

/// One frame at 60 fps: the usual gap between motion inputs in fighting games.
pub const MOTION_FRAME_MS: u64 = 17;

pub const DIAGONAL: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Stick positions offered for "Move stick" steps (y positive is down).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickPreset {
    Center,
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
    Custom,
}

impl StickPreset {
    pub const ALL: [StickPreset; 10] = [
        StickPreset::Center,
        StickPreset::Up,
        StickPreset::UpRight,
        StickPreset::Right,
        StickPreset::DownRight,
        StickPreset::Down,
        StickPreset::DownLeft,
        StickPreset::Left,
        StickPreset::UpLeft,
        StickPreset::Custom,
    ];

    pub fn position(self) -> Option<(f32, f32)> {
        let d = DIAGONAL;
        Some(match self {
            StickPreset::Center => (0.0, 0.0),
            StickPreset::Up => (0.0, -1.0),
            StickPreset::UpRight => (d, -d),
            StickPreset::Right => (1.0, 0.0),
            StickPreset::DownRight => (d, d),
            StickPreset::Down => (0.0, 1.0),
            StickPreset::DownLeft => (-d, d),
            StickPreset::Left => (-1.0, 0.0),
            StickPreset::UpLeft => (-d, -d),
            StickPreset::Custom => return None,
        })
    }

    pub fn of(x: f32, y: f32) -> Self {
        Self::ALL
            .into_iter()
            .find(|p| p.position().is_some_and(|(px, py)| (px - x).abs() < 0.01 && (py - y).abs() < 0.01))
            .unwrap_or(StickPreset::Custom)
    }
}

impl std::fmt::Display for StickPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            StickPreset::Center => "● Center",
            StickPreset::Up => "↑ Up",
            StickPreset::UpRight => "↗ Up-right",
            StickPreset::Right => "→ Right",
            StickPreset::DownRight => "↘ Down-right",
            StickPreset::Down => "↓ Down",
            StickPreset::DownLeft => "↙ Down-left",
            StickPreset::Left => "← Left",
            StickPreset::UpLeft => "↖ Up-left",
            StickPreset::Custom => "Custom",
        })
    }
}

/// Fighting-game motions, written for a character facing right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    QuarterCircleForward,
    QuarterCircleBack,
    DragonPunch,
    HalfCircleForward,
    HalfCircleBack,
}

impl Motion {
    pub const ALL: [Motion; 5] = [
        Motion::QuarterCircleForward,
        Motion::QuarterCircleBack,
        Motion::DragonPunch,
        Motion::HalfCircleForward,
        Motion::HalfCircleBack,
    ];

    pub fn directions(self) -> &'static [StickPreset] {
        use StickPreset::*;
        match self {
            Motion::QuarterCircleForward => &[Down, DownRight, Right],
            Motion::QuarterCircleBack => &[Down, DownLeft, Left],
            Motion::DragonPunch => &[Right, Down, DownRight],
            Motion::HalfCircleForward => &[Left, DownLeft, Down, DownRight, Right],
            Motion::HalfCircleBack => &[Right, DownRight, Down, DownLeft, Left],
        }
    }

    /// Steps moving the left stick through the motion, one frame per direction.
    pub fn steps(self) -> Vec<MacroStep> {
        self.directions()
            .iter()
            .filter_map(|p| p.position())
            .flat_map(|(x, y)| [MacroStep::Stick { stick: Stick::Left, x, y }, MacroStep::Wait(MOTION_FRAME_MS)])
            .collect()
    }
}

impl std::fmt::Display for Motion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Motion::QuarterCircleForward => "Quarter circle forward (↓↘→)",
            Motion::QuarterCircleBack => "Quarter circle back (↓↙←)",
            Motion::DragonPunch => "Dragon punch (→↓↘)",
            Motion::HalfCircleForward => "Half circle forward (←↙↓↘→)",
            Motion::HalfCircleBack => "Half circle back (→↘↓↙←)",
        })
    }
}
