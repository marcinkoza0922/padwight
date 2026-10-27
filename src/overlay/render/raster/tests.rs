//! Each op drawn small and compared with a picture kept in `golden/` (within a level, for
//! rounding between machines), and the sums behind them checked pixel by pixel. Run with
//! `PADWIGHT_BLESS=1` to write the pictures afresh; a picture that differs is written to
//! `screenshots/render/` beside its difference, to look at.

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4},
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};

use tiny_skia::{Pixmap, Transform};

use super::{draw, forget, picture, pictures::kept_counts};
use crate::{
    config::{Checkerboard, FilterOn},
    overlay::{
        art,
        render::{Cap, Corners, Fill, File, Fit, Join, Layer, Linear, Markup, Op, Outline, Path, PatternRef, Point, Rect, Rgba, Rule, Shadow, Stroke, Svg},
    },
    snapshot::Image,
};

mod parity;

const BACKGROUND: [u8; 3] = [0x20, 0x22, 0x26];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `ops` drawn at `scale` on a `w` by `h` (logical) picture of the background.
fn drawn(ops: &[Op], w: u32, h: u32, scale: f32) -> Image {
    let (pw, ph) = ((w as f32 * scale).round() as u32, (h as f32 * scale).round() as u32);
    let mut pixmap = Pixmap::new(pw, ph).unwrap();
    let [r, g, b] = BACKGROUND;
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    draw(ops, &mut pixmap.as_mut(), Transform::from_scale(scale, scale));
    straight(&pixmap)
}

/// The pixmap's pixels, no longer premultiplied.
fn straight(pixmap: &Pixmap) -> Image {
    let rgba = pixmap.pixels().iter().flat_map(|p| {
        let c = p.demultiply();
        [c.red(), c.green(), c.blue(), c.alpha()]
    });
    Image { width: pixmap.width(), height: pixmap.height(), rgba: rgba.collect() }
}

/// The largest difference in any channel of any pixel.
pub(super) fn worst(a: &Image, b: &Image) -> u8 {
    assert_eq!((a.width, a.height), (b.width, b.height), "the pictures are the same size");
    a.rgba.iter().zip(&b.rgba).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

/// Where each pixel differs, white for the most, over black.
fn difference(a: &Image, b: &Image) -> Image {
    let rgba = a.rgba.as_chunks::<4>().0.iter().zip(b.rgba.as_chunks::<4>().0).flat_map(|(p, q)| {
        let d = (0..4).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0).saturating_mul(32);
        [d, d, d, 255]
    });
    Image { width: a.width, height: a.height, rgba: rgba.collect() }
}

/// Checks `image` against `golden/<name>.png`, within a level.
fn golden(name: &str, image: &Image) {
    let path = root().join(format!("src/overlay/render/raster/golden/{name}.png"));
    let out = root().join("screenshots/render");
    if std::env::var_os("PADWIGHT_BLESS").is_some() {
        image.save(&path).unwrap();
        image.save(&out.join(format!("{name}.png"))).unwrap();
        return;
    }
    let Ok(kept) = image::open(&path) else {
        image.save(&out.join(format!("{name}.png"))).unwrap();
        panic!("no picture at {}: run with PADWIGHT_BLESS=1 to write it", path.display());
    };
    let kept = kept.to_rgba8();
    let kept = Image { width: kept.width(), height: kept.height(), rgba: kept.into_raw() };
    let off = worst(image, &kept);
    if off > 1 {
        image.save(&out.join(format!("{name}.png"))).unwrap();
        difference(image, &kept).save(&out.join(format!("{name}-difference.png"))).unwrap();
        panic!("{name} differs from its picture by {off} levels; see screenshots/render/");
    }
}

fn rgb(image: &Image, x: u32, y: u32) -> [u8; 3] {
    let [r, g, b, _] = image.pixel(x, y);
    [r, g, b]
}

