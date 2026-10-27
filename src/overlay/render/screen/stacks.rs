//! The info and log panels, stacked where they share a spot, and the Guide overlay's row of
//! Notes, menu and Mappings.

use super::{Context, Shown, put_at};
use crate::{
    config::ScreenPosition,
    motion::Anim,
    overlay::{
        OverlayFrame, OverlayView,
        render::{self, Align, Length, Node, Op, Padding, Point, Rect, place::EDGE_MARGIN},
    },
};

/// How far apart the panels in a stack are.
const SPACING: f32 = 12.0;

/// Draws the stacks of info and log panels, those sharing a spot stacked up there, info first,
/// each panel a layer of its own that only fades.
/// With `guide` up, the stacks at the middle of the left and right edges (its Notes and
/// Mappings) are handed back instead, to stand in a row with its menu.
pub(super) fn put_stacks(ops: &mut Vec<Op>, shown: &Shown, cx: &Context, guide: bool) -> [Option<Node>; 2] {
    let mut sides = [None, None];
    for spot in ScreenPosition::GRID {
        let infos: Vec<_> = shown.info.iter().filter(|(v, _)| v.style.position == spot).collect();
        let logs: Vec<_> = shown.logs.iter().filter(|(v, _)| v.style.position == spot).collect();
        let style = infos.first().map(|(v, _)| &v.style).or_else(|| logs.first().map(|(v, _)| &v.style));
        let Some(style) = style else { continue };
        let panels = infos
            .iter()
            .map(|(v, anim)| render::info_panel(v, cx.font, &cx.fit(&v.style)).opacity(v.opacity * anim.opacity()).layered())
            .chain(logs.iter().map(|(v, anim)| render::log_panel(v, cx.font, &cx.fit(&v.style)).opacity(v.opacity * anim.opacity()).layered()));
        let stack = Node::column(panels).spacing(SPACING);
        match spot {
            ScreenPosition::CenterLeft if guide => sides[0] = Some(stack),
            ScreenPosition::CenterRight if guide => sides[1] = Some(stack),
            _ => put_at(ops, &stack, style, cx),
        }
    }
    sides
}

/// Whether the Guide overlay is up with its menu alone in the middle of the screen, its size and
/// place the style's own, so Notes and Mappings can line up beside it.
pub(super) fn guide_up(frame: &OverlayFrame, shown: &Shown, cx: &Context) -> bool {
    let [(OverlayView::Menu(menu), _)] = shown.panels.as_slice() else { return false };
    let fit = cx.fit(&menu.style);
    !frame.peek
        && menu.style.position == ScreenPosition::Center
        && (fit.width, fit.height, fit.x, fit.y) == (None, None, 0.0, 0.0)
        && shown.info.iter().any(|(v, _)| crate::info::is_guide_panel(&v.name))
}

/// A centered `menu` with the stacks beside it, their tops in a line and the three centered on
/// the screen as one, as the Guide overlay draws its Notes, menu and Mappings. Each is a picture
/// of its own: the menu fades, moves and zooms by its animation, and Notes and Mappings fade by
/// theirs, as they do anywhere else, so the row is three pictures that don't overlap.
pub(super) fn beside(ops: &mut Vec<Op>, sides: [Option<Node>; 2], menu: Node, anim: &Anim, cx: &Context) {
    let [left, right] = sides;
    let (x, y) = anim.offset();
    let side = |stack: Option<Node>, align: Align, padding: Padding| {
        Node::container(stack.unwrap_or_else(Node::space)).width(Length::Fill).align_x(align).padding(padding)
    };
    let menu = menu.scale(anim.scale().max(0.0)).shift(Point::new(x, y)).opacity(anim.opacity()).layered();
    let line = Node::row([
        side(left, Align::Start, Padding { left: EDGE_MARGIN, ..Padding::ZERO }),
        menu,
        side(right, Align::End, Padding { right: EDGE_MARGIN, ..Padding::ZERO }),
    ])
    .align_y(Align::Start);
    let size = line.measure(cx.display.x);
    line.place(Rect::new(0.0, (cx.display.y - size.y) / 2.0, cx.display.x, size.y), ops);
}
