//! Sharing a box out among a node's children: rows (as iced's flex lays them out), columns,
//! wrapping rows and stacks, and pinned children. Each kind works out its children's sizes
//! the same way when measured and when placed, so they come out the same both times.

use super::{Kind, Length, Node, Op, Pin, Point, Rect, SLACK, share};

/// The width a node's contents are offered, and whether a fill among them shrinks to its
/// contents rather than taking it all (iced's compression).
#[derive(Debug, Clone, Copy)]
pub(super) struct Room {
    pub max: f32,
    pub squeeze: bool,
}

/// A child's size, and whether a fill inside it shrinks.
#[derive(Debug, Clone, Copy)]
pub(super) struct Slot {
    size: Point,
    squeeze: bool,
}

/// Where each length ends, laid end to end `spacing` apart, and where the last one ends.
fn ends(lengths: impl Iterator<Item = f32>, spacing: f32) -> f32 {
    lengths.enumerate().fold(0.0, |at, (i, l)| if i > 0 { at + spacing + l } else { at + l })
}

/// A Fill height's height in `room` tall, held to its own max.
fn tall(child: &Node, size: Point, room: f32) -> f32 {
    if child.height.fills() { room.min(child.max_height) } else { size.y }
}

/// The children of a row in `room`, and the size they take. Those that neither fill nor stretch
/// are laid out first, in order, each offered what is left; then those that stretch to the
/// row's height; then those that fill share what remains by their portions.
pub(super) fn row(node: &Node, children: &[Node], room: Room) -> (Vec<Slot>, Point) {
    // A row of set height is as tall as that, so a fill height needn't wait for the others.
    let stretch = !matches!(node.height, Length::Fixed(_));
    let fills = |c: &Node| !room.squeeze && c.width.fills();
    let stretches = |c: &Node| stretch && c.height.fills();
    let mut slots = vec![Slot { size: Point::ZERO, squeeze: room.squeeze }; children.len()];
    let mut left = room.max - node.spacing * children.len().saturating_sub(1) as f32;
    for pass in [false, true] {
        for (slot, c) in slots.iter_mut().zip(children).filter(|(_, c)| !fills(c) && stretches(c) == pass) {
            // A set width that stretches takes its width whatever is left, as in iced.
            let max = if pass && matches!(c.width, Length::Fixed(_)) { f32::INFINITY } else { left.max(0.0) };
            slot.size = c.size_in(Room { max, squeeze: room.squeeze });
            left -= slot.size.x;
        }
    }
    let portions: f32 = children.iter().filter(|c| fills(c)).map(|c| f32::from(c.width.portions())).sum();
    let remaining = left.max(0.0);
    for (slot, c) in slots.iter_mut().zip(children).filter(|(_, c)| fills(c)) {
        slot.size = c.size_in(Room { max: remaining * f32::from(c.width.portions()) / portions, squeeze: false });
    }
    let cross = slots.iter().zip(children).filter(|(_, c)| !stretches(c)).map(|(s, _)| s.size.y).fold(0.0, f32::max);
    let main = ends(slots.iter().map(|s| s.size.x), node.spacing);
    (slots, Point::new(main, cross))
}

/// The children of a column in `room`, and the size they take. In a column that shrinks to its
/// contents, one that fills is as wide as the widest of the others (or its own contents, alone).
pub(super) fn column(node: &Node, children: &[Node], room: Room) -> (Vec<Slot>, Point) {
    let fills = |c: &Node| room.squeeze && c.width.fills();
    let mut slots = vec![Slot { size: Point::ZERO, squeeze: false }; children.len()];
    let mut cross: f32 = 0.0;
    for (slot, c) in slots.iter_mut().zip(children).filter(|(_, c)| !fills(c)) {
        slot.size = c.size_in(Room { max: room.max, squeeze: false });
        cross = cross.max(slot.size.x);
    }
    let wide = if children.iter().all(fills) { Room { max: room.max, squeeze: true } } else { Room { max: cross, squeeze: false } };
    for (slot, c) in slots.iter_mut().zip(children).filter(|(_, c)| fills(c)) {
        *slot = Slot { size: c.size_in(wide), squeeze: wide.squeeze };
        cross = cross.max(slot.size.x);
    }
    let main = ends(slots.iter().map(|s| s.size.y), node.spacing);
    (slots, Point::new(cross, main))
}

