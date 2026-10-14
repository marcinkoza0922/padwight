//! The menu the Quick Settings shortcut (Guide + Start unless moved) opens over the game. It isn't in any profile: Quick Settings (a few
//! values of the active profile) comes first, and Edit Controls last. The daemon routes
//! controller input here while it is open, and the overlay draws its [`MenuView`].

use std::time::Instant;

mod profiles;
mod quick;
use quick::Row;

use crate::{
    layout_editor::{EditStep, LayoutEditor},
    config::{Button, Config, Feature, OverlayStyle, Profile, ProfileRef},
    input::{Axis, InputEvent},
    menu::{ItemView, MenuView, Tone},
    offer::OfferView,
};

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
    /// Closing with changes not saved: save, discard, or keep editing, which goes back to the
    /// screen (and cursor) the menu was being left from. Drawn over that screen.
    Confirm(Box<Screen>, usize, Exit),
}

/// Why the menu is on hold, drawn as a prompt over it until it carries on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// The controller driving it went away: the next controller to press a button takes over.
    NoController,
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
        let dir = self.repeat.due(now)?;
        self.step(dir_step(dir), config)
    }

    /// When [`Self::tick`] next has a repeat to make.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.repeat.deadline()
    }

    fn step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        match self.screen {
            Screen::Main => self.main_step(step, config),
            Screen::Confirm(..) => self.confirm_step(step),
            Screen::Quick => self.quick_step(step, config),
            Screen::Editor(_) => self.editor_step(step, config),
            Screen::Profiles => self.profiles_step(step, config),
            Screen::NewProfile => self.new_profile_step(step, config),
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

    /// Leaves the menu's top page, `how`: back to the Guide overlay, or closes. Unsaved changes
    /// ask first.
    fn leave(&mut self, config: &Config, how: Exit) -> Option<Outcome> {
        if *self.saved == *config {
            return Some(if self.from_guide && how == Exit::Back { Outcome::ReturnToGuide } else { Outcome::Close });
        }
        let left = std::mem::replace(&mut self.screen, Screen::Main);
        self.screen = Screen::Confirm(Box::new(left), self.cursor, how);
        None
    }

    /// Guide was pressed: closes the menu outright, unless there are unsaved changes, which it
    /// asks about first (the question then needs an answer).
    pub fn guide_pressed(&mut self, config: &Config) -> Option<Outcome> {
        match self.screen {
            _ if self.hold.is_some() => None,
            Screen::Confirm(..) => None,
            _ if *self.saved == *config => Some(Outcome::Close),
            _ => self.leave(config, Exit::Close),
        }
    }

    /// The config as it was when the menu opened, which Discard puts back.
    pub fn saved(&self) -> &Config {
        &self.saved
    }

    /// Whether `config` has changes the menu hasn't saved.
    pub fn unsaved(&self, config: &Config) -> bool {
        *self.saved != *config
    }

    /// Remapping was turned on or off elsewhere, which is saved at once: it isn't a change of
    /// the menu's.
    pub fn remapping_set(&mut self, enabled: bool) {
        self.saved.enabled = enabled;
    }

    /// Puts the menu on hold, `why`. A held direction stops repeating.
    pub fn hold(&mut self, why: Hold) {
        self.hold = Some(why);
        self.repeat.stop();
        self.stick = (0.0, 0.0);
        self.stick_dir = crate::nav::StickDir::default();
        self.remapping_armed = false;
    }

    /// Takes the menu off hold, where it was.
    pub fn carry_on(&mut self) {
        self.hold = None;
    }

    /// Why the menu is on hold, if it is.
    pub fn on_hold(&self) -> Option<Hold> {
        self.hold
    }

    /// Input while on hold: once remapping is off, A saves and X discards, then the menu closes.
    fn held(&self, hold: Hold, ev: InputEvent) -> Option<Outcome> {
        match (hold, ev) {
            (Hold::RemappingOff, InputEvent::Button(Button::South, true)) => Some(Outcome::Save { back: false }),
            (Hold::RemappingOff, InputEvent::Button(Button::West, true)) => Some(Outcome::Discard { saved: self.saved.clone(), back: false }),
            _ => None,
        }
    }

    /// Back from the unsaved-changes question to the screen it was asked on.
    fn keep_editing(&mut self) {
        if let Screen::Confirm(left, cursor, _) = std::mem::replace(&mut self.screen, Screen::Main) {
            self.screen = *left;
            self.cursor = cursor;
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
        if self.from_guide { "Guide" } else { "Menu" }.into()
    }

    /// Whether answering the unsaved-changes question goes back to the Guide overlay, not out.
    fn answer_goes_back(&self) -> bool {
        matches!(self.screen, Screen::Confirm(_, _, Exit::Back)) && self.from_guide
    }

    fn discard(&self) -> Option<Outcome> {
        Some(Outcome::Discard { saved: self.saved.clone(), back: self.answer_goes_back() })
    }

    /// The unsaved-changes question: A saves, X discards, B keeps editing.
    fn confirm_step(&mut self, step: Step) -> Option<Outcome> {
        match step {
            Step::Pick => Some(Outcome::Save { back: self.answer_goes_back() }),
            Step::Back => {
                self.keep_editing();
                None
            }
            Step::Up | Step::Down | Step::Left | Step::Right => None,
        }
    }

    /// Back to the main page, from wherever the menu is.
    pub fn back_to_main(&mut self) {
        self.screen = Screen::Main;
        self.cursor = 0;
    }

    fn quick_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let rows = quick::rows(config, &self.saved);
        let armed = std::mem::take(&mut self.remapping_armed);
        match step {
            Step::Up | Step::Down => {
                self.cursor = wrap(self.cursor, step, rows.len());
                None
            }
            // Turning remapping off takes a second press of A.
            Step::Pick if rows.get(self.cursor) == Some(&Row::Remapping) && !armed => {
                self.remapping_armed = true;
                None
            }
            Step::Left | Step::Right if rows.get(self.cursor) == Some(&Row::Remapping) => None,
            Step::Back if self.from_guide => self.leave(config, Exit::Back),
            Step::Back => {
                self.screen = Screen::Main;
                self.cursor = 0;
                None
            }
            Step::Left | Step::Right | Step::Pick => {
                let row = *rows.get(self.cursor)?;
                let dir = match step {
                    Step::Left => -1,
                    _ => 1,
                };
                quick::adjust(row, dir, self.big_steps, config, &self.saved)
            }
        }
    }

    fn profiles_step(&mut self, step: Step, config: &Config) -> Option<Outcome> {
        let rows = profiles::rows(config);
        match step {
            Step::Up | Step::Down | Step::Left | Step::Right => self.cursor = list_move(self.cursor, step, rows.len()),
            Step::Pick => match *rows.get(self.cursor)? {
                profiles::Row::New => {
                    self.screen = Screen::NewProfile;
                    self.cursor = 0;
                }
                row => {
                    let target = profiles::target(row, config)?;
                    if target == config.active_ref() {
                        return self.leave(config, Exit::Close);
                    }
                    return Some(Outcome::Switch(target));
                }
            },
            Step::Back => return self.leave(config, Exit::Back),
        }
        None
    }

    fn new_profile_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let rows = profiles::new_rows(config);
        match step {
            Step::Up | Step::Down | Step::Left | Step::Right => self.cursor = list_move(self.cursor, step, rows.len()),
            Step::Pick => return profiles::create(*rows.get(self.cursor)?, config).map(Outcome::Added),
            Step::Back => {
                self.screen = Screen::Profiles;
                self.cursor = profiles::rows(config).len() - 1;
            }
        }
        None
    }

    fn editor_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let Screen::Editor(editor) = &mut self.screen else { return None };
        match step {
            Step::Up => editor.move_cursor((0, -1), config),
            Step::Down => editor.move_cursor((0, 1), config),
            Step::Pick => return (editor.choose(config) == EditStep::Changed).then_some(Outcome::Changed),
            Step::Back => {
                if editor.back() == EditStep::Leave {
                    if self.from_guide {
                        return self.leave(config, Exit::Back);
                    }
                    self.screen = Screen::Main;
                    self.cursor = 1;
                }
            }
            Step::Left | Step::Right if editor.two_columns(config) => {
                editor.move_cursor((if step == Step::Left { -1 } else { 1 }, 0), config);
            }
            Step::Left => return (editor.adjust(-1, config) == EditStep::Changed).then_some(Outcome::Changed),
            Step::Right => return (editor.adjust(1, config) == EditStep::Changed).then_some(Outcome::Changed),
        }
        None
    }

    /// What the overlay draws now. The unsaved-changes question is drawn over the screen it was
    /// asked on.
    pub fn view(&self, config: &Config) -> MenuView {
        let mut view = match &self.screen {
            Screen::Confirm(left, cursor, how) => {
                let mut view = self.screen_view(left, *cursor, config);
                view.prompt = Some(Box::new(self.unsaved_prompt(*how, view.style.clone())));
                view
            }
            screen => self.screen_view(screen, self.cursor, config),
        };
        if let Some(hold) = self.hold {
            view.prompt = Some(Box::new(hold_prompt(hold, view.style.clone())));
        }
        view
    }

    /// The question asked when the menu is left with changes not saved, in the look of the offer
    /// to add a game.
    fn unsaved_prompt(&self, how: Exit, mut style: OverlayStyle) -> OfferView {
        // Solid, so the page under it doesn't show through the words.
        style.background.opacity = 1.0;
        let then = if self.from_guide && how == Exit::Back { "back to the Guide" } else { "close the menu" };
        OfferView {
            title: "Save your changes?".into(),
            detail: "You changed settings that aren't saved yet.".into(),
            answers: [("{confirm}", "Save"), ("{west}", "Discard"), ("{back}", "Keep editing")]
                .iter()
                .map(|(b, t)| ((*b).into(), (*t).into()))
                .collect(),
            hint: format!("Save or Discard, then {then}"),
            style,
        }
    }

    fn screen_view(&self, screen: &Screen, cursor: usize, config: &Config) -> MenuView {
        let style = config.active_menu_style().clone();
        match screen {
            Screen::Main => main_page(style, cursor),
            Screen::Editor(editor) => editor.view(config, &[self.parent()]),
            Screen::Confirm(left, cursor, _) => self.screen_view(left, *cursor, config),
            Screen::Profiles => list_page(
                "Switch Profile",
                profiles::rows(config).into_iter().map(|r| profiles::item(r, config, self.features.as_deref())).collect(),
                cursor,
                vec![self.parent()],
                style,
            ),
            Screen::NewProfile => list_page(
                "New profile",
                profiles::new_rows(config).into_iter().map(|r| profiles::new_item(r, config)).collect(),
                cursor,
                vec![self.parent(), "Switch Profile".into()],
                style,
            ),
            Screen::Quick => MenuView {
                title: "Quick Settings".into(),
                kind: crate::config::MenuKind::List,
                items: quick::rows(config, &self.saved)
                    .into_iter()
                    .map(|row| {
                        let label = match row {
                            Row::Remapping if self.remapping_armed => "Press {confirm} again to turn remapping off".into(),
                            row => quick::label(row, config, &self.saved),
                        };
                        ItemView { tone: if row == Row::Remapping { Tone::Remove } else { Tone::Normal }, label, ..ItemView::plain() }
                    })
                    .collect(),
                selected: Some(cursor),
                picks: 0,
                crumbs: vec![self.parent()],
                hint: "{left} {right} or {confirm} change · hold {rb} for big steps · {back} back".into(),
                style,
                touch: false,
                aim: None,
                prompt: None,
            },
        }
    }
}

