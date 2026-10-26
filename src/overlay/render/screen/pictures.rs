//! Each panel drawn once into a picture of its own, and laid onto the screen as it fades, slides
//! and zooms.
//!
//! A panel is a layer at the top level of the screen's list (see `screen_ops`). Its ops are
//! drawn at full strength, at the device scale, into a picture as big as their ink (shadow and
//! all) plus a clear pixel all round, in the screen's own device pixels: the picture's corner
//! is on a whole pixel and the panel lies in it where it lies on the screen at rest, so a
//! picture laid down unmoved is the same as drawing the ops straight onto the screen. The
//! picture is kept, keyed by the list's `digest` and the scale and checked against the list
//! itself, while the list stays the same; a few are kept, the least recently used dropped.
//!
//! Laying it down: pixmaps hold premultiplied color, so a picture laid at opacity `a` has all
//! four channels multiplied by `a` (tiny-skia's pattern opacity) and goes over the screen
//! source-over: every part of the panel fades by the same amount, so an outline never shows
//! ahead of the label inside it. Unzoomed, its offset is rounded to whole device pixels and the
//! picture copied pixel for pixel, so text doesn't shimmer as it slides. Zoomed, it is sampled
//! bilinearly; sampling premultiplied pixels, and the clear pixel round the edge, keep its rim
//! free of dark fringes and hard cut edges.
//!
//! What moves inside a panel stays in its list as layers: a motion style that staggers its
//! items fades each in as a layer of its own, so through the open the list changes every frame
//! and the picture is drawn anew each time (the panel as a whole still fades as one picture
//! over it). Once the items are in, the list settles and the picture is reused. The same goes
//! for a highlight gliding, an item swelling or a gradient drifting.
//!
//! The picture's corner is on a whole pixel, so the ops land on the same fraction of a pixel as
//! drawn straight; only a glyph right at the edge of a sub-pixel bin can fall into the next one
//! (the float sums differ slightly), a level or so of difference at one glyph now and then.

use tiny_skia::{FilterQuality, Pixmap, PixmapMut, PixmapPaint, Transform};

use super::{Shown, screen_ops};
use crate::overlay::{
    OverlayFrame,
    render::{Layer, Op, Point, bounds, digest, raster},
};

/// How many panel pictures are kept besides those on screen: enough for the panels open at
/// once and the few they change between (a list's highlight moving back and forth).
const KEPT: usize = 8;
/// Far past any screen, as a guard against a runaway size.
const LIMIT: i32 = 16_384;

/// A panel's list drawn into a picture.
struct Kept {
    digest: u64,
    /// The device scale's bits.
    scale: u32,
    ops: Vec<Op>,
    pixmap: Pixmap,
    /// Where the picture's top left corner lies on the screen at rest, in device pixels.
    origin: (i32, i32),
    /// The frame it was last laid down in.
    used: u64,
}

/// The screen drawn on: its size in logical pixels, and device pixels per logical one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    pub size: Point,
    pub scale: f32,
}

/// Draws whole screens, keeping each panel's picture from one frame to the next.
#[derive(Default)]
pub struct Screen {
    kept: Vec<Kept>,
    frame: u64,
    drawn: u64,
}

impl Screen {
    pub fn new() -> Screen {
        Screen::default()
    }

    /// Draws `shown` on `display`, over what `target` (as big as the display in device pixels)
    /// holds: clear it first for a fresh screen.
    pub fn draw(&mut self, frame: &OverlayFrame, shown: &Shown, display: Display, target: &mut PixmapMut) {
        self.lay(&screen_ops(frame, shown, display.size), display.scale, target);
    }

    /// Draws a screen's list: each layer at its top level is a panel, laid down from its
    /// picture; anything else there is drawn as it is.
    pub fn lay(&mut self, ops: &[Op], scale: f32, target: &mut PixmapMut) {
        self.frame += 1;
        let straight = Transform::from_scale(scale, scale);
        let mut at = 0;
        while let Some(op) = ops.get(at) {
            at = match op {
                Op::Layer(layer) => {
                    let (end, next) = close(ops, at + 1);
                    self.panel(layer, &ops[at + 1..end], scale, target);
                    next
                }
                Op::Clip { .. } => {
                    let next = close(ops, at + 1).1;
                    raster::draw(&ops[at..next], target, straight);
                    next
                }
                // A close with nothing open is passed over, as the rasterizer does.
                Op::Unlayer | Op::Unclip => at + 1,
                op => {
                    raster::draw(std::slice::from_ref(op), target, straight);
                    at + 1
                }
            };
        }
        self.prune();
    }

    /// How many panel pictures have been drawn (rather than reused) so far.
    pub fn drawn(&self) -> u64 {
        self.drawn
    }

