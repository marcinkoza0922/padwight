//! The random steps of a soak run, and the controllers and processes they act on.

use std::{
    path::PathBuf,
    process::Command,
    thread,
    time::Duration,
};

use serde_json::{Value, json};

use crate::{
    common::{daemon_log, uhid::HidPad},
    probe::{overlay_pids, request},
};

/// A seeded xorshift64* generator: the same seed gives the same steps.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 once, so small seeds don't start the generator in a poor state.
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Rng((z ^ (z >> 31)) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A value in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        (self.next() >> 11) % n
    }
}

/// A plugged controller and what it is holding.
struct Plugged {
    hid: HidPad,
    buttons: u8,
    x: i16,
    y: i16,
}

pub struct World {
    root: PathBuf,
    pub runtime: PathBuf,
    pub pid: u32,
    slots: Vec<Option<Plugged>>,
    /// Overlay processes this run has stopped with SIGSTOP.
    stopped: Vec<u32>,
    focus_hook: Option<String>,
    focus_classes: Vec<String>,
}

const PROFILES: [&str; 2] = ["Gamepad", "Desktop"];

impl World {
    pub fn new(root: PathBuf, runtime: PathBuf, pid: u32, pads: usize) -> Self {
        let classes = std::env::var("PADWIGHT_SOAK_FOCUS_CLASSES").unwrap_or_else(|_| "game-a,game-b,none".into());
        World {
            root,
            runtime,
            pid,
            slots: (0..pads).map(|_| None).collect(),
            stopped: Vec::new(),
            focus_hook: std::env::var("PADWIGHT_SOAK_FOCUS_HOOK").ok().filter(|h| !h.is_empty()),
            focus_classes: classes.split(',').map(str::to_string).collect(),
        }
    }

    pub fn log(&self) -> String {
        daemon_log(&self.root)
    }

    pub fn plug(&mut self, slot: usize) {
        if self.slots[slot].is_none() {
            let hid = HidPad::with_uniq(&format!("Soak Pad {slot}"), &format!("padwight-soak-{slot}"));
            self.slots[slot] = Some(Plugged { hid, buttons: 0, x: 0, y: 0 });
        }
    }

    fn send(&self, request_json: &Value) -> String {
        request(&self.runtime, &request_json.to_string()).map_or_else(|| "no answer".into(), |r| r.trim().chars().take(60).collect())
    }

    /// Lets go of everything: every pad's buttons and sticks, the overlays, SIGSTOPped processes.
    pub fn release_everything(&mut self) {
        self.cont_all();
        for _ in 0..2 {
            for pad in self.slots.iter_mut().flatten() {
                pad.buttons = 0;
                pad.x = 0;
                pad.y = 0;
                pad.hid.report(0, 0, 0);
            }
            thread::sleep(Duration::from_millis(100));
        }
        self.send(&json!("CloseOverlays"));
        self.send(&json!({"SetEnabled": true}));
        self.send(&json!({"SetProfileLock": false}));
        thread::sleep(Duration::from_millis(500));
    }

    /// The end of the run: nothing left stopped.
    pub fn finish(&mut self) {
        self.cont_all();
        self.slots.iter_mut().for_each(|s| *s = None);
    }

    fn cont_all(&mut self) {
        for pid in self.stopped.drain(..) {
            // SAFETY: a signal to a child process of the daemon, which may have gone already.
            unsafe { libc::kill(pid as libc::pid_t, libc::SIGCONT) };
        }
    }

    fn signal_overlay(&mut self, rng: &mut Rng, signal: libc::c_int) -> String {
        let pids = overlay_pids(self.pid);
        let Some(&pid) = pids.get(rng.below(pids.len().max(1) as u64) as usize) else { return "no overlay process".into() };
        // SAFETY: a signal to a child process of the daemon.
        unsafe { libc::kill(pid as libc::pid_t, signal) };
        if signal == libc::SIGSTOP {
            self.stopped.push(pid);
        }
        format!("pid index of {} overlay process(es)", pids.len())
    }

