//! What is on screen and when each thing opened, moved and was picked, so the overlay process
//! can draw each with its animation.

use std::time::Instant;

use super::{Anim, MotionSet, NEVER, OverlayKind, Timing, clock};

/// Something an overlay shows, so its timing can be kept from one frame to the next.
pub trait Tracked: Clone {
    /// Tells the things of one kind apart: a thing that keeps its key keeps its timing.
    fn key(&self) -> String;
    /// Which motion style it moves with.
    fn kind(&self) -> OverlayKind;
    /// Where the cursor is, if it has one: when that changes the highlight glides from the old place.
    fn cursor(&self) -> Option<u32> {
        None
    }
    /// Changes when something is picked in it.
    fn pick(&self) -> Option<u32> {
        None
    }
}

/// One thing on screen, or one that has just gone and is still fading out.
#[derive(Debug, Clone)]
pub struct Entry<T> {
    key: String,
    pub value: T,
    opened: Instant,
    closed: Option<Instant>,
    moved: Instant,
    from: Option<u32>,
    cursor: Option<u32>,
    picked: Option<Instant>,
    pick: Option<u32>,
    /// Whether a frame with it in has been put on screen (`Presence::presented`).
    seen: bool,
}

impl<T: Tracked> Entry<T> {
    fn new(now: Instant, key: String, value: T) -> Self {
        Entry {
            key,
            cursor: value.cursor(),
            pick: value.pick(),
            value,
            opened: now,
            closed: None,
            moved: now,
            from: None,
            picked: None,
            seen: false,
        }
    }

    fn refresh(&mut self, now: Instant, value: T) {
        let cursor = value.cursor();
        if cursor != self.cursor {
            self.from = self.cursor;
            self.moved = now;
            self.cursor = cursor;
        }
        let pick = value.pick();
        if pick != self.pick {
            if pick.is_some() {
                self.picked = Some(now);
            }
            self.pick = pick;
        }
        self.value = value;
    }

    fn timing(&self, now: Instant) -> Timing {
        let since = |t: Instant| now.saturating_duration_since(t).as_secs_f32();
        Timing {
            opened: since(self.opened),
            closed: self.closed.map(since),
            moved: since(self.moved),
            from: self.from,
            picked: self.picked.map_or(NEVER, since),
        }
    }
}

/// What is on screen, with when each thing opened, moved and was picked. A thing that goes is
/// kept, shown fading out, until its kind's close has played.
#[derive(Debug, Clone)]
pub struct Presence<T> {
    entries: Vec<Entry<T>>,
    /// Whether only one thing is shown at a time (the panels): a new one then replaces the old at
    /// once, rather than the two fading over each other.
    single: bool,
}

impl<T> Default for Presence<T> {
    fn default() -> Self {
        Presence { entries: Vec::new(), single: false }
    }
}

