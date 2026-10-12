//! `padwight debug gui`: draws the settings window to PNG files with no display and no daemon,
//! for the screenshots in the manual. The window is driven by a list of steps (a page, a tab,
//! a game from the library) and saves a picture at each `shot`. Nothing is read from or written to
//! the user's own config.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use iced::{Color, Size};

use super::*;

pub const USAGE: &str = "
Drawing the settings window without a display or a daemon (for the manual's screenshots):
  gui [options] <step>...        run the steps in order, saving a picture at each `shot`
  steps: library <name>          add that library game        game <name>   show a game's page
         page <overview|settings|manual|general>              profile <name>  edit that profile
         tab <profiles|modes|layers|macros|menus|info|logs|details>
         sub <buttons|sticks|combos|gyro|touchpad|guide>      chapter <file.md>  open a manual chapter
         demo                    add a macro and an info overlay to the game shown
         settings <general|shortcuts>   the App settings tab    shortcuts-demo  move the opener to Start + Select and Tab to Start
         size <w>x<h>            the size from here on
         picture <hide|show>     the live controller drawing
         open <button>           open that button's row (South, LeftBumper, ...)
         expand                  open every row               macro|menu|info <n>  open that card
         dialog <add|export>     open a dialog                close  close it
         shot <name>             save <out>/<name>.png
  options: --out <dir> (default screenshots/manual)   --size <w>x<h> (default 1280x2600)
           --dark (default is light)";

struct Options {
    out: PathBuf,
    size: Size,
    dark: bool,
}

pub fn run(args: &[&str]) -> Result<()> {
    let mut options = Options { out: PathBuf::from("screenshots/manual"), size: Size::new(1280.0, 2600.0), dark: false };
    let mut steps = Vec::new();
    let mut it = args.iter().copied();
    while let Some(arg) = it.next() {
        match arg {
            "--out" => options.out = PathBuf::from(it.next().context("--out needs a folder")?),
            "--size" => {
                let v = it.next().context("--size needs <w>x<h>")?;
                let (w, h) = v.split_once('x').with_context(|| format!("--size takes <w>x<h>, not {v}"))?;
                options.size = Size::new(w.parse()?, h.parse()?);
            }
            "--dark" => options.dark = true,
            _ => steps.push(arg),
        }
    }
    let mut app = new_app(options.dark);
    let mut steps = steps.into_iter();
    while let Some(step) = steps.next() {
        run_step(&mut app, &mut options, step, &mut steps)?;
    }
    Ok(())
}

/// The window as it starts: no config of the user's, a controller connected, no daemon.
fn new_app(dark: bool) -> App {
    let mut app = App::boot().0;
    app.config.appearance = if dark { Appearance::Dark } else { Appearance::Light };
    app.status = Some(crate::ipc::Status {
        enabled: true,
        devices: vec![crate::ipc::DeviceInfo {
            name: "Xbox Wireless Controller".into(),
            path: String::new(),
            managed: true,
            ignored: false,
            analog_triggers: true,
            rumble: true,
            gyro: true,
            paddles: false,
            touchpad: true,
            family: None,
            model: None,
        }],
        ..Default::default()
    });
    // The library list says "Built-in games", not that it's still looking for installed ones.
    send(&mut app, Message::Installed(crate::launchers::Installed::default()));
    app
}

/// One step: `step`, and the values it takes from `steps`.
fn run_step<'a>(app: &mut App, options: &mut Options, step: &str, steps: &mut impl Iterator<Item = &'a str>) -> Result<()> {
    let mut arg = || steps.next().with_context(|| format!("{step} needs a value"));
    match step {
        "library" => {
            let name = arg()?;
            let entry = app.library.iter().find(|e| e.pack.pack.name.eq_ignore_ascii_case(name)).with_context(|| format!("no library pack {name:?}"))?;
            let game = entry.pack.to_game();
            app.config.games.push(game);
            app.saved = app.config.clone();
        }
        "demo" => demo(app),
        "settings" => settings_tab(app, arg()?)?,
        "shortcuts-demo" => shortcuts_demo(app),
        "page" => send(app, Message::SelectPage(page_named(arg()?)?)),
        "game" => {
            let name = arg()?;
            let game = app.config.games.iter().find(|g| g.name.eq_ignore_ascii_case(name)).with_context(|| format!("no game {name:?}"))?.name.clone();
            send(app, Message::SelectPage(Page::Game(Some(game))));
        }
        "profile" => send(app, Message::EditProfile(arg()?.to_string())),
        "tab" => {
            let word = arg()?;
            let tab = *GameTab::ALL.iter().find(|t| tab_word(**t) == word).with_context(|| format!("no tab {word:?}"))?;
            send(app, Message::SelectGameTab(tab));
        }
        "sub" => send(app, Message::SelectProfileTab(profile_tab_named(arg()?)?)),
        "chapter" => {
            let file = arg()?;
            app.manual_chapter = manual::chapter_of(file).with_context(|| format!("no chapter {file:?}"))?;
            app.page = Page::Manual;
        }
        "size" => {
            let v = arg()?;
            let (w, h) = v.split_once('x').with_context(|| format!("size takes <w>x<h>, not {v}"))?;
            options.size = Size::new(w.parse()?, h.parse()?);
        }
        "picture" => {
            // `hide` or `show` the live controller drawing at the top of a profile.
            let hide = arg()? == "hide";
            if app.picture_hidden != hide {
                send(app, Message::TogglePicture);
            }
        }
        "open" => {
            let name = arg()?;
            let button: Button = serde_json::from_str(&format!("\"{name}\"")).with_context(|| format!("no button {name:?} (South, LeftBumper, DpadUp, ...)"))?;
            send(app, Message::ToggleExpanded(Target::Button(button)));
        }
        "expand" => send(app, Message::ExpandAll(true)),
        "macro" => send(app, Message::ToggleMacro(arg()?.parse()?)),
        "menu" => send(app, Message::ToggleMenu(arg()?.parse()?)),
        "info" => send(app, Message::ToggleInfo(arg()?.parse()?)),
        "dialog" => match arg()? {
            "add" => send(app, Message::OpenAddGame),
            "export" => send(app, Message::OpenExport),
            other => bail!("no dialog {other:?}"),
        },
        "close" => send(app, Message::CloseDialog),
        "shot" => take_shot(app, options, arg()?)?,
        other => bail!("unknown step {other:?}{USAGE}"),
    }
    Ok(())
}

