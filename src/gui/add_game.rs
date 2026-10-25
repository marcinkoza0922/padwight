//! The library picker ("+ Add setup"): the built-in games, installed ones first, and a search.
//! The library may hold thousands of games, so what the list shows is worked out when the
//! search or the installed games change rather than every frame, and only the rows in view are
//! built.

use iced::widget::{column, row};

use super::*;
use super::packs::{Dialog, pack_summary};

/// How tall the list is.
const LIST_HEIGHT: f32 = 380.0;
/// Each row's height, the gap below it included. Every row is this tall, so the scroll offset
/// says which ones are in view.
const ROW_HEIGHT: f32 = 36.0;
/// Rows built past each edge of the view, so a quick scroll doesn't show a blank frame.
const OVERSCAN: usize = 4;
const LIST_ID: &str = "library-list";

pub(super) struct Picker {
    search: String,
    /// The library's names in lowercase, for the search.
    names: Vec<String>,
    rows: Vec<Row>,
    /// The game picked for the details pane (index in `App::library`).
    selected: Option<usize>,
    /// The game in the details pane, parsed: the listings hold only what the list shows.
    details: Option<(usize, library::Entry)>,
    /// How far the list is scrolled.
    top: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    /// The library entry at this index.
    Game(usize),
    /// Where the installed games end, so the order says what it's sorted by.
    NotInstalled,
}

impl Picker {
    fn new(library: &[library::Listing], installed: Option<&[bool]>) -> Self {
        let names = library.iter().map(|l| l.name.to_lowercase()).collect();
        let mut picker = Picker { search: String::new(), names, rows: Vec::new(), selected: None, details: None, top: 0.0 };
        picker.refresh(installed);
        picker.load(library);
        picker
    }

    /// Parses the game the details pane shows, when that changes.
    fn load(&mut self, library: &[library::Listing]) {
        let shown = self.shown();
        if self.details.as_ref().map(|(i, _)| *i) != shown {
            self.details = shown.and_then(|i| Some((i, library.get(i)?.entry()?)));
        }
    }

    /// The games matching the search, installed ones first (the library is sorted by name).
    fn refresh(&mut self, installed: Option<&[bool]>) {
        let query = self.search.trim().to_lowercase();
        let is_installed = |i: usize| installed.is_some_and(|found| found.get(i) == Some(&true));
        let (mine, rest): (Vec<usize>, Vec<usize>) =
            (0..self.names.len()).filter(|i| self.names[*i].contains(&query)).partition(|i| is_installed(*i));
        let split = !mine.is_empty() && !rest.is_empty();
        self.rows = mine.into_iter().map(Row::Game).chain(split.then_some(Row::NotInstalled)).chain(rest.into_iter().map(Row::Game)).collect();
    }

    /// The selected game, or the first one listed, so the details pane is never blank.
    fn shown(&self) -> Option<usize> {
        let first = || self.rows.iter().find_map(|r| if let Row::Game(i) = r { Some(*i) } else { None });
        self.selected.filter(|i| self.rows.contains(&Row::Game(*i))).or_else(first)
    }
}

/// The list back at its top, as after a change to what it shows.
fn scroll_to_top(picker: &mut Picker) -> Task<Message> {
    picker.top = 0.0;
    iced::widget::operation::snap_to(LIST_ID, scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) })
}

impl App {
    pub(super) fn update_add_game(&mut self, message: Message) -> Task<Message> {
        let library = self.library;
        let installed = self.installed.as_deref();
        let picker = match &mut self.dialog {
            Some(Dialog::AddGame(picker)) => Some(picker.as_mut()),
            _ => None,
        };
        match (message, picker) {
            (Message::OpenAddGame, _) => {
                self.dialog = Some(Dialog::AddGame(Box::new(Picker::new(self.library, installed))));
                if self.installed.is_none() {
                    let find = move || {
                        let found = launchers::scan();
                        library.iter().map(|l| found.has(&l.rules)).collect()
                    };
                    return Task::perform(async move { tokio::task::spawn_blocking(find).await.unwrap_or_default() }, Message::Installed);
                }
            }
            (Message::Installed(found), picker) => {
                self.installed = Some(found);
                if let Some(picker) = picker {
                    picker.refresh(self.installed.as_deref());
                    picker.load(library);
                    return scroll_to_top(picker);
                }
            }
            (Message::SetLibrarySearch(text), Some(picker)) => {
                picker.search = text;
                picker.refresh(installed);
                picker.load(library);
                return scroll_to_top(picker);
            }
            (Message::SelectLibrary(i), Some(picker)) => {
                picker.selected = Some(i);
                picker.load(library);
            }
            (Message::LibraryScrolled(top), Some(picker)) => picker.top = top,
            _ => {}
        }
        Task::none()
    }

