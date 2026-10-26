//! The bodies of the kinds of menu: a list (or buttons), a D-pad or face cluster, a grid and a
//! carousel. Each is the part of the panel between its title and its hint.

use super::cell;
use crate::{
    menu::{GridLayout, MenuView, VISIBLE_ROWS},
    overlay::render::{Align, Length, Node, Op, Pin, Point, Rect, Style, glyphs::label, paint},
};

/// A D-pad's or face cluster's four slots, around the middle. The slot picked last stays lit,
/// and flashes as it's picked.
pub(super) fn directional(m: &MenuView, style: &Style, s: f32) -> Node {
    let (w, h) = (190.0 * s, 52.0 * s);
    let slot = |i: usize| -> Node {
        match m.items.get(i).filter(|item| !item.label.is_empty()) {
            Some(_) => {
                let round = style.corners * h / 2.0;
                cell(m, i, style, 16.0 * s).boxed(|label| Node::container(label).center_x(w).center_y(h), &m.style, round)
            }
            None => Node::space().width(w).height(h),
        }
    };
    Node::column([
        slot(0),
        Node::row([slot(3), Node::space().width(40.0 * s), slot(1)]).align_y(Align::Center),
        slot(2),
    ])
    .spacing(10.0 * s)
    .align_x(Align::Center)
}

/// Items in rows of equal cells, read left to right; a short last row keeps to the left. A
/// merged item covers several cells, and the items under it aren't drawn.
pub(super) fn grid(m: &MenuView, style: &Style, s: f32) -> Node {
    let spans: Vec<_> = m.items.iter().map(|item| item.span).collect();
    let layout = GridLayout::new(&spans, m.kind.grid_columns().unwrap_or(1));
    let (w, h, gap) = (150.0 * s, 64.0 * s, 8.0 * s);
    // How far `n` cells reach, with the gaps between them.
    let reach = |n: usize, size: f32| n as f32 * size + n.saturating_sub(1) as f32 * gap;
    let mut cells = Node::space().width(reach(layout.columns, w)).height(reach(layout.rows, h));
    for i in 0..m.items.len() {
        let Some(at) = layout.cells(i) else { continue };
        let (cw, ch) = (reach(at.columns, w), reach(at.rows, h));
        let round = style.corners * h / 2.0;
        let boxed = cell(m, i, style, 16.0 * s).boxed(|label| Node::container(label).center_x(cw).center_y(ch).padding([0.0, 6.0 * s]), &m.style, round);
        cells = cells.pin(Pin::At(Point::new(at.column as f32 * (w + gap), at.row as f32 * (h + gap))), boxed);
    }
    cells
}

