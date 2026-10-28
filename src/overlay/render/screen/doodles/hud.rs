//! The Doodle HUD: the carousel of tools across the top, and the tray of settings along the bottom.
//! Each part is laid out as a piece with its size, and drawn where the row puts it. A part is text
//! with its controller glyphs, or an icon of what it is, and separators divide the groups.

use std::f32::consts::TAU;

use super::{super::Context, ink};
use crate::{
    overlay::{
        DoodleCell, DoodleChecks, DoodleHud,
        render::{Corners, Node, Op, Outline, Path, Point, Rect, Rgba, Rule, Style, rich},
    },
    tools::doodle::{Kind, PALETTE, Tip},
};

/// The HUD's text size, the space inside its panel, the gap between a piece and the next, and the
/// space each side of a separator.
const TEXT: f32 = 16.0;
const PAD_X: f32 = 14.0;
const PAD_Y: f32 = 8.0;
const GAP: f32 = 8.0;
const SEPARATOR_GAP: f32 = 12.0;
const SEPARATOR_WIDTH: f32 = 1.0;
/// An icon's side, and a palette swatch's side and the gap between swatches.
const ICON: f32 = 18.0;
const SWATCH: f32 = 12.0;
const SWATCH_GAP: f32 = 4.0;
/// How far the panels stand from the screen's top and bottom edges.
const TOP: f32 = 16.0;
const BOTTOM: f32 = 24.0;
/// The carousel's names, in the order of `tools::doodle::Tool`.
const TOOL_NAMES: [&str; 3] = ["Pen", "Eraser", "Text"];
/// How far the checklist's panel stands above the tray, and the gap between its rows.
const CHECKS_GAP: f32 = 8.0;
const ROW_GAP: f32 = 6.0;
/// A check mark on the unit square, drawn in a ticked box.
const TICK: [(f32, f32); 6] = [(0.14, 0.52), (0.26, 0.4), (0.42, 0.56), (0.76, 0.2), (0.88, 0.32), (0.42, 0.8)];

/// How a piece draws at the point its top left corner lands on.
type Draw = Box<dyn FnOnce(&mut Vec<Op>, Point)>;

/// A part of a row: what it takes up, and how it draws.
struct Piece {
    size: Point,
    draw: Draw,
}

/// A row's entries: a piece, or the separator between two groups.
enum Slot {
    Piece(Piece),
    Separator,
}

/// Where a panel stands: at the top of the screen, or the bottom.
#[derive(Clone, Copy)]
enum Anchor {
    Top,
    Bottom,
}

/// Where a strip was laid out: its panel, and each of its pieces, in order.
pub struct Placed {
    panel: Rect,
    pieces: Vec<Rect>,
}

/// The HUD's style: the menus' look, with the controller's glyphs.
fn hud_style(hud: &DoodleHud, cx: &Context) -> Style {
    Style::of(&hud.style, cx.font).with_glyphs(cx.glyphs)
}

/// The text `text` as a piece, drawn in `color`.
fn words(text: &str, style: &Style, color: Rgba, width: f32) -> Piece {
    text_piece(rich(text, style, TEXT, color), width)
}

fn text_piece(node: Node, width: f32) -> Piece {
    let size = node.measure(width);
    Piece { size, draw: Box::new(move |ops, at| node.place(Rect::new(at.x, at.y, size.x, size.y), ops)) }
}

