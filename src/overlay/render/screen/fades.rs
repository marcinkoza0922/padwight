//! Panel pictures: a fade or slide draws each panel once and only lays it down again, and a
//! panel fades as one picture, every part of it by the same amount.

use tiny_skia::Pixmap;

use super::*;
use crate::{
    debug::scenes,
    overlay::render::{Layer, bounds, raster},
};

const DISPLAY: Point = Point::new(1920.0, 1080.0);

/// The scenes the fade checks run on: every kind of panel, info and log included, a question
/// over a menu, the Guide's row, and several at once.
const SCENES: &[&str] = &[
    "menu-list", "menu-styled", "menu-metal", "menu-radial", "menu-radial-boxes", "keyboard", "numpad", "media", "offer",
    "info", "info-formatted", "log", "unsaved", "guide", "busy",
];

/// Everything in `frame` held `opacity` of the way into a linear fade-in.
fn faint(frame: &OverlayFrame, opacity: f32) -> Shown<'_> {
    let anim = Anim::faint(opacity);
    Shown {
        panels: frame.active.iter().map(|v| (v, anim)).collect(),
        info: frame.info.iter().map(|v| (v, anim)).collect(),
        logs: frame.logs.iter().map(|v| (v, anim)).collect(),
    }
}

/// `ops` with every panel (each layer at the top level) changed by `change`.
fn moved(ops: &[Op], change: impl Fn(&mut Layer)) -> Vec<Op> {
    let mut depth = 0;
    let mut ops = ops.to_vec();
    for op in &mut ops {
        match op {
            Op::Layer(layer) if depth == 0 => {
                change(layer);
                depth += 1;
            }
            Op::Layer(_) | Op::Clip { .. } => depth += 1,
            Op::Unlayer | Op::Unclip => depth -= 1,
            _ => {}
        }
    }
    ops
}

fn laid(screen: &mut Screen, ops: &[Op]) -> Pixmap {
    let mut pixmap = Pixmap::new(DISPLAY.x as u32, DISPLAY.y as u32).unwrap();
    screen.lay(ops, 1.0, &mut pixmap.as_mut());
    pixmap
}

/// How many panels `ops` holds: its layers at the top level.
fn panels(ops: &[Op]) -> u64 {
    let (mut n, mut depth) = (0, 0);
    for op in ops {
        match op {
            Op::Layer(_) | Op::Clip { .. } => {
                n += u64::from(depth == 0);
                depth += 1;
            }
            Op::Unlayer | Op::Unclip => depth -= 1,
            _ => {}
        }
    }
    n
}

#[test]
fn a_fade_slide_and_zoom_redraw_nothing_but_the_composite() {
    for name in SCENES {
        let frame = scenes::frame(name).unwrap();
        let mut screen = Screen::new();
        let mut target = Pixmap::new(DISPLAY.x as u32, DISPLAY.y as u32).unwrap();
        let mut first = None;
        for (step, opacity) in [0.25, 0.6, 1.0].into_iter().enumerate() {
            target.fill(tiny_skia::Color::TRANSPARENT);
            screen.draw(&frame, &faint(&frame, opacity), Display { size: DISPLAY, scale: 1.0 }, &mut target.as_mut());
            let drawn = *first.get_or_insert(screen.drawn());
            assert_eq!(screen.drawn(), drawn, "{name}: step {step} drew a picture again");
        }
        let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
        assert_eq!(first, Some(panels(&ops)), "{name}: one picture per panel");
        // Sliding and zooming every panel of a few (it's slow in an unoptimized build).
        let steps = if ["menu-list", "keyboard", "info", "log", "guide"].contains(name) { 0..3 } else { 0..0 };
        for step in steps {
            let t = step as f32 * 2.0;
            laid(&mut screen, &moved(&ops, |l| (l.shift, l.opacity) = (Point::new(t * 3.3, -t * 7.1), 0.2 + t / 8.0)));
            laid(&mut screen, &moved(&ops, |l| (l.scale, l.about) = (0.8 + t * 0.05, Point::new(900.0, 500.0))));
        }
        assert_eq!(Some(screen.drawn()), first, "{name}: moving and zooming drew a picture again");
    }
}

/// Each panel of `ops` alone: its layer at the top level, opened and closed.
fn each_panel(ops: &[Op]) -> Vec<&[Op]> {
    let (mut panels, mut depth, mut start) = (Vec::new(), 0, 0);
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Layer(_) | Op::Clip { .. } => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            Op::Unlayer | Op::Unclip => {
                depth -= 1;
                if depth == 0 {
                    panels.push(&ops[start..=i]);
                }
            }
            _ => {}
        }
    }
    panels
}

/// The bytes of each row of `p` that `area` covers, a pixel more all round.
fn rows(p: &Pixmap, area: Rect) -> impl Iterator<Item = &[u8]> {
    let (w, h) = (p.width() as usize, p.height() as usize);
    let x0 = (area.x.floor() as usize).saturating_sub(1).min(w);
    let x1 = ((area.right().ceil() as usize) + 1).clamp(x0, w);
    let y0 = (area.y.floor() as usize).saturating_sub(1).min(h);
    let y1 = ((area.bottom().ceil() as usize) + 1).clamp(y0, h);
    (y0..y1).map(move |y| &p.data()[4 * (y * w + x0)..4 * (y * w + x1)])
}