/// Where each child of a wrapping row goes (from the top left of its contents), and the size
/// they take. A line is as tall as its tallest child, which the others line up in by
/// `align_y`; each line lines up inside the widest by `align_x`.
pub(super) fn wrap(node: &Node, children: &[Node], room: Room) -> (Vec<Rect>, Point) {
    let mut rects: Vec<Rect> = Vec::with_capacity(children.len());
    // Where each line starts in `rects`, and where it ends across.
    let mut lines: Vec<(usize, f32)> = Vec::new();
    let (mut x, mut y, mut line_h, mut widest) = (0.0, 0.0, 0.0_f32, 0.0_f32);
    for c in children {
        let size = c.size_in(Room { max: room.max, squeeze: room.squeeze });
        match lines.last_mut() {
            Some(line) if x + size.x <= room.max + SLACK => line.1 = x + size.x,
            Some(_) => {
                y += line_h + node.line_spacing.unwrap_or(node.spacing);
                (x, line_h) = (0.0, 0.0);
                lines.push((rects.len(), size.x));
            }
            None => lines.push((0, size.x)),
        }
        rects.push(Rect::new(x, y, size.x, size.y));
        widest = widest.max(x + size.x);
        line_h = line_h.max(size.y);
        x += size.x + node.spacing;
    }
    let height = rects.last().map_or(0.0, |_| y + line_h);
    for (k, &(start, end)) in lines.iter().enumerate() {
        let stop = lines.get(k + 1).map_or(rects.len(), |next| next.0);
        let line = &mut rects[start..stop];
        let tallest = line.iter().map(|r| r.h).fold(0.0, f32::max);
        for r in line {
            r.x += (widest - end) * share(node.align_x);
            r.y += (tallest - r.h) * share(node.align_y);
        }
    }
    (rects, Point::new(widest, height))
}

pub(super) fn put_row(node: &Node, children: &[Node], inner: Rect, room: Room, ops: &mut Vec<Op>) {
    let (slots, used) = row(node, children, room);
    let mut x = inner.x + (inner.w - used.x) * share(node.align_x);
    for (slot, c) in slots.iter().zip(children) {
        let h = tall(c, slot.size, inner.h);
        c.put(Rect::new(x, inner.y + (inner.h - h) * share(node.align_y), slot.size.x, h), slot.squeeze, ops);
        x += slot.size.x + node.spacing;
    }
}

pub(super) fn put_column(node: &Node, children: &[Node], inner: Rect, room: Room, ops: &mut Vec<Op>) {
    let (slots, used) = column(node, children, room);
    let mut y = inner.y + (inner.h - used.y) * share(node.align_y);
    for (slot, c) in slots.iter().zip(children) {
        let Point { x: w, y: h } = slot.size;
        c.put(Rect::new(inner.x + (inner.w - w) * share(node.align_x), y, w, h), slot.squeeze, ops);
        y += h + node.spacing;
    }
}

pub(super) fn put_wrap(node: &Node, children: &[Node], inner: Rect, room: Room, ops: &mut Vec<Op>) {
    let (rects, _) = wrap(node, children, room);
    for (r, c) in rects.into_iter().zip(children) {
        c.put(r.shifted(Point::new(inner.x, inner.y)), room.squeeze, ops);
    }
}

/// The base at its size and the layers offered it, each lined up in the box by the stack's
/// alignment (at its top left, as in iced, by default). A fill height fills the box.
pub(super) fn put_stack(node: &Node, inner: Rect, room: Room, ops: &mut Vec<Op>) {
    let Kind::Stack { layers, base } = &node.kind else { return };
    for (i, layer) in layers.iter().enumerate() {
        let room = if i == *base { room } else { Room { max: inner.w, squeeze: false } };
        let size = layer.size_in(room);
        let h = if layer.height.fills() { inner.h.min(layer.max_height) } else { size.y.min(inner.h) };
        let at = Point::new(inner.x + (inner.w - size.x) * share(node.align_x), inner.y + (inner.h - h) * share(node.align_y));
        layer.put(Rect::new(at.x, at.y, size.x, h), room.squeeze, ops);
    }
}

/// A pinned child in its parent's box `rect`, offered what of it lies right of and below where
/// the child starts.
pub(super) fn put_pin(child: &Node, pin: Pin, rect: Rect, ops: &mut Vec<Op>) {
    let from = match pin {
        Pin::At(p) => p,
        Pin::Aligned(..) => Point::ZERO,
    };
    let size = child.size_in(Room { max: (rect.w - from.x).max(0.0), squeeze: false });
    let h = tall(child, size, (rect.h - from.y).max(0.0));
    let at = match pin {
        Pin::At(p) => Point::new(rect.x + p.x, rect.y + p.y),
        Pin::Aligned(x, y) => Point::new(rect.x + (rect.w - size.x) * share(x), rect.y + (rect.h - h) * share(y)),
    };
    child.put(Rect::new(at.x, at.y, size.x, h), false, ops);
}
