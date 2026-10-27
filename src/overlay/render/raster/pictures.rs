//! Pictures (PNG and JPEG through `image`), SVGs (through resvg) and filter patterns, each kept
//! once made: a picture by its file, an SVG by its file or markup and the pixel size it was
//! drawn at, and a pattern's part as `draw/texture/anchored.rs` keeps it.

use std::{cell::RefCell, panic};

use resvg::usvg;
use tiny_skia::{ColorU8, FilterQuality, IntSize, Paint, Pixmap, PixmapPaint, SpreadMode, Transform};

use super::{Pen, blit, kept::Kept};
use crate::overlay::{
    art,
    render::{File, Fit, PatternRef, Point, Rect, Svg},
};

/// Where an SVG comes from, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Source {
    File(File),
    Markup(u64),
}

/// A pattern's part: its key, the shape it is cut to, and its place and size in the picture.
type PartKey = ([u32; 3], u64, [usize; 4]);

struct Caches {
    trees: Kept<Source, usvg::Tree>,
    svgs: Kept<(Source, u32, u32), Pixmap>,
    files: Kept<File, Pixmap>,
    patterns: Kept<PartKey, Pixmap>,
}

thread_local! {
    static KEPT: RefCell<Caches> = RefCell::new(Caches {
        trees: Kept::new(8 << 20),
        svgs: Kept::new(32 << 20),
        files: Kept::new(64 << 20),
        // As `anchored.rs` keeps: a panel's background or two, and every key of a full keyboard
        // under several filters.
        patterns: Kept::new(64 << 20),
    });
}

/// Drops every picture, SVG and pattern kept, as when the overlay hides.
pub fn forget() {
    KEPT.with_borrow_mut(|kept| {
        kept.trees.clear();
        kept.svgs.clear();
        kept.files.clear();
        kept.patterns.clear();
    });
}

/// How many SVG and pattern pixmaps are kept, for tests.
#[cfg(test)]
pub(super) fn kept_counts() -> (usize, usize) {
    KEPT.with_borrow(|kept| (kept.svgs.len(), kept.patterns.len()))
}

/// Where content `size` big lands in `rect`, fitted as `fit` says, centered.
pub(super) fn fitted(rect: Rect, (w, h): (f32, f32), fit: Fit) -> Rect {
    let (sx, sy) = (rect.w / w, rect.h / h);
    let s = match fit {
        Fit::Contain => sx.min(sy),
        Fit::Cover => sx.max(sy),
        Fit::Stretch => return rect,
    };
    let (fw, fh) = (w * s, h * s);
    Rect::new(rect.x + (rect.w - fw) / 2.0, rect.y + (rect.h - fh) / 2.0, fw, fh)
}

/// `r` moved out to whole device pixels at its edges, and how many pixels that is each way.
/// Halves round up on either side of the pixmap's corner (`round` takes them away from it), so
/// a picture drawn in tiles snaps the same in each.
fn snapped(pen: &Pen, r: Rect) -> (Rect, u32, u32) {
    let ([x0, y0, x1, y1], t) = (pen.edges(r), pen.transform);
    let round = |v: f32| (v + 0.5).floor();
    let (x0, y0, x1, y1) = (round(x0), round(y0), round(x1), round(y1));
    let back = Rect::new((x0 - t.tx) / t.sx, (y0 - t.ty) / t.sy, (x1 - x0) / t.sx, (y1 - y0) / t.sy);
    (back, (x1 - x0).max(0.0) as u32, (y1 - y0).max(0.0) as u32)
}

/// Lays `image` stretched over `place`, showing what of it lies in `visible`, at the paint's
/// opacity and filtered by its quality.
fn lay(pen: &mut Pen, image: &Pixmap, place: Rect, visible: Rect, how: PixmapPaint) {
    let Some(shown) = place.intersection(visible) else { return };
    if shown == place
        && let Some(at) = blit::whole(pen, place)
    {
        return blit::blit(pen, image, at, how.opacity);
    }
    let Some(shown) = tiny_skia::Rect::from_xywh(shown.x, shown.y, shown.w, shown.h) else { return };
    let (iw, ih) = (image.width() as f32, image.height() as f32);
    let at = Transform::from_row(place.w / iw, 0.0, 0.0, place.h / ih, place.x, place.y);
    let shader = tiny_skia::Pattern::new(image.as_ref(), SpreadMode::Pad, how.quality, how.opacity.clamp(0.0, 1.0), at);
    pen.pixmap.fill_rect(shown, &Paint { shader, anti_alias: true, ..Paint::default() }, pen.transform, pen.mask);
}

