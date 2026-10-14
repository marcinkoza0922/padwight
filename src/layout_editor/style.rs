//! The look pages of Edit Controls: where an overlay sits, its size and shape, its colors, its
//! text, and (in `decor.rs`) its decorations. One set of pages serves every overlay with a look:
//! menus, info and log overlays, a layer's name label, and the game's own keyboard, numpad, media
//! controls and menu.

use crate::config::{Case, Config, OverlayStyle, Paint, ScreenPosition};

use super::{
    decor::{Decor, decor_paint, set_decor_paint},
    owner::{Shift, game_mut, layer, layer_mut},
    pages::Page,
    rows::{Act, Number, Row, cycle, nudge, set},
};

/// Which overlay's look a page sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleAt {
    Menu(usize),
    Info(usize),
    Log(usize),
    /// A mode's or layer's name label.
    Indicator(Shift, usize),
    /// One of the game's own looks, which the app's apply to until it has one.
    Game(Look),
}

/// The overlays a game can give a look of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Keyboard,
    Numpad,
    Media,
    Menu,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Keyboard, Look::Numpad, Look::Media, Look::Menu];

    pub fn name(self) -> &'static str {
        match self {
            Look::Keyboard => "On-screen keyboard",
            Look::Numpad => "Numpad",
            Look::Media => "Media controls",
            Look::Menu => "In-game menu",
        }
    }

    fn of(self, game: &crate::config::Game) -> Option<&OverlayStyle> {
        match self {
            Look::Keyboard => game.keyboard_style.as_ref(),
            Look::Numpad => game.numpad_style.as_ref(),
            Look::Media => game.media_style.as_ref(),
            Look::Menu => game.menu_style.as_ref(),
        }
    }

    fn slot(self, game: &mut crate::config::Game) -> &mut Option<OverlayStyle> {
        match self {
            Look::Keyboard => &mut game.keyboard_style,
            Look::Numpad => &mut game.numpad_style,
            Look::Media => &mut game.media_style,
            Look::Menu => &mut game.menu_style,
        }
    }

    /// The app's look for it, which a game's own starts from.
    fn app(self, config: &Config) -> OverlayStyle {
        match self {
            Look::Keyboard => config.keyboard_style.clone(),
            Look::Numpad => config.numpad_style.clone(),
            Look::Media => config.media_style.clone(),
            Look::Menu => config.menu_style.clone(),
        }
    }
}

/// Which color of a look a page sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintOf {
    Background,
    Items,
    Selected,
    /// A decoration's color.
    Decor(Decor),
}

/// Colors offered as swatches, as in the settings window; any other is typed as #rrggbb.
const SWATCHES: [(&str, &str); 12] = [
    ("#000000", "Black"),
    ("#16181c", "Ink"),
    ("#30343c", "Slate"),
    ("#5a606b", "Gray"),
    ("#e6e8eb", "Mist"),
    ("#ffffff", "White"),
    ("#2f5db0", "Blue"),
    ("#1f7a7a", "Teal"),
    ("#2e7d46", "Green"),
    ("#6a3fb0", "Purple"),
    ("#a83232", "Red"),
    ("#c26a1d", "Orange"),
];

const SIZE: Number = Number::new(0.05, (0.5, 2.0), 2, "×");
const PERCENT: Number = Number::new(1.0, (10.0, 100.0), 0, "%");
const OFFSET: Number = Number::new(1.0, (-50.0, 50.0), 0, "%");
const CORNERS: Number = Number::new(0.05, (0.0, 1.0), 2, "");
const OPACITY: Number = Number::new(0.05, (0.0, 1.0), 2, "");
const SPACING: Number = Number::new(1.0, (0.0, 12.0), 0, " px");

pub(super) fn style(config: &Config, at: StyleAt) -> Option<&OverlayStyle> {
    let game = config.active_game();
    match at {
        StyleAt::Menu(m) => game.menus.get(m).map(|x| &x.style),
        StyleAt::Info(i) => game.info.get(i).map(|x| &x.style),
        StyleAt::Log(i) => game.logs.get(i).map(|x| &x.style),
        StyleAt::Indicator(shift, i) => layer(config, shift, i).map(|l| &l.indicator_style),
        StyleAt::Game(look) => look.of(game),
    }
}

