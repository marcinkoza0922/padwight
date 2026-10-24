//! The stage's inspector: a panel docked at one side of the screen with the overlay's layout and
//! grid, its look (the cards' Appearance editor), and its contents, plus Undo, Save & apply and
//! Done. It folds down to a bar to see the screen behind it.

use iced::widget::{column, row};

use super::*;

/// How wide the inspector is: room for a menu's kind and an item's label with its buttons.
const WIDTH: f32 = 720.0;
/// Room the inspector keeps above and below its scrolling page, for its header, tabs and footer.
const CHROME: f32 = 230.0;

const HOW_TO: &str = "Drag the overlay to move it: it snaps to the nearest of nine spots and grows from there. Drag an \
                      edge or corner to resize it. With snapping on, moved edges land on the grid or back on the spot; \
                      hold Alt to place freely. Arrow keys nudge it (10 px with Shift). Click a menu item or info \
                      overlay cell to edit it. Esc closes the editor; Ctrl+S saves.";

impl App {
    pub(super) fn view_inspector<'a>(&'a self, stage: &'a Stage, display: iced::Size) -> Element<'a, Message> {
        let body: Element<'a, Message> = if stage.folded { self.folded_bar(stage) } else { self.inspector(stage, display.height - CHROME) };
        let docked = container(opaque(body)).padding(16);
        if stage.dock_left { row![docked, space::horizontal()] } else { row![space::horizontal(), docked] }.into()
    }

    /// The inspector unfolded, its page scrolling past `tallest`.
    fn inspector<'a>(&'a self, stage: &'a Stage, tallest: f32) -> Element<'a, Message> {
        let tabs = [(StageTab::Layout, "Layout"), (StageTab::Look, "Look"), (StageTab::Content, "Content")]
            .into_iter()
            .filter(|(tab, _)| *tab != StageTab::Content || has_content(stage.target))
            .fold(row![].spacing(6), |tabs, (tab, label)| {
                let style = if stage.tab == tab { style::primary } else { style::secondary };
                tabs.push(button(text(label).size(14)).style(style).on_press(Message::Stage(StageMessage::Tab(tab))))
            });
        let page = match stage.tab {
            StageTab::Look => self.stage_style(stage.target).map_or_else(|| space().into(), |style| {
                let target = stage.target;
                style_editor(style, Rc::new(move |s| self.stage_restyle(target, s).unwrap_or(Message::Stage(StageMessage::Close))))
            }),
            StageTab::Content if has_content(stage.target) => self.stage_content(stage.target),
            _ => self.stage_layout(stage),
        };
        let layout = column![
            self.inspector_header(stage),
            tabs,
            rule::horizontal(1),
            container(scrollable(container(page).padding(iced::Padding::ZERO.right(14))).id(super::pick::PAGE)).max_height(tallest.max(120.0)),
            rule::horizontal(1),
            self.inspector_footer(),
        ]
        .spacing(12);
        panel(layout.into(), Length::Fixed(WIDTH))
    }