/// Straight RGBA pixels as a pixmap, premultiplied.
fn premultiplied(w: u32, h: u32, mut rgba: Vec<u8>) -> Option<(Pixmap, usize)> {
    for p in rgba.as_chunks_mut::<4>().0 {
        let c = ColorU8::from_rgba(p[0], p[1], p[2], p[3]).premultiply();
        *p = [c.red(), c.green(), c.blue(), c.alpha()];
    }
    let bytes = rgba.len();
    Some((Pixmap::from_vec(rgba, IntSize::from_wh(w, h)?)?, bytes))
}

/// A PNG or JPEG fitted to `rect`, cut to it when it covers it.
pub(super) fn picture(pen: &mut Pen, file: &File, rect: Rect, fit: Fit, opacity: f32) {
    KEPT.with_borrow_mut(|kept| {
        let Some(image) = kept.files.get(file, || decode(file)) else { return };
        let place = fitted(rect, (image.width() as f32, image.height() as f32), fit);
        lay(pen, &image, place, rect, PixmapPaint { opacity, quality: FilterQuality::Bilinear, ..PixmapPaint::default() });
    });
}

fn decode(file: &File) -> Option<(Pixmap, usize)> {
    let image = image::ImageReader::open(file).ok()?.with_guessed_format().ok()?.decode().ok()?.to_rgba8();
    let (w, h) = image.dimensions();
    premultiplied(w, h, image.into_raw())
}

/// An SVG fitted to `rect`: drawn at the device pixels it covers, on whole pixels, so it stays
/// sharp, and cut to `rect` when it covers it.
pub(super) fn svg(pen: &mut Pen, svg: &Svg, rect: Rect, fit: Fit, opacity: f32) {
    let source = match svg {
        Svg::File(file) => Source::File(file.clone()),
        Svg::Markup(markup) => Source::Markup(markup.key()),
    };
    KEPT.with_borrow_mut(|kept| {
        let Some(tree) = kept.trees.get(&source, || parse(svg)) else { return };
        let size = tree.size();
        let (place, w, h) = snapped(pen, fitted(rect, (size.width(), size.height()), fit));
        let Some(image) = kept.svgs.get(&(source, w, h), || drawn(&tree, w, h)) else { return };
        lay(pen, &image, place, rect, PixmapPaint { opacity, quality: FilterQuality::Nearest, ..PixmapPaint::default() });
    });
}

fn parse(svg: &Svg) -> Option<(usvg::Tree, usize)> {
    let tree = match svg {
        Svg::File(file) => {
            let data = std::fs::read(file).ok()?;
            let options = usvg::Options { resources_dir: file.parent().map(Into::into), ..usvg::Options::default() };
            usvg::Tree::from_data(&data, &options).ok().map(|tree| (tree, data.len()))
        }
        Svg::Markup(markup) => usvg::Tree::from_str(markup.text(), &usvg::Options::default()).ok().map(|tree| (tree, markup.text().len())),
    };
    // A parsed tree takes a few times the room of its text.
    tree.map(|(tree, len)| (tree, len * 4))
}

/// The tree drawn into a `w` by `h` pixmap, stretched to it.
fn drawn(tree: &usvg::Tree, w: u32, h: u32) -> Option<(Pixmap, usize)> {
    let mut pixmap = Pixmap::new(w, h)?;
    let size = tree.size();
    let scale = Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    // resvg can panic on a malformed or odd picture; that one draws nothing.
    panic::catch_unwind(panic::AssertUnwindSafe(|| resvg::render(tree, scale, &mut pixmap.as_mut()))).ok()?;
    Some((pixmap, w as usize * h as usize * 4))
}

/// The part of a filter's picture under `rect`, as `anchored.rs` places and cuts it: built at
/// the rect's size in logical pixels, from where it lies past the panel's corner at `origin`,
/// cut to `cut`, then stretched over the rect on whole device pixels.
pub(super) fn pattern(pen: &mut Pen, pattern: &PatternRef, (rect, origin): (Rect, Point), cut: art::Shape, opacity: f32) {
    let (w, h) = (rect.w.max(1.0) as usize, rect.h.max(1.0) as usize);
    let (x, y) = ((rect.x - origin.x).round().max(0.0) as usize, (rect.y - origin.y).round().max(0.0) as usize);
    let (place, dw, dh) = snapped(pen, rect);
    KEPT.with_borrow_mut(|kept| {
        let make = || premultiplied(w as u32, h as u32, cut.cut((pattern.pattern().build)(x, y, w, h), w, h));
        let Some(image) = kept.patterns.get(&(pattern.key(), cut.key(), [x, y, w, h]), make) else { return };
        let quality = if (dw as usize, dh as usize) == (w, h) { FilterQuality::Nearest } else { FilterQuality::Bilinear };
        lay(pen, &image, place, place, PixmapPaint { opacity, quality, ..PixmapPaint::default() });
    });
}
