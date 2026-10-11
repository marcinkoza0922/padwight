//! The menu Guide + Start opens over the game. It isn't in any profile: Quick Settings (a few
//! values of the active profile) comes first, and Edit Controls last. The daemon routes
//! controller input here while it is open, and the overlay draws its [`MenuView`].

use std::{collections::HashSet, time::Instant};

mod quick;
use quick::Row;

use crate::{
    layout_editor::{EditStep, LayoutEditor},
    config::{Button, Config, OverlayStyle, Profile, ProfileRef},
    input::{Axis, InputEvent},
    menu::{ItemView, MenuView, Tone},
};

/// What the menu asks the daemon to do after a choice.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Put the overlay away.
    Close,
    /// A value changed in the config: save it and apply it.
    Changed,
    /// Switch to this profile (the menu stays up).
    Switch(ProfileRef),
    /// Edit Controls was chosen: the daemon makes a pack for the game if there isn't one.
    EditControls,
    /// Save the changes to the config file, then close.
    Save,
    /// Put the config back as it was when the menu opened, then close.
    Discard(Box<Config>),
    /// Back to the Guide overlay, which the menu was opened from.
    ReturnToGuide,
    /// Stop remapping altogether (Quick Settings' last row, chosen twice).
    RemappingOff,
}

/// The page a menu opens on. The Guide overlay opens Quick Settings or Edit Controls directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Main,
    Quick,
    EditControls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Screen {
    Main,
    Quick,
    Editor(LayoutEditor),
    /// Closing with changes not saved: save, discard, or keep editing, which goes back to the
    /// screen (and cursor) the menu was being left from.
    Confirm(Box<Screen>, usize),
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
    /// RB is held: Quick Settings' numbers change in bigger steps.
    big_steps: bool,
    /// Turn remapping off was chosen once; choosing it again does it.
    remapping_armed: bool,
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
            big_steps: false,
            remapping_armed: false,
        }
    }

    /// This menu, for a controller with back paddles (or without). Call before [`Self::on_page`].
    pub fn with_paddles(self, paddles: bool) -> Self {
        SystemMenu { paddles, ..self }
    }

    /// This menu, opened on `page`. Edit Controls still needs the daemon's checks (see
    /// [`Outcome::EditControls`]).
    pub fn on_page(mut self, page: Page) -> Self {
        self.from_guide = page != Page::Main;
        self.screen = match page {
            Page::Main => Screen::Main,
            Page::Quick => Screen::Quick,
            Page::EditControls => Screen::Editor(LayoutEditor::new().with_paddles(self.paddles)),
        };
        self
    }

    /// Handles one controller event. `None` when nothing needs the daemon. A direction held on
    /// the D-pad or the left stick repeats through [`Self::tick`].
    pub fn handle(&mut self, ev: InputEvent, now: Instant, config: &mut Config) -> Option<Outcome> {
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
            Step::Back => self.leave(config),
            Step::Left | Step::Right => None,
        }
    }

    /// Leaves the menu's top page: back to the Guide overlay, or closes. Unsaved changes ask first.
    fn leave(&mut self, config: &Config) -> Option<Outcome> {
        if *self.saved == *config {
            return Some(if self.from_guide { Outcome::ReturnToGuide } else { Outcome::Close });
        }
        let left = std::mem::replace(&mut self.screen, Screen::Main);
        self.screen = Screen::Confirm(Box::new(left), self.cursor);
        self.cursor = 0;
        None
    }

    /// Guide was pressed: closes the menu outright, unless there are unsaved changes, which it
    /// asks about first (the question then needs an answer).
    pub fn guide_pressed(&mut self, config: &Config) -> Option<Outcome> {
        match self.screen {
            Screen::Confirm(..) => None,
            _ if *self.saved == *config => Some(Outcome::Close),
            _ => self.leave(config),
        }
    }

    /// Back from the unsaved-changes question to the screen it was asked on.
    fn keep_editing(&mut self) {
        if let Screen::Confirm(left, cursor) = std::mem::replace(&mut self.screen, Screen::Main) {
            self.screen = *left;
            self.cursor = cursor;
        }
    }

    /// Whether the menu was opened from the Guide overlay, so leaving goes back to it.
    pub fn opened_from_guide(&self) -> bool {
        self.from_guide
    }

    /// Puts the cursor back on Quick Settings' Profile row after a switch, whose rows may differ.
    pub fn profile_switched(&mut self, config: &Config) {
        if self.screen == Screen::Quick {
            let rows = quick::rows(config, &self.saved);
            self.cursor = rows.iter().position(|r| *r == Row::Profile).unwrap_or(0).min(rows.len().saturating_sub(1));
        }
    }

    /// The breadcrumb for the menu's own pages: where it was opened from.
    fn parent(&self) -> String {
        if self.from_guide { "Guide" } else { "Menu" }.into()
    }

    fn confirm_step(&mut self, step: Step) -> Option<Outcome> {
        match step {
            Step::Up | Step::Down => {
                self.cursor = wrap(self.cursor, step, 3);
                None
            }
            Step::Pick => match self.cursor {
                0 => Some(Outcome::Save),
                1 => Some(Outcome::Discard(self.saved.clone())),
                _ => {
                    self.keep_editing();
                    None
                }
            },
            Step::Back => {
                self.keep_editing();
                None
            }
            Step::Left | Step::Right => None,
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
            Step::Back if self.from_guide => self.leave(config),
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

    fn editor_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let Screen::Editor(editor) = &mut self.screen else { return None };
        match step {
            Step::Up => editor.move_cursor((0, -1), config),
            Step::Down => editor.move_cursor((0, 1), config),
            Step::Pick => return (editor.choose(config) == EditStep::Changed).then_some(Outcome::Changed),
            Step::Back => {
                if editor.back() == EditStep::Leave {
                    if self.from_guide {
                        return self.leave(config);
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

    /// What the overlay draws now.
    pub fn view(&self, config: &Config) -> MenuView {
        let style = config.active_menu_style().clone();
        match &self.screen {
            Screen::Main => main_page(style, self.cursor),
            Screen::Editor(editor) => editor.view(config, &[self.parent()]),
            Screen::Confirm(..) => MenuView {
                title: "Unsaved changes".into(),
                kind: crate::config::MenuKind::List,
                items: vec![
                    ItemView { label: "Save and close".into(), button: None, submenu: false, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
                    ItemView { label: "Discard and close".into(), button: None, submenu: false, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
                    ItemView { label: "Keep editing".into(), button: None, submenu: false, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: crate::config::Span::ONE },
                ],
                selected: Some(self.cursor),
                picks: 0,
                crumbs: vec![self.parent()],
                hint: "{south} choose · {east} keep editing".into(),
                style,
                touch: false,
                aim: None,
            },
            Screen::Quick => MenuView {
                title: "Quick Settings".into(),
                kind: crate::config::MenuKind::List,
                items: quick::rows(config, &self.saved)
                    .into_iter()
                    .map(|row| {
                        let label = match row {
                            Row::Remapping if self.remapping_armed => "Press {south} again to turn remapping off".into(),
                            row => quick::label(row, config, &self.saved),
                        };
                        ItemView { tone: if row == Row::Remapping { Tone::Remove } else { Tone::Normal }, label, ..ItemView::plain() }
                    })
                    .collect(),
                selected: Some(self.cursor),
                picks: 0,
                crumbs: vec![self.parent()],
                hint: "{left} {right} or {south} change · hold {rb} for big steps · {east} back".into(),
                style,
                touch: false,
                aim: None,
            },
        }
    }
}

/// True when these events press Guide and Start together: one of them goes down while the other
/// is already held. `held` is what was down before the events.
pub fn chord(held: &HashSet<Button>, events: &[InputEvent]) -> bool {
    let mut held = held.clone();
    for ev in events {
        let InputEvent::Button(b, down) = *ev else { continue };
        if down {
            let other = match b {
                Button::Start => Button::Guide,
                Button::Guide => Button::Start,
                _ => {
                    held.insert(b);
                    continue;
                }
            };
            if held.contains(&other) {
                return true;
            }
            held.insert(b);
        } else {
            held.remove(&b);
        }
    }
    false
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
        hint: "{south} choose · {east} close".into(),
        style,
        touch: false,
        aim: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MouseResponse, Stick, StickAction};

    fn press(b: Button) -> InputEvent {
        InputEvent::Button(b, true)
    }

    impl SystemMenu {
        fn handle_now(&mut self, ev: InputEvent, config: &mut Config) -> Option<Outcome> {
            self.handle(ev, Instant::now(), config)
        }
    }

    /// A config whose active profile has a mouse-moving left stick.
    fn mouse_config() -> Config {
        let mut config = Config::default();
        let profile = &mut config.general.profiles[0];
        profile.left_stick.action = StickAction::Mouse { speed: 1000.0, response: MouseResponse::default(), invert_y: false };
        config
    }

    fn speed(config: &Config) -> f32 {
        match config.active().unwrap().left_stick.action {
            StickAction::Mouse { speed, .. } => speed,
            _ => panic!("left stick is not a mouse"),
        }
    }

    #[test]
    fn chord_needs_both_buttons_together() {
        let none = HashSet::new();
        assert!(chord(&none, &[press(Button::Guide), press(Button::Start)]));
        assert!(chord(&HashSet::from([Button::Guide]), &[press(Button::Start)]));
        assert!(chord(&HashSet::from([Button::Start]), &[press(Button::Guide)]));
        assert!(!chord(&none, &[press(Button::Start)]));
        // Guide let go before Start goes down is not a chord.
        assert!(!chord(&none, &[press(Button::Guide), InputEvent::Button(Button::Guide, false), press(Button::Start)]));
        assert!(!chord(&HashSet::from([Button::Guide]), &[press(Button::South)]));
    }

    #[test]
    fn the_guide_rows_open_their_page_directly() {
        let config = mouse_config();
        assert_eq!(SystemMenu::new(&config).on_page(Page::Quick).screen, Screen::Quick);
        assert!(matches!(SystemMenu::new(&config).on_page(Page::EditControls).screen, Screen::Editor(_)));
        assert_eq!(SystemMenu::new(&config).on_page(Page::Main).screen, Screen::Main);
    }

    #[test]
    fn b_on_a_page_opened_from_the_guide_goes_back_to_the_guide() {
        let mut config = mouse_config();
        let mut quick = SystemMenu::new(&config).on_page(Page::Quick);
        assert_eq!(quick.handle_now(press(Button::East), &mut config), Some(Outcome::ReturnToGuide));
        let mut editor = SystemMenu::new(&config).on_page(Page::EditControls);
        assert_eq!(editor.handle_now(press(Button::East), &mut config), Some(Outcome::ReturnToGuide));
    }

    #[test]
    fn main_menu_opens_quick_settings_first_and_edit_controls_last() {
        let mut config = mouse_config();
        let mut menu = SystemMenu::new(&config);
        assert_eq!(menu.handle_now(press(Button::South), &mut config), None);
        assert_eq!(menu.screen, Screen::Quick);
        assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
        assert_eq!(menu.screen, Screen::Main);
        assert_eq!(menu.handle_now(press(Button::DpadDown), &mut config), None);
        assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::EditControls));
        // B leaves the editor for the main page, and B again closes the menu.
        assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
        assert_eq!(menu.handle_now(press(Button::East), &mut config), Some(Outcome::Close));
    }

    #[test]
    fn quick_settings_only_lists_what_the_profile_uses() {
        let mut config = mouse_config();
        config.general.profiles.truncate(1);
        assert_eq!(quick::rows(&config, &config), [Row::StickSpeed(Stick::Left), Row::InvertY, Row::Deadzone(Stick::Left), Row::Deadzone(Stick::Right), Row::Rumble, Row::OverlaySize, Row::NintendoLayout, Row::Remapping]);
        // A second profile adds the profile switch.
        config.general.profiles.push(config.general.profiles[0].clone());
        config.general.profiles[1].name = "Other".into();
        assert_eq!(quick::rows(&config, &config).last(), Some(&Row::Remapping));
        assert!(quick::rows(&config, &config).contains(&Row::Profile));
    }

    #[test]
    fn quick_nintendo_layout_sets_the_active_game_only() {
        let mut config = mouse_config();
        assert!(!config.active_nintendo_layout());
        // Picking the row turns it on for the game the menu is over, and leaves Settings alone.
        assert_eq!(quick::adjust(Row::NintendoLayout, 1, false, &mut config, &Config::default()), Some(Outcome::Changed));
        assert!(config.active_nintendo_layout());
        assert_eq!(config.general.nintendo_layout, Some(true));
        assert!(!config.nintendo_layout);
        assert_eq!(quick::label(Row::NintendoLayout, &config, &config), "Nintendo button layout: on");
        assert_eq!(quick::adjust(Row::NintendoLayout, 1, false, &mut config, &Config::default()), Some(Outcome::Changed));
        assert_eq!(config.general.nintendo_layout, Some(false));
    }

    #[test]
    fn closing_with_unsaved_changes_asks_first() {
        let mut config = mouse_config();
        let saved = config.clone();
        let mut menu = SystemMenu::new(&saved);
        // Change the stick speed: now the config differs from what was saved.
        menu.handle_now(press(Button::South), &mut config);
        menu.handle_now(press(Button::DpadRight), &mut config);
        menu.handle_now(press(Button::East), &mut config);
        assert_eq!(menu.screen, Screen::Main);
        assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
        assert!(matches!(menu.screen, Screen::Confirm(..)));
        // Keep editing goes back to the main page.
        menu.handle_now(press(Button::DpadDown), &mut config);
        menu.handle_now(press(Button::DpadDown), &mut config);
        assert_eq!(menu.handle_now(press(Button::South), &mut config), None);
        assert_eq!(menu.screen, Screen::Main);
        // Closing again asks again; discard hands back what was saved.
        menu.handle_now(press(Button::East), &mut config);
        menu.handle_now(press(Button::DpadDown), &mut config);
        assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Discard(Box::new(saved.clone()))));
    }

    #[test]
    fn keep_editing_goes_back_to_the_page_that_was_left() {
        let mut config = mouse_config();
        let saved = config.clone();
        let mut menu = SystemMenu::new(&saved).on_page(Page::Quick);
        assert_eq!(menu.view(&config).crumbs, ["Guide"], "opened from the Guide overlay");
        menu.handle_now(press(Button::DpadDown), &mut config);
        menu.handle_now(press(Button::South), &mut config);
        assert_eq!(menu.handle_now(press(Button::East), &mut config), None, "unsaved, so it asks");
        assert!(matches!(menu.screen, Screen::Confirm(..)));
        assert_eq!(menu.view(&config).crumbs, ["Guide"]);
        menu.handle_now(press(Button::East), &mut config);
        assert_eq!(menu.screen, Screen::Quick, "B keeps editing on Quick Settings");
        assert_eq!(menu.cursor, 1, "on the row it was on");
    }

    #[test]
    fn a_profile_switch_keeps_the_cursor_on_the_profile_row() {
        let mut config = mouse_config();
        config.general.profiles.truncate(1);
        let mut other = config.general.profiles[0].clone();
        other.name = "Other".into();
        other.left_stick.action = StickAction::Disabled;
        config.general.profiles.push(other);
        let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
        menu.cursor = quick::rows(&config, &config).iter().position(|r| *r == Row::Profile).unwrap();
        let Some(Outcome::Switch(target)) = menu.handle_now(press(Button::South), &mut config) else { panic!("no switch") };
        config.active = target;
        menu.profile_switched(&config);
        assert_eq!(quick::rows(&config, &config)[menu.cursor], Row::Profile, "fewer rows now, still on Profile");
    }

    #[test]
    fn guide_closes_the_menu_but_asks_about_unsaved_changes_first() {
        let mut config = mouse_config();
        let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
        assert_eq!(menu.guide_pressed(&config), Some(Outcome::Close), "closes, not back to the Guide overlay");
        menu.handle_now(press(Button::DpadRight), &mut config);
        assert_eq!(menu.guide_pressed(&config), None);
        assert!(matches!(menu.screen, Screen::Confirm(..)));
        assert_eq!(menu.guide_pressed(&config), None, "the question needs an answer");
    }

    #[test]
    fn stick_speed_steps_up_and_down() {
        let mut config = mouse_config();
        let mut menu = SystemMenu::new(&config);
        menu.handle_now(press(Button::South), &mut config);
        assert_eq!(menu.handle_now(press(Button::DpadRight), &mut config), Some(Outcome::Changed));
        assert!((speed(&config) - 1100.0).abs() < 0.01);
        assert_eq!(menu.handle_now(press(Button::DpadLeft), &mut config), Some(Outcome::Changed));
        assert!((speed(&config) - 1000.0).abs() < 0.01);
    }

    #[test]
    fn holding_right_keeps_raising_the_speed() {
        let mut config = mouse_config();
        let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
        let t0 = Instant::now();
        assert_eq!(menu.handle(press(Button::DpadRight), t0, &mut config), Some(Outcome::Changed));
        assert_eq!(menu.tick(t0, &mut config), None, "not before the delay");
        let delay = crate::nav::REPEAT_DELAY;
        assert_eq!(menu.tick(t0 + delay, &mut config), Some(Outcome::Changed));
        assert!((speed(&config) - 1210.0).abs() < 0.1, "{}", speed(&config));
        menu.handle(InputEvent::Button(Button::DpadRight, false), t0 + delay, &mut config);
        assert_eq!(menu.next_deadline(), None);
    }

    #[test]
    fn rb_makes_big_steps_and_a_changed_number_says_what_it_was() {
        let mut config = mouse_config();
        let saved = config.clone();
        let mut menu = SystemMenu::new(&saved).on_page(Page::Quick);
        menu.handle_now(press(Button::RightBumper), &mut config);
        menu.handle_now(press(Button::DpadRight), &mut config);
        assert!((speed(&config) - 1500.0).abs() < 0.1, "{}", speed(&config));
        assert_eq!(menu.view(&config).items[0].label, "Left stick speed: 1500 px/s (was 1000 px/s)");
        menu.handle_now(InputEvent::Button(Button::RightBumper, false), &mut config);
        menu.handle_now(press(Button::DpadRight), &mut config);
        assert!((speed(&config) - 1650.0).abs() < 0.1);
    }

    #[test]
    fn invert_y_flips_every_flag_at_once() {
        let mut config = mouse_config();
        let mut menu = SystemMenu::new(&config);
        menu.handle_now(press(Button::South), &mut config);
        menu.handle_now(press(Button::DpadDown), &mut config);
        assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Changed));
        let profile = config.active().unwrap();
        assert!(quick::inverted_y(profile));
        assert!(matches!(profile.left_stick.action, StickAction::Mouse { invert_y: true, .. }));
        assert!(profile.gyro.invert_y);
        menu.handle_now(press(Button::South), &mut config);
        assert!(!quick::inverted_y(config.active().unwrap()));
    }

    #[test]
    fn stick_push_steps_once_until_let_go() {
        let mut config = mouse_config();
        let mut menu = SystemMenu::new(&config);
        menu.handle_now(InputEvent::Axis(Axis::LeftY, 0.9), &mut config);
        assert_eq!(menu.cursor, 1);
        menu.handle_now(InputEvent::Axis(Axis::LeftY, 1.0), &mut config);
        assert_eq!(menu.cursor, 1);
        menu.handle_now(InputEvent::Axis(Axis::LeftY, 0.0), &mut config);
        menu.handle_now(InputEvent::Axis(Axis::LeftY, -0.9), &mut config);
        assert_eq!(menu.cursor, 0);
    }
}
