//! What a part of a panel is (a menu's item, an info overlay's cell), carried on the widget that
//! draws it. The overlay process never asks; the on-screen editor finds what a click landed on
//! by walking the panel for them (see `found_at`).

use iced::{
    Element, Event, Length, Point, Rectangle, Size,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
        widget::{Operation, Tree, tree},
    },
};

/// A part of a panel that can be picked to edit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// A menu's item, by its index in the menu.
    Item(usize),
    /// An info overlay's cell, by row and column.
    Cell(usize, usize),
}

struct Tag<'a, M, T, R> {
    content: Element<'a, M, T, R>,
    part: Part,
}

/// `content`, marked as `part`. It lays out and draws as `content` does, and keeps its widget tree.
pub fn tag<'a, M: 'a, T: 'a, R: iced::advanced::Renderer + 'a>(content: impl Into<Element<'a, M, T, R>>, part: Part) -> Element<'a, M, T, R> {
    Element::new(Tag { content: content.into(), part })
}

impl<M, T, R: iced::advanced::Renderer> Widget<M, T, R> for Tag<'_, M, T, R> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &R, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn draw(&self, tree: &Tree, renderer: &mut R, theme: &T, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &R, operation: &mut dyn Operation) {
        operation.custom(None, layout.bounds(), &mut self.part);
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
    }

    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &R, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }
}

/// The part of the panel `content` under `at`, and where it is.
pub fn found_at<M, T, R: iced::advanced::Renderer>(content: &mut Element<'_, M, T, R>, tree: &mut Tree, layout: Layout<'_>, renderer: &R, at: Point) -> Option<(Part, Rectangle)> {
    let mut find = Find { at, found: None };
    content.as_widget_mut().operate(tree, layout, renderer, &mut find);
    find.found
}

/// Walks a panel for the parts under a point; the innermost one wins.
struct Find {
    at: Point,
    found: Option<(Part, Rectangle)>,
}

impl Operation for Find {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&iced::advanced::widget::Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
        if let Some(part) = state.downcast_ref::<Part>()
            && bounds.contains(self.at)
        {
            self.found = Some((*part, bounds));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{column, container, space};

    #[test]
    fn a_point_finds_the_part_drawn_there() {
        let mut panel: Element<'_, ()> = column![tag(space().width(100).height(40), Part::Item(0)), tag(container(space().width(100).height(40)), Part::Item(1))].into();
        let mut tree = Tree::new(&panel);
        use iced::advanced::renderer::Headless;
        let renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(iced::Font::DEFAULT, iced::Pixels(16.0), Some("tiny-skia"))).unwrap();
        let node = panel.as_widget_mut().layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, Size::new(500.0, 500.0)));
        let mut at = |x, y| found_at(&mut panel, &mut tree, Layout::new(&node), &renderer, Point::new(x, y)).map(|(part, _)| part);
        assert_eq!(at(50.0, 20.0), Some(Part::Item(0)));
        assert_eq!(at(50.0, 60.0), Some(Part::Item(1)));
        assert_eq!(at(150.0, 20.0), None);
    }
}
