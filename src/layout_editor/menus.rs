//! The menu pages of Edit Controls: the game's menus, a menu's kind and settings, its items, and
//! an item's label, action, button and size.

use crate::config::{Button, ButtonAction, CarouselControls, Cluster, Config, GRID_MAX, ItemKind, Menu, MenuItem, MenuKind, MenuKindTag, OverlayStyle, Span, Stick};

use super::{
    action::{ActionAt, NEW_ITEM, Slot},
    named,
    owner::game_mut,
    pages::{Page, stick_name},
    rows::{Act, Nav, Number, Row, cycle, set},
    style::StyleAt,
};

const WEIGHT: Number = Number::new(0.25, (0.25, 4.0), 2, "×");
const COLUMNS: Number = Number::new(1.0, (1.0, 8.0), 0, "");

fn menu_mut(config: &mut Config, m: usize) -> Option<&mut Menu> {
    game_mut(config)?.menus.get_mut(m)
}

fn item_mut(config: &mut Config, m: usize, i: usize) -> Option<&mut MenuItem> {
    menu_mut(config, m)?.items.get_mut(i)
}

/// The game's menus, and a row to add one.
pub(super) fn list_rows(config: &Config) -> Vec<Row> {
    let menus = &config.active_game().menus;
    let mut rows: Vec<Row> = menus.iter().enumerate().map(|(m, x)| Row::open(format!("{} · {} · {} items", x.name, x.kind.tag().short(), x.items.len()), Page::Menu(m))).collect();
    let new = menus.len();
    rows.push(Row::add("Add menu", move |c| {
        let name = named::new_name(c, ItemKind::Menu, "Menu");
        let Some(game) = game_mut(c) else { return Act::stay() };
        game.menus.push(Menu { name, kind: MenuKind::List, items: Vec::new(), cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() });
        Act::changed().then(Nav::Open(Page::Menu(new)))
    }));
    rows
}

/// A menu's page: its name, kind and the kind's settings, items, close button, and deleting it.
pub(super) fn menu_rows(config: &Config, m: usize) -> Vec<Row> {
    let Some(menu) = config.active_game().menus.get(m) else { return Vec::new() };
    let mut rows = vec![
        named::rename_row(config, ItemKind::Menu, m),
        Row::open(format!("Items ({})", menu.items.len()), Page::MenuItems(m)),
        Row::step(format!("Kind: {}", menu.kind.tag().short()), move |c, dir| {
            menu_mut(c, m).is_some_and(|menu| {
                let next = next_kind(menu, dir);
                set(&mut menu.kind, next)
            })
        }),
    ];
    rows.extend(kind_rows(m, menu));
    rows.push(Row::open(format!("Closes with: {}", close_label(menu)), Page::MenuClose(m)));
    rows.push(Row::open("Look", Page::Style(StyleAt::Menu(m))));
    rows.push(Row::toggle("Stays up after a choice", menu.keep_open, move |c, on| menu_mut(c, m).is_some_and(|menu| set(&mut menu.keep_open, on))));
    rows.push(named::delete_row(config, ItemKind::Menu, m, move |c| {
        let Some(game) = game_mut(c).filter(|g| m < g.menus.len()) else { return false };
        game.menus.remove(m);
        true
    }));
    rows
}