/// `c` laid over the background at alpha `a`, as bytes.
fn over(c: Rgba, a: f32) -> [u8; 3] {
    let [r, g, b] = BACKGROUND.map(f32::from);
    let mix = |x: f32, y: f32| (x + (y * 255.0 - x) * a).round() as u8;
    [mix(r, c.r), mix(g, c.g), mix(b, c.b)]
}

fn near(a: [u8; 3], b: [u8; 3], within: u8) -> bool {
    (0..3).all(|i| a[i].abs_diff(b[i]) <= within)
}

fn solid(rect: Rect, radii: f32, color: Rgba) -> Op {
    Op::Rect { rect, radii: Corners::all(radii), fill: Some(Fill::Solid(color)), border: None }
}

const RED: Rgba = Rgba::rgb(0.9, 0.2, 0.2);
const BLUE: Rgba = Rgba::rgb(0.2, 0.3, 0.9);
const CREAM: Rgba = Rgba::rgb(0.95, 0.92, 0.8);

#[test]
fn boxes_fill_and_draw_their_borders_inside() {
    let line = |width: f32, color: Rgba| Some(Outline { width, color });
    let ops = vec![
        Op::Rect { rect: Rect::new(4.0, 4.0, 24.0, 16.0), radii: Corners::all(6.0), fill: Some(BLUE.into()), border: line(2.0, CREAM) },
        // Corners tighter than half the border: square there, cut to the box's own.
        Op::Rect {
            rect: Rect::new(36.0, 4.0, 24.0, 16.0),
            radii: Corners { top_left: 0.0, top_right: 4.0, bottom_right: 1.0, bottom_left: 8.0 },
            fill: Some(BLUE.into()),
            border: line(3.0, CREAM),
        },
        // Round all over: a circle.
        Op::Rect { rect: Rect::new(4.0, 26.0, 16.0, 16.0), radii: Corners::all(8.0), fill: Some(RED.into()), border: line(1.0, CREAM.scale_alpha(0.5)) },
        // A border wider than half the box covers it.
        Op::Rect { rect: Rect::new(26.0, 26.0, 34.0, 18.0), radii: Corners::all(30.0), fill: None, border: line(20.0, RED) },
    ];
    let image = drawn(&ops, 64, 48, 1.0);
    golden("boxes", &image);
    golden("boxes@2", &drawn(&ops, 64, 48, 2.0));
    assert_eq!(rgb(&image, 16, 12), over(BLUE, 1.0), "the middle is the fill");
    assert_eq!(rgb(&image, 4, 12), over(CREAM, 1.0), "the border lies inside the box");
    assert_eq!(rgb(&image, 3, 12), BACKGROUND, "and not outside it");
    assert_eq!(rgb(&image, 43, 35), over(RED, 1.0), "the widest border fills its box");
}

#[test]
fn gradients_run_along_iced_angles() {
    let across = Linear::new(FRAC_PI_2).stop(0.0, RED).stop(1.0, BLUE);
    let slanted = Linear::new(FRAC_PI_4).stop(0.0, CREAM).stop(0.5, RED).stop(1.0, BLUE);
    let mut wedge = Path::new();
    wedge.move_to(Point::new(40.0, 34.0)).line_to(Point::new(62.0, 34.0)).line_to(Point::new(51.0, 62.0)).close();
    let ops = vec![
        Op::Rect { rect: Rect::new(0.0, 0.0, 64.0, 16.0), radii: Corners::ZERO, fill: Some(Fill::Linear(across)), border: None },
        Op::Rect { rect: Rect::new(2.0, 18.0, 36.0, 44.0), radii: Corners::all(8.0), fill: Some(Fill::Linear(slanted)), border: None },
        // Bottom to top, over the path's own box.
        Op::Path { path: wedge, fill: Some(Fill::Linear(Linear::new(0.0).stop(0.0, BLUE).stop(1.0, CREAM))), rule: Rule::NonZero, stroke: None },
    ];
    let image = drawn(&ops, 64, 64, 1.0);
    golden("gradients", &image);
    assert!(near(rgb(&image, 0, 8), over(RED, 1.0), 4), "left is the first stop");
    assert!(near(rgb(&image, 63, 8), over(BLUE, 1.0), 4), "right is the last");
    let reds: Vec<u8> = (0..64).map(|x| rgb(&image, x, 8)[0]).collect();
    assert!(reds.is_sorted_by(|a, b| a >= b), "and it runs evenly between: {reds:?}");
}

