use super::SLACK;
use crate::overlay::render::{
    Corners, Fill, Layer, Length, Node, Op, Padding, Painter, Pin, Point, Rect, Rgba, balanced,
    layout::Align,
    text::{self, Spec},
};

/// A leaf that draws its box as a rect, marked by `id` in its red channel.
fn leaf(id: u8) -> Node {
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| ops.push(Op::Rect { rect, radii: Corners::ZERO, fill: Some(Rgba::from_rgb8(id, 0, 0).into()), border: None }))
}

fn block(id: u8, w: f32, h: f32) -> Node {
    leaf(id).width(w).height(h)
}

/// Each leaf's box, by its id, in the order drawn.
fn boxes(ops: &[Op]) -> Vec<(u8, Rect)> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Rect { rect, fill: Some(Fill::Solid(c)), .. } => Some((c.to_u8()[0], *rect)),
            _ => None,
        })
        .collect()
}

/// `node` measured at `width` and placed at the origin: its size and its leaves' boxes.
fn laid(node: &Node, width: f32) -> (Point, Vec<(u8, Rect)>) {
    let (size, ops) = node.ops(Point::ZERO, width);
    assert!(balanced(&ops));
    (size, boxes(&ops))
}

fn at(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

#[test]
fn rows_and_columns_space_their_children_inside_their_padding() {
    let row = Node::row([block(1, 10.0, 10.0), block(2, 20.0, 10.0), block(3, 10.0, 10.0)]).spacing(5.0).padding([2.0, 4.0]);
    let (size, b) = laid(&row, f32::INFINITY);
    assert_eq!(size, Point::new(58.0, 14.0));
    assert_eq!(b, [(1, at(4.0, 2.0, 10.0, 10.0)), (2, at(19.0, 2.0, 20.0, 10.0)), (3, at(44.0, 2.0, 10.0, 10.0))]);
    let column = Node::column([block(1, 10.0, 10.0), block(2, 10.0, 20.0)]).spacing(6.0).padding(Padding { left: 3.0, ..Padding::ZERO });
    let (size, b) = laid(&column, 100.0);
    assert_eq!(size, Point::new(13.0, 36.0));
    assert_eq!(b, [(1, at(3.0, 0.0, 10.0, 10.0)), (2, at(3.0, 16.0, 10.0, 20.0))]);
}

#[test]
fn children_line_up_across_and_a_bigger_box_places_them_as_a_block() {
    let row = |align| laid(&Node::row([block(1, 10.0, 10.0), block(2, 10.0, 30.0)]).align_y(align), 100.0).1[0].1.y;
    assert_eq!([row(Align::Start), row(Align::Center), row(Align::End)], [0.0, 10.0, 20.0]);
    let column = Node::column([block(1, 10.0, 10.0), block(2, 30.0, 10.0)]).align_x(Align::End);
    assert_eq!(laid(&column, 100.0).1[0].1, at(20.0, 0.0, 10.0, 10.0));
    // A box of one child, centered both ways, as iced's `center_x` and `center_y` do.
    let boxed = Node::container(block(1, 10.0, 10.0)).center_x(50.0).center_y(40.0);
    assert_eq!(laid(&boxed, 100.0), (Point::new(50.0, 40.0), vec![(1, at(20.0, 15.0, 10.0, 10.0))]));
    // Along a row or column, the children move as one.
    let low = Node::column([block(1, 10.0, 10.0), block(2, 10.0, 10.0)]).spacing(5.0).height(40.0).align_y(Align::End);
    assert_eq!(laid(&low, 100.0).1, [(1, at(0.0, 15.0, 10.0, 10.0)), (2, at(0.0, 30.0, 10.0, 10.0))]);
    let right = Node::row([block(1, 10.0, 10.0)]).width(30.0).align_x(Align::Center);
    assert_eq!(laid(&right, 100.0).1, [(1, at(10.0, 0.0, 10.0, 10.0))]);
}

#[test]
fn fills_share_what_the_others_leave_by_their_portions() {
    let row = Node::row([block(1, 50.0, 10.0), leaf(2).width(Length::Fill).height(10.0), leaf(3).width(Length::FillPortion(3)).height(10.0)]).spacing(10.0);
    // Holding a fill, the row fills what it is offered.
    let (size, b) = laid(&row, 200.0);
    assert_eq!(size, Point::new(200.0, 10.0));
    assert_eq!(b, [(1, at(0.0, 0.0, 50.0, 10.0)), (2, at(60.0, 0.0, 32.5, 10.0)), (3, at(102.5, 0.0, 97.5, 10.0))]);
    // The ones that don't fill are laid out first, wherever they are: a spacer pushes what
    // follows it to the end, as a menu row's ▸ is.
    let line = Node::row([block(1, 10.0, 10.0), Node::space().width(Length::Fill), block(2, 10.0, 10.0)]);
    let list = Node::column([line, block(3, 40.0, 10.0)]).width(100.0);
    assert_eq!(laid(&list, 300.0).1, [(1, at(0.0, 0.0, 10.0, 10.0)), (2, at(90.0, 0.0, 10.0, 10.0)), (3, at(0.0, 10.0, 40.0, 10.0))]);
    // With nothing left, a fill gets nothing.
    let full = Node::row([leaf(1).width(Length::Fill).height(5.0), block(2, 120.0, 5.0)]);
    assert_eq!(laid(&full, 100.0).1, [(1, at(0.0, 0.0, 0.0, 5.0)), (2, at(0.0, 0.0, 100.0, 5.0))]);
}

#[test]
fn a_fill_inside_a_box_set_to_shrink_shrinks_to_its_contents() {
    let inner = Node::row([block(2, 10.0, 10.0)]).width(Length::Fill);
    let row = Node::row([block(1, 10.0, 10.0), inner]).width(Length::Shrink);
    assert_eq!(row.measure(200.0), Point::new(20.0, 10.0));
    // In a column, a fill is as wide as the widest of the others: a list's divider.
    let rows = || [block(1, 40.0, 10.0), leaf(2).width(Length::Fill).height(2.0), block(3, 30.0, 10.0)];
    let (size, b) = laid(&Node::column(rows()).width(Length::Shrink), 200.0);
    assert_eq!(size, Point::new(40.0, 22.0));
    assert_eq!(b[1], (2, at(0.0, 10.0, 40.0, 2.0)));
    // Left as it is, the column fills, as its divider does.
    assert_eq!(Node::column(rows()).measure(200.0), Point::new(200.0, 22.0));
}

#[test]
fn a_fill_height_is_as_tall_as_the_row_or_stack_it_is_in() {
    let row = Node::row([block(1, 10.0, 24.0), leaf(2).width(5.0).height(Length::Fill), block(3, 10.0, 8.0)]).align_y(Align::Center);
    let (size, b) = laid(&row, 100.0);
    assert_eq!(size, Point::new(25.0, 24.0));
    assert_eq!(b[1..], [(2, at(10.0, 0.0, 5.0, 24.0)), (3, at(15.0, 8.0, 10.0, 8.0))]);
    // In a column, a fill height is as tall as what it holds: contents are measured with no
    // limit on their height, as in a panel.
    let column = Node::column([block(1, 10.0, 10.0), Node::container(block(2, 10.0, 6.0)).height(Length::Fill)]);
    assert_eq!(column.measure(100.0), Point::new(10.0, 16.0));
}

#[test]
fn a_stack_is_as_big_as_its_base_with_layers_under_and_over_it() {
    let stack = Node::stack(block(1, 40.0, 20.0)).under(leaf(2).width(Length::Fill).height(Length::Fill)).over(block(3, 10.0, 10.0));
    let (size, b) = laid(&stack, 100.0);
    assert_eq!(size, Point::new(40.0, 20.0));
    assert_eq!(b, [(2, at(0.0, 0.0, 40.0, 20.0)), (1, at(0.0, 0.0, 40.0, 20.0)), (3, at(0.0, 0.0, 10.0, 10.0))]);
}

#[test]
fn pinned_children_sit_in_the_box_without_changing_its_size() {
    let node = block(1, 100.0, 60.0)
        .padding(5.0)
        .pin(Pin::At(Point::new(70.0, 10.0)), block(2, 50.0, 10.0))
        .pin(Pin::Aligned(Align::End, Align::Center), block(3, 10.0, 10.0))
        .pin(Pin::Aligned(Align::Start, Align::Start), leaf(4).width(Length::Fill).height(Length::Fill));
    let (size, b) = laid(&node, 200.0);
    assert_eq!(size, Point::new(100.0, 60.0));
    // A leaf paints inside its padding; a pin is offered what lies right of and below it.
    assert_eq!(b, [(1, at(5.0, 5.0, 90.0, 50.0)), (2, at(70.0, 10.0, 30.0, 10.0)), (3, at(90.0, 25.0, 10.0, 10.0)), (4, at(0.0, 0.0, 100.0, 60.0))]);
}

#[test]
fn a_wrapping_row_starts_a_line_where_the_next_child_does_not_fit() {
    let words = || Node::wrap((1..=5).map(|i| block(i, 30.0, if i == 2 { 20.0 } else { 10.0 }))).spacing(10.0).line_spacing(4.0);
    let (size, b) = laid(&words(), 100.0);
    assert_eq!(size, Point::new(70.0, 48.0));
    let corners: Vec<(f32, f32)> = b.iter().map(|(_, r)| (r.x, r.y)).collect();
    assert_eq!(corners, [(0.0, 0.0), (40.0, 0.0), (0.0, 24.0), (40.0, 24.0), (0.0, 38.0)]);
    // Each line lines up inside the widest, and each child inside its line.
    let (_, b) = laid(&words().align_x(Align::Center).align_y(Align::Center), 100.0);
    assert_eq!((b[0].1.y, b[4].1.x), (5.0, 20.0));
    // Offered as much as it likes, it keeps to one line.
    assert_eq!(words().measure(f32::INFINITY), Point::new(190.0, 20.0));
}

#[test]
fn text_is_shaped_at_the_width_it_is_given() {
    let spec = Spec::new("Press any button to continue", 16.0);
    let shaped_at = |width: f32| text::shape(&Spec { width: Some(width + SLACK), ..spec.clone() });
    let text = || Node::text(spec.clone(), Rgba::WHITE);
    assert_eq!(text().measure(f32::INFINITY), text::shape(&spec).size);
    let narrow = text::shape(&spec).size.x / 2.0;
    assert_eq!(text().measure(narrow), Point::new(shaped_at(narrow).size.x.min(narrow), shaped_at(narrow).size.y));
    // In a row it's offered what the glyph before it leaves.
    let line = Node::row([block(1, 30.0, 4.0), text()]);
    assert_eq!(line.measure(100.0).y, shaped_at(70.0).size.y);
    // In a column of set width, it is offered that width less the padding, and it's drawn as the
    // run shaped at its box's width, at the box's corner.
    let column = Node::column([text()]).width(120.0).padding(10.0);
    let inside = text().measure(100.0);
    let (size, ops) = column.ops(Point::ZERO, f32::INFINITY);
    assert_eq!(size, Point::new(120.0, inside.y + 20.0));
    let drawn = shaped_at(inside.x);
    let corner = Point::new(10.0, 10.0);
    assert_eq!(ops, [Op::Text { run: drawn.run, at: corner, color: Rgba::WHITE, ink: drawn.ink.shifted(corner) }]);
}

#[test]
fn text_lines_up_inside_its_box() {
    let run = |align: Align, node: fn(Node) -> Node| {
        let spec = Spec { align, ..Spec::new("Menu", 20.0) };
        let (_, ops) = node(Node::text(spec.clone(), Rgba::WHITE).width(200.0)).ops(Point::ZERO, f32::INFINITY);
        let Some(Op::Text { at, .. }) = ops.first() else { panic!("no text in {ops:?}") };
        let shaped = text::shape(&Spec { width: Some(200.0 + SLACK), ..spec });
        (at.x, at.x + shaped.size.x)
    };
    // The spec's alignment is the node's, until the node says otherwise.
    let (left, right) = run(Align::Center, |n| n);
    assert!(((left + right) / 2.0 - 100.0).abs() < 1e-3);
    let (_, right) = run(Align::Start, |n| n.align_x(Align::End));
    assert!((right - 200.0).abs() < 1e-3);
}

#[test]
fn nested_boxes_measure_as_what_they_hold() {
    // A glyph beside words that wrap, over a footer, all centered in a padded panel.
    let words = Node::wrap((1..=4).map(|i| block(i, 40.0, 10.0))).spacing(5.0).line_spacing(2.0);
    let line = Node::row([block(9, 24.0, 24.0), words]).spacing(8.0).align_y(Align::Center);
    let panel = Node::column([line, block(10, 60.0, 12.0)]).spacing(6.0).padding(10.0).align_x(Align::Center);
    // The words get what the glyph leaves: 88, room for two a line.
    let (size, b) = laid(&panel, 140.0);
    assert_eq!(size, Point::new(137.0, 62.0));
    assert_eq!(b[..3], [(9, at(10.0, 10.0, 24.0, 24.0)), (1, at(42.0, 11.0, 40.0, 10.0)), (2, at(87.0, 11.0, 40.0, 10.0))]);
    assert_eq!(b[3..], [(3, at(42.0, 23.0, 40.0, 10.0)), (4, at(87.0, 23.0, 40.0, 10.0)), (10, at(38.5, 40.0, 60.0, 12.0))]);
    // Measured at its own width it comes out the same, so placing it there keeps its lines.
    assert_eq!(panel.measure(137.0), size);
    assert_eq!(panel.measure(f32::INFINITY), Point::new(227.0, 62.0));
}

#[test]
fn clips_and_effects_wrap_what_a_node_draws() {
    let rect = at(10.0, 10.0, 40.0, 20.0);
    let drawn = |node: Node| node.ops(Point::new(10.0, 10.0), f32::INFINITY).1;
    let fill = Some(Rgba::from_rgb8(1, 0, 0).into());
    let moved = block(1, 40.0, 20.0).clip(Corners::all(6.0)).opacity(0.5).scale(1.1).shift(Point::new(3.0, 0.0));
    let layer = Layer { opacity: 0.5, scale: 1.1, about: Point::new(30.0, 20.0), shift: Point::new(3.0, 0.0) };
    let expected = [
        Op::Layer(layer),
        Op::Clip { rect, radii: Corners::all(6.0) },
        Op::Rect { rect, radii: Corners::ZERO, fill, border: None },
        Op::Unclip,
        Op::Unlayer,
    ];
    assert_eq!(drawn(moved), expected);
    // Scaled about a point of its own, from its corner.
    let about = drawn(block(1, 40.0, 20.0).scale(2.0).about(Point::new(0.0, 20.0)));
    assert_eq!(about[0], Op::Layer(Layer { scale: 2.0, about: Point::new(10.0, 30.0), ..Layer::IDENTITY }));
    // Unmoved, it is no layer; faded out, it draws nothing.
    assert_eq!(drawn(block(1, 40.0, 20.0).opacity(1.0)).len(), 1);
    assert!(drawn(block(1, 40.0, 20.0).opacity(0.0)).is_empty());
    // A swelling item doesn't push its neighbors.
    let row = |s: f32| boxes(&drawn(Node::row([block(1, 10.0, 10.0).scale(s), block(2, 10.0, 10.0)])));
    assert_eq!(row(1.5), row(1.0));
}

#[test]
fn a_max_size_holds_the_box_in() {
    let wide = Node::container(block(1, 30.0, 10.0)).width(Length::Fill).max_width(80.0);
    assert_eq!((wide.measure(200.0).x, wide.measure(50.0).x), (80.0, 50.0));
    // A set width is held to what it is offered, as in iced.
    assert_eq!(block(1, 300.0, 10.0).measure(120.0).x, 120.0);
    // Held to a max height, the box is shorter, and what it holds runs on past it.
    let tall = Node::column((1..=3).map(|i| block(i, 10.0, 10.0))).max_height(25.0);
    let (size, b) = laid(&tall, 100.0);
    assert_eq!((size.y, b[2].1.y), (25.0, 20.0));
}

#[test]
fn a_painter_can_be_a_type_of_its_own() {
    struct Bar(f32);
    impl Painter for Bar {
        fn paint(&self, rect: Rect, ops: &mut Vec<Op>) {
            let done = Rect { w: rect.w * self.0, ..rect };
            ops.push(Op::Rect { rect: done, radii: Corners::ZERO, fill: Some(Rgba::from_rgb8(7, 0, 0).into()), border: None });
        }
    }
    let bar = Node::container(Node::leaf(Bar(0.25)).width(Length::Fill).height(6.0)).width(160.0);
    assert_eq!(laid(&bar, 400.0).1, [(7, at(0.0, 0.0, 40.0, 6.0))]);
}
