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
                explain: "A mode changes some of the controller's mappings until you switch to another mode, or back: \
                          e.g. a Driving mode where the sticks steer and the triggers accelerate. Only one mode is on at \
                          a time; with none on, the profile itself is the default mode. Switch with \"Switch mode…\" on \
                          any input or menu item: it switches to that mode, or back to the default if that mode is \
                          already on. A mode applies over whichever of the game's profiles is active, and layers apply \
                          on top of it.",
                empty: "This setup has no modes yet, so each profile has just its default mode.",
                name: "Mode name",
                delete: "Delete mode",
                indicator: "How the mode shows that it's on: its name, one of the game's info overlays (e.g. a cheat \
                            sheet of the mode's controls), or nothing.",
                delay: "Waits this long after switching before showing the above.",
                swallow: "Do nothing while the mode is on",
                over: "Buttons the mode doesn't change keep this profile's mapping. The mode works with any of the setup's profiles.",
            },
            Shift::Layer => Words {
                new: "+ New layer",
                blurb: "A layer changes some of the mappings while you hold it.",
                explain: "A layer changes some of the controller's mappings while you hold it, then restores them: e.g. \
                          hold LB, and the face buttons type F1–F4. Hold one with \"Hold layer…\" on any button, \
                          trigger, stick direction, zone, gesture or combo. It applies over whichever of the game's \
                          profiles and modes are on; what it doesn't set stays as it was. Several layers can be held \
                          at once; the newest wins. To switch control schemes until you switch back, use a mode.",
                empty: "This setup has no layers yet.",
                name: "Layer name",
                delete: "Delete layer",
                indicator: "How the layer shows that it's on: its name, one of the game's info overlays (e.g. a cheat \
                            sheet of what the layer's buttons do), or nothing, for quick ones such as \"hold Y to \
                            lean\".",
                delay: "Waits this long before showing the above, so a quick tap of the input holding the layer doesn't flash it.",
                swallow: "Do nothing while the layer is on",
                over: "Buttons the layer doesn't change keep this profile's mapping. The layer works with any of the setup's profiles.",
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
