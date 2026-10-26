//! Pictures of settings, drawn as SVG: a stick's deadzone, a mouse stick's response, a button
//! ring, the gyro, a trigger's travel, and a mouse region on the desktop. The settings window and the in-game editor both draw
//! them.

pub mod deadzone;
pub mod gyro;
pub mod region;
pub mod response;
pub mod ring;
pub mod trigger;
mod figure;

pub use figure::{Figure, Live, MouseShown};
