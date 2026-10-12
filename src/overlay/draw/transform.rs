//! A widget drawn scaled about its center and shifted, while laid out where it sits unmoved:
//! how panels zoom and items bounce and slide in without pushing what is around them.

use iced::{
    Element, Event, Length, Rectangle, Size, Transformation, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, renderer,
        widget::{Operation, Tree, tree},
    },
};

struct Transform<'a, M, T, R> {
    content: Element<'a, M, T, R>,
    scale: f32,
    shift: Vector,
}

/// `content` drawn `scale` times its size about its center and `shift` pixels off its place.
/// The widget tree is the content's own, so a panel keeps its state as it starts and stops moving.
pub fn transform<'a, M: 'a, T: 'a, R: iced::advanced::Renderer + 'a>(content: impl Into<Element<'a, M, T, R>>, scale: f32, shift: Vector) -> Element<'a, M, T, R> {
    let content = content.into();
    if (scale - 1.0).abs() < 1e-4 && shift == Vector::ZERO {
        return content;
    }
    Element::new(Transform { content, scale: scale.max(0.0), shift })
}

impl<M, T, R: iced::advanced::Renderer> Widget<M, T, R> for Transform<'_, M, T, R> {
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
        let center = layout.bounds().center();
        let moved = Transformation::translate(center.x + self.shift.x, center.y + self.shift.y)
            * Transformation::scale(self.scale)
            * Transformation::translate(-center.x, -center.y);
        // The content culls against the part of the screen that lands where it is drawn.
        let seen = *viewport * moved.inverse();
        renderer.with_transformation(moved, |renderer| {
            self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, &seen);
        });
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
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
    }

    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &R, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }
}
