//! The on-screen editor ("stage"): an overlay drawn full size in a see-through window over the
//! whole screen, right where the overlay process would put it, to move and resize with the mouse
//! (its edges snapping to a grid), restyle and fill from a panel beside it, and save from there.
//! Edits go through the same messages as the settings window's cards, so undo and Save & apply
//! work as they do there.

use iced::window;

use super::*;
use crate::{motion::Anim, overlay::fit::Fit};

mod grid;
mod handles;
mod inspector;
pub(super) mod snap;

/// The overlay on the stage. Menus, info and log overlays are the shown setup's, by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StageTarget {
    Menu(usize),
    Info(usize),
    Log(usize),
    Keyboard,
    Numpad,
    Media,
    InGameMenu,
}

impl StageTarget {
    /// Whether it belongs to a setup, rather than to the app.
    fn in_setup(self) -> bool {
        matches!(self, StageTarget::Menu(_) | StageTarget::Info(_) | StageTarget::Log(_))
    }
}

/// The inspector's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum StageTab {
    #[default]
    Layout,
    Look,
    Content,
}

/// Grid sizes to pick from, in pixels.
const GRID_STEPS: [f32; 6] = [8.0, 16.0, 24.0, 32.0, 48.0, 64.0];

/// The theme name the stage window is drawn in: the settings window's colors over a clear
/// background, so the screen shows through (see `App::window_style`).
const STAGE_THEME: &str = "Padwight stage";

/// The stage, while it's up.
pub(super) struct Stage {
    pub(super) window: window::Id,
    target: StageTarget,
    /// The setup it was opened on: a setup's overlays are its by index, so the stage closes when
    /// the settings window moves to another one.
    game: Option<String>,
    tab: StageTab,
    snap: bool,
    step: f32,
    show_grid: bool,
    dim: bool,
    dock_left: bool,
    folded: bool,
}

