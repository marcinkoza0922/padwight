//! Controllers on Windows: XInput's four slots, each read on its own thread.
//!
//! Windows can't hide a controller from other programs the way evdev's grab does, so games see
//! the physical pad next to our virtual one. HidHide (see the README) hides it; without it, a
//! game that reads every pad gets both.

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use super::{Daemon, Msg, SeenGamepad};
use crate::controller_id::controller_id;
use crate::{
    input::xinput::{self, Controller, XInputNormalizer},
    monitor::log,
    output::{VirtualPad, is_virtual_slot, slots_known},
    pad_identity::PadIdentity,
    rumble,
};

/// How often a controller's state is read. XInput has no events to wait on.
const POLL: Duration = Duration::from_millis(2);

/// A controller's slot; slots don't get recreated the way device nodes do, so the second half is
/// always 0.
pub(super) type NodeKey = (PathBuf, u64);

/// Windows has no separate motion-sensor or touchpad nodes to pair with a controller.
#[derive(Default)]
pub(super) struct Nodes {}

/// A plain Xbox 360 pad, for an injected controller whose output goes out for real.
pub(super) fn live_pad() -> anyhow::Result<VirtualPad> {
    VirtualPad::new()
}

impl Nodes {
    /// XInput pads have no motion sensors to be denied.
    pub(super) fn denied_sensors(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A controller's rumble, which its virtual pad passes on. Every XInput pad that rumbles has the
/// same two motors, so there's nothing more to keep.
pub struct FfSpec;

/// The virtual pad a controller feeds, presenting itself as `identity`.
pub fn virtual_pad_for(_ff: Option<&FfSpec>, identity: PadIdentity) -> Option<VirtualPad> {
    match VirtualPad::with_identity(identity) {
        Ok(pad) => Some(pad),
        Err(e) => {
            log!("cannot create virtual pad: {e:#}");
            None
        }
    }
}

impl Daemon {
    /// Looks at XInput's slots for new controllers and starts reading the ones we should
    /// manage. Our own virtual pads take slots too, and are passed over.
    pub(super) fn scan_devices(&mut self) {
        let mut present = HashSet::new();
        for slot in 0..xinput::SLOTS {
            if is_virtual_slot(slot) {
                continue;
            }
            let Some(controller) = xinput::controller(slot) else { continue };
            let path = xinput::slot_path(slot);
            let key = (path.clone(), 0);
            present.insert(key.clone());
            if self.devices.values().any(|d| d.path == path) {
                continue;
            }
            let name = controller.name(slot);
            let id = controller_id(controller.vendor, controller.product, None);
            self.gamepads.insert(
                key,
                SeenGamepad { name: name.clone(), id: id.clone(), analog_triggers: true, rumble: controller.rumble, paddles: false },
            );
            if !self.should_manage(&id) {
                continue;
            }
            // A virtual pad whose slot isn't known yet may be this one.
            if !slots_known() {
                continue;
            }
            self.manage(slot, name, id, &controller);
        }
        self.gamepads.retain(|k, _| present.contains(k));
    }

    fn manage(&mut self, slot: u32, name: String, pad_id: String, controller: &Controller) {
        let path = xinput::slot_path(slot);
        let family = crate::info::PadFamily::detect(controller.vendor, &name).or(Some(crate::info::PadFamily::Xbox));
        let model = crate::info::PadModel::detect(controller.vendor, controller.product, &name);
        let ff = controller.rumble.then_some(FfSpec);
        let identity = PadIdentity::resolve(self.identity_override, model, self.active_support());
        // No pad is kept on Windows (see `first_pad.rs`), so this is always a new one.
        let Some(new_pad) = self.pad_for_new(ff.as_ref(), identity, self.config.controller(&pad_id).and_then(|c| c.player)) else { return };
        let pad = new_pad.pad;
        log!("managing {name} ({})", path.display());
        let id = self.next_id;
        self.next_id += 1;
        let stop = Arc::new(AtomicBool::new(false));
        {
            let stop = stop.clone();
            let tx = self.tx.clone();
            thread::spawn(move || read_slot(id, slot, &stop, &tx));
        }
        let rumble_stop = Arc::new(AtomicBool::new(false));
        if ff.is_some() {
            rumble::spawn(pad.shared(), path.clone(), rumble_stop.clone());
        }
        let mut managed = self.new_managed(path, name, Some(pad), stop);
        managed.first = new_pad.first;
        managed.refit_at = new_pad.refit_at;
        managed.ff = ff;
        managed.rumble_stop = rumble_stop;
        managed.family = family;
        managed.model = model;
        managed.id = pad_id;
        self.add_managed(id, managed);
    }
}

/// Reads the controller in `slot` until `stop` is set or it disconnects, sending what changed to
/// the controller `id`.
fn read_slot(id: u64, slot: u32, stop: &AtomicBool, tx: &super::inbox::Tx) {
    let mut norm = XInputNormalizer::default();
    let mut events = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        crate::priority::keep_up();
        let Some(state) = xinput::state(slot) else { break };
        norm.translate(&state, &mut events);
        if !events.is_empty() && tx.send(Msg::Input { id, events: std::mem::take(&mut events), stamp: 0 }).is_err() {
            return;
        }
        thread::sleep(POLL);
    }
    let _ = tx.send(Msg::Gone { id });
}
