//! Made-up overlay frames, one per kind of overlay and menu, for drawing the overlay without a
//! daemon (`padwight debug overlay`). Each is built the way the daemon builds its frames: menus
//! through a menu session, info overlays resolved from their `{tokens}`, logs from input.

use std::time::Instant;

use anyhow::{Result, bail};

mod list;
mod styles;

pub use list::SCENES;

use crate::{
    config::{
        Button, ButtonAction, CarouselControls, Cluster, InfoOverlay, InputLogSettings, Menu, MenuItem, MenuKind, OverlayStyle, Span, Stick,
    },
    engine::Opener,
    info::{self, Glyphs, Live, PadFamily},
    inputlog,
    keyboard::{self, Board},
    media::MediaView,
    menu::MenuSession,
    overlay::{KeyboardView, OverlayFrame, OverlayView},
};


/// The frame of the scene called `name`.
pub fn frame(name: &str) -> Result<OverlayFrame> {
    let active = |view| OverlayFrame { active: Some(view), ..OverlayFrame::default() };
    if let Some(style) = styles::list_scene(name) {
        return Ok(active(menu_in(MenuKind::List, style, false)));
    }
    Ok(match name {
        "keyboard" => active(OverlayView::Keyboard(keyboard_view(Board::Basic))),
        "keyboard-full" => active(OverlayView::Keyboard(keyboard_view(Board::Full))),
        "keyboard-plaid" => active(OverlayView::Keyboard(KeyboardView { style: styles::plaid_items(), ..keyboard_view(Board::Full) })),
        "numpad" => active(OverlayView::Keyboard(keyboard_view(Board::Numpad))),
        "menu-list" => active(menu(MenuKind::List)),
        "menu-buttons" => active(menu(MenuKind::Buttons)),
        "menu-grid" => active(menu(MenuKind::Grid { columns: 3 })),
        "menu-radial" => active(menu(MenuKind::Radial { stick: Stick::Right, boxes: false })),
        "menu-radial-boxes" => active(menu(MenuKind::Radial { stick: Stick::Right, boxes: true })),
        "menu-radial-plaid" => active(menu_in(MenuKind::Radial { stick: Stick::Right, boxes: false }, styles::plaid_items(), false)),
        "menu-directional" => active(menu(MenuKind::Directional { cluster: Cluster::DPad })),
        "menu-face" => active(menu(MenuKind::Directional { cluster: Cluster::FaceButtons })),
        "menu-carousel" => active(menu(MenuKind::Carousel { controls: CarouselControls::Bumpers })),
        "menu-styled" => active(menu_in(MenuKind::List, styles::ornate(), true)),
        "in-game-menu" => active(OverlayView::Menu(crate::system_menu::main_page(OverlayStyle::default(), 0))),
        "media" => active(OverlayView::Media(MediaView::sample(OverlayStyle::media()))),
        "offer" => active(offer()),
        "library-hint" => {
            let toast = crate::offer::hint("Deus Ex", Instant::now());
            OverlayFrame { info: toast.view(Instant::now(), crate::info::Glyphs::default()).into_iter().collect(), ..OverlayFrame::default() }
        }
        "info" => OverlayFrame { info: vec![info_view()], ..OverlayFrame::default() },
        "info-empty" => OverlayFrame { info: vec![blank_info()], ..OverlayFrame::default() },
        "info-formatted" => OverlayFrame {
            info: vec![formatted_view()],
            pack_icons: Some(std::collections::BTreeMap::from([("flame".to_string(), FLAME.to_string())])),
            ..OverlayFrame::default()
        },
        "log" => OverlayFrame { logs: vec![log_view()], ..OverlayFrame::default() },
        "recording" => OverlayFrame { info: vec![info::recording_view()], ..OverlayFrame::default() },
        "busy" => OverlayFrame { info: vec![info_view()], logs: vec![log_view()], ..active(menu(MenuKind::Buttons)) },
        "unsaved" => active(unsaved()),
        "quick-styled" => active(quick_styled()),
        "held-remapping" => active(held(crate::system_menu::Hold::RemappingOff)),
        "guide" => guide(0),
        "guide-mode" => guide(1),
        "tools" => active(tools(false)),
        "calculator" => active(tools(true)),
        "doodles" => doodles(),
        "doodle-typing" => doodle_typing(),
        "switch-profile" => active(switch_profile(false)),
        "new-profile" => active(switch_profile(true)),
        "edit-controls" => active(edit_controls(&[])),
        "edit-stick" => active(edit_controls(&[Button::DpadDown, Button::South, Button::DpadDown, Button::South])),
        "edit-action" => active(edit_controls(&[Button::South, Button::South])),
        "edit-gyro" => active(edit_controls(&[Button::DpadDown, Button::DpadDown, Button::DpadDown, Button::South])),
        "edit-layer" => active(edit_controls(&[[Button::DpadDown; 7].as_slice(), &[Button::South, Button::South]].concat())),
        "edit-typing" => active(edit_controls(&[[Button::DpadDown; 8].as_slice(), &[Button::South, Button::South, Button::South]].concat())),
        "edit-game" => active(edit_controls(&[Button::DpadUp, Button::South])),
        "edit-trigger" => active(edit_controls(&[Button::DpadDown, Button::DpadDown, Button::South, Button::South, Button::DpadRight, Button::DpadRight])),
        "edit-touchpad" => active(edit_controls(&[[Button::DpadDown; 4].as_slice(), &[Button::South]].concat())),
        "edit-menu" => active(edit_controls(&[[Button::DpadDown; 9].as_slice(), &[Button::South, Button::South]].concat())),
        "edit-look" => active(edit_controls(&[[Button::DpadUp, Button::South].as_slice(), &[Button::DpadDown; 8], &[Button::South, Button::DpadDown, Button::South]].concat())),
        _ => bail!("no scene called {name} (see `padwight debug overlay list`)"),
    })
}