/// Draws an icon's `shape`, given as points on a unit square, into the icon's `rect`, in `color`.
fn fill(ops: &mut Vec<Op>, rect: Rect, shape: &[(f32, f32)], color: Rgba) {
    let mut path = Path::new();
    for (i, &(u, v)) in shape.iter().enumerate() {
        let p = Point::new(rect.x + u * rect.w, rect.y + v * rect.h);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close();
    ops.push(Op::Path { path, fill: Some(color.into()), rule: Rule::NonZero, stroke: None });
}

/// A circle on the unit square, for the round tip.
fn circle() -> Vec<(f32, f32)> {
    (0..24).map(|i| {
        let angle = TAU * i as f32 / 24.0;
        (0.5 + 0.42 * angle.cos(), 0.5 + 0.42 * angle.sin())
    }).collect()
}

/// A square on the unit square with its center at (`u`, `v`) and side `side`.
fn square(u: f32, v: f32, side: f32) -> Vec<(f32, f32)> {
    let half = side / 2.0;
    vec![(u - half, v - half), (u + half, v - half), (u + half, v + half), (u - half, v + half)]
}

/// The icon of a carousel tool, in `color`.
fn tool_icon(ops: &mut Vec<Op>, tool: usize, rect: Rect, color: Rgba) {
    let shape: Vec<(f32, f32)> = match tool {
        0 => vec![(0.15, 0.85), (0.25, 0.62), (0.8, 0.07), (0.93, 0.2), (0.38, 0.75)],
        1 => vec![(0.1, 0.55), (0.5, 0.1), (0.9, 0.45), (0.5, 0.9)],
        _ => vec![(0.1, 0.12), (0.9, 0.12), (0.9, 0.34), (0.62, 0.34), (0.62, 0.9), (0.38, 0.9), (0.38, 0.34), (0.1, 0.34)],
    };
    fill(ops, rect, &shape, color);
}

/// The icon of a pen tip: the shape of the mark it makes.
fn tip_icon(ops: &mut Vec<Op>, tip: Tip, rect: Rect) {
    let shape = match tip {
        Tip::Round => circle(),
        Tip::Square => square(0.5, 0.5, 0.8),
        Tip::Chisel => vec![(0.15, 0.9), (0.35, 0.1), (0.9, 0.1), (0.7, 0.9)],
        Tip::Diagonal => vec![(0.0, 0.86), (0.86, 0.0), (1.0, 0.14), (0.14, 1.0)],
    };
    fill(ops, rect, &shape, Rgba::WHITE);
}

/// The icon of a pen style: what a stroke in it looks like.
fn kind_icon(ops: &mut Vec<Op>, kind: Kind, rect: Rect) {
    match kind {
        Kind::Solid => fill(ops, rect, &[(0.05, 0.4), (0.95, 0.4), (0.95, 0.6), (0.05, 0.6)], Rgba::WHITE),
        Kind::Brush => {
            fill(ops, rect, &[(0.05, 0.42), (0.95, 0.42), (0.95, 0.58), (0.05, 0.58)], Rgba::WHITE);
            for y in [0.12, 0.8] {
                fill(ops, rect, &[(0.05, y), (0.35, y), (0.35, y + 0.08), (0.05, y + 0.08)], Rgba::WHITE);
                fill(ops, rect, &[(0.45, y), (0.95, y), (0.95, y + 0.08), (0.45, y + 0.08)], Rgba::WHITE);
            }
        }
        Kind::Spray => {
            for (u, v) in [(0.15, 0.3), (0.45, 0.2), (0.75, 0.35), (0.3, 0.7), (0.6, 0.62), (0.88, 0.8)] {
                fill(ops, rect, &square(u, v, 0.16), Rgba::WHITE);
            }
        }
    }
}

/// An icon's box in a piece whose top left corner is `at` and whose height is `height`.
fn icon_rect(at: Point, height: f32) -> Rect {
    Rect::new(at.x, at.y + (height - ICON) / 2.0, ICON, ICON)
}

/// A carousel tool: its icon and name, lit when it's the one in use.
fn tool_piece(index: usize, name: &str, lit: bool, style: &Style, width: f32) -> Piece {
    let color = if lit { Rgba::WHITE } else { style.muted };
    let label = rich(name, style, TEXT, color);
    let size = Point::new(ICON + GAP + label.measure(width).x, ICON.max(label.measure(width).y));
    let label_size = label.measure(width);
    Piece {
        size,
        draw: Box::new(move |ops, at| {
            if lit {
                let lit_rect = Rect::new(at.x - 6.0, at.y - 4.0, size.x + 12.0, size.y + 8.0);
                ops.push(Op::Rect { rect: lit_rect, radii: Corners::all(8.0), fill: Some(Rgba::rgb(0.23, 0.45, 0.9).into()), border: None });
            }
            tool_icon(ops, index, icon_rect(at, size.y), color);
            let label_at = Rect::new(at.x + ICON + GAP, at.y + (size.y - label_size.y) / 2.0, label_size.x, label_size.y);
            label.place(label_at, ops);
        }),
    }
}

/// A pen tip: its name, with a hint, and an icon of its mark.
fn tip_piece(tip: Tip, label: Node, width: f32) -> Piece {
    let text = label.measure(width);
    let size = Point::new(ICON + GAP + text.x, ICON.max(text.y));
    Piece {
        size,
        draw: Box::new(move |ops, at| {
            tip_icon(ops, tip, icon_rect(at, size.y));
            label.place(Rect::new(at.x + ICON + GAP, at.y + (size.y - text.y) / 2.0, text.x, text.y), ops);
        }),
    }
}

/// A pen style: its name, with a hint, and an icon of its look.
fn kind_piece(kind: Kind, label: Node, width: f32) -> Piece {
    let text = label.measure(width);
    let size = Point::new(ICON + GAP + text.x, ICON.max(text.y));
    Piece {
        size,
        draw: Box::new(move |ops, at| {
            kind_icon(ops, kind, icon_rect(at, size.y));
            label.place(Rect::new(at.x + ICON + GAP, at.y + (size.y - text.y) / 2.0, text.x, text.y), ops);
        }),
    }
}

/// The palette: its hint, then every color in a row, with the one in use ringed.
fn palette_piece(current: usize, hint: Node, width: f32) -> Piece {
    let text = hint.measure(width);
    let swatches = PALETTE.len() as f32 * SWATCH + (PALETTE.len() - 1) as f32 * SWATCH_GAP;
    let size = Point::new(text.x + GAP + swatches, text.y.max(SWATCH));
    Piece {
        size,
        draw: Box::new(move |ops, at| {
            hint.place(Rect::new(at.x, at.y + (size.y - text.y) / 2.0, text.x, text.y), ops);
            let row = at.x + text.x + GAP;
            for index in 0..PALETTE.len() {
                let x = row + index as f32 * (SWATCH + SWATCH_GAP);
                let y = at.y + (size.y - SWATCH) / 2.0;
                let edge = Outline { width: 1.0, color: Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.35 } };
                ops.push(Op::Rect { rect: Rect::new(x, y, SWATCH, SWATCH), radii: Corners::all(3.0), fill: Some(ink(index, 1.0).into()), border: Some(edge) });
                if index == current {
                    let ring = Outline { width: 2.0, color: Rgba::WHITE };
                    ops.push(Op::Rect { rect: Rect::new(x - 3.0, y - 3.0, SWATCH + 6.0, SWATCH + 6.0), radii: Corners::all(5.0), fill: None, border: Some(ring) });
                }
            }
        }),
    }
}