#[test]
fn shadows_soften_grow_and_stay_hard_with_no_blur() {
    let shadow = |blur: f32, a: f32| Shadow { color: Rgba { a, ..Rgba::BLACK }, offset: (2.0, 3.0), blur };
    let hard = Rgba { a: 0.8, ..RED };
    let ops = vec![
        Op::Shadow { rect: Rect::new(10.0, 10.0, 24.0, 24.0), radii: Corners::all(6.0), shadow: shadow(6.0, 0.9), spread: 0.0 },
        solid(Rect::new(10.0, 10.0, 24.0, 24.0), 6.0, CREAM.scale_alpha(0.5)),
        Op::Shadow { rect: Rect::new(52.0, 14.0, 20.0, 20.0), radii: Corners::all(4.0), shadow: Shadow { color: hard, offset: (0.0, 0.0), blur: 0.0 }, spread: 4.0 },
    ];
    let image = drawn(&ops, 80, 48, 1.0);
    golden("shadows", &image);
    golden("shadows@2", &drawn(&ops, 80, 48, 2.0));
    // No blur: full strength out to the grown box, nothing past it.
    assert!(near(rgb(&image, 49, 24), over(hard, 0.8), 1));
    assert_eq!(rgb(&image, 47, 24), BACKGROUND);
    assert!(near(rgb(&image, 75, 24), over(hard, 0.8), 1));
    assert_eq!(rgb(&image, 76, 24), BACKGROUND);
}

#[test]
fn paths_fill_by_their_rule_and_stroke_with_caps_and_joins() {
    let star = |cx: f32| {
        let mut p = Path::new();
        for i in 0..5 {
            let a = (i as f32 * 144.0).to_radians();
            let at = Point::new(cx + 14.0 * a.sin(), 18.0 - 14.0 * a.cos());
            if i == 0 { p.move_to(at) } else { p.line_to(at) };
        }
        p.close();
        p
    };
    let zigzag = |y: f32| {
        let mut p = Path::new();
        p.move_to(Point::new(8.0, y)).line_to(Point::new(24.0, y - 8.0)).line_to(Point::new(40.0, y)).line_to(Point::new(56.0, y - 8.0));
        p
    };
    let line = |cap, join| Some(Stroke { width: 4.0, color: CREAM, cap, join });
    let mut ring = Path::new();
    ring.circle(Point::new(52.0, 52.0), 9.0).circle(Point::new(52.0, 52.0), 5.0);
    let ops = vec![
        Op::Path { path: star(16.0), fill: Some(RED.into()), rule: Rule::EvenOdd, stroke: None },
        Op::Path { path: star(48.0), fill: Some(RED.into()), rule: Rule::NonZero, stroke: Some(Stroke::new(1.0, CREAM)) },
        Op::Path { path: zigzag(46.0), fill: None, rule: Rule::NonZero, stroke: line(Cap::Butt, Join::Miter) },
        Op::Path { path: zigzag(62.0), fill: None, rule: Rule::NonZero, stroke: line(Cap::Round, Join::Round) },
        Op::Path { path: zigzag(78.0), fill: None, rule: Rule::NonZero, stroke: line(Cap::Square, Join::Bevel) },
        Op::Path { path: ring, fill: Some(BLUE.into()), rule: Rule::EvenOdd, stroke: None },
    ];
    let image = drawn(&ops, 64, 84, 1.0);
    golden("paths", &image);
    assert_eq!(rgb(&image, 16, 19), BACKGROUND, "even-odd leaves the middle of a star empty");
    assert_eq!(rgb(&image, 48, 19), over(RED, 1.0), "non-zero fills it");
    assert_eq!(rgb(&image, 52, 52), BACKGROUND, "and the ring's hole");
}