    pub(super) fn view_add_game<'a>(&'a self, picker: &'a Picker) -> Element<'a, Message> {
        let added = |id: &str| self.config.games.iter().find(|g| g.origin.as_ref().is_some_and(|o| o.id == id));
        let installed = |i: usize| self.installed.as_ref().is_some_and(|found| found.get(i) == Some(&true));
        let installed_first = matches!(picker.rows.first(), Some(Row::Game(i)) if installed(*i));
        let selected = picker.details.as_ref().map(|(i, _)| *i);

        let mut list = column![
            row![
                dropdown(Template::NEW, None::<Template>, Message::AddEmptyGame).placeholder("Empty setup from a template…").style(style::action_dropdown).width(300),
                button(text("Import a file…")).style(style::secondary).on_press(Message::ImportFile),
            ]
            .spacing(8),
            row![
                text(match &self.installed {
                    None => "Checking which games you have installed…",
                    Some(_) if installed_first => "Built-in games, installed ones first:",
                    Some(_) => "Built-in games:",
                })
                .size(13)
                .color(MUTED_COLOR),
                space::horizontal(),
                tutorial_link("tutorial-first-setup.md"),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8);
        list = list.push(if self.library.is_empty() {
            Element::from(text("The built-in library is empty in this build.").size(13).color(MUTED_COLOR))
        } else if picker.rows.is_empty() {
            text("No built-in game matches.").size(13).color(MUTED_COLOR).into()
        } else {
            self.library_rows(picker, selected)
        });

        let mut details = column![].spacing(8).width(320);
        // The main action sits at the bottom with Cancel, as in every dialog.
        let mut action = None;
        match &picker.details {
            Some((i, entry)) => {
                let i = *i;
                let p = &entry.pack;
                details = details.push(text(p.pack.name.clone()).size(20)).push(pack_summary(p, |f| self.pad_has(f)));
                action = Some(match added(&p.pack.id) {
                    Some(g) => button(text("Open it")).on_press(Message::SelectPage(Page::Game(Some(g.name.clone())))),
                    None => button(text("Preview & add…")).on_press(Message::PreviewLibrary(i)),
                });
            }
            None => details = details.push(note("Pick a setup to see what's in it.")),
        }

        column![
            text("Add a setup").size(22),
            field("Search the library…", &picker.search).on_input(Message::SetLibrarySearch),
            row![list.width(Length::Fill), rule::vertical(1), details].spacing(16),
            row![space::horizontal(), button(text("Cancel")).style(style::secondary).on_press(Message::CloseDialog)].push(action).spacing(8),
        ]
        .spacing(12)
        .width(820)
        .into()
    }

    /// The scrolling list, built only where it's in view: space stands in for the rest.
    fn library_rows<'a>(&'a self, picker: &'a Picker, selected: Option<usize>) -> Element<'a, Message> {
        let added: HashSet<&str> = self.config.games.iter().filter_map(|g| g.origin.as_ref()).map(|o| o.id.as_str()).collect();
        let installed = |i: usize| self.installed.as_ref().is_some_and(|found| found.get(i) == Some(&true));
        let rows = &picker.rows;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a non-negative row count, rounded down")]
        let at = |y: f32| (y.max(0.0) / ROW_HEIGHT) as usize;
        let first = at(picker.top).saturating_sub(OVERSCAN).min(rows.len());
        let last = (at(picker.top + LIST_HEIGHT) + 1 + OVERSCAN).min(rows.len());
        let height = |n: usize| ROW_HEIGHT * n as f32;

        let mut entries = column![space().height(height(first))];
        for row in &rows[first..last] {
            let line: Element<'a, Message> = match *row {
                Row::NotInstalled => container(text("Not installed").size(12).color(MUTED_COLOR)).align_bottom(ROW_HEIGHT).padding([4, 0]).into(),
                Row::Game(i) => {
                    let listing = &self.library[i];
                    let mut line = row![text(listing.name.as_str()).size(15).width(Length::Fill)].spacing(8).align_y(Alignment::Center);
                    for f in listing.required_by_all() {
                        line = line.push(text(f.label()).size(12).color(MUTED_COLOR));
                    }
                    if installed(i) {
                        line = line.push(text("installed").size(12).color(style_accent()));
                    }
                    if added.contains(listing.id.as_str()) {
                        line = line.push(text("added").size(12).color(MUTED_COLOR));
                    }
                    let entry = button(line).width(Length::Fill).padding([6, 10]).style(style::segment(selected == Some(i))).on_press(Message::SelectLibrary(i));
                    container(entry).center_y(ROW_HEIGHT).into()
                }
            };
            entries = entries.push(line);
        }
        entries = entries.push(space().height(height(rows.len() - last)));
        // A gap on the right keeps the tags clear of the scrollbar.
        scrollable(entries.width(Length::Fill).padding(iced::Padding::ZERO.right(14)))
            .id(LIST_ID)
            .on_scroll(|view| Message::LibraryScrolled(view.absolute_offset().y))
            .height(LIST_HEIGHT)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    fn picker(app: &App) -> &Picker {
        let Some(Dialog::AddGame(picker)) = &app.dialog else { panic!("no library picker") };
        picker
    }

    #[test]
    fn the_add_game_picker_and_page_changes_close_dialogs() {
        let mut app = app();
        let _ = app.update(Message::OpenAddGame);
        let _ = app.update(Message::SetLibrarySearch("doo".into()));
        assert_eq!(picker(&app).search, "doo");
        let _ = app.update(Message::SelectPage(Page::Settings));
        assert!(app.dialog.is_none() && app.page == Page::Settings);
    }

    #[test]
    fn installed_games_come_first_under_their_own_heading() {
        let mut app = app();
        let _ = app.update(Message::OpenAddGame);
        let last = app.library.len() - 1;
        let _ = app.update(Message::Installed((0..=last).map(|i| i == last).collect()));
        let rows = &picker(&app).rows;
        assert_eq!(rows[..2], [Row::Game(last), Row::NotInstalled]);
        assert_eq!(rows.len(), app.library.len() + 1);
        assert_eq!(picker(&app).shown(), Some(last), "the first one listed until another is picked");
        assert_eq!(picker(&app).details.as_ref().map(|(i, e)| (*i, e.pack.pack.id.as_str())), Some((last, app.library[last].id.as_str())));
        let _ = app.update(Message::SelectLibrary(1));
        assert_eq!(picker(&app).details.as_ref().map(|(i, _)| *i), Some(1), "parsed when picked");

        let name = app.library[0].name.to_uppercase();
        let _ = app.update(Message::SetLibrarySearch(format!(" {name} ")));
        assert_eq!(picker(&app).rows[0], Row::Game(0), "any case, spaces trimmed");
        assert!(!picker(&app).rows.contains(&Row::NotInstalled), "no heading without installed games above it");
        let _ = app.update(Message::SetLibrarySearch("no game is called this".into()));
        assert!(picker(&app).rows.is_empty() && picker(&app).details.is_none());
        let _ = app.view();
    }

    #[test]
    fn only_the_rows_in_view_are_built() {
        let mut app = app();
        let _ = app.update(Message::OpenAddGame);
        let _ = app.update(Message::Installed(Vec::new()));
        let built = |app: &App| {
            let picker = picker(app);
            let selected = picker.shown();
            let list = app.library_rows(picker, selected);
            // The column inside the scrollable: two spacers around the rows.
            iced::advanced::Widget::children(list.as_widget())[0].children.len() - 2
        };
        let in_view = built(&app);
        assert!(in_view < app.library.len() && in_view <= (LIST_HEIGHT / ROW_HEIGHT) as usize + 2 + OVERSCAN, "{in_view}");
        let _ = app.update(Message::LibraryScrolled(ROW_HEIGHT * (app.library.len() as f32)));
        assert!(built(&app) <= OVERSCAN + 1, "past the end: the last few");
    }

    /// How long the library picker takes with thousands of games in the library.
    /// `cargo nextest run --release --run-ignored only add_game_picker_scales --no-capture`
    #[test]
    #[ignore = "a timing benchmark"]
    #[expect(clippy::print_stderr, reason = "the figures are what it prints")]
    fn add_game_picker_scales() {
        let mut app = app();
        let copies = (0..20).flat_map(|copy| {
            app.library.iter().cloned().map(move |mut l| {
                l.name = format!("{} {copy}", l.name);
                l.id = format!("{}-{copy}", l.id);
                l
            })
        });
        app.library = Vec::leak(copies.collect());
        let started = std::time::Instant::now();
        let found = launchers::scan();
        let scanned = started.elapsed();
        let started = std::time::Instant::now();
        let installed: Vec<bool> = app.library.iter().map(|l| found.has(&l.rules)).collect();
        eprintln!("{} games: scan {scanned:?}, matching {:?}", app.library.len(), started.elapsed());
        let started = std::time::Instant::now();
        let _ = app.update(Message::OpenAddGame);
        let _ = app.update(Message::Installed(installed));
        eprintln!("opening: {:?}", started.elapsed());
        let started = std::time::Instant::now();
        for _ in 0..10 {
            let _ = app.view();
        }
        eprintln!("view: {:?}", started.elapsed() / 10);
        let started = std::time::Instant::now();
        for q in ["d", "do", "doo", "doom", "doo", "do", "d", ""] {
            let _ = app.update(Message::SetLibrarySearch(q.into()));
            let _ = app.view();
        }
        eprintln!("typing: {:?} a key", started.elapsed() / 8);
        let started = std::time::Instant::now();
        for y in 0..100 {
            let _ = app.update(Message::LibraryScrolled(y as f32 * 50.0));
            let _ = app.view();
        }
        eprintln!("scrolling: {:?} a step", started.elapsed() / 100);
        let _ = app.update(Message::CloseDialog);
        let started = std::time::Instant::now();
        let _ = app.update(Message::OpenBrowse(ItemKind::Menu));
        let _ = app.view();
        eprintln!("copy from another setup: opening {:?}", started.elapsed());
        let started = std::time::Instant::now();
        for _ in 0..10 {
            let _ = app.view();
        }
        eprintln!("copy from another setup: view {:?}", started.elapsed() / 10);
    }
}
