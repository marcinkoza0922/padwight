//! Pictures of menus part way through fading in, drawn by the renderer the overlay uses, so a
//! fade can be checked without a compositor in the way. Each scene is drawn over a black and a
//! light background at several opacities, and the test asserts the fade is even: no staged
//! drawing, so every pixel is its full-strength color blended over the background by the very
//! same share. `PADWIGHT_FADE_FRAMES=<dir>` also writes `<scene>-<bg>-<opacity>.png` for
//! `scripts/harness/fade-check.py --frames <dir>` to compare.

use iced::Color;

use crate::{
    config::{Button, ButtonAction, Cluster, Menu, MenuItem, MenuKind, OverlayStyle, Stick},
    engine::Opener,
    menu::{MenuSession, MenuView},
    motion::Anim,
    overlay::{
        OverlayFrame, OverlayView,
        render::{Display, Point, Screen, Shown},
    },
    snapshot::Image,
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

/// The menu `opacity` of the way into a linear fade-in, over `background`.
fn at(view: &MenuView, opacity: f32, background: Color) -> Image {
    let frame = OverlayFrame { active: Some(OverlayView::Menu(view.clone())), ..Default::default() };
    let shown = Shown { panels: frame.active.iter().map(|v| (v, Anim::faint(opacity))).collect(), info: Vec::new(), logs: Vec::new() };
    let mut pixmap = tiny_skia::Pixmap::new(1024, 768).expect("a pixmap");
    pixmap.fill(tiny_skia::Color::from_rgba(background.r, background.g, background.b, 1.0).expect("a color"));
    Screen::new().draw(&frame, &shown, Display { size: Point::new(1024.0, 768.0), scale: 1.0 }, &mut pixmap.as_mut());
    let rgba = pixmap.data().as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    Image { width: 1024, height: 768, rgba }
}

#[test]
fn fade_frames() {
    let dir = std::env::var_os("PADWIGHT_FADE_FRAMES").map(std::path::PathBuf::from);
    let backgrounds = [("black", Color::BLACK), ("light", Color::from_rgb8(0xc8, 0xc8, 0xc8))];
    for (name, view) in scenes() {
        for (bg_name, bg) in backgrounds {
            let full = at(&view, 1.0, bg);
            let back = [bg.r, bg.g, bg.b].map(|c| (c * 255.0).round() as u8);
            let ink = full.rgba.as_chunks::<4>().0.iter().filter(|p| p[..3] != back).count();
            assert!(ink > 0, "{name} {bg_name}: the menu draws something");
            for opacity in [0.1, 0.25, 0.5, 0.75, 1.0] {
                let faded = at(&view, opacity, bg);
                // Every pixel is its full-strength color blended over the background by the
                // same share: an outline or box never shows ahead of its label.
                for (i, (f, s)) in faded.rgba.as_chunks::<4>().0.iter().zip(full.rgba.as_chunks::<4>().0).enumerate() {
                    for c in 0..3 {
                        let want = back[c] as f32 + (s[c] as f32 - back[c] as f32) * opacity;
                        assert!((f[c] as f32 - want).abs() <= 2.0, "{name} {bg_name} at {opacity}: pixel {i} channel {c} is {} for {want}", f[c]);
                    }
                }
                if let Some(dir) = &dir {
                    faded.save(&dir.join(format!("{name}-{bg_name}-{:03}.png", (opacity * 100.0) as u32))).expect("writes the picture");
                }
            }
        }
    }
}
