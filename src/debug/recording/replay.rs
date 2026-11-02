//! `padwight debug replay`: playing a recording back through virtual devices.
//!
//! [`play`] only knows two seams: a [`Clock`] that says when each frame is due and a [`Sink`] that
//! delivers a frame. The real ones sleep and write to uinput or uhid; a test (or a fake clock in the
//! debug daemon) can stand in for either.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, EventType, InputEvent, InputId, KeyCode, MiscCode, PropType, RelativeAxisCode, SwitchCode, UinputAbsSetup,
    uinput::{VirtualDevice, VirtualDeviceBuilder},
};

use super::model::{DeviceSpec, Event, Recording};

/// Decides when a moment of the recording (microseconds from its start) has come.
pub trait Clock {
    /// Returns when `micros` has been reached; a fake clock jumps there at once.
    fn wait_until(&mut self, micros: u64);
}

/// Delivers one frame: the events of one device up to a `SYN_REPORT`, which is not among them.
pub trait Sink {
    fn emit(&mut self, device: usize, frame: &[Event]) -> Result<()>;
}

/// Wall-clock time, `speed` times as fast as recorded.
pub struct RealClock {
    start: Instant,
    speed: f64,
}

impl RealClock {
    pub fn new(speed: f64) -> Self {
        Self { start: Instant::now(), speed }
    }
}

impl Clock for RealClock {
    fn wait_until(&mut self, micros: u64) {
        let due = self.start + Duration::from_secs_f64(micros as f64 / 1e6 / self.speed);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
}

/// Plays every frame of `rec` at its time. Returns the number of frames.
pub fn play(rec: &Recording, clock: &mut dyn Clock, sink: &mut dyn Sink) -> Result<usize> {
    let mut pending: Vec<Vec<Event>> = vec![Vec::new(); rec.devices.len()];
    let mut frames = 0;
    for ev in &rec.events {
        if ev.is_sync() {
            let frame = std::mem::take(&mut pending[ev.device]);
            if !frame.is_empty() {
                clock.wait_until(ev.micros);
                sink.emit(ev.device, &frame)?;
                frames += 1;
            }
        } else {
            pending[ev.device].push(*ev);
        }
    }
    // Events with no SYN_REPORT after them (an evemu file cut short) are sent at the end.
    for (device, frame) in pending.into_iter().enumerate() {
        if !frame.is_empty() {
            clock.wait_until(frame.last().map_or(0, |e| e.micros));
            sink.emit(device, &frame)?;
            frames += 1;
        }
    }
    Ok(frames)
}

/// The set of codes `$codes` as an attribute set of `$kind`.
macro_rules! set {
    ($codes:expr, $kind:ident) => {
        $codes.iter().map(|&c| $kind(c)).collect::<AttributeSet<$kind>>()
    };
}

/// A uinput device with the recorded identity and capabilities. (uinput cannot set the unique ID
/// or the force-feedback effects.)
pub fn uinput_device(spec: &DeviceSpec) -> Result<VirtualDevice> {
    let mut b: VirtualDeviceBuilder = VirtualDevice::builder()?
        .name(&spec.name)
        .input_id(InputId::new(BusType(spec.bus), spec.vendor, spec.product, spec.version));
    if !spec.props.is_empty() {
        b = b.with_properties(&set!(spec.props, PropType))?;
    }
    if !spec.keys.is_empty() {
        b = b.with_keys(&set!(spec.keys, KeyCode))?;
    }
    if !spec.rel.is_empty() {
        b = b.with_relative_axes(&set!(spec.rel, RelativeAxisCode))?;
    }
    for a in &spec.abs {
        b = b.with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode(a.code), AbsInfo::new(0, a.min, a.max, a.fuzz, a.flat, a.res)))?;
    }
    if !spec.msc.is_empty() {
        b = b.with_msc(&set!(spec.msc, MiscCode))?;
    }
    if !spec.sw.is_empty() {
        b = b.with_switches(&set!(spec.sw, SwitchCode))?;
    }
    b.build().with_context(|| format!("creating a uinput device for {} (needs /dev/uinput)", spec.name))
}

/// The recording's devices as uinput devices.
pub struct UinputSink {
    devices: Vec<VirtualDevice>,
}

impl UinputSink {
    pub fn new(rec: &Recording) -> Result<Self> {
        Ok(Self { devices: rec.devices.iter().map(uinput_device).collect::<Result<_>>()? })
    }
}

impl Sink for UinputSink {
    fn emit(&mut self, device: usize, frame: &[Event]) -> Result<()> {
        let events: Vec<InputEvent> = frame.iter().map(|e| InputEvent::new(e.kind, e.code, e.value)).collect();
        debug_assert!(frame.iter().all(|e| EventType(e.kind) != EventType::SYNCHRONIZATION));
        self.devices[device].emit(&events).context("writing to a uinput device")
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use crate::debug::recording::model::{EV_KEY, SYN_REPORT};

    /// One log both fakes write to, so the order of waits and frames shows.
    type Log = Rc<RefCell<Vec<String>>>;
    struct FakeClock(Log);
    struct FakeSink(Log);
    impl Clock for FakeClock {
        fn wait_until(&mut self, micros: u64) {
            self.0.borrow_mut().push(format!("@{micros}"));
        }
    }
    impl Sink for FakeSink {
        fn emit(&mut self, device: usize, frame: &[Event]) -> Result<()> {
            self.0.borrow_mut().push(format!("{device}:{}", frame.iter().map(|e| format!("{}={}", e.code, e.value)).collect::<Vec<_>>().join(",")));
            Ok(())
        }
    }

    fn ev(micros: u64, device: usize, kind: u16, code: u16, value: i32) -> Event {
        Event { micros, device, kind, code, value }
    }

    #[test]
    fn frames_are_cut_at_each_devices_syn_report_and_timed() {
        let rec = Recording {
            devices: vec![DeviceSpec::default(), DeviceSpec::default()],
            events: vec![
                ev(10, 0, EV_KEY, 304, 1),
                ev(12, 1, 3, 3, 9),
                ev(15, 0, 3, 0, -5),
                ev(15, 0, 0, SYN_REPORT, 0),
                ev(20, 1, 0, SYN_REPORT, 0),
                ev(30, 0, 0, SYN_REPORT, 0),
                ev(40, 0, EV_KEY, 304, 0),
            ],
        };
        let log = Log::default();
        let frames = play(&rec, &mut FakeClock(log.clone()), &mut FakeSink(log.clone())).unwrap();
        assert_eq!(frames, 3);
        // An empty frame (device 0 at 30) is skipped; a trailing one goes at its last event.
        assert_eq!(*log.borrow(), ["@15", "0:304=1,0=-5", "@20", "1:3=9", "@40", "0:304=0"]);
    }
}
