//! The menu the Quick Settings shortcut (Guide + Start unless moved) opens over the game. It isn't in any profile: Quick Settings (a few
//! values of the active profile) comes first, and Edit Controls last. The daemon routes
//! controller input here while it is open, and the overlay draws its [`MenuView`].

use std::time::Instant;

mod profiles;
mod quick;
mod leaving;
mod steps;
mod view;
pub use view::main_page;
use quick::Row;

use crate::{
    layout_editor::{EditStep, LayoutEditor},
    config::{Button, Config, Feature, OverlayStyle, Profile, ProfileRef},
    input::{Axis, InputEvent},
    overlay::{OverlayAction, OverlayController, OverlayView},
    menu::{ItemView, MenuView, Tone},
    offer::OfferView,
};

/// The menu's name on screen, as App settings names it.
const TITLE: &str = "In-game menu";

/// What the menu asks the daemon to do after a choice.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Put the overlay away.
    Close,
    /// A value changed in the config: save it and apply it.
    Changed,
    /// Switch to this profile. The menu stays up, but for Switch Profile, which then leaves
    /// (see [`SystemMenu::profile_switched`]).
    Switch(ProfileRef),
    /// A new profile was added to the config: apply it and switch to it.
    Added(ProfileRef),
    /// Edit Controls was chosen: the daemon makes a pack for the game if there isn't one.
    EditControls,
    /// Save the changes to the config file, then close, or go `back` to the Guide overlay.
    Save { back: bool },
    /// Put the config back as it was when the menu opened, then close, or go `back` to the Guide
    /// overlay.
    Discard { saved: Box<Config>, back: bool },
    /// Back to the Guide overlay, which the menu was opened from.
    ReturnToGuide,
    /// Stop remapping altogether (Quick Settings' last row, chosen twice).
    RemappingOff,
}

/// The page a menu opens on. The Guide overlay opens Quick Settings, Edit Controls or Switch
/// Profile directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Main,
    Quick,
    EditControls,
    Profiles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Screen {
    Main,
    Quick,
    Editor(LayoutEditor),
    /// Switch Profile's list, and what a new profile starts from.
    Profiles,
    NewProfile,
    /// Typing into a field of the editor, drawn as the on-screen keyboard. The editor waits under
    /// it, on the row that asked.
    Typing(LayoutEditor, Box<Typing>),
    /// Closing with changes not saved: save, discard, or keep editing, which goes back to the
    /// screen (and cursor) the menu was being left from. Drawn over that screen.
    Confirm(Box<Screen>, usize, Exit),
}

/// The keyboard typing into a field. Two are equal when they hold the same text.
#[derive(Debug, Clone)]
pub struct Typing(OverlayController);

impl PartialEq for Typing {
    fn eq(&self, other: &Self) -> bool {
        self.0.typed() == other.0.typed()
    }
}

impl Eq for Typing {}

/// Why the menu is on hold, drawn as a prompt over it until it carries on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// Remapping was turned off elsewhere (the tray) with changes not saved: the controller
    /// driving the menu is kept until Save or Discard.
    RemappingOff,
}

/// How the menu was being left when it asked about unsaved changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    /// Back (B): to the Guide overlay the menu was opened from, if it was.
    Back,
    /// Guide: out of the menu altogether.
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Up,
    Down,
    Left,
    Right,
    Pick,
    Back,
}

/// The open menu: which screen, and where the cursor is on it.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemMenu {
    screen: Screen,
    cursor: usize,
    /// Where the left stick is, read as a D-pad, and the direction repeating while held.
    stick: (f32, f32),
    stick_dir: crate::nav::StickDir,
    repeat: crate::nav::Repeat,
    /// Opened from the Guide overlay: leaving the page goes back to it.
    from_guide: bool,
    /// The config as it was when the menu opened: what Discard goes back to, and what counts as
    /// unsaved.
    saved: Box<Config>,
    /// The controller driving the menu has back paddles, which Edit Controls then lists.
    paddles: bool,
    /// What the controllers have, so Switch Profile can say what a profile needs. `None`
    /// without a controller.
    features: Option<Vec<Feature>>,
    /// RB is held: Quick Settings' numbers change in bigger steps.
    big_steps: bool,
    /// Turn remapping off was chosen once; choosing it again does it.
    remapping_armed: bool,
    /// On hold, and why.
    hold: Option<Hold>,
}

