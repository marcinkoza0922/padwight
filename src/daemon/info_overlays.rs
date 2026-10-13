//! Which info overlays show, and the live values they draw: those always up, those an action
//! holds, those shown for a while, the Guide overlay's panels, and the mode's and layers'
//! indicators.

use super::*;

impl Daemon {
    /// The info overlays to draw now and how visible each is: steady ones fully, and those
    /// shown for a while (at the game's start, or lingering after being let go) fading out at
    /// the end of it.
    pub(super) fn timed_info(&mut self, now: Instant) -> Vec<(crate::config::InfoOverlay, f32)> {
        let held: HashSet<String> = self.devices.values().flat_map(|d| d.engine.shown_info()).cloned().collect();
        self.info_timers.update(held, &self.scope.info, now);
        let keys: Vec<String> = self.active_shifts().into_iter().map(|(key, _)| key).collect();
        self.layer_started.retain(|key, _| keys.contains(key));
        for key in keys {
            self.layer_started.entry(key).or_insert(now);
        }
        self.info_fade = [self.info_timers.next_redraw(now, FADE_FRAME), self.layer_indicator_due(now)].into_iter().flatten().min();
        self.visible_info(now)
            .into_iter()
            .map(|(o, steady)| {
                let opacity = if steady { 1.0 } else { self.info_timers.opacity(&o.name, now) };
                (o, opacity)
            })
            .filter(|(_, opacity)| *opacity > 0.0)
            .collect()
    }

    /// Info overlays to show now, and whether each is up steadily: those of the active game set
    /// to always show, any an action holds up, those shown for a while (at the game's start,
    /// or lingering), and active layers' indicators.
    pub(super) fn visible_info(&self, now: Instant) -> Vec<(crate::config::InfoOverlay, bool)> {
        use crate::config::{Indicator, InfoOverlay};
        let mut shown: Vec<(InfoOverlay, bool)> = self
            .scope
            .info
            .iter()
            .filter_map(|o| {
                let steady = o.always || self.info_timers.held(&o.name);
                let timed = self.info_timers.timed(&o.name);
                (steady || timed).then(|| (o.clone(), steady))
            })
            .collect();
        // The Guide overlay's Notes and Mappings, while it's up.
        if self.devices.values().any(|d| d.engine.guide_overlay_open())
            && let Some(view) = self.guide_view()
        {
            shown.extend(view.overlays().into_iter().map(|o| (o, true)));
        }
        // The mode's and each active layer's indicator: its name, or an info overlay of the game's.
        for (key, layer) in self.active_shifts() {
            if self.layer_indicator_due_at(&key, layer).is_some_and(|due| due > now) {
                continue;
            }
            let name = &layer.name;
            match &layer.indicator {
                Indicator::Bindings => shown.push((crate::info::layer_sheet(layer), true)),
                Indicator::Name => shown.push((
                    InfoOverlay {
                        name: format!("layer {key}"),
                        always: true,
                        on_start: None,
                        linger: None, title: None,
                        current_input: Default::default(),
                        style: layer.indicator_style.clone(),
                        rows: vec![vec![name.clone()]],
                    },
                    true,
                )),
                Indicator::Info(info) => match shown.iter_mut().find(|(o, _)| &o.name == info) {
                    Some((_, steady)) => *steady = true,
                    None => {
                        if let Some(o) = self.scope.info.iter().find(|o| &o.name == info) {
                            shown.push((o.clone(), true));
                        }
                    }
                },
                Indicator::Off => {}
            }
        }
        shown
    }

    /// When the indicator of `layer` (the mode or a layer, by its key in
    /// [`Daemon::active_shifts`]) is due to appear, if it has been on and has a delay.
    pub(super) fn layer_indicator_due_at(&self, key: &str, layer: &crate::config::Layer) -> Option<Instant> {
        let started = self.layer_started.get(key)?;
        (layer.indicator_delay_ms > 0).then(|| *started + Duration::from_millis(layer.indicator_delay_ms.into()))
    }