/// The piece for one part of the tray.
fn cell_piece(cell: &DoodleCell, style: &Style, width: f32) -> Piece {
    match cell {
        DoodleCell::Palette { current, hint } => palette_piece(*current, rich(hint, style, TEXT, Rgba::WHITE), width),
        DoodleCell::Text(text) => words(text, style, Rgba::WHITE, width),
        DoodleCell::Tip { tip, text } => tip_piece(*tip, rich(text, style, TEXT, Rgba::WHITE), width),
        DoodleCell::Kind { kind, text } => kind_piece(*kind, rich(text, style, TEXT, Rgba::WHITE), width),
    }
}

/// The carousel across the top: the bumpers, and the three tools with a separator between each.
pub fn carousel(ops: &mut Vec<Op>, hud: &DoodleHud, cx: &Context) {
    let style = hud_style(hud, cx);
    let width = cx.display.x;
    let mut slots = vec![Slot::Piece(words("{lb}", &style, Rgba::WHITE, width))];
    for (index, name) in TOOL_NAMES.iter().enumerate() {
        if index > 0 {
            slots.push(Slot::Separator);
        }
        slots.push(Slot::Piece(tool_piece(index, name, index == hud.tool, &style, width)));
    }
    slots.push(Slot::Piece(words("{rb}", &style, Rgba::WHITE, width)));
    strip(ops, slots, cx, Anchor::Top);
}

/// The tray along the bottom: each setting, with a separator between the groups. Returns where its
/// cells went, for the checklist to stand over the one it opens from.
pub fn tray(ops: &mut Vec<Op>, hud: &DoodleHud, cx: &Context) -> Placed {
    let style = hud_style(hud, cx);
    let width = cx.display.x;
    let mut slots = Vec::new();
    for (index, cell) in hud.cells.iter().enumerate() {
        if index > 0 {
            slots.push(Slot::Separator);
        }
        slots.push(Slot::Piece(cell_piece(cell, &style, width)));
    }
    strip(ops, slots, cx, Anchor::Bottom)
}

/// The checklist of text styles: a box to tick for each style, with the row the cursor is on lit.
/// It stands just above the tray, over the cell it opens from (`checks.cell`), kept on the screen;
/// without the tray it stands where the tray would be.
pub fn checklist(ops: &mut Vec<Op>, checks: &DoodleChecks, hud: &DoodleHud, tray: Option<&Placed>, cx: &Context) {
    let style = hud_style(hud, cx);
    let width = cx.display.x;
    let rows: Vec<Piece> = checks.rows.iter().enumerate().map(|(index, row)| check_piece(&row.label, row.on, index == checks.cursor, &style, width)).collect();
    let tall = rows.iter().map(|piece| piece.size.y).sum::<f32>() + ROW_GAP * rows.len().saturating_sub(1) as f32;
    let wide = rows.iter().map(|piece| piece.size.x).fold(0.0, f32::max);
    let (w, h) = (wide + 2.0 * PAD_X, tall + 2.0 * PAD_Y);
    let (middle, bottom) = match tray {
        Some(placed) => {
            let over = placed.pieces.get(checks.cell).unwrap_or(&placed.panel);
            (over.x + over.w / 2.0, placed.panel.y - CHECKS_GAP)
        }
        None => (cx.display.x / 2.0, cx.display.y - BOTTOM),
    };
    let left = (middle - w / 2.0).clamp(0.0, (cx.display.x - w).max(0.0));
    let frame = Rect::new(left, bottom - h, w, h);
    ops.push(Op::Rect { rect: frame, radii: Corners::all(12.0), fill: Some(panel().into()), border: None });
    let mut y = frame.y + PAD_Y;
    for piece in rows {
        let height = piece.size.y;
        (piece.draw)(ops, Point::new(frame.x + PAD_X, y));
        y += height + ROW_GAP;
    }
}