/// A list: one column while it is short, two once it is longer. Two columns fill top to
/// bottom, the left one first, and scroll together with the cursor.
pub(super) fn list(m: &MenuView, style: &Style, s: f32) -> Node {
    let len = m.items.len();
    let cursor = m.selected.unwrap_or(0);
    let row = |i: usize| -> Node {
        let boxed = |label: Node| {
            // At least as tall as a glyph, so rows with glyphs are no taller than plain ones, and
            // taller when a long label wraps.
            let content = Node::row([Node::space().height(24.0 * s), label]).align_y(Align::Center);
            Node::container(content).padding([10.0 * s, 14.0 * s]).width(Length::Fill)
        };
        // A note has no box, so it doesn't look like something to pick.
        cell(m, i, style, 17.0 * s).boxed(boxed, &m.style, style.corners * 20.0 * s)
    };
    let columns = m.columns();
    let start = crate::menu::list_start_in(cursor, len, columns);
    let half = if columns == 2 { len.div_ceil(2) } else { len };
    let shown = |from: usize, to: usize| (from..to.min(from + VISIBLE_ROWS)).map(&row).collect::<Vec<Node>>();
    // Between rows, the style's dividers, if it has any.
    let divider = if m.style.lite.plain { None } else { m.style.ornament.dividers.as_ref() };
    let column_of = |items: Vec<Node>, width: f32| {
        let mut col = Node::column([]).spacing(6.0 * s).width(width * s);
        for (k, item) in items.into_iter().enumerate() {
            if let Some(line) = divider.filter(|_| k > 0) {
                col = col.push(rule(line, s, style.fade));
            }
            col = col.push(item);
        }
        col
    };
    // Wide enough that a long label stays on one line, in the style's case, size and letter
    // spacing: a wrapped row is taller, and a full page of them overflows the box. Past the
    // widest, labels wrap.
    let t = &m.style.typography;
    let capitals = if t.case == crate::config::Case::Upper { 1.25 } else { 1.0 };
    let per_letter = 9.0 * capitals * t.size.clamp(0.5, 2.0) + t.letter_spacing.clamp(0.0, 12.0);
    let longest = m.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0);
    let width = (longest as f32 * per_letter + 60.0).clamp(300.0, 560.0);
    let left = column_of(shown(start, half), width);
    let grid = if half < len { Node::row([left, column_of(shown(half + start, len), width)]).spacing(14.0 * s) } else { left };
    // How many rows are scrolled out above and below, in both columns.
    let right = len - half;
    let above = start + start.min(right);
    let end = start + VISIBLE_ROWS;
    let below = half.saturating_sub(end) + right.saturating_sub(end);
    let marker = |t: String| label(style, &t, 13.0 * s, style.muted);
    let mut body = Node::column([]).spacing(6.0 * s).align_x(Align::Center);
    if above > 0 {
        body = body.push(marker(format!("▲ {above} more")));
    }
    body = body.push(grid);
    if below > 0 {
        body = body.push(marker(format!("▼ {below} more")));
    }
    body
}

/// A line between two rows of a list: `width` thick at scale 1, across the column.
fn rule(stroke: &crate::config::Stroke, s: f32, fade: f32) -> Node {
    let mut color = paint(&stroke.paint(), [0xff, 0xff, 0xff]);
    color.a = (color.a * fade).clamp(0.0, 1.0);
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        ops.push(Op::Rect { rect, radii: crate::overlay::render::Corners::ZERO, fill: Some(color.into()), border: None });
    })
    .width(Length::Fill)
    .height(stroke.width.max(0.5) * s)
}

/// A strip of items with the selected one big in the middle, between arrows.
pub(super) fn carousel(m: &MenuView, style: &Style, s: f32) -> Node {
    let n = m.items.len();
    let selected = m.selected.unwrap_or(0);
    let arrow = |t: &str| label(style, t, 22.0 * s, style.muted);
    let mut line = vec![arrow("◀")];
    // The selected item in the middle, with up to two neighbors on each side. An even count
    // leaves the right side one short, so a blank keeps the selected item centered.
    let shown = n.min(5) as i32;
    let (before, after) = (shown / 2, shown - 1 - shown / 2);
    // On a touchpad the items stay put, all of them in order, the way the pad lays them out.
    let slots: Vec<Option<usize>> = if m.touch {
        (0..n).map(Some).collect()
    } else {
        (-before..=before).map(|offset| (offset <= after).then(|| (selected as i32 + offset).rem_euclid(n as i32) as usize)).collect()
    };
    for slot in slots {
        let Some(i) = slot else {
            line.push(Node::space().width(120.0 * s).height(60.0 * s));
            continue;
        };
        let big = if m.touch { m.selected == Some(i) } else { i == selected };
        let height = if big { 80.0 } else { 60.0 } * s;
        let width = if big { 170.0 } else { 120.0 } * s;
        let mut c = cell(m, i, style, if big { 19.0 } else { 14.0 } * s);
        // The big one is outlined as the picked one.
        c.state.selected = big;
        line.push(c.boxed(|label| Node::container(label).center_x(width).center_y(height), &m.style, style.corners * height / 2.0));
    }
    line.push(arrow("▶"));
    Node::row(line).spacing(12.0 * s).align_y(Align::Center)
}