#[derive(Debug, Clone)]
pub(super) enum StageMessage {
    Open(StageTarget),
    Close,
    /// The window went away without being asked to (the compositor closed it).
    Closed(window::Id),
    Gesture(snap::Gesture),
    FitContents,
    Tab(StageTab),
    Snap(bool),
    Step(f32),
    ShowGrid(bool),
    Dim(bool),
    Dock,
    Fold,
    Key(window::Id, StageKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StageKey {
    Escape,
    Save,
}

/// The window: borderless, see-through, over everything and the whole screen, as the overlay is.
fn stage_window() -> window::Settings {
    window::Settings {
        fullscreen: true,
        transparent: true,
        decorations: false,
        level: window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        icon: crate::icon::window(),
        ..window::Settings::default()
    }
}

/// `base`'s colors under the stage's name, which `App::window_style` draws over nothing.
pub(super) fn stage_theme(base: &iced::Theme) -> iced::Theme {
    iced::Theme::custom(STAGE_THEME, base.palette())
}

pub(super) fn is_stage_theme(theme: &iced::Theme) -> bool {
    iced::theme::Base::name(theme) == STAGE_THEME
}

/// Esc closes the stage, Ctrl+S saves. Which window they came from is sorted out in `update_stage`.
pub(super) fn stage_keys(event: iced::Event, status: iced::event::Status, id: window::Id) -> Option<Message> {
    use iced::keyboard::{Event, Key, key::Named};
    let iced::Event::Keyboard(Event::KeyPressed { key, modifiers, .. }) = event else { return None };
    let key = match key {
        Key::Named(Named::Escape) if status == iced::event::Status::Ignored => StageKey::Escape,
        Key::Character(c) if modifiers.command() && c.eq_ignore_ascii_case("s") => StageKey::Save,
        _ => return None,
    };
    Some(Message::Stage(StageMessage::Key(id, key)))
}

/// The Appearance toggle of a card, with the button that opens the overlay on screen beside it.
pub(super) fn appearance_row<'a>(open: bool, toggle: Message, target: StageTarget) -> Element<'a, Message> {
    let edit = button(text("Edit on screen").size(13)).style(style::secondary).on_press(Message::Stage(StageMessage::Open(target)));
    let edit = tooltip(
        edit,
        container(text("Show it full size where it goes, over everything, to drag, resize and restyle it there.").size(13)).padding(8).style(style::tooltip),
        tooltip::Position::Top,
    );
    row![disclosure("Appearance", open, toggle), edit].spacing(12).align_y(Alignment::Center).into()
}

impl App {
    pub(super) fn update_stage(&mut self, message: StageMessage) -> Task<Message> {
        match message {
            StageMessage::Open(target) => return self.open_stage(target),
            StageMessage::Close => return self.close_stage(),
            StageMessage::Closed(id) => {
                if self.stage.as_ref().is_some_and(|s| s.window == id) {
                    self.stage = None;
                }
            }
            StageMessage::Key(id, key) => {
                if self.stage.as_ref().is_none_or(|s| s.window != id) {
                    return Task::none();
                }
                return match key {
                    StageKey::Save => Task::done(Message::Save),
                    StageKey::Escape if self.popup().is_none() => self.close_stage(),
                    StageKey::Escape => Task::none(),
                };
            }
            StageMessage::Gesture(g) => {
                let Some(stage) = &self.stage else { return Task::none() };
                let (target, grid) = (stage.target, stage.snap.then_some(stage.step));
                if let Some(style) = self.stage_style(target) {
                    let moved = snap::apply(style, &g, grid);
                    return self.restyle(target, moved);
                }
            }
            StageMessage::FitContents => {
                let Some(target) = self.stage.as_ref().map(|s| s.target) else { return Task::none() };
                if let Some(style) = self.stage_style(target) {
                    let fitted = OverlayStyle { width: None, height: None, ..style.clone() };
                    return self.restyle(target, fitted);
                }
            }
            other => {
                if let Some(stage) = &mut self.stage {
                    stage.set(&other);
                }
            }
        }
        Task::none()
    }

    /// Puts `target` on the stage, opening its window if it isn't up.
    fn open_stage(&mut self, target: StageTarget) -> Task<Message> {
        let game = self.game_key().map(str::to_owned);
        // The inspector starts on the side away from the overlay.
        let dock_left = self.stage_style(target).is_some_and(|s| s.position.cell().0 == 2);
        if let Some(stage) = &mut self.stage {
            stage.target = target;
            stage.game = game;
            stage.dock_left = dock_left;
            return window::gain_focus(stage.window);
        }
        let (window, open) = window::open(stage_window());
        self.stage = Some(Stage {
            window,
            target,
            game,
            tab: StageTab::Layout,
            snap: true,
            step: 32.0,
            show_grid: true,
            dim: false,
            dock_left,
            folded: false,
        });
        open.discard()
    }

    /// Closes the stage's window, if it's up.
    pub(super) fn close_stage(&mut self) -> Task<Message> {
        self.stage.take().map_or_else(Task::none, |s| window::close(s.window))
    }

    /// Closes the stage if what it shows is gone: another setup is shown, or the overlay was
    /// deleted (or undone).
    pub(super) fn settle_stage(&mut self) -> Task<Message> {
        let Some(stage) = &self.stage else { return Task::none() };
        let moved = stage.target.in_setup() && stage.game.as_deref() != self.game_key();
        if moved || self.stage_style(stage.target).is_none() {
            return self.close_stage();
        }
        Task::none()
    }

    /// The style of the overlay `target` names.
    fn stage_style(&self, target: StageTarget) -> Option<&OverlayStyle> {
        match target {
            StageTarget::Menu(i) => self.menus().get(i).map(|m| &m.style),
            StageTarget::Info(i) => self.infos().get(i).map(|o| &o.style),
            StageTarget::Log(i) => self.logs().get(i).map(|o| &o.style),
            StageTarget::Keyboard => Some(&self.config.keyboard_style),
            StageTarget::Numpad => Some(&self.config.numpad_style),
            StageTarget::Media => Some(&self.config.media_style),
            StageTarget::InGameMenu => Some(&self.config.menu_style),
        }
    }

    /// The message that gives `target` the style `style`, as its card's Appearance editor sends.
    fn stage_restyle(&self, target: StageTarget, style: OverlayStyle) -> Option<Message> {
        Some(match target {
            StageTarget::Menu(i) => Message::SetMenuStyle(i, style),
            StageTarget::Info(i) => Message::SetInfoStyle(i, style),
            StageTarget::Log(i) => Message::SetLog(i, LogOverlay { style, ..self.logs().get(i)?.clone() }),
            StageTarget::Keyboard => Message::SetKeyboardStyle(style),
            StageTarget::Numpad => Message::SetNumpadStyle(style),
            StageTarget::Media => Message::SetMediaStyle(style),
            StageTarget::InGameMenu => Message::SetInGameMenuStyle(style),
        })
    }

    fn restyle(&mut self, target: StageTarget, style: OverlayStyle) -> Task<Message> {
        self.stage_restyle(target, style).map_or_else(Task::none, |set| self.handle(set))
    }

    /// The stage window: the grid, the overlay with its grips, the inspector, and any picker the
    /// inspector opened.
    pub(super) fn view_stage(&self) -> Element<'_, Message> {
        let Some(stage) = &self.stage else { return space().into() };
        iced::widget::responsive(move |display| {
            let shade = if stage.dim { 0.45 } else { 0.0 };
            let grid = grid::grid(stage.show_grid.then_some(stage.step), shade);
            let panel = self
                .stage_style(stage.target)
                .and_then(|style| {
                    let fit = Fit::of(style, display);
                    let panel = self.stage_panel(stage.target, style, &fit)?;
                    let held = handles::handles(panel, display, |g| Message::Stage(StageMessage::Gesture(g)));
                    Some(crate::overlay::draw::place(held, style, &fit, &Anim::still()))
                })
                .unwrap_or_else(|| space().into());
            let mut layers = stack![grid, panel, self.view_inspector(stage, display)];
            if let Some(popup) = self.popup() {
                layers = layers.push(popup);
            }
            layers.into()
        })
        .into()
    }

    /// The overlay as the overlay process draws it, at its real size on this display, with
    /// made-up contents where the real ones come from the game (a log's presses, the media playing).
    fn stage_panel<'a>(&self, target: StageTarget, style: &OverlayStyle, fit: &Fit) -> Option<Element<'a, Message>> {
        use crate::overlay::draw;
        let font = self.preview_font();
        let still = Anim::still();
        Some(match target {
            StageTarget::Menu(i) => {
                let menu = self.menus().get(i)?;
                let menus = std::slice::from_ref(menu);
                let opener = Opener { buttons: vec![Button::LeftBumper], ..Opener::default() };
                match MenuSession::open(menus, &menu.name, opener).and_then(|s| s.view(menus)) {
                    Some(view) => draw::menu_panel(&view, font, self.menu_look(), fit),
                    None => placeholder("This menu has nothing to show yet: give it a name and an item."),
                }
            }
            StageTarget::Info(i) => {
                let live = crate::info::Live::sample(self.config.info_glyphs).with_layout(self.nintendo_layout());
                draw::info_panel(&crate::info::resolve(self.infos().get(i)?, &live), font, fit, &still)
            }
            StageTarget::Log(i) => {
                let o = self.logs().get(i)?;
                let crate::config::LogSource::Input(source) = &o.source;
                let sample = crate::inputlog::Inputs::sample();
                let now = sample.now.unwrap_or_else(Instant::now);
                let glyphs = crate::info::Glyphs { family: self.config.info_glyphs, nintendo_layout: self.nintendo_layout() };
                draw::log_panel(&crate::inputlog::log_view(&sample.entries(None), source, style, glyphs, now), font, fit, &still)
            }
            StageTarget::Keyboard | StageTarget::Numpad => {
                let board = match target {
                    StageTarget::Numpad => crate::keyboard::Board::Numpad,
                    _ if self.config.full_keyboard => crate::keyboard::Board::Full,
                    _ => crate::keyboard::Board::Basic,
                };
                let view = crate::overlay::KeyboardView {
                    board,
                    style: style.clone(),
                    cursor: crate::keyboard::find(board, "KEY_H").unwrap_or_default(),
                    latched: Vec::new(),
                    pressed: None,
                    closing: 0.0,
                    caps: false,
                    typed: String::new(),
                    field: None,
                };
                draw::keyboard_panel(&view, font, self.glyphs(), fit, &still)
            }
            StageTarget::Media => draw::media_panel(&crate::media::MediaView::sample(style.clone()), font, self.glyphs(), fit, &still),
            StageTarget::InGameMenu => draw::menu_panel(&crate::system_menu::main_page(style.clone(), 0), font, self.menu_look(), fit),
        })
    }
}

/// Something to hold on to where an overlay draws nothing yet.
fn placeholder<'a>(note: &'a str) -> Element<'a, Message> {
    container(text(note).size(16).color(Color::WHITE))
        .padding(24)
        .style(|_: &iced::Theme| container::Style { background: Some(Color::from_rgba(0.1, 0.1, 0.12, 0.85).into()), ..container::Style::default() })
        .into()
}

impl Stage {
    /// Takes one of the inspector's own settings.
    fn set(&mut self, message: &StageMessage) {
        match *message {
            StageMessage::Tab(tab) => self.tab = tab,
            StageMessage::Snap(on) => self.snap = on,
            StageMessage::Step(step) => self.step = step,
            StageMessage::ShowGrid(on) => self.show_grid = on,
            StageMessage::Dim(on) => self.dim = on,
            StageMessage::Dock => self.dock_left = !self.dock_left,
            StageMessage::Fold => self.folded = !self.folded,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