#[test]
fn clips_cut_round_nest_and_move_with_their_layer() {
    let ops = vec![
        Op::Clip { rect: Rect::new(4.0, 4.0, 40.0, 32.0), radii: Corners::all(10.0) },
        Op::Clip { rect: Rect::new(20.0, 0.0, 40.0, 40.0), radii: Corners::ZERO },
        solid(Rect::new(0.0, 0.0, 64.0, 40.0), 0.0, RED),
        Op::Unclip,
        solid(Rect::new(0.0, 0.0, 16.0, 40.0), 0.0, BLUE),
        Op::Unclip,
        // Halved about the corner and moved right: the clip comes along.
        Op::Layer(Layer { opacity: 1.0, scale: 0.5, about: Point::ZERO, shift: Point::new(48.0, 0.0) }),
        Op::Clip { rect: Rect::new(0.0, 0.0, 32.0, 32.0), radii: Corners::all(16.0) },
        solid(Rect::new(0.0, 0.0, 64.0, 64.0), 0.0, CREAM),
        Op::Unclip,
        Op::Unlayer,
    ];
    let image = drawn(&ops, 72, 40, 1.0);
    golden("clips", &image);
    assert_eq!(rgb(&image, 30, 20), over(RED, 1.0), "inside both clips");
    assert_eq!(rgb(&image, 18, 20), BACKGROUND, "inside the outer clip only");
    assert_eq!(rgb(&image, 10, 20), over(BLUE, 1.0), "the outer clip on its own");
    assert_eq!(rgb(&image, 5, 5), BACKGROUND, "cut round at the corner");
    assert_eq!(rgb(&image, 56, 8), over(CREAM, 1.0), "the circle, halved and moved");
    assert_eq!(rgb(&image, 49, 1), BACKGROUND, "its corner cut round too");
}

#[test]
fn a_layer_fades_as_one_picture() {
    let inner = [solid(Rect::new(4.0, 4.0, 20.0, 20.0), 0.0, CREAM), solid(Rect::new(14.0, 4.0, 20.0, 20.0), 0.0, RED)];
    let mut ops = vec![Op::Layer(Layer::faded(0.5))];
    ops.extend(inner.iter().cloned());
    ops.extend([Op::Unlayer, Op::Layer(Layer::faded(0.5)), Op::Layer(Layer::faded(0.5))]);
    ops.push(solid(Rect::new(40.0, 4.0, 10.0, 20.0), 0.0, RED));
    ops.extend([Op::Unlayer, Op::Unlayer]);
    let image = drawn(&ops, 56, 28, 1.0);
    golden("layers", &image);
    assert!(near(rgb(&image, 18, 14), over(RED, 0.5), 1), "the red over the cream shows only the red");
    assert!(near(rgb(&image, 30, 14), over(RED, 0.5), 1));
    assert!(near(rgb(&image, 8, 14), over(CREAM, 0.5), 1));
    assert!(near(rgb(&image, 45, 14), over(RED, 0.25), 2), "nested layers multiply");
}

#[test]
fn a_layer_scales_about_its_point_and_moves() {
    let layer = Layer { opacity: 1.0, scale: 2.0, about: Point::new(10.0, 10.0), shift: Point::new(5.0, 0.0) };
    let rect = Rect::new(10.0, 10.0, 10.0, 10.0);
    let ops = vec![Op::Layer(layer), solid(rect, 0.0, RED), Op::Unlayer];
    let image = drawn(&ops, 48, 40, 1.0);
    let lands = layer.map(rect);
    assert_eq!(lands, Rect::new(15.0, 10.0, 20.0, 20.0));
    assert_eq!(rgb(&image, 15, 10), over(RED, 1.0));
    assert_eq!(rgb(&image, 34, 29), over(RED, 1.0));
    assert_eq!(rgb(&image, 14, 10), BACKGROUND);
    assert_eq!(rgb(&image, 35, 29), BACKGROUND);
    // Zoomed part way at a fractional scale, edges and corners stay smooth.
    let zoom = Layer { opacity: 0.9, scale: 1.3, about: Point::new(24.0, 20.0), shift: Point::new(0.5, 0.0) };
    let ops = vec![Op::Layer(zoom), solid(Rect::new(12.0, 10.0, 24.0, 20.0), 6.0, BLUE), Op::Unlayer];
    golden("zoom@1.5", &drawn(&ops, 48, 40, 1.5));
}