impl<T: Tracked> Presence<T> {
    /// For things where one at a time is shown.
    pub fn single() -> Self {
        Presence { entries: Vec::new(), single: true }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Takes in what is shown now: new things open, things no longer shown start to close, and
    /// the rest take their new values.
    pub fn update(&mut self, now: Instant, shown: Vec<T>) {
        let mut matched = vec![false; self.entries.len()];
        let mut opened = false;
        for value in shown {
            let key = value.key();
            let found = (0..self.entries.len()).find(|&i| !matched[i] && self.entries[i].closed.is_none() && self.entries[i].key == key);
            match found {
                Some(i) => {
                    matched[i] = true;
                    self.entries[i].refresh(now, value);
                }
                None => {
                    matched.push(true);
                    opened = true;
                    self.entries.push(Entry::new(now, key, value));
                }
            }
        }
        for (entry, matched) in self.entries.iter_mut().zip(matched) {
            if !matched && entry.closed.is_none() {
                entry.closed = Some(now);
            }
        }
        if self.single && opened {
            self.entries.retain(|e| e.closed != Some(now));
        }
    }

    /// A frame showing everything here went on screen at `at`: what hadn't been on before opens
    /// from then, not from when it came. The first frame of a thing can take a while to draw
    /// (its picture, a screen-sized buffer), and its open would otherwise have run on meanwhile,
    /// so its next frame would jump ahead by that much.
    pub fn presented(&mut self, at: Instant) {
        for entry in self.entries.iter_mut().filter(|e| !e.seen) {
            entry.seen = true;
            if entry.closed.is_some() || at <= entry.opened {
                continue;
            }
            // A cursor that hasn't moved since it opened stays at rest from the open.
            if entry.moved == entry.opened {
                entry.moved = at;
            }
            entry.opened = at;
        }
    }

    /// Drops the things that have finished closing.
    pub fn prune(&mut self, now: Instant, motion: &MotionSet) {
        self.entries.retain(|e| {
            let timing = e.timing(now);
            timing.closed.is_none_or(|closed| closed < motion.feel(e.value.kind()).close + timing.pick_hold())
        });
    }

    /// What to draw now, each with its animation, oldest first so newer things draw on top.
    pub fn shown<'a>(&'a self, now: Instant, motion: &MotionSet) -> impl Iterator<Item = (&'a T, Anim)> + 'a {
        let motion = *motion;
        self.entries.iter().map(move |e| (&e.value, Anim { feel: motion.feel(e.value.kind()), timing: e.timing(now), clock: clock(now) }))
    }

    /// Whether anything is still moving, so the screen needs redrawing.
    pub fn busy(&self, now: Instant, motion: &MotionSet) -> bool {
        self.shown(now, motion).any(|(_, anim)| anim.is_busy())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::motion::MotionStyle;

    /// A stand-in for an overlay: a name, a kind, a cursor and a pick count.
    #[derive(Debug, Clone, PartialEq)]
    struct Thing {
        name: &'static str,
        cursor: Option<u32>,
        picks: u32,
    }

    impl Tracked for Thing {
        fn key(&self) -> String {
            self.name.into()
        }
        fn kind(&self) -> OverlayKind {
            OverlayKind::Menu
        }
        fn cursor(&self) -> Option<u32> {
            self.cursor
        }
        fn pick(&self) -> Option<u32> {
            (self.picks > 0).then_some(self.picks)
        }
    }

    fn thing(name: &'static str, cursor: Option<u32>) -> Thing {
        Thing { name, cursor, picks: 0 }
    }

    fn only(motion: MotionStyle) -> MotionSet {
        MotionSet { menu: motion, ..MotionSet::default() }
    }

    #[test]
    fn off_has_no_motion_so_nothing_is_kept_after_closing() {
        let t0 = Instant::now();
        let motion = only(MotionStyle::Off);
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("a", Some(0))]);
        shown.update(t0, Vec::new());
        shown.prune(t0, &motion);
        assert!(shown.is_empty());
    }

