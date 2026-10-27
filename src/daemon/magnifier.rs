//! The magnifying glass: while a Magnifier action is held (Guide + left trigger), the screen is
//! grabbed once and the overlay window shows part of that picture enlarged, in a lens the right
//! stick moves. The overlay can't see the game's pixels itself, so the picture is a frozen one.

use super::*;

/// The magnifying glass's state: the grab it shows, and how many have been made.
#[derive(Default)]
pub(super) struct Lens {
    shot: Option<Shot>,
    grabs: u64,
}

/// A grab: where the picture goes, and whether it has been saved yet.
struct Shot {
    path: PathBuf,
    ready: bool,
}

impl Daemon {
    /// Grabs the screen when a lens goes up and lets the picture go when it comes down.
    pub(super) fn sync_magnifier(&mut self) {
        let aiming = self.devices.values().any(|d| d.engine.magnifier().is_some());
        match (aiming, self.lens.shot.is_some()) {
            (true, false) => {
                // A new name each time: the overlay keeps pictures by their path.
                self.lens.grabs += 1;
                let dir = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
                let path = dir.join(format!("padwight-lens-{}-{}.png", std::process::id(), self.lens.grabs));
                self.lens.shot = Some(Shot { path: path.clone(), ready: false });
                let tx = self.tx.clone();
                thread::spawn(move || {
                    let ok = crate::capture::grab(&path)
                        .inspect_err(|e| log!("magnifier: grabbing the screen failed: {e:#}"))
                        .is_ok();
                    let _ = tx.send(Msg::LensShot { path, ok });
                });
            }
            (false, true) => {
                if let Some(shot) = self.lens.shot.take() {
                    let _ = std::fs::remove_file(shot.path);
                }
            }
            _ => {}
        }
    }

    /// The grab at `path` finished (or failed).
    pub(super) fn lens_shot_done(&mut self, path: &std::path::Path, ok: bool) {
        match self.lens.shot.as_mut().filter(|s| s.path == path) {
            Some(shot) => shot.ready = ok,
            // The lens came down first.
            None => {
                let _ = std::fs::remove_file(path);
            }
        }
        self.broadcast_overlay();
    }

    /// The lens as the overlay draws it: only once the picture it shows has been saved.
    pub(super) fn lens_view(&self) -> Option<crate::overlay::Lens> {
        let (x, y, zoom, size) = self.devices.values().find_map(|d| d.engine.magnifier())?;
        let shot = self.lens.shot.as_ref().filter(|s| s.ready)?.path.clone();
        let note = self.lens_note();
        Some(crate::overlay::Lens { x, y, zoom, size, shot, note })
    }

    /// What is said under the lens: how to get rid of it, when it is pinned.
    fn lens_note(&self) -> Option<String> {
        let pinned = self.devices.values().any(|d| d.engine.lens_pinned());
        let in_mode = self.devices.values().any(|d| d.engine.magnifier_mode());
        let note = if !pinned {
            return None;
        } else if in_mode {
            "Pinned: press X to unpin it"
        } else {
            "Pinned: hold Guide and pull the left trigger to remove it"
        };
        Some(note.into())
    }

    /// Whether the magnifying glass mode is up with its picture saved, which the notice says is
    /// only a screenshot.
    pub(super) fn lens_notice(&self) -> bool {
        self.lens.shot.as_ref().is_some_and(|s| s.ready) && self.devices.values().any(|d| d.engine.magnifier_mode())
    }

    /// Whether a lens is up or coming down, so the frame has to follow the sticks.
    pub(super) fn lens_active(&self) -> bool {
        self.lens.shot.is_some() || self.devices.values().any(|d| d.engine.magnifier().is_some())
    }
}
