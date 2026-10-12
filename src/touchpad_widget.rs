//! The touchpad as a picture: its sides, what each does (a menu's items where they lie on it),
//! and the fingers on it. Shared by the settings window's Touchpad tab and the debug window.
//! (iced's SVG renderer may not draw SVG text, so labels are widgets pinned over the drawing.)

use std::fmt::Write;

use iced::{
    Color, Element,
    widget::{container, pin, stack, svg, text},
};

use crate::{
    config::{Menu, TouchSide, TouchpadConfig, TouchpadMotion},
    menu::{ItemShape, TOUCH_CENTER_RADIUS, item_at, item_shapes},
};

const PANEL: &str = "#1d2026";
const EDGE: &str = "#3a3f48";
const ZONE: [&str; 2] = ["#262a31", "#2d323a"];
const ACCENT: &str = "#4ea1ff";
const LABEL: Color = Color::from_rgb(0.79, 0.82, 0.85);
const PICKED: Color = Color::WHITE;

/// What to draw.
pub struct PadPicture<'a> {
    /// The settings to show on the pad, and the menus they can lay out; `None` is a bare pad.
    pub settings: Option<(&'a TouchpadConfig, &'a [&'a Menu])>,
    /// Fingers on the pad, each (x, y) 0..1 across it.
    pub touches: &'a [(f32, f32)],
    pub click: bool,
    /// The pad's height in widths.
    pub height: f32,
    /// How wide to draw it, in logical pixels.
    pub width: f32,
}

/// A side of the pad as drawn: its settings and where it is, in pixels.
struct Area<'a> {
    side: TouchSide,
    motion: &'a TouchpadMotion,
    x: f32,
    w: f32,
    h: f32,
}

impl Area<'_> {
    /// Where a point 0..1 across the area is, in pixels.
    fn at(&self, (u, v): (f32, f32)) -> (f32, f32) {
        (self.x + u * self.w, v * self.h)
    }

    /// The point `r` pixels from the middle at `degrees` clockwise from up.
    fn polar(&self, degrees: f32, r: f32) -> (f32, f32) {
        let a = degrees.to_radians();
        (self.x + self.w / 2.0 + r * a.sin(), self.h / 2.0 - r * a.cos())
    }
}

