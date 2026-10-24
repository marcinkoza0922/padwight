//! A filter's picture laid down once over the whole panel and cut where each layer sits: a
//! layer shows the part of the picture under it, so a filter runs on unbroken across the keys
//! and items it covers rather than starting over in each one.

use std::{cell::Cell, collections::HashMap, sync::Mutex};

use iced::{
    Element, Length, Point, Rectangle, Size,
    advanced::{
        Layout, Widget, image, layout, mouse, renderer,
        widget::Tree,
    },
};

thread_local! {
    /// Where the panel being drawn starts: its pictures' top left corner.
    static ORIGIN: Cell<Point> = const { Cell::new(Point::ORIGIN) };
}

/// Marks where a panel starts, for the filters in it. It draws nothing, and must lie under
/// everything else in the panel so it is drawn first.
pub fn anchor<'a, M: 'a>() -> Element<'a, M> {
    Element::new(Anchor)
}

struct Anchor;

impl<M, T, R: iced::advanced::Renderer> Widget<M, T, R> for Anchor {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(&self, _tree: &Tree, _renderer: &mut R, _theme: &T, _style: &renderer::Style, layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle) {
        ORIGIN.set(layout.bounds().position());
    }
}

type Build<'a> = Box<dyn Fn(usize, usize, usize, usize) -> Vec<u8> + 'a>;

struct Anchored<'a> {
    key: [u32; 3],
    round: f32,
    opacity: f32,
    build: Build<'a>,
}

/// A layer the size of what it covers, rounded `round` pixels at its corners, showing the part
/// of a picture that spans the panel (see `anchor`). `build(x, y, w, h)` makes the pixels of the
/// `w` by `h` part whose top left corner is `x`, `y` pixels into the panel. `key` names the picture, so the
/// parts drawn lately are kept and a layer that holds its place doesn't rebuild every frame.
pub fn sized<'a, M: 'a>(key: [u32; 3], round: f32, opacity: f32, build: impl Fn(usize, usize, usize, usize) -> Vec<u8> + 'a) -> Element<'a, M> {
    Element::new(Anchored { key, round, opacity, build: Box::new(build) })
}

impl<M, T, R: image::Renderer<Handle = image::Handle>> Widget<M, T, R> for Anchored<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(&self, _tree: &Tree, renderer: &mut R, _theme: &T, _style: &renderer::Style, layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle) {
        let bounds = layout.bounds();
        let (w, h) = (bounds.width.max(1.0) as usize, bounds.height.max(1.0) as usize);
        let origin = ORIGIN.get();
        let (x, y) = ((bounds.x - origin.x).round().max(0.0) as usize, (bounds.y - origin.y).round().max(0.0) as usize);
        let handle = cached((self.key, self.round.to_bits(), [x, y, w, h]), || rounded((self.build)(x, y, w, h), w, h, self.round));
        let picture = image::Image { opacity: self.opacity, snap: true, ..image::Image::new(handle) };
        renderer.draw_image(picture, bounds, bounds);
    }
}

/// The pixels with what's outside corners `round` pixels round made clear, the edge softened.
fn rounded(mut pixels: Vec<u8>, w: usize, h: usize, round: f32) -> Vec<u8> {
    let r = round.min(w.min(h) as f32 / 2.0);
    if r < 1.0 {
        return pixels;
    }
    let reach = r.ceil() as usize;
    for y in (0..reach).chain(h.saturating_sub(reach).max(reach)..h) {
        for x in (0..reach).chain(w.saturating_sub(reach).max(reach)..w) {
            // How far the pixel's middle lies outside the corner's circle.
            let dx = (r - (x as f32 + 0.5)).max(x as f32 + 0.5 - (w as f32 - r));
            let dy = (r - (y as f32 + 0.5)).max(y as f32 + 0.5 - (h as f32 - r));
            if dx <= 0.0 || dy <= 0.0 {
                continue;
            }
            let cover = (r + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let alpha = &mut pixels[(y * w + x) * 4 + 3];
            *alpha = (f32::from(*alpha) * cover) as u8;
        }
    }
    pixels
}

type Key = ([u32; 3], u32, [usize; 4]);

/// How many bytes of pictures are kept: a panel's background or two, and every key of a full
/// keyboard under several filters.
const KEPT_BYTES: usize = 64 << 20;

#[derive(Default)]
struct Recent {
    pictures: HashMap<Key, (image::Handle, u64)>,
    bytes: usize,
    clock: u64,
}

/// The picture for `key`, built the first time. The ones least recently used go once the rest
/// pass the budget: each key of a keyboard has its own part of every filter on it.
fn cached(key: Key, build: impl FnOnce() -> Vec<u8>) -> image::Handle {
    static RECENT: Mutex<Option<Recent>> = Mutex::new(None);
    let mut recent = RECENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let recent = recent.get_or_insert_with(Recent::default);
    recent.clock += 1;
    let clock = recent.clock;
    if let Some((handle, used)) = recent.pictures.get_mut(&key) {
        *used = clock;
        return handle.clone();
    }
    let [_, _, w, h] = key.2;
    let handle = image::Handle::from_rgba(w as u32, h as u32, build());
    recent.bytes += w * h * 4;
    while recent.bytes > KEPT_BYTES {
        let Some(oldest) = recent.pictures.iter().min_by_key(|(_, (_, used))| *used).map(|(k, _)| *k) else { break };
        recent.pictures.remove(&oldest);
        recent.bytes -= oldest.2[2] * oldest.2[3] * 4;
    }
    recent.pictures.insert(key, (handle.clone(), clock));
    handle
}
