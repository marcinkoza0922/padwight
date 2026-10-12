//! On-screen action menus: how controller input moves through them. The daemon routes all
//! controller input here while a menu is open; the overlay process draws `MenuView`.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::{Button, ButtonAction, CarouselControls, Keyword, Menu, MenuKind, OverlayStyle, Span, Stick, Trigger},
    engine::Opener,
    input::{Axis, InputEvent},
};

/// Stick deflection that aims a radial menu.
const STICK_PRESS: f32 = crate::nav::STICK_PRESS;
/// A menu nothing holds up (see [`Mode::Unwatched`]) closes after this long without input, so
/// it can't keep the controller forever.
const IDLE_CLOSE: Duration = Duration::from_secs(30);
/// Holding the close button in a submenu this long closes the whole menu, rather than going back.
const HOLD_TO_CLOSE: Duration = crate::overlay::HOLD_TO_CLOSE;
/// Stick deflection from which a radial menu shows where the stick points.
const AIM_SHOWN: f32 = 0.3;
/// A trigger opener counts as let go below this.
const TRIGGER_RELEASE: f32 = 0.3;
/// Trigger pull that steps a trigger-driven carousel.
const TRIGGER_PRESS: f32 = 0.6;

/// What the overlay draws for an open menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuView {
    pub title: String,
    pub kind: MenuKind,
    pub items: Vec<ItemView>,
    pub selected: Option<usize>,
    /// How many items have been chosen in this menu's session; changes on each pick.
    #[serde(default)]
    pub picks: u32,
    /// The menus this one was opened from, outermost first. Shown above the title.
    #[serde(default)]
    pub crumbs: Vec<String>,
    /// Short reminder of the controls.
    pub hint: String,
    #[serde(default)]
    pub style: OverlayStyle,
    /// Laid out on a touchpad: the items stay where the pad has them, so a carousel is drawn as
    /// a fixed strip rather than turning under the finger.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub touch: bool,
    /// Where a radial menu's stick points, in degrees clockwise from up (rounded to a few), or
    /// `None` while it rests in the middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aim: Option<f32>,
    /// A question asked over the menu, drawn on top of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Box<crate::offer::OfferView>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemView {
    pub label: String,
    /// Button that picks it directly (quick-select, or the cascade slot).
    pub button: Option<String>,
    /// Opens another menu rather than acting.
    pub submenu: bool,
    /// Buttons drawn as their controller glyphs, in front of the label.
    #[serde(default)]
    pub buttons: Vec<Button>,
    /// What kind of row it is, for its background.
    #[serde(default)]
    pub tone: Tone,
    /// The keyword the row's action starts with (Macro, Turbo…), drawn as its icon in front of the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<Keyword>,
    /// The share of a radial menu's circle its arc takes.
    #[serde(default = "crate::config::default_weight")]
    pub weight: f32,
    /// The cells of a grid it covers.
    #[serde(default, skip_serializing_if = "Span::is_one")]
    pub span: Span,
}

impl ItemView {
    /// A row with just a label to fill in: no button, glyphs, icon or tint.
    pub fn plain() -> Self {
        ItemView { label: String::new(), button: None, submenu: false, buttons: Vec::new(), tone: Tone::Normal, keyword: None, weight: 1.0, span: Span::ONE }
    }
}

/// What kind of row an item is. The overlay tints each kind differently, so a row that adds to
/// something or removes it stands out from the things it acts on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    #[default]
    Normal,
    /// Adds something: a new item, a new step.
    Add,
    /// Removes something, or deletes it.
    Remove,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuOutcome {
    /// Run this item's action; `close` says whether the menu goes away too (with all its
    /// submenus), as it does unless it's set to stay open.
    Choose { menu: String, item: usize, action: ButtonAction, close: bool },
    Close,
}

/// What keeps a menu on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Up while the opener is held; letting go closes it (a radial menu chooses first).
    Held,
    /// A list with a close button, once its opener is let go: up until the opener is pressed
    /// again or the close button closes it.
    Latched,
    /// Opened by something that can't be held (the command line, an analog zone): up until
    /// an item is chosen or the menu's back button closes it.
    Unwatched,
}

/// A stick-direction opener counts as let go once the stick is back within this.
const DIRECTION_RELEASE: f32 = 0.25;