    fn inspector_header<'a>(&self, stage: &Stage) -> Element<'a, Message> {
        let small = |label: &'static str, tip: &'static str, message: StageMessage| {
            tooltip(
                button(text(label).size(14)).style(style::secondary).on_press(Message::Stage(message)),
                container(text(tip).size(13)).padding(8).style(style::tooltip),
                tooltip::Position::Bottom,
            )
        };
        row![
            text(self.stage_title(stage.target)).size(18),
            space::horizontal(),
            small("⇆", "Dock on the other side", StageMessage::Dock),
            small("–", "Fold into a bar", StageMessage::Fold),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }

    fn inspector_footer<'a>(&self) -> Element<'a, Message> {
        let dirty = self.config != self.saved;
        let note: Element<'a, Message> = match &self.message {
            Some((m, is_err)) => text(m.clone()).size(13).color(if *is_err { ERROR_COLOR } else { MUTED_COLOR }).into(),
            None if dirty => text("Unsaved changes").size(13).color(MUTED_COLOR).into(),
            None => space().into(),
        };
        row![
            container(note).width(Length::Fill),
            button(text("Undo")).style(style::secondary).on_press_maybe((!self.undo.is_empty()).then_some(Message::Undo)),
            button(text("Save & apply")).style(style::primary).on_press_maybe(dirty.then_some(Message::Save)),
            button(text("Done")).style(style::secondary).on_press(Message::Stage(StageMessage::Close)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    /// The inspector folded: the overlay's name and the buttons to unfold, save and close.
    fn folded_bar<'a>(&self, stage: &Stage) -> Element<'a, Message> {
        let dirty = self.config != self.saved;
        let bar = row![
            text(self.stage_title(stage.target)).size(15),
            button(text("Show editor").size(13)).style(style::secondary).on_press(Message::Stage(StageMessage::Fold)),
            button(text("Save & apply").size(13)).style(style::primary).on_press_maybe(dirty.then_some(Message::Save)),
            button(text("Done").size(13)).style(style::secondary).on_press(Message::Stage(StageMessage::Close)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        panel(bar.into(), Length::Shrink)
    }

    /// What the stage shows, as its card calls it.
    pub(super) fn stage_title(&self, target: StageTarget) -> String {
        let named = |kind: &str, name: Option<&String>| match name.filter(|n| !n.is_empty()) {
            Some(name) => format!("{kind} “{name}”"),
            None => kind.to_string(),
        };
        match target {
            StageTarget::Menu(i) => named("Menu", self.menus().get(i).map(|m| &m.name)),
            StageTarget::Info(i) => named("Info overlay", self.infos().get(i).map(|o| &o.name)),
            StageTarget::Log(i) => named("Log overlay", self.logs().get(i).map(|o| &o.name)),
            StageTarget::Panel(panel) => panel.name().into(),
            StageTarget::Setup(panel) => format!("{} for {}", panel.name(), self.game().name),
        }
    }

    /// Where the overlay sits and how big it is, and the grid.
    fn stage_layout<'a>(&self, stage: &Stage) -> Element<'a, Message> {
        let steps = GRID_STEPS.into_iter().fold(row![].spacing(4), |steps, step| {
            let style = if (stage.step - step).abs() < f32::EPSILON { style::primary } else { style::secondary };
            steps.push(button(text(format!("{step:.0}")).size(13)).style(style).on_press(Message::Stage(StageMessage::Step(step))))
        });
        let mut rows = vec![
            labeled("Snap", checkbox(stage.snap).label("Snap to the grid").on_toggle(|on| Message::Stage(StageMessage::Snap(on))).into()),
            labeled("Grid size", row![steps, text("px").size(13).color(MUTED_COLOR)].spacing(8).align_y(Alignment::Center).into()),
            labeled("Show", checkbox(stage.show_grid).label("Grid lines").on_toggle(|on| Message::Stage(StageMessage::ShowGrid(on))).into()),
            labeled("Screen", checkbox(stage.dim).label("Dim what's behind").on_toggle(|on| Message::Stage(StageMessage::Dim(on))).into()),
        ];
        if let Some(style) = self.stage_style(stage.target) {
            let size = match (style.width, style.height) {
                (None, None) => "Fits its contents".to_string(),
                (w, h) => {
                    let side = |v: Option<f32>| v.map_or("fit".to_string(), |v| format!("{v:.1}%"));
                    format!("{} wide × {} tall", side(w), side(h))
                }
            };
            let fit = button(text("Fit contents").size(13))
                .style(style::secondary)
                .on_press_maybe((style.width.is_some() || style.height.is_some()).then_some(Message::Stage(StageMessage::FitContents)));
            rows.push(labeled("Spot", text(format!("{} · moved {:.1}% across, {:.1}% down", style.position, style.x_offset, style.y_offset)).size(14).into()));
            rows.push(labeled("Size", row![text(size).size(14), fit].spacing(12).align_y(Alignment::Center).into()));
        }
        let flash = button(text("Flash").size(13)).style(style::secondary).on_press(Message::Stage(StageMessage::Flash));
        let tip = container(text("Make the overlay flash, to spot it against a dark or busy background.").size(13)).padding(8).style(style::tooltip);
        rows.push(labeled("Find it", tooltip(flash, tip, tooltip::Position::Top).into()));
        let animate = checkbox(stage.animate).label("Play animations").on_toggle(|on| Message::Stage(StageMessage::Animate(on)));
        let replay = button(text("Replay opening").size(13)).style(style::secondary).on_press_maybe(stage.animate.then_some(Message::Stage(StageMessage::Replay)));
        let tip = container(text("Move it as it does in game: its motion style opens it, and drifting, pulsing glows and moving filters run.").size(13))
            .padding(8)
            .style(style::tooltip);
        rows.push(labeled("Motion", tooltip(row![animate, replay].spacing(12).align_y(Alignment::Center), tip, tooltip::Position::Top).into()));
        rows.push(labeled("Also show", self.around_picker(stage)));
        rows.push(note(HOW_TO).into());
        column(rows).spacing(10).into()
    }

    /// What the overlay shows: a menu's items, an info overlay's cells.
    fn stage_content<'a>(&'a self, target: StageTarget) -> Element<'a, Message> {
        match target {
            StageTarget::Menu(i) => {
                let Some(menu) = self.menus().get(i) else { return space().into() };
                let mut rows = menu_basics(i, menu);
                rows.push(self.menu_items_editor(i, menu, &self.names(), true));
                column(rows).spacing(10).into()
            }
            StageTarget::Info(i) => {
                let Some(o) = self.infos().get(i) else { return space().into() };
                column![
                    labeled("Name", field("Info overlay name", &o.name).on_input(move |n| Message::RenameInfo(i, n)).width(240).into()),
                    info_cells_editor(i, o, self.config.info_glyphs, self.preview_font()),
                ]
                .spacing(10)
                .into()
            }
            _ => space().into(),
        }
    }
}

/// Whether the overlay has contents to edit here, besides its look.
fn has_content(target: StageTarget) -> bool {
    matches!(target, StageTarget::Menu(_) | StageTarget::Info(_))
}

/// The inspector's box: the window's colors, a little see-through, with a shadow to stand off the game.
fn panel<'a>(content: Element<'a, Message>, width: Length) -> Element<'a, Message> {
    container(content)
        .padding(16)
        .width(width)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(Color { a: 0.97, ..palette.background.base.color }.into()),
                text_color: Some(palette.background.base.text),
                border: iced::Border { radius: 12.0.into(), width: 1.0, color: palette.background.strong.color },
                shadow: iced::Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.4), offset: iced::Vector::new(0.0, 6.0), blur_radius: 24.0 },
                ..container::Style::default()
            }
        })
        .into()
}
