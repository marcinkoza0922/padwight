//! What every Edit Controls page is made of: rows, each with its label and what A and left/right
//! do to the config. A page builds its rows from the config each time it is drawn or used, so
//! counting, drawing, choosing and stepping all read the same list.

use crate::{
    config::{Button, Config, Keyword},
    menu::{ItemView, Tone},
};

use super::Page;

/// What A does on a row (and left/right, through [`Row::step`]) to the config.
pub(super) type Apply = Box<dyn Fn(&mut Config) -> Act>;
/// One step of a value, `dir` being -1 or 1. True when it changed.
pub(super) type Stepper = Box<dyn Fn(&mut Config, i32) -> bool>;
/// Sets typed text. True when it changed.
pub(super) type SetText = Box<dyn Fn(&mut Config, &str) -> bool>;

/// What choosing a row does.
pub(super) enum Choose {
    /// Nothing: a line that only says something.
    Nothing,
    Open(Page),
    Apply(Apply),
    /// Deletes something. It takes a second press; then it runs and the page goes back.
    Delete(Box<dyn Fn(&mut Config) -> bool>),
    /// Asks for text on the on-screen keyboard, starting from `text`, then sets it.
    Text { title: String, text: String, set: SetText },
}

/// After a choice: whether the config changed, and where the editor goes.
pub(super) struct Act {
    pub changed: bool,
    pub nav: Nav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Nav {
    Stay,
    Open(Page),
    /// Back this many pages.
    Back(usize),
    /// Back this many pages, then open a page from there.
    BackOpen(usize, Page),
    /// The page is now this one (an item moved), and the page it came from has its cursor here.
    Follow(Page, usize),
}

impl Act {
    pub fn stay() -> Self {
        Act { changed: false, nav: Nav::Stay }
    }

    pub fn changed() -> Self {
        Act { changed: true, nav: Nav::Stay }
    }

    pub fn changed_if(changed: bool) -> Self {
        Act { changed, nav: Nav::Stay }
    }

    pub fn then(self, nav: Nav) -> Self {
        Act { nav, ..self }
    }
}

pub(super) struct Row {
    pub item: ItemView,
    pub choose: Choose,
    /// Left and right change the row's value. Without a choice of its own, A steps it forward.
    pub step: Option<Stepper>,
}

impl Row {
    /// A row that only says `label`, drawn muted; the cursor passes it by.
    pub fn note(label: impl Into<String>) -> Self {
        Row { item: ItemView { tone: Tone::Note, ..item(label.into()) }, choose: Choose::Nothing, step: None }
    }

    /// Whether the cursor can land on it: anything but a note.
    pub fn stops(&self) -> bool {
        self.item.tone != Tone::Note
    }

    /// A row that opens `page`.
    pub fn open(label: impl Into<String>, page: Page) -> Self {
        Row { item: ItemView { submenu: true, ..item(label.into()) }, choose: Choose::Open(page), step: None }
    }

    /// A row whose A does `apply`.
    pub fn apply(label: impl Into<String>, apply: impl Fn(&mut Config) -> Act + 'static) -> Self {
        Row { item: item(label.into()), choose: Choose::Apply(Box::new(apply)), step: None }
    }

    /// A row that adds something (tinted as adding).
    pub fn add(label: impl Into<String>, apply: impl Fn(&mut Config) -> Act + 'static) -> Self {
        Row::apply(label, apply).tone(Tone::Add)
    }

    /// A row that deletes something, after a second press, and goes back a page.
    pub fn delete(label: impl Into<String>, delete: impl Fn(&mut Config) -> bool + 'static) -> Self {
        Row { item: ItemView { tone: Tone::Remove, ..item(label.into()) }, choose: Choose::Delete(Box::new(delete)), step: None }
    }

    /// A row whose value left and right step; A steps it forward.
    pub fn step(label: impl Into<String>, step: impl Fn(&mut Config, i32) -> bool + 'static) -> Self {
        Row { item: item(label.into()), choose: Choose::Nothing, step: Some(Box::new(step)) }
    }