/// One step through a list, grid or carousel: (x, y), each -1, 0 or 1; y is positive down.
type Step = (i32, i32);

struct Frame {
    menu: usize,
    cursor: Option<usize>,
}

/// One open menu (and any submenus opened from it, which close with it).
pub struct MenuSession {
    stack: Vec<Frame>,
    opener: Opener,
    mode: Mode,
    /// Whether the opener was down after the last event.
    opener_down: bool,
    /// A latched menu closes on the opener's next press, once it has been let go.
    armed: bool,
    /// When the close button went down in a submenu: let go, it goes back; held, it closes.
    closing: Option<Instant>,
    axes: HashMap<Axis, f32>,
    held: HashSet<Button>,
    /// The direction stepping through a list, grid or carousel, repeating while held.
    stepping: crate::nav::Repeat,
    /// Which step direction the stick currently holds.
    stick_step: crate::nav::StickDir,
    /// Which carousel trigger is currently pulled past the step point.
    trigger_step: Option<Trigger>,
    /// How many items have been chosen so far, shown to the overlay so it can animate the pick.
    picks: u32,
    /// The last input, for closing an unwatched menu left alone.
    last_input: Instant,
}

fn find(menus: &[Menu], name: &str) -> Option<usize> {
    menus.iter().position(|m| m.name == name)
}

/// A label for a button in hints and quick-select badges.
pub fn button_badge(b: Button) -> &'static str {
    match b {
        Button::South => "A",
        Button::East => "B",
        Button::North => "Y",
        Button::West => "X",
        Button::LeftBumper => "LB",
        Button::RightBumper => "RB",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::Guide => "Guide",
        Button::LeftStick => "L3",
        Button::RightStick => "R3",
        Button::DpadUp => "D↑",
        Button::DpadDown => "D↓",
        Button::DpadLeft => "D←",
        Button::DpadRight => "D→",
        Button::LeftStickUp => "LS↑",
        Button::LeftStickDown => "LS↓",
        Button::LeftStickLeft => "LS←",
        Button::LeftStickRight => "LS→",
        Button::RightStickUp => "RS↑",
        Button::RightStickDown => "RS↓",
        Button::RightStickLeft => "RS←",
        Button::RightStickRight => "RS→",
        Button::LeftPaddle => "LP",
        Button::RightPaddle => "RP",
        Button::LeftPaddle2 => "LP2",
        Button::RightPaddle2 => "RP2",
    }
}

impl MenuSession {
    /// Opens the menu named `name`, or `None` if there is no such menu or it is empty.
    pub fn open(menus: &[Menu], name: &str, opener: Opener) -> Option<Self> {
        let menu = find(menus, name)?;
        if menus[menu].items.is_empty() {
            return None;
        }
        let watchable = !opener.buttons.is_empty() || opener.trigger.is_some();
        let mode = if watchable { Mode::Held } else { Mode::Unwatched };
        let mut session = MenuSession {
            stack: Vec::new(),
            opener,
            mode,
            // Assumed held, as it just opened the menu; `prime` checks.
            opener_down: true,
            armed: false,
            closing: None,
            axes: HashMap::new(),
            held: HashSet::new(),
            stepping: crate::nav::Repeat::default(),
            stick_step: crate::nav::StickDir::default(),
            trigger_step: None,
            picks: 0,
            last_input: Instant::now(),
        };
        // Until `prime` says otherwise, the opener is taken to be held (it just fired).
        for b in session.opener.buttons.clone() {
            match b.stick_direction() {
                Some((stick, (dx, dy))) => {
                    let (ax, ay) = stick_axes(stick);
                    session.axes.insert(ax, dx);
                    session.axes.insert(ay, dy);
                }
                None => {
                    session.held.insert(b);
                }
            }
        }
        if let Some(t) = session.opener.trigger {
            session.axes.insert(trigger_axis(t), 1.0);
        }
        session.push(menus, menu);
        Some(session)
    }