impl SystemMenu {
    /// Where the cursor is, for the sounds of its steps.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// A menu over a config that is currently saved as `saved`.
    pub fn new(saved: &Config) -> Self {
        SystemMenu {
            screen: Screen::Main,
            cursor: 0,
            stick: (0.0, 0.0),
            stick_dir: crate::nav::StickDir::default(),
            repeat: crate::nav::Repeat::default(),
            from_guide: false,
            saved: Box::new(saved.clone()),
            paddles: false,
            features: None,
            big_steps: false,
            remapping_armed: false,
            hold: None,
        }
    }

    /// This menu, for a controller with back paddles (or without). Call before [`Self::on_page`].
    pub fn with_paddles(self, paddles: bool) -> Self {
        SystemMenu { paddles, ..self }
    }

    /// This menu, for controllers with `features` (`None` without any).
    pub fn with_features(self, features: Option<Vec<Feature>>) -> Self {
        SystemMenu { features, ..self }
    }

    /// This menu, opened on `page`. Edit Controls still needs the daemon's checks (see
    /// [`Outcome::EditControls`]).
    pub fn on_page(mut self, page: Page) -> Self {
        self.from_guide = page != Page::Main;
        self.screen = match page {
            Page::Main => Screen::Main,
            Page::Quick => Screen::Quick,
            Page::EditControls => Screen::Editor(LayoutEditor::new().with_paddles(self.paddles)),
            Page::Profiles => Screen::Profiles,
        };
        self
    }

    /// Handles one controller event. `None` when nothing needs the daemon. A direction held on
    /// the D-pad or the left stick repeats through [`Self::tick`].
    pub fn handle(&mut self, ev: InputEvent, now: Instant, config: &mut Config) -> Option<Outcome> {
        if let Some(hold) = self.hold {
            return self.held(hold, ev);
        }
        if let Screen::Typing(_, typing) = &mut self.screen {
            let actions = typing.0.handle(ev, now);
            return self.typed(actions, config);
        }
        let step = match ev {
            InputEvent::Button(b, pressed) if crate::nav::dpad_dir(b).is_some() => {
                let dir = crate::nav::dpad_dir(b)?;
                if !pressed {
                    self.repeat.release(dir);
                    return None;
                }
                self.repeat.start(dir, now);
                dir_step(dir)
            }
            InputEvent::Button(Button::South, true) => Step::Pick,
            InputEvent::Button(Button::East, true) => Step::Back,
            InputEvent::Button(Button::West, true) if matches!(self.screen, Screen::Confirm(..)) => return self.discard(),
            // On Quick Settings RB makes big steps; in Edit Controls the bumpers jump a page.
            InputEvent::Button(b @ (Button::LeftBumper | Button::RightBumper), held) => {
                self.big_steps = held && b == Button::RightBumper;
                if let Screen::Editor(editor) = &mut self.screen
                    && held
                {
                    editor.jump_page(if b == Button::LeftBumper { -1 } else { 1 }, config);
                }
                return None;
            }
            // A page that shows the left stick live leaves it to the picture.
            InputEvent::Axis(Axis::LeftX | Axis::LeftY, _) if self.shows_left_stick() => return None,
            InputEvent::Axis(axis @ (Axis::LeftX | Axis::LeftY), v) => {
                if axis == Axis::LeftX { self.stick.0 = v } else { self.stick.1 = v }
                match self.stick_dir.update(self.stick)? {
                    Some(dir) => {
                        self.repeat.start(dir, now);
                        dir_step(dir)
                    }
                    None => {
                        self.repeat.stop();
                        return None;
                    }
                }
            }
            _ => return None,
        };
        self.step(step, config)
    }

    /// Repeats a held direction once it's due.
    pub fn tick(&mut self, now: Instant, config: &mut Config) -> Option<Outcome> {
        if let Screen::Typing(_, typing) = &mut self.screen {
            let (actions, _) = typing.0.tick(now);
            return self.typed(actions, config);
        }
        let dir = self.repeat.due(now)?;
        self.step(dir_step(dir), config)
    }