    /// A row that asks for text, titled `title`, starting from `text`.
    pub fn text(label: impl Into<String>, title: impl Into<String>, text: impl Into<String>, set: impl Fn(&mut Config, &str) -> bool + 'static) -> Self {
        let choose = Choose::Text { title: title.into(), text: text.into(), set: Box::new(set) };
        Row { item: item(label.into()), choose, step: None }
    }

    /// A switch: A and left/right flip it. `set` takes the new state and says whether it changed.
    pub fn toggle(label: &str, on: bool, set: impl Fn(&mut Config, bool) -> bool + 'static) -> Self {
        Row::step(format!("{label}: {}", on_off(on)), move |c, _| set(c, !on))
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.item.tone = tone;
        self
    }

    pub fn buttons(mut self, buttons: Vec<Button>) -> Self {
        self.item.buttons = buttons;
        self
    }

    pub fn keyword(mut self, keyword: Option<Keyword>) -> Self {
        self.item.keyword = keyword;
        self
    }
}

pub(super) fn item(label: String) -> ItemView {
    ItemView { label, ..ItemView::plain() }
}

/// A switch's value, worded as Quick Settings words its own.
pub(super) fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// A row's label with how many things it holds: "Macros (2)", or "Macros: none".
pub(super) fn counted(label: &str, n: usize) -> String {
    if n == 0 { format!("{label}: none") } else { format!("{label} ({n})") }
}

/// The value `options` has after `at`, `dir` steps on, round the end.
pub(super) fn cycle<T: Copy + PartialEq>(options: &[T], at: T, dir: i32) -> T {
    let i = options.iter().position(|o| *o == at).unwrap_or(0) as i32;
    options[(i + dir).rem_euclid(options.len() as i32) as usize]
}

/// `value` one step on: `step` added `dir` times, kept in `range`.
pub(super) fn nudge(value: f32, step: f32, dir: i32, (lo, hi): (f32, f32)) -> f32 {
    // Rounded to the step, so repeated steps don't drift.
    let next = ((value + step * dir as f32) / step).round() * step;
    next.max(lo).min(hi)
}

/// A number field: left and right step `value` by `step` within `range`, shown with `digits`
/// decimals and `unit`. `set` stores the new value; it says whether it changed.
pub(super) struct Number {
    pub step: f32,
    pub range: (f32, f32),
    pub digits: usize,
    pub unit: &'static str,
}

impl Number {
    pub const fn new(step: f32, range: (f32, f32), digits: usize, unit: &'static str) -> Self {
        Number { step, range, digits, unit }
    }

    pub fn row(&self, label: &str, value: f32, set: impl Fn(&mut Config, f32) -> bool + 'static) -> Row {
        let (step, range) = (self.step, self.range);
        Row::step(format!("{label}: {}", self.show(value)), move |c, dir| set(c, nudge(value, step, dir, range)))
    }

    /// A row for a whole number of milliseconds, or anything else counted in steps.
    pub fn row_u(&self, label: &str, value: u64, set: impl Fn(&mut Config, u64) -> bool + 'static) -> Row {
        self.row(label, value as f32, move |c, v| set(c, v.round() as u64))
    }

    pub fn show(&self, value: f32) -> String {
        format!("{value:.*}{}", self.digits, self.unit)
    }
}

/// Sets `*slot` to `value`, saying whether that changed it.
pub(super) fn set<T: PartialEq>(slot: &mut T, value: T) -> bool {
    if *slot == value {
        return false;
    }
    *slot = value;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_range_and_on_the_step() {
        assert_eq!(nudge(0.48, 0.05, 1, (0.0, 1.0)), 0.55);
        assert_eq!(nudge(0.98, 0.05, 1, (0.0, 1.0)), 1.0);
        assert_eq!(nudge(0.0, 0.05, -1, (0.0, 1.0)), 0.0);
        assert_eq!(cycle(&[1, 2, 3], 3, 1), 1);
        assert_eq!(cycle(&[1, 2, 3], 1, -1), 3);
    }
}
