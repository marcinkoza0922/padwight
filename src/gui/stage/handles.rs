//! A panel on the stage with an outline and grips around it: dragging the middle moves it, an
//! edge or a corner resizes it, and the arrow keys nudge it (Shift for 10 px). Each move of the
//! pointer reports where the panel would be (`Gesture`), measured from where it was when the drag
//! began, so the panel can be redrawn under it as it goes.

use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
        widget::{Tree, tree},
    },
    keyboard::{self, key::Named},
};

use super::snap::{Gesture, Grip};

/// How far outside or inside an edge the pointer still holds it, and a grip's half size.
const REACH: f32 = 7.0;
const GRIP: f32 = 5.0;
/// The outline's color: the stage's accent, which reads on light and dark games alike.
const ACCENT: Color = Color::from_rgb(0.25, 0.65, 1.0);

pub struct Handles<'a, M> {
    content: Element<'a, M>,
    display: Size,
    on_gesture: Box<dyn Fn(Gesture) -> M + 'a>,
}

/// `content` with grips, on a display of `display`.
pub fn handles<'a, M: 'a>(content: impl Into<Element<'a, M>>, display: Size, on_gesture: impl Fn(Gesture) -> M + 'a) -> Element<'a, M> {
    Element::new(Handles { content: content.into(), display, on_gesture: Box::new(on_gesture) })
}

#[derive(Default)]
struct State {
    /// What is held, where the pointer was when it took hold, and where the panel was.
    held: Option<(Grip, Point, Rectangle)>,
    modifiers: keyboard::Modifiers,
}

/// What the pointer at `at` would take hold of on a panel at `bounds`.
fn grip_at(bounds: Rectangle, at: Point) -> Option<Grip> {
    let near = |a: f32, b: f32| (a - b).abs() <= REACH;
    let across = at.x >= bounds.x - REACH && at.x <= bounds.x + bounds.width + REACH;
    let down = at.y >= bounds.y - REACH && at.y <= bounds.y + bounds.height + REACH;
    if !(across && down) {
        return None;
    }
    let (w, e) = (near(at.x, bounds.x), near(at.x, bounds.x + bounds.width));
    let (n, s) = (near(at.y, bounds.y), near(at.y, bounds.y + bounds.height));
    Some(match (n, s, w, e) {
        (true, _, true, _) => Grip::NW,
        (true, _, _, true) => Grip::NE,
        (_, true, true, _) => Grip::SW,
        (_, true, _, true) => Grip::SE,
        (true, ..) => Grip::N,
        (_, true, ..) => Grip::S,
        (.., true, _) => Grip::W,
        (.., true) => Grip::E,
        _ => Grip::Move,
    })
}

/// Where a panel at `from` is when `grip` has been dragged `by`.
fn dragged(from: Rectangle, grip: Grip, by: Vector) -> Rectangle {
    let (mut x0, mut y0, mut x1, mut y1) = (from.x, from.y, from.x + from.width, from.y + from.height);
    match grip {
        Grip::Move => return Rectangle { x: from.x + by.x, y: from.y + by.y, ..from },
        Grip::N => y0 += by.y,
        Grip::S => y1 += by.y,
        Grip::W => x0 += by.x,
        Grip::E => x1 += by.x,
        Grip::NW => (x0, y0) = (x0 + by.x, y0 + by.y),
        Grip::NE => (x1, y0) = (x1 + by.x, y0 + by.y),
        Grip::SW => (x0, y1) = (x0 + by.x, y1 + by.y),
        Grip::SE => (x1, y1) = (x1 + by.x, y1 + by.y),
    }
    Rectangle { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }
}

fn cursor_for(grip: Grip, holding: bool) -> mouse::Interaction {
    match grip {
        Grip::Move if holding => mouse::Interaction::Grabbing,
        Grip::Move => mouse::Interaction::Grab,
        Grip::N | Grip::S => mouse::Interaction::ResizingVertically,
        Grip::E | Grip::W => mouse::Interaction::ResizingHorizontally,
        Grip::NW | Grip::SE => mouse::Interaction::ResizingDiagonallyDown,
        Grip::NE | Grip::SW => mouse::Interaction::ResizingDiagonallyUp,
    }
}

