//! A click on the overlay: the item or cell under it opens in the inspector's Content tab, its
//! field scrolled into view and focused to type in.

use iced::{
    Rectangle, Vector,
    advanced::widget::{Id, Operation, operation},
};

use super::*;

/// The inspector's scrolling page, which the field is looked for in: the settings window has the
/// same fields, and they stay as they are.
pub(super) const PAGE: &str = "stage-page";

/// The label field of item `i` of menu `menu`.
pub(in crate::gui) fn item_field(menu: usize, i: usize) -> Id {
    Id::from(format!("menu-item-label:{menu}:{i}"))
}

/// The field of cell `c` in row `r` of info overlay `info`.
pub(in crate::gui) fn cell_field(info: usize, r: usize, c: usize) -> Id {
    Id::from(format!("info-cell:{info}:{r}:{c}"))
}

impl App {
    pub(super) fn pick(&mut self, part: Part) -> Task<Message> {
        let Some(stage) = &mut self.stage else { return Task::none() };
        let field = match (stage.target, part) {
            (StageTarget::Menu(m), Part::Item(i)) => item_field(m, i),
            (StageTarget::Info(o), Part::Cell(r, c)) => cell_field(o, r, c),
            _ => return Task::none(),
        };
        stage.tab = StageTab::Content;
        stage.picked = Some(part);
        stage.folded = false;
        iced::advanced::widget::operate(Reveal { field: Some(field), ..Reveal::default() }).discard()
    }
}

/// Focuses the field with the id `field` on the inspector's page, and scrolls the page to it.
#[derive(Default)]
struct Reveal {
    field: Option<Id>,
    /// Set by the page, for the walk into it that comes next.
    entering: bool,
    inside: bool,
    /// The page's box and the box of all it holds, and the field's box.
    page: Option<(Rectangle, Rectangle)>,
    found: Option<Rectangle>,
}

impl Operation for Reveal {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let was = self.inside;
        self.inside |= std::mem::take(&mut self.entering);
        operate(self);
        self.inside = was;
    }

    fn scrollable(&mut self, id: Option<&Id>, bounds: Rectangle, content_bounds: Rectangle, _translation: Vector, _state: &mut dyn operation::Scrollable) {
        if id == Some(&Id::new(PAGE)) {
            self.entering = true;
            self.page = Some((bounds, content_bounds));
        }
    }

    fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn operation::TextInput) {
        if self.inside && id.is_some() && id == self.field.as_ref() {
            self.found = Some(bounds);
        }
    }

    fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn operation::Focusable) {
        if self.inside && id.is_some() && id == self.field.as_ref() {
            state.focus();
        } else {
            state.unfocus();
        }
    }

    fn finish(&self) -> operation::Outcome<()> {
        let (Some((page, contents)), Some(field)) = (self.page, self.found) else { return operation::Outcome::None };
        // A third of the way down, so what's around it shows too.
        let y = (field.y - contents.y - page.height / 3.0).max(0.0);
        operation::Outcome::Chain(Box::new(operation::scrollable::scroll_to(Id::new(PAGE), operation::scrollable::AbsoluteOffset { x: None, y: Some(y) })))
    }
}
