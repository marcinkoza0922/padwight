//! Frames for looking at the new menu features on screen. Opt-in: with `PADWIGHT_SCREEN_FRAMES`
//! naming a file, `cargo test screens -- --ignored` writes one overlay frame per scene to it, a
//! name and the frame's JSON on each line, for `scripts/menu-screens.sh` to show and capture.

use std::io::Write;

use super::*;
use crate::config::{Cluster, MenuItem};
use crate::overlay::{OverlayFrame, OverlayView};

fn labeled(labels: &[&str]) -> Vec<MenuItem> {
    labels.iter().enumerate().map(|(i, l)| MenuItem::new(*l, key(&format!("KEY_{}", i + 1)))).collect()
}

/// A 3×3 grid with a big "Map" over four cells and a tall "Inventory" beside it.
fn merged_grid() -> Menu {
    let mut items = labeled(&["Map", "-", "Inventory", "-", "-", "-", "Quick save", "Quick load", "Photo"]);
    items[0].span = Span { columns: 2, rows: 2 };
    items[2].span = Span { columns: 1, rows: 2 };
    menu("Field kit", MenuKind::Grid { columns: 3 }, items)
}

fn scenes() -> Vec<(&'static str, MenuView)> {
    let mut scenes = Vec::new();
    let mut shot = |name, pad: &Pad| scenes.push((name, pad.view()));

    let menus = [merged_grid()];
    let mut pad = Pad::open(&menus, "Field kit");
    shot("grid-merged-on-big", &pad);
    pad.press(Button::DpadRight);
    shot("grid-merged-on-tall", &pad);
    pad.press(Button::DpadDown);
    shot("grid-merged-on-bottom-row", &pad);

    let menus = [Menu { start: 7, ..merged_grid() }];
    shot("grid-start-cell", &Pad::open(&menus, "Field kit"));

    let wide = {
        let mut items = labeled(&["Attack", "-", "-", "Defend", "Item", "Flee", "Skills", "-", "Talk"]);
        items[0].span = Span { columns: 3, rows: 1 };
        items[6].span = Span { columns: 2, rows: 1 };
        menu("Battle", MenuKind::Grid { columns: 3 }, items)
    };
    let menus = [wide];
    shot("grid-merged-rows", &Pad::open(&menus, "Battle"));

    let link = |to: &str| MenuItem::new(to, open_menu(to));
    let mut top = vec![link("Field kit")];
    top.extend(labeled(&["Resume", "Settings"]));
    let mut kit = merged_grid();
    kit.items[7] = link("Saves");
    let menus = [menu("Pause", MenuKind::List, top), kit, menu("Saves", MenuKind::List, labeled(&["Slot 1", "Slot 2", "Slot 3"]))];
    let mut pad = Pad::open(&menus, "Pause");
    shot("submenu-top", &pad);
    pad.press(Button::South);
    shot("submenu-one-deep", &pad);
    pad.press(Button::DpadDown);
    pad.press(Button::DpadRight);
    pad.press(Button::South);
    shot("submenu-two-deep", &pad);
    let mut pad = Pad::open_with(&menus, "Pause", Opener::default());
    pad.press(Button::South);
    shot("submenu-unwatched", &pad);

    let menus = [
        menu("Weapons", MenuKind::Radial { stick: Stick::Right, boxes: false }, labeled(&["Pistol", "Rifle", "Shotgun", "Knife"])),
        menu("Quick", MenuKind::Directional { cluster: Cluster::FaceButtons }, labeled(&["Heal", "Reload", "Grenade", "Use"])),
        menu("Emotes", MenuKind::Directional { cluster: Cluster::DPad }, labeled(&["Wave", "Point", "Cheer", "Sit"])),
    ];
    shot("default-close-radial", &Pad::open(&menus, "Weapons"));
    shot("default-close-face-buttons", &Pad::open(&menus, "Quick"));
    shot("default-close-dpad", &Pad::open(&menus, "Emotes"));

    let touch = touch_view(&merged_grid(), Some(0), crate::config::TouchChoose::Lift, 0);
    scenes.push(("grid-merged-touchpad", touch));
    scenes
}

#[test]
#[ignore = "writes frames for scripts/menu-screens.sh"]
fn screens() {
    let path = std::env::var_os("PADWIGHT_SCREEN_FRAMES").expect("PADWIGHT_SCREEN_FRAMES names the file to write");
    let mut file = std::fs::File::create(path).unwrap();
    for (name, view) in scenes() {
        let frame = OverlayFrame { active: Some(OverlayView::Menu(view)), ..OverlayFrame::default() };
        writeln!(file, "{name}\t{}", serde_json::to_string(&frame).unwrap()).unwrap();
    }
}
