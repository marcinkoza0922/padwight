//! A panel on the stage with an outline and grips around it: dragging the middle moves it, an
//! edge or a corner resizes it, and the arrow keys nudge it (Shift for 10 px). Each move of the
//! pointer reports where the panel would be (`Gesture`), measured from where it was when the drag
//! began, so the panel can be redrawn under it as it goes. A click that doesn't drag picks the
//! item or cell under it (`Part`), which is outlined as the pointer passes over it.

use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Size, Vector,
    advanced::{
        Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
        widget::{Tree, tree},
    },
    keyboard::{self, key::Named},
};

use super::snap::{Gesture, Grip};
use crate::overlay::draw::{Part, found_at};

/// How far outside or inside an edge the pointer still holds it, and a grip's half size.
const REACH: f32 = 7.0;
const GRIP: f32 = 5.0;
/// How far the pointer goes before a press is a drag rather than a click.
const SLOP: f32 = 4.0;
/// The outline's color: the stage's accent, which reads on light and dark games alike.
const ACCENT: Color = Color::from_rgb(0.25, 0.65, 1.0);

/// The flash's color: bright against a dark wallpaper and a black overlay both.
const FLASH: Color = Color::from_rgb(1.0, 0.85, 0.2);

pub struct Handles<'a, M> {
    content: Element<'a, M>,
    display: Size,
    /// How strongly the panel is flashing, 0 to 1.
    flash: f32,
    on_gesture: Box<dyn Fn(Gesture) -> M + 'a>,
    on_pick: Box<dyn Fn(Part) -> M + 'a>,
}

