//! The two kinds the Modes and Layers tabs edit, and what each tab says about its kind.

use crate::config::ItemKind;

/// Which of the two tabs is shown: a game's modes or its layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gui) enum Shift {
    Mode,
    Layer,
}

impl Shift {
    pub(in crate::gui) fn kind(self) -> ItemKind {
        match self {
            Shift::Mode => ItemKind::Mode,
            Shift::Layer => ItemKind::Layer,
        }
    }

    /// "Mode" or "Layer", as a new one is first named.
    pub(in crate::gui) fn title(self) -> &'static str {
        match self {
            Shift::Mode => "Mode",
            Shift::Layer => "Layer",
        }
    }

    /// What the tab says about the kind.
    pub(in crate::gui) fn words(self) -> Words {
        match self {
            Shift::Mode => Words {
                new: "+ New mode",
                blurb: "A mode swaps in another control scheme until you switch back.",
                explain: "A mode changes some of the controller's mappings until you switch to another mode or back, \
                          e.g. a Driving mode where the sticks steer and the triggers accelerate. One mode is on at a \
                          time; with none on, the profile itself is the default. \"Switch mode…\" on any input or menu \
                          item switches to that mode, or back to the default if it's already on. A mode applies over \
                          the active profile, and layers apply on top of it.",
                empty: "This setup has no modes yet, so each profile has just its default mode.",
                name: "Mode name",
                delete: "Delete mode",
                indicator: "How the mode shows it's on: its name, an info overlay (e.g. a cheat sheet of its \
                            controls), or nothing.",
                delay: "How long after switching to wait before showing it.",
                swallow: "Do nothing while the mode is on",
                over: "Buttons the mode doesn't change keep this profile's mapping. Modes work with any of the setup's profiles.",
            },
            Shift::Layer => Words {
                new: "+ New layer",
                blurb: "A layer changes some of the mappings while you hold it.",
                explain: "A layer changes some of the controller's mappings while you hold it, e.g. hold LB and the \
                          face buttons type F1–F4. Hold one with \"Hold layer…\" on any button, trigger, stick \
                          direction, zone, gesture or combo. It applies over the active profile and mode; what it \
                          doesn't set stays as it was. Several layers can be held at once; the newest wins. To switch \
                          control schemes until you switch back, use a mode.",
                empty: "This setup has no layers yet.",
                name: "Layer name",
                delete: "Delete layer",
                indicator: "How the layer shows it's on: its name, an info overlay (e.g. a cheat sheet of its \
                            buttons), or nothing, for quick ones such as \"hold Y to lean\".",
                delay: "How long to wait before showing it, so a quick tap doesn't flash it.",
                swallow: "Do nothing while the layer is on",
                over: "Buttons the layer doesn't change keep this profile's mapping. Layers work with any of the setup's profiles.",
            },
        }
    }
}

/// The texts of the Modes or Layers tab.
pub(in crate::gui) struct Words {
    pub(in crate::gui) new: &'static str,
    pub(in crate::gui) blurb: &'static str,
    pub(in crate::gui) explain: &'static str,
    pub(in crate::gui) empty: &'static str,
    pub(in crate::gui) name: &'static str,
    pub(in crate::gui) delete: &'static str,
    pub(in crate::gui) indicator: &'static str,
    pub(in crate::gui) delay: &'static str,
    pub(in crate::gui) swallow: &'static str,
    pub(in crate::gui) over: &'static str,
}
