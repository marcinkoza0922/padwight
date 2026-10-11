//! Where a grid menu's items sit, now that an item can cover several cells, and moving between
//! them.

use super::Step;
use crate::config::{GRID_MAX, Span};

/// The cells an item covers: its top left cell, and how many columns and rows from there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cells {
    pub column: usize,
    pub row: usize,
    pub columns: usize,
    pub rows: usize,
}

/// A grid's items laid out on its cells. Item `i` belongs in cell `i` (left to right, then down)
/// and covers its span from there, as far as there are cells free. An item whose own cell an
/// earlier item covers is hidden (it comes back once that item is split again).
#[derive(Debug, Clone, PartialEq)]
pub struct GridLayout {
    pub columns: usize,
    pub rows: usize,
    /// The item covering each cell. There are as many cells as items, so the last row may be short.
    owners: Vec<Option<usize>>,
    /// Each item's cells, or `None` while it is hidden.
    cells: Vec<Option<Cells>>,
}

impl GridLayout {
    /// The layout of items with these spans, `columns` wide.
    pub fn new(spans: &[Span], columns: usize) -> Self {
        let n = spans.len();
        // Fewer items than columns make one short row.
        let columns = columns.clamp(1, GRID_MAX).min(n.max(1));
        let mut owners = vec![None; n];
        let mut cells = vec![None; n];
        let free = |owners: &[Option<usize>], cell: usize| owners.get(cell).is_some_and(Option::is_none);
        for (i, span) in spans.iter().enumerate() {
            if owners[i].is_some() {
                continue;
            }
            let (column, row) = (i % columns, i / columns);
            let mut wide = 1;
            while wide < usize::from(span.columns) && column + wide < columns && free(&owners, i + wide) {
                wide += 1;
            }
            let mut tall = 1;
            while tall < usize::from(span.rows) && (0..wide).all(|c| free(&owners, i + tall * columns + c)) {
                tall += 1;
            }
            for r in 0..tall {
                for c in 0..wide {
                    owners[i + r * columns + c] = Some(i);
                }
            }
            cells[i] = Some(Cells { column, row, columns: wide, rows: tall });
        }
        GridLayout { columns, rows: n.div_ceil(columns), owners, cells }
    }

    /// The layout of `menu`'s items, if it is a grid.
    pub fn of(menu: &crate::config::Menu) -> Option<Self> {
        let columns = menu.kind.grid_columns()?;
        Some(Self::new(&menu.items.iter().map(|item| item.span).collect::<Vec<_>>(), columns))
    }

    /// The cells item `i` covers, or `None` if it is hidden (or there's no such item).
    pub fn cells(&self, i: usize) -> Option<Cells> {
        self.cells.get(i).copied().flatten()
    }

    /// The item covering the cell at `column`, `row`, if any.
    pub fn at(&self, column: usize, row: usize) -> Option<usize> {
        if column >= self.columns {
            return None;
        }
        self.owners.get(row * self.columns + column).copied().flatten()
    }

    /// The item covering cell `cell` (counting left to right, then down), or the first item.
    pub fn owner(&self, cell: usize) -> usize {
        self.owners.get(cell).copied().flatten().unwrap_or(0)
    }

    /// The next item from `cursor` one way, wrapping round within the row (or column) of its top
    /// left cell. Cells past a short last row are skipped.
    pub fn step(&self, cursor: usize, (dx, dy): Step) -> usize {
        let Some(at) = self.cells(cursor) else { return self.owner(cursor) };
        let (mut column, mut row) = (at.column as i32, at.row as i32);
        let (columns, rows) = (self.columns as i32, self.rows as i32);
        for _ in 0..columns.max(rows) {
            column = (column + dx).rem_euclid(columns);
            row = (row + dy).rem_euclid(rows);
            if let Some(next) = self.at(column as usize, row as usize)
                && next != cursor
            {
                return next;
            }
        }
        cursor
    }

