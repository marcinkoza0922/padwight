//! The whole screen as the overlay window shows it: every panel built, placed by its position,
//! margin, offset and animation, and drawn into one picture. This is `ui::layers` for the
//! overlay's own renderer.
//!
//! Panels are drawn in this order, bottom up: the stacks of info and log panels, the editor's
//! backdrop (the menu being edited), then the panels (the keyboard or a menu goes on top of the
//! info and log overlays), a question asked over a menu on top of it. With the Guide overlay up
//! the info stacks at the middle of the left and right edges stand beside its menu instead.
//!
//! Each panel fades itself (its colors carry the animation's opacity, as in iced); the screen
//! moves and zooms it as a layer. Whole-panel pictures and damage are steps 4.2 and 4.3.

mod panels;
mod stacks;
#[cfg(test)]
mod tests;

use tiny_skia::{Pixmap, Transform};

use super::{Layer, Node, Op, Point, Rect, Rgba, place, raster};
use crate::{
    config::OverlayStyle,
    info::{Glyphs, InfoView},
    inputlog::LogView,
    motion::{Anim, MotionSet, MotionStyle},
    overlay::{OverlayFrame, OverlayView, fit::Fit},
    snapshot::Image,
};

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

/// The ops that draw `shown` on a `display` (logical pixels), bottom up.
pub fn screen_ops(frame: &OverlayFrame, shown: &Shown, display: Point) -> Vec<Op> {
    let cx = Context::of(frame, display);
    let mut ops = Vec::new();
    let guide = stacks::guide_up(frame, shown, &cx);
    let mut sides = stacks::put_stacks(&mut ops, shown, &cx, guide);
    if let Some(menu) = &frame.backdrop {
        let anim = Anim::still();
        put(&mut ops, &panels::menu(menu, &cx, anim), &menu.style, &cx, &anim);
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
                    put(&mut ops, &menu, &m.style, &cx, anim);
                }
                // A question asked over the menu stands on top of it, in the middle of the screen.
                if let Some(o) = &m.prompt {
                    put(&mut ops, &panels::offer(o, &cx, &Anim::still()), &o.style, &cx, &Anim::still());
                }
            }
            other => put(&mut ops, &panels::any(other, &cx, anim), other.style(), &cx, anim),
        }
    }
    ops
}

/// Places `panel` on the screen where `style` puts it, moved by its animation and the style's
/// offset, and zoomed about its center by the animation.
pub(super) fn put(ops: &mut Vec<Op>, panel: &Node, style: &OverlayStyle, cx: &Context, anim: &Anim) {
    let (x, y) = anim.offset();
    let fit = cx.fit(style);
    let shift = (x + fit.x, y + fit.y);
    let room = place::room(cx.display, place::margins(style.position, shift));
    let size = panel.measure(room.x);
    let at = place::origin(cx.display, size, style.position, shift);
    let rect = Rect::new(at.x, at.y, size.x, size.y);
    let scale = anim.scale().max(0.0);
    let zoomed = (scale - 1.0).abs() >= 1e-4;
    if zoomed {
        ops.push(Op::Layer(Layer { opacity: 1.0, scale, about: rect.center(), shift: Point::ZERO }));
    }
    panel.place(rect, ops);
    if zoomed {
        ops.push(Op::Unlayer);
    }
}

/// Draws `frame` on a `display` (logical pixels) at `scale` over the opaque `background`, as
/// the overlay window shows it once everything has finished opening.
pub fn screen(frame: &OverlayFrame, display: Point, scale: f32, background: Rgba) -> Option<Image> {
    let mut frame = frame.clone();
    frame.motion = MotionSet::all(MotionStyle::Off);
    frame.apply_lite();
    let ops = screen_ops(&frame, &Shown::settled(&frame), display);
    let mut pixmap = Pixmap::new((display.x * scale).round() as u32, (display.y * scale).round() as u32)?;
    pixmap.fill(tiny_skia::Color::from_rgba(background.r, background.g, background.b, 1.0)?);
    raster::draw(&ops, &mut pixmap.as_mut(), Transform::from_scale(scale, scale));
    let rgba = pixmap.data().as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    Some(Image { width: pixmap.width(), height: pixmap.height(), rgba })
}