    /// Tells the session what the controller is holding as the menu opens. Returns false if
    /// the opener is already let go (e.g. a quick tap), so the menu shouldn't show at all.
    #[must_use]
    pub fn prime(&mut self, menus: &[Menu], buttons: impl IntoIterator<Item = Button>, axes: impl IntoIterator<Item = (Axis, f32)>) -> bool {
        self.held = buttons.into_iter().collect();
        self.axes = axes.into_iter().collect();
        self.opener_down = self.opener_is_down();
        if self.mode == Mode::Held && !self.opener_down {
            // A quick tap opens a list that stays up; anything else it can't hold up.
            if !self.top(menus).is_some_and(stays_open) {
                return false;
            }
            self.latch();
        }
        if let Some(menu) = self.top(menus).cloned()
            && let MenuKind::Radial { stick, .. } = menu.kind
        {
            self.aim(&menu, stick);
        }
        true
    }

    fn opener_is_down(&self) -> bool {
        let buttons = self.opener.buttons.iter().all(|b| match b.stick_direction() {
            Some((stick, (dx, dy))) => {
                let (ax, ay) = stick_axes(stick);
                let (x, y) = (self.axis(ax), self.axis(ay));
                x * dx + y * dy >= DIRECTION_RELEASE
            }
            None => self.held.contains(b),
        });
        let trigger = self.opener.trigger.is_none_or(|t| self.axis(trigger_axis(t)) >= TRIGGER_RELEASE);
        buttons && trigger
    }