    /// Item `i`'s span after merging in the cells next to it, to the right or below, or `None`
    /// if that can't be done: there are no cells there, or an item there reaches outside them.
    pub fn merged(&self, i: usize, right: bool) -> Option<Span> {
        let at = self.cells(i)?;
        let (columns, rows) = if right { (at.columns + 1, at.rows) } else { (at.columns, at.rows + 1) };
        if at.column + columns > self.columns {
            return None;
        }
        // Every cell taken in must exist, and any item there must fit wholly inside.
        let inside = |c: Cells| c.column >= at.column && c.row >= at.row && c.column + c.columns <= at.column + columns && c.row + c.rows <= at.row + rows;
        for row in at.row..at.row + rows {
            for column in at.column..at.column + columns {
                let owner = self.at(column, row)?;
                if owner != i && !self.cells(owner).is_some_and(inside) {
                    return None;
                }
            }
        }
        Some(Span { columns: u8::try_from(columns).ok()?, rows: u8::try_from(rows).ok()? })
    }

    /// The item hiding item `i`, if another covers its cell.
    pub fn covered_by(&self, i: usize) -> Option<usize> {
        self.owners.get(i).copied().flatten().filter(|&owner| owner != i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(n: usize) -> Vec<Span> {
        vec![Span::ONE; n]
    }

    #[test]
    fn steps_cover_every_cell_and_undo() {
        for columns in 1..=GRID_MAX {
            for n in 1..=GRID_MAX * GRID_MAX {
                let grid = GridLayout::new(&spans(n), columns);
                for cursor in 0..n {
                    for dir in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let next = grid.step(cursor, dir);
                        assert!(next < n, "{columns} columns, {n} items, {cursor} {dir:?}");
                        assert_eq!(grid.step(next, (-dir.0, -dir.1)), cursor, "steps undo");
                    }
                }
            }
        }
    }

    #[test]
    fn a_merged_item_hides_the_cells_it_covers_and_is_stepped_over_as_one() {
        // A A 2      (item 0 covers two columns and two rows)
        // A A 5
        // 6 7 8
        let mut s = spans(9);
        s[0] = Span { columns: 2, rows: 2 };
        let grid = GridLayout::new(&s, 3);
        assert_eq!(grid.cells(0), Some(Cells { column: 0, row: 0, columns: 2, rows: 2 }));
        assert_eq!([1, 3, 4].map(|i| grid.cells(i)), [None; 3], "covered items are hidden");
        assert_eq!(grid.covered_by(4), Some(0));
        assert_eq!(grid.step(0, (1, 0)), 2, "right goes past its own cells");
        assert_eq!(grid.step(0, (0, 1)), 6);
        assert_eq!(grid.step(7, (0, -1)), 0, "from below, up lands on it");
        assert_eq!(grid.step(5, (1, 0)), 0, "wrapping right from the end of row two");
    }

    #[test]
    fn merging_takes_in_whole_items_only_and_stays_on_the_grid() {
        let mut s = spans(6);
        let grid = GridLayout::new(&s, 3);
        assert_eq!(grid.merged(0, true), Some(Span { columns: 2, rows: 1 }));
        assert_eq!(grid.merged(2, true), None, "nothing right of the last column");
        assert_eq!(grid.merged(3, false), None, "no row below the last");
        // Item 1 spans down; item 0 can't take half of it.
        s[1] = Span { columns: 1, rows: 2 };
        let grid = GridLayout::new(&s, 3);
        assert_eq!(grid.merged(0, false), Some(Span { columns: 1, rows: 2 }));
        assert_eq!(grid.merged(0, true), None, "item 1 reaches below the merge");
        s[0] = Span { columns: 1, rows: 2 };
        assert_eq!(GridLayout::new(&s, 3).merged(0, true), Some(Span { columns: 2, rows: 2 }), "now it fits inside");
    }

    #[test]
    fn a_span_too_big_for_the_grid_covers_only_what_there_is() {
        let mut s = spans(6);
        s[2] = Span { columns: 3, rows: 3 };
        let grid = GridLayout::new(&s, 3);
        assert_eq!(grid.cells(2), Some(Cells { column: 2, row: 0, columns: 1, rows: 2 }), "one column left, and a row below");
        assert_eq!(grid.cells(3), Some(Cells { column: 0, row: 1, columns: 1, rows: 1 }));
    }
}
