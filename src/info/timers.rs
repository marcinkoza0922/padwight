//! Info and log overlays shown for a while: at a game's start, lingering after being let go, and
//! toasts, each fading out at the end of its time.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use super::{InfoView, Segment};
use crate::config::{InfoOverlay, OverlayStyle};

/// How long an info overlay takes to fade out at the end of its time.
pub const FADE_OUT: Duration = Duration::from_millis(600);

/// The longest an overlay's time can be: a year is as good as for ever, and keeps every moment
/// worked out from it in range.
const LONGEST: f32 = 365.0 * 24.0 * 3600.0;

/// A time in seconds read from a pack or config file (how long an overlay shows or lingers, when
/// a log line fades), as a duration: nothing below 0, and no more than [`LONGEST`]. A value too
/// big for a duration, infinite or not a number would otherwise panic.
#[expect(clippy::manual_clamp, reason = "clamp keeps NaN, which from_secs_f32 panics on; max takes 0 over it")]
pub fn seconds(secs: f32) -> Duration {
    Duration::from_secs_f32(secs.max(0.0).min(LONGEST))
}

/// Info overlays shown for a while rather than steadily: at the game's start, or lingering
/// after the action holding them lets go.
#[derive(Debug, Default)]
pub struct Timers {
    /// Shown for the game's start, and when each goes.
    start: HashMap<String, Instant>,
    /// Let go and lingering, and when each goes.
    lingering: HashMap<String, Instant>,
    /// What actions held up when last updated, to notice them let go.
    held: HashSet<String>,
}

impl Timers {
    /// The game started: its overlays set to show then do, for their time.
    pub fn game_started(&mut self, overlays: &[InfoOverlay], now: Instant) {
        self.start = overlays
            .iter()
            .filter_map(|o| o.on_start.map(|s| (o.name.clone(), now + seconds(s))))
            .collect();
    }

    /// Another game: nothing timed carries over.
    pub fn clear(&mut self) {
        self.start.clear();
        self.lingering.clear();
    }

    /// Notes what actions hold up now. Overlays let go since last time stay their linger time.
    pub fn update(&mut self, held: HashSet<String>, overlays: &[InfoOverlay], now: Instant) {
        self.update_with(held, |name| overlays.iter().find(|o| o.name == name).and_then(|o| o.linger), now);
    }

    /// Like [`Timers::update`], for overlays whose lingering `linger` looks up by name.
    pub fn update_with(&mut self, held: HashSet<String>, linger: impl Fn(&str) -> Option<f32>, now: Instant) {
        for name in self.held.difference(&held) {
            if let Some(s) = linger(name) {
                self.lingering.insert(name.clone(), now + seconds(s));
            }
        }
        self.lingering.retain(|name, end| !held.contains(name) && *end > now);
        self.start.retain(|_, end| *end > now);
        self.held = held;
    }

    /// Held up by an action right now.
    pub fn held(&self, name: &str) -> bool {
        self.held.contains(name)
    }

    /// Shown for a while right now.
    pub fn timed(&self, name: &str) -> bool {
        self.start.contains_key(name) || self.lingering.contains_key(name)
    }

    /// How visible a timed overlay is (0 once its time is up).
    pub fn opacity(&self, name: &str, now: Instant) -> f32 {
        [self.start.get(name), self.lingering.get(name)]
            .into_iter()
            .flatten()
            .filter_map(|end| fade(now, *end))
            .fold(0.0, f32::max)
    }

    /// When the overlay next needs redrawing for a timed overlay: as one starts to fade, then
    /// every frame while it does.
    pub fn next_redraw(&self, now: Instant, frame: Duration) -> Option<Instant> {
        self.start
            .values()
            .chain(self.lingering.values())
            .map(|end| redraw_at(*end, now, frame))
            .min()
    }
}

/// When something due to go at `end` next needs redrawing: as it starts to fade, then every
/// frame while it does.
pub fn redraw_at(end: Instant, now: Instant, frame: Duration) -> Instant {
    let fading = end.checked_sub(FADE_OUT).unwrap_or(now);
    if now >= fading { now + frame } else { fading }
}

/// How long a toast stays up, including its fade.
pub const TOAST_TIME: Duration = Duration::from_millis(2500);

/// A short notice (such as the profile just switched to), drawn like an info overlay at the
/// top of the screen and fading out after [`TOAST_TIME`].
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    lines: Vec<String>,
    end: Instant,
    /// A bar under the lines that fills from `.0` over `.1`: how long a hold has left to go.
    progress: Option<(Instant, Duration)>,
}

/// How often a toast's bar redraws while it fills.
pub(super) const BAR_FRAME: Duration = Duration::from_millis(50);

impl Toast {
    pub fn new(lines: Vec<String>, now: Instant) -> Self {
        Self::lasting(lines, now, TOAST_TIME)
    }

    /// A toast that stays up for `length`, fade included, for something worth reading twice.
    pub fn lasting(lines: Vec<String>, now: Instant, length: Duration) -> Self {
        Toast { lines, end: now + length, progress: None }
    }

    /// A toast with a bar that fills over `length`, for a button held that long; it stays until
    /// replaced or dropped.
    pub fn filling(lines: Vec<String>, now: Instant, length: Duration) -> Self {
        Toast { lines, end: now + length + TOAST_TIME, progress: Some((now, length)) }
    }

