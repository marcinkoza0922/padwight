//! The daemon's main loop: waiting for input, messages and deadlines, the timers, and the tick
//! that drives continuous output (sticks moving the mouse, scrolling, turbo, macros).

use super::*;

/// Where the loop stands between rounds: when it last ticked, and when it next scans for controllers.
pub(super) struct Pace {
    next_scan: Instant,
    last_tick: Instant,
}

impl Pace {
    pub(super) fn new() -> Self {
        let now = crate::clock::now();
        Self { next_scan: now + SCAN_INTERVAL, last_tick: now }
    }
}

/// How long a daemon on the manual clock waits for a message before looking again: time never
/// passes by itself there, so deadlines are no reason to wake.
const MANUAL_POLL: Duration = Duration::from_millis(50);

impl Daemon {
    pub(super) fn run(&mut self, rx: &Receiver<Msg>) {
        loop {
            crate::priority::keep_up();
            let ticking = self.needs_tick();
            let timeout = if crate::clock::is_manual() { MANUAL_POLL } else { self.sleep_time(ticking) };
            if !self.wait_and_handle(rx, timeout) {
                return;
            }
            self.step(ticking);
        }
    }

    fn tick_period(&self) -> Duration {
        if self.config.fast_tick { FAST_TICK } else { TICK }
    }

    /// How long until the loop has something to do without a message: the next tick, scan or deadline.
    fn sleep_time(&self, ticking: bool) -> Duration {
        let mut timeout = if ticking {
            self.tick_period().saturating_sub(crate::clock::since(self.pace.last_tick))
        } else {
            self.pace.next_scan.saturating_duration_since(crate::clock::now())
        };
        if let Some(deadline) = self.next_deadline() {
            timeout = timeout.min(deadline.saturating_duration_since(crate::clock::now()));
        }
        timeout
    }

    /// One round of the loop after the wait: timers, then the tick and the scan if they are due.
    fn step(&mut self, ticking: bool) {
        self.run_timers();
        self.check_fallbacks(crate::clock::now());
        // The two-second hold ends on a deadline, with no input to carry the check along.
        if self.quit_hold.is_some() {
            self.update_force_quit();
        }
        self.finish_stop();
        self.catch_up_on_focus();
        let now = crate::clock::now();
        let period = self.tick_period();
        if ticking {
            let dt = now.duration_since(self.pace.last_tick);
            if dt >= period {
                // Clamp so a stall never turns into a huge cursor jump.
                self.tick(dt.min(Duration::from_millis(20)).as_secs_f32());
                self.pace.last_tick = now;
            }
        } else {
            self.pace.last_tick = now;
        }
        if now >= self.pace.next_scan {
            self.scan();
            // A release the kernel refused is sent again here, not only on the next key.
            if let Err(e) = self.kbm.sync() {
                log!("output error: {e:#}");
            }
            self.ask_pending_pad();
            crate::priority::check();
            self.pace.next_scan = now + SCAN_INTERVAL;
        }
    }

    /// On the manual clock: moves time forward by `by`, stopping at every tick, scan and deadline
    /// on the way to run it at its exact time. A `--debug --fake-clock` daemon's only way to pass time.
    pub(super) fn advance(&mut self, by: Duration) {
        let target = crate::clock::now() + by;
        let mut stepped_at = None;
        loop {
            let ticking = self.needs_tick();
            let now = crate::clock::now();
            let mut next = target.min(self.pace.next_scan);
            if ticking {
                next = next.min(self.pace.last_tick + self.tick_period());
            }
            if let Some(deadline) = self.next_deadline() {
                next = next.min(deadline);
            }
            let mut next = next.max(now);
            if stepped_at == Some(next) {
                // Already ran here: whatever is due didn't clear, so it can't hold time back.
                if next >= target {
                    return;
                }
                next = (next + Duration::from_millis(1)).min(target);
            }
            crate::clock::set(next);
            self.step(ticking);
            stepped_at = Some(next);
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let overlay = match &self.active {
            Some(Active::Keyboard(k)) => k.next_deadline(),
            Some(Active::Menu { session, .. }) => session.next_deadline(),
            Some(Active::Media(m)) => m.next_deadline(),
            Some(Active::Offer(o)) => o.next_deadline(),
            Some(Active::Settings(m, _)) => m.next_deadline(),
            Some(Active::NewPad(s, _)) => s.next_deadline(),
            Some(Active::Tools(..)) | None => None,
        };
        self.devices
            .values()
            .filter_map(|d| d.engine.next_deadline())
            .chain(overlay)
            .chain(self.info_refresh)
            .chain(self.info_fade)
            .chain(self.log_redraw)
            .chain(self.quit_hold.filter(|(_, done)| !done).map(|(since, _)| since + FORCE_QUIT_HOLD))
            .chain(self.stop_deadline())
            .chain(self.fallback_deadline())
            .min()
    }

    /// Fires combo members whose combo window has expired.
    fn run_timers(&mut self) {
        let now = crate::clock::now();
        self.tick_overlays(now);
        if self.config.active().is_none() {
            return;
        }
        let mut requests = Vec::new();
        let mut fired = Vec::new();
        let mut guide_keys = Vec::new();
        for (id, dev) in self.devices.iter_mut() {
            if dev.engine.next_deadline().is_none_or(|d| d > now) {
                continue;
            }
            fired.push(*id);
            let Some(base) = self.config.profile_for(&dev.id) else { continue };
            let mut out = Vec::new();
            dev.engine.timers(layered(&dev.layered, base), now, &mut out);
            let keys = dev.engine.take_guide_nav();
            if !keys.is_empty() {
                guide_keys.push((*id, keys));
            }
            if dev.refresh_layers(&self.config) {
                dev.engine.resync(layered(&dev.layered, base), &mut out);
            }
            dev.dispatch(&mut self.kbm, out);
            requests.push((*id, Requests::take(&mut dev.engine)));
            dev.draw_status();
        }
        for (id, asked) in requests {
            self.run_requests(id, asked);
        }
        for (id, keys) in guide_keys {
            self.guide_keys(id, &keys);
        }
        // A gesture or combo window ran out: its action labels the press it came from.
        if fired.into_iter().fold(false, |any, id| self.log_fired(id) | any) {
            self.log_changed();
        }
        self.check_info_changes();
    }

    fn needs_tick(&self) -> bool {
        self.doodle_ticking() || self.devices.values().any(|d| self.config.profile_for(&d.id).is_some_and(|base| d.engine.needs_tick(layered(&d.layered, base))))
    }

    fn tick(&mut self, dt: f32) {
        self.tick_doodle(dt);
        for dev in self.devices.values_mut() {
            let Some(base) = self.config.profile_for(&dev.id) else { continue };
            let mut out = Vec::new();
            dev.engine.tick(layered(&dev.layered, base), dt, &mut out);
            dev.dispatch(&mut self.kbm, out);
            dev.out_view.end_tick(dt);
        }
        if self.lens_active() {
            self.broadcast_overlay();
        }
        if crate::clock::since(self.last_draw) >= STATUS_REDRAW
            && let Some(dev) = self.last_active.and_then(|id| self.devices.get(&id))
        {
            dev.draw_status();
            self.last_draw = crate::clock::now();
        }
    }
}
