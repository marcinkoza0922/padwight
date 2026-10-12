//! The motion styles a player or a pack picks from, and the timing and shape of each.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{Curve, Feel};

/// How an overlay moves. Subtle is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionStyle {
    /// Nothing moves.
    Off,
    /// A quick fade, and the highlight gliding from item to item.
    #[default]
    Subtle,
    /// Panels spring in and bounce into place, and the item the cursor lands on boings. Cheerful games.
    Playful,
    /// Items cascade in one after another, sliding into place.
    Stagger,
    /// Panels whoosh in from the side and on out the other, items trailing. Racing and sci-fi.
    Swift,
    /// Panels drift down and settle, softly, and pick with a gentle glow. Cozy games.
    Dreamy,
    /// Chunky steps like an old console: panels grow in blocks and picks blink. Retro games.
    Retro,
    /// Slow fades that rise into place, and a dim, heavy pulse on a pick. Dark fantasy.
    Grim,
    /// Snaps in and out, the highlight jumps, and a pick jolts and flashes. Shooters and fights.
    Brutal,
}

impl MotionStyle {
    pub const ALL: [MotionStyle; 9] = [
        MotionStyle::Off,
        MotionStyle::Subtle,
        MotionStyle::Playful,
        MotionStyle::Stagger,
        MotionStyle::Swift,
        MotionStyle::Dreamy,
        MotionStyle::Retro,
        MotionStyle::Grim,
        MotionStyle::Brutal,
    ];

    /// What the settings say about this style.
    pub fn describe(self) -> &'static str {
        match self {
            MotionStyle::Off => "Nothing moves.",
            MotionStyle::Subtle => "A quick fade, and the highlight glides from item to item.",
            MotionStyle::Playful => "Panels spring in and bounce into place, and the item the cursor lands on boings. Suits cheerful games.",
            MotionStyle::Stagger => "Items cascade in one after another, sliding into place.",
            MotionStyle::Swift => "Panels whoosh in from the side and on out the other, items trailing. Suits racing and sci-fi.",
            MotionStyle::Dreamy => "Panels drift down and settle softly, and picks glow gently. Suits cozy games.",
            MotionStyle::Retro => "Chunky steps like an old console: panels grow in blocks and picks blink. Suits retro games.",
            MotionStyle::Grim => "Slow fades that rise into place, and a dim, heavy pulse on a pick. Suits dark fantasy.",
            MotionStyle::Brutal => "Snaps in and out, the highlight jumps, and a pick jolts and flashes. Suits shooters and fights.",
        }
    }

    /// The timing and shape of this style.
    pub fn feel(self) -> Feel {
        match self {
            MotionStyle::Off => Feel::OFF,
            MotionStyle::Subtle => Feel { open: 0.18, close: 0.12, glide: 0.12, pick: 0.18, flash: 0.4, ..Feel::OFF },
            MotionStyle::Playful => PLAYFUL,
            MotionStyle::Stagger => STAGGER,
            MotionStyle::Swift => SWIFT,
            MotionStyle::Dreamy => DREAMY,
            MotionStyle::Retro => RETRO,
            MotionStyle::Grim => GRIM,
            MotionStyle::Brutal => Feel { open: 0.07, close: 0.06, glide: 0.0, pick: 0.16, flash: 1.0, jolt: 8.0, ..Feel::OFF },
        }
    }
}

impl fmt::Display for MotionStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MotionStyle::Off => "Off",
            MotionStyle::Subtle => "Subtle",
            MotionStyle::Playful => "Playful",
            MotionStyle::Stagger => "Stagger",
            MotionStyle::Swift => "Swift",
            MotionStyle::Dreamy => "Dreamy",
            MotionStyle::Retro => "Retro",
            MotionStyle::Grim => "Grim",
            MotionStyle::Brutal => "Brutal",
        })
    }
}

const PLAYFUL: Feel = Feel {
    open: 0.45,
    close: 0.16,
    glide: 0.14,
    pick: 0.35,
    slide: -22.0,
    zoom: 0.22,
    pop: 0.12,
    bounce: 0.4,
    flash: 0.5,
    curve: Curve::Spring,
    exit: Curve::In,
    ..Feel::OFF
};

const STAGGER: Feel = Feel {
    open: 0.1,
    close: 0.12,
    glide: 0.12,
    pick: 0.18,
    item: 0.26,
    stagger: 0.05,
    shift: -28.0,
    flash: 0.4,
    curve: Curve::Out,
    ..Feel::OFF
};

const SWIFT: Feel = Feel {
    open: 0.18,
    close: 0.13,
    glide: 0.08,
    pick: 0.16,
    sweep: -60.0,
    item: 0.2,
    stagger: 0.025,
    shift: -24.0,
    flash: 0.6,
    curve: Curve::Out,
    exit: Curve::In,
    ..Feel::OFF
};

const DREAMY: Feel = Feel {
    open: 0.42,
    close: 0.32,
    glide: 0.24,
    pick: 0.5,
    slide: -12.0,
    zoom: -0.05,
    pop: 0.03,
    bounce: 0.5,
    flash: 0.3,
    curve: Curve::InOut,
    exit: Curve::InOut,
    ..Feel::OFF
};

const RETRO: Feel = Feel {
    open: 0.2,
    close: 0.15,
    glide: 0.0,
    pick: 0.3,
    zoom: 0.4,
    pop: 0.1,
    bounce: 0.24,
    flash: 1.0,
    steps: 4,
    ..Feel::OFF
};

const GRIM: Feel = Feel {
    open: 0.6,
    close: 0.5,
    glide: 0.32,
    pick: 0.7,
    slide: 26.0,
    flash: 0.35,
    curve: Curve::InOut,
    exit: Curve::InOut,
    ..Feel::OFF
};
