//! Pictures of menus part way through fading in, drawn in-process by the renderer the overlay
//! uses (wgpu where there's a GPU, tiny-skia otherwise), so a fade can be checked without a
//! compositor in the way. Opt-in: `PADWIGHT_FADE_FRAMES=<dir> cargo test fade_frames --
//! --ignored` writes `<scene>-<bg>-<opacity>.png` for each scene, over a black and a light
//! background, for `scripts/harness/fade-check.py --frames <dir>` to compare.

use iced::{Color, Element, Size, Theme, widget::container};

use crate::{
    config::{ButtonAction, Button, Cluster, Menu, MenuItem, MenuKind, OverlayStyle, Stick},
    engine::Opener,
    menu::{MenuSession, MenuView},
    motion::{Anim, MotionStyle, Timing},
    overlay::{draw, fit::Fit},
};

fn menu(name: &str, kind: MenuKind, labels: &[&str]) -> Menu {
    let items = labels.iter().enumerate().map(|(i, l)| MenuItem::new(*l, ButtonAction::Keys(vec![format!("KEY_{}", i + 1)]))).collect();
    Menu { name: name.into(), kind, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() }
}

fn scenes() -> Vec<(&'static str, MenuView)> {
    let menus = [
        menu("Radial", MenuKind::Radial { stick: Stick::Right, boxes: false }, &["Item 1", "Item 2", "Item 3", "Item 4"]),
        menu("Boxes", MenuKind::Radial { stick: Stick::Right, boxes: true }, &["Item 1", "Item 2", "Item 3", "Item 4"]),
        menu("Directional", MenuKind::Directional { cluster: Cluster::DPad }, &["1", "2", "3", "4"]),
        menu("Faces", MenuKind::Directional { cluster: Cluster::FaceButtons }, &["Heal", "Reload", "Grenade", "Use"]),
        menu("List", MenuKind::List, &["Resume", "Settings", "Quit"]),
    ];
    let held = Opener { buttons: vec![Button::LeftBumper], ..Opener::default() };
    let view = |name: &str| MenuSession::open(&menus, name, held.clone()).and_then(|s| s.view(&menus)).expect("the menu opens");
    let mut aimed = view("Radial");
    aimed.selected = Some(1);
    vec![
        ("radial", view("Radial")),
        ("radial-aimed", aimed),
        ("boxes", view("Boxes")),
        ("dpad", view("Directional")),
        ("faces", view("Faces")),
        ("list", view("List")),
    ]
}

/// The menu as the overlay draws it at `opacity` of the way into its fade-in under Subtle, whose
/// fade is linear.
fn panel<'a>(m: &MenuView, opacity: f32) -> Element<'a, ()> {
    let feel = MotionStyle::Subtle.feel();
    let anim = Anim { feel, timing: Timing { opened: opacity * feel.open, moved: 10.0, ..Timing::default() } };
    let look = draw::MenuLook { colorblind: false, family: crate::info::PadFamily::Xbox, nintendo_layout: false, anim };
    let fit = Fit::of(&m.style, Size::new(1024.0, 768.0));
    container(draw::menu_panel(m, crate::font::resolve(None), look, &fit)).center(iced::Length::Fill).into()
}

#[test]
#[ignore = "writes pictures for scripts/harness/fade-check.py"]
fn fade_frames() {
    let dir = std::path::PathBuf::from(std::env::var_os("PADWIGHT_FADE_FRAMES").expect("PADWIGHT_FADE_FRAMES names the folder to write"));
    let backgrounds = [("black", Color::BLACK), ("light", Color::from_rgb8(0xc8, 0xc8, 0xc8))];
    let settings = || iced::Settings { fonts: crate::font::BUNDLED.iter().map(|b| b.bytes.into()).collect(), ..iced::Settings::default() };
    for (name, view) in scenes() {
        for (bg_name, bg) in backgrounds {
            let theme = Theme::custom("fade", iced::theme::Palette { background: bg, ..iced::theme::Palette::DARK });
            for opacity in [0.1, 0.25, 0.5, 0.75, 1.0] {
                let mut sim = iced_test::Simulator::with_size(settings(), Size::new(1024.0, 768.0), panel(&view, opacity));
                let snapshot = sim.snapshot(&theme).expect("draws");
                let path = dir.join(format!("{name}-{bg_name}-{:03}.png", (opacity * 100.0) as u32));
                let _ = std::fs::remove_file(&path);
                snapshot.matches_image(&path).expect("writes the picture");
            }
        }
    }
}