    /// When [`Self::tick`] next has a repeat to make.
    pub fn next_deadline(&self) -> Option<Instant> {
        match &self.screen {
            Screen::Typing(_, typing) => typing.0.next_deadline(),
            _ => self.repeat.deadline(),
        }
    }

    /// Whether the keyboard is up, typing into a field.
    #[cfg(test)]
    pub fn typing(&self) -> bool {
        matches!(self.screen, Screen::Typing(..))
    }

    fn step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        match self.screen {
            Screen::Main => self.main_step(step, config),
            Screen::Confirm(..) => self.confirm_step(step),
            Screen::Quick => self.quick_step(step, config),
            Screen::Editor(_) => self.editor_step(step, config),
            Screen::Profiles => self.profiles_step(step, config),
            Screen::NewProfile => self.new_profile_step(step, config),
            Screen::Typing(..) => None,
        }
    }

    fn main_step(&mut self, step: Step, config: &Config) -> Option<Outcome> {
        match step {
            Step::Up | Step::Down => {
                self.cursor = wrap(self.cursor, step, 2);
                None
            }
            Step::Pick => match self.cursor {
                0 => {
                    self.screen = Screen::Quick;
                    self.cursor = 0;
                    None
                }
                _ => {
                    self.screen = Screen::Editor(LayoutEditor::new().with_paddles(self.paddles));
                    Some(Outcome::EditControls)
                }
            },
            Step::Back => self.leave(config, Exit::Back),
            Step::Left | Step::Right => None,
        }
    }

    /// Whether the menu was opened from the Guide overlay, so leaving goes back to it.
    pub fn opened_from_guide(&self) -> bool {
        self.from_guide
    }

    /// After a switch to a profile: one that was saved isn't a change to save. Quick Settings
    /// puts the cursor back on its Profile row, whose rows may differ; Switch Profile leaves, which
    /// may ask about unsaved changes first (the outcome is what leaving asks of the daemon).
    pub fn profile_switched(&mut self, config: &Config) -> Option<Outcome> {
        if profiles::has(&self.saved, &config.active) {
            self.saved.active = config.active.clone();
        }
        match self.screen {
            Screen::Quick => {
                let rows = quick::rows(config, &self.saved);
                self.cursor = rows.iter().position(|r| *r == Row::Profile).unwrap_or(0).min(rows.len().saturating_sub(1));
                None
            }
            Screen::Profiles => self.leave(config, Exit::Close),
            _ => None,
        }
    }

    /// After a new profile was added and switched to: the list, with the cursor on it.
    pub fn profile_added(&mut self, config: &Config) {
        self.screen = Screen::Profiles;
        let at = config.active_ref();
        self.cursor = config.active_game().profiles.iter().position(|p| p.name == at.profile).unwrap_or(0);
    }

    /// The breadcrumb for the menu's own pages: where it was opened from.
    fn parent(&self) -> String {
        if self.from_guide { "Guide" } else { TITLE }.into()
    }

    /// Back to the main page, from wherever the menu is.
    pub fn back_to_main(&mut self) {
        self.screen = Screen::Main;
        self.cursor = 0;
    }

}

/// The step a direction makes.
fn dir_step((x, y): crate::nav::Dir) -> Step {
    match (x, y) {
        (_, -1) => Step::Up,
        (_, 1) => Step::Down,
        (-1, _) => Step::Left,
        _ => Step::Right,
    }
}

/// Where `step` moves the cursor of a list page of `len` rows, which may be in two columns.
fn list_move(cursor: usize, step: Step, len: usize) -> usize {
    let dir = match step {
        Step::Up => (0, -1),
        Step::Down => (0, 1),
        Step::Left => (-1, 0),
        _ => (1, 0),
    };
    crate::menu::list_step(cursor, len, dir)
}

/// The next index `step` moves to in a list of `len`, wrapping at both ends.
fn wrap(cursor: usize, step: Step, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    match step {
        Step::Up => (cursor + len - 1) % len,
        _ => (cursor + 1) % len,
    }
}

/// The profile the menu edits: the active one, in its game (or General).
pub(crate) fn active_profile_mut(config: &mut Config) -> Option<&mut Profile> {
    let at = config.active_ref();
    config.game_mut(at.game.as_deref())?.profiles.iter_mut().find(|p| p.name == at.profile)
}

#[cfg(test)]
mod tests;