    #[test]
    fn a_closed_thing_is_kept_until_its_close_has_played() {
        let t0 = Instant::now();
        let motion = only(MotionStyle::Subtle);
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("a", Some(0))]);
        shown.update(t0 + Duration::from_millis(500), Vec::new());
        shown.prune(t0 + Duration::from_millis(600), &motion);
        assert!(!shown.is_empty(), "still fading out 100ms after going");
        assert!(shown.busy(t0 + Duration::from_millis(600), &motion));
        shown.prune(t0 + Duration::from_millis(700), &motion);
        assert!(shown.is_empty());
    }

    #[test]
    fn a_thing_opens_from_when_it_was_first_on_screen() {
        let t0 = Instant::now();
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("belt", Some(0))]);
        let on = t0 + Duration::from_millis(15);
        shown.presented(on);
        let motion = only(MotionStyle::Subtle);
        let (_, anim) = shown.shown(on, &motion).next().unwrap();
        assert!(anim.timing.opened.abs() < 1e-6 && anim.timing.moved.abs() < 1e-6, "{:?}", anim.timing);
        shown.presented(on + Duration::from_millis(15));
        let (_, later) = shown.shown(on + Duration::from_millis(15), &motion).next().unwrap();
        assert!((later.timing.opened - 0.015).abs() < 1e-3, "only the first frame on screen moves it: {:?}", later.timing);
        shown.update(on + Duration::from_millis(20), vec![thing("belt", Some(0)), thing("hint", None)]);
        shown.presented(on + Duration::from_millis(30));
        let opened: Vec<f32> = shown.shown(on + Duration::from_millis(30), &motion).map(|(_, a)| a.timing.opened).collect();
        assert!((opened[0] - 0.030).abs() < 1e-3 && opened[1].abs() < 1e-6, "{opened:?}");
    }

    #[test]
    fn a_thing_that_stays_keeps_when_it_opened() {
        let t0 = Instant::now();
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("belt", Some(0))]);
        let later = t0 + Duration::from_secs(1);
        shown.update(later, vec![thing("belt", Some(0))]);
        let motion = only(MotionStyle::Subtle);
        let (_, anim) = shown.shown(later, &motion).next().unwrap();
        assert!((anim.timing.opened - 1.0).abs() < 1e-3, "{:?}", anim.timing);
    }

    #[test]
    fn moving_the_cursor_glides_from_the_item_it_left() {
        let t0 = Instant::now();
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("belt", Some(2))]);
        let t1 = t0 + Duration::from_millis(500);
        shown.update(t1, vec![thing("belt", Some(3))]);
        let motion = only(MotionStyle::Subtle);
        let (_, anim) = shown.shown(t1, &motion).next().unwrap();
        assert_eq!(anim.timing.from, Some(2));
        assert!(anim.timing.moved.abs() < 1e-3);
        let t2 = t1 + Duration::from_millis(50);
        shown.update(t2, vec![thing("belt", Some(3))]);
        let (_, still) = shown.shown(t2, &motion).next().unwrap();
        assert!((still.timing.moved - 0.05).abs() < 1e-3, "no move, no new glide");
    }

    #[test]
    fn a_pick_is_seen_when_the_pick_count_changes() {
        let t0 = Instant::now();
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("belt", Some(0))]);
        let t1 = t0 + Duration::from_millis(300);
        shown.update(t1, vec![Thing { picks: 1, ..thing("belt", Some(0)) }]);
        let motion = only(MotionStyle::Brutal);
        let (_, anim) = shown.shown(t1, &motion).next().unwrap();
        assert!(anim.timing.picked.abs() < 1e-3);
        assert!(anim.flash() > 0.9, "a fresh pick flashes at full strength");
    }

    #[test]
    fn a_new_thing_under_the_same_key_after_closing_opens_again() {
        let t0 = Instant::now();
        let mut shown = Presence::default();
        shown.update(t0, vec![thing("belt", Some(0))]);
        shown.update(t0, Vec::new());
        let t1 = t0 + Duration::from_secs(5);
        shown.update(t1, vec![thing("belt", Some(0))]);
        let motion = only(MotionStyle::Subtle);
        assert_eq!(shown.shown(t1, &motion).count(), 2, "the old one fades beside the new one");
    }

    #[test]
    fn a_single_presence_replaces_its_thing_at_once() {
        let t0 = Instant::now();
        let mut panel = Presence::single();
        panel.update(t0, vec![thing("belt", Some(0))]);
        panel.update(t0, vec![thing("weapons", Some(0))]);
        assert_eq!(panel.shown(t0, &only(MotionStyle::Grim)).count(), 1, "the old menu is cut, not faded over");
        let mut infos = Presence::default();
        infos.update(t0, vec![thing("a", None)]);
        infos.update(t0, vec![thing("b", None)]);
        assert_eq!(infos.shown(t0, &only(MotionStyle::Grim)).count(), 2, "independent overlays still fade out");
    }

    #[test]
    fn repeated_frames_do_not_pile_up_entries() {
        let t0 = Instant::now();
        let mut shown = Presence::default();
        for _ in 0..5 {
            shown.update(t0, vec![thing("a", None), thing("a", None)]);
        }
        assert_eq!(shown.shown(t0, &MotionSet::default()).count(), 2);
    }
}
