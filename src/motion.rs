//! How overlays move. Each kind of overlay (the keyboard, the numpad, menus, media controls,
//! offers, info overlays and input logs) has a motion style of its own, and a game can set its
//! own. A style says how fast things open and close, how the highlight glides from one item to
//! the next, and what a pick looks like. The overlay process samples it with [`Anim`] while it
//! draws, and keeps the timing of what is on screen with [`Presence`].

mod presence;
mod style;

use std::{f32::consts::PI, time::Instant};

use serde::{Deserialize, Deserializer, Serialize};

pub use presence::{Presence, Tracked};
pub use style::MotionStyle;

/// The overlays a motion style can be set for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayKind {
    Keyboard,
    Numpad,
    Menu,
    Media,
    Offer,
    Info,
    Log,
}

impl OverlayKind {
    pub const ALL: [OverlayKind; 7] = [
        OverlayKind::Keyboard,
        OverlayKind::Numpad,
        OverlayKind::Menu,
        OverlayKind::Media,
        OverlayKind::Offer,
        OverlayKind::Info,
        OverlayKind::Log,
    ];

    /// What the settings call this overlay.
    pub fn label(self) -> &'static str {
        match self {
            OverlayKind::Keyboard => "On-screen keyboard",
            OverlayKind::Numpad => "On-screen numpad",
            OverlayKind::Menu => "Menus",
            OverlayKind::Media => "Media controls",
            OverlayKind::Offer => "Library offers",
            OverlayKind::Info => "Info overlays",
            OverlayKind::Log => "Log overlays",
        }
    }
}

/// The motion style of every kind of overlay. A game's own set replaces the global one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionSet {
    pub keyboard: MotionStyle,
    pub numpad: MotionStyle,
    pub menu: MotionStyle,
    pub media: MotionStyle,
    pub offer: MotionStyle,
    pub info: MotionStyle,
    pub log: MotionStyle,
}

impl MotionSet {
    /// One style for every kind of overlay.
    pub fn all(style: MotionStyle) -> Self {
        MotionSet { keyboard: style, numpad: style, menu: style, media: style, offer: style, info: style, log: style }
    }

    pub fn get(&self, kind: OverlayKind) -> MotionStyle {
        match kind {
            OverlayKind::Keyboard => self.keyboard,
            OverlayKind::Numpad => self.numpad,
            OverlayKind::Menu => self.menu,
            OverlayKind::Media => self.media,
            OverlayKind::Offer => self.offer,
            OverlayKind::Info => self.info,
            OverlayKind::Log => self.log,
        }
    }

    pub fn set(&mut self, kind: OverlayKind, style: MotionStyle) {
        let slot = match kind {
            OverlayKind::Keyboard => &mut self.keyboard,
            OverlayKind::Numpad => &mut self.numpad,
            OverlayKind::Menu => &mut self.menu,
            OverlayKind::Media => &mut self.media,
            OverlayKind::Offer => &mut self.offer,
            OverlayKind::Info => &mut self.info,
            OverlayKind::Log => &mut self.log,
        };
        *slot = style;
    }

    pub fn feel(&self, kind: OverlayKind) -> Feel {
        self.get(kind).feel()
    }

    pub fn is_default(&self) -> bool {
        *self == MotionSet::default()
    }
}

/// Reads a set either as the table it is written as, or as one style for every kind of overlay:
/// the single setting that earlier versions kept, for menus only.
pub fn deserialize_set<'de, D: Deserializer<'de>>(de: D) -> Result<MotionSet, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Style(MotionStyle),
        Set(MotionSet),
    }
    Ok(match Either::deserialize(de)? {
        Either::Style(style) => MotionSet::all(style),
        Either::Set(set) => set,
    })
}

/// How an easing curve shapes progress from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Curve {
    #[default]
    Linear,
    /// Starts fast and slows into place.
    Out,
    /// Starts slow and speeds away.
    In,
    /// Slow at both ends.
    InOut,
    /// Overshoots its end a little before settling.
    Back,
    /// Shoots past its end and bounces back and forth, less each time, into place.
    Spring,
}

