//! Reading the controllers on the main loop's own thread. The loop waits on every managed
//! controller's nodes (gamepad, touchpad, motion sensors) and on its inbox together, and handles
//! input the moment it arrives: one thread wakes per press, not a reader thread and then the loop.

use std::{
    io::ErrorKind,
    os::fd::AsRawFd,
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, UNIX_EPOCH},
};

use evdev::Device;

use super::{Daemon, Msg, input_timing};
use crate::input::{InputEvent, MotionNormalizer, MotionSample, Normalizer, TouchNormalizer};

/// Which of a controller's nodes a reader reads. A controller has one of each at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Pad,
    Touchpad,
    Motion,
}

enum Translate {
    Pad(Normalizer),
    Touchpad(TouchNormalizer),
    Motion(MotionNormalizer),
}

/// One node of a managed controller. Dropping it closes the node, which lets go of its grab.
pub(super) struct Reader {
    dev: Device,
    translate: Translate,
}

/// What one read of a node gave.
enum Read {
    /// Inputs, and the kernel's time stamp of the newest raw event, for the input timing log.
    Input(Vec<InputEvent>, u64),
    Motion(Vec<MotionSample>),
    Nothing,
    /// The node went away (unplugged) or failed.
    Gone,
}

impl Reader {
    pub(super) fn pad(dev: Device, xbox_labels: bool) -> std::io::Result<Self> {
        let translate = Translate::Pad(Normalizer::new(&dev, xbox_labels));
        Self::new(dev, translate)
    }

    pub(super) fn touchpad(dev: Device) -> std::io::Result<Self> {
        let translate = Translate::Touchpad(TouchNormalizer::new(&dev));
        Self::new(dev, translate)
    }

    pub(super) fn motion(dev: Device) -> std::io::Result<Self> {
        let translate = Translate::Motion(MotionNormalizer::new(&dev));
        Self::new(dev, translate)
    }

    fn new(dev: Device, translate: Translate) -> std::io::Result<Self> {
        dev.set_nonblocking(true)?;
        Ok(Reader { dev, translate })
    }

    pub(super) fn role(&self) -> Role {
        match self.translate {
            Translate::Pad(_) => Role::Pad,
            Translate::Touchpad(_) => Role::Touchpad,
            Translate::Motion(_) => Role::Motion,
        }
    }

    /// Reads what the node has waiting.
    fn read(&mut self) -> Read {
        let Reader { dev, translate } = self;
        let raw = match dev.fetch_events() {
            Ok(raw) => raw,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Read::Nothing,
            Err(_) => return Read::Gone,
        };
        let mut stamp = UNIX_EPOCH;
        let mut events = Vec::new();
        match translate {
            Translate::Motion(norm) => return Read::Motion(raw.filter_map(|ev| norm.translate(ev)).collect()),
            Translate::Pad(norm) => raw.for_each(|ev| {
                stamp = stamp.max(ev.timestamp());
                norm.translate(ev, &mut events);
            }),
            Translate::Touchpad(norm) => raw.for_each(|ev| {
                stamp = stamp.max(ev.timestamp());
                norm.translate(ev, &mut events);
            }),
        }
        if events.is_empty() { Read::Nothing } else { Read::Input(events, input_timing::ns(stamp)) }
    }
}

impl Daemon {
    /// Waits up to `timeout` for input from a controller or a message, and handles what came.
    /// False once no message can come any more.
    pub(super) fn wait_and_handle(&mut self, rx: &Receiver<Msg>, timeout: Duration) -> bool {
        let poll = |fd: i32| libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        let mut fds = vec![poll(self.tx.wake.fd())];
        let mut from = Vec::new();
        for (id, dev) in &self.devices {
            for reader in &dev.readers {
                fds.push(poll(reader.dev.as_raw_fd()));
                from.push((*id, reader.role()));
            }
        }
        let wait = libc::timespec { tv_sec: timeout.as_secs() as libc::time_t, tv_nsec: timeout.subsec_nanos() as libc::c_long };
        // SAFETY: `fds` is a valid array of that many pollfds, alive for the call; no signal mask.
        let ready = unsafe { libc::ppoll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, &wait, std::ptr::null()) };
        if ready > 0 {
            for (fd, (id, role)) in fds[1..].iter().zip(from) {
                if fd.revents != 0 {
                    self.read_node(id, role, fd.revents);
                }
            }
            if fds[0].revents != 0 {
                self.tx.wake.clear();
            }
        }
        let mut handled = false;
        loop {
            match rx.try_recv() {
                Ok(msg) => {
                    self.handle(msg);
                    handled = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
        if handled {
            self.sync_tray();
        }
        true
    }

    /// Reads controller `id`'s node `role`, which poll said is ready, and handles what it gave.
    fn read_node(&mut self, id: u64, role: Role, revents: libc::c_short) {
        let Some(reader) = self.devices.get_mut(&id).and_then(|d| d.readers.iter_mut().find(|r| r.role() == role)) else { return };
        let read = if revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 { Read::Gone } else { reader.read() };
        match read {
            Read::Input(events, stamp) => {
                let read = input_timing::enabled().then(input_timing::now_ns);
                self.input(id, events, stamp);
                if let Some(read) = read {
                    input_timing::write(format_args!("read {stamp} {read}"));
                }
            }
            Read::Motion(samples) => samples.into_iter().for_each(|sample| self.motion(id, sample)),
            Read::Nothing => {}
            Read::Gone => self.node_gone(id, role),
        }
    }

    /// A node of controller `id` went away. Without its gamepad node the controller is gone; it
    /// can lose its touchpad or motion sensors on their own.
    fn node_gone(&mut self, id: u64, role: Role) {
        if role == Role::Pad {
            return self.device_gone(id);
        }
        let Some(dev) = self.devices.get_mut(&id) else { return };
        dev.readers.retain(|r| r.role() != role);
        match role {
            Role::Touchpad => dev.touchpad = None,
            Role::Motion => dev.motion = None,
            Role::Pad => {}
        }
    }
}
