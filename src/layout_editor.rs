//! Edit Controls, the in-game editor of the Guide + Start menu: everything about the active
//! profile and its game, changed with the controller. It works on the active profile only, so it
//! is the pack of the game being played (the daemon makes one first when there isn't one).
//!
//! Every page is a list of rows ([`rows::Row`]) built from the config each time; a row says what
//! A and left/right do. `pages.rs` has the pages and their titles.

mod action;
mod action_page;
mod gyro;
mod infos;
mod inputs;
mod layers;
mod macros;
mod menus;
mod named;
mod owner;
mod pages;
mod rows;
mod sticks;
mod top;
mod touchpad;
mod triggers;

use pages::Page;

use crate::{
    config::{Button, Config, MenuKind},
    menu::{ItemView, MenuView},
};

use rows::{Choose, Nav, Row};

/// What the editor asks of the menu after a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditStep {
    /// Stay on the editor.
    Stay,
    /// Left the top page: back to the menu's main page.
    Leave,
    /// A setting changed in the config.
    Changed,
    /// Asks for text on the on-screen keyboard: what it's for, and the text to start from. The
    /// answer goes to [`LayoutEditor::set_text`].
    Text { title: String, text: String },
}

/// What a page is built from besides the config.
pub(crate) struct Cx<'a> {
    pub config: &'a Config,
    /// The buttons a profile sets: every button but Guide, and the paddles when the controller
    /// has them.
    pub inputs: Vec<Button>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutEditor {
    page: Page,
    cursor: usize,
    /// The page and cursor each open page was entered from, innermost last.
    history: Vec<(Page, usize)>,
    /// The controller has back paddles, so they can be set too.
    paddles: bool,
    /// A delete row was chosen once: choosing it again deletes. Any move disarms it.
    armed: bool,
}

impl Default for LayoutEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEditor {
    pub fn new() -> Self {
        LayoutEditor { page: Page::Top, cursor: 0, history: Vec::new(), paddles: false, armed: false }
    }

    /// This editor, for a controller with back paddles (or without).
    pub fn with_paddles(self, paddles: bool) -> Self {
        LayoutEditor { paddles, ..self }
    }