impl Curve {
    fn apply(self, t: f32) -> f32 {
        match self {
            Curve::Linear => t,
            Curve::Out => 1.0 - (1.0 - t).powi(3),
            Curve::In => t * t,
            Curve::InOut => t * t * (3.0 - 2.0 * t),
            Curve::Back => {
                let x = t - 1.0;
                1.0 + 2.701_58 * x * x * x + 1.701_58 * x * x
            }
            Curve::Spring => {
                // A damped wave, less what is left of it at the end so it lands exactly on 1.
                let wave = |t: f32| (-5.0 * t).exp() * (3.0 * PI * t).cos();
                1.0 - wave(t) + t * wave(1.0)
            }
        }
    }

    /// This curve without its overshoot, for what can't go past its end (a color, say).
    fn calm(self) -> Curve {
        match self {
            Curve::Back | Curve::Spring => Curve::Out,
            curve => curve,
        }
    }
}

/// The timing and shape of one motion style, in seconds and pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Feel {
    /// How long a panel takes to open, and to close.
    pub open: f32,
    pub close: f32,
    /// How long the highlight takes to glide to the next item, and a pick to settle.
    pub glide: f32,
    pub pick: f32,
    /// How long each item takes to come in, and how far apart those starts are.
    pub item: f32,
    pub stagger: f32,
    /// Pixels each item starts left (negative) or right of its place as it comes in.
    pub shift: f32,
    /// Pixels a panel starts below its place (positive) or above it (negative) on opening, and
    /// moves on by, the other way, as it closes.
    pub slide: f32,
    /// Pixels a panel starts left (negative) or right of its place on opening; it closes on
    /// past its place the way it came in.
    pub sweep: f32,
    /// How much smaller a panel starts, as a share of its size (negative: bigger), and half of
    /// which it shrinks by as it closes.
    pub zoom: f32,
    /// How much the item the cursor lands on swells before it settles, as a share of its
    /// size, and a picked item squashes; and how long that takes.
    pub pop: f32,
    pub bounce: f32,
    /// How strongly a pick flashes the item it lands on, 0 to 1.
    pub flash: f32,
    /// Pixels a pick jolts the panel sideways.
    pub jolt: f32,
    /// Moves in this many still steps rather than smoothly, when more than 1.
    pub steps: u32,
    /// The curve things come in on, and the one a panel closes on.
    pub curve: Curve,
    pub exit: Curve,
}

impl Feel {
    /// No motion at all: everything is already where it ends up.
    pub const OFF: Feel = Feel {
        open: 0.0,
        close: 0.0,
        glide: 0.0,
        pick: 0.0,
        item: 0.0,
        stagger: 0.0,
        shift: 0.0,
        slide: 0.0,
        sweep: 0.0,
        zoom: 0.0,
        pop: 0.0,
        bounce: 0.0,
        flash: 0.0,
        jolt: 0.0,
        steps: 0,
        curve: Curve::Linear,
        exit: Curve::Linear,
    };

    /// How far along `curve` an event `elapsed` seconds ago is, when it takes `secs`: in steps
    /// when the style moves in steps.
    fn ease(&self, elapsed: f32, secs: f32, curve: Curve) -> f32 {
        if secs <= 0.0 {
            return 1.0;
        }
        let t = (elapsed / secs).clamp(0.0, 1.0);
        let t = if self.steps > 1 {
            let n = self.steps as f32;
            ((t * n).floor() / (n - 1.0)).min(1.0)
        } else {
            t
        };
        curve.apply(t)
    }

    /// The longest anything in this style keeps moving after it starts, in seconds.
    fn longest(&self) -> f32 {
        [self.open, self.item + self.stagger * STAGGER_ITEMS, self.glide, self.pick, self.bounce].into_iter().fold(0.0, f32::max)
    }
}

impl Default for Feel {
    fn default() -> Self {
        Feel::OFF
    }
}

