//! Building a layout tree: the kinds of node, their children, and their box.

use super::{Align, Effect, Kind, Length, Node, Padding, Painter, Pin, Spec};
use crate::overlay::render::{Corners, Point, Rgba};

/// Building a tree.
impl Node {
    fn of(kind: Kind) -> Node {
        Node {
            kind,
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            padding: Padding::ZERO,
            spacing: 0.0,
            line_spacing: None,
            align_x: Align::Start,
            align_y: Align::Start,
            clip: None,
            effect: Effect::NONE,
            pins: Vec::new(),
        }
    }

    /// Nothing, as big as its lengths say (none until they're set).
    pub fn space() -> Node {
        Node::of(Kind::Space)
    }

    /// A run of text in `color`. It wraps at the width it is given; its lines line up as
    /// `align_x` says, which starts as the spec's `align`. Its spec's `width` is the node's to set.
    pub fn text(spec: Spec, color: Rgba) -> Node {
        let align_x = spec.align;
        Node { align_x, ..Node::of(Kind::Text { spec, color }) }
    }

    /// Whatever `painter` draws, as big as its lengths say.
    pub fn leaf(painter: impl Painter + 'static) -> Node {
        Node::of(Kind::Leaf(Box::new(painter)))
    }

    /// Children side by side, left to right.
    pub fn row(children: impl IntoIterator<Item = Node>) -> Node {
        Node::of(Kind::Row(Vec::new())).extend(children)
    }

    /// Children one under another, top to bottom.
    pub fn column(children: impl IntoIterator<Item = Node>) -> Node {
        Node::of(Kind::Column(Vec::new())).extend(children)
    }

    /// Children side by side, starting a new line below (`line_spacing` apart) where the next
    /// one doesn't fit. Each is offered the whole width.
    pub fn wrap(children: impl IntoIterator<Item = Node>) -> Node {
        Node::of(Kind::Wrap(Vec::new())).extend(children)
    }

    /// One child in a box: a column of one, as iced's container is.
    pub fn container(child: Node) -> Node {
        Node::column([child])
    }

    /// `base`, with layers to come `under` and `over` it: the base sets the stack's size, and
    /// the layers are offered that size (one that fills fills it).
    pub fn stack(base: Node) -> Node {
        let (width, height) = (Length::Shrink.enclose(base.width), Length::Shrink.enclose(base.height));
        Node { width, height, ..Node::of(Kind::Stack { layers: vec![base], base: 0 }) }
    }

    /// Adds a child at the end: to the right of a row, under a column, on top of a stack.
    pub fn push(mut self, child: Node) -> Node {
        match &mut self.kind {
            Kind::Row(children) | Kind::Column(children) | Kind::Wrap(children) => {
                self.width = self.width.enclose(child.width);
                self.height = self.height.enclose(child.height);
                children.push(child);
            }
            Kind::Stack { layers, .. } => layers.push(child),
            Kind::Space | Kind::Text { .. } | Kind::Leaf(_) => panic!("only rows, columns, wrapping rows and stacks hold children"),
        }
        self
    }

    pub fn extend(self, children: impl IntoIterator<Item = Node>) -> Node {
        children.into_iter().fold(self, Node::push)
    }

    /// A stack's layer on top of the others.
    pub fn over(self, layer: Node) -> Node {
        self.push(layer)
    }

    /// A stack's layer under the others.
    pub fn under(mut self, layer: Node) -> Node {
        let Kind::Stack { layers, base } = &mut self.kind else { panic!("only stacks have layers under them") };
        layers.insert(0, layer);
        *base += 1;
        self
    }

    /// `child` placed in this node's box as `pin` says, over its contents.
    pub fn pin(mut self, pin: Pin, child: Node) -> Node {
        self.pins.push((pin, child));
        self
    }
}

/// A node's box.
impl Node {
    pub fn width(self, width: impl Into<Length>) -> Node {
        Node { width: width.into(), ..self }
    }

    pub fn height(self, height: impl Into<Length>) -> Node {
        Node { height: height.into(), ..self }
    }

    pub fn max_width(self, max: f32) -> Node {
        Node { max_width: max, ..self }
    }

    /// The box is held to this height; what it holds can reach past it unless it clips.
    pub fn max_height(self, max: f32) -> Node {
        Node { max_height: max, ..self }
    }

    pub fn padding(self, padding: impl Into<Padding>) -> Node {
        Node { padding: padding.into(), ..self }
    }

    /// The space between children (and between a wrapping row's lines, unless `line_spacing` says).
    pub fn spacing(self, spacing: f32) -> Node {
        Node { spacing, ..self }
    }

    /// The space between a wrapping row's lines.
    pub fn line_spacing(self, spacing: f32) -> Node {
        Node { line_spacing: Some(spacing), ..self }
    }

    pub fn align_x(self, align: Align) -> Node {
        Node { align_x: align, ..self }
    }

    pub fn align_y(self, align: Align) -> Node {
        Node { align_y: align, ..self }
    }

    /// `width` wide, its contents centered across it.
    pub fn center_x(self, width: impl Into<Length>) -> Node {
        self.width(width).align_x(Align::Center)
    }

    /// `height` tall, its contents centered down it.
    pub fn center_y(self, height: impl Into<Length>) -> Node {
        self.height(height).align_y(Align::Center)
    }

    /// Draws what it holds (pins too) only inside its box, rounded by `radii`.
    pub fn clip(self, radii: Corners) -> Node {
        Node { clip: Some(radii), ..self }
    }

    pub fn opacity(mut self, opacity: f32) -> Node {
        self.effect.opacity = opacity;
        self
    }

    /// Drawn `scale` times its size, about its center unless `about` says.
    pub fn scale(mut self, scale: f32) -> Node {
        self.effect.scale = scale;
        self
    }

    /// Drawn this far from where it is laid out.
    pub fn shift(mut self, by: Point) -> Node {
        self.effect.shift = by;
        self
    }

    /// The point it scales about, from its box's top left corner.
    pub fn about(mut self, at: Point) -> Node {
        self.effect.about = Some(at);
        self
    }

    /// Drawn as a layer even when it isn't faded, scaled or moved, so the screen can tell where
    /// it starts and ends (each panel is a picture of its own).
    pub fn layered(mut self) -> Node {
        self.effect.always = true;
        self
    }
}
