//! The on-screen editor ("stage"): an overlay drawn full size in a see-through window over the
//! whole screen, right where the overlay process would put it, to move and resize with the mouse
//! (its edges snapping to a grid), restyle and fill from a panel beside it, and save from there.
//! Edits go through the same messages as the settings window's cards, so undo and Save & apply
//! work as they do there. Clicking an item or cell on the overlay opens it in the panel, and the
//! overlays that can be up with it are drawn faintly around it.

use iced::window;

use super::*;
use crate::{
    motion::Anim,
    overlay::{draw::Part, fit::Fit},
};

mod context;
mod grid;
mod handles;
mod inspector;
mod panel;
mod pick;
mod playback;
pub(super) mod snap;

pub(super) use pick::{cell_field, item_field};

/// The overlay on the stage. Menus, info and log overlays are the shown setup's, by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StageTarget {
    Menu(usize),
    Info(usize),
    Log(usize),
    /// A panel in the look set on the App settings page.
    Panel(Panel),
    /// A panel in the shown setup's own look, from its Details tab.
    Setup(Panel),
}

/// The overlays that come one at a time, each with one look for the app and maybe one per setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Panel {
    Keyboard,
    Numpad,
    Media,
    InGameMenu,
}

impl Panel {
    pub(super) fn name(self) -> &'static str {
        match self {
            Panel::Keyboard => "On-screen keyboard",
            Panel::Numpad => "On-screen numpad",
            Panel::Media => "Media controls",
            Panel::InGameMenu => "In-game menu",
        }
    }
}