/// Staggered items wait at most this many starts; later ones come in with the last.
const STAGGER_ITEMS: f32 = 12.0;
/// Seconds since something happened, when it never did.
const NEVER: f32 = f32::INFINITY;
/// An overlay closed this soon after a pick closed because of it...
const PICK_WINDOW: f32 = 0.05;
/// ...so it stays this long first, the item it picked lit and blinking, before its close plays.
pub const PICK_HOLD: f32 = 0.3;
/// How many times the picked item blinks while it holds.
const PICK_BLINKS: f32 = 2.0;

/// A boing over `p` from 0 to 1: up to about 0.7, back through 0 to about -0.25, and settling
/// at 0.
fn boing(p: f32) -> f32 {
    if p >= 1.0 { 0.0 } else { (3.0 * PI * p).sin() * (1.0 - p) * (1.0 - p) }
}

/// When each thing happened to an overlay, in seconds ago.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timing {
    pub opened: f32,
    /// `None` while it is still shown.
    pub closed: Option<f32>,
    /// When the cursor last moved.
    pub moved: f32,
    /// Where the cursor was before its last move.
    pub from: Option<u32>,
    pub picked: f32,
}

impl Timing {
    /// How long a closing overlay holds still before its close plays: a moment when the close
    /// came with a pick, so the item picked is seen, and none otherwise.
    pub fn pick_hold(&self) -> f32 {
        match self.closed {
            Some(closed) if (0.0..=PICK_WINDOW).contains(&(self.picked - closed)) => PICK_HOLD,
            _ => 0.0,
        }
    }

    /// Seconds since the close started playing, once it has (after any hold).
    fn closing(&self) -> Option<f32> {
        self.closed.map(|closed| (closed - self.pick_hold()).max(0.0))
    }
}

impl Default for Timing {
    fn default() -> Self {
        Timing { opened: NEVER, closed: None, moved: NEVER, from: None, picked: NEVER }
    }
}

/// Seconds from the overlay process's start to `now`: the clock looping effects run on.
pub fn clock(now: Instant) -> f32 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    now.saturating_duration_since(*START.get_or_init(Instant::now)).as_secs_f32()
}

/// An overlay's feel and timing, as the overlay process draws it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Anim {
    pub feel: Feel,
    pub timing: Timing,
    /// Seconds since the overlay process started, for effects that loop (see [`clock`]).
    pub clock: f32,
}

impl Anim {
    /// Drawn as it ends up: no motion.
    pub fn still() -> Self {
        Anim::default()
    }

    /// Drawn still, at `opacity` of its full strength: held that far into a linear fade-in.
    pub fn faint(opacity: f32) -> Self {
        let timing = Timing { opened: opacity.clamp(0.0, 1.0), ..Timing::default() };
        Anim { feel: Feel { open: 1.0, ..Feel::OFF }, timing, clock: 0.0 }
    }

    /// How far into its opening the overlay is, 0 to 1 (past 1 as a spring overshoots).
    fn opening(&self) -> f32 {
        self.feel.ease(self.timing.opened, self.feel.open, self.feel.curve)
    }

    /// How far into its close the overlay is, 0 to 1, once the close plays.
    fn closing(&self) -> Option<f32> {
        self.timing.closing().map(|s| self.feel.ease(s, self.feel.close, self.feel.exit))
    }

    /// How visible the whole overlay is, 0 to 1: fading in as it opens, and out as it closes.
    pub fn opacity(&self) -> f32 {
        let open = self.feel.ease(self.timing.opened, self.feel.open, self.feel.curve.calm());
        let closing = self.closing().map_or(1.0, |c| 1.0 - c.clamp(0.0, 1.0));
        open.clamp(0.0, 1.0) * closing
    }

    /// Where the overlay is pushed off its place, in pixels (x right, y down).
    pub fn offset(&self) -> (f32, f32) {
        let left = 1.0 - self.opening();
        let (mut x, mut y) = (left * self.feel.sweep, left * self.feel.slide);
        if let Some(c) = self.closing() {
            x -= c * self.feel.sweep;
            y += c * self.feel.slide;
        }
        let p = self.pick_progress();
        if p < 1.0 {
            x += self.feel.jolt * (1.0 - p) * (p * PI * 3.0).sin();
        }
        (x, y)
    }