    /// What to draw now, or `None` once it's gone.
    pub fn view(&self, now: Instant) -> Option<InfoView> {
        let opacity = fade(now, self.end)?;
        let mut rows: Vec<Vec<Vec<Segment>>> = self.lines.iter().map(|l| vec![vec![Segment::Text(l.clone())]]).collect();
        if let Some((start, length)) = self.progress {
            let done = now.saturating_duration_since(start).as_secs_f32() / length.as_secs_f32().max(f32::EPSILON);
            rows.push(vec![vec![Segment::Bar(done.min(1.0))]]);
        }
        Some(InfoView { name: "toast".into(), style: OverlayStyle::toast(), title: None, rows, opacity })
    }

    pub fn next_redraw(&self, now: Instant, frame: Duration) -> Instant {
        let fading = redraw_at(self.end, now, frame);
        match self.progress {
            Some((start, length)) if now < start + length => fading.min(now + BAR_FRAME),
            _ => fading,
        }
    }
}

/// How visible an overlay due to go at `end` is at `now`: 1 until its last moments, then
/// fading to 0; `None` once it's gone.
pub fn fade(now: Instant, end: Instant) -> Option<f32> {
    let left = end.checked_duration_since(now).filter(|d| !d.is_zero())?;
    Some((left.as_secs_f32() / FADE_OUT.as_secs_f32()).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An info overlay called `name`, shown for `on_start` seconds at the start and lingering
    /// `linger` seconds.
    fn timed(name: &str, on_start: Option<f32>, linger: Option<f32>) -> InfoOverlay {
        InfoOverlay {
            name: name.into(),
            always: false,
            on_start,
            linger,
            title: None,
            current_input: Default::default(),
            style: OverlayStyle::info(),
            rows: Vec::new(),
        }
    }

    #[test]
    fn a_filling_toast_draws_its_bar_and_redraws_while_it_fills() {
        let t0 = Instant::now();
        let toast = Toast::filling(vec!["Keep holding".into()], t0, Duration::from_secs(2));
        let bar = |at| toast.view(at).unwrap().rows.last().unwrap()[0][0].clone();
        assert_eq!(bar(t0), Segment::Bar(0.0));
        assert_eq!(bar(t0 + Duration::from_secs(1)), Segment::Bar(0.5));
        assert!(toast.next_redraw(t0, Duration::from_millis(33)) <= t0 + BAR_FRAME);
    }

    #[test]
    fn overlays_fade_out_at_the_end_of_their_time() {
        let now = std::time::Instant::now();
        let s = std::time::Duration::from_secs_f32;
        assert_eq!(fade(now, now + s(5.0)), Some(1.0));
        let half = fade(now, now + FADE_OUT / 2).unwrap();
        assert!((half - 0.5).abs() < 0.01, "{half}");
        assert_eq!(fade(now, now), None);
        assert_eq!(fade(now + s(1.0), now), None);
    }

    #[test]
    fn toasts_show_their_lines_then_fade_away() {
        let now = Instant::now();
        let toast = Toast::new(vec!["Doom".into(), "Play".into()], now);
        let view = toast.view(now).unwrap();
        assert_eq!(view.rows, [[[Segment::Text("Doom".into())]], [[Segment::Text("Play".into())]]]);
        assert_eq!(view.opacity, 1.0);
        assert_eq!(toast.next_redraw(now, Duration::from_millis(33)), now + TOAST_TIME - FADE_OUT);
        assert!(toast.view(now + TOAST_TIME - FADE_OUT / 2).unwrap().opacity < 1.0);
        assert_eq!(toast.view(now + TOAST_TIME), None);
    }

    #[test]
    fn timed_overlays_show_at_start_and_linger_after_release() {
        let now = Instant::now();
        let s = Duration::from_secs_f32;
        let overlays = [timed("Loaded", Some(3.0), None), timed("Sheet", None, Some(2.0))];
        let mut t = Timers::default();

        t.game_started(&overlays, now);
        assert!(t.timed("Loaded") && !t.timed("Sheet"));
        assert_eq!(t.opacity("Loaded", now + s(1.0)), 1.0);
        assert!(t.next_redraw(now, s(0.03)).is_some_and(|w| w > now + s(2.0)), "wakes as it starts to fade");

        // Held, then let go: it lingers, fading at the end, and holding it again ends that.
        t.update(HashSet::from(["Sheet".to_string()]), &overlays, now);
        assert!(t.held("Sheet") && !t.timed("Sheet"));
        t.update(HashSet::new(), &overlays, now + s(1.0));
        assert!(t.timed("Sheet") && !t.held("Sheet"));
        assert!(t.opacity("Sheet", now + s(3.0) - FADE_OUT / 2) < 0.6);
        t.update(HashSet::from(["Sheet".to_string()]), &overlays, now + s(1.5));
        assert!(!t.timed("Sheet"));

        // Time's up for the start one.
        t.update(HashSet::new(), &overlays, now + s(4.0));
        assert!(!t.timed("Loaded"));
        assert_eq!(t.opacity("Loaded", now + s(4.0)), 0.0);
        t.clear();
        assert!(!t.timed("Sheet"));
    }

    #[test]
    fn a_time_too_long_for_a_duration_lasts_for_good_rather_than_crashing() {
        // TOML reads `inf` and `nan`, and 1e30 seconds is more than a duration holds.
        let now = Instant::now();
        let overlays = [timed("Start", Some(f32::INFINITY), None), timed("Odd", Some(f32::NAN), None), timed("Sheet", None, Some(1e30))];
        let mut t = Timers::default();
        t.game_started(&overlays, now);
        t.update(HashSet::from(["Sheet".to_string()]), &overlays, now);
        t.update(HashSet::new(), &overlays, now);
        let much_later = now + Duration::from_secs(30 * 24 * 3600);
        assert_eq!((t.opacity("Start", much_later), t.opacity("Sheet", much_later)), (1.0, 1.0));
        assert_eq!(seconds(-1.0), Duration::ZERO);
    }
}