    fn axis(&self, axis: Axis) -> f32 {
        self.axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// The input that opened the menu, for hints.
    fn opener_label(&self) -> String {
        let mut parts: Vec<String> = self.opener.buttons.iter().map(|b| crate::info::button_hint(*b)).collect();
        if let Some(t) = self.opener.trigger {
            parts.push(if t == Trigger::Left { "{lt}" } else { "{rt}" }.into());
        }
        parts.join(" + ")
    }

    fn push(&mut self, menus: &[Menu], menu: usize) {
        let cursor = match menus[menu].kind {
            // A button menu has no cursor: each item is chosen by its own button.
            MenuKind::Radial { .. } | MenuKind::Directional { .. } | MenuKind::Buttons => None,
            MenuKind::List | MenuKind::Carousel { .. } => Some(0),
            // A grid starts where it's set to, or on the item merged over that cell.
            MenuKind::Grid { .. } => GridLayout::of(&menus[menu]).map(|grid| grid.owner(menus[menu].start)),
        };
        self.stack.push(Frame { menu, cursor });
        self.stepping.stop();
        self.closing = None;
    }

    fn top<'m>(&self, menus: &'m [Menu]) -> Option<&'m Menu> {
        self.stack.last().and_then(|f| menus.get(f.menu))
    }

    pub fn view(&self, menus: &[Menu]) -> Option<MenuView> {
        let frame = self.stack.last()?;
        let menu = menus.get(frame.menu)?;
        let items = item_views(menu);
        let radial = matches!(menu.kind, MenuKind::Radial { .. });
        let close = self.close_hint(menu, radial);
        let hint = match menu.kind {
            MenuKind::Radial { stick: Stick::Left, .. } => format!("Aim {{ls}} · {close}"),
            MenuKind::Radial { stick: Stick::Right, .. } => format!("Aim {{rs}} · {close}"),
            MenuKind::Directional { cluster } => format!("Press a {} direction · {close}", cluster.to_string().to_lowercase()),
            MenuKind::List if pages(menu) => format!("{{dpad}} move · {{lb}} {{rb}} page · {{south}} choose · {close}"),
            MenuKind::List if list_columns(menu.items.len()) == 2 => format!("{{dpad}} move · {{south}} choose · {close}"),
            MenuKind::List => format!("{{up}} {{down}} move · {{south}} choose · {close}"),
            MenuKind::Buttons => format!("Press an item's button · {close}"),
            MenuKind::Carousel { controls } => format!("{} to cycle · {{south}} choose · {close}", carousel_hint(controls)),
            MenuKind::Grid { .. } => format!("{{dpad}} move · {{south}} choose · {close}"),
        };
        Some(MenuView {
            title: menu.name.clone(),
            kind: menu.kind,
            items,
            selected: frame.cursor,
            picks: self.picks,
            crumbs: self.stack[..self.stack.len() - 1].iter().filter_map(|f| menus.get(f.menu)).map(|m| m.name.clone()).collect(),
            hint,
            style: menu.style.clone(),
            touch: false,
            aim: self.live_aim(menu),
            prompt: None,
        })
    }

    /// How the menu goes away (and, for a radial menu, chooses), for its hint. The close button,
    /// when it has one, backs out of a submenu and closes the menu at the top (a radial menu
    /// without choosing).
    fn close_hint(&self, menu: &Menu, radial: bool) -> String {
        let deep = self.stack.len() > 1;
        let opener = self.opener_label();
        let Some(cancel) = menu.cancel_button().map(crate::info::button_hint) else {
            // Only a menu with a close button latches, so this one is only ever held or unwatched.
            return match (self.mode, radial) {
                (Mode::Held | Mode::Latched, true) => "let go to choose".into(),
                (Mode::Held | Mode::Latched, false) => "let go to close".into(),
                (Mode::Unwatched, _) => "{south} choose".into(),
            };
        };
        // A list that latches says so while it's still held, as letting go won't close it.
        let mode = if self.mode == Mode::Held && stays_open(menu) { Mode::Latched } else { self.mode };
        match (mode, radial) {
            (Mode::Held, true) => format!("let go to choose · {cancel} cancel"),
            (Mode::Held, false) if deep => format!("{cancel} back (hold to close) · let go to close"),
            (Mode::Held, false) => format!("let go or {cancel} to close"),
            (Mode::Latched, true) => format!("{opener} again to choose · {cancel} cancel"),
            (Mode::Latched, false) if deep => format!("{cancel} back (hold to close) · {opener} again to close"),
            (Mode::Latched, false) => format!("{opener} again or {cancel} to close"),
            (Mode::Unwatched, true) => format!("{{south}} choose · {cancel} close"),
            (Mode::Unwatched, false) if deep => format!("{cancel} back (hold to close)"),
            (Mode::Unwatched, false) => format!("{cancel} close"),
        }
    }

    /// Keeps the menu up after the opener is let go, until it's pressed again.
    fn latch(&mut self) {
        self.mode = Mode::Latched;
        self.armed = true;
    }

    /// The item the cursor is on in the menu on top, if it has one.
    pub fn cursor(&self) -> Option<usize> {
        self.stack.last().and_then(|f| f.cursor)
    }

    /// Handles controller input; returns what to do, if anything.
    pub fn handle(&mut self, menus: &[Menu], ev: InputEvent, now: Instant) -> Option<MenuOutcome> {
        let outcome = self.act(menus, ev, now);
        if matches!(outcome, Some(MenuOutcome::Choose { .. })) {
            self.picks += 1;
        }
        outcome
    }

    fn act(&mut self, menus: &[Menu], ev: InputEvent, now: Instant) -> Option<MenuOutcome> {
        self.last_input = now;
        let menu = self.top(menus)?.clone();
        let mut fresh_press = false;
        match ev {
            InputEvent::Button(b, true) => fresh_press = self.held.insert(b),
            InputEvent::Button(b, false) => {
                self.held.remove(&b);
                if let Some(dir) = crate::nav::dpad_dir(b) {
                    self.stepping.release(dir);
                }
            }
            InputEvent::Axis(axis, value) => {
                self.axes.insert(axis, value);
            }
            InputEvent::Touchpad(_) => {}
        }

        let down = self.opener_is_down();
        let was_down = std::mem::replace(&mut self.opener_down, down);
        match self.mode {
            Mode::Held if was_down && !down && stays_open(&menu) => self.latch(),
            Mode::Held if was_down && !down => return Some(self.finish(&menu)),
            Mode::Latched if !down => self.armed = true,
            Mode::Latched if self.armed && !was_down => return Some(self.finish(&menu)),
            _ => {}
        }

        match ev {
            // Letting go of the close button soon enough backs out of the submenu.
            InputEvent::Button(b, false) if self.closing.is_some() && Some(b) == menu.cancel_button() => {
                self.closing = None;
                self.back()
            }
            // The opener's own buttons only hold the menu up.
            InputEvent::Button(b, true) if fresh_press && !self.opener.buttons.contains(&b) => {
                self.button_pressed(menus, &menu, b, now)
            }
            InputEvent::Button(..) => None,
            InputEvent::Axis(axis, value) => self.axis_moved(&menu, axis, value, now),
            InputEvent::Touchpad(_) => None,
        }
    }

    /// The opener was let go (or pressed again, for a latched menu): a radial menu chooses
    /// what it's aimed at, and the menu closes with all its submenus.
    fn finish(&mut self, menu: &Menu) -> MenuOutcome {
        let aimed = self.stack.last().and_then(|f| f.cursor);
        if let MenuKind::Radial { .. } = menu.kind
            && let Some(i) = aimed
            && let Some(item) = menu.items.get(i)
            && !matches!(item.action, ButtonAction::Disabled | ButtonAction::OpenMenu(_))
        {
            return MenuOutcome::Choose { menu: menu.name.clone(), item: i, action: item.action.clone(), close: true };
        }
        MenuOutcome::Close
    }

    fn button_pressed(&mut self, menus: &[Menu], menu: &Menu, b: Button, now: Instant) -> Option<MenuOutcome> {
        // The cancel button backs out of a submenu, or closes the menu at the top. A menu that
        // can't be let go of always needs it; one that can be lets the menu use it first (a
        // button menu's item, a face-button slot).
        if Some(b) == menu.cancel_button() && (self.mode == Mode::Unwatched || !uses(menu, b, self.mode)) {
            // In a submenu it waits to see whether it's held (closing) or let go (going back).
            if self.stack.len() > 1 {
                self.closing = Some(now);
                return None;
            }
            return self.back();
        }
        match menu.kind {
            MenuKind::Directional { cluster } => {
                let slot = cluster.slots().iter().position(|s| *s == b)?;
                // The slot picked stays lit, so a menu that stays up shows what fired.
                if menu.items.get(slot).is_some_and(|item| item.action != ButtonAction::Disabled)
                    && let Some(frame) = self.stack.last_mut()
                {
                    frame.cursor = Some(slot);
                }
                self.choose(menus, slot)
            }
            MenuKind::Radial { .. } => {
                // With nothing to let go of, A chooses.
                (self.mode == Mode::Unwatched && b == Button::South).then(|| self.choose_cursor(menus)).flatten()
            }
            MenuKind::Buttons => {
                let i = menu.items.iter().position(|item| item.button == Some(b))?;
                self.choose(menus, i)
            }
            MenuKind::List | MenuKind::Grid { .. } => self.list_button(menus, menu, b, now),
            MenuKind::Carousel { controls } => {
                if let Some(step) = carousel_step(controls, b) {
                    self.start_stepping(menu, step, now);
                    return None;
                }
                (b == Button::South).then(|| self.choose_cursor(menus)).flatten()
            }
        }
    }

    /// The D-pad moves a list up and down (and between its columns once it has two), and a grid
    /// in all four directions; A chooses.
    fn list_button(&mut self, menus: &[Menu], menu: &Menu, b: Button, now: Instant) -> Option<MenuOutcome> {
        let sideways = moves_sideways(menu);
        match b {
            Button::DpadUp => self.start_stepping(menu, (0, -1), now),
            Button::DpadDown => self.start_stepping(menu, (0, 1), now),
            Button::DpadLeft if sideways => self.start_stepping(menu, (-1, 0), now),
            Button::DpadRight if sideways => self.start_stepping(menu, (1, 0), now),
            Button::LeftBumper | Button::RightBumper if pages(menu) => self.jump_page(menu, if b == Button::LeftBumper { -1 } else { 1 }),
            Button::South => return self.choose_cursor(menus),
            _ => {}
        }
        None
    }

    /// Moves a long list's cursor a page of rows up or down, stopping at the ends.
    fn jump_page(&mut self, menu: &Menu, dir: i32) {
        let last = menu.items.len().saturating_sub(1) as i32;
        if let Some(frame) = self.stack.last_mut() {
            let at = frame.cursor.unwrap_or(0) as i32;
            frame.cursor = Some((at + dir * VISIBLE_ROWS as i32).clamp(0, last) as usize);
        }
    }

    fn axis_moved(&mut self, menu: &Menu, axis: Axis, value: f32, now: Instant) -> Option<MenuOutcome> {
        // A trigger opener only holds the menu up.
        if self.opener.trigger.is_some_and(|t| axis == trigger_axis(t)) {
            return None;
        }
        match menu.kind {
            MenuKind::Radial { stick, .. } => {
                let (ax, ay) = stick_axes(stick);
                if axis == ax || axis == ay {
                    self.aim(menu, stick);
                }
                None
            }
            MenuKind::List if !moves_sideways(menu) => {
                if axis == Axis::LeftY {
                    self.stick_step(menu, (0.0, value), now);
                }
                None
            }
            MenuKind::List | MenuKind::Grid { .. } => {
                if matches!(axis, Axis::LeftX | Axis::LeftY) {
                    self.stick_step(menu, (self.axis(Axis::LeftX), self.axis(Axis::LeftY)), now);
                }
                None
            }
            MenuKind::Carousel { controls } => {
                match (controls, axis) {
                    (CarouselControls::LeftStick, Axis::LeftX) | (CarouselControls::RightStick, Axis::RightX) => {
                        self.stick_step(menu, (value, 0.0), now)
                    }
                    (CarouselControls::Triggers, Axis::LeftTrigger | Axis::RightTrigger) => {
                        let t = if axis == Axis::LeftTrigger { Trigger::Left } else { Trigger::Right };
                        if value >= TRIGGER_PRESS && self.trigger_step.is_none() {
                            self.trigger_step = Some(t);
                            self.start_stepping(menu, (if t == Trigger::Left { -1 } else { 1 }, 0), now);
                        } else if value < TRIGGER_RELEASE && self.trigger_step == Some(t) {
                            self.trigger_step = None;
                            self.stepping.stop();
                        }
                    }
                    _ => {}
                }
                None
            }
            MenuKind::Directional { .. } | MenuKind::Buttons => None,
        }
    }

    /// A stick used like a D-pad: up/down for lists, left/right for carousels, all four for
    /// grids (whichever way it leans most).
    fn stick_step(&mut self, menu: &Menu, pos: (f32, f32), now: Instant) {
        match self.stick_step.update(pos) {
            Some(Some(d)) => self.start_stepping(menu, d, now),
            Some(None) => self.stepping.stop(),
            None => {}
        }
    }

    /// Where a radial menu's stick points right now, for the overlay to show, in steps of a few
    /// degrees so small wobbles don't redraw it.
    fn live_aim(&self, menu: &Menu) -> Option<f32> {
        let MenuKind::Radial { stick, .. } = menu.kind else { return None };
        let (ax, ay) = stick_axes(stick);
        let (x, y) = (self.axis(ax), self.axis(ay));
        (x.hypot(y) >= AIM_SHOWN).then(|| (x.atan2(-y).to_degrees().rem_euclid(360.0) / 3.0).round() * 3.0)
    }

    /// Points a radial menu's selection where the stick is aimed (first item at the top,
    /// going clockwise). The last aim sticks if the stick springs back to center.
    fn aim(&mut self, menu: &Menu, stick: Stick) {
        let (ax, ay) = stick_axes(stick);
        let x = self.axes.get(&ax).copied().unwrap_or(0.0);
        let y = self.axes.get(&ay).copied().unwrap_or(0.0);
        if x.hypot(y) < STICK_PRESS {
            return;
        }
        // Clockwise from straight up; y is positive down.
        let angle = x.atan2(-y).to_degrees().rem_euclid(360.0);
        let weights: Vec<f32> = menu.items.iter().map(|item| item.weight).collect();
        let boxes = matches!(menu.kind, MenuKind::Radial { boxes: true, .. });
        if let Some(i) = crate::radial::index_at(&crate::radial::for_menu(boxes, &weights), angle)
            && let Some(frame) = self.stack.last_mut()
        {
            frame.cursor = Some(i);
        }
    }

    fn start_stepping(&mut self, menu: &Menu, dir: Step, now: Instant) {
        self.step(menu, dir);
        self.stepping.start(dir, now);
    }

    fn step(&mut self, menu: &Menu, (dx, dy): Step) {
        let n = menu.items.len();
        if let Some(frame) = self.stack.last_mut()
            && n > 0
        {
            let cursor = frame.cursor.unwrap_or(0).min(n - 1);
            frame.cursor = Some(match GridLayout::of(menu) {
                Some(grid) => grid.step(cursor, (dx, dy)),
                None if matches!(menu.kind, MenuKind::List | MenuKind::Buttons) => list_step(cursor, n, (dx, dy)),
                // Carousels are one line, whichever way it runs.
                None => (cursor as i32 + dx + dy).rem_euclid(n as i32) as usize,
            });
        }
    }

    fn choose_cursor(&mut self, menus: &[Menu]) -> Option<MenuOutcome> {
        let i = self.stack.last()?.cursor?;
        self.choose(menus, i)
    }

    /// Chooses item `i`: opens its submenu in its place, or hands back its action to run, which
    /// closes the menu unless it's set to stay open.
    fn choose(&mut self, menus: &[Menu], i: usize) -> Option<MenuOutcome> {
        let menu = self.top(menus)?;
        let item = menu.items.get(i)?;
        match &item.action {
            ButtonAction::Disabled => None,
            ButtonAction::OpenMenu(name) => {
                let sub = find(menus, name).filter(|m| can_open(menu, &menus[*m]))?;
                self.push(menus, sub);
                None
            }
            action => {
                // The menu shows the item it picked, as it goes or as it stays.
                if let Some(frame) = self.stack.last_mut() {
                    frame.cursor = Some(i);
                }
                Some(MenuOutcome::Choose { menu: menu.name.clone(), item: i, action: action.clone(), close: !menu.keep_open })
            }
        }
    }

    /// Cancel: back out of a submenu, or close.
    fn back(&mut self) -> Option<MenuOutcome> {
        self.stack.pop();
        self.stepping.stop();
        self.closing = None;
        if self.stack.is_empty() { Some(MenuOutcome::Close) } else { None }
    }

    /// List/carousel repeat while a direction is held. Returns whether the view changed.
    pub fn tick(&mut self, menus: &[Menu], now: Instant) -> bool {
        let Some(menu) = self.top(menus).cloned() else { return false };
        let Some(dir) = self.stepping.due(now) else { return false };
        self.step(&menu, dir);
        true
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        let idle = (self.mode == Mode::Unwatched).then_some(self.last_input + IDLE_CLOSE);
        let closing = self.closing.map(|t| t + HOLD_TO_CLOSE);
        self.stepping.deadline().into_iter().chain(idle).chain(closing).min()
    }

    /// Whether the menu closes by itself now: its close button has been held down in a submenu,
    /// or it's unwatched and has been left alone long enough.
    pub fn expired(&self, now: Instant) -> bool {
        self.closing.is_some_and(|t| now >= t + HOLD_TO_CLOSE) || (self.mode == Mode::Unwatched && now >= self.last_input + IDLE_CLOSE)
    }
}