    /// How big the overlay is drawn against its size: growing in as it opens (springing past
    /// its size under Playful), and shrinking a little as it closes.
    pub fn scale(&self) -> f32 {
        let zoom = self.feel.zoom;
        let closing = self.closing().unwrap_or(0.0);
        1.0 - zoom * (1.0 - self.opening()) - zoom * 0.5 * closing
    }

    /// How lit an item is, 0 to 1: the cursor's item glides in, and the one it came from glides
    /// out.
    pub fn lit(&self, selected: bool, from: bool) -> f32 {
        let glided = self.feel.ease(self.timing.moved, self.feel.glide, self.feel.curve.calm());
        // An item just picked (the cursor jumped to it as it was) is lit at once.
        if selected && self.timing.picked <= self.timing.moved {
            return 1.0;
        }
        if selected {
            glided
        } else if from {
            1.0 - glided
        } else {
            0.0
        }
    }

    /// How big an item is drawn against its size: the cursor's item swells as the cursor lands
    /// on it and squashes as it is picked, then bounces back.
    pub fn item_scale(&self, selected: bool) -> f32 {
        if !selected || self.feel.pop == 0.0 {
            return 1.0;
        }
        let landed = boing(self.feel.ease(self.timing.moved, self.feel.bounce, Curve::Linear));
        let pressed = boing(self.feel.ease(self.timing.picked, self.feel.bounce, Curve::Linear));
        1.0 + self.feel.pop * (landed - pressed)
    }

    /// How much a pick still flashes the item it landed on, 0 to 1.
    pub fn flash(&self) -> f32 {
        // A pick that closes the overlay blinks its item through the hold, as strongly as the
        // style's flash allows (not at all with motion off), so it's seen before the close.
        let hold = self.timing.pick_hold();
        if self.timing.picked < hold {
            let wave = (self.timing.picked / hold * PICK_BLINKS * PI).sin();
            return (2.0 * self.feel.flash).min(1.0) * wave * wave;
        }
        let remaining = 1.0 - self.pick_progress();
        self.feel.flash * remaining * remaining
    }

    /// How far into coming in item `index` is, 0 to 1: all at once, or one after another when
    /// the style staggers its items.
    fn item_in(&self, index: usize, curve: Curve) -> f32 {
        if self.feel.stagger <= 0.0 {
            return 1.0;
        }
        let since = self.timing.opened - (index as f32).min(STAGGER_ITEMS) * self.feel.stagger;
        self.feel.ease(since, self.feel.item, curve)
    }

    /// How visible item `index` is, 0 to 1.
    pub fn item_fade(&self, index: usize) -> f32 {
        self.item_in(index, Curve::Out).clamp(0.0, 1.0)
    }

    /// Pixels item `index` is still off its place, sideways, as it comes in.
    pub fn item_shift(&self, index: usize) -> f32 {
        if self.feel.shift == 0.0 {
            return 0.0;
        }
        (1.0 - self.item_in(index, Curve::Back)) * self.feel.shift
    }

    /// Whether this still changes from one frame to the next, so the screen needs redrawing.
    pub fn is_busy(&self) -> bool {
        let (t, f) = (&self.timing, &self.feel);
        // A closed overlay is kept until its close has played, then dropped.
        t.closed.is_some() || t.opened < f.longest() || t.moved < f.glide.max(f.bounce) || t.picked < f.pick.max(f.bounce)
    }

