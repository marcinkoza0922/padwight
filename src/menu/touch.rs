//! Menus laid out on a touchpad: which item is under a finger, and what the overlay shows while
//! a finger is on the pad. The engine runs the items; nothing here takes the controller.

use super::{MenuView, item_views, list_columns};
use crate::config::{Menu, MenuKind, TouchChoose};

/// A finger this close to the middle (in widths of the area) points at nothing in a radial or
/// directional menu, so landing in the middle doesn't pick an item at random.
const CENTER_RADIUS: f32 = 0.12;

/// The item of `menu` under a finger at (`x`, `y`), each 0..1 across the area the menu covers,
/// which is `height` widths tall. The items are where the overlay draws them: a radial menu by
/// angle from the middle (first item at the top, going clockwise), a directional menu by the
/// direction from the middle, a grid by cells, a list by rows (in two columns once it is long),
/// and a carousel from left to right.
pub fn item_at(menu: &Menu, (x, y): (f32, f32), height: f32) -> Option<usize> {
    let n = menu.items.len();
    if n == 0 {
        return None;
    }
    let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
    // Which of `count` equal strips a share of 0..1 falls in.
    let strip = |share: f32, count: usize| ((share * count as f32) as usize).min(count - 1);
    let (dx, dy) = (x - 0.5, (y - 0.5) * height);
    let central = dx.hypot(dy) < CENTER_RADIUS;
    let i = match menu.kind {
        MenuKind::Radial { boxes, .. } => {
            if central {
                return None;
            }
            // Clockwise from straight up; y is positive down.
            let angle = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
            let weights: Vec<f32> = menu.items.iter().map(|item| item.weight).collect();
            crate::radial::index_at(&crate::radial::for_menu(boxes, &weights), angle)?
        }
        MenuKind::Directional { .. } => {
            if central {
                return None;
            }
            // Slots go up, right, down, left.
            match (dx.abs() > dy.abs(), dx > 0.0, dy > 0.0) {
                (true, true, _) => 1,
                (true, false, _) => 3,
                (false, _, true) => 2,
                (false, _, false) => 0,
            }
        }
        MenuKind::Grid { columns } => {
            let columns = usize::from(columns).clamp(1, n);
            strip(y, n.div_ceil(columns)) * columns + strip(x, columns)
        }
        MenuKind::List | MenuKind::Buttons => {
            let columns = list_columns(n);
            let rows = n.div_ceil(columns);
            strip(x, columns) * rows + strip(y, rows)
        }
        MenuKind::Carousel { .. } => strip(x, n),
    };
    (i < n).then_some(i)
}

/// Where an item lies on the area a menu covers, for pictures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemShape {
    /// A rectangle: left, top, right and bottom, each 0..1 across the area.
    Cell(f32, f32, f32, f32),
    /// A slice of the circle round the middle, between two angles (degrees clockwise from
    /// straight up), outside the middle that picks nothing.
    Slice(f32, f32),
}

/// The radius of the middle that picks nothing in a radial or directional menu, in widths of
/// the area.
pub const TOUCH_CENTER_RADIUS: f32 = CENTER_RADIUS;

/// Where each of `menu`'s items lies, as [`item_at`] finds them.
pub fn item_shapes(menu: &Menu) -> Vec<ItemShape> {
    let n = menu.items.len();
    if n == 0 {
        return Vec::new();
    }
    let share = |i: usize, count: usize| (i as f32 / count as f32, (i + 1) as f32 / count as f32);
    match menu.kind {
        MenuKind::Radial { boxes, .. } => {
            let weights: Vec<f32> = menu.items.iter().map(|item| item.weight).collect();
            crate::radial::for_menu(boxes, &weights).into_iter().map(|(a, b)| ItemShape::Slice(a, b)).collect()
        }
        MenuKind::Directional { .. } => (0..n.min(4)).map(|i| ItemShape::Slice(i as f32 * 90.0 - 45.0, i as f32 * 90.0 + 45.0)).collect(),
        MenuKind::Grid { columns } => {
            let columns = usize::from(columns).clamp(1, n);
            let rows = n.div_ceil(columns);
            (0..n)
                .map(|i| {
                    let ((x0, x1), (y0, y1)) = (share(i % columns, columns), share(i / columns, rows));
                    ItemShape::Cell(x0, y0, x1, y1)
                })
                .collect()
        }
        MenuKind::List | MenuKind::Buttons => {
            let columns = list_columns(n);
            let rows = n.div_ceil(columns);
            (0..n)
                .map(|i| {
                    let ((x0, x1), (y0, y1)) = (share(i / rows, columns), share(i % rows, rows));
                    ItemShape::Cell(x0, y0, x1, y1)
                })
                .collect()
        }
        MenuKind::Carousel { .. } => (0..n)
            .map(|i| {
                let (x0, x1) = share(i, n);
                ItemShape::Cell(x0, 0.0, x1, 1.0)
            })
            .collect(),
    }
}

