//! Where the panel being laid down starts, for the pictures of filters on the items inside it.
//!
//! A filter's picture spans the whole panel, and each item shows the part under it, so a filter
//! runs on unbroken across items (see `Op::Pattern`). The panel's frame sets its top left corner
//! as it emits its ops, before anything inside it, as iced's anchor widget does; the item boxes
//! that follow read it. Outside a panel it is the origin.

use std::cell::Cell;

use super::Point;

thread_local! {
    static ORIGIN: Cell<Point> = const { Cell::new(Point::ZERO) };
}

/// Marks `at` as the corner of the panel whose ops come next.
pub(super) fn set(at: Point) {
    ORIGIN.set(at);
}

/// The corner of the panel being laid down.
pub(super) fn origin() -> Point {
    ORIGIN.get()
}
