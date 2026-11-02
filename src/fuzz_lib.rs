//! The crate root of the fuzz targets (`fuzz/`): the same modules as `main.rs`, built as a library
//! so `cargo fuzz` can reach them. It is not part of the normal build (`fuzz/Cargo.toml` points at
//! it); keep this module list equal to `main.rs`'s.
#![allow(warnings, reason = "the whole program is compiled, but only the fuzz targets use any of it")]

pub mod capture;
pub mod clipboard;
pub mod config;
pub mod controller_id;
pub mod daemon;
pub mod debug;
#[cfg(target_os = "linux")]
pub mod doctor;
pub mod engine;
pub mod figures;
pub mod focus;
pub mod assets;
pub mod font;
pub mod gui;
pub mod guide_menu;
pub mod tools;
pub mod icon;
pub mod icons;
pub mod info;
pub mod inputlog;
pub mod input;
pub mod ipc;
pub mod keyboard;
pub mod keycode;
pub mod keyword_icon;
pub mod launchers;
pub mod library;
pub mod media;
pub mod new_pad;
pub mod offer;
pub mod opener_watch;
pub mod menu;
pub mod nav;
pub mod monitor;
pub mod motion;
pub mod output;
pub mod overlay;
pub mod pack;
pub mod pad_identity;
pub mod pad_svg;
pub mod pad_widget;
pub mod priority;
pub mod radial;
pub mod quit;
pub mod record;
pub mod rumble;
pub mod style;
pub mod layout_editor;
pub mod snapshot;
pub mod sound;
pub mod system_menu;
pub mod touchpad_widget;
pub mod tray;
pub mod update;

pub mod fuzz_targets;
