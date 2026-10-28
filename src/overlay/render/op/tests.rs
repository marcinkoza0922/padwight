use std::{
    f32::consts::PI,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use crate::overlay::{
    art,
    render::{
        Cap, Corners, Fill, File, Fit, Join, Layer, Linear, Markup, Op, Outline, Path, PatternRef, Point, Rect, Rgba, Rule, RunKey, Seg, Shadow, Stop, Stroke, Svg,
        balanced, bounds, digest,
    },
};

fn hash_of(v: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    v.hash(&mut hasher);
    hasher.finish()
}

fn wash(rgb: [u8; 3]) -> PatternRef {
    PatternRef::new(art::wash(&art::Tint { rgb, opacity: 0.5 }))
}

/// A panel's list with one op of every kind, each made afresh: nothing is shared between two
/// calls, so equal lists are equal by their contents.
fn panel(item: Rgba) -> Vec<Op> {
    let rect = Rect::new(0.0, 0.0, 200.0, 120.0);
    let radii = Corners::all(16.0);
    let shadow = Shadow { color: Rgba { a: 0.4, ..Rgba::BLACK }, offset: (0.0, 4.0), blur: 12.0 };
    let gradient = Linear::new(PI).stop(0.0, Rgba::WHITE).stop(1.0, item);
    let mut arc = Path::new();
    arc.arc(Point::new(100.0, 60.0), 40.0, 0.0, 90.0).arc(Point::new(100.0, 60.0), 20.0, 90.0, 0.0).close();
    let file: File = Arc::from(std::path::Path::new("backgrounds/crest.png"));
    vec![
        Op::Shadow { rect, radii, shadow, spread: 0.0 },
        Op::Rect { rect, radii, fill: Some(Fill::Linear(gradient)), border: Some(Outline { width: 1.0, color: Rgba::WHITE.scale_alpha(0.15) }) },
        Op::Clip { rect, radii },
        Op::Picture { file, rect, fit: Fit::Cover, opacity: 0.8 },
        Op::Pattern { pattern: wash([1, 2, 3]), rect: Rect::new(10.0, 10.0, 80.0, 30.0), origin: Point::ZERO, cut: art::Shape::Rounded(8.0), opacity: 0.5 },
        Op::Layer(Layer { opacity: 0.5, scale: 1.1, about: Point::new(50.0, 25.0), shift: Point::new(4.0, 0.0) }),
        Op::Rect { rect: Rect::new(10.0, 10.0, 80.0, 30.0), radii: Corners::all(8.0), fill: Some(item.into()), border: None },
        Op::Text { run: RunKey::of(&("Jump", 16_u32)), at: Point::new(20.0, 14.0), color: Rgba::WHITE, ink: Rect::new(19.0, 14.0, 40.0, 21.0) },
        Op::Unlayer,
        Op::Path { path: arc, fill: Some(item.into()), rule: Rule::NonZero, stroke: Some(Stroke { cap: Cap::Round, join: Join::Round, ..Stroke::new(2.0, Rgba::BLACK) }) },
        Op::Svg { svg: Svg::Markup(Markup::new(String::from("<svg/>"))), rect: Rect::new(170.0, 0.0, 30.0, 30.0), fit: Fit::Contain, opacity: 1.0 },
        Op::Unclip,
    ]
}

#[test]
fn equal_lists_compare_and_hash_equal() {
    let item = Rgba::from_rgb8(0x30, 0x34, 0x3c);
    let (a, b) = (panel(item), panel(item));
    assert!(balanced(&a));
    assert_eq!(a, b);
    assert_eq!(digest(&a), digest(&b));
    // -0 is 0, in the hash as in the comparison.
    let moved = |x: f32| vec![Op::Clip { rect: Rect::new(x, 0.0, 1.0, 1.0), radii: Corners::ZERO }, Op::Unclip];
    assert_eq!(moved(0.0), moved(-0.0));
    assert_eq!(digest(&moved(0.0)), digest(&moved(-0.0)));
}

#[test]
fn a_changed_color_or_number_differs() {
    let item = Rgba::from_rgb8(0x30, 0x34, 0x3c);
    let base = panel(item);
    let other = panel(Rgba { a: 0.99, ..item });
    assert_ne!(base, other);
    assert_ne!(digest(&base), digest(&other));
    // Each op on its own: a text's color, a pattern's key, an SVG's markup, a layer's scale.
    let text = |color: Rgba| Op::Text { run: RunKey(7), at: Point::ZERO, color, ink: Rect::default() };
    assert_ne!(hash_of(&text(Rgba::WHITE)), hash_of(&text(Rgba::BLACK)));
    let pattern = |p: PatternRef| Op::Pattern { pattern: p, rect: Rect::default(), origin: Point::ZERO, cut: art::Shape::Rounded(0.0), opacity: 1.0 };
    assert_eq!(pattern(wash([1, 2, 3])), pattern(wash([1, 2, 3])));
    assert_ne!(pattern(wash([1, 2, 3])), pattern(wash([1, 2, 4])));
    let svg = |s: &str| Svg::Markup(Markup::new(s));
    assert_ne!(svg("<svg/>"), svg("<svg />"));
    assert_ne!(hash_of(&svg("<svg/>")), hash_of(&svg("<svg />")));
    assert_ne!(Op::Layer(Layer::faded(0.5)), Op::Layer(Layer { scale: 1.01, ..Layer::faded(0.5) }));
    assert_ne!(Op::Unclip, Op::Unlayer);
    assert_ne!(hash_of(&Op::Unclip), hash_of(&Op::Unlayer));
}

#[test]
fn bounds_include_shadows() {
    let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
    let shadow = |offset, blur, spread| Op::Shadow { rect, radii: Corners::all(8.0), shadow: Shadow { color: Rgba::BLACK, offset, blur }, spread };
    // Blur reaches out on every side, spread grows the box first, and the offset moves it.
    assert_eq!(shadow((0.0, 0.0), 12.0, 0.0).bounds(), Some(Rect::new(-2.0, -2.0, 124.0, 74.0)));
    assert_eq!(shadow((6.0, -4.0), 12.0, 3.0).bounds(), Some(Rect::new(1.0, -9.0, 130.0, 80.0)));
    let invisible = Op::Shadow { rect, radii: Corners::ZERO, shadow: Shadow::default(), spread: 0.0 };
    assert_eq!(invisible.bounds(), None);
    // A list's bounds hold its box and the shadow under it.
    let list = [shadow((0.0, 8.0), 20.0, 0.0), Op::Rect { rect, radii: Corners::all(8.0), fill: Some(Rgba::WHITE.into()), border: None }];
    assert_eq!(bounds(&list), Some(Rect::new(-10.0, -2.0, 140.0, 90.0)));
}

#[test]
fn bounds_follow_clips_and_layers() {
    let item = |rect| Op::Rect { rect, radii: Corners::ZERO, fill: Some(Rgba::WHITE.into()), border: None };
    let clipped = [Op::Clip { rect: Rect::new(0.0, 0.0, 50.0, 50.0), radii: Corners::all(10.0) }, item(Rect::new(40.0, 40.0, 100.0, 100.0)), Op::Unclip, item(Rect::new(0.0, 0.0, 1.0, 1.0))];
    assert_eq!(bounds(&clipped), Some(Rect::new(0.0, 0.0, 50.0, 50.0)));
    // Twice the size about (10, 10), then 5 right.
    let zoom = Layer { opacity: 0.3, scale: 2.0, about: Point::new(10.0, 10.0), shift: Point::new(5.0, 0.0) };
    let layered = [Op::Layer(zoom), Op::Clip { rect: Rect::new(10.0, 10.0, 10.0, 10.0), radii: Corners::ZERO }, item(Rect::new(0.0, 0.0, 30.0, 30.0)), Op::Unclip, Op::Unlayer];
    assert!(balanced(&layered));
    assert_eq!(bounds(&layered), Some(Rect::new(15.0, 10.0, 20.0, 20.0)));
    assert_eq!(bounds(&[Op::Layer(Layer::IDENTITY), Op::Unlayer]), None);
    assert!(!balanced(&[Op::Layer(Layer::IDENTITY), Op::Unclip]));
    assert!(!balanced(&[Op::Clip { rect: Rect::default(), radii: Corners::ZERO }]));
}

#[test]
fn paths_and_strokes_reach_their_bounds() {
    let mut ring = Path::new();
    ring.circle(Point::new(50.0, 50.0), 20.0);
    assert!(matches!(ring.segs(), [Seg::MoveTo(_), Seg::CubicTo(..), Seg::CubicTo(..), Seg::CubicTo(..), Seg::CubicTo(..), Seg::Close]));
    let b = ring.bounds().unwrap();
    // The handles reach a little past the circle; the curve ends on it.
    assert!((b.x - 30.0).abs() < 0.01 && (b.w - 40.0).abs() < 0.01, "{b:?}");
    let Some(&Seg::CubicTo(_, _, end)) = ring.segs().get(1) else { unreachable!() };
    assert!((end.x - 70.0).abs() < 1e-4 && (end.y - 50.0).abs() < 1e-4, "{end:?}");
    let stroked = Op::Path { path: ring, fill: None, rule: Rule::EvenOdd, stroke: Some(Stroke { join: Join::Bevel, ..Stroke::new(4.0, Rgba::WHITE) }) };
    assert_eq!(stroked.bounds(), Some(b.outset(2.0)));
}

#[test]
fn gradients_run_as_iced_runs_them() {
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let close = |(a, b): (Point, Point), (c, d): (Point, Point)| [a.x - c.x, a.y - c.y, b.x - d.x, b.y - d.y].iter().all(|v| v.abs() < 1e-4);
    // π runs top to bottom, π/2 left to right.
    assert!(close(Linear::new(PI).points(rect), (Point::new(50.0, 0.0), Point::new(50.0, 50.0))));
    assert!(close(Linear::new(PI / 2.0).points(rect), (Point::new(0.0, 25.0), Point::new(100.0, 25.0))));
    let stops = Linear::new(0.0).stop(0.25, Rgba::BLACK).stops;
    assert_eq!(stops, vec![Stop { offset: 0.25, color: Rgba::BLACK }]);
}