/// The controls that cycle a carousel, as hint tokens.
fn carousel_hint(controls: CarouselControls) -> &'static str {
    match controls {
        CarouselControls::Bumpers => "{lb} {rb}",
        CarouselControls::Triggers => "{lt} {rt}",
        CarouselControls::DPad => "{left} {right}",
        CarouselControls::LeftStick => "{ls}",
        CarouselControls::RightStick => "{rs}",
    }
}

/// The step a carousel's stepping button makes, if `b` is one.
fn carousel_step(controls: CarouselControls, b: Button) -> Option<Step> {
    match (controls, b) {
        (CarouselControls::Bumpers, Button::LeftBumper) | (CarouselControls::DPad, Button::DpadLeft) => Some((-1, 0)),
        (CarouselControls::Bumpers, Button::RightBumper) | (CarouselControls::DPad, Button::DpadRight) => Some((1, 0)),
        _ => None,
    }
}

/// Whether a list stays up once its opener is let go: it does when it has a close button to put
/// it away.
fn stays_open(menu: &Menu) -> bool {
    menu.kind == MenuKind::List && menu.cancel_button().is_some()
}

/// Whether the bumpers jump a page: in a list too long to show at once.
fn pages(menu: &Menu) -> bool {
    matches!(menu.kind, MenuKind::List) && menu.items.len() > 2 * VISIBLE_ROWS
}