/// One row of the checklist: a box, ticked when the style is on, and the style's name. The row
/// under the cursor is lit like the carousel's tool in use.
fn check_piece(label: &str, on: bool, lit: bool, style: &Style, width: f32) -> Piece {
    let label = rich(label, style, TEXT, Rgba::WHITE);
    let text = label.measure(width);
    let size = Point::new(ICON + GAP + text.x, ICON.max(text.y));
    Piece {
        size,
        draw: Box::new(move |ops, at| {
            if lit {
                let lit_rect = Rect::new(at.x - 6.0, at.y - 4.0, size.x + 12.0, size.y + 8.0);
                ops.push(Op::Rect { rect: lit_rect, radii: Corners::all(8.0), fill: Some(Rgba::rgb(0.23, 0.45, 0.9).into()), border: None });
            }
            let box_rect = icon_rect(at, size.y);
            let edge = Outline { width: 2.0, color: Rgba::WHITE };
            ops.push(Op::Rect { rect: box_rect, radii: Corners::all(3.0), fill: None, border: Some(edge) });
            if on {
                fill(ops, box_rect, &TICK, Rgba::WHITE);
            }
            label.place(Rect::new(at.x + ICON + GAP, at.y + (size.y - text.y) / 2.0, text.x, text.y), ops);
        }),
    }
}

/// Lays `slots` out in a centred panel at `anchor`: pieces a gap apart, and a separator between
/// groups. The panel is drawn first, then each piece in its place.
fn strip(ops: &mut Vec<Op>, slots: Vec<Slot>, cx: &Context, anchor: Anchor) -> Placed {
    let mut span = 0.0;
    let mut tall: f32 = 0.0;
    let mut after_piece = false;
    for slot in &slots {
        match slot {
            Slot::Piece(piece) => {
                span += if after_piece { GAP } else { 0.0 } + piece.size.x;
                tall = tall.max(piece.size.y);
                after_piece = true;
            }
            Slot::Separator => {
                span += 2.0 * SEPARATOR_GAP + SEPARATOR_WIDTH;
                after_piece = false;
            }
        }
    }
    let panel_width = span + 2.0 * PAD_X;
    let panel_height = tall + 2.0 * PAD_Y;
    let x = (cx.display.x - panel_width) / 2.0;
    let y = match anchor {
        Anchor::Top => TOP,
        Anchor::Bottom => cx.display.y - panel_height - BOTTOM,
    };
    let panel_rect = Rect::new(x, y, panel_width, panel_height);
    ops.push(Op::Rect { rect: panel_rect, radii: Corners::all(12.0), fill: Some(panel().into()), border: None });
    let mut pieces = Vec::new();
    let mut left = x + PAD_X;
    let mut after_piece = false;
    for slot in slots {
        match slot {
            Slot::Piece(piece) => {
                if after_piece {
                    left += GAP;
                }
                let at = Point::new(left, y + PAD_Y + (tall - piece.size.y) / 2.0);
                pieces.push(Rect::new(at.x, at.y, piece.size.x, piece.size.y));
                (piece.draw)(ops, at);
                left += piece.size.x;
                after_piece = true;
            }
            Slot::Separator => {
                left += SEPARATOR_GAP;
                let rule = Rect::new(left, y + PAD_Y, SEPARATOR_WIDTH, tall);
                ops.push(Op::Rect { rect: rule, radii: Corners::all(0.0), fill: Some(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.22 }.into()), border: None });
                left += SEPARATOR_WIDTH + SEPARATOR_GAP;
                after_piece = false;
            }
        }
    }
    Placed { panel: panel_rect, pieces }
}

/// The dark panel behind the HUD.
fn panel() -> Rgba {
    Rgba { r: 0.09, g: 0.09, b: 0.11, a: 0.9 }
}
