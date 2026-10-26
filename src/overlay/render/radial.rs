//! The radial menu: items as arcs of a circle sized by their weights, or as boxes at equal
//! angles around a hub, with the stick that aims it named in the middle. This is
//! `draw/radial.rs`, with its numbers; the angles and radii are `crate::radial::Ring`, shared.
//!
//! The style is at full strength: the panel is faded as one picture, so the parts of the ring
//! don't show through each other as the menu opens and closes. An item's stagger fade is its
//! arc's own opacity (with the filters cut to it), or the layer round its box.

mod label;
#[cfg(test)]
mod tests;

use std::f32::consts::TAU;

use super::{
    Corners, Fill, Item, Node, Op, Outline, Path, PatternRef, Pin, Point, Rect, Rgba, Rule, Stroke, Style, anchor, item,
    rich,
};
use crate::{
    config::{MenuKind, Stick},
    menu::MenuView,
    overlay::art::{self, Filter},
    radial::{CELL, HUB, Ring, box_square},
};

/// The menu's circle or boxes, at the style's scale `s`, in `style`.
pub fn radial(m: &MenuView, style: &Style, s: f32) -> Node {
    match m.kind {
        MenuKind::Radial { boxes: true, .. } => boxes(m, style, s),
        _ => arcs(m, style, s),
    }
}

/// How item `i` is, from the animation.
fn state(m: &MenuView, i: usize, style: &Style) -> Item {
    Item::of(&style.anim, i, m.selected == Some(i))
}

/// The menu as a circle divided into arcs, one per item, sized by their weights.
fn arcs(m: &MenuView, style: &Style, s: f32) -> Node {
    let weights: Vec<f32> = m.items.iter().map(|item| item.weight).collect();
    let ring = Ring::new(&weights, s);
    let (size, cell) = (ring.size(), (CELL.0 * s, CELL.1 * s));
    let states: Vec<Item> = (0..m.items.len()).map(|i| state(m, i, style)).collect();
    let mut node = Node::stack(painted(style, &ring, &states));
    for (i, state) in states.iter().enumerate() {
        let (x, y, ..) = ring.arc_box(i);
        for filter in filters(m, style, &ring, (i, state.fade)) {
            node = node.pin(Pin::At(Point::new(x, y)), filter);
        }
    }
    for (i, &(start, end)) in ring.arcs.iter().enumerate() {
        // Each label sits in the middle of its arc.
        let (x, y) = ring.point((start + end) / 2.0, (ring.inner + ring.outer) / 2.0);
        let colors = label::colors(m, i, style, &states[i], states[i].fade);
        let body = Node::container(label::content(&m.items[i], &colors, 15.0 * s)).center_x(cell.0).center_y(cell.1);
        node = node.pin(Pin::At(Point::new(x - cell.0 / 2.0, y - cell.1 / 2.0)), body);
    }
    center(node, m, style, (ring.hub, size), false)
}

/// One arc as the ring draws it: how lit, flashed and faded in.
#[derive(Clone, Copy)]
struct Wedge {
    span: (f32, f32),
    lit: f32,
    flash: f32,
    fade: f32,
}

/// The ring as one picture: the panel's color under it, every arc in the items' color, the lit
/// ones blended toward the selected color and the picked one flashed white, the lines between
/// them, and the hub's disc.
fn painted(style: &Style, ring: &Ring, states: &[Item]) -> Node {
    let (c, ring) = (*style, ring.clone());
    let size = ring.size();
    let wedges: Vec<Wedge> = ring
        .arcs
        .iter()
        .zip(states)
        .map(|(&span, s)| Wedge { span, lit: s.lit, flash: s.flash * 0.5, fade: s.fade })
        .collect();
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        let middle = Point::new(rect.x + size / 2.0, rect.y + size / 2.0);
        let (inner, outer) = (ring.inner, ring.outer);
        let shade = |color: Rgba, alpha: f32| Rgba { a: alpha.clamp(0.0, 1.0), ..color };
        let one = wedges.len() == 1;
        // The panel's color under it all, so the gap around the hub is the panel's.
        let mut under = Path::new();
        under.circle(middle, outer);
        ops.push(Op::Path { path: under, fill: Some(Fill::Solid(c.background)), rule: Rule::NonZero, stroke: None });
        for w in &wedges {
            let mut path = Path::new();
            if one {
                // One item takes the whole circle, which an arc can't draw: a ring.
                path.circle(middle, (inner + outer) / 2.0);
            } else {
                path.arc(middle, outer, w.span.0, w.span.1).arc(middle, inner, w.span.1, w.span.0).close();
            }
            let layers = [(c.item, c.item.a * w.fade), (c.selected, w.lit * c.selected.a * w.fade), (Rgba::WHITE, w.flash * c.item.a * w.fade)];
            for (color, alpha) in layers.into_iter().filter(|&(_, a)| a > 0.0) {
                let color = shade(color, alpha);
                let (fill, stroke) = if one { (None, Some(Stroke::new(outer - inner, color))) } else { (Some(Fill::Solid(color)), None) };
                ops.push(Op::Path { path: path.clone(), fill, rule: Rule::NonZero, stroke });
            }
        }
        // The lines between the arcs, in the panel's color, on top of the arcs on either side.
        if !one {
            for w in &wedges {
                let at = |r: f32| {
                    let (sin, cos) = w.span.0.to_radians().sin_cos();
                    Point::new(middle.x + sin * r, middle.y - cos * r)
                };
                let mut line = Path::new();
                line.move_to(at(inner - 1.0)).line_to(at(outer + 1.0));
                ops.push(Op::Path { path: line, fill: None, rule: Rule::NonZero, stroke: Some(Stroke::new(2.0 * ring.s, c.background)) });
            }
        }
        let mut hub = Path::new();
        hub.circle(middle, ring.hub / 2.0 - 0.5);
        let edge = Stroke::new(1.0, c.item_text.scale_alpha(0.25));
        ops.push(Op::Path { path: hub, fill: Some(Fill::Solid(c.item)), rule: Rule::NonZero, stroke: Some(edge) });
    })
    .width(size)
    .height(size)
}