/// What the overlay draws for a menu on the touchpad, with `selected` under the finger. `picks`
/// counts the items chosen so far, so the overlay can show each pick.
pub fn touch_view(menu: &Menu, selected: Option<usize>, choose: TouchChoose, picks: u32) -> MenuView {
    let hint = match choose {
        TouchChoose::Click => "Slide to pick · click the pad to choose",
        TouchChoose::Lift => "Slide to pick · lift to choose",
    };
    MenuView {
        title: menu.name.clone(),
        kind: menu.kind,
        items: item_views(menu),
        selected,
        picks,
        crumbs: Vec::new(),
        hint: hint.into(),
        style: menu.style.clone(),
        touch: true,
        aim: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ButtonAction, Cluster, MenuItem, OverlayStyle, Stick};

    fn menu(kind: MenuKind, n: usize) -> Menu {
        let items = (0..n).map(|i| MenuItem { label: i.to_string(), action: ButtonAction::Disabled, button: None, weight: 1.0 }).collect();
        Menu { name: "M".into(), kind, items, cancel: None, style: OverlayStyle::default() }
    }

    #[test]
    fn a_radial_menu_goes_clockwise_from_the_top_and_its_middle_picks_nothing() {
        let m = menu(MenuKind::Radial { stick: Stick::Right, boxes: false }, 4);
        assert_eq!(item_at(&m, (0.5, 0.0), 0.5), Some(0));
        assert_eq!(item_at(&m, (1.0, 0.5), 0.5), Some(1));
        assert_eq!(item_at(&m, (0.5, 1.0), 0.5), Some(2));
        assert_eq!(item_at(&m, (0.0, 0.5), 0.5), Some(3));
        assert_eq!(item_at(&m, (0.52, 0.52), 0.5), None);
    }

    #[test]
    fn a_grid_is_divided_into_its_cells_and_a_short_last_row_leaves_a_gap() {
        let m = menu(MenuKind::Grid { columns: 2 }, 3);
        assert_eq!(item_at(&m, (0.1, 0.1), 0.5), Some(0));
        assert_eq!(item_at(&m, (0.9, 0.1), 0.5), Some(1));
        assert_eq!(item_at(&m, (0.1, 0.9), 0.5), Some(2));
        assert_eq!(item_at(&m, (0.9, 0.9), 0.5), None);
        // The edges belong to the cells next to them.
        assert_eq!(item_at(&m, (1.0, 0.0), 0.5), Some(1));
    }

    #[test]
    fn a_list_goes_by_rows_and_a_long_one_by_two_columns() {
        let short = menu(MenuKind::List, 4);
        assert_eq!(item_at(&short, (0.9, 0.0), 0.5), Some(0));
        assert_eq!(item_at(&short, (0.1, 0.6), 0.5), Some(2));
        // Ten rows: the left column has 0..5, the right 5..10.
        let long = menu(MenuKind::List, 10);
        assert_eq!(item_at(&long, (0.2, 0.99), 0.5), Some(4));
        assert_eq!(item_at(&long, (0.8, 0.0), 0.5), Some(5));
    }

    #[test]
    fn each_shape_holds_its_own_item() {
        let kinds = [
            (MenuKind::Radial { stick: Stick::Right, boxes: false }, 5),
            (MenuKind::Directional { cluster: Cluster::FaceButtons }, 4),
            (MenuKind::Grid { columns: 3 }, 7),
            (MenuKind::List, 11),
            (MenuKind::Buttons, 3),
            (MenuKind::Carousel { controls: crate::config::CarouselControls::Bumpers }, 4),
        ];
        let height = 0.6;
        for (kind, n) in kinds {
            let m = menu(kind, n);
            let shapes = item_shapes(&m);
            assert_eq!(shapes.len(), n, "{kind:?}");
            for (i, shape) in shapes.into_iter().enumerate() {
                let middle = match shape {
                    ItemShape::Cell(x0, y0, x1, y1) => ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
                    ItemShape::Slice(a, b) => {
                        let mid = ((a + b) / 2.0).to_radians();
                        (0.5 + 0.25 * mid.sin(), 0.5 - 0.25 * mid.cos() / height)
                    }
                };
                assert_eq!(item_at(&m, middle, height), Some(i), "{kind:?} item {i}");
            }
        }
    }

    #[test]
    fn directions_and_carousels() {
        let d = menu(MenuKind::Directional { cluster: Cluster::DPad }, 4);
        assert_eq!(item_at(&d, (0.5, 0.05), 0.5), Some(0));
        assert_eq!(item_at(&d, (0.95, 0.5), 0.5), Some(1));
        assert_eq!(item_at(&d, (0.5, 0.95), 0.5), Some(2));
        assert_eq!(item_at(&d, (0.05, 0.5), 0.5), Some(3));
        assert_eq!(item_at(&menu(MenuKind::Directional { cluster: Cluster::DPad }, 2), (0.5, 0.95), 0.5), None, "no third item");
        let c = menu(MenuKind::Carousel { controls: crate::config::CarouselControls::Bumpers }, 5);
        assert_eq!(item_at(&c, (0.5, 0.0), 0.5), Some(2));
        assert_eq!(item_at(&menu(MenuKind::List, 0), (0.5, 0.5), 0.5), None);
    }
}