    /// How many pictures are kept.
    #[cfg(test)]
    pub(super) fn kept(&self) -> usize {
        self.kept.len()
    }

    /// Drops every picture, as when the overlay hides.
    pub fn forget(&mut self) {
        self.kept.clear();
    }

    /// Lays down a panel's picture, drawing it first unless it is kept.
    fn panel(&mut self, layer: &Layer, ops: &[Op], scale: f32, target: &mut PixmapMut) {
        if !(layer.opacity > 0.0 && layer.scale > 0.0) {
            return;
        }
        let key = (digest(ops), scale.to_bits());
        let found = self.kept.iter().position(|k| (k.digest, k.scale) == key && k.ops == ops);
        let i = match found {
            Some(i) => i,
            None => {
                let Some((pixmap, origin)) = picture(ops, scale) else { return };
                self.drawn += 1;
                self.kept.push(Kept { digest: key.0, scale: key.1, ops: ops.to_vec(), pixmap, origin, used: 0 });
                self.kept.len() - 1
            }
        };
        let kept = &mut self.kept[i];
        kept.used = self.frame;
        composite(&kept.pixmap, kept.origin, layer, scale, target);
    }

    /// Keeps the pictures laid down this frame and the most recently used of the rest, up to
    /// `KEPT` of those.
    fn prune(&mut self) {
        let now = self.kept.iter().filter(|k| k.used == self.frame).count();
        self.kept.sort_by_key(|k| std::cmp::Reverse(k.used));
        self.kept.truncate(now + KEPT);
    }
}

/// Where the group opened just before `at` ends: the index of its close (or the end), and the
/// index after it.
fn close(ops: &[Op], mut at: usize) -> (usize, usize) {
    let mut depth = 0;
    while let Some(op) = ops.get(at) {
        match op {
            Op::Clip { .. } | Op::Layer(_) => depth += 1,
            Op::Unclip | Op::Unlayer if depth == 0 => return (at, at + 1),
            Op::Unclip | Op::Unlayer => depth -= 1,
            _ => {}
        }
        at += 1;
    }
    (at, at)
}

/// `ops` drawn at `scale` into a clear picture covering their ink and a pixel more all round,
/// and where its corner lies in device pixels; `None` when they draw nothing.
fn picture(ops: &[Op], scale: f32) -> Option<(Pixmap, (i32, i32))> {
    let ink = bounds(ops)?;
    let edge = |v: f32| (v * scale).clamp(-LIMIT as f32, LIMIT as f32);
    let (x0, y0) = (edge(ink.x).floor() as i32 - 1, edge(ink.y).floor() as i32 - 1);
    let (x1, y1) = (edge(ink.right()).ceil() as i32 + 1, edge(ink.bottom()).ceil() as i32 + 1);
    if x1 - x0 > LIMIT || y1 - y0 > LIMIT {
        return None;
    }
    let mut pixmap = Pixmap::new((x1 - x0) as u32, (y1 - y0) as u32)?;
    raster::draw(ops, &mut pixmap.as_mut(), Transform::from_row(scale, 0.0, 0.0, scale, -x0 as f32, -y0 as f32));
    Some((pixmap, (x0, y0)))
}

/// Lays `pixmap`, whose corner lies at `origin` (device pixels) at rest, onto `target` as
/// `layer` moves it: faded, moved, and zoomed about the layer's point.
fn composite(pixmap: &Pixmap, origin: (i32, i32), layer: &Layer, scale: f32, target: &mut PixmapMut) {
    let opacity = layer.opacity.clamp(0.0, 1.0);
    let z = layer.scale;
    if (z - 1.0).abs() < 1e-4 {
        let (dx, dy) = ((layer.shift.x * scale).round() as i32, (layer.shift.y * scale).round() as i32);
        let paint = PixmapPaint { opacity, quality: FilterQuality::Nearest, ..PixmapPaint::default() };
        target.draw_pixmap(origin.0 + dx, origin.1 + dy, pixmap.as_ref(), &paint, Transform::identity(), None);
        return;
    }
    // Picture pixel u is device point origin + u at rest, logical (origin + u) / scale, which
    // the layer takes to about + shift + (p - about) * z.
    let (a, s) = (layer.about, layer.shift);
    let tx = z * origin.0 as f32 + scale * (a.x * (1.0 - z) + s.x);
    let ty = z * origin.1 as f32 + scale * (a.y * (1.0 - z) + s.y);
    let paint = PixmapPaint { opacity, quality: FilterQuality::Bilinear, ..PixmapPaint::default() };
    target.draw_pixmap(0, 0, pixmap.as_ref(), &paint, Transform::from_row(z, 0.0, 0.0, z, tx, ty), None);
}