fn keyboard_view(board: Board) -> KeyboardView {
    let (style, cursor, latched) = match board {
        Board::Basic | Board::Full => (OverlayStyle::keyboard(), keyboard::find(board, "KEY_H").unwrap_or_default(), vec!["KEY_LEFTSHIFT".into()]),
        Board::Numpad => (OverlayStyle::numpad(), board.home(), Vec::new()),
    };
    KeyboardView { board, style, cursor, latched, pressed: None, closing: 0.0, caps: false, typed: String::new(), field: None }
}

/// A menu of this kind, open with the cursor on its second item: keys, a toggle, a submenu, a
/// mouse click and an overlay.
fn menu(kind: MenuKind) -> OverlayView {
    menu_in(kind, OverlayStyle::default(), false)
}

/// The sample menu drawn in `style`, with an `{icon:…}` token in front of each label when `icons` is set.
fn menu_in(kind: MenuKind, style: OverlayStyle, icons: bool) -> OverlayView {
    let icon = |name: &str| icons.then(|| name.to_string());
    let item = |label: &str, action: ButtonAction, button: Option<Button>, icon: Option<String>| {
        let label = match icon {
            Some(name) => format!("{{icon:{name}}} {label}"),
            None => label.to_string(),
        };
        MenuItem { label, action, button, weight: 1.0, span: Span::ONE }
    };
    let keys = |k: &str| ButtonAction::Keys(vec![k.into()]);
    let items = vec![
        item("Quick save", keys("KEY_F5"), Some(Button::South), icon("potion")),
        item("Crouch", ButtonAction::toggle(keys("KEY_LEFTCTRL")), Some(Button::East), icon("car")),
        item("Weapons", ButtonAction::OpenMenu("Weapons".into()), Some(Button::West), icon("sword")),
        item("Fire", ButtonAction::Mouse(crate::config::MouseButton::Left), Some(Button::North), icon("pistol")),
        item("Keyboard", ButtonAction::ToggleOverlay, Some(Button::LeftBumper), icon("map")),
    ];
    let menus = vec![
        Menu { name: "Actions".into(), kind, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style },
        Menu { name: "Weapons".into(), kind: MenuKind::List, items: vec![item("Pistol", keys("KEY_1"), None, None)], cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() },
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
    // Each gesture's badge, beside the face buttons it would otherwise be mistaken for.
    let gestures = crate::config::Gestures { double_tap: Some(keys("KEY_E")), triple_tap: Some(keys("KEY_Q")), long_press: Some(keys("KEY_TAB")), ..Default::default() };
    p.gestures.insert(Button::West, gestures);
    // An On release beside the long press, marked as games print it: "Release" before the glyph.
    p.gestures.insert(Button::RightBumper, crate::config::Gestures { on_release: Some(keys("KEY_G")), ..Default::default() });
    // A glyph leading a long note, and one before the last words, show how a note wraps around glyphs.
    p.guide.notes = "{start} backs out of every in-game screen, from the inventory to the keypads and computers.\nHold {lb} for the hotkeys.\nThe driving mode turns the face buttons into pedals, and the bumpers {lb} {rb} into the gears and the lights.".into();
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
    let menu = crate::guide_menu::view(false, 0, "Gameplay", OverlayStyle::default());
    OverlayFrame { info, ..OverlayFrame { active: Some(OverlayView::Menu(menu)), ..OverlayFrame::default() } }
}

/// Doodles over the game: a few strokes in each tip and kind, a text box, and the Doodle cursor
/// on the pen's ring.
fn doodles() -> OverlayFrame {
    use crate::tools::doodle::{Canvas, Kind, Pen, Point, TextBox, TextStyle, Tip};
    let mut canvas = Canvas::default();
    let tips = [Tip::Round, Tip::Square, Tip::Chisel, Tip::Diagonal];
    let kinds = [Kind::Solid, Kind::Brush, Kind::Spray];
    for (row, kind) in kinds.into_iter().enumerate() {
        for (column, tip) in tips.into_iter().enumerate() {
            let y = 0.2 + row as f32 * 0.12;
            let x = 0.1 + column as f32 * 0.2;
            let pen = Pen { color: 2 + column * 2, size: 0.012, tip, kind };
            canvas.begin_stroke(pen, Point::new(x, y));
            for step in 1..=12 {
                let t = step as f32 / 12.0;
                canvas.extend_stroke(Point::new(x + 0.14 * t, y + 0.05 * (t * std::f32::consts::TAU).sin()));
            }
            canvas.end_stroke();
        }
    }
    canvas.add_text(TextBox { at: Point::new(0.1, 0.6), text: "Doodle".into(), style: TextStyle { color: 7, size: 0.05, bold: true, ..TextStyle::default() } });
    let cursor = crate::overlay::DoodleCursor { at: Point::new(0.5, 0.8), radius: 0.006, color: [230, 57, 70] };
    let style = crate::config::OverlayStyle::default();
    let cells = vec![
        crate::overlay::DoodleCell::Palette { current: 2, hint: "{leftright} colour".into() },
        crate::overlay::DoodleCell::Text("{updown} size 14".into()),
        crate::overlay::DoodleCell::Tip { tip: crate::tools::doodle::Tip::Round, text: "{north} Round".into() },
        crate::overlay::DoodleCell::Kind { kind: crate::tools::doodle::Kind::Solid, text: "{west} Solid".into() },
    ];
    let hud = crate::overlay::DoodleHud { tool: 0, tray: true, cells, style };
    OverlayFrame { doodles: canvas, doodle_cursor: Some(cursor), doodle_hud: Some(hud), ..OverlayFrame::default() }
}

/// The text keyboard the Doodle screen opens for a text box, typing "Hello", over the doodles.
fn doodle_typing() -> OverlayFrame {
    let keys = crate::overlay::OverlayController::for_field("Text", "Hello");
    OverlayFrame { active: Some(OverlayView::Keyboard(keys.view(Instant::now()))), ..doodles() }
}

/// The Tools menu: its list, or the calculator with 7 × 6 keyed in.
fn tools(calculator: bool) -> OverlayView {
    use crate::tools::{ToolsSession, doodle::Canvas};
    let (mut session, mut canvas, now) = (ToolsSession::new(), Canvas::default(), std::time::Instant::now());
    if calculator {
        // South opens the calculator; then 7, × (one row down from ÷), 6, and =, the press order below.
        for b in [Button::South, Button::South, Button::DpadRight, Button::DpadRight, Button::DpadRight, Button::DpadDown, Button::South, Button::DpadLeft, Button::South] {
            session.handle(b, true, &mut canvas, now);
        }
    }
    OverlayView::Menu(session.view(OverlayStyle::default()).expect("the Tools menu draws a menu"))
}

/// Switch Profile's list (with the cursor on a profile that needs a gyro), or its new-profile page.
fn switch_profile(new: bool) -> OverlayView {
    use crate::{
        config::{Config, Feature, Game, Profile, ProfileRef},
        input::InputEvent,
        system_menu::{Page, SystemMenu},
    };
    let mut config = Config::default();
    let mut gyro = Profile::gyro_aim("Gyro aim");
    gyro.requires = vec![Feature::Gyro];
    config.games.push(Game::new("Deus Ex", vec![Profile::pc_action("Gameplay"), gyro, Profile::desktop("Menus")]));
    config.active = ProfileRef::new(Some("Deus Ex"), "Gameplay");
    let mut menu = SystemMenu::new(&config).with_features(Some(Vec::new())).on_page(Page::Profiles);
    let now = std::time::Instant::now();
    let presses: &[Button] = if new { &[Button::DpadUp, Button::South] } else { &[Button::DpadDown] };
    for &b in presses {
        menu.handle(InputEvent::Button(b, true), now, &mut config);
    }
    OverlayView::Menu(menu.view(&config))
}

/// Edit Controls over a game with a mouse stick, a layer and a macro, after `presses`.
fn edit_controls(presses: &[Button]) -> OverlayView {
    use crate::{
        config::{Config, Game, Layer, Macro, Profile, ProfileRef},
        input::InputEvent,
        system_menu::{Page, SystemMenu},
    };
    let mut config = Config::default();
    let mut game = Game::new("Deus Ex", vec![Profile::gyro_aim("Gameplay")]);
    game.layers.push(Layer::new("Hotkeys"));
    game.macros.push(Macro { name: "Quick save".into(), steps: Vec::new() });
    let items = ["Heal", "Reload", "Map", "Inventory"].map(|l| crate::config::MenuItem::new(l, crate::config::ButtonAction::Screenshot)).to_vec();
    game.menus.push(crate::config::Menu { name: "Quick".into(), kind: crate::config::MenuKind::Grid { columns: 2 }, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() });
    config.games.push(game);
    config.active = ProfileRef::new(Some("Deus Ex"), "Gameplay");
    let mut menu = SystemMenu::new(&config).on_page(Page::EditControls);
    let now = std::time::Instant::now();
    for &b in presses {
        menu.handle(InputEvent::Button(b, true), now, &mut config);
        menu.handle(InputEvent::Button(b, false), now, &mut config);
    }
    menu.overlay_view(&config, &sample_live())
}

/// A controller with its sticks pushed, a trigger pulled, a finger on the touchpad and the gyro
/// turning, for the editor's live pictures.
fn sample_live() -> crate::figures::Live {
    let mut live = crate::figures::Live { sticks: [(0.45, 0.2), (0.7, -0.35)], triggers: [0.65, 0.2], pad_height: crate::input::DUALSENSE_HEIGHT, ..Default::default() };
    live.touches.insert(0, (0.3, 0.6));
    let t0 = std::time::Instant::now();
    for i in 0..40 {
        let t = i as f32 / 10.0;
        live.turning([40.0 * t.cos(), 60.0 * t.sin(), 0.0], t0 + std::time::Duration::from_millis(20 * i));
    }
    live
}

/// Quick Settings left with a change not saved: the question stands over the page.
fn unsaved() -> OverlayView {
    use crate::{
        config::Config,
        input::InputEvent,
        system_menu::{Page, SystemMenu},
    };
    let mut config = Config::default();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    config.general.nintendo_layout = Some(true);
    menu.handle(InputEvent::Button(Button::East, true), std::time::Instant::now(), &mut config);
    OverlayView::Menu(menu.view(&config))
}

/// Quick Settings in a style whose capitals and letter spacing make its labels long.
fn quick_styled() -> OverlayView {
    use crate::system_menu::{Page, SystemMenu};
    let mut config = crate::config::Config::default();
    config.menu_style = OverlayStyle {
        typography: crate::config::Typography { case: crate::config::Case::Upper, letter_spacing: 3.0, ..Default::default() },
        ..styles::ornate()
    };
    let menu = SystemMenu::new(&config).on_page(Page::Quick);
    OverlayView::Menu(menu.view(&config))
}

/// The in-game menu on hold, `why`, over Quick Settings.
fn held(why: crate::system_menu::Hold) -> OverlayView {
    use crate::system_menu::{Page, SystemMenu};
    let config = crate::config::Config::default();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    menu.hold(why);
    OverlayView::Menu(menu.view(&config))
}

/// The offer as the daemon makes it, for the library's Deus Ex pack.
fn offer() -> OverlayView {
    let deus_ex = crate::library::listings().iter().find(|l| l.name == "Deus Ex").and_then(crate::library::Listing::entry).expect("the library has Deus Ex");
    crate::offer::OfferSession::new(&[deus_ex], &[0], None, Instant::now()).view(&OverlayStyle::default())
}

/// An info overlay with nothing written in it yet, as its preview shows it.
fn blank_info() -> crate::info::InfoView {
    let blank = InfoOverlay {
        name: "Info".into(),
        always: false,
        on_start: None,
        linger: None,
        title: None,
        current_input: Default::default(),
        style: OverlayStyle::info(),
        rows: Vec::new(),
    };
    info::resolve(&blank, &Live::sample(PadFamily::Xbox))
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

/// An icon a pack might carry: it draws in the text's color where it says `currentColor`.
const FLAME: &str = "<svg viewBox='0 0 24 24'><path d='M12 2c1 4 6 6 6 12a6 6 0 0 1-12 0c0-3 2-4 2-7 2 1 3 2 4-5z' fill='currentColor'/></svg>";

/// An info overlay using formatting tags, icons, and the gyro and touchpad tokens.
fn formatted_view() -> crate::info::InfoView {
    let overlay: InfoOverlay = toml::from_str(
        r#"
        name = "Formatted"
        title = "{b}{color:orange}{icon:flame} Quick reference{/color}{/b}"
        rows = [
            ["{icon:sword} {b}Melee{/b}", "{south}"],
            ["{icon:pistol} {i}Fire{/i}", "{rt}"],
            ["{icon:potion} {u}Heal{/u}", "{box}{west}{/box}"],
            ["{color:red}Danger{/color} {color:green}Safe{/color}", "{box}{b}Hold{/b}{/box}"],
            ["{font:Cinzel}Fancy{/font} {font:Press Start 2P}Retro{/font}", "{font:Bangers}Bang!{/font}"],
            ["Pitch {gyro:p}  Roll {gyro:r}  Yaw {gyro:y}", "{trackpad}"],
        ]
        "#,
    )
    .expect("the sample formatted overlay parses");
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