impl PadPicture<'_> {
    fn size(&self) -> (f32, f32) {
        (self.width, self.width * self.height)
    }

    fn areas(&self) -> Vec<Area<'_>> {
        let (w, h) = self.size();
        let Some((cfg, _)) = self.settings else { return Vec::new() };
        let split = cfg.right.is_some();
        cfg.sides()
            .iter()
            .map(|&side| {
                let (x, aw) = match (split, side) {
                    (false, _) => (0.0, w),
                    (true, TouchSide::Left) => (0.0, w / 2.0),
                    (true, TouchSide::Right) => (w / 2.0, w / 2.0),
                };
                Area { side, motion: cfg.motion(side), x, w: aw, h }
            })
            .collect()
    }

    /// The menu `area` lays out, when it is in menu mode and the menu exists.
    fn menu_of<'m>(&'m self, area: &Area<'_>) -> Option<&'m Menu> {
        let name = area.motion.menu()?;
        self.settings?.1.iter().find(|m| &m.name == name).copied()
    }

    /// The items fingers are on in `area`.
    fn picked(&self, area: &Area<'_>, menu: &Menu) -> Vec<usize> {
        let (w, _) = self.size();
        self.touches
            .iter()
            .filter(|(x, _)| (x * w) >= area.x && (x * w) < area.x + area.w + 0.5)
            .filter_map(|&(x, y)| item_at(menu, (((x * w) - area.x) / area.w, y), area.h / area.w))
            .collect()
    }

    fn svg(&self) -> String {
        let (w, h) = self.size();
        let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><defs>"#);
        for (i, a) in self.areas().iter().enumerate() {
            let _ = write!(s, r#"<clipPath id="area{i}"><rect x="{}" y="0" width="{}" height="{h}"/></clipPath>"#, a.x, a.w);
        }
        let _ = write!(s, r#"</defs><rect x="1" y="1" width="{}" height="{}" rx="12" fill="{PANEL}"/>"#, w - 2.0, h - 2.0);
        for (i, area) in self.areas().iter().enumerate() {
            let _ = write!(s, r#"<g clip-path="url(#area{i})">"#);
            self.draw_area(&mut s, area);
            s.push_str("</g>");
        }
        if self.areas().len() > 1 {
            let _ = write!(s, r#"<line x1="{x}" y1="6" x2="{x}" y2="{}" stroke="{EDGE}" stroke-width="2" stroke-dasharray="6 4"/>"#, h - 6.0, x = w / 2.0);
        }
        let edge = if self.click { ACCENT } else { EDGE };
        let _ = write!(s, r#"<rect x="1" y="1" width="{}" height="{}" rx="12" fill="none" stroke="{edge}" stroke-width="2"/>"#, w - 2.0, h - 2.0);
        for &(x, y) in self.touches {
            let _ = write!(s, r#"<circle cx="{}" cy="{}" r="7" fill="white" fill-opacity="0.9" stroke="{ACCENT}" stroke-width="2"/>"#, x * w, y * h);
        }
        s.push_str("</svg>");
        s
    }

    fn draw_area(&self, s: &mut String, area: &Area<'_>) {
        match area.motion {
            TouchpadMotion::Menu { .. } => {
                let Some(menu) = self.menu_of(area) else { return };
                let picked = self.picked(area, menu);
                let reach = area.w.hypot(area.h);
                let mut slices = false;
                for (i, shape) in item_shapes(menu).into_iter().enumerate() {
                    let (fill, opacity) = if picked.contains(&i) { (ACCENT, 0.45) } else { (ZONE[i % 2], 1.0) };
                    match shape {
                        ItemShape::Cell(x0, y0, x1, y1) => {
                            let ((ax, ay), (bx, by)) = (area.at((x0, y0)), area.at((x1, y1)));
                            let _ = write!(s, r#"<rect x="{ax}" y="{ay}" width="{}" height="{}" fill="{fill}" fill-opacity="{opacity}" stroke="{EDGE}"/>"#, bx - ax, by - ay);
                        }
                        ItemShape::Hidden => {}
                        ItemShape::Slice(a, b) => {
                            slices = true;
                            let ((cx, cy), (sx, sy), (ex, ey)) = (area.polar(0.0, 0.0), area.polar(a, reach), area.polar(b, reach));
                            let large = i32::from(b - a > 180.0);
                            let _ = write!(s, r#"<path d="M {cx} {cy} L {sx} {sy} A {reach} {reach} 0 {large} 1 {ex} {ey} Z" fill="{fill}" fill-opacity="{opacity}" stroke="{EDGE}"/>"#);
                        }
                    }
                }
                if slices {
                    let (cx, cy) = area.polar(0.0, 0.0);
                    let _ = write!(s, r#"<circle cx="{cx}" cy="{cy}" r="{}" fill="{PANEL}" stroke="{EDGE}"/>"#, TOUCH_CENTER_RADIUS * area.w);
                }
            }
            TouchpadMotion::Joystick { relative, .. } => {
                let (cx, cy) = area.polar(0.0, 0.0);
                let r = area.w.min(area.h) * 0.42;
                let dash = if *relative { r#" stroke-dasharray="5 4""# } else { "" };
                let _ = write!(s, r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{EDGE}" stroke-width="2"{dash}/>"#);
                let _ = write!(s, r#"<path d="M {} {cy} H {} M {cx} {} V {}" stroke="{EDGE}"/>"#, cx - 8.0, cx + 8.0, cy - 8.0, cy + 8.0);
            }
            TouchpadMotion::Swipe { .. } => {
                // An arrowhead at each edge, pointing out.
                let (cx, cy) = area.polar(0.0, 0.0);
                for degrees in [0.0_f32, 90.0, 180.0, 270.0] {
                    let r = if degrees % 180.0 == 0.0 { area.h / 2.0 - 12.0 } else { area.w / 2.0 - 12.0 };
                    let tip = area.polar(degrees, r);
                    let (l, rt) = (area.polar(degrees - 8.0, r - 10.0), area.polar(degrees + 8.0, r - 10.0));
                    let _ = write!(s, r#"<path d="M {} {} L {} {} L {} {} Z" fill="{EDGE}"/>"#, tip.0, tip.1, l.0, l.1, rt.0, rt.1);
                }
                let _ = write!(s, r#"<circle cx="{cx}" cy="{cy}" r="3" fill="{EDGE}"/>"#);
            }
            TouchpadMotion::Mouse { .. } | TouchpadMotion::Off => {}
        }
    }

    /// The labels pinned over the drawing: a menu's items where they lie, or what the side does.
    fn labels<'a, M: 'a>(&self) -> Vec<Element<'a, M>> {
        let mut pins = Vec::new();
        let label = |content: String, picked: bool, (x, y): (f32, f32), (w, h): (f32, f32)| -> Element<'a, M> {
            let t = text(content).size(11).color(if picked { PICKED } else { LABEL }).wrapping(text::Wrapping::WordOrGlyph).center();
            pin(container(t).center_x(w.max(1.0)).center_y(h.max(1.0)).clip(true)).x(x).y(y).into()
        };
        for area in self.areas() {
            let middle = (area.x + 4.0, 0.0);
            let whole = (area.w - 8.0, area.h);
            match area.motion {
                TouchpadMotion::Menu { menu, .. } => {
                    let Some(m) = self.menu_of(&area) else {
                        pins.push(label(format!("No menu “{menu}”"), false, middle, whole));
                        continue;
                    };
                    let picked = self.picked(&area, m);
                    for (i, (shape, item)) in item_shapes(m).into_iter().zip(&m.items).enumerate() {
                        let (at, size) = match shape {
                            ItemShape::Cell(x0, y0, x1, y1) => {
                                let ((ax, ay), (bx, by)) = (area.at((x0, y0)), area.at((x1, y1)));
                                ((ax + 2.0, ay), (bx - ax - 4.0, by - ay))
                            }
                            ItemShape::Slice(a, b) => {
                                let (x, y) = area.polar((a + b) / 2.0, area.w.min(area.h) * 0.36);
                                ((x - 40.0, y - 9.0), (80.0, 18.0))
                            }
                            ItemShape::Hidden => continue,
                        };
                        pins.push(label(item.label.clone(), picked.contains(&i), at, size));
                    }
                }
                TouchpadMotion::Swipe { .. } => {
                    for (dir, (u, v)) in crate::config::SwipeDirection::ALL.into_iter().zip([(0.5, 0.2), (0.5, 0.8), (0.2, 0.5), (0.8, 0.5)]) {
                        let action = area.motion.swipe(dir).filter(|a| **a != crate::config::ButtonAction::Disabled);
                        let Some(action) = action else { continue };
                        let (x, y) = area.at((u, v));
                        let box_w = (area.w * 0.36).max(40.0);
                        pins.push(label(action.summary(), false, (x - box_w / 2.0, y - 9.0), (box_w, 18.0)));
                    }
                }
                motion => {
                    let what = match motion {
                        TouchpadMotion::Off => "Off".to_string(),
                        TouchpadMotion::Mouse { .. } => "Mouse".to_string(),
                        TouchpadMotion::Joystick { stick, relative, .. } => {
                            format!("{stick}{}", if *relative { "\ncenters where you touch" } else { "" })
                        }
                        TouchpadMotion::Menu { .. } | TouchpadMotion::Swipe { .. } => String::new(),
                    };
                    let side = if self.areas().len() > 1 { if area.side == TouchSide::Left { "Left half: " } else { "Right half: " } } else { "" };
                    pins.push(label(format!("{side}{what}"), false, middle, whole));
                }
            }
        }
        pins
    }
}

/// The picture as a widget.
pub fn touchpad_picture<'a, M: 'a>(picture: &PadPicture<'_>) -> Element<'a, M> {
    let (w, h) = picture.size();
    let body = svg::Handle::from_memory(picture.svg().into_bytes());
    let mut layers: Vec<Element<'a, M>> = vec![svg(body).width(w).height(h).into()];
    layers.extend(picture.labels());
    stack(layers).width(w).height(h).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ButtonAction, MenuItem, MenuKind, OverlayStyle, Stick, TouchChoose, TouchRegion};

    #[test]
    fn every_mode_draws_and_fingers_light_up_their_item() {
        let items = ["A", "B"].map(|l| MenuItem::new(l, ButtonAction::Disabled)).to_vec();
        let menus = [
            Menu { name: "Grid".into(), kind: MenuKind::Grid { columns: 2 }, items: items.clone(), cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() },
            Menu { name: "Ring".into(), kind: MenuKind::Radial { stick: Stick::Right, boxes: false }, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() },
        ];
        let modes = [
            TouchpadMotion::Off,
            TouchpadMotion::mouse(1000.0),
            TouchpadMotion::Joystick { stick: Stick::Left, relative: true, radius: 0.2, deadzone: 0.1 },
            TouchpadMotion::Menu { menu: "Ring".into(), choose: TouchChoose::Click, overlay: true },
            TouchpadMotion::Menu { menu: "Missing".into(), choose: TouchChoose::Click, overlay: true },
            TouchpadMotion::Swipe { up: ButtonAction::Keys(vec!["KEY_W".into()]), down: ButtonAction::Disabled, left: ButtonAction::Disabled, right: ButtonAction::Disabled, distance: 0.2 },
        ];
        let menus: Vec<&Menu> = menus.iter().collect();
        for motion in modes {
            let cfg = TouchpadConfig { motion: motion.clone(), click: ButtonAction::Disabled, right: Some(TouchRegion { motion, click: ButtonAction::Disabled }) };
            let picture = PadPicture { settings: Some((&cfg, &menus)), touches: &[(0.2, 0.2)], click: true, height: 0.5, width: 300.0 };
            let svg = picture.svg();
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
            let _: Element<'_, ()> = touchpad_picture(&picture);
        }
        let grid = TouchpadConfig { motion: TouchpadMotion::Menu { menu: "Grid".into(), choose: TouchChoose::Click, overlay: true }, ..TouchpadConfig::default() };
        let picture = PadPicture { settings: Some((&grid, &menus)), touches: &[(0.9, 0.5)], click: false, height: 0.5, width: 300.0 };
        let area = &picture.areas()[0];
        assert_eq!(picture.picked(area, menus[0]), [1]);
        assert!(picture.svg().contains(ACCENT));
    }
}
