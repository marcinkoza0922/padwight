use tiny_skia::{Mask, Pixmap, Transform};

use super::*;
use crate::font;

mod compare;

fn spec(text: &str, size: f32) -> Spec {
    Spec { family: Some("Inter"), ..Spec::new(text, size) }
}

fn shaped_count() -> usize {
    engine::with(|e| e.shaped)
}

/// Draws a run on a clear pixmap of `w` × `h` device pixels at `scale`, its box at `at`.
fn drawn(spec: &Spec, at: Point, scale: f32, (w, h): (u32, u32), mask: Option<&Mask>) -> (Shaped, Pixmap) {
    let shaped = shape(spec);
    let mut pixmap = Pixmap::new(w, h).unwrap();
    let mut canvas = Canvas { pixmap: pixmap.as_mut(), transform: Transform::from_scale(scale, scale), mask };
    draw(shaped.run, at, Rgba::WHITE, &mut canvas);
    (shaped, pixmap)
}

/// The device pixels with any ink, as a box: left, top, right, bottom (exclusive).
fn inked(pixmap: &Pixmap) -> Option<(u32, u32, u32, u32)> {
    let w = pixmap.width();
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (i, px) in pixmap.pixels().iter().enumerate() {
        if px.alpha() > 0 {
            let (x, y) = (i as u32 % w, i as u32 / w);
            found = Some(found.map_or((x, y, x + 1, y + 1), |(l, t, r, b)| (l.min(x), t.min(y), r.max(x + 1), b.max(y + 1))));
        }
    }
    found
}

#[test]
fn shaping_the_same_run_twice_hits_the_cache() {
    let one = spec("Shaped once, found after", 17.0);
    let before = shaped_count();
    let first = shape(&one);
    assert_eq!(shaped_count(), before + 1);
    let second = shape(&one.clone());
    assert_eq!(shaped_count(), before + 1, "the second time is found, not shaped");
    assert_eq!(first, second);
    // Drawing it, in any color, shapes nothing either.
    let mut pixmap = Pixmap::new(300, 40).unwrap();
    for color in [Rgba::WHITE, Rgba::BLACK.scale_alpha(0.5)] {
        let mut canvas = Canvas { pixmap: pixmap.as_mut(), transform: Transform::identity(), mask: None };
        draw(first.run, Point::new(2.0, 2.0), color, &mut canvas);
    }
    assert_eq!(shaped_count(), before + 1);
    assert!(engine::with(|e| e.holds(first.run)));
}

#[test]
fn the_key_covers_everything_that_shapes_a_run() {
    let base = spec("Key", 14.0);
    let changes: [fn(&mut Spec); 10] = [
        |s| s.text.push('s'),
        |s| s.family = Some("Kreon"),
        |s| s.size = 15.0,
        |s| s.bold = true,
        |s| s.italic = true,
        |s| s.underline = true,
        |s| s.case = Case::Upper,
        |s| s.spacing = 1.0,
        |s| s.width = Some(14.0),
        |s| s.align = Align::End,
    ];
    let mut keys = vec![base.key()];
    for change in changes {
        let mut other = base.clone();
        change(&mut other);
        keys.push(other.key());
    }
    let unique: std::collections::HashSet<_> = keys.iter().collect();
    assert_eq!(unique.len(), keys.len(), "{keys:?}");
    assert_eq!(Spec { size: -0.0, ..base.clone() }.key(), Spec { size: 0.0, ..base }.key());
}

#[test]
fn lines_are_1_3_times_the_size_and_wrap_between_words() {
    let one = shape(&spec("one line", 20.0));
    assert!((one.size.y - 26.0).abs() < 1e-3, "{:?}", one.size);
    let two = shape(&spec("first\nsecond", 20.0));
    assert!((two.size.y - 52.0).abs() < 1e-3, "{:?}", two.size);
    let long = "words that wrap at a width";
    let unwrapped = shape(&spec(long, 16.0));
    let wrapped = shape(&Spec { width: Some(unwrapped.size.x / 2.0), ..spec(long, 16.0) });
    assert!(wrapped.size.y >= 2.0 * 16.0 * LINE_HEIGHT - 1e-3, "{:?}", wrapped.size);
    assert!(wrapped.size.x <= unwrapped.size.x / 2.0);
}