/// `content` with grips, on a display of `display`.
pub fn handles<'a, M: 'a>(content: impl Into<Element<'a, M>>, display: Size, flash: f32, on_gesture: impl Fn(Gesture) -> M + 'a, on_pick: impl Fn(Part) -> M + 'a) -> Element<'a, M> {
    Element::new(Handles { content: content.into(), display, flash, on_gesture: Box::new(on_gesture), on_pick: Box::new(on_pick) })
}

#[derive(Default)]
struct State {
    held: Option<Hold>,
    /// The part under the pointer, and where it is.
    hovered: Option<(Part, Rectangle)>,
    modifiers: keyboard::Modifiers,
}

/// What is held, where the pointer was when it took hold, where the panel was, and whether it
/// has gone far enough to be a drag.
#[derive(Clone, Copy)]
struct Hold {
    grip: Grip,
    start: Point,
    from: Rectangle,
    dragging: bool,
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
            if self.flash > 0.0 {
                let glow = Rectangle { x: b.x - 6.0, y: b.y - 6.0, width: b.width + 12.0, height: b.height + 12.0 };
                renderer.fill_quad(
                    renderer::Quad { bounds: glow, border: Border { color: Color { a: self.flash, ..FLASH }, width: 6.0, radius: 8.0.into() }, ..renderer::Quad::default() },
                    Background::Color(Color { a: self.flash * 0.35, ..FLASH }),
                );
            }
            let (x0, x1, xm) = (b.x, b.x + b.width, b.x + b.width / 2.0);
            let (y0, y1, ym) = (b.y, b.y + b.height, b.y + b.height / 2.0);
            if let Some((_, part)) = tree.state.downcast_ref::<State>().hovered {
                renderer.fill_quad(
                    renderer::Quad { bounds: part, border: Border { color: ACCENT, width: 1.5, radius: 4.0.into() }, ..renderer::Quad::default() },
                    Background::Color(Color { a: 0.15, ..ACCENT }),
                );
            }
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
                    state.held = Some(Hold { grip, start: at, from: bounds, dragging: false });
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => match &mut state.held {
                Some(hold) => {
                    let by = *position - hold.start;
                    // A hand that shakes a little as it clicks doesn't move the panel, which
                    // with snapping on could jump a whole grid step.
                    hold.dragging |= by.x.abs().max(by.y.abs()) > SLOP;
                    if hold.dragging {
                        let to = dragged(hold.from, hold.grip, by);
                        shell.publish((self.on_gesture)(report(hold.grip, hold.from, to, state.modifiers.alt())));
                        state.hovered = None;
                    }
                    shell.capture_event();
                }
                None => {
                    let over = grip_at(bounds, *position) == Some(Grip::Move);
                    let hovered = over.then(|| found_at(&mut self.content, &mut tree.children[0], layout, renderer, *position)).flatten();
                    let state = tree.state.downcast_mut::<State>();
                    if hovered != state.hovered {
                        state.hovered = hovered;
                        shell.request_redraw();
                    }
                }
            },
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.held.is_some() => {
                if let Some(Hold { grip: Grip::Move, start, dragging: false, .. }) = state.held.take()
                    && let Some((part, _)) = found_at(&mut self.content, &mut tree.children[0], layout, renderer, start)
                {
                    shell.publish((self.on_pick)(part));
                }
                shell.capture_event();
            }
            // Clicks belong to the grips; the rest (a scroll inside a long panel) to the panel.
            Event::Mouse(mouse::Event::ButtonPressed(_) | mouse::Event::ButtonReleased(_)) => {}
            _ => self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport),
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, _viewport: &Rectangle, _renderer: &iced::Renderer) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if let Some(hold) = state.held {
            return cursor_for(hold.grip, hold.dragging);
        }
        if state.hovered.is_some_and(|(_, part)| cursor.is_over(part)) {
            return mouse::Interaction::Pointer;
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

    #[derive(Debug, Clone, PartialEq)]
    enum Got {
        Moved(Grip),
        Picked(Part),
    }

    /// Two items, one over the other, held by grips: what pressing at `at`, moving to `to` and
    /// letting go sends.
    fn press_move_release(at: Point, to: Point) -> Vec<Got> {
        use crate::overlay::draw::tag;
        use iced::widget::{column, space};
        let items = column![tag(space().width(200).height(50), Part::Item(0)), tag(space().width(200).height(50), Part::Item(1))];
        let panel = handles(items, Size::new(1000.0, 1000.0), 0.0, |g| Got::Moved(g.grip), Got::Picked);
        let mut sim = iced_test::Simulator::with_size(iced::Settings::default(), Size::new(1000.0, 1000.0), panel);
        sim.point_at(at);
        let _ = sim.simulate([Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))]);
        sim.point_at(to);
        let _ = sim.simulate([Event::Mouse(mouse::Event::CursorMoved { position: to }), Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))]);
        sim.into_messages().collect()
    }

    #[test]
    fn a_click_picks_the_item_under_it_and_a_drag_moves_the_panel() {
        assert_eq!(press_move_release(Point::new(100.0, 75.0), Point::new(100.0, 75.0)), [Got::Picked(Part::Item(1))]);
        assert_eq!(press_move_release(Point::new(100.0, 75.0), Point::new(102.0, 77.0)), [Got::Picked(Part::Item(1))], "a shaky click is still a click");
        assert_eq!(press_move_release(Point::new(100.0, 25.0), Point::new(130.0, 25.0)), [Got::Moved(Grip::Move)]);
        assert_eq!(press_move_release(Point::new(200.0, 25.0), Point::new(200.0, 25.0)), [], "an edge picks nothing");
    }

    #[test]
    fn a_drag_moves_only_the_edges_held() {
        let by = Vector::new(30.0, -20.0);
        assert_eq!(dragged(PANEL, Grip::Move, by), Rectangle { x: 130.0, y: 80.0, ..PANEL });
        assert_eq!(dragged(PANEL, Grip::E, by), Rectangle { width: 230.0, ..PANEL });
        assert_eq!(dragged(PANEL, Grip::NW, by), Rectangle { x: 130.0, y: 80.0, width: 170.0, height: 120.0 });
    }
}
