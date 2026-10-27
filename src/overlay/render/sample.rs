//! Test support: draws a node the way a panel is drawn and saves the picture under
//! `screenshots/render/`, to look at (`raster/tests.rs` saves its pictures there too).

use std::path::PathBuf;

use tiny_skia::Pixmap;

use super::{
    Corners, Node, Op, Outline, Point, Rect, Rgba, Shadow,
    raster::picture,
    text::Spec,
};

/// `node` placed at the origin and drawn at `width` and `scale`, on a clear background as big as
/// its ink (its shadow included).
pub fn draw(node: &Node, width: f32, scale: f32) -> Pixmap {
    let (_, ops) = node.ops(Point::ZERO, width);
    picture(&ops, scale).expect("the node has ink").pixmap
}

/// Writes `pixmap` to `screenshots/render/<name>.png` and returns the path.
pub fn save(name: &str, pixmap: &Pixmap) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("screenshots/render");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.png"));
    pixmap.save_png(&path).unwrap();
    path
}

/// A rounded, shadowed, bordered box with nothing in it but what is drawn under it.
fn card() -> Node {
    Node::leaf(|rect: Rect, ops: &mut Vec<Op>| {
        let radii = Corners::all(12.0);
        let shadow = Shadow { color: Rgba { a: 0.6, ..Rgba::BLACK }, offset: (0.0, 4.0), blur: 10.0 };
        ops.push(Op::Shadow { rect, radii, shadow, spread: 0.0 });
        let border = Some(Outline { width: 1.0, color: Rgba::rgb(0.4, 0.45, 0.55) });
        ops.push(Op::Rect { rect, radii, fill: Some(Rgba::rgb(0.14, 0.15, 0.18).into()), border });
    })
}

/// A card with a title on it, and a line of text under the card.
fn panel() -> Node {
    let text = Rgba::rgb(0.9, 0.9, 0.92);
    let titled = Node::stack(card().width(220.0).height(70.0)).over(Node::column([Node::text(Spec::new("Panel", 18.0), text)]).padding(14.0));
    Node::column([titled, Node::text(Spec::new("A line of text under the card", 14.0), text)]).spacing(8.0).padding(10.0)
}

#[test]
fn a_panel_draws_and_saves_its_picture() {
    let pixmap = draw(&panel(), 240.0, 2.0);
    let path = save("sample", &pixmap);
    assert!(path.exists());
    assert!(pixmap.pixels().iter().any(|p| p.alpha() > 0), "the panel draws something");
}