/// The App settings page's tab named `word`.
fn settings_tab(app: &mut App, word: &str) -> Result<()> {
    let tab = match word {
        "general" => shortcuts_tab::SettingsTab::General,
        "shortcuts" => shortcuts_tab::SettingsTab::Shortcuts,
        other => bail!("no settings tab {other:?}"),
    };
    send(app, Message::SelectSettingsTab(tab));
    Ok(())
}

/// Moves a few shortcuts, so the tab has its warnings to show.
fn shortcuts_demo(app: &mut App) {
    use crate::config::{Shortcut, ShortcutInput};
    let opener = ShortcutInput::combo(&[Button::Start, Button::Select]);
    send(app, Message::SetShortcut(Shortcut::OpenGuide, Some(opener)));
    send(app, Message::SetShortcut(Shortcut::Tab, Some(ShortcutInput::button(Button::Start))));
    send(app, Message::SetShortcut(Shortcut::Screenshot, Some(ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap))));
}

fn send(app: &mut App, message: Message) {
    let _ = app.update(message);
}

fn tab_word(tab: GameTab) -> &'static str {
    match tab {
        GameTab::Profiles => "profiles",
        GameTab::Modes => "modes",
        GameTab::Layers => "layers",
        GameTab::Macros => "macros",
        GameTab::Menus => "menus",
        GameTab::Info => "info",
        GameTab::Logs => "logs",
        GameTab::Details => "details",
    }
}

/// What the macro and overlay tutorials build, added to the setup on screen.
fn demo(app: &mut App) {
    let game = app.game_mut();
    game.macros.push(Macro {
        name: "Quick turn".into(),
        steps: vec![
            MacroStep::Tap { action: ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]), hold_ms: 40 },
            MacroStep::Wait(50),
            MacroStep::Tap { action: ButtonAction::Mouse(MouseButton::Right), hold_ms: 40 },
        ],
    });
    game.info.push(InfoOverlay {
        name: "Cheat sheet".into(),
        always: true,
        on_start: None,
        linger: None,
        title: Some("Controls".into()),
        current_input: Default::default(),
        style: OverlayStyle::info(),
        rows: vec![vec!["{south}".into(), "Attack".into()], vec!["{east}".into(), "Cancel".into()], vec!["{lb}".into(), "Control groups".into()]],
    });
    app.saved = app.config.clone();
}

fn page_named(word: &str) -> Result<Page> {
    Ok(match word {
        "overview" => Page::Overview,
        "settings" => Page::Settings,
        "manual" => Page::Manual,
        "general" => Page::Game(None),
        other => bail!("no page {other:?}"),
    })
}

fn profile_tab_named(word: &str) -> Result<ProfileTab> {
    Ok(match word {
        "buttons" => ProfileTab::Buttons,
        "sticks" => ProfileTab::Sticks,
        "combos" => ProfileTab::Combos,
        "gyro" => ProfileTab::Gyro,
        "touchpad" => ProfileTab::Touchpad,
        "guide" => ProfileTab::Guide,
        other => bail!("no profile tab {other:?}"),
    })
}

/// Draws the window to `<out>/<name>.png`, and prints where.
#[expect(clippy::print_stdout, reason = "CLI output")]
fn take_shot(app: &mut App, options: &Options, name: &str) -> Result<()> {
    let path = options.out.join(format!("{name}.png"));
    app.saved = app.config.clone();
    let theme = app.theme();
    crate::snapshot::render_themed(app.view(), options.size, 1.0, Color::WHITE, &theme)?.save(&path)?;
    println!("{}", path.display());
    Ok(())
}