fn checkers() -> PatternRef {
    let board = Checkerboard { colors: ["#e8d8b0".into(), "#30343c".into()], opacity: 1.0, size: 4.0, on: FilterOn::default() };
    PatternRef::new(art::checkerboard(&board, 1.0))
}

#[test]
fn patterns_run_on_across_the_parts_they_are_cut_to() {
    let part = |rect: Rect, cut| Op::Pattern { pattern: checkers(), rect, origin: Point::new(4.0, 4.0), cut, opacity: 1.0 };
    let whole = drawn(&[part(Rect::new(4.0, 4.0, 40.0, 20.0), art::Shape::Rounded(0.0))], 48, 28, 1.0);
    let halves = drawn(&[part(Rect::new(4.0, 4.0, 18.0, 20.0), art::Shape::Rounded(0.0)), part(Rect::new(22.0, 4.0, 22.0, 20.0), art::Shape::Rounded(0.0))], 48, 28, 1.0);
    assert_eq!(worst(&whole, &halves), 0, "two parts are the picture unbroken");
    let ops = vec![
        part(Rect::new(4.0, 4.0, 18.0, 20.0), art::Shape::Rounded(6.0)),
        Op::Pattern { pattern: PatternRef::new(art::wash(&art::Tint { rgb: [200, 40, 40], opacity: 0.5 })), rect: Rect::new(26.0, 4.0, 18.0, 20.0), origin: Point::new(4.0, 4.0), cut: art::Shape::Rounded(6.0), opacity: 0.5 },
    ];
    golden("patterns", &drawn(&ops, 48, 28, 1.0));
    golden("patterns@2", &drawn(&ops, 48, 28, 2.0));
}