#[test]
fn every_panel_fades_as_one_picture_in_even_steps() {
    for name in SCENES {
        let frame = scenes::frame(name).unwrap();
        let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
        let mut screen = Screen::new();
        for (n, panel) in each_panel(&ops).into_iter().enumerate() {
            let area = bounds(panel).unwrap();
            let ink = |p: &Pixmap| rows(p, area).flat_map(|r| r.iter().skip(3).step_by(4)).map(|a| *a as f64).sum::<f64>();
            let full = laid(&mut screen, panel);
            let whole = ink(&full);
            assert!(whole > 0.0, "{name} {n} draws something");
            for opacity in [0.1, 0.3, 0.5, 0.7, 0.9] {
                let faded = laid(&mut screen, &moved(panel, |l| l.opacity = opacity));
                // The panel's ink is linear in its opacity...
                let share = ink(&faded) / whole;
                assert!((share - opacity as f64).abs() < 0.005, "{name} {n} at {opacity}: {share}");
                // ...and so is every pixel's, color too: no outline or box shows ahead of its label.
                for (f, s) in rows(&faded, area).zip(rows(&full, area)).flat_map(|(f, s)| f.iter().zip(s)) {
                    let want = *s as f32 * opacity;
                    assert!((*f as f32 - want).abs() <= 1.0, "{name} {n} at {opacity}: {f} for {want}");
                }
            }
        }
    }
}

#[test]
fn a_settled_screen_is_the_list_drawn_straight() {
    for name in ["menu-list", "guide", "busy", "unsaved"] {
        let frame = scenes::frame(name).unwrap();
        let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
        let mut straight = Pixmap::new(DISPLAY.x as u32, DISPLAY.y as u32).unwrap();
        raster::draw(&ops, &mut straight.as_mut(), tiny_skia::Transform::identity());
        let pictured = laid(&mut Screen::new(), &ops);
        let worst = pictured.data().iter().zip(straight.data()).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(worst <= 1, "{name}: {worst}");
    }
}

#[test]
fn a_slide_moves_the_picture_by_whole_pixels() {
    let frame = scenes::frame("menu-list").unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    let mut screen = Screen::new();
    let rest = laid(&mut screen, &ops);
    let slid = laid(&mut screen, &moved(&ops, |l| l.shift = Point::new(3.4, -1.6)));
    // 3.4 right and 1.6 up land on 3 and 2 whole pixels: the very same pixels, moved.
    let at = |p: &Pixmap, x: u32, y: u32| p.pixel(x, y).unwrap();
    for y in 300..780 {
        for x in 600..1320 {
            assert_eq!(at(&slid, x + 3, y - 2), at(&rest, x, y), "({x}, {y})");
        }
    }
}

#[test]
fn a_zoom_shrinks_the_picture_about_its_point() {
    let frame = scenes::frame("menu-list").unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    let mut screen = Screen::new();
    let ink = |p: &Pixmap| p.pixels().iter().map(|c| c.alpha() as f64).sum::<f64>();
    let rest = ink(&laid(&mut screen, &ops));
    let half = laid(&mut screen, &moved(&ops, |l| l.scale = 0.5));
    // Filtered, a quarter of the area keeps a quarter of the ink: no rim lost or darkened.
    assert!((ink(&half) / rest - 0.25).abs() < 0.01, "{}", ink(&half) / rest);
    assert_eq!(screen.drawn(), 1);
}

#[test]
fn a_few_pictures_are_kept_and_the_oldest_dropped() {
    let mut frame = scenes::frame("menu-list").unwrap();
    let mut screen = Screen::new();
    let mut target = Pixmap::new(DISPLAY.x as u32, DISPLAY.y as u32).unwrap();
    let items = match &frame.active {
        Some(OverlayView::Menu(m)) => m.items.len(),
        _ => unreachable!(),
    };
    assert!(items >= 4);
    let mut select = |frame: &mut OverlayFrame, screen: &mut Screen, i: usize| {
        if let Some(OverlayView::Menu(m)) = &mut frame.active {
            m.selected = Some(i);
        }
        screen.draw(frame, &Shown::settled(frame), Display { size: DISPLAY, scale: 1.0 }, &mut target.as_mut());
    };
    // The cursor moving back and forth over a few rows reuses their pictures.
    for i in [0, 1, 2, 3, 2, 1, 0, 1, 2, 3] {
        select(&mut frame, &mut screen, i);
    }
    assert_eq!(screen.drawn(), 4);
    assert!(screen.kept() <= 4);
    // A new scale is a new picture.
    screen.draw(&frame, &Shown::settled(&frame), Display { size: DISPLAY, scale: 1.5 }, &mut target.as_mut());
    assert_eq!(screen.drawn(), 5);
    screen.forget();
    assert!(screen.kept() == 0);
}