//! Made-up overlay frames, one per kind of overlay and menu, for drawing the overlay without a
//! daemon (`padwight debug overlay`). Each is built the way the daemon builds its frames: menus
//! through a menu session, info overlays resolved from their `{tokens}`, logs from input.

use std::time::Instant;

use anyhow::{Result, bail};

use crate::{
    config::{Button, ButtonAction, CarouselControls, Span, Cluster, InfoOverlay, InputLogSettings, Menu, MenuItem, MenuKind, OverlayStyle, Stick},
    engine::Opener,
    info::{self, Glyphs, Live, PadFamily},
    inputlog,
    keyboard::{self, Board},
    media::MediaView,
    menu::MenuSession,
    offer::OfferView,
    overlay::{KeyboardView, OverlayFrame, OverlayView},
};

/// Every scene, with what it shows.
pub const SCENES: [(&str, &str); 20] = [
    ("keyboard", "the basic on-screen keyboard, Shift latched, the cursor on H"),
    ("keyboard-full", "the full on-screen keyboard, Shift latched, the cursor on H"),
    ("numpad", "the on-screen numpad"),
    ("menu-list", "a list menu"),
    ("menu-buttons", "a menu with quick-select buttons"),
    ("menu-grid", "a three-column grid menu"),
    ("menu-radial", "a radial menu"),
    ("menu-radial-boxes", "a radial menu in boxes"),
    ("menu-directional", "a D-pad directional menu"),
    ("menu-face", "a face-button directional menu"),
    ("menu-carousel", "a carousel menu"),
    ("in-game-menu", "the Guide + Start menu"),
    ("media", "the media controls"),
    ("offer", "the offer to add a library game"),
    ("info", "an info overlay with live values and glyphs"),
    ("log", "an input log"),
    ("recording", "the recording badge"),
    ("busy", "an info overlay, an input log and a menu at once"),
    ("guide", "the Guide overlay: its menu, Notes, and Mappings with a held layer's section"),
    ("guide-mode", "the Guide overlay turned to a mode that isn't on"),
];

/// The frame of the scene called `name`.
pub fn frame(name: &str) -> Result<OverlayFrame> {
    let active = |view| OverlayFrame { active: Some(view), ..OverlayFrame::default() };
    Ok(match name {
        "keyboard" => active(OverlayView::Keyboard(keyboard_view(Board::Basic))),
        "keyboard-full" => active(OverlayView::Keyboard(keyboard_view(Board::Full))),
        "numpad" => active(OverlayView::Keyboard(keyboard_view(Board::Numpad))),
        "menu-list" => active(menu(MenuKind::List)),
        "menu-buttons" => active(menu(MenuKind::Buttons)),
        "menu-grid" => active(menu(MenuKind::Grid { columns: 3 })),
        "menu-radial" => active(menu(MenuKind::Radial { stick: Stick::Right, boxes: false })),
        "menu-radial-boxes" => active(menu(MenuKind::Radial { stick: Stick::Right, boxes: true })),
        "menu-directional" => active(menu(MenuKind::Directional { cluster: Cluster::DPad })),
        "menu-face" => active(menu(MenuKind::Directional { cluster: Cluster::FaceButtons })),
        "menu-carousel" => active(menu(MenuKind::Carousel { controls: CarouselControls::Bumpers })),
        "in-game-menu" => active(OverlayView::Menu(crate::system_menu::main_page(OverlayStyle::default(), 0))),
        "media" => active(OverlayView::Media(MediaView::sample(OverlayStyle::media()))),
        "offer" => active(OverlayView::Offer(offer())),
        "info" => OverlayFrame { info: vec![info_view()], ..OverlayFrame::default() },
        "log" => OverlayFrame { logs: vec![log_view()], ..OverlayFrame::default() },
        "recording" => OverlayFrame { info: vec![info::recording_view()], ..OverlayFrame::default() },
        "busy" => OverlayFrame { info: vec![info_view()], logs: vec![log_view()], ..active(menu(MenuKind::Buttons)) },
        "guide" => guide(0),
        "guide-mode" => guide(1),
        _ => bail!("no scene called {name} (see `padwight debug overlay list`)"),
    })
}

fn keyboard_view(board: Board) -> KeyboardView {
    let (style, cursor, latched) = match board {
        Board::Basic | Board::Full => (OverlayStyle::keyboard(), keyboard::find(board, "KEY_H").unwrap_or_default(), vec!["KEY_LEFTSHIFT".into()]),
        Board::Numpad => (OverlayStyle::numpad(), board.home(), Vec::new()),
    };
    KeyboardView { board, style, cursor, latched, pressed: None, closing: 0.0, caps: false, typed: String::new() }
}

