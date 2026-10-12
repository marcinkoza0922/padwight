//! Offers a library game's controller profiles: when Guide is tapped over a game that the
//! library knows but the config doesn't, a panel asks whether to add it (and which pack, when
//! more than one fits). `OfferSession` turns controller input into an answer; the overlay
//! process draws `OfferView`.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{
    config::{Button, Config, Feature, OverlayStyle},
    focus,
    input::InputEvent,
    ipc::WindowInfo,
    library::Entry,
};

/// An unanswered offer goes away by itself, so it can't hold the controller forever.
const IDLE_CLOSE: Duration = Duration::from_secs(30);

/// Library packs for the window in front, if it is a game the config doesn't have: no game
/// rule matches it, none of the packs is installed or turned down for good, and at least one
/// of its profiles works with the controllers' features `have` (`None`: no controller to
/// judge by). Indexes into `library`.
pub fn candidates(config: &Config, library: &[Entry], window: &WindowInfo, have: Option<&[Feature]>) -> Vec<usize> {
    if focus::is_own_window(window) || focus::profile_for(config, window).is_some() {
        return Vec::new();
    }
    let seen = std::slice::from_ref(window);
    library
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            let id = &e.pack.pack.id;
            !config.declined_packs.contains(id)
                && !config.games.iter().any(|g| g.origin.as_ref().is_some_and(|o| &o.id == id))
                && e.pack.rules.iter().any(|r| focus::rule_matches_any(r, seen))
                && e.pack.profiles.iter().any(|p| have.is_none_or(|h| p.usable_with(h)))
        })
        .map(|(i, _)| i)
        .collect()
}

/// How long the hint that Guide adds a library setup stays up.
const HINT_TIME: Duration = Duration::from_secs(7);

/// The toast shown when the game `name`, which the library has a setup for, comes to the front:
/// a tap of Guide opens the offer.
pub fn hint(name: &str, now: Instant) -> crate::info::Toast {
    crate::info::Toast::lasting(vec![format!("Padwight has controls for {name}"), "Tap Guide to set them up".into()], now, HINT_TIME)
}

/// What the overlay draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfferView {
    pub title: String,
    /// What the pack is.
    pub detail: String,
    /// The answers on offer: (button, what it does).
    pub answers: Vec<(String, String)>,
    pub hint: String,
    pub style: OverlayStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferOutcome {
    /// Add the candidate at this position, with this one of its profiles in use for the
    /// window (`None`: the pack's own rules stand).
    Add(usize, Option<usize>),
    /// Not now: asked again once the game has been started again (a tap of Guide in it).
    No,
    /// Never ask about these packs again.
    Never,
}

struct Offered {
    name: String,
    description: String,
    id: String,
    /// The profiles the controllers can play, with their positions in the pack.
    profiles: Vec<(usize, String)>,
    /// How many profiles the pack has in all.
    total: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Stage {
    Ask,
    /// Choosing between packs.
    Pack,
    /// Choosing between the profiles of pack `.0`.
    Profile(usize),
}

pub struct OfferSession {
    packs: Vec<Offered>,
    stage: Stage,
    selected: usize,
    last_input: Instant,
    /// The left stick, read as a D-pad, and the direction repeating while held.
    stick_dir: crate::nav::StickDir,
    repeat: crate::nav::Repeat,
}

impl OfferSession {
    /// The stage and the highlighted row, for the sounds of its steps and picks.
    pub(crate) fn position(&self) -> (Stage, usize) {
        (self.stage, self.selected)
    }

    pub fn new(library: &[Entry], candidates: &[usize], have: Option<&[Feature]>, now: Instant) -> Self {
        let packs = candidates
            .iter()
            .map(|&i| {
                let pack = &library[i].pack;
                let p = &pack.pack;
                Offered {
                    name: p.name.clone(),
                    description: p.description.clone(),
                    id: p.id.clone(),
                    profiles: pack
                        .profiles
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| have.is_none_or(|h| p.usable_with(h)))
                        .map(|(i, p)| (i, p.name.clone()))
                        .collect(),
                    total: pack.profiles.len(),
                }
            })
            .collect();
        OfferSession {
            packs,
            stage: Stage::Ask,
            selected: 0,
            last_input: now,
            stick_dir: crate::nav::StickDir::default(),
            repeat: crate::nav::Repeat::default(),
        }
    }

