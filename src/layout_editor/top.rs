//! Edit Controls' first page: what can be edited, each opening its own list.

use super::{
    Cx,
    owner::{Owner, Shift},
    pages::Page,
    rows::Row,
};

pub(super) fn rows(cx: &Cx) -> Vec<Row> {
    let config = cx.config;
    let game = config.active_game();
    let combos = config.active().map_or(0, |p| p.combos.len());
    vec![
        Row::open("Buttons", Page::Buttons(Owner::Profile)),
        Row::open("{ls} {rs} Sticks", Page::Sticks(Owner::Profile)),
        Row::open("{lt} {rt} Triggers", Page::Triggers(Owner::Profile)),
        Row::open("Gyro", Page::Gyro(Owner::Profile)),
        Row::open("Touchpad", Page::Touch(Owner::Profile, crate::config::TouchSide::Left)),
        Row::open(format!("Combos ({combos})"), Page::Combos(Owner::Profile)),
        Row::open(format!("Modes ({})", game.modes.len()), Page::Layers(Shift::Mode)),
        Row::open(format!("Layers ({})", game.layers.len()), Page::Layers(Shift::Layer)),
        Row::open(format!("Macros ({})", game.macros.len()), Page::Macros),
        Row::open(format!("Menus ({})", game.menus.len()), Page::Menus),
        Row::open(format!("Info overlays ({})", game.info.len()), Page::Infos),
    ]
}