/// A menu of this kind, open with the cursor on its second item: keys, a toggle, a submenu, a
/// mouse click and an overlay.
fn menu(kind: MenuKind) -> OverlayView {
    let item = |label: &str, action: ButtonAction, button: Option<Button>| MenuItem { label: label.into(), action, button, weight: 1.0, span: Span::ONE };
    let keys = |k: &str| ButtonAction::Keys(vec![k.into()]);
    let items = vec![
        item("Quick save", keys("KEY_F5"), Some(Button::South)),
        item("Crouch", ButtonAction::toggle(keys("KEY_LEFTCTRL")), Some(Button::East)),
        item("Weapons", ButtonAction::OpenMenu("Weapons".into()), Some(Button::West)),
        item("Fire", ButtonAction::Mouse(crate::config::MouseButton::Left), Some(Button::North)),
        item("Keyboard", ButtonAction::ToggleOverlay, Some(Button::LeftBumper)),
    ];
    let menus = vec![
        Menu { name: "Actions".into(), kind, items, cancel: None, no_cancel: false, start: 0, style: OverlayStyle::default() },
        Menu { name: "Weapons".into(), kind: MenuKind::List, items: vec![item("Pistol", keys("KEY_1"), None)], cancel: None, no_cancel: false, start: 0, style: OverlayStyle::default() },
    ];
    let opener = Opener { buttons: vec![Button::LeftBumper], ..Opener::default() };
    let session = MenuSession::open(&menus, "Actions", opener).expect("the sample menu has items");
    let mut view = session.view(&menus).expect("an open menu has a view");
    // Moving there by input would differ by kind; the drawing only reads the selection.
    view.selected = Some(1);
    OverlayView::Menu(view)
}

/// The Guide overlay over a profile with a Driving mode and a Hotkeys layer, its Mappings
/// showing mode `shown` (0 is the default) while the default is on.
fn guide(shown: usize) -> OverlayFrame {
    use crate::config::{Layer, Mode, Profile};
    let keys = |k: &str| ButtonAction::Keys(vec![k.into()]);
    let mut p = Profile::passthrough("Gameplay");
    for b in Button::ALL {
        p.set_button(b, ButtonAction::Disabled);
    }
    p.set_button(Button::South, keys("KEY_SPACE"));
    p.set_button(Button::East, keys("KEY_LEFTCTRL"));
    p.set_button(Button::West, keys("KEY_R"));
    p.set_button(Button::North, ButtonAction::Mode("Driving".into()));
    p.set_button(Button::LeftBumper, ButtonAction::Layer("Hotkeys".into()));
    p.guide.notes = "Hold LB for the hotkeys.".into();
    let mut driving = Mode::new("Driving");
    driving.buttons.insert(Button::South, keys("KEY_W"));
    driving.buttons.insert(Button::East, keys("KEY_S"));
    driving.buttons.insert(Button::LeftBumper, keys("KEY_H"));
    let mut hotkeys = Layer::new("Hotkeys");
    hotkeys.buttons.insert(Button::South, keys("KEY_F1"));
    hotkeys.buttons.insert(Button::East, keys("KEY_F2"));
    let (modes, layers) = ([driving], [hotkeys]);
    let view = info::GuideView { profile: &p, modes: &modes, layers: &layers, shown, current: 0, page: 0 };
    let live = Live::sample(PadFamily::Xbox);
    let info = view.overlays().iter().map(|o| info::resolve(o, &live)).collect();
    let menu = crate::guide_menu::view(false, 0, "Gameplay", OverlayStyle::default(), &[]);
    OverlayFrame { info, ..OverlayFrame { active: Some(OverlayView::Menu(menu)), ..OverlayFrame::default() } }
}

fn offer() -> OfferView {
    OfferView {
        title: "Add Deus Ex?".into(),
        detail: "The library has controls for this game.".into(),
        answers: vec![("A".into(), "Add".into()), ("B".into(), "Not now".into()), ("Y".into(), "Never".into())],
        hint: "A add · B not now · Y never".into(),
        style: OverlayStyle::default(),
    }
}

/// An info overlay using most kinds of token, with the sample live values the settings show.
fn info_view() -> crate::info::InfoView {
    let overlay: InfoOverlay = toml::from_str(
        r#"
        name = "Stats"
        title = "{controller} {profile}"
        rows = [
            ["{time}", "{date}"],
            ["{cpu}", "{ram}"],
            ["{system_battery:icon}", "{wifi}"],
            ["{south} Jump", "{lt} Aim"],
            ["{keyboard:leftshift} Run", "{mouse:leftclick} Fire"],
            ["{dpad} Items", "{l3} Sprint"],
        ]
        "#,
    )
    .expect("the sample info overlay parses");
    info::resolve(&overlay, &Live::sample(PadFamily::Xbox))
}

fn log_view() -> inputlog::LogView {
    let sample = inputlog::Inputs::sample();
    let now = sample.now.unwrap_or_else(Instant::now);
    let glyphs = Glyphs { family: PadFamily::Xbox, nintendo_layout: false };
    inputlog::log_view(&sample.entries(None), &InputLogSettings::default(), &OverlayStyle::log(), glyphs, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scene_has_a_frame() {
        for (name, _) in SCENES {
            frame(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        assert!(frame("nothing").is_err());
    }
}
