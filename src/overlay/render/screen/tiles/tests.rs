//! Tiles: see `tiles.rs`.

use tiny_skia::Pixmap;

use super::*;
use crate::{
    debug::scenes,
    overlay::render::{
        Point, bounds,
        screen::{Shown, screen_ops},
    },
};

/// The inner ops of the first panel of `scene`.
fn panel(scene: &str) -> Vec<Op> {
    let frame = scenes::frame(scene).unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), Point::new(1920.0, 1080.0));
    ops[1..ops.len() - 1].to_vec()
}


/// The grid of a picture covering `ops` at `scale`.
fn grid_of(ops: &[Op], scale: f32) -> Grid {
    let ink = bounds(ops).unwrap();
    let origin = ((ink.x * scale).floor() as i32 - 1, (ink.y * scale).floor() as i32 - 1);
    let size = ((ink.right() * scale).ceil() as i32 + 1 - origin.0, (ink.bottom() * scale).ceil() as i32 + 1 - origin.1);
    Grid { origin, size, scale }
}

/// `ops` drawn in all of `grid`'s tiles, over `under` when given.
fn tiled(ops: &[Op], grid: Grid, under: Option<&Pixmap>) -> Pixmap {
    let mut pixmap = Pixmap::new(grid.size.0 as u32, grid.size.1 as u32).unwrap();
    draw(&Marked::new(ops), grid, &vec![true; grid.count()], &mut pixmap, under);
    pixmap
}

#[test]
fn a_picture_drawn_in_tiles_is_its_list_drawn_whole() {
    for scale in [1.0, 1.5] {
        for scene in ["menu-list", "menu-radial", "menu-styled", "keyboard-plaid", "info-formatted", "edit-gyro"] {
            let ops = panel(scene);
            let grid = grid_of(&ops, scale);
            let pixmap = tiled(&ops, grid, None);
            let mut whole = Pixmap::new(pixmap.width(), pixmap.height()).unwrap();
            let at = Transform::from_row(scale, 0.0, 0.0, scale, -grid.origin.0 as f32, -grid.origin.1 as f32);
            raster::draw(&ops, &mut whole.as_mut(), at);
            let worst = pixmap.data().iter().zip(whole.data()).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
            assert!(worst <= 1, "{scene} at {scale}: {worst}");
        }
    }
}

#[test]
fn a_picture_drawn_on_from_its_list_drawn_part_way_is_the_whole_list() {
    let ops = panel("menu-list");
    let grid = grid_of(&ops, 1.0);
    let whole = tiled(&ops, grid, None);
    // Cut inside the panel's clip: the ops after the cut are drawn inside it again.
    let upto = ops.iter().rposition(|op| matches!(op, Op::Text { .. })).unwrap() - 3;
    assert!(ops[..upto].iter().any(|op| matches!(op, Op::Clip { .. })));
    let under = tiled(&ops[..upto], grid, None);
    let drawn_on = tiled(&super::super::under::rest(&ops, upto), grid, Some(&under));
    assert_eq!(drawn_on.data(), whole.data());
}

#[test]
#[ignore]
#[expect(clippy::print_stdout, reason = "a benchmark reports its timing")]
fn bench_kinds() {
    for scene in ["menu-radial", "menu-list"] {
        let all = panel(scene);
        for kind in ["", "Shadow", "Rect", "Text", "Path", "Pattern", "Svg"] {
            let ops: Vec<Op> = all
                .iter()
                .filter(|op| matches!(op, Op::Clip { .. } | Op::Unclip | Op::Layer(_) | Op::Unlayer) || format!("{op:?}").starts_with(kind))
                .cloned()
                .collect();
            if bounds(&ops).is_none() {
                continue;
            }
            let grid = grid_of(&ops, 1.0);
            let list = Marked::new(&ops);
            let mut pixmap = Pixmap::new(grid.size.0 as u32, grid.size.1 as u32).unwrap();
            let which = vec![true; grid.count()];
            draw(&list, grid, &which, &mut pixmap, None);
            let start = std::time::Instant::now();
            for _ in 0..20 {
                draw(&list, grid, &which, &mut pixmap, None);
            }
            println!("{scene} {kind:8} {} ops: {:.2} ms a picture", ops.len(), start.elapsed().as_secs_f64() * 1000.0 / 20.0);
        }
    }
}