    fn pick_progress(&self) -> f32 {
        self.feel.ease(self.timing.picked, self.feel.pick, Curve::Out).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(motion: MotionStyle) -> MotionSet {
        MotionSet { menu: motion, ..MotionSet::default() }
    }

    #[test]
    fn a_faint_overlay_is_see_through_but_in_place() {
        let faint = Anim::faint(0.45);
        assert!((faint.opacity() - 0.45).abs() < 1e-4);
        assert_eq!((faint.scale(), faint.offset(), faint.item_fade(3)), (1.0, (0.0, 0.0), 1.0));
    }

    #[test]
    fn subtle_is_the_default_for_every_kind() {
        assert_eq!(MotionSet::default().get(OverlayKind::Keyboard), MotionStyle::Subtle);
        assert!(MotionSet::default().is_default());
        assert!(!only(MotionStyle::Grim).is_default());
    }

    #[test]
    fn an_earlier_single_style_applies_to_every_kind() {
        #[derive(Deserialize)]
        struct Config {
            #[serde(default, deserialize_with = "deserialize_set")]
            motion: MotionSet,
        }
        let old: Config = toml::from_str("motion = \"playful\"").unwrap();
        assert_eq!(old.motion, MotionSet::all(MotionStyle::Playful));
        let table: Config = toml::from_str("[motion]\nmenu = \"grim\"").unwrap();
        assert_eq!(table.motion.get(OverlayKind::Menu), MotionStyle::Grim);
        assert_eq!(table.motion.get(OverlayKind::Keyboard), MotionStyle::Subtle, "unset kinds are subtle");
        let missing: Config = toml::from_str("").unwrap();
        assert!(missing.motion.is_default());
    }

    #[test]
    fn a_set_changes_only_its_kind() {
        let mut set = MotionSet::default();
        set.set(OverlayKind::Info, MotionStyle::Brutal);
        assert_eq!(set.get(OverlayKind::Info), MotionStyle::Brutal);
        assert_eq!(set.get(OverlayKind::Menu), MotionStyle::Subtle);
    }

    #[test]
    fn a_pick_that_closes_blinks_its_item_through_the_hold() {
        let feel = MotionStyle::Subtle.feel();
        let at = |picked: f32| Anim { feel, timing: Timing { opened: 1.0, closed: Some(picked - 0.01), moved: picked, picked, from: None }, clock: 0.0 };
        let peak = PICK_HOLD / (2.0 * PICK_BLINKS);
        assert!(at(peak).flash() > 0.75, "bright at each blink's peak");
        assert!(at(2.0 * peak).flash() < 0.05, "back to just lit between blinks");
        assert!(at(3.0 * peak).flash() > 0.75, "and blinks again");
        assert_eq!(at(peak).opacity(), 1.0, "fully shown while it blinks");
        assert_eq!(at(peak).lit(true, false), 1.0);
        let off = Anim { feel: MotionStyle::Off.feel(), ..at(peak) };
        assert_eq!(off.flash(), 0.0, "with motion off it holds lit without blinking");
    }

    #[test]
    fn open_and_close_fade_the_opacity() {
        let feel = MotionStyle::Subtle.feel();
        let anim = Anim { feel, timing: Timing { opened: 0.0, ..Timing::default() }, clock: 0.0 };
        assert!(anim.opacity() < 0.01, "starts invisible");
        let open = Anim { feel, timing: Timing { opened: 1.0, ..Timing::default() }, clock: 0.0 };
        assert!((open.opacity() - 1.0).abs() < 1e-6);
        let closing = Anim { feel, timing: Timing { opened: 1.0, closed: Some(feel.close), ..Timing::default() }, clock: 0.0 };
        assert!(closing.opacity() < 0.01, "gone once closed");
    }

    #[test]
    fn brutal_highlight_jumps_and_grim_rises_into_place() {
        let jump = Anim { feel: MotionStyle::Brutal.feel(), timing: Timing { moved: 0.0, ..Timing::default() }, clock: 0.0 };
        assert!((jump.lit(true, false) - 1.0).abs() < 1e-6, "no glide: lit at once");
        let rising = Anim { feel: MotionStyle::Grim.feel(), timing: Timing { opened: 0.0, ..Timing::default() }, clock: 0.0 };
        let (_, y) = rising.offset();
        assert!(y > 20.0, "starts well below its place");
        let settled = Anim { feel: MotionStyle::Grim.feel(), timing: Timing { opened: 10.0, ..Timing::default() }, clock: 0.0 };
        assert_eq!(settled.offset(), (0.0, 0.0));
    }

    #[test]
    fn playful_drops_in_from_above() {
        let drop = Anim { feel: MotionStyle::Playful.feel(), timing: Timing { opened: 0.0, ..Timing::default() }, clock: 0.0 };
        assert!(drop.offset().1 < 0.0, "starts above its place");
    }

    #[test]
    fn stagger_fades_items_one_after_another() {
        let anim = Anim { feel: MotionStyle::Stagger.feel(), timing: Timing { opened: 0.1, ..Timing::default() }, clock: 0.0 };
        assert!(anim.item_fade(0) > anim.item_fade(3));
        assert_eq!(Anim::still().item_fade(5), 1.0);
    }

    #[test]
    fn off_draws_as_it_ends_up() {
        let anim = Anim { feel: MotionStyle::Off.feel(), timing: Timing { opened: 0.0, moved: 0.0, ..Timing::default() }, clock: 0.0 };
        assert_eq!(anim.opacity(), 1.0);
        assert_eq!(anim.offset(), (0.0, 0.0));
        assert_eq!(anim.lit(true, false), 1.0);
        assert_eq!(anim.lit(false, true), 0.0);
        assert!(!anim.is_busy());
    }

    fn opened(style: MotionStyle, secs: f32) -> Anim {
        Anim { feel: style.feel(), timing: Timing { opened: secs, ..Timing::default() }, clock: 0.0 }
    }

    #[test]
    fn playful_springs_past_its_place_and_settles() {
        let samples: Vec<Anim> = (0..50).map(|i| opened(MotionStyle::Playful, i as f32 * 0.01)).collect();
        assert!(samples.iter().any(|a| a.scale() > 1.02), "grows past its size");
        assert!(samples.iter().any(|a| a.offset().1 > 2.0), "drops past its place");
        let settled = opened(MotionStyle::Playful, 5.0);
        assert_eq!((settled.scale(), settled.offset()), (1.0, (0.0, 0.0)));
    }

    #[test]
    fn the_cursor_landing_swells_its_item_and_a_pick_squashes_it() {
        let feel = MotionStyle::Playful.feel();
        let landed = Anim { feel, timing: Timing { opened: 5.0, moved: 0.04, ..Timing::default() }, clock: 0.0 };
        assert!(landed.item_scale(true) > 1.05);
        assert_eq!(landed.item_scale(false), 1.0, "only the cursor's item");
        let picked = Anim { feel, timing: Timing { opened: 5.0, moved: 5.0, picked: 0.06, ..Timing::default() }, clock: 0.0 };
        assert!(picked.item_scale(true) < 0.95);
        assert_eq!(Anim { timing: Timing { picked: 5.0, ..picked.timing }, ..picked }.item_scale(true), 1.0, "settled");
    }

    #[test]
    fn retro_moves_in_steps() {
        let open = MotionStyle::Retro.feel().open;
        let scales: Vec<f32> = (0..=40).map(|i| opened(MotionStyle::Retro, i as f32 * open / 40.0).scale()).collect();
        let mut distinct = scales.clone();
        distinct.dedup();
        assert_eq!(distinct.len(), 4, "{scales:?}");
    }

    #[test]
    fn swift_comes_in_from_one_side_and_leaves_by_the_other() {
        assert!(opened(MotionStyle::Swift, 0.0).offset().0 < -30.0);
        let feel = MotionStyle::Swift.feel();
        let leaving = Anim { feel, timing: Timing { opened: 5.0, closed: Some(feel.close * 0.9), ..Timing::default() }, clock: 0.0 };
        assert!(leaving.offset().0 > 30.0);
    }

    #[test]
    fn stagger_slides_later_items_in_after_earlier_ones() {
        let anim = opened(MotionStyle::Stagger, 0.12);
        assert!(anim.item_shift(3) < anim.item_shift(0), "item 3 is further off its place");
        assert_eq!(opened(MotionStyle::Stagger, 5.0).item_shift(3), 0.0);
        assert_eq!(opened(MotionStyle::Subtle, 0.0).item_shift(3), 0.0, "subtle doesn't slide items");
    }

    #[test]
    fn every_style_settles_within_a_second() {
        for style in MotionStyle::ALL {
            let anim = Anim { feel: style.feel(), timing: Timing { opened: 1.0, moved: 1.0, picked: 1.0, ..Timing::default() }, clock: 0.0 };
            assert!(!anim.is_busy(), "{style} still moving after a second");
        }
    }
}