    /// The soonest moment a delayed layer indicator should appear, if one is waiting.
    pub(super) fn layer_indicator_due(&self, now: Instant) -> Option<Instant> {
        self.active_shifts()
            .into_iter()
            .filter_map(|(key, l)| self.layer_indicator_due_at(&key, l))
            .filter(|due| *due > now)
            .min()
    }

    /// The active mode, then the active layers, that the active game has, each with a key that
    /// tells a mode from a layer of the same name.
    pub(super) fn active_shifts(&self) -> Vec<(String, &crate::config::Layer)> {
        let mode = self.active_mode().and_then(|n| self.scope.modes.iter().find(|m| m.name == n)).map(|m| (format!("mode {}", m.name), m));
        let layers = self.active_layers().into_iter().filter_map(|n| self.scope.layers.iter().find(|l| l.name == n).map(|l| (n, l)));
        mode.into_iter().chain(layers).collect()
    }

    /// The mode of the controller used last (or of the first, before any is used), if the
    /// active game has it.
    pub(super) fn active_mode(&self) -> Option<String> {
        let pad = self.last_active.and_then(|id| self.devices.get(&id)).or_else(|| self.devices.values().next());
        let mode = pad?.engine.mode()?;
        self.scope.modes.iter().any(|m| m.name == mode).then(|| mode.to_string())
    }

    /// Layers active on any controller (the last one used first), oldest first, that the
    /// active game has.
    pub(super) fn active_layers(&self) -> Vec<String> {
        let last = self.last_active.and_then(|id| self.devices.get(&id));
        let others = self.devices.iter().filter(|(id, _)| Some(**id) != self.last_active).map(|(_, d)| d);
        let mut layers: Vec<String> = Vec::new();
        for name in last.into_iter().chain(others).flat_map(|d| d.engine.layers()) {
            if !layers.contains(&name) && self.scope.layers.iter().any(|l| l.name == name) {
                layers.push(name);
            }
        }
        layers
    }

    pub(super) fn live_values(&self) -> crate::info::Live {
        let window = self.focused.clone().unwrap_or_default();
        let pad = self.last_active.and_then(|id| self.devices.get(&id)).or_else(|| self.devices.values().next());
        let identity = PadIdentity::resolve(self.identity_override, pad.and_then(|d| d.model), self.active_support());
        crate::info::Live {
            profile: self.config.active().map(|p| p.name.clone()).unwrap_or_default(),
            app: window.exe,
            title: window.title,
            pid: window.pid,
            controller: pad.map(|d| d.name.clone()).unwrap_or_default(),
            layers: self.active_layers(),
            mode: self.active_mode(),
            family: pad.and_then(|d| d.family).unwrap_or(self.config.info_glyphs),
            pad_family: identity.family(),
            nintendo_layout: identity.applies_nintendo_layout(self.config.active_nintendo_layout()),
            system: self.sampler.stats.clone(),
            controller_battery: pad.and_then(|d| d.parent.as_deref()).and_then(crate::info::controller_battery),
            form: crate::info::form_factor(),
            inputs: crate::inputlog::Inputs::default(),
            gyro: pad.and_then(|d| d.view.gyro()),
            touchpad: pad.filter(|d| d.touchpad.is_some()).map(|d| d.view.touchpad()),
        }
    }

    /// Redraws info overlays if a ShowInfo action changed what is held up, or a layer (with its
    /// indicator and `{layer}`) started or ended.
    pub(super) fn check_info_changes(&mut self) {
        let mut changed = false;
        for dev in self.devices.values_mut() {
            changed |= dev.engine.take_info_changed();
            changed |= dev.engine.take_layers_changed();
        }
        if changed {
            self.sync_guide_menu();
            self.broadcast_overlay();
        }
    }
}
