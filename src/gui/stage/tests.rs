use iced::{Rectangle, Size};

use super::snap::{Gesture, Grip};
use crate::config::ScreenPosition;
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
    for target in [StageTarget::Menu(0), StageTarget::Info(0), StageTarget::Panel(Panel::Keyboard), StageTarget::Panel(Panel::Numpad), StageTarget::Panel(Panel::Media), StageTarget::Panel(Panel::InGameMenu)] {
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
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Panel(Panel::Keyboard))));
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

#[test]
fn a_setups_own_look_is_edited_on_the_stage_and_the_apps_is_left_alone() {
    let mut app = with_game();
    let _ = app.update(Message::SetGameOverlayStyle(crate::keyboard::Layout::Keyboard, Some(app.config.keyboard_style.clone())));
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Setup(Panel::Keyboard))));
    let from = Rectangle { x: 900.0, y: 700.0, width: 200.0, height: 200.0 };
    drag(&mut app, Grip::Move, from, Rectangle { x: 40.0, y: 40.0, ..from });
    assert_eq!(app.game().keyboard_style.as_ref().map(|s| s.position), Some(ScreenPosition::TopLeft));
    assert_ne!(app.config.keyboard_style.position, ScreenPosition::TopLeft, "the App settings look stays");
    let _ = app.view_stage();
    let _ = app.update(Message::SetGameOverlayStyle(crate::keyboard::Layout::Keyboard, None));
    assert!(app.stage.is_none(), "following App settings again leaves nothing of its own to edit");
}

#[test]
fn clicking_an_item_opens_it_in_the_content_tab() {
    let mut app = open_on_a_menu();
    let _ = app.update(Message::AddMenuItem(0));
    let _ = app.update(Message::Stage(StageMessage::Pick(Part::Item(1))));
    let stage = app.stage.as_ref().unwrap();
    assert_eq!((stage.tab, stage.picked), (StageTab::Content, Some(Part::Item(1))));
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Panel(Panel::Keyboard))));
    let _ = app.update(Message::Stage(StageMessage::Pick(Part::Item(0))));
    assert_eq!(app.stage.as_ref().unwrap().picked, None, "the keyboard's keys aren't edited here");
}

#[test]
fn any_other_overlay_can_be_drawn_around_it_and_info_and_logs_are_at_first() {
    let mut app = open_on_a_menu();
    let _ = app.update(Message::NewInfo);
    let _ = app.update(Message::NewInfo);
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Menu(0))));
    let shown = |app: &App| app.shown_around(app.stage.as_ref().unwrap());
    let choices = app.around_choices(StageTarget::Menu(0));
    assert!(!choices.contains(&StageTarget::Menu(0)), "not itself");
    assert!(choices.contains(&StageTarget::Panel(Panel::Keyboard)));
    let _ = app.update(Message::Stage(StageMessage::Close));
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Menu(0))));
    assert_eq!(shown(&app), [StageTarget::Info(0), StageTarget::Info(1)], "info overlays can be up with anything");
    let _ = app.update(Message::Stage(StageMessage::Around(StageTarget::Panel(Panel::Keyboard), true)));
    assert!(shown(&app).contains(&StageTarget::Panel(Panel::Keyboard)));
    let _ = app.view_stage();
    let _ = app.update(Message::Stage(StageMessage::AroundAll(false)));
    assert!(shown(&app).is_empty());
    let _ = app.update(Message::Stage(StageMessage::AroundAll(true)));
    assert_eq!(shown(&app), choices);
    let _ = app.view_stage();
}

#[test]
fn info_overlays_that_stack_with_it_are_not_drawn_around_it() {
    let mut app = open_on_a_menu();
    let _ = app.update(Message::NewInfo);
    let _ = app.update(Message::NewInfo);
    assert!(!app.around_choices(StageTarget::Info(0)).contains(&StageTarget::Info(1)), "it shares the spot, and stacks with it there");
    assert!(app.around_choices(StageTarget::Info(0)).contains(&StageTarget::Menu(0)));
    assert_eq!(app.stacked_with(StageTarget::Info(0)), 1);
    app.game_mut().info[1].style.position = ScreenPosition::BottomLeft;
    assert!(app.around_choices(StageTarget::Info(0)).contains(&StageTarget::Info(1)));
}

#[test]
fn panels_are_drawn_around_it_in_the_look_this_setup_uses() {
    let mut app = with_game();
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    let _ = app.update(Message::SetGameMediaStyle(Some(app.config.media_style.clone())));
    let choices = app.around_choices(StageTarget::Menu(0));
    assert!(choices.contains(&StageTarget::Setup(Panel::Media)) && !choices.contains(&StageTarget::Panel(Panel::Media)));
    let _ = app.update(Message::Stage(StageMessage::Open(StageTarget::Menu(0))));
    let _ = app.update(Message::Stage(StageMessage::Around(StageTarget::Panel(Panel::Media), true)));
    let shown = |app: &App| app.shown_around(app.stage.as_ref().unwrap());
    assert!(shown(&app).contains(&StageTarget::Setup(Panel::Media)), "the media controls, in the look this setup uses");
    let _ = app.update(Message::Stage(StageMessage::Around(StageTarget::Setup(Panel::Media), false)));
    assert!(!shown(&app).contains(&StageTarget::Setup(Panel::Media)));
    let choices = app.around_choices(StageTarget::Panel(Panel::Keyboard));
    assert!(!choices.iter().any(|t| matches!(t, StageTarget::Panel(Panel::Keyboard) | StageTarget::Setup(Panel::Keyboard))), "not itself in another look");
}

#[test]
fn playing_animations_opens_the_overlay_in_its_motion_style() {
    let mut app = open_on_a_menu();
    let still = app.stage_anim(app.stage.as_ref().unwrap());
    assert_eq!((still.clock, still.opacity()), (0.0, 1.0), "drawn still by default");
    let _ = app.update(Message::Stage(StageMessage::Animate(true)));
    let stage = app.stage.as_ref().unwrap();
    assert!(stage.is_ticking(), "the window redraws while it moves");
    let playing = app.stage_anim(stage);
    assert!(playing.opacity() < 0.5, "the opening plays from the start");
    let _ = app.view_stage();
    let _ = app.update(Message::Stage(StageMessage::Animate(false)));
    assert!(!app.stage.as_ref().unwrap().is_ticking());
}