/// How far an arrow key nudges, if `key` is one.
fn nudge(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Vector> {
    let step = if modifiers.shift() { 10.0 } else { 1.0 };
    match key {
        keyboard::Key::Named(Named::ArrowLeft) => Some(Vector::new(-step, 0.0)),
        keyboard::Key::Named(Named::ArrowRight) => Some(Vector::new(step, 0.0)),
        keyboard::Key::Named(Named::ArrowUp) => Some(Vector::new(0.0, -step)),
        keyboard::Key::Named(Named::ArrowDown) => Some(Vector::new(0.0, step)),
        _ => None,
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Handles<'_, M> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(&self, tree: &Tree, renderer: &mut iced::Renderer, theme: &iced::Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        let b = layout.bounds();
        renderer.with_layer(*viewport, |renderer| {
            let outline = Rectangle { x: b.x - 1.0, y: b.y - 1.0, width: b.width + 2.0, height: b.height + 2.0 };
            renderer.fill_quad(
                renderer::Quad { bounds: outline, border: Border { color: ACCENT, width: 1.5, radius: 2.0.into() }, ..renderer::Quad::default() },
                Background::Color(Color::TRANSPARENT),
            );
            let (x0, x1, xm) = (b.x, b.x + b.width, b.x + b.width / 2.0);
            let (y0, y1, ym) = (b.y, b.y + b.height, b.y + b.height / 2.0);
            for (x, y) in [(x0, y0), (xm, y0), (x1, y0), (x0, ym), (x1, ym), (x0, y1), (xm, y1), (x1, y1)] {
                let square = Rectangle { x: x - GRIP, y: y - GRIP, width: GRIP * 2.0, height: GRIP * 2.0 };
                renderer.fill_quad(
                    renderer::Quad { bounds: square, border: Border { color: ACCENT, width: 1.5, radius: 1.0.into() }, ..renderer::Quad::default() },
                    Background::Color(Color::WHITE),
                );
            }
        });
    }

    fn update(&mut self, tree: &mut Tree, event: &Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &iced::Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, M>, viewport: &Rectangle) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();
        let report = |grip, from, to, free| Gesture { grip, from, to, display: self.display, free };
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => state.modifiers = *m,
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) if state.held.is_none() => {
                if let Some(by) = nudge(key, *modifiers) {
                    let to = Rectangle { x: bounds.x + by.x, y: bounds.y + by.y, ..bounds };
                    shell.publish((self.on_gesture)(report(Grip::Move, bounds, to, true)));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(at) = cursor.position()
                    && let Some(grip) = grip_at(bounds, at)
                {
                    state.held = Some((grip, at, bounds));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((grip, start, from)) = state.held {
                    let to = dragged(from, grip, *position - start);
                    shell.publish((self.on_gesture)(report(grip, from, to, state.modifiers.alt())));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.held.take().is_some() => shell.capture_event(),
            // Clicks belong to the grips; the rest (a scroll inside a long panel) to the panel.
            Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_)) => {}
            _ => self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport),
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, _viewport: &Rectangle, _renderer: &iced::Renderer) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if let Some((grip, ..)) = state.held {
            return cursor_for(grip, true);
        }
        cursor.position().and_then(|at| grip_at(layout.bounds(), at)).map_or(mouse::Interaction::None, |grip| cursor_for(grip, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANEL: Rectangle = Rectangle { x: 100.0, y: 100.0, width: 200.0, height: 100.0 };

    #[test]
    fn edges_and_corners_resize_and_the_middle_moves() {
        assert_eq!(grip_at(PANEL, Point::new(200.0, 150.0)), Some(Grip::Move));
        assert_eq!(grip_at(PANEL, Point::new(98.0, 103.0)), Some(Grip::NW));
        assert_eq!(grip_at(PANEL, Point::new(303.0, 199.0)), Some(Grip::SE));
        assert_eq!(grip_at(PANEL, Point::new(200.0, 204.0)), Some(Grip::S));
        assert_eq!(grip_at(PANEL, Point::new(95.0, 150.0)), Some(Grip::W));
        assert_eq!(grip_at(PANEL, Point::new(50.0, 150.0)), None, "outside reach");
    }

    #[test]
    fn a_drag_moves_only_the_edges_held() {
        let by = Vector::new(30.0, -20.0);
        assert_eq!(dragged(PANEL, Grip::Move, by), Rectangle { x: 130.0, y: 80.0, ..PANEL });
        assert_eq!(dragged(PANEL, Grip::E, by), Rectangle { width: 230.0, ..PANEL });
        assert_eq!(dragged(PANEL, Grip::NW, by), Rectangle { x: 130.0, y: 80.0, width: 170.0, height: 120.0 });
    }
}