/// The settings of the menu's kind: a radial menu's stick and boxes, the buttons a directional
/// menu uses, what turns a carousel, a grid's columns and where its cursor starts.
fn kind_rows(m: usize, menu: &Menu) -> Vec<Row> {
    let kind = menu.kind;
    let set_kind = move |c: &mut Config, next: MenuKind| menu_mut(c, m).is_some_and(|menu| set(&mut menu.kind, next));
    match kind {
        MenuKind::Radial { stick, boxes } => vec![
            Row::step(format!("Aimed with: {}", stick_name(stick)), move |c, _| {
                let other = if stick == Stick::Left { Stick::Right } else { Stick::Left };
                set_kind(c, MenuKind::Radial { stick: other, boxes })
            }),
            Row::toggle("Items in boxes", boxes, move |c, on| set_kind(c, MenuKind::Radial { stick, boxes: on })),
        ],
        MenuKind::Directional { cluster } => vec![Row::step(format!("Picked with: {cluster}"), move |c, _| {
            let other = if cluster == Cluster::DPad { Cluster::FaceButtons } else { Cluster::DPad };
            set_kind(c, MenuKind::Directional { cluster: other })
        })],
        MenuKind::Carousel { controls } => vec![Row::step(format!("Turned with: {controls}"), move |c, dir| {
            set_kind(c, MenuKind::Carousel { controls: cycle(&CarouselControls::ALL, controls, dir) })
        })],
        MenuKind::Grid { columns } => {
            let len = menu.items.len();
            vec![
                COLUMNS.row("Columns", f32::from(columns), move |c, v| set_kind(c, MenuKind::Grid { columns: v as u8 })),
                Number::new(1.0, (1.0, len.max(1) as f32), 0, "").row_u("Cursor starts on item", menu.start as u64 + 1, move |c, v| {
                    menu_mut(c, m).is_some_and(|menu| set(&mut menu.start, (v as usize).saturating_sub(1)))
                }),
            ]
        }
        MenuKind::List | MenuKind::Buttons => Vec::new(),
    }
}

/// The next kind a menu can be, `dir` steps on. A directional menu holds four items, so it's
/// skipped for a longer menu; a grid gets enough columns for its rows.
fn next_kind(menu: &Menu, dir: i32) -> MenuKind {
    let tags = MenuKindTag::ALL;
    let at = tags.iter().position(|t| *t == menu.kind.tag()).unwrap_or(0) as i32;
    let fits = |tag: &MenuKindTag| *tag != MenuKindTag::Directional || menu.items.len() <= 4;
    let next = (1..=tags.len() as i32).map(|step| tags[(at + step * dir).rem_euclid(tags.len() as i32) as usize]).find(fits).unwrap_or(menu.kind.tag());
    match MenuKind::default_for(next) {
        MenuKind::Grid { columns } => MenuKind::Grid { columns: columns.max(menu.items.len().div_ceil(GRID_MAX) as u8) },
        kind => kind,
    }
}

fn close_label(menu: &Menu) -> String {
    let default = menu.default_cancel();
    match (menu.no_cancel, menu.cancel) {
        (true, _) if default.is_some() => "nothing".into(),
        (false, Some(b)) => b.short_name().into(),
        _ => format!("{} (its kind's)", default.map_or("nothing", |b| b.short_name())),
    }
}

/// What a menu's close button can be: its kind's, none, or any button but Guide.
pub(super) fn close_rows(config: &Config, m: usize) -> Vec<Row> {
    let Some(menu) = config.active_game().menus.get(m) else { return Vec::new() };
    let default = menu.default_cancel();
    let mut rows = vec![Row::apply(format!("Its kind's ({})", default.map_or("nothing", |b| b.short_name())), move |c| {
        let changed = menu_mut(c, m).is_some_and(|menu| set(&mut menu.cancel, None) | set(&mut menu.no_cancel, false));
        Act::changed_if(changed).then(Nav::Back(1))
    })];
    if default.is_some() {
        rows.push(Row::apply("Nothing (it goes when let go or chosen)", move |c| {
            let changed = menu_mut(c, m).is_some_and(|menu| set(&mut menu.cancel, None) | set(&mut menu.no_cancel, true));
            Act::changed_if(changed).then(Nav::Back(1))
        }));
    }
    rows.extend(Button::ALL.into_iter().filter(|&b| b != Button::Guide && Some(b) != default).map(|b| {
        Row::apply("", move |c| {
            let changed = menu_mut(c, m).is_some_and(|menu| set(&mut menu.cancel, Some(b)) | set(&mut menu.no_cancel, false));
            Act::changed_if(changed).then(Nav::Back(1))
        })
        .buttons(vec![b])
    }));
    rows
}

