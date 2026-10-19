//! Edit Controls' first page: what can be edited, each opening its own list.

use super::{
    Cx,
    owner::{Owner, Shift},
    pages::Page,
    rows::{Row, counted},
};

pub(super) fn rows(cx: &Cx) -> Vec<Row> {
    let config = cx.config;
    let game = config.active_game();
    let combos = config.active().map_or(0, |p| p.combos.len());
    vec![
        Row::open("Buttons", Page::Buttons(Owner::Profile)),
        Row::open("Sticks", Page::Sticks(Owner::Profile)),
        Row::open("Triggers", Page::Triggers(Owner::Profile)),
        Row::open("Gyro", Page::Gyro(Owner::Profile)),
        Row::open("Touchpad", Page::Touch(Owner::Profile, crate::config::TouchSide::Left)),
        Row::open(counted("Combos", combos), Page::Combos(Owner::Profile)),
        Row::open(counted("Modes", game.modes.len()), Page::Layers(Shift::Mode)),
        Row::open(counted("Layers", game.layers.len()), Page::Layers(Shift::Layer)),
        Row::open(counted("Macros", game.macros.len()), Page::Macros),
        Row::open(counted("Menus", game.menus.len()), Page::Menus),
        Row::open(counted("Info overlays", game.info.len()), Page::Infos),
        Row::open(counted("Log overlays", game.logs.len()), Page::Logs),
        Row::open("Guide overlay", Page::Guide),
        Row::open(format!("Profile: {}", config.active_ref().profile), Page::Profile),
        Row::open(format!("Game: {}", config.active_ref().game.unwrap_or_else(|| "General".into())), Page::Game),
    ]
}