pub(super) fn style_mut(config: &mut Config, at: StyleAt) -> Option<&mut OverlayStyle> {
    match at {
        StyleAt::Indicator(shift, i) => layer_mut(config, shift, i).map(|l| &mut l.indicator_style),
        _ => {
            let game = game_mut(config)?;
            match at {
                StyleAt::Menu(m) => game.menus.get_mut(m).map(|x| &mut x.style),
                StyleAt::Info(i) => game.info.get_mut(i).map(|x| &mut x.style),
                StyleAt::Log(i) => game.logs.get_mut(i).map(|x| &mut x.style),
                StyleAt::Game(look) => look.slot(game).as_mut(),
                StyleAt::Indicator(..) => None,
            }
        }
    }
}

/// A number of a look, changed by `field`.
pub(super) fn num(at: StyleAt, label: &str, value: f32, n: &Number, field: fn(&mut OverlayStyle, f32) -> bool) -> Row {
    n.row(label, value, move |c, v| style_mut(c, at).is_some_and(|s| field(s, v)))
}

/// A size that is either fitted to the contents or a share of the display. Stepping below the
/// smallest share fits it to the contents again.
fn fitted(at: StyleAt, label: &str, value: Option<f32>, field: fn(&mut OverlayStyle, Option<f32>) -> bool) -> Row {
    let (lo, hi) = PERCENT.range;
    let shown = value.map_or_else(|| "fits its contents".into(), |v| PERCENT.show(v));
    Row::step(format!("{label}: {shown}"), move |c, dir| {
        let next = match value {
            None if dir > 0 => Some(lo),
            None => return false,
            Some(v) if v <= lo && dir < 0 => None,
            Some(v) => Some(nudge(v, 5.0, dir, (lo, hi))),
        };
        style_mut(c, at).is_some_and(|s| field(s, next))
    })
}

/// A look's page: where it sits, its size and shape, its colors, and the pages for its text and
/// decorations. A game's own look can go back to the app's.
pub(super) fn style_rows(config: &Config, at: StyleAt) -> Vec<Row> {
    let Some(s) = style(config, at) else {
        let StyleAt::Game(look) = at else { return Vec::new() };
        return vec![
            Row::note("Looks as the app's settings have it"),
            Row::add("Give it a look of its own", move |c| {
                let app = look.app(c);
                let Some(game) = game_mut(c) else { return Act::stay() };
                Act::changed_if(set(look.slot(game), Some(app)))
            }),
        ];
    };
    let position = s.position;
    let mut rows = vec![
        Row::step(format!("Position: {position}"), move |c, dir| style_mut(c, at).is_some_and(|s| set(&mut s.position, cycle(&ScreenPosition::GRID, position, dir)))),
        num(at, "Offset right", s.x_offset, &OFFSET, |s, v| set(&mut s.x_offset, v)),
        num(at, "Offset down", s.y_offset, &OFFSET, |s, v| set(&mut s.y_offset, v)),
        num(at, "Size", s.scale, &SIZE, |s, v| set(&mut s.scale, v)),
        fitted(at, "Width", s.width, |s, v| set(&mut s.width, v)),
        fitted(at, "Height", s.height, |s, v| set(&mut s.height, v)),
        num(at, "Widest", s.max_width, &PERCENT, |s, v| set(&mut s.max_width, v)),
        num(at, "Tallest", s.max_height, &PERCENT, |s, v| set(&mut s.max_height, v)),
        num(at, "Corners", s.corners, &CORNERS, |s, v| set(&mut s.corners, v)),
    ];
    for (label, of, paint) in [("Background", PaintOf::Background, &s.background), ("Items", PaintOf::Items, &s.items), ("Selected item", PaintOf::Selected, &s.selected)] {
        rows.push(Row::open(format!("{label}: {}", paint_name(paint)), Page::Paint(at, of)));
    }
    rows.push(Row::open("Text…", Page::StyleText(at)));
    rows.push(Row::open("Decorations…", Page::Decorations(at)));
    if let StyleAt::Game(look) = at {
        rows.push(Row::apply("Use the app's look", move |c| {
            let Some(game) = game_mut(c) else { return Act::stay() };
            Act::changed_if(set(look.slot(game), None))
        }).tone(crate::menu::Tone::Remove));
    }
    rows
}

