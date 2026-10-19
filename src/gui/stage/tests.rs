use iced::{Rectangle, Size};

use super::snap::{Gesture, Grip};
use super::*;
use crate::gui::tests::{app, with_game};

const DISPLAY: Size = Size::new(2000.0, 1000.0);

fn open_on_a_menu() -> App {
    let mut app = app();
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Menu(0))));
    app
}

fn drag(app: &mut App, grip: Grip, from: Rectangle, to: Rectangle) {
    let _ = app.update(Message::Stage(StageMessage::Gesture(Gesture { grip, from, to, display: DISPLAY, free: false })));
}

#[test]
fn dragging_on_the_stage_edits_the_overlay_and_undoes_like_any_edit() {
    let mut app = open_on_a_menu();
    assert!(app.stage.is_some());
    let from = Rectangle { x: 900.0, y: 400.0, width: 200.0, height: 200.0 };
    drag(&mut app, Grip::Move, from, Rectangle { x: 40.0, y: 40.0, ..from });
    let style = &app.config.general.menus[0].style;
    assert_eq!((style.position, style.x_offset, style.y_offset), (ScreenPosition::TopLeft, 0.0, 0.0), "snapped home to the corner");
    drag(&mut app, Grip::SE, Rectangle { x: 40.0, y: 40.0, ..from }, Rectangle { x: 40.0, y: 40.0, width: 320.0, height: 200.0 });
    assert_eq!(app.config.general.menus[0].style.width, Some(15.6), "the right edge snapped from 360 to 352 px");
    let _ = app.update(Message::Stage(StageMessage::FitContents));
    assert_eq!(app.config.general.menus[0].style.width, None);
    let _ = app.update(Message::Undo);
    assert_eq!(app.config.general.menus[0].style.position, ScreenPosition::Center, "one drag run undoes as one step");
    assert!(app.config != app.saved, "nothing is saved until Save & apply");
}

#[test]
fn the_stage_draws_every_kind_of_overlay() {
    let mut app = open_on_a_menu();
    let _ = app.update(Message::NewInfo);
    for target in [StageTarget::Menu(0), StageTarget::Info(0), StageTarget::Keyboard, StageTarget::Numpad, StageTarget::Media, StageTarget::InGameMenu] {
        let _ = app.update(Message::Stage(StageMessage::Open(target)));
        for tab in [StageTab::Layout, StageTab::Look, StageTab::Content] {
            let _ = app.update(Message::Stage(StageMessage::Tab(tab)));
            let _ = app.view_stage();
        }
    }
    let _ = app.update(Message::Stage(StageMessage::Fold));
    let _ = app.view_stage();
}

#[test]
fn the_stage_closes_when_its_overlay_goes() {
    let mut app = open_on_a_menu();
    let _ = app.update(Message::DeleteMenu(0));
    assert!(app.stage.is_none());
    let mut app = with_game();
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Menu(0))));
    let _ = app.update(Message::SelectPage(Page::Game(None)));
    assert!(app.stage.is_none(), "another setup's menus are another's");
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Keyboard)));
    let _ = app.update(Message::SelectPage(Page::Overview));
    assert!(app.stage.is_some(), "the keyboard is the app's, on any page");
}

#[test]
fn escape_closes_only_from_the_stage() {
    let mut app = open_on_a_menu();
    let main = app.main_window;
    let _ = app.update(Message::Stage(StageMessage::Key(main, StageKey::Escape)));
    assert!(app.stage.is_some());
    let stage = app.stage.as_ref().map(|s| s.window).unwrap();
    let _ = app.update(Message::Stage(StageMessage::Key(stage, StageKey::Escape)));
    assert!(app.stage.is_none());
}
