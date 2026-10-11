//! A grid menu's cells in the menu editor: merging an item with the cells beside or below it,
//! splitting it again, and the cell the cursor starts on.

use iced::widget::row;

use super::*;
use crate::{config::Span, menu::GridLayout};

/// A cell the cursor can start on, in the start picker: an item's top left cell, by row and
/// column from 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartCell {
    item: usize,
    row: usize,
    column: usize,
    label: String,
}

impl fmt::Display for StartCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.row + 1, self.column + 1)?;
        if !self.label.is_empty() {
            write!(f, "  {}", self.label)?;
        }
        Ok(())
    }
}

/// The picker for the cell grid menu `mi` starts its cursor on, if it is a grid.
pub(super) fn grid_start_picker<'a>(mi: usize, menu: &Menu) -> Option<Element<'a, Message>> {
    let grid = GridLayout::of(menu)?;
    let cells: Vec<StartCell> = (0..menu.items.len())
        .filter_map(|i| grid.cells(i).map(|at| StartCell { item: i, row: at.row, column: at.column, label: menu.items[i].label.clone() }))
        .collect();
    let start = grid.owner(menu.start);
    let current = cells.iter().find(|c| c.item == start).cloned();
    let picker = dropdown(cells, current, move |c: StartCell| Message::SetMenuStart(mi, c.item)).width(200);
    let note = help("Where the cursor is each time the menu opens, as (row, column).".into());
    Some(row![picker, note].spacing(8).align_y(Alignment::Center).into())
}

/// The buttons that merge item `i` of grid menu `mi` with the cells to its right or below, or
/// split it back into single cells.
pub(super) fn merge_buttons<'a>(mi: usize, i: usize, span: Span, grid: &GridLayout) -> Element<'a, Message> {
    let small = |label: &'static str, tip: &'static str, msg: Option<Message>| {
        tooltip(
            button(text(label).size(13)).style(style::secondary).on_press_maybe(msg),
            container(text(tip).size(13)).padding(8).style(style::tooltip),
            tooltip::Position::Top,
        )
    };
    let merge = |right| grid.merged(i, right).map(|span| Message::SetMenuItemSpan(mi, i, span));
    row![
        small("⇥", "Merge with the cells to the right", merge(true)),
        small("⤓", "Merge with the cells below", merge(false)),
        small("⊞", "Split back into single cells", (!span.is_one()).then_some(Message::SetMenuItemSpan(mi, i, Span::ONE))),
    ]
    .spacing(4)
    .into()
}

/// The box of item `i`, which item `owner` is merged over.
pub(super) fn covered_cell<'a>(i: usize, owner: usize) -> Element<'a, Message> {
    let line = row![
        container(text(format!("{}.", i + 1))).width(70),
        text(format!("Merged into item {}. Split that item to use this cell again.", owner + 1)).size(13).color(MUTED_COLOR),
    ]
    .spacing(8);
    container(line).padding(8).style(style::inset).into()
}

impl App {
    pub(super) fn update_grid(&mut self, message: &Message) {
        match *message {
            Message::SetMenuItemSpan(mi, i, span) => {
                if let Some(item) = self.menus_mut().get_mut(mi).and_then(|m| m.items.get_mut(i)) {
                    item.span = span;
                }
            }
            Message::SetMenuStart(mi, cell) => {
                if let Some(menu) = self.menus_mut().get_mut(mi) {
                    menu.start = cell;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn grid_cells_merge_split_and_reset_with_the_columns() {
        let mut app = app();
        let _ = app.update(Message::NewMenu(MenuKindTag::Grid));
        for _ in 0..5 {
            let _ = app.update(Message::AddMenuItem(0));
        }
        let grid = |app: &App| GridLayout::of(&app.menus()[0]).unwrap();
        let wide = grid(&app).merged(0, true).unwrap();
        let _ = app.update(Message::SetMenuItemSpan(0, 0, wide));
        assert_eq!(grid(&app).covered_by(1), Some(0));
        let _ = app.update(Message::SetMenuStart(0, 4));
        assert_eq!(app.menus()[0].start, 4);
        let _ = app.update(Message::SetMenuKind(0, MenuKind::Grid { columns: 2 }));
        assert_eq!((app.menus()[0].items[0].span, app.menus()[0].start), (Span::ONE, 0), "new columns, fresh cells");
        let _ = app.update(Message::SetMenuItemSpan(0, 0, Span { columns: 1, rows: 2 }));
        let _ = app.update(Message::SetMenuItemSpan(0, 0, Span::ONE));
        assert_eq!(grid(&app).covered_by(2), None, "split again");
    }
}