impl StageTarget {
    /// Whether it belongs to a setup, rather than to the app.
    fn in_setup(self) -> bool {
        !matches!(self, StageTarget::Panel(_))
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
    /// The other overlays drawn faintly around it.
    around: Vec<StageTarget>,
    /// The item or cell last clicked on the overlay, open in the Content tab.
    picked: Option<Part>,
    /// When the Flash button was pressed, while the overlay is still flashing.
    flash: Option<Instant>,
    /// Whether the overlay moves as it would in game: its motion style and looping effects.
    animate: bool,
    /// When its opening last started playing.
    played: Instant,
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
    /// Draws another overlay around it, or stops.
    Around(StageTarget, bool),
    /// Draws all the others around it, or none.
    AroundAll(bool),
    Pick(Part),
    /// Makes the overlay flash, to find it against a busy or dark background.
    Flash,
    /// Plays the overlay's motion and looping effects, or draws it still.
    Animate(bool),
    /// Plays its opening again.
    Replay,
    /// A frame passes while it flashes or moves.
    Tick(Instant),
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
    row![disclosure("Appearance", open, toggle), edit_button(target)].spacing(12).align_y(Alignment::Center).into()
}

/// The button that opens `target` on the stage.
pub(super) fn edit_button<'a>(target: StageTarget) -> Element<'a, Message> {
    let edit = button(text("Edit on screen").size(13)).style(style::secondary).on_press(Message::Stage(StageMessage::Open(target)));
    tooltip(
        edit,
        container(text("Show it full size where it goes, over everything, to drag, resize and restyle it there.").size(13)).padding(8).style(style::tooltip),
        tooltip::Position::Top,
    )
    .into()
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
            StageMessage::Pick(part) => return self.pick(part),
            StageMessage::Flash => {
                if let Some(stage) = &mut self.stage {
                    stage.flash = Some(Instant::now());
                }
            }
            StageMessage::Tick(now) => self.stage.iter_mut().for_each(|s| s.tick(now)),
            StageMessage::AroundAll(on) => {
                let all = if on { self.stage.as_ref().map(|s| self.around_choices(s.target)).unwrap_or_default() } else { Vec::new() };
                if let Some(stage) = &mut self.stage {
                    stage.around = all;
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
            stage.picked = None;
            stage.played = Instant::now();
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
            around: self.default_around(),
            picked: None,
            flash: None,
            animate: false,
            played: Instant::now(),
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
            StageTarget::Panel(Panel::Keyboard) => Some(&self.config.keyboard_style),
            StageTarget::Panel(Panel::Numpad) => Some(&self.config.numpad_style),
            StageTarget::Panel(Panel::Media) => Some(&self.config.media_style),
            StageTarget::Panel(Panel::InGameMenu) => Some(&self.config.menu_style),
            StageTarget::Setup(panel) => {
                let game = self.game();
                match panel {
                    Panel::Keyboard => game.keyboard_style.as_ref(),
                    Panel::Numpad => game.numpad_style.as_ref(),
                    Panel::Media => game.media_style.as_ref(),
                    Panel::InGameMenu => game.menu_style.as_ref(),
                }
            }
        }
    }

    /// The message that gives `target` the style `style`, as its card's Appearance editor sends.
    fn stage_restyle(&self, target: StageTarget, style: OverlayStyle) -> Option<Message> {
        Some(match target {
            StageTarget::Menu(i) => Message::SetMenuStyle(i, style),
            StageTarget::Info(i) => Message::SetInfoStyle(i, style),
            StageTarget::Log(i) => Message::SetLog(i, LogOverlay { style, ..self.logs().get(i)?.clone() }),
            StageTarget::Panel(Panel::Keyboard) => Message::SetKeyboardStyle(style),
            StageTarget::Panel(Panel::Numpad) => Message::SetNumpadStyle(style),
            StageTarget::Panel(Panel::Media) => Message::SetMediaStyle(style),
            StageTarget::Panel(Panel::InGameMenu) => Message::SetInGameMenuStyle(style),
            StageTarget::Setup(Panel::Keyboard) => Message::SetGameOverlayStyle(crate::keyboard::Layout::Keyboard, Some(style)),
            StageTarget::Setup(Panel::Numpad) => Message::SetGameOverlayStyle(crate::keyboard::Layout::Numpad, Some(style)),
            StageTarget::Setup(Panel::Media) => Message::SetGameMediaStyle(Some(style)),
            StageTarget::Setup(Panel::InGameMenu) => Message::SetGameMenuStyle(Some(style)),
        })
    }

    fn restyle(&mut self, target: StageTarget, style: OverlayStyle) -> Task<Message> {
        self.stage_restyle(target, style).map_or_else(Task::none, |set| self.handle(set))
    }

    /// The stage window: the grid, the overlays that can be up with this one, the overlay with its
    /// grips, the inspector, and any picker the inspector opened.
    pub(super) fn view_stage(&self) -> Element<'_, Message> {
        let Some(stage) = &self.stage else { return space().into() };
        iced::widget::responsive(move |display| {
            let shade = if stage.dim { 0.45 } else { 0.0 };
            let grid = grid::grid(stage.show_grid.then_some(stage.step), shade);
            let panel = self
                .stage_style(stage.target)
                .and_then(|style| {
                    let fit = Fit::of(style, display);
                    let anim = self.stage_anim(stage);
                    let panel = self.stage_panel(stage.target, style, &fit, &anim)?;
                    let held = handles::handles(panel, display, stage.flashing(), |g| Message::Stage(StageMessage::Gesture(g)), |p| Message::Stage(StageMessage::Pick(p)));
                    Some(crate::overlay::draw::place(held, style, &fit, &anim))
                })
                .unwrap_or_else(|| space().into());
            let context = self.stage_context(stage, display);
            let mut layers = stack![grid, context, panel, self.view_inspector(stage, display)];
            if let Some(popup) = self.popup() {
                layers = layers.push(popup);
            }
            layers.into()
        })
        .into()
    }
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
            StageMessage::Animate(on) => {
                self.animate = on;
                self.played = Instant::now();
            }
            StageMessage::Replay => self.played = Instant::now(),
            StageMessage::Around(target, true) if !self.shows_around(target) => self.around.push(target),
            StageMessage::Around(target, false) => self.around.retain(|t| !context::same_overlay(*t, target)),
            StageMessage::Dock => self.dock_left = !self.dock_left,
            StageMessage::Fold => self.folded = !self.folded,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