/// Whether left and right move the cursor: in a grid, and in a list long enough for two columns.
fn moves_sideways(menu: &Menu) -> bool {
    match menu.kind {
        MenuKind::Grid { .. } => true,
        MenuKind::List => list_columns(menu.items.len()) == 2,
        _ => false,
    }
}

/// Whether `menu` itself does something with a press of `b` (in `mode`), so the press isn't
/// free to cancel.
fn uses(menu: &Menu, b: Button, mode: Mode) -> bool {
    let dpad = matches!(b, Button::DpadUp | Button::DpadDown) || (moves_sideways(menu) && matches!(b, Button::DpadLeft | Button::DpadRight));
    match menu.kind {
        MenuKind::Directional { cluster } => cluster.slots().contains(&b),
        MenuKind::Radial { .. } => mode == Mode::Unwatched && b == Button::South,
        MenuKind::Buttons => menu.items.iter().any(|item| item.button == Some(b)),
        MenuKind::List | MenuKind::Grid { .. } => dpad || b == Button::South || (pages(menu) && matches!(b, Button::LeftBumper | Button::RightBumper)),
        MenuKind::Carousel { controls } => carousel_step(controls, b).is_some() || b == Button::South,
    }
}

/// What the overlay draws for each of a menu's items.
fn item_views(menu: &Menu) -> Vec<ItemView> {
    let slots = match menu.kind {
        MenuKind::Directional { cluster } => Some(cluster.slots()),
        _ => None,
    };
    menu.items
        .iter()
        .enumerate()
        .map(|(i, item)| ItemView {
            label: item.label.clone(),
            button: None,
            submenu: matches!(item.action, ButtonAction::OpenMenu(_)),
            weight: item.weight,
            // The slot's or item's button, drawn as the pad's glyph.
            buttons: match slots {
                Some(slots) => slots.get(i).copied().into_iter().collect(),
                None => item.button.into_iter().collect(),
            },
            tone: Tone::Normal,
            keyword: None,
            span: item.span,
        })
        .collect()
}