/// A page that is a plain list: {confirm} chooses, {back} goes back.
fn list_page(title: &str, items: Vec<ItemView>, cursor: usize, crumbs: Vec<String>, style: OverlayStyle) -> MenuView {
    MenuView {
        title: title.into(),
        kind: crate::config::MenuKind::List,
        items,
        selected: Some(cursor),
        picks: 0,
        crumbs,
        hint: "{confirm} choose · {back} back".into(),
        style,
        touch: false,
        aim: None,
        prompt: None,
    }
}

/// What the menu shows while on hold, in the look of the unsaved-changes question.
fn hold_prompt(hold: Hold, mut style: OverlayStyle) -> OfferView {
    style.background.opacity = 1.0;
    let (title, detail, answers, hint) = match hold {
        Hold::NoController => (
            "Controller disconnected",
            "Reconnect it, or press a button on another controller, to carry on.",
            Vec::new(),
            "The menu waits where it was",
        ),
        Hold::RemappingOff => (
            "Remapping is off",
            "It was turned off while you had changes here that aren't saved yet.",
            vec![("{confirm}", "Save"), ("{west}", "Discard")],
            "Save or Discard, then the controller is let go",
        ),
    };
    OfferView {
        title: title.into(),
        detail: detail.into(),
        answers: answers.into_iter().map(|(b, t)| (b.into(), t.into())).collect(),
        hint: hint.into(),
        style,
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

/// The menu's first page, drawn with `style`; the settings previews show it too.
pub fn main_page(style: OverlayStyle, cursor: usize) -> MenuView {
    MenuView {
        title: "Menu".into(),
        kind: crate::config::MenuKind::List,
        items: vec![
            ItemView { label: "Quick Settings".into(), button: None, submenu: true, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
            ItemView { label: "Edit Controls".into(), button: None, submenu: false, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
        ],
        selected: Some(cursor),
        picks: 0,
        crumbs: Vec::new(),
        hint: "{confirm} choose · {back} close".into(),
        style,
        touch: false,
        aim: None,
        prompt: None,
    }
}

#[cfg(test)]
mod tests;