/// The style's filters on items over arc `i`, each cut to the arc, less the line between it and
/// the next, in a layer just big enough to hold it. They fade in with their arc.
fn filters(m: &MenuView, style: &Style, ring: &Ring, (i, fade): (usize, f32)) -> Vec<Node> {
    let (x0, y0, w, h) = ring.arc_box(i);
    let (start, end) = ring.arcs[i];
    let center = ring.size() / 2.0;
    let gap = if ring.arcs.len() > 1 { 2.0 * ring.s } else { 0.0 };
    let cut = art::Shape::Arc { center: (center - x0, center - y0), inner: ring.inner, outer: ring.outer, start, end, gap };
    let fade = style.fade * fade;
    art::filters(&m.style, true, Some(style.anim.clock))
        .into_iter()
        .map(|filter| {
            let pattern = match filter {
                Filter::Pattern(pattern) => pattern,
                Filter::Tint(tint) => art::wash(&tint),
            };
            let pattern = PatternRef::new(pattern);
            let opacity = (pattern.pattern().opacity * fade).clamp(0.0, 1.0);
            Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
                ops.push(Op::Pattern { pattern: pattern.clone(), rect, origin: anchor::origin(), cut, opacity });
            })
            .width(w)
            .height(h)
        })
        .collect()
}

/// The menu as boxes at equal angles around a hub.
fn boxes(m: &MenuView, style: &Style, s: f32) -> Node {
    let (cell_w, cell_h) = (CELL.0 * s, CELL.1 * s);
    let n = m.items.len().max(1) as f32;
    let (size, radius) = box_square(m.items.len(), s);
    let mut node = Node::stack(Node::space().width(size).height(size));
    for (i, it) in m.items.iter().enumerate() {
        // First item at the top, then clockwise.
        let angle = i as f32 / n * TAU;
        let (x, y) = (size / 2.0 + radius * angle.sin(), size / 2.0 - radius * angle.cos());
        // Nothing swells or slides here, and a row that only says something is never lit.
        let state = Item { selected: label::selected(m, i), scale: 1.0, shift: 0.0, ..state(m, i, style) };
        let state = if it.tone == crate::menu::Tone::Note { Item { fade: state.fade, ..Item::REST } } else { state };
        let colors = label::colors(m, i, style, &state, 1.0);
        let body = Node::container(label::content(it, &colors, 15.0 * s)).center_x(cell_w).center_y(cell_h);
        let cell = item(body, &colors, &state, &m.style, style.corners * cell_h / 2.0);
        node = node.pin(Pin::At(Point::new(x - cell_w / 2.0, y - cell_h / 2.0)), cell);
    }
    center(node, m, style, (HUB * s, size), true)
}

/// The circle in the middle of a radial menu, naming the stick that aims it, and the dot just
/// outside it that shows where the stick points. With `disc` the hub's disc is drawn here too.
fn center(node: Node, m: &MenuView, style: &Style, (hub, size): (f32, f32), disc: bool) -> Node {
    let s = hub / HUB;
    let stick = match m.kind {
        MenuKind::Radial { stick: Stick::Left, .. } => "{ls}",
        _ => "{rs}",
    };
    let name = Node::container(rich(stick, style, 16.0 * s, style.item_text)).center_x(hub).center_y(hub);
    let name = if disc {
        let edge = Outline { width: 1.0, color: style.item_text.scale_alpha(0.25) };
        super::glyphs::backed(name, Corners::all(hub / 2.0), Some(style.item), Some(edge))
    } else {
        name
    };
    let node = node.pin(Pin::At(Point::new(size / 2.0 - hub / 2.0, size / 2.0 - hub / 2.0)), name);
    let Some(degrees) = m.aim else { return node };
    let dot = 11.0 * s;
    let (sin, cos) = degrees.to_radians().sin_cos();
    let reach = hub / 2.0 + 10.0 * s;
    let (fill, edge) = (style.selected_text, style.selected);
    let marker = Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        ops.push(Op::Rect { rect, radii: Corners::all(dot / 2.0), fill: Some(fill.into()), border: Some(Outline { width: 1.0, color: edge }) });
    })
    .width(dot)
    .height(dot);
    node.pin(Pin::At(Point::new(size / 2.0 + sin * reach - dot / 2.0, size / 2.0 - cos * reach - dot / 2.0)), marker)
}
