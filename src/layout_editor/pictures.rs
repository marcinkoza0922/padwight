//! The pictures beside Edit Controls' pages: a stick, trigger, gyro or touchpad being set, with
//! the controller's live input. A menu, info or log overlay being edited shows on screen where it
//! goes (see [`Preview`]).

use crate::{
    config::{Analog, Button, Config, Stick, StickAction, Trigger, TriggerAction},
    figures::{Figure, Live, MouseShown},
    menu::{MenuSession, MenuView},
};

use super::{
    action::Slot,
    logs::Tracked,
    owner::{self, Owner},
    pages::Page,
    style::StyleAt,
};

/// An overlay being edited, shown on screen where it goes while its pages are open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preview {
    Info(usize),
    Log(usize),
    /// A menu, and the item being edited.
    Menu(usize, Option<usize>),
}

/// What a page is about, for its picture.
enum About {
    Stick(Owner, Stick),
    Trigger(Owner, Trigger),
    Gyro(Owner),
    Touchpad(Owner),
    /// A menu, and the item being edited.
    Menu(usize, Option<usize>),
    Info(usize),
    Log(usize),
}

/// The look a page sets, if it's one of the look pages.
fn style_at(page: Page) -> Option<StyleAt> {
    match page {
        Page::Style(at) | Page::Paint(at, _) | Page::StyleText(at) | Page::Decorations(at) | Page::Decor(at, _) => Some(at),
        _ => None,
    }
}

fn about(page: Page) -> Option<About> {
    if let Some(at) = style_at(page) {
        return match at {
            StyleAt::Menu(m) => Some(About::Menu(m, None)),
            StyleAt::Info(i) => Some(About::Info(i)),
            StyleAt::Log(i) => Some(About::Log(i)),
            StyleAt::Indicator(..) | StyleAt::Game(_) => None,
        };
    }
    Some(match page {
        Page::Stick(o, s) | Page::Ring(o, s) | Page::Zones(o, Analog::Stick(s)) | Page::Zone(o, Analog::Stick(s), _) => About::Stick(o, s),
        Page::Trigger(o, t) | Page::Zones(o, Analog::Trigger(t)) | Page::Zone(o, Analog::Trigger(t), _) => About::Trigger(o, t),
        Page::Gyro(o) | Page::GyroInput(o, _) => About::Gyro(o),
        Page::Touch(o, _) => About::Touchpad(o),
        Page::Menu(m) | Page::MenuItems(m) | Page::MenuClose(m) => About::Menu(m, None),
        Page::MenuItem(m, i) | Page::ItemButton(m, i) => About::Menu(m, Some(i)),
        Page::Info(i) | Page::InfoCells(i) | Page::InfoRow(i, _) | Page::Tracking(Tracked::Info(i)) | Page::TrackGroup(Tracked::Info(i), _) => About::Info(i),
        Page::Log(i) | Page::Tracking(Tracked::Log(i)) | Page::TrackGroup(Tracked::Log(i), _) => About::Log(i),
        Page::Action(at) | Page::Pad(at) | Page::KeyGroups(at) | Page::Keys(at, _) | Page::Pick(at, _) | Page::Multi(at) => match at.slot {
            Slot::Ring(o, s, _) | Slot::Zone(o, Analog::Stick(s), _) => About::Stick(o, s),
            Slot::Trigger(o, t) | Slot::Zone(o, Analog::Trigger(t), _) => About::Trigger(o, t),
            Slot::Touch(o, _) => About::Touchpad(o),
            Slot::MenuItem(m, i) => About::Menu(m, Some(i)),
            _ => return None,
        },
        _ => return None,
    })
}

/// The stick whose live position a page shows, so moving it doesn't move the cursor.
pub(super) fn live_stick(page: Page) -> Option<Stick> {
    match about(page) {
        Some(About::Stick(_, s)) => Some(s),
        _ => None,
    }
}

/// The overlay a page shows on screen where it goes.
pub(super) fn preview(page: Page) -> Option<Preview> {
    match about(page) {
        Some(About::Info(i)) => Some(Preview::Info(i)),
        Some(About::Log(i)) => Some(Preview::Log(i)),
        Some(About::Menu(m, item)) => Some(Preview::Menu(m, item)),
        _ => None,
    }
}

/// The picture beside a page, with the live input in it.
pub(super) fn figure(page: Page, config: &Config, live: &Live) -> Option<Figure> {
    let profile = config.active()?;
    Some(match about(page)? {
        About::Stick(o, s) => {
            let cfg = owner::stick(config, o, s).unwrap_or(profile.stick(s));
            let ring = match cfg.action {
                StickAction::Ring { sectors, start_angle, inner_radius, .. } => Some((sectors, start_angle, inner_radius)),
                _ => None,
            };
            let mouse = match cfg.action {
                StickAction::Mouse { speed, response, .. } => Some(MouseShown { speed, curve: cfg.curve, response }),
                _ => None,
            };
            Figure::Stick { deadzone: cfg.dead(), ring, mouse, live: Some(live.sticks[usize::from(s == Stick::Right)]) }
        }
        About::Trigger(o, t) => {
            let cfg = owner::trigger(config, o, t).unwrap_or(if t == Trigger::Left { &profile.left_trigger } else { &profile.right_trigger });
            let press_at = match cfg.action {
                TriggerAction::Button { threshold, .. } => Some(threshold),
                _ => None,
            };
            let zones = cfg.zones.iter().map(|z| (z.min, z.max)).collect();
            Figure::Trigger { press_at, zones, live: Some(live.triggers[usize::from(t == Trigger::Right)]) }
        }
        About::Gyro(o) => {
            let g = owner::gyro(config, o).unwrap_or(&profile.gyro);
            let (trail, at) = live.trail.points();
            Figure::Gyro { curve: g.mode.uses_curve().then_some(g.curve), speed: live.turning, trail, at }
        }
        About::Touchpad(o) => {
            let pad = owner::touchpad(config, o).unwrap_or(&profile.touchpad).clone();
            let height = if live.pad_height > 0.0 { live.pad_height } else { crate::input::DUALSENSE_HEIGHT };
            Figure::Touchpad { pad: Box::new(pad), menus: config.active_game().menus.clone(), touches: live.touches.values().copied().collect(), click: live.click, height }
        }
        About::Menu(..) | About::Info(_) | About::Log(_) => return None,
    })
}

/// Menu `m` as it opens, with the cursor on `item` when one is being edited.
pub fn menu_view(config: &Config, m: usize, item: Option<usize>) -> Option<MenuView> {
    let menu = config.active_game().menus.get(m)?;
    let menus = std::slice::from_ref(menu);
    let opener = crate::engine::Opener { buttons: vec![Button::LeftBumper], ..crate::engine::Opener::default() };
    let view = MenuSession::open(menus, &menu.name, opener)?.view(menus)?;
    Some(MenuView { selected: item.or(view.selected), ..view })
}