    fn cx<'a>(&self, config: &'a Config) -> Cx<'a> {
        let paddles = Button::PADDLES.iter().filter(|_| self.paddles);
        let inputs = Button::ALL.iter().filter(|&&b| b != Button::Guide).chain(paddles).copied().collect();
        Cx { config, inputs }
    }

    fn rows(&self, config: &Config) -> Vec<Row> {
        pages::rows(self.page, &self.cx(config))
    }

    /// How many rows the current page has.
    pub fn row_count(&self, config: &Config) -> usize {
        self.rows(config).len()
    }

    /// How many columns the page is shown in. A page with values to step stays in one column, so
    /// left and right can step them.
    fn columns(rows: &[Row]) -> usize {
        if rows.iter().any(|r| r.step.is_some()) { 1 } else { crate::menu::list_columns(rows.len()) }
    }

    /// Whether the current page is in two columns, so left and right move between them rather
    /// than changing a value.
    pub fn two_columns(&self, config: &Config) -> bool {
        Self::columns(&self.rows(config)) == 2
    }

    /// Moves the cursor on the current page: (x, y), y positive down. Pages in two columns move
    /// between them with x.
    pub fn move_cursor(&mut self, step: (i32, i32), config: &Config) {
        self.armed = false;
        let rows = self.rows(config);
        self.cursor = crate::menu::list_step_in(self.cursor.min(rows.len().saturating_sub(1)), rows.len(), Self::columns(&rows), step);
    }

    /// Moves the cursor a page of rows up or down (`dir` -1 or 1), stopping at the ends.
    pub fn jump_page(&mut self, dir: i32, config: &Config) {
        self.armed = false;
        let last = self.row_count(config).saturating_sub(1) as i32;
        self.cursor = (self.cursor as i32 + dir * crate::menu::VISIBLE_ROWS as i32).clamp(0, last) as usize;
    }

    /// Chooses the highlighted row.
    pub fn choose(&mut self, config: &mut Config) -> EditStep {
        let mut rows = self.rows(config);
        if rows.is_empty() {
            return EditStep::Stay;
        }
        self.cursor = self.cursor.min(rows.len() - 1);
        let row = rows.swap_remove(self.cursor);
        let armed = std::mem::take(&mut self.armed);
        match row.choose {
            Choose::Nothing => match row.step {
                Some(step) => EditStep::changed(step(config, 1)),
                None => EditStep::Stay,
            },
            Choose::Open(page) => {
                self.push(page);
                EditStep::Stay
            }
            Choose::Apply(apply) => {
                let act = apply(config);
                self.go(act.nav);
                EditStep::changed(act.changed)
            }
            // Deleting takes a second press, so a stray A doesn't lose anything.
            Choose::Delete(_) if !armed => {
                self.armed = true;
                EditStep::Stay
            }
            Choose::Delete(delete) => {
                let deleted = delete(config);
                if deleted {
                    self.go(Nav::Back(1));
                }
                EditStep::changed(deleted)
            }
            Choose::Text { title, text, .. } => EditStep::Text { title, text },
        }
    }

    /// The text asked for by the highlighted row ([`EditStep::Text`]) was typed: sets it.
    pub fn set_text(&mut self, config: &mut Config, typed: &str) -> EditStep {
        let mut rows = self.rows(config);
        if self.cursor >= rows.len() {
            return EditStep::Stay;
        }
        match rows.swap_remove(self.cursor).choose {
            Choose::Text { set, .. } => EditStep::changed(set(config, typed)),
            _ => EditStep::Stay,
        }
    }

    /// Changes the highlighted value by one step (`dir` is -1 or 1). Rows without one do nothing.
    pub fn adjust(&mut self, dir: i32, config: &mut Config) -> EditStep {
        self.armed = false;
        let mut rows = self.rows(config);
        if self.cursor >= rows.len() {
            return EditStep::Stay;
        }
        match rows.swap_remove(self.cursor).step {
            Some(step) => EditStep::changed(step(config, dir)),
            None => EditStep::Stay,
        }
    }

    /// Goes back one page. From the top page it leaves the editor.
    pub fn back(&mut self) -> EditStep {
        self.armed = false;
        if self.page == Page::Top || !self.pop() {
            self.page = Page::Top;
            self.cursor = 0;
            return EditStep::Leave;
        }
        EditStep::Stay
    }

    fn go(&mut self, nav: Nav) {
        match nav {
            Nav::Stay => {}
            Nav::Open(page) => self.push(page),
            Nav::Back(n) => {
                for _ in 0..n {
                    self.pop();
                }
            }
            Nav::BackOpen(n, page) => {
                self.go(Nav::Back(n));
                self.push(page);
            }
            Nav::Follow(page, cursor) => {
                self.page = page;
                if let Some((_, parent)) = self.history.last_mut() {
                    *parent = cursor;
                }
            }
        }
    }

    fn push(&mut self, page: Page) {
        self.history.push((self.page, self.cursor));
        self.page = page;
        self.cursor = 0;
    }

    /// Returns to the page this one was opened from, with its cursor. False at the top.
    fn pop(&mut self) -> bool {
        match self.history.pop() {
            Some((page, cursor)) => {
                self.page = page;
                self.cursor = cursor;
                true
            }
            None => false,
        }
    }

    /// The page as the overlay draws it. `parents` are the menus the editor was opened from,
    /// outermost first; the editor's own pages before this one follow them as breadcrumbs.
    pub fn view(&self, config: &Config, parents: &[String]) -> MenuView {
        let rows = self.rows(config);
        let columns = Self::columns(&rows);
        let steps = rows.iter().any(|r| r.step.is_some());
        let cursor = self.cursor.min(rows.len().saturating_sub(1));
        let items: Vec<ItemView> = rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| {
                let mut item = row.item;
                if self.armed && i == cursor {
                    item.label = "Press {confirm} again to delete".into();
                }
                item
            })
            .collect();
        let hint = if steps { "{confirm} choose · {left} {right} change · {back} back" } else { "{confirm} choose · {back} back" };
        let crumbs = parents.iter().cloned().chain(self.history.iter().map(|&(page, _)| pages::title(page, config))).collect();
        MenuView {
            title: pages::title(self.page, config),
            kind: MenuKind::List,
            items,
            selected: Some(cursor),
            picks: 0,
            crumbs,
            hint: hint.into(),
            style: config.active_menu_style().clone(),
            touch: false,
            aim: None,
            prompt: None,
            one_column: columns == 1,
        }
    }
}

impl EditStep {
    fn changed(changed: bool) -> Self {
        if changed { EditStep::Changed } else { EditStep::Stay }
    }
}

#[cfg(test)]
mod tests;
