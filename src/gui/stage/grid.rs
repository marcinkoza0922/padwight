//! The stage's backdrop: an optional dimming of the screen, and the grid the panel snaps to, with
//! every fourth line and the display's middle drawn stronger.

use iced::{
    Background, Color, Element, Length, Rectangle, Size,
    advanced::{Layout, Widget, layout, mouse, renderer, widget::Tree},
};

struct Grid {
    step: Option<f32>,
    shade: f32,
}

/// Lines every `step` pixels (none when unset) over the screen darkened by `shade` (0..1).
pub fn grid<'a, M: 'a>(step: Option<f32>, shade: f32) -> Element<'a, M> {
    Element::new(Grid { step: step.filter(|s| *s >= 2.0), shade })
}

/// A line in two tones, so it shows over light and dark games alike.
fn line<R: iced::advanced::Renderer>(renderer: &mut R, bounds: Rectangle, strength: f32) {
    let dark = Rectangle { x: bounds.x + 1.0, y: bounds.y + 1.0, ..bounds };
    renderer.fill_quad(renderer::Quad { bounds: dark, ..renderer::Quad::default() }, Background::Color(Color::from_rgba(0.0, 0.0, 0.0, strength)));
    renderer.fill_quad(renderer::Quad { bounds, ..renderer::Quad::default() }, Background::Color(Color::from_rgba(1.0, 1.0, 1.0, strength)));
}

impl<M, T, R: iced::advanced::Renderer> Widget<M, T, R> for Grid {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(&self, _tree: &Tree, renderer: &mut R, _theme: &T, _style: &renderer::Style, layout: Layout<'_>, _cursor: mouse::Cursor, _viewport: &Rectangle) {
        let b = layout.bounds();
        if self.shade > 0.0 {
            renderer.fill_quad(renderer::Quad { bounds: b, ..renderer::Quad::default() }, Background::Color(Color::from_rgba(0.0, 0.0, 0.0, self.shade)));
        }
        let Some(step) = self.step else { return };
        let strength = |i: u32| if i.is_multiple_of(4) { 0.16 } else { 0.07 };
        let mut i = 1;
        while i as f32 * step < b.width {
            line(renderer, Rectangle { x: b.x + i as f32 * step, y: b.y, width: 1.0, height: b.height }, strength(i));
            i += 1;
        }
        i = 1;
        while i as f32 * step < b.height {
            line(renderer, Rectangle { x: b.x, y: b.y + i as f32 * step, width: b.width, height: 1.0 }, strength(i));
            i += 1;
        }
        let middle = b.center();
        line(renderer, Rectangle { x: middle.x, y: b.y, width: 1.0, height: b.height }, 0.3);
        line(renderer, Rectangle { x: b.x, y: middle.y, width: b.width, height: 1.0 }, 0.3);
    }
}