    /// Runs one random step, then waits a random moment. Returns what it did.
    pub fn step(&mut self, rng: &mut Rng) -> String {
        let pick = rng.below(100);
        let slot = rng.below(self.slots.len() as u64) as usize;
        let what = match pick {
            0..=29 => self.press(rng, slot),
            30..=37 => match self.slots[slot].as_mut() {
                Some(pad) => {
                    pad.buttons = 0;
                    pad.x = 0;
                    pad.y = 0;
                    pad.hid.report(0, 0, 0);
                    format!("release pad {slot}")
                }
                None => format!("release pad {slot}: not plugged"),
            },
            38..=43 => self.sweep(rng, slot),
            44..=49 => {
                self.plug(slot);
                format!("plug pad {slot}")
            }
            50..=55 => {
                self.slots[slot] = None;
                format!("unplug pad {slot}")
            }
            56..=60 => {
                let name = PROFILES[rng.below(2) as usize];
                format!("SetProfile {name}: {}", self.send(&json!({"SetProfile": name})))
            }
            61..=62 => {
                let name = PROFILES[rng.below(2) as usize];
                format!("Activate {name}: {}", self.send(&json!({"Activate": {"profile": name}})))
            }
            63..=64 => format!("NextProfile: {}", self.send(&json!("NextProfile"))),
            65 => format!("SetProfileLock: {}", self.send(&json!({"SetProfileLock": rng.below(2) == 0}))),
            66..=68 => format!("Reload: {}", self.send(&json!("Reload"))),
            69..=71 => self.set_config(rng),
            72..=73 => format!("SetEnabled false then true: {} {}", self.send(&json!({"SetEnabled": false})), self.send(&json!({"SetEnabled": true}))),
            74..=78 => format!("ToggleOverlay: {}", self.send(&json!("ToggleOverlay"))),
            79..=82 => format!("ToggleNumpad: {}", self.send(&json!("ToggleNumpad"))),
            83..=86 => format!("CloseOverlays: {}", self.send(&json!("CloseOverlays"))),
            87..=88 => format!("SIGKILL overlay: {}", self.signal_overlay(rng, libc::SIGKILL)),
            89..=90 => format!("SIGSTOP overlay: {}", self.signal_overlay(rng, libc::SIGSTOP)),
            91..=93 => {
                self.cont_all();
                "SIGCONT overlays".into()
            }
            94..=96 => self.focus(rng),
            _ => format!("Status: {}", self.send(&json!("Status"))),
        };
        thread::sleep(Duration::from_millis(5 + rng.below(120)));
        what
    }

    /// A random button state (South, East, North, West) and stick position on one pad.
    fn press(&mut self, rng: &mut Rng, slot: usize) -> String {
        let (buttons, x, y) = (rng.below(16) as u8, rng.below(65536) as i64 - 32768, rng.below(65536) as i64 - 32768);
        let Some(pad) = self.slots[slot].as_mut() else { return format!("press pad {slot}: not plugged") };
        pad.buttons = buttons;
        pad.x = if rng.below(2) == 0 { x as i16 } else { 0 };
        pad.y = if rng.below(2) == 0 { y as i16 } else { 0 };
        pad.hid.report(pad.buttons, pad.x, pad.y);
        format!("press pad {slot}: buttons {buttons:04b}, stick ({}, {})", pad.x, pad.y)
    }

    /// A burst of stick reports, a millisecond apart.
    fn sweep(&mut self, rng: &mut Rng, slot: usize) -> String {
        let count = 20 + rng.below(100);
        let Some(pad) = self.slots[slot].as_mut() else { return format!("sweep pad {slot}: not plugged") };
        for _ in 0..count {
            pad.x = pad.x.wrapping_add(1500);
            pad.y = pad.y.wrapping_sub(900);
            pad.hid.report(pad.buttons, pad.x, pad.y);
            thread::sleep(Duration::from_millis(1));
        }
        format!("sweep pad {slot}: {count} reports")
    }

    /// Reads the config, changes a timing in it and sends it back.
    fn set_config(&mut self, rng: &mut Rng) -> String {
        let Some(reply) = request(&self.runtime, "\"GetConfig\"") else { return "SetConfig: no config".into() };
        let Some(mut config) = serde_json::from_str::<Value>(&reply).ok().and_then(|v| v.get("Config").cloned()) else { return "SetConfig: no config".into() };
        let value = 400 + rng.below(300);
        let changed = set_every(&mut config, "long_press_ms", value);
        format!("SetConfig long_press_ms={value} ({changed} places): {}", self.send(&json!({"SetConfig": config})))
    }

    /// Runs the focus hook (T-7's stand-in compositor) with a window class.
    fn focus(&mut self, rng: &mut Rng) -> String {
        let class = self.focus_classes[rng.below(self.focus_classes.len() as u64) as usize].clone();
        let Some(hook) = &self.focus_hook else { return "focus: no hook set".into() };
        let ok = Command::new(hook).arg(&class).status().is_ok_and(|s| s.success());
        format!("focus {class}: hook {}", if ok { "ok" } else { "failed" })
    }
}

/// Sets every number called `key` in `value` and returns how many it set.
fn set_every(value: &mut Value, key: &str, to: u64) -> usize {
    match value {
        Value::Object(map) => {
            let mut n = 0;
            for (k, v) in map.iter_mut() {
                if k == key && v.is_number() {
                    *v = json!(to);
                    n += 1;
                } else {
                    n += set_every(v, key, to);
                }
            }
            n
        }
        Value::Array(items) => items.iter_mut().map(|v| set_every(v, key, to)).sum(),
        _ => 0,
    }
}
