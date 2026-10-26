//! The whole screen as the overlay window shows it: every panel built, placed by its position,
//! margin, offset and animation, and drawn into one picture. This is `ui::layers` for the
//! overlay's own renderer.
//!
//! Panels are drawn in this order, bottom up: the stacks of info and log panels, the editor's
//! backdrop (the menu being edited), then the panels (the keyboard or a menu goes on top of the
//! info and log overlays), a question asked over a menu on top of it. With the Guide overlay up
//! the info stacks at the middle of the left and right edges stand beside its menu instead.
//!
//! Each panel is built at full strength and is one layer at the top level of the list: its
//! opacity, slide and zoom are that layer's, never its colors'. `Screen` (`pictures.rs`) draws
//! each such layer's list once into a picture, keeps it while the list stays the same, and lays
//! it down at the layer's opacity, offset and zoom, so a panel fades and moves as one picture.
//! What moves inside a panel (an item's stagger fade, highlight or swell) changes its list,
//! which is then drawn again. Damage is step 4.3.

mod panels;
mod pictures;
mod stacks;
#[cfg(test)]
mod fades;
#[cfg(test)]
mod tests;

use tiny_skia::Pixmap;

use super::{Node, Op, Point, Rect, Rgba, place};
use crate::{
    config::OverlayStyle,
    info::{Glyphs, InfoView},
    inputlog::LogView,
    motion::{Anim, MotionSet, MotionStyle},
    overlay::{OverlayFrame, OverlayView, fit::Fit},
    snapshot::Image,
};

pub use pictures::{Display, Screen};

/// What is on screen at one moment, each panel with how it is moving.
pub struct Shown<'a> {
    pub panels: Vec<(&'a OverlayView, Anim)>,
    pub info: Vec<(&'a InfoView, Anim)>,
    pub logs: Vec<(&'a LogView, Anim)>,
}

impl<'a> Shown<'a> {
    /// What `frame` shows once everything has finished opening.
    pub fn settled(frame: &'a OverlayFrame) -> Shown<'a> {
        Shown {
            panels: frame.active.iter().map(|v| (v, Anim::still())).collect(),
            info: frame.info.iter().map(|v| (v, Anim::still())).collect(),
            logs: frame.logs.iter().map(|v| (v, Anim::still())).collect(),
        }
    }
}

/// What every panel is drawn with besides its own view, and the display it is drawn for.
#[derive(Debug, Clone, Copy)]
pub(super) struct Context {
    pub font: Option<&'static str>,
    pub glyphs: Glyphs,
    pub colorblind: bool,
    pub display: Point,
}

impl Context {
    fn of(frame: &OverlayFrame, display: Point) -> Context {
        Context {
            font: crate::font::family(frame.font.as_deref()),
            glyphs: Glyphs { family: frame.family, nintendo_layout: frame.nintendo_layout },
            colorblind: frame.colorblind,
            display,
        }
    }

    /// `style`'s size limits on this display.
    pub(super) fn fit(&self, style: &OverlayStyle) -> Fit {
        Fit::within(style, self.display.x, self.display.y)
    }
}

/// The ops that draw `shown` on a `display` (logical pixels), bottom up. Each panel is a layer
/// at the top level of the list (see `Node::layered`), laid out where it rests; the layer fades
/// it and moves and zooms it by its animation.
pub fn screen_ops(frame: &OverlayFrame, shown: &Shown, display: Point) -> Vec<Op> {
    let cx = Context::of(frame, display);
    let mut ops = Vec::new();
    let guide = stacks::guide_up(frame, shown, &cx);
    let mut sides = stacks::put_stacks(&mut ops, shown, &cx, guide);
    if let Some(menu) = &frame.backdrop {
        let anim = Anim::still();
        put(&mut ops, panels::menu(menu, &cx, anim), &menu.style, &cx, &anim);
    }
    if frame.peek {
        return ops;
    }
    for (view, anim) in &shown.panels {
        match view {
            OverlayView::Menu(m) => {
                let menu = panels::menu(m, &cx, *anim);
                if guide {
                    stacks::beside(&mut ops, std::mem::take(&mut sides), menu, anim, &cx);
                } else {
                    put(&mut ops, menu, &m.style, &cx, anim);
                }
                // A question asked over the menu stands on top of it, in the middle of the
                // screen, a picture of its own; the menu under it is dimmed in its own colors.
                if let Some(o) = &m.prompt {
                    put(&mut ops, panels::offer(o, &cx), &o.style, &cx, &Anim::still());
                }
            }
            other => put(&mut ops, panels::any(other, &cx, anim), other.style(), &cx, anim),
        }
    }
    ops
}

/// Places `panel` as a layer of its own where `style` puts it at rest, the layer faded, moved
/// and zoomed (about the panel's center) by its animation. The move is from where the panel
/// rests to where the animation's offset puts it, held at the screen's edge as `place` holds
/// it, so the panel's list stays the same as it slides.
fn put(ops: &mut Vec<Op>, panel: Node, style: &OverlayStyle, cx: &Context, anim: &Anim) {
    let fit = cx.fit(style);
    let rest = (fit.x, fit.y);
    let room = place::room(cx.display, place::margins(style.position, rest));
    let size = panel.measure(room.x);
    let at = place::origin(cx.display, size, style.position, rest);
    let (x, y) = anim.offset();
    let moved = place::origin(cx.display, size, style.position, (rest.0 + x, rest.1 + y));
    let shift = Point::new(moved.x - at.x, moved.y - at.y);
    let panel = panel.opacity(anim.opacity()).scale(anim.scale().max(0.0)).shift(shift).layered();
    panel.place(Rect::new(at.x, at.y, size.x, size.y), ops);
}

/// Places `node` where `style` puts it, as it is: a stack whose panels are layers each.
pub(super) fn put_at(ops: &mut Vec<Op>, node: &Node, style: &OverlayStyle, cx: &Context) {
    let fit = cx.fit(style);
    let shift = (fit.x, fit.y);
    let room = place::room(cx.display, place::margins(style.position, shift));
    let size = node.measure(room.x);
    let at = place::origin(cx.display, size, style.position, shift);
    node.place(Rect::new(at.x, at.y, size.x, size.y), ops);
}

/// Draws `frame` on a `display` (logical pixels) at `scale` over the opaque `background`, as
/// the overlay window shows it once everything has finished opening.
pub fn screen(frame: &OverlayFrame, display: Point, scale: f32, background: Rgba) -> Option<Image> {
    let mut frame = frame.clone();
    frame.motion = MotionSet::all(MotionStyle::Off);
    frame.apply_lite();
    let mut pixmap = Pixmap::new((display.x * scale).round() as u32, (display.y * scale).round() as u32)?;
    pixmap.fill(tiny_skia::Color::from_rgba(background.r, background.g, background.b, 1.0)?);
    Screen::new().draw(&frame, &Shown::settled(&frame), Display { size: display, scale }, &mut pixmap.as_mut());
    let rgba = pixmap.data().as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    Some(Image { width: pixmap.width(), height: pixmap.height(), rgba })
}
