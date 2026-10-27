//! Diffing lists: see `diff.rs`.

use super::*;
use crate::overlay::render::{Corners, Fill, Layer, Rgba};

fn rect(x: f32, color: f32) -> Op {
    Op::Rect { rect: Rect::new(x, 0.0, 10.0, 10.0), radii: Corners::ZERO, fill: Some(Fill::Solid(Rgba::rgb(color, 0.0, 0.0))), border: None }
}

fn changed_of(old: &[Op], new: &[Op]) -> Vec<Rect> {
    changed(&Marked::new(old), &Marked::new(new)).unwrap().0
}

#[test]
fn the_same_list_changes_nothing() {
    let ops = [rect(0.0, 0.1), rect(20.0, 0.2)];
    assert!(changed_of(&ops, &ops).is_empty());
}

#[test]
fn an_op_changed_reaches_where_it_was_and_is() {
    let old = [rect(0.0, 0.1), rect(20.0, 0.2), rect(40.0, 0.3)];
    let mut new = old.clone();
    new[1] = rect(20.0, 0.9);
    let reach = changed_of(&old, &new);
    assert_eq!(reach, vec![Rect::new(20.0, 0.0, 10.0, 10.0), Rect::new(20.0, 0.0, 10.0, 10.0)]);
}

#[test]
fn an_op_added_or_taken_away_reaches_only_where_it_is() {
    let old = [rect(0.0, 0.1), rect(40.0, 0.3)];
    let new = [rect(0.0, 0.1), rect(20.0, 0.2), rect(40.0, 0.3)];
    assert_eq!(changed_of(&old, &new), vec![Rect::new(20.0, 0.0, 10.0, 10.0)]);
    assert_eq!(changed_of(&new, &old), vec![Rect::new(20.0, 0.0, 10.0, 10.0)]);
}

#[test]
fn a_layer_taken_away_changes_its_whole_group_both_ways() {
    let shifted = Layer { shift: crate::overlay::render::Point::new(100.0, 0.0), ..Layer::IDENTITY };
    let old = [rect(0.0, 0.1), Op::Layer(shifted), rect(20.0, 0.2), Op::Unlayer];
    let new = [rect(0.0, 0.1), rect(20.0, 0.2)];
    let reach = changed_of(&old, &new);
    // Where the box was, moved by its layer, and where it is now.
    assert!(reach.contains(&Rect::new(120.0, 0.0, 10.0, 10.0)), "{reach:?}");
    assert!(reach.contains(&Rect::new(20.0, 0.0, 10.0, 10.0)), "{reach:?}");
    assert!(!reach.contains(&Rect::new(0.0, 0.0, 10.0, 10.0)), "{reach:?}");
}

#[test]
fn a_clip_cuts_what_its_ops_reach() {
    let clip = Op::Clip { rect: Rect::new(0.0, 0.0, 25.0, 10.0), radii: Corners::ZERO };
    let old = [clip.clone(), rect(20.0, 0.2), Op::Unclip];
    let new = [clip, rect(20.0, 0.5), Op::Unclip];
    assert_eq!(changed_of(&old, &new), vec![Rect::new(20.0, 0.0, 5.0, 10.0), Rect::new(20.0, 0.0, 5.0, 10.0)]);
}

#[test]
fn lists_too_far_apart_give_up() {
    let old: Vec<Op> = (0..400).map(|i| rect(i as f32, 0.1)).collect();
    let new: Vec<Op> = (0..400).map(|i| rect(i as f32, 0.2)).collect();
    assert!(changed(&Marked::new(&old), &Marked::new(&new)).is_none());
}
