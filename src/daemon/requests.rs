//! Requests on the control socket: from the GUI, the tray and the command line.

use std::{path::PathBuf, time::Instant};

use super::{Active, BY_HAND, Calibration, Daemon};
use crate::{
    config::Config,
    engine::Opener,
    ipc::{Request, Response},
    keyboard::Layout,
    monitor::log,
    rumble,
};

impl Daemon {
    #[expect(clippy::too_many_lines, reason = "predates the size lints")]
    pub(super) fn request(&mut self, req: Request) -> Response {
        match req {
            Request::Status => Response::Status(self.status()),
            Request::GetConfig => Response::Config(Box::new(self.config.clone())),
            Request::SetConfig(new) => {
                // What couldn't be loaded isn't taken over IPC either.
                if let Some(problem) = new.problem() {
                    return Response::Error(format!("not saved: {problem}"));
                }
                let mut new = *new;
                new.enabled = self.config.enabled;
                new.carry_active(&self.config.active);
                // What the folder held when the daemon read it, whatever the window last saw.
                new.unreadable.clone_from(&self.config.unreadable);
                self.replace_config(new)
            }
            Request::Reload => match Config::load() {
                Ok(new) => {
                    new.log_unreadable();
                    self.replace_config(new)
                }
                Err(e) => Response::Error(format!("{e:#}")),
            },
            Request::SetEnabled(enabled) => {
                self.config.enabled = enabled;
                let kept = self.settings_remapping_set(enabled);
                if !enabled {
                    self.release_devices(|id, _| Some(id) != kept);
                }
                self.scan();
                match self.config.save_settings() {
                    Ok(()) => Response::Ok,
                    Err(e) => Response::Error(format!("saving config: {e:#}")),
                }
            }
            Request::SetProfile(name) => {
                let Some(target) = self.config.find_profile(&name) else {
                    return Response::Error(format!("no profile named {name:?}"));
                };
                self.switch_profile(target, BY_HAND);
                Response::Ok
            }
            Request::Activate(target) => {
                if self.config.profile(&target).is_none() {
                    return Response::Error(format!("no profile {target}"));
                }
                self.switch_profile(target, BY_HAND);
                Response::Ok
            }
            Request::NextProfile => {
                if let Some(next) = self.config.next_profile(self.pad_features().as_deref()) {
                    self.switch_profile(next, BY_HAND);
                }
                Response::Ok
            }
            // Handled by the connection thread, which registers through `Msg::Watch`.
            Request::WatchInput | Request::WatchFeed => Response::Error("watch requests must be the only request".into()),
            Request::ToggleOverlay => {
                self.toggle_overlay(Layout::Keyboard, None);
                Response::Ok
            }
            Request::ToggleNumpad => {
                self.toggle_overlay(Layout::Numpad, None);
                Response::Ok
            }
            Request::OpenMenu(name) => {
                if !self.scope.menus.iter().any(|m| m.name == name) {
                    return Response::Error(format!("no menu named {name:?}"));
                }
                let device = self.last_active.or_else(|| self.devices.keys().next().copied()).unwrap_or(u64::MAX);
                self.open_menu(device, &name, Opener::default());
                Response::Ok
            }
            Request::WatchOverlay => Response::Error("WatchOverlay must be the only request".into()),
            Request::DebugAttach { .. }
            | Request::DebugInput { .. }
            | Request::DebugOutput { .. }
            | Request::DebugDetach(_)
            | Request::DebugIdentify(_) => {
                self.debug_request(req)
            }
            Request::CalibrateGyro(path) => {
                let dev = self.devices.values_mut().find(|d| d.path.as_os_str() == path.as_str());
                match dev {
                    Some(dev) if dev.motion.is_some() => {
                        log!("calibrating gyro for {}: keep it still", dev.name);
                        dev.calibrating = Some(Calibration { started: Instant::now(), sum: [0.0; 3], samples: 0 });
                        Response::Ok
                    }
                    Some(_) => Response::Error(format!("{path} has no gyro")),
                    None => Response::Error(format!("{path} is not being managed")),
                }
            }
            Request::TestTurn(px) => {
                crate::output::test_turn(px);
                Response::Ok
            }
            Request::TestRumble(path) => {
                let known = self
                    .gamepads
                    .iter()
                    .find(|((p, _), _)| p.as_os_str() == path.as_str())
                    .map(|(_, pad)| pad.rumble);
                match known {
                    Some(true) => {
                        rumble::test(PathBuf::from(path));
                        Response::Ok
                    }
                    Some(false) => Response::Error(format!("{path} does not support rumble")),
                    None => Response::Error(format!("no controller at {path}")),
                }
            }
        }
    }

    pub(super) fn replace_config(&mut self, new: Config) -> Response {
        if new.general.profiles.is_empty() {
            return Response::Error("General must have at least one profile".into());
        }
        // An open menu refers to menus by position, which the new config may change. The
        // in-game menu edits the config it opened over: its Discard would put that back over
        // the new one, and its question would ask about changes made elsewhere.
        if matches!(self.active, Some(Active::Menu { .. } | Active::Settings(..))) {
            self.close_overlay();
        }
        // The mode stays on; it applies again if the game still has it.
        self.release_all(true);
        self.config = new;
        self.refresh_scope();
        let ignored = self.config.ignored_devices.clone();
        let enabled = self.config.enabled;
        self.release_devices(|_, d| !enabled || ignored.contains(&d.name));
        self.resync_all();
        self.broadcast_overlay();
        self.save_and_rescan()
    }

    pub(super) fn save_and_rescan(&mut self) -> Response {
        self.scan();
        match self.config.save() {
            Ok(()) => Response::Ok,
            Err(e) => Response::Error(format!("saving config: {e:#}")),
        }
    }
}