/// A list longer than this many rows is shown in two columns.
pub const ONE_COLUMN_ROWS: usize = 8;
/// Rows each column shows at once; a longer column scrolls with the cursor.
pub const VISIBLE_ROWS: usize = 8;

/// How many columns a list of `n` rows is shown in.
pub fn list_columns(n: usize) -> usize {
    if n > ONE_COLUMN_ROWS { 2 } else { 1 }
}

/// Moves the cursor through a list of `n` rows, as the overlay shows it. A two-column list
/// fills the left column first, then the right. Up and down follow that order, wrapping round the
/// whole list. Left and right move to the other column, on the same row or its last one.
pub fn list_step(cursor: usize, n: usize, (dx, dy): Step) -> usize {
    if n == 0 {
        return 0;
    }
    if list_columns(n) == 1 || dx == 0 {
        // Up and down go through the list in order, from the bottom of one column to the top of
        // the next and round again.
        return (cursor as i32 + dx + dy).rem_euclid(n as i32) as usize;
    }
    let half = n.div_ceil(2);
    let col = usize::from(cursor >= half);
    let row = cursor - col * half;
    let other_len = if col == 0 { n - half } else { half };
    (1 - col) * half + row.min(other_len - 1)
}

/// The first row a two-column list shows, so the cursor's row is on screen. Both columns share
/// it, so their rows line up. A one-column list always starts at the top.
pub fn list_start(cursor: usize, n: usize) -> usize {
    if list_columns(n) == 1 {
        return 0;
    }
    let half = n.div_ceil(2);
    let row = cursor.min(n - 1) - usize::from(cursor >= half) * half;
    if row >= VISIBLE_ROWS { row + 1 - VISIBLE_ROWS } else { 0 }
}

/// Whether `parent` may open `child` as a submenu: only when both have the same close button
/// (which backs out of the submenu), and never from a radial menu, which chooses as it's let go.
pub fn can_open(parent: &Menu, child: &Menu) -> bool {
    !matches!(parent.kind, MenuKind::Radial { .. })
        && parent.cancel_button().is_some()
        && parent.cancel_button() == child.cancel_button()
        && !child.items.is_empty()
}

fn stick_axes(s: Stick) -> (Axis, Axis) {
    match s {
        Stick::Left => (Axis::LeftX, Axis::LeftY),
        Stick::Right => (Axis::RightX, Axis::RightY),
    }
}

fn trigger_axis(t: Trigger) -> Axis {
    match t {
        Trigger::Left => Axis::LeftTrigger,
        Trigger::Right => Axis::RightTrigger,
    }
}

mod grid;
pub use grid::GridLayout;
mod touch;
pub use touch::{ItemShape, TOUCH_CENTER_RADIUS, item_at, item_shapes, touch_view};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod suite;