/// Red on the left half, blue on the right, twice as wide as it is tall.
fn halves_svg() -> Svg {
    Svg::Markup(Markup::new(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="10" height="10" fill="#e63333"/><rect x="10" width="10" height="10" fill="#334ce6"/></svg>"##))
}

#[test]
fn svgs_fit_their_box_and_are_kept_by_markup_and_size() {
    forget();
    let red = [0xe6, 0x33, 0x33];
    let blue = [0x33, 0x4c, 0xe6];
    let svg = |x: f32, fit| Op::Svg { svg: halves_svg(), rect: Rect::new(x, 4.0, 40.0, 40.0), fit, opacity: 1.0 };
    let ops = [svg(4.0, Fit::Contain), svg(48.0, Fit::Cover), svg(92.0, Fit::Stretch)];
    let image = drawn(&ops, 136, 48, 1.0);
    golden("svgs", &image);
    // Contain: a band 20 tall in the middle.
    assert_eq!((rgb(&image, 10, 13), rgb(&image, 10, 15), rgb(&image, 40, 33), rgb(&image, 40, 34)), (BACKGROUND, red, blue, BACKGROUND));
    // Cover: 80 wide, its middle 40 shown, half and half.
    assert_eq!((rgb(&image, 48, 4), rgb(&image, 67, 43), rgb(&image, 68, 4), rgb(&image, 87, 43), rgb(&image, 88, 20)), (red, red, blue, blue, BACKGROUND));
    // Stretch: the whole box.
    assert_eq!((rgb(&image, 92, 4), rgb(&image, 131, 43)), (red, blue));
    assert_eq!(kept_counts().0, 3, "one picture for each size it is drawn at");
    drawn(&ops, 136, 48, 1.0);
    assert_eq!(kept_counts().0, 3, "drawn again, nothing new is made");
    drawn(&ops, 136, 48, 2.0);
    assert_eq!(kept_counts().0, 5, "at another scale, each is drawn at that size (contain at 2 is cover at 1)");
}

#[test]
fn pictures_fit_their_box_and_are_kept_by_path() {
    forget();
    // A 4 by 2 picture: red on the left, blue on the right.
    let path = root().join("screenshots/render/halves.png");
    let px = |c: [u8; 3]| [c[0], c[1], c[2], 255];
    let (red, blue) = ([0xe6, 0x33, 0x33], [0x33, 0x4c, 0xe6]);
    let row = [px(red), px(red), px(blue), px(blue)].concat();
    Image { width: 4, height: 2, rgba: [row.clone(), row].concat() }.save(&path).unwrap();
    let file: File = Arc::from(FsPath::new(&path));
    let at = |x: f32, fit| Op::Picture { file: file.clone(), rect: Rect::new(x, 4.0, 40.0, 40.0), fit, opacity: 1.0 };
    let image = drawn(&[at(4.0, Fit::Contain), at(48.0, Fit::Cover), at(92.0, Fit::Stretch)], 136, 48, 1.0);
    assert_eq!((rgb(&image, 10, 12), rgb(&image, 10, 18), rgb(&image, 38, 30), rgb(&image, 38, 36)), (BACKGROUND, red, blue, BACKGROUND));
    assert_eq!((rgb(&image, 49, 20), rgb(&image, 86, 20), rgb(&image, 88, 20)), (red, blue, BACKGROUND));
    assert_eq!((rgb(&image, 93, 5), rgb(&image, 130, 42)), (red, blue));
    let faded = drawn(&[Op::Picture { file, rect: Rect::new(4.0, 4.0, 40.0, 40.0), fit: Fit::Stretch, opacity: 0.5 }], 48, 48, 1.0);
    assert!(near(rgb(&faded, 6, 20), over(Rgba::from_rgb8(red[0], red[1], red[2]), 0.5), 1));
    let missing: File = Arc::from(FsPath::new("no/such/picture.png"));
    let blank = drawn(&[Op::Picture { file: missing, rect: Rect::new(0.0, 0.0, 8.0, 8.0), fit: Fit::Cover, opacity: 1.0 }], 8, 8, 1.0);
    assert_eq!(blank.drawn(BACKGROUND), 0, "a picture that won't load draws nothing");
}

#[test]
fn a_gradient_box_is_the_same_wherever_the_pixmap_lies() {
    let fill = Fill::Linear(Linear::new(std::f32::consts::PI).stop(0.0, RED).stop(1.0, BLUE));
    let at = |x: f32| [Op::Rect { rect: Rect::new(x + 4.0, 4.0, 40.0, 20.0), radii: Corners::all(6.0), fill: Some(fill.clone()), border: None }];
    let mut here = Pixmap::new(48, 28).unwrap();
    draw(&at(0.0), &mut here.as_mut(), Transform::identity());
    let mut there = Pixmap::new(48, 28).unwrap();
    draw(&at(700.0), &mut there.as_mut(), Transform::from_translate(-700.0, 0.0));
    let worst = here.data().iter().zip(there.data()).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    assert!(worst <= 1, "{worst}");
    assert!(here.pixel(24, 14).unwrap().alpha() > 200);
}

#[test]
fn a_picture_is_as_big_as_its_ink() {
    let ops = [solid(Rect::new(-3.0, 2.0, 10.0, 4.5), 0.0, RED)];
    let pic = picture(&ops, 2.0).unwrap();
    assert_eq!((pic.origin, pic.pixmap.width(), pic.pixmap.height()), ((-6, 4), 20, 9));
    assert_eq!(pic.pixmap.pixel(0, 0).map(tiny_skia::PremultipliedColorU8::alpha), Some(255));
    assert!(picture(&[Op::Unclip], 1.0).is_none(), "nothing to draw, no picture");
}
