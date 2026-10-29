//! The Games page: games imported from Steam, Heroic and Lutris, started through the store's
//! own link. A game's setup is the one that plays it. Thousands of rows are possible, so only
//! the ones in view are built.

use std::collections::HashMap;

use iced::widget::{column, row};

use super::*;
use crate::{config::{Store, Title}, titles};

const LIST_HEIGHT: f32 = 420.0;
const ROW_HEIGHT: f32 = 40.0;
const OVERSCAN: usize = 4;
const LIST_ID: &str = "titles-list";

#[derive(Default)]
pub(super) struct GamesPage {
    search: String,
    /// The title picked for the details below the list.
    selected: Option<String>,
    top: f32,
    importing: bool,
    /// An import has run since the app started, so opening the page doesn't repeat it.
    imported: bool,
    show_hidden: bool,
    help: bool,
    /// Library game (index in `App::library`) that looks like each title that has no setup.
    library: HashMap<String, usize>,
    /// The variables field as typed for one title, which the parsed variables can't show
    /// (`NAME=` while the value is still to come).
    env_draft: Option<(String, String)>,
}

/// The `NAME=value` words of `text`; words with no name or no `=` are left out.
fn parse_env(text: &str) -> std::collections::BTreeMap<String, String> {
    text.split_whitespace().filter_map(|word| word.split_once('=')).filter(|(k, _)| !k.is_empty()).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// The launch-option line that makes the game start through Padwight.
fn launch_line(title: &Title) -> String {
    match title.source {
        Store::Steam => "padwight run -- %command%".to_string(),
        _ => format!("padwight run --title {} --", title.id),
    }
}

/// Plays a title through the daemon, which follows the game; with no daemon running, the store's
/// link is opened here and nothing follows the game.
async fn play(title: Title) -> Result<(), String> {
    match call(Request::LaunchTitle(Box::new(title.clone()))).await {
        Ok(Response::Error(e)) => Err(e),
        Ok(_) => Ok(()),
        Err(_) => tokio::task::spawn_blocking(move || titles::open_link(&title.link())).await.unwrap_or(Ok(())),
    }
}

impl GamesPage {
    /// Picks the first title, as a click would.
    pub(super) fn select_first(&mut self, titles: &[Title]) {
        self.selected = titles.first().map(|t| t.id.clone());
        self.imported = true;
        self.importing = false;
    }
}

/// A setup in the dropdown; `None` leaves the choice to the setups' rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SetupChoice(Option<String>);

impl std::fmt::Display for SetupChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_deref().unwrap_or("Match by rules"))
    }
}

impl App {
    pub(super) fn update_titles(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ImportTitles => return self.import_titles(),
            Message::TitlesScanned(scanned) => {
                self.games_page.importing = false;
                titles::merge(&mut self.config.titles, &scanned);
                self.games_page.library = scanned.iter().filter_map(|s| Some((s.title.id.clone(), s.library?))).collect();
            }
            Message::SetTitleSearch(text) => {
                self.games_page.search = text;
                return self.scroll_titles_to_top();
            }
            Message::SelectTitle(id) => self.games_page.selected = Some(id),
            Message::TitlesScrolled(top) => self.games_page.top = top,
            Message::ToggleHiddenTitles(on) => {
                self.games_page.show_hidden = on;
                return self.scroll_titles_to_top();
            }
            Message::ToggleAddGameHelp => self.games_page.help = !self.games_page.help,
            Message::PlayTitle(id) => {
                let Some(title) = self.config.titles.iter().find(|t| t.id == id).cloned() else { return Task::none() };
                return Task::perform(play(title), Message::TitleOpened);
            }
            Message::StopTitle(id) => return call_ok(Request::StopTitle(id)),
            Message::TitleOpened(Err(e)) => self.message = Some((e, true)),
            Message::SetTitleSetup(id, setup) => {
                if let Some(title) = self.config.titles.iter_mut().find(|t| t.id == id) {
                    title.setup = setup;
                }
            }
            Message::SetTitleHideControllers(id, on) => {
                if let Some(title) = self.config.titles.iter_mut().find(|t| t.id == id) {
                    title.hide_controllers = on;
                }
            }
            Message::SetTitleEnv(id, text) => {
                if let Some(title) = self.config.titles.iter_mut().find(|t| t.id == id) {
                    title.env = parse_env(&text);
                }
                self.games_page.env_draft = Some((id, text));
            }
            Message::CopyLaunchLine(line) => return iced::clipboard::write(line),
            Message::SetTitleHidden(id, hidden) => {
                if let Some(title) = self.config.titles.iter_mut().find(|t| t.id == id) {
                    title.hidden = hidden;
                }
            }
            _ => {}
        }
        Task::none()
    }

    /// Looks through the stores' files off the UI thread, once when the page first opens and
    /// again whenever "Import games" is pressed.
    pub(super) fn import_titles(&mut self) -> Task<Message> {
        if self.games_page.importing {
            return Task::none();
        }
        self.games_page.importing = true;
        self.games_page.imported = true;
        let setups: Vec<(String, Vec<Rule>)> = self.config.games.iter().map(|g| (g.name.clone(), g.rules.clone())).collect();
        let library = self.library;
        Task::perform(async move { tokio::task::spawn_blocking(move || titles::scan(&setups, library)).await.unwrap_or_default() }, Message::TitlesScanned)
    }

    pub(super) fn import_titles_once(&mut self) -> Task<Message> {
        if self.games_page.imported { Task::none() } else { self.import_titles() }
    }

    fn scroll_titles_to_top(&mut self) -> Task<Message> {
        self.games_page.top = 0.0;
        iced::widget::operation::snap_to(LIST_ID, scrollable::RelativeOffset { x: Some(0.0), y: Some(0.0) })
    }

    /// Whether the game started for this title is found yet, if one was started.
    fn launch_of(&self, id: &str) -> Option<bool> {
        self.status.as_ref()?.launches.iter().find(|l| l.id == id).map(|l| l.running)
    }

    /// Play, or Stop while the game was started from here.
    fn play_button<'a>(&self, title: &Title) -> iced::widget::Button<'a, Message> {
        match self.launch_of(&title.id) {
            Some(_) => button(text("Stop")).style(style::secondary).on_press(Message::StopTitle(title.id.clone())),
            None => button(text("Play")).style(style::secondary).on_press_maybe(title.installed.then(|| Message::PlayTitle(title.id.clone()))),
        }
    }

    /// The titles to list: installed ones first, each group by name.
    fn listed_titles(&self) -> Vec<&Title> {
        let query = self.games_page.search.trim().to_lowercase();
        let mut listed: Vec<&Title> = self
            .config
            .titles
            .iter()
            .filter(|t| (self.games_page.show_hidden || !t.hidden) && t.name.to_lowercase().contains(&query))
            .collect();
        listed.sort_by_cached_key(|t| (!t.installed, t.name.to_lowercase()));
        listed
    }

    pub(super) fn view_games_page(&self) -> Element<'_, Message> {
        let page = &self.games_page;
        let listed = self.listed_titles();
        let status = if page.importing {
            "Looking through Steam, Heroic and Lutris…".to_string()
        } else if self.config.titles.is_empty() {
            "No games yet. Import games to list what Steam, Heroic and Lutris have installed.".to_string()
        } else {
            format!("{} games. Play starts one through its store.", self.config.titles.iter().filter(|t| t.installed && !t.hidden).count())
        };
        let mut content = column![
            row![
                text("Games").size(22),
                space::horizontal(),
                checkbox(page.show_hidden).label("Show hidden").on_toggle(Message::ToggleHiddenTitles),
                button(text("Import games")).style(style::secondary).on_press_maybe((!page.importing).then_some(Message::ImportTitles)),
                button(text("+ Add game")).style(style::secondary).on_press(Message::ToggleAddGameHelp),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            note(status),
        ]
        .spacing(10);
        if page.help {
            content = content.push(container(note(ADD_GAME_HELP)).padding(12).style(style::card));
        }
        if !self.config.titles.is_empty() {
            content = content.push(field("Search games…", &page.search).on_input(Message::SetTitleSearch));
            content = content.push(if listed.is_empty() {
                Element::from(note("No game matches."))
            } else {
                self.title_rows(&listed)
            });
            let shown = page.selected.as_ref().and_then(|id| self.config.titles.iter().find(|t| &t.id == id));
            if let Some(title) = shown {
                content = content.push(self.title_details(title));
            }
        }
        content.into()
    }

    fn title_rows<'a>(&'a self, listed: &[&'a Title]) -> Element<'a, Message> {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a non-negative row count, rounded down")]
        let at = |y: f32| (y.max(0.0) / ROW_HEIGHT) as usize;
        let first = at(self.games_page.top).saturating_sub(OVERSCAN).min(listed.len());
        let last = (at(self.games_page.top + LIST_HEIGHT) + 1 + OVERSCAN).min(listed.len());
        let mut rows = column![space().height(ROW_HEIGHT * first as f32)];
        for title in &listed[first..last] {
            let selected = self.games_page.selected.as_deref() == Some(title.id.as_str());
            let setup = if title.setup.is_empty() { String::new() } else { title.setup.clone() };
            let mut line = row![
                text(title.source.label()).size(12).color(MUTED_COLOR).width(60),
                text(title.name.as_str()).size(15).width(Length::Fill),
                text(setup).size(13).color(MUTED_COLOR),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            match self.launch_of(&title.id) {
                Some(true) => line = line.push(text("running").size(12).color(style_accent())),
                Some(false) => line = line.push(text("starting…").size(12).color(style_accent())),
                None if !title.installed => line = line.push(text("not installed").size(12).color(MUTED_COLOR)),
                None => {}
            }
            let pick = button(line).width(Length::Fill).padding([6, 10]).style(style::segment(selected)).on_press(Message::SelectTitle(title.id.clone()));
            let play = self.play_button(title);
            rows = rows.push(container(row![pick, play].spacing(8).align_y(Alignment::Center)).center_y(ROW_HEIGHT));
        }
        rows = rows.push(space().height(ROW_HEIGHT * (listed.len() - last) as f32));
        scrollable(rows.width(Length::Fill).padding(iced::Padding::ZERO.right(14)))
            .id(LIST_ID)
            .on_scroll(|view| Message::TitlesScrolled(view.absolute_offset().y))
            .height((ROW_HEIGHT * listed.len() as f32).min(LIST_HEIGHT))
            .into()
    }

    /// The picked game: which setup plays it, and whether it's listed.
    fn title_details<'a>(&'a self, title: &'a Title) -> Element<'a, Message> {
        let mut choices = vec![SetupChoice(None)];
        choices.extend(self.config.games.iter().map(|g| SetupChoice(Some(g.name.clone()))));
        let current = SetupChoice((!title.setup.is_empty()).then(|| title.setup.clone()));
        let id = title.id.clone();
        let mut setup = row![
            text("Setup").size(14),
            dropdown(choices, Some(current), move |c: SetupChoice| Message::SetTitleSetup(id.clone(), c.0.unwrap_or_default())),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        // A built-in game that looks like this one, when none of the user's setups plays it.
        let offer = self.games_page.library.get(&title.id).filter(|_| title.setup.is_empty()).and_then(|i| self.library.get(*i).map(|l| (*i, l)));
        if let Some((i, listing)) = offer {
            setup = setup.push(button(text(format!("Add setup: {}", listing.name))).style(style::secondary).on_press(Message::PreviewLibrary(i)));
        }
        let hidden = title.hidden;
        let id = title.id.clone();
        let (hide_id, env_id) = (title.id.clone(), title.id.clone());
        let env_text = match &self.games_page.env_draft {
            Some((draft_id, text)) if *draft_id == title.id => text.clone(),
            _ => title.env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" "),
        };
        let line = launch_line(title);
        let wrapper = column![
            text("Start through Padwight").size(14),
            note("Put this line in the game's launch options (Steam), wrapper setting (Heroic) or command prefix (Lutris). It lets Padwight follow the whole game and apply the settings below. Without it they do nothing."),
            row![
                container(text(line.clone()).size(13)).padding([4, 8]).style(style::card),
                button(text("Copy").size(13)).style(style::secondary).on_press(Message::CopyLaunchLine(line)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            checkbox(title.hide_controllers).label("Hide real controllers from the game").on_toggle(move |on| Message::SetTitleHideControllers(hide_id.clone(), on)),
            field("Variables to set, e.g. PROTON_LOG=1 SDL_JOYSTICK_HIDAPI=0", &env_text).on_input(move |text| Message::SetTitleEnv(env_id.clone(), text)),
        ]
        .spacing(8);
        container(
            column![
                text(title.name.as_str()).size(18),
                setup,
                row![
                    checkbox(hidden).label("Hide from the list").on_toggle(move |on| Message::SetTitleHidden(id.clone(), on)),
                    space::horizontal(),
                    note(format!("{} · {}", title.source.label(), title.link())),
                    self.play_button(title),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                wrapper,
            ]
            .spacing(10),
        )
        .padding(12)
        .style(style::card)
        .into()
    }
}

const ADD_GAME_HELP: &str = "Add a game to Steam (Games, then Add a Non-Steam Game), Heroic (Add game) or Lutris, with the Wine or \
Proton version it needs, then press Import games. Padwight starts every game through its store, so the store sets up the game's \
runner and prefix.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Store, gui::tests::*, titles::Scanned};

    fn title(id: &str, name: &str, installed: bool) -> Title {
        Title { id: id.into(), name: name.into(), source: Store::Steam, store_id: "1".into(), runner: String::new(), setup: String::new(), hidden: false, installed, env: Default::default(), hide_controllers: false }
    }

    #[test]
    fn lists_installed_titles_first_and_hides_the_hidden() {
        let mut app = app();
        app.config.titles = vec![title("a", "Zebra", true), title("b", "Apple", false), title("c", "Mango", true), Title { hidden: true, ..title("d", "Aaa", true) }];
        let names = |app: &App| app.listed_titles().iter().map(|t| t.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&app), ["Mango", "Zebra", "Apple"]);
        let _ = app.update(Message::ToggleHiddenTitles(true));
        assert_eq!(names(&app), ["Aaa", "Mango", "Zebra", "Apple"]);
        let _ = app.update(Message::SetTitleSearch("ma".into()));
        assert_eq!(names(&app), ["Mango"]);
    }

    #[test]
    fn launch_settings_are_edited_and_kept_through_an_import() {
        let mut app = app();
        app.config.titles = vec![title("a", "Zebra", true)];
        let _ = app.update(Message::SetTitleHideControllers("a".into(), true));
        let _ = app.update(Message::SetTitleEnv("a".into(), "PROTON_LOG=1 oops FOO= =x".into()));
        let t = &app.config.titles[0];
        assert!(t.hide_controllers);
        assert_eq!(t.env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(), ["FOO=", "PROTON_LOG=1"]);
        let found = Scanned { title: title("a", "Zebra", true), setup: None, library: None };
        let _ = app.update(Message::TitlesScanned(vec![found]));
        assert!(app.config.titles[0].hide_controllers && app.config.titles[0].env.len() == 2);
        assert_eq!(launch_line(&app.config.titles[0]), "padwight run -- %command%");
        let heroic = Title { source: Store::Heroic, id: "heroic-gog-7".into(), ..title("x", "G", true) };
        assert_eq!(launch_line(&heroic), "padwight run --title heroic-gog-7 --");
    }

    #[test]
    fn picks_a_setup_and_survives_an_import() {
        let mut app = app();
        app.config.titles = vec![title("a", "Zebra", true)];
        let _ = app.update(Message::SetTitleSetup("a".into(), "Doom".into()));
        let _ = app.update(Message::SetTitleHidden("a".into(), true));
        let found = Scanned { title: title("a", "Zebra 2", true), setup: None, library: Some(0) };
        let _ = app.update(Message::TitlesScanned(vec![found]));
        let t = &app.config.titles[0];
        assert_eq!((t.name.as_str(), t.setup.as_str(), t.hidden), ("Zebra 2", "Doom", true));
        assert_eq!(app.games_page.library.get("a"), Some(&0));
    }

    #[test]
    fn a_started_game_shows_as_running_and_can_be_stopped() {
        use crate::ipc::{LaunchInfo, Status};
        let mut app = app();
        app.config.titles = vec![title("a", "Zebra", true), title("b", "Mango", true)];
        assert_eq!(app.launch_of("a"), None);
        app.status = Some(Status { launches: vec![LaunchInfo { id: "a".into(), running: false }, LaunchInfo { id: "b".into(), running: true }], ..Default::default() });
        assert_eq!((app.launch_of("a"), app.launch_of("b")), (Some(false), Some(true)));
        // Drawing the page with games starting and running works.
        let _ = app.view_games_page();
    }
}
