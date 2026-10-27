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

use crate::overlay::art::{Pattern, Shape};

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

struct Anchored {
    pattern: Pattern,
    shape: Shape,
    opacity: f32,
}

/// A layer the size of what it covers, cut to `shape` and faded by `fade`, showing the part
/// of a picture that spans the panel (see `anchor`). The pattern's `build(x, y, w, h)` makes the
/// pixels of the `w` by `h` part whose top left corner is `x`, `y` pixels into the panel. Its
/// `key` names the picture, so the parts drawn lately are kept and a layer that holds its place
/// doesn't rebuild every frame.
pub fn sized<'a, M: 'a>(pattern: Pattern, shape: Shape, fade: f32) -> Element<'a, M> {
    let opacity = (pattern.opacity * fade).clamp(0.0, 1.0);
    Element::new(Anchored { pattern, shape, opacity })
}

impl<M, T, R: image::Renderer<Handle = image::Handle>> Widget<M, T, R> for Anchored {
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
        let handle = cached((self.pattern.key, self.shape.key(), [x, y, w, h]), || self.shape.cut((self.pattern.build)(x, y, w, h), w, h));
        let picture = image::Image { opacity: self.opacity, snap: true, ..image::Image::new(handle) };
        renderer.draw_image(picture, bounds, bounds);
    }
}

type Key = ([u32; 3], u64, [usize; 4]);

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
