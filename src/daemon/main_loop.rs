//! The daemon's main loop: waiting for input, messages and deadlines, the timers, and the tick
//! that drives continuous output (sticks moving the mouse, scrolling, turbo, macros).

use super::*;

impl Daemon {
    pub(super) fn run(&mut self, rx: &Receiver<Msg>) {
        let mut next_scan = Instant::now() + SCAN_INTERVAL;
        let mut last_tick = Instant::now();
        loop {
            crate::priority::keep_up();
            let ticking = self.needs_tick();
            let period = if self.config.fast_tick { FAST_TICK } else { TICK };
            let mut timeout = if ticking {
                period.saturating_sub(last_tick.elapsed())
            } else {
                next_scan.saturating_duration_since(Instant::now())
            };
            if let Some(deadline) = self.next_deadline() {
                timeout = timeout.min(deadline.saturating_duration_since(Instant::now()));
            }
            if !self.wait_and_handle(rx, timeout) {
                return;
            }

            self.run_timers();
            self.check_fallbacks(Instant::now());
            self.finish_stop();
            self.catch_up_on_focus();
            let now = Instant::now();
            if ticking {
                let dt = now.duration_since(last_tick);
                if dt >= period {
                    // Clamp so a stall never turns into a huge cursor jump.
                    self.tick(dt.min(Duration::from_millis(20)).as_secs_f32());
                    last_tick = now;
                }
            } else {
                last_tick = now;
            }
            if now >= next_scan {
                self.scan();
                // A release the kernel refused is sent again here, not only on the next key.
                if let Err(e) = self.kbm.sync() {
                    log!("output error: {e:#}");
                }
                self.ask_pending_pad();
                crate::priority::check();
                next_scan = now + SCAN_INTERVAL;
            }
        }
    }

    /// Waits up to `timeout` for a message, and handles what came. False once no message can
    /// come any more. (Linux waits on the controllers too: see `readers.rs`.)
    #[cfg(windows)]
    fn wait_and_handle(&mut self, rx: &Receiver<Msg>, timeout: Duration) -> bool {
        match rx.recv_timeout(timeout) {
            Ok(msg) => {
                self.handle(msg);
                while let Ok(msg) = rx.try_recv() {
                    self.handle(msg);
                }
                self.sync_tray();
                true
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => true,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => false,
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
            None => None,
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
        let now = Instant::now();
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
        self.devices.values().any(|d| self.config.profile_for(&d.id).is_some_and(|base| d.engine.needs_tick(layered(&d.layered, base))))
    }

    fn tick(&mut self, dt: f32) {
        for dev in self.devices.values_mut() {
            let Some(base) = self.config.profile_for(&dev.id) else { continue };
            let mut out = Vec::new();
            dev.engine.tick(layered(&dev.layered, base), dt, &mut out);
            dev.dispatch(&mut self.kbm, out);
            dev.out_view.end_tick(dt);
        }
        if self.last_draw.elapsed() >= STATUS_REDRAW
            && let Some(dev) = self.last_active.and_then(|id| self.devices.get(&id))
        {
            dev.draw_status();
            self.last_draw = Instant::now();
        }
    }
}
