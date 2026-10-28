//! A mouse region's rows in Edit Controls, shared by the stick and touchpad pages: its shape,
//! where its middle is on the desktop, how big it is, and two ready-made places.

use crate::config::{Config, MouseRegion, RegionShape};

use super::rows::{Act, Number, Row, cycle, set};

const ACROSS: Number = Number::new(5.0, (0.0, 100.0), 0, "% from the left");
const DOWN: Number = Number::new(5.0, (0.0, 100.0), 0, "% from the top");
const SIZE: Number = Number::new(5.0, (5.0, 100.0), 0, "% of the desktop");

/// The rows of `region`, which `change` finds in the config and changes, saying whether it did.
pub(super) fn region_rows<F>(region: MouseRegion, change: F) -> Vec<Row>
where
    F: Fn(&mut Config, &mut dyn FnMut(&mut MouseRegion) -> bool) -> bool + Clone + 'static,
{
    let field = |label: &str, value: f32, n: &Number, pick: fn(&mut MouseRegion) -> &mut f32| {
        let change = change.clone();
        n.row(label, value * 100.0, move |c, v| change(c, &mut |r| set(pick(r), v / 100.0)))
    };
    let preset = |label: &str, to: MouseRegion| {
        let change = change.clone();
        Row::apply(label, move |c| Act::changed_if(change(c, &mut |r| {
            let to = r.placed_as(to);
            set(r, to)
        })))
    };
    let shape = {
        let change = change.clone();
        Row::step(format!("Shape: {}", region.shape), move |c, dir| {
            change(c, &mut |r| {
                let next = cycle(&RegionShape::ALL, r.shape, dir);
                set(&mut r.shape, next)
            })
        })
    };
    vec![
        shape,
        field("X offset", region.x, &ACROSS, |r| &mut r.x),
        field("Y offset", region.y, &DOWN, |r| &mut r.y),
        field("Width", region.width, &SIZE, |r| &mut r.width),
        field("Height", region.height, &SIZE, |r| &mut r.height),
        preset("Fill the whole desktop", MouseRegion::FULL),
        preset("Middle half of the desktop", MouseRegion::default()),
    ]
}