    /// The IDs of the packs on offer.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.packs.iter().map(|p| p.id.as_str())
    }

    /// What the overlay draws, in `style` (the menus' look): the question, or the list to pick
    /// a set of profiles from, drawn as a list menu.
    pub fn view(&self, style: &OverlayStyle) -> crate::overlay::OverlayView {
        use crate::{menu::MenuView, overlay::OverlayView};
        let pick = |title: String, choices: Vec<String>| {
            let items = choices.into_iter().map(|label| crate::menu::ItemView { label, ..crate::menu::ItemView::plain() }).collect();
            OverlayView::Menu(MenuView {
                title,
                kind: crate::config::MenuKind::List,
                items,
                selected: Some(self.selected),
                picks: 0,
                crumbs: vec!["Add controller profiles".into()],
                hint: "{up} {down} choose · {south} add · {east} back".into(),
                style: style.clone(),
                touch: false,
                aim: None,
                prompt: None,
            })
        };
        match self.stage {
            Stage::Pack => return pick("Which set of profiles?".into(), self.packs.iter().map(|p| p.name.clone()).collect()),
            Stage::Profile(i) => {
                return pick(format!("Which {} profile?", self.packs[i].name), self.packs[i].profiles.iter().map(|(_, n)| n.clone()).collect());
            }
            Stage::Ask => {}
        }
        let (title, detail) = match self.packs.as_slice() {
            [p] => (format!("Add controller profiles for {}?", p.name), p.description.clone()),
            [p, ..] => (format!("Add controller profiles for {}?", p.name), format!("{} sets of profiles fit this game", self.packs.len())),
            [] => (String::new(), String::new()),
        };
        let answers = [("{south}", "Yes"), ("{east}", "Not now"), ("{west}", "Never")];
        OverlayView::Offer(OfferView {
            title,
            detail,
            answers: answers.iter().map(|(b, t)| ((*b).into(), (*t).into())).collect(),
            hint: "Not now asks again the next time this game starts and you tap Guide in it".into(),
            style: style.clone(),
        })
    }

    pub fn handle(&mut self, ev: InputEvent, now: Instant) -> Option<OfferOutcome> {
        self.last_input = now;
        // Up and down move through a list, from the D-pad or the left stick, repeating while held.
        let dir = match ev {
            InputEvent::Button(b, pressed) if crate::nav::dpad_dir(b).is_some() => {
                let dir = crate::nav::dpad_dir(b)?;
                if !pressed {
                    self.repeat.release(dir);
                    return None;
                }
                Some(dir)
            }
            InputEvent::Axis(crate::input::Axis::LeftY, v) => match self.stick_dir.update((0.0, v))? {
                Some(dir) => Some(dir),
                None => {
                    self.repeat.stop();
                    return None;
                }
            },
            _ => None,
        };
        if let Some(dir) = dir {
            if self.stage != Stage::Ask && dir.1 != 0 {
                self.repeat.start(dir, now);
                self.move_cursor(dir.1);
            }
            return None;
        }
        let InputEvent::Button(b, true) = ev else { return None };
        if self.stage != Stage::Ask {
            return self.handle_pick(b);
        }
        match b {
            Button::South if self.packs.len() == 1 => self.chosen_pack(0),
            Button::South => {
                self.stage = Stage::Pack;
                self.selected = 0;
                None
            }
            Button::East => Some(OfferOutcome::No),
            Button::West => Some(OfferOutcome::Never),
            _ => None,
        }
    }

    /// Pack `i` is chosen: add it, after asking for a profile if it has several.
    fn chosen_pack(&mut self, i: usize) -> Option<OfferOutcome> {
        let pack = &self.packs[i];
        if pack.profiles.len() > 1 {
            self.stage = Stage::Profile(i);
            self.selected = 0;
            return None;
        }
        // Some profiles don't suit the controllers: the one left is the one to use.
        let only = pack.profiles.first().filter(|_| pack.total > 1).map(|(at, _)| *at);
        Some(OfferOutcome::Add(i, only))
    }

    /// Moves the highlight `dy` rows through the list on show, wrapping.
    fn move_cursor(&mut self, dy: i32) {
        let len = match self.stage {
            Stage::Profile(i) => self.packs[i].profiles.len(),
            _ => self.packs.len(),
        };
        self.selected = (self.selected as i32 + dy).rem_euclid(len.max(1) as i32) as usize;
    }

    /// Repeats a held direction once it's due. Returns whether the highlight moved.
    pub fn tick(&mut self, now: Instant) -> bool {
        let Some(dir) = self.repeat.due(now) else { return false };
        self.move_cursor(dir.1);
        true
    }

    fn handle_pick(&mut self, b: Button) -> Option<OfferOutcome> {
        match (b, self.stage) {
            (Button::South, Stage::Profile(i)) => return Some(OfferOutcome::Add(i, Some(self.packs[i].profiles[self.selected].0))),
            (Button::South, _) => return self.chosen_pack(self.selected),
            (Button::East, Stage::Profile(_)) if self.packs.len() > 1 => {
                self.stage = Stage::Pack;
                self.selected = 0;
            }
            (Button::East, _) => self.stage = Stage::Ask,
            _ => {}
        }
        None
    }

    pub fn expired(&self, now: Instant) -> bool {
        self.last_input + IDLE_CLOSE <= now
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        let idle = self.last_input + IDLE_CLOSE;
        Some(self.repeat.deadline().map_or(idle, |r| r.min(idle)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Game, Profile, Rule, RuleKind},
        pack,
    };

    fn entry(name: &str, exe: &str) -> Entry {
        let mut game = Game::new(name, vec![Profile::passthrough("P")]);
        game.rules = vec![Rule::new(RuleKind::Executable, exe, "P")];
        let pack = pack::export(&game, &pack::draft(&game, false)).pack;
        Entry { file: "x.padpack", pack }
    }

    fn window(exe: &str) -> WindowInfo {
        WindowInfo { exe: exe.into(), class: "game".into(), ..Default::default() }
    }

    fn press(s: &mut OfferSession, b: Button) -> Option<OfferOutcome> {
        s.handle(InputEvent::Button(b, true), Instant::now())
    }

    /// The list on show, if the offer is at one, and its highlighted row.
    fn choices(s: &OfferSession) -> (Vec<String>, Option<usize>) {
        match s.view(&OverlayStyle::default()) {
            crate::overlay::OverlayView::Menu(m) => (m.items.into_iter().map(|i| i.label).collect(), m.selected),
            _ => (Vec::new(), None),
        }
    }

    #[test]
    fn only_unknown_library_games_are_offered() {
        let library = vec![entry("Doom", "doom.exe"), entry("Doom Remix", "DOOM.EXE"), entry("Quake", "quake.exe")];
        let mut config = Config::default();
        assert_eq!(candidates(&config, &library, &window("doom.exe"), None), [0, 1], "both packs fit");
        assert!(candidates(&config, &library, &window("other.exe"), None).is_empty());

        config.declined_packs.push(library[1].pack.pack.id.clone());
        assert_eq!(candidates(&config, &library, &window("doom.exe"), None), [0]);

        let plan = pack::plan(&config, library[0].pack.clone(), true);
        pack::apply(&mut config, &plan, &pack::Choices::default());
        assert!(candidates(&config, &library, &window("doom.exe"), None).is_empty(), "a setup with rules is the config's");
    }

    #[test]
    fn answers_and_picking() {
        let library = vec![entry("Doom", "doom.exe"), entry("Doom Remix", "doom.exe")];
        let one = |i: usize| OfferSession::new(&library, &[i], None, Instant::now());
        assert_eq!(press(&mut one(0), Button::South), Some(OfferOutcome::Add(0, None)));
        assert_eq!(press(&mut one(0), Button::East), Some(OfferOutcome::No));
        assert_eq!(press(&mut one(0), Button::West), Some(OfferOutcome::Never));
        assert_eq!(press(&mut one(0), Button::North), None);

        let mut two = OfferSession::new(&library, &[0, 1], None, Instant::now());
        assert_eq!(press(&mut two, Button::South), None, "yes leads to the list");
        assert_eq!(choices(&two).0, ["Doom", "Doom Remix"]);
        press(&mut two, Button::DpadDown);
        press(&mut two, Button::DpadDown);
        press(&mut two, Button::DpadUp);
        assert_eq!(press(&mut two, Button::South), Some(OfferOutcome::Add(1, None)));

        // A pack with several profiles asks which one.
        let mut multi = entry("Doom", "doom.exe");
        multi.pack.profiles.push(Profile::passthrough("Alt"));
        let library = vec![multi];
        let mut s = OfferSession::new(&library, &[0], None, Instant::now());
        assert_eq!(press(&mut s, Button::South), None);
        assert_eq!(choices(&s).0, ["P", "Alt"]);
        press(&mut s, Button::DpadDown);
        assert_eq!(press(&mut s, Button::South), Some(OfferOutcome::Add(0, Some(1))));
        press(&mut s, Button::East);
        assert!(choices(&s).0.is_empty(), "B from the profile list goes back to the question");

    }

    #[test]
    fn profiles_the_controller_cannot_play_are_not_offered() {
        // Profiles that need a gyro are neither listed nor offered without one.
        let mut gyro = entry("Doom", "doom.exe");
        gyro.pack.profiles[0].requires = vec![Feature::Gyro];
        gyro.pack.profiles.push(Profile::passthrough("Plain"));
        gyro.pack.profiles.push(Profile::passthrough("Plain 2"));
        let library = vec![gyro];
        let mut s = OfferSession::new(&library, &[0], Some(&[]), Instant::now());
        press(&mut s, Button::South);
        assert_eq!(choices(&s).0, ["Plain", "Plain 2"]);
        press(&mut s, Button::DpadDown);
        assert_eq!(press(&mut s, Button::South), Some(OfferOutcome::Add(0, Some(2))), "the position in the pack");
        let mut s = OfferSession::new(&library, &[0], Some(&[Feature::Gyro]), Instant::now());
        press(&mut s, Button::South);
        assert_eq!(choices(&s).0, ["P", "Plain", "Plain 2"]);

        let mut one = entry("Doom", "doom.exe");
        one.pack.profiles[0].requires = vec![Feature::Gyro];
        one.pack.profiles.push(Profile::passthrough("Plain"));
        let library = vec![one];
        let mut s = OfferSession::new(&library, &[0], Some(&[]), Instant::now());
        assert_eq!(press(&mut s, Button::South), Some(OfferOutcome::Add(0, Some(1))), "a lone fit is used without asking");
        let mut gyro_only = entry("Doom", "doom.exe");
        gyro_only.pack.profiles[0].requires = vec![Feature::Gyro];
        assert!(candidates(&Config::default(), &[gyro_only.clone()], &window("doom.exe"), Some(&[])).is_empty());
        assert_eq!(candidates(&Config::default(), &[gyro_only.clone()], &window("doom.exe"), Some(&[Feature::Gyro])), [0]);
        assert_eq!(candidates(&Config::default(), &[gyro_only], &window("doom.exe"), None), [0]);
    }

    #[test]
    fn the_stick_moves_through_the_list_and_holding_repeats() {
        let library = vec![entry("A", "a.exe"), entry("B", "a.exe"), entry("C", "a.exe")];
        let t0 = Instant::now();
        let mut s = OfferSession::new(&library, &[0, 1, 2], None, t0);
        press(&mut s, Button::South);
        s.handle(InputEvent::Axis(crate::input::Axis::LeftY, 0.9), t0);
        assert_eq!(choices(&s).1.unwrap(), 1);
        assert!(s.tick(t0 + crate::nav::REPEAT_DELAY));
        assert_eq!(choices(&s).1.unwrap(), 2);
        s.handle(InputEvent::Axis(crate::input::Axis::LeftY, 0.0), t0);
        assert!(!s.tick(t0 + crate::nav::REPEAT_DELAY * 4));
    }

    #[test]
    fn b_goes_back_to_the_question() {
        let library = vec![entry("Doom", "doom.exe"), entry("Doom Remix", "doom.exe")];
        let mut back = OfferSession::new(&library, &[0, 1], None, Instant::now());
        press(&mut back, Button::South);
        assert_eq!(press(&mut back, Button::East), None, "B goes back to the question");
        assert!(choices(&back).0.is_empty());
    }
}