/// A menu's items, and a row to add one.
pub(super) fn items_rows(config: &Config, m: usize) -> Vec<Row> {
    let Some(menu) = config.active_game().menus.get(m) else { return Vec::new() };
    let mut rows: Vec<Row> = menu
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| Row::open(format!("{}: {}", it.label, it.action.summary()), Page::MenuItem(m, i)).buttons(it.button.into_iter().collect()).keyword(it.action.keyword()))
        .collect();
    let new = menu.items.len();
    rows.push(Row::add("Add item", move |c| {
        let Some(menu) = menu_mut(c, m) else { return Act::stay() };
        menu.items.push(MenuItem::new(NEW_ITEM, ButtonAction::Disabled));
        Act::changed().then(Nav::Open(Page::MenuItem(m, new)))
    }));
    rows
}

/// An item's page: its label, action, the settings its menu's kind gives it, moving and removing it.
pub(super) fn item_rows(config: &Config, m: usize, i: usize) -> Vec<Row> {
    let Some(menu) = config.active_game().menus.get(m) else { return Vec::new() };
    // The page names the item by position, which a profile switch under the menu can leave past
    // the end of another game's menu.
    let Some(it) = menu.items.get(i) else { return Vec::new() };
    let mut rows = vec![
        Row::text(format!("Label: {}", it.label), "Label of the item", it.label.clone(), move |c, typed| {
            let typed = typed.trim();
            !typed.is_empty() && item_mut(c, m, i).is_some_and(|it| set(&mut it.label, typed.to_string()))
        }),
        Row::open(format!("Action: {}", it.action.summary()), Page::Action(ActionAt::new(Slot::MenuItem(m, i)))).keyword(it.action.keyword()),
    ];
    match menu.kind {
        MenuKind::Buttons => rows.push(Row::open(format!("Its button: {}", it.button.map_or("none", |b| b.short_name())), Page::ItemButton(m, i)).buttons(it.button.into_iter().collect())),
        MenuKind::Radial { .. } => rows.push(WEIGHT.row("Share of the circle", it.weight, move |c, v| item_mut(c, m, i).is_some_and(|it| set(&mut it.weight, v)))),
        MenuKind::Grid { columns } => {
            let span = it.span;
            let wide = Number::new(1.0, (1.0, f32::from(columns)), 0, " cells");
            let tall = Number::new(1.0, (1.0, GRID_MAX as f32), 0, " cells");
            rows.push(wide.row("Width", f32::from(span.columns), move |c, v| item_mut(c, m, i).is_some_and(|it| set(&mut it.span, Span { columns: v as u8, ..span }))));
            rows.push(tall.row("Height", f32::from(span.rows), move |c, v| item_mut(c, m, i).is_some_and(|it| set(&mut it.span, Span { rows: v as u8, ..span }))));
        }
        _ => {}
    }
    let len = menu.items.len();
    for (label, to) in [("Move up", i.checked_sub(1)), ("Move down", (i + 1 < len).then_some(i + 1))] {
        let Some(to) = to else { continue };
        rows.push(Row::apply(label, move |c| {
            let Some(menu) = menu_mut(c, m) else { return Act::stay() };
            menu.items.swap(i, to);
            // Follow the item to where it went.
            Act::changed().then(Nav::Follow(Page::MenuItem(m, to), to))
        }));
    }
    rows.push(Row::delete("Remove item", move |c| {
        let Some(menu) = menu_mut(c, m).filter(|menu| i < menu.items.len()) else { return false };
        menu.items.remove(i);
        true
    }));
    rows
}

/// The buttons a button menu's item can be picked with: none, or any but Guide.
pub(super) fn item_button_rows(config: &Config, m: usize, i: usize) -> Vec<Row> {
    let taken: Vec<Button> = config.active_game().menus.get(m).map(|menu| menu.items.iter().filter_map(|it| it.button).collect()).unwrap_or_default();
    let pick = |b: Option<Button>| {
        move |c: &mut Config| Act::changed_if(item_mut(c, m, i).is_some_and(|it| set(&mut it.button, b))).then(Nav::Back(1))
    };
    let mut rows = vec![Row::apply("None", pick(None))];
    rows.extend(Button::ALL.into_iter().filter(|&b| b != Button::Guide).map(|b| {
        let used = if taken.contains(&b) { "taken" } else { "" };
        Row::apply(used, pick(Some(b))).buttons(vec![b])
    }));
    rows
}
