//! Connects the virtual controller to the daemon (`padwight debug --attach`): the daemon runs it
//! as a controller of its own, so mappings, layers, menus and overlays react to it.
//!
//! Changes are sent as they are made, on the thread that made them, and the daemon answers once
//! it has handled them. So a command returns only when its input has had its effect, and a test
//! can read the output right after.

use std::{
    sync::{Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

use super::server::Session;
use crate::{
    info::PadModel,
    ipc::{self, DebugEvent, Request, Response},
    pad_identity::PadIdentity,
};

/// The controller the daemon runs for this session.
pub struct Bridge {
    /// The daemon's name for it.
    path: String,
    model: Option<PadModel>,
    live: bool,
    /// A request failed, so the daemon may have forgotten this controller (it restarted) or
    /// missed a change. The next flush attaches a fresh one and gives it the whole state.
    stale: bool,
}

/// Where a session stands with the daemon.
#[derive(Default)]
pub struct Link(Mutex<Option<Bridge>>);

impl Link {
    fn lock(&self) -> MutexGuard<'_, Option<Bridge>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One request to the daemon, with its failures in terms of the debug session.
fn ask(req: &Request) -> Result<Response> {
    ipc::request(req).context("the daemon refused or could not be reached")
}

fn attach_pad(model: Option<PadModel>, live: bool) -> Result<String> {
    match ask(&Request::DebugAttach { model, live })? {
        Response::Attached(path) => Ok(path),
        _ => bail!("unexpected reply from the daemon"),
    }
}

impl Session {
    /// Has the daemon run this pad as a controller, from now on. `live` also sends its output
    /// to the real virtual devices. The daemon must run with `--debug`.
    pub fn attach(&self, live: bool) -> Result<String> {
        let mut link = self.link.lock();
        let model = {
            let mut pad = self.pad();
            pad.track_events();
            pad.model
        };
        let path = attach_pad(model, live)?;
        // Whatever is already held starts out held. The link stays locked until that is sent, and
        // the changes queued so far are dropped (the rebuild has them): a flush in between would
        // send a press the rebuild sends again, and a button's On press would fire twice.
        let rebuilt = {
            let mut pad = self.pad();
            pad.take_events();
            pad.state_events()
        };
        let stale = self.send(&path, rebuilt).is_err();
        *link = Some(Bridge { path: path.clone(), model, live, stale });
        Ok(path)
    }

    /// Removes the controller from the daemon, letting go of what it held.
    pub fn detach(&self) {
        if let Some(bridge) = self.link.lock().take() {
            let _ = ipc::request(&Request::DebugDetach(bridge.path));
        }
    }

    /// The daemon's name for the controller, while attached.
    pub fn attached(&self) -> Option<String> {
        self.link.lock().as_ref().map(|b| b.path.clone())
    }

    /// Sends the changes made to the pad since the last time. Changing the model replaces the
    /// controller at the daemon, keeping what is held, and so does a failed send: whether the
    /// daemon restarted or only missed the change, the fresh controller is back in step with
    /// the pad. While the daemon can't take one, every flush tries again.
    pub fn flush(&self) -> Result<()> {
        let mut link = self.link.lock();
        let Some(bridge) = link.as_mut() else { return Ok(()) };
        let (events, model) = {
            let mut pad = self.pad();
            (pad.take_events(), pad.model)
        };
        if bridge.stale || model != bridge.model {
            return self.reattach(bridge, model);
        }
        self.send(&bridge.path, events).or_else(|e| self.reattach(bridge, model).map_err(|_| e))
    }

    /// Replaces the daemon's controller with a fresh one of `model`, holding what the pad holds.
    fn reattach(&self, bridge: &mut Bridge, model: Option<PadModel>) -> Result<()> {
        bridge.stale = true;
        // Lets go of what the old one held; after a restart the daemon has no such controller.
        let _ = ipc::request(&Request::DebugDetach(bridge.path.clone()));
        bridge.path = attach_pad(model, bridge.live)?;
        bridge.model = model;
        bridge.stale = false;
        let rebuilt = self.pad().state_events();
        self.send(&bridge.path, rebuilt).inspect_err(|_| bridge.stale = true)
    }

    fn send(&self, path: &str, events: Vec<DebugEvent>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        ask(&Request::DebugInput { path: path.to_string(), events }).map(drop)
    }

    /// What the daemon's mappings have output for this controller (see `padwight debug help`).
    /// When the daemon doesn't know the controller (it restarted), a fresh one is attached, which
    /// has output nothing yet.
    pub fn output(&self, clear: bool) -> Result<Vec<String>> {
        let ask_output = |path: &str| match ask(&Request::DebugOutput { path: path.to_string(), clear })? {
            Response::Lines(lines) => Ok(lines),
            _ => bail!("unexpected reply from the daemon"),
        };
        let mut link = self.link.lock();
        let Some(bridge) = link.as_mut() else { bail!("not attached to the daemon (start the session with --attach)") };
        if !bridge.stale
            && let Ok(lines) = ask_output(&bridge.path)
        {
            return Ok(lines);
        }
        let model = self.pad().model;
        self.reattach(bridge, model)?;
        ask_output(&bridge.path)
    }

    /// Makes every virtual pad on the daemon present as `identity` (`None` lets it choose). This
    /// is daemon-wide, so it doesn't need the session to be attached. The daemon must run with
    /// `--debug`.
    pub fn identify(&self, identity: Option<PadIdentity>) -> Result<()> {
        match ask(&Request::DebugIdentify(identity))? {
            Response::Ok => Ok(()),
            Response::Error(e) => bail!(e),
            _ => bail!("unexpected reply from the daemon"),
        }
    }

    /// The daemon's transcript (see `padwight debug help`); needs no controller.
    pub fn transcript(&self, clear: bool) -> Result<Vec<String>> {
        match ask(&Request::DebugTranscript { clear })? {
            Response::Lines(lines) => Ok(lines),
            Response::Error(e) => bail!(e),
            _ => bail!("unexpected reply from the daemon"),
        }
    }

    /// Moves a `--fake-clock` daemon's clock forward `ms`, running everything due on the way.
    pub fn advance(&self, ms: u64) -> Result<()> {
        match ask(&Request::DebugAdvance(ms))? {
            Response::Ok => Ok(()),
            Response::Error(e) => bail!(e),
            _ => bail!("unexpected reply from the daemon"),
        }
    }

    /// Whether the daemon this session is attached to runs on the manual clock. Asked once.
    fn fake_clock(&self) -> bool {
        if self.attached().is_none() {
            return false;
        }
        if let Some(known) = self.fake.get() {
            return *known;
        }
        let manual = matches!(ask(&Request::DebugClock), Ok(Response::Lines(l)) if l.first().is_some_and(|s| s == "manual"));
        let _ = self.fake.set(manual);
        manual
    }

    /// Lets `ms` pass: on the daemon's manual clock if it has one, otherwise by waiting.
    pub fn pause(&self, ms: u64) -> Result<()> {
        if self.fake_clock() {
            self.advance(ms)
        } else {
            std::thread::sleep(Duration::from_millis(ms));
            Ok(())
        }
    }

    /// Turns the pad at `rates` (degrees per second) for `ms`, as a stream of motion readings
    /// like a real controller's, then lets it stop.
    pub fn turn(&self, rates: [f32; 3], ms: u64) -> Result<()> {
        const STEP: Duration = Duration::from_millis(4);
        let Some(path) = self.attached() else { return Ok(()) };
        self.pad().set_gyro(Some(rates));
        let fake = self.fake_clock();
        let (start, mut last) = (Instant::now(), Instant::now());
        let mut passed = 0u64;
        let result = loop {
            let step_ms = if fake {
                // Exact: the daemon's time moves by the reading's length.
                let step_ms = STEP.as_millis() as u64;
                if let Err(e) = self.advance(step_ms) {
                    break Err(e);
                }
                passed += step_ms;
                step_ms as u32
            } else {
                std::thread::sleep(STEP);
                let now = Instant::now();
                let d = now.duration_since(last).as_millis().max(1) as u32;
                last = now;
                d
            };
            let step = DebugEvent::Motion { gyro: rates, ms: step_ms };
            if let Err(e) = self.send(&path, vec![step]) {
                if let Some(bridge) = self.link.lock().as_mut() {
                    bridge.stale = true;
                }
                break Err(e);
            }
            let elapsed = if fake { Duration::from_millis(passed) } else { start.elapsed() };
            if elapsed >= Duration::from_millis(ms) {
                break Ok(());
            }
        };
        self.pad().set_gyro(None);
        result
    }
}