/// A color as its swatch's name, or its #rrggbb, with its opacity when it isn't solid.
pub(super) fn paint_name(paint: &Paint) -> String {
    let name = SWATCHES.iter().find(|(hex, _)| paint.color.eq_ignore_ascii_case(hex)).map_or(paint.color.as_str(), |(_, name)| name);
    if paint.opacity < 1.0 { format!("{name}, {:.0}%", paint.opacity * 100.0) } else { name.to_string() }
}

fn paint(s: &OverlayStyle, of: PaintOf) -> Option<Paint> {
    match of {
        PaintOf::Background => Some(s.background.clone()),
        PaintOf::Items => Some(s.items.clone()),
        PaintOf::Selected => Some(s.selected.clone()),
        PaintOf::Decor(d) => decor_paint(s, d),
    }
}

fn set_paint(s: &mut OverlayStyle, of: PaintOf, p: Paint) -> bool {
    match of {
        PaintOf::Background => set(&mut s.background, p),
        PaintOf::Items => set(&mut s.items, p),
        PaintOf::Selected => set(&mut s.selected, p),
        PaintOf::Decor(d) => set_decor_paint(s, d, &p),
    }
}

/// A color's page: the swatches, typing any #rrggbb, and how see-through it is.
pub(super) fn paint_rows(config: &Config, at: StyleAt, of: PaintOf) -> Vec<Row> {
    let Some(now) = style(config, at).and_then(|s| paint(s, of)) else { return Vec::new() };
    let mut rows: Vec<Row> = SWATCHES
        .iter()
        .map(|&(hex, name)| {
            let chosen = now.color.eq_ignore_ascii_case(hex);
            let opacity = now.opacity;
            Row::apply(format!("{}{name}", if chosen { "✓ " } else { "" }), move |c| {
                Act::changed_if(style_mut(c, at).is_some_and(|s| set_paint(s, of, Paint { color: hex.into(), opacity })))
            })
        })
        .collect();
    let opacity = now.opacity;
    rows.push(Row::text(format!("Type a color: {}", now.color), "A color, as #rrggbb", now.color.clone(), move |c, typed| {
        let typed = typed.trim();
        let hex = if typed.starts_with('#') { typed.to_string() } else { format!("#{typed}") };
        crate::config::parse_hex(&hex).is_some() && style_mut(c, at).is_some_and(|s| set_paint(s, of, Paint { color: hex.clone(), opacity }))
    }));
    let color = now.color.clone();
    rows.push(OPACITY.row("Opacity", now.opacity, move |c, v| style_mut(c, at).is_some_and(|s| set_paint(s, of, Paint { color: color.clone(), opacity: v }))));
    rows
}

/// How a look sets its text: case, spacing and size.
pub(super) fn text_rows(config: &Config, at: StyleAt) -> Vec<Row> {
    let Some(t) = style(config, at).map(|s| s.typography.clone()) else { return Vec::new() };
    let case = t.case;
    let name = match case {
        Case::None => "as typed",
        Case::Upper => "CAPITALS",
        Case::Lower => "lower case",
    };
    vec![
        Row::step(format!("Letters: {name}"), move |c, dir| style_mut(c, at).is_some_and(|s| set(&mut s.typography.case, cycle(&[Case::None, Case::Upper, Case::Lower], case, dir)))),
        num(at, "Letter spacing", t.letter_spacing, &SPACING, |s, v| set(&mut s.typography.letter_spacing, v)),
        num(at, "Text size", t.size, &SIZE, |s, v| set(&mut s.typography.size, v)),
    ]
}

/// What a look's page is called.
pub(super) fn style_title(config: &Config, at: StyleAt) -> String {
    let game = config.active_game();
    let named = |name: Option<&str>, what: &str| format!("{} · look", name.unwrap_or(what));
    match at {
        StyleAt::Menu(m) => named(game.menus.get(m).map(|x| x.name.as_str()), "Menu"),
        StyleAt::Info(i) => named(game.info.get(i).map(|x| x.name.as_str()), "Info overlay"),
        StyleAt::Log(i) => named(game.logs.get(i).map(|x| x.name.as_str()), "Log overlay"),
        StyleAt::Indicator(shift, i) => format!("{} · name label", layer(config, shift, i).map_or(shift.title(), |l| l.name.as_str())),
        StyleAt::Game(look) => format!("{} · look", look.name()),
    }
}