#[test]
fn case_bold_and_spacing_change_the_width() {
    let plain = shape(&spec("Ab cd", 16.0)).size.x;
    assert!(shape(&Spec { case: Case::Upper, ..spec("Ab cd", 16.0) }).size.x > plain);
    assert!(shape(&Spec { bold: true, family: None, ..spec("Ab cd", 16.0) }).size.x >= shape(&Spec { family: None, ..spec("Ab cd", 16.0) }).size.x);
    // Spaced as iced spaces it: between the letters of a word and after each space, so three
    // gaps here.
    let spaced = shape(&Spec { spacing: 3.0, ..spec("Ab cd", 16.0) }).size.x;
    assert!((spaced - plain - 3.0 * 3.0).abs() < 0.01, "{plain} then {spaced}");
}

#[test]
fn lines_line_up_inside_the_width() {
    let at = |align| {
        let (shaped, pixmap) = drawn(&Spec { width: Some(200.0), align, ..spec("Mid", 16.0) }, Point::ZERO, 1.0, (200, 30), None);
        assert_eq!(shaped.size.x, if align == Align::Start { shaped.size.x } else { 200.0 });
        inked(&pixmap).unwrap()
    };
    let (start, center, end) = (at(Align::Start), at(Align::Center), at(Align::End));
    assert!(start.0 <= 2, "{start:?}");
    assert!(end.2 >= 197, "{end:?}");
    let middle = (center.0 + center.2) as f32 / 2.0;
    assert!((middle - 100.0).abs() <= 2.0, "{center:?}");
}

#[test]
fn the_ink_holds_every_glyph_at_any_scale() {
    let tall = Spec { italic: true, underline: true, ..spec("Jÿq|ƒ gj, 0123 ÅÉ", 23.0) };
    for (s, family) in [(tall.clone(), Some("Inter")), (tall, Some("Nosifer")), (spec("{Creepy}", 30.0), Some("Creepster"))] {
        for scale in [1.0, 1.5, 2.0] {
            let s = Spec { family, ..s.clone() };
            let at = Point::new(13.3, 7.6);
            let (shaped, pixmap) = drawn(&s, at, scale, (900, 200), None);
            let (l, t, r, b) = inked(&pixmap).expect("it draws");
            let ink = shaped.ink.shifted(at);
            let device = |v: f32| v * scale;
            assert!(device(ink.x) <= l as f32 && device(ink.y) <= t as f32, "{family:?} at {scale}: {ink:?} vs {l},{t}");
            assert!(device(ink.right()) >= r as f32 && device(ink.bottom()) >= b as f32, "{family:?} at {scale}: {ink:?} vs {r},{b}");
        }
    }
    let blank = shape(&spec("   ", 16.0));
    assert!(blank.ink.is_empty() && blank.size.y > 0.0);
}

#[test]
fn the_canvas_mask_cuts_the_glyphs() {
    let mut mask = Mask::new(200, 40).unwrap();
    let (_, open) = drawn(&spec("Masked", 20.0), Point::new(4.0, 4.0), 1.0, (200, 40), None);
    assert!(inked(&open).is_some());
    let (_, shut) = drawn(&spec("Masked", 20.0), Point::new(4.0, 4.0), 1.0, (200, 40), Some(&mask));
    assert!(inked(&shut).is_none(), "an empty mask lets nothing through");
    mask.data_mut()[..200 * 20].fill(255);
    let (_, half) = drawn(&spec("Masked", 20.0), Point::new(4.0, 4.0), 1.0, (200, 40), Some(&mask));
    assert!(inked(&half).is_some_and(|(_, _, _, b)| b <= 20));
}

#[test]
fn a_font_loaded_later_reshapes_the_runs_shaped_before() {
    let s = Spec { family: Some("Kreon"), ..Spec::new("Reshaped", 18.0) };
    shape(&s);
    let before = shaped_count();
    shape(&s);
    assert_eq!(shaped_count(), before);
    let mut odd = font::BUNDLED.iter().find(|b| b.name == "Kreon").unwrap().bytes.to_vec();
    odd.extend_from_slice(b"later");
    font::load(&odd).unwrap();
    shape(&s);
    assert_eq!(shaped_count(), before + 1, "shaped again with the new fonts");
}

#[test]
fn tiny_or_broken_sizes_draw_nothing() {
    for size in [0.0, -3.0, f32::NAN, f32::INFINITY] {
        let (shaped, pixmap) = drawn(&spec("x", size), Point::ZERO, 1.0, (20, 20), None);
        assert!(shaped.ink.is_empty() && inked(&pixmap).is_none(), "{size}");
    }
}
