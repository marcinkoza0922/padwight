//! Reads the laid-out text of a window from iced's widget tree: where each text widget sits, and
//! where its parents and the scrolled viewports around it end. iced offers no text measuring
//! through `Operation`, so wrapping is found by laying the same view out twice (see `scan`).

use std::collections::HashMap;

use iced::{
    Rectangle, Vector,
    advanced::widget::{Id, Operation, operation::Scrollable},
};

/// One text widget as laid out.
#[derive(Debug, Clone)]
pub struct Seen {
    pub text: String,
    pub bounds: Rectangle,
    /// The part inside every scrolled viewport around it (screen coordinates).
    pub visible: Option<Rectangle>,
    /// The container directly around it, if any.
    pub parent: Option<Rectangle>,
}

/// Collects every text widget, depth first.
#[derive(Default)]
pub struct Probe {
    pub texts: Vec<Seen>,
    viewport: Option<Rectangle>,
    translation: Vector,
    parent: Option<Rectangle>,
    next_parent: Option<Rectangle>,
    saved: Vec<(Option<Rectangle>, Vector, Option<Rectangle>)>,
}

impl Probe {
    pub fn new() -> Self {
        Probe { viewport: Some(Rectangle::INFINITE), ..Default::default() }
    }
}

impl Operation for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        self.saved.push((self.viewport, self.translation, self.parent));
        self.parent = self.next_parent.take().or(self.parent);
        operate(self);
        if let Some((viewport, translation, parent)) = self.saved.pop() {
            (self.viewport, self.translation, self.parent) = (viewport, translation, parent);
        }
    }

    fn container(&mut self, _id: Option<&Id>, bounds: Rectangle) {
        self.next_parent = Some(bounds);
    }

    fn scrollable(&mut self, _id: Option<&Id>, bounds: Rectangle, _content: Rectangle, translation: Vector, _state: &mut dyn Scrollable) {
        let visible = self.viewport.and_then(|v| v.intersection(&(bounds + self.translation)));
        // Its content is as tall as it needs to be, so it isn't a parent to fit inside.
        self.next_parent = None;
        self.parent = None;
        self.translation -= translation;
        self.viewport = Some(visible.unwrap_or_default());
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        let visible = self.viewport.and_then(|v| v.intersection(&(bounds + self.translation)));
        self.texts.push(Seen { text: text.to_string(), bounds: bounds + self.translation, visible, parent: self.parent.map(|p| p + self.translation) });
    }
}

/// How the text widgets of a view sit once a window `wide` enough to hold every label on one line
/// is laid out, by text and how many times it came before: what each one measures unwrapped.
pub fn natural(wide: &[Seen]) -> HashMap<(&str, usize), Rectangle> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut out = HashMap::new();
    for s in wide {
        let n = seen.entry(&s.text).or_default();
        out.insert((s.text.as_str(), *n), s.bounds);
        *n += 1;
    }
    out
}
