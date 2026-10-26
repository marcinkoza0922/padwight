//! Text against iced's: the sizes layout measures, and a page of every bundled font drawn by
//! both, to look at side by side.

use iced::{
    Alignment, Color, Element, Font, Pixels, Size,
    advanced::{
        graphics::text::Paragraph,
        text::{self as itext, Paragraph as _},
    },
    alignment::Vertical,
    font::{Style, Weight},
    widget::{column, container, rich_text, row, space, span, text},
};

use super::*;
use crate::snapshot::Image;

fn iced_font(s: &Spec) -> Font {
    let font = s.family.map_or(Font::DEFAULT, Font::with_name);
    Font { weight: if s.bold { Weight::Bold } else { Weight::Normal }, style: if s.italic { Style::Italic } else { Style::Normal }, ..font }
}

fn load_iced_fonts() {
    for bundled in &font::BUNDLED {
        font::load(bundled.bytes).unwrap();
    }
}

/// What iced measures a run as, for text with no letter spacing.
fn iced_size(s: &Spec) -> Point {
    let content = s.case.apply(&s.text);
    let paragraph = Paragraph::with_text(itext::Text {
        content: content.as_str(),
        bounds: Size::new(s.width.unwrap_or(f32::INFINITY), f32::INFINITY),
        size: Pixels(s.size),
        line_height: itext::LineHeight::default(),
        font: iced_font(s),
        align_x: itext::Alignment::Default,
        align_y: Vertical::Top,
        shaping: itext::Shaping::Auto,
        wrapping: itext::Wrapping::Word,
    });
    let size = paragraph.min_bounds();
    Point::new(size.width, size.height)
}

fn families() -> impl Iterator<Item = Option<&'static str>> {
    std::iter::once(None).chain(font::BUNDLED.iter().map(|b| Some(b.name)))
}

#[test]
fn every_font_measures_as_iced_measures_it() {
    load_iced_fonts();
    for family in families() {
        for (text, size, width) in [("The quick brown fox, 0123", 18.0, None), ("Ünïcödé — “quotes” …", 13.0, None), ("a line that wraps between its words", 15.0, Some(90.0))] {
            for (bold, italic) in [(false, false), (true, false), (false, true)] {
                let s = Spec { family, bold, italic, width, ..Spec::new(text, size) };
                let (ours, iced) = (shape(&s).size, iced_size(&s));
                assert!((ours.x - iced.x).abs() < 0.01 && (ours.y - iced.y).abs() < 0.01, "{s:?}: {ours:?}, iced {iced:?}");
            }
        }
    }
}

/// Rows are this far apart, and the page has this margin, in logical pixels.
const PITCH: f32 = 30.0;
const MARGIN: f32 = 10.0;
/// Wide enough that no row wraps (Press Start 2P's is the widest).
const WIDTH: f32 = 900.0;
const BACKGROUND: [u8; 3] = [0x20, 0x20, 0x20];
const TINT: Rgba = Rgba { r: 1.0, g: 0.8, b: 0.3, a: 0.6 };

fn color(s: &Spec) -> Rgba {
    if s.spacing > 0.0 { TINT } else { Rgba::WHITE }
}

/// Four rows for each font: plain, bold, italic and underlined, then spaced capitals (drawn in
/// `TINT`, see-through).
fn page() -> Vec<Spec> {
    families()
        .flat_map(|family| {
            let name = family.unwrap_or("Default");
            [
                Spec { family, ..Spec::new(format!("{name}: The quick brown fox jumps, 0123"), 18.0) },
                Spec { family, bold: true, ..Spec::new("Bold weight AaBbGgQq 4567", 14.0) },
                Spec { family, italic: true, underline: true, ..Spec::new("Italic and underlined, gjpqy", 14.0) },
                Spec { family, case: Case::Upper, spacing: 2.0, ..Spec::new("spaced capitals here", 12.0) },
            ]
        })
        .collect()
}

/// A row as the iced overlay sets it (`draw/typeset.rs`): spaced text a letter at a time.
fn iced_row(s: &Spec) -> Element<'static, ()> {
    let font = iced_font(s);
    let (size, underline) = (s.size, s.underline);
    let Rgba { r, g, b, a } = color(s);
    let ink = Color { r, g, b, a };
    let piece = move |t: String| -> Element<'static, ()> {
        if underline {
            rich_text([span::<(), Font>(t).font(font).size(size).color(ink).underline(true)]).into()
        } else {
            text(t).font(font).size(size).color(ink).into()
        }
    };
    let t = s.case.apply(&s.text);
    if s.spacing <= 0.0 {
        return piece(t);
    }
    let mut parts: Vec<Element<'static, ()>> = Vec::new();
    for (i, word) in t.split(' ').enumerate() {
        if i > 0 {
            parts.push(piece(" ".into()));
            parts.push(space().width(s.spacing).into());
        }
        parts.push(row(word.chars().map(|c| piece(c.to_string()))).spacing(s.spacing).align_y(Alignment::Center).into());
    }
    row(parts).wrap().into()
}

fn page_size(rows: usize) -> Size {
    Size::new(WIDTH, rows as f32 * PITCH + 2.0 * MARGIN)
}

fn ours(rows: &[Spec], scale: f32) -> Image {
    let size = page_size(rows.len());
    let (w, h) = ((size.width * scale).round() as u32, (size.height * scale).round() as u32);
    let mut pixmap = Pixmap::new(w, h).unwrap();
    let [r, g, b] = BACKGROUND;
    pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
    for (i, s) in rows.iter().enumerate() {
        let shaped = shape(s);
        let mut canvas = Canvas { pixmap: pixmap.as_mut(), transform: Transform::from_scale(scale, scale), mask: None };
        draw(shaped.run, Point::new(MARGIN, MARGIN + i as f32 * PITCH), color(s), &mut canvas);
    }
    Image { width: w, height: h, rgba: pixmap.take() }
}

fn iced_page(rows: &[Spec], scale: f32) -> Image {
    let page = column(rows.iter().map(|s| container(iced_row(s)).height(PITCH).into())).padding(MARGIN);
    let [r, g, b] = BACKGROUND;
    crate::snapshot::render(page.into(), page_size(rows.len()), scale, Color::from_rgb8(r, g, b)).unwrap()
}

/// Each pixel's largest channel difference, four times over, in gray.
fn difference(a: &Image, b: &Image) -> Image {
    let rgba = a.rgba.chunks(4).zip(b.rgba.chunks(4)).flat_map(|(p, q)| {
        let d = (0..3).map(|c| p[c].abs_diff(q[c])).max().unwrap_or(0).saturating_mul(4);
        [d, d, d, 255]
    });
    Image { width: a.width, height: a.height, rgba: rgba.collect() }
}

/// How much each row differs: the summed channel differences in its band, per device pixel row.
fn row_differences(a: &Image, b: &Image, rows: &[Spec], scale: f32) -> Vec<(f32, String)> {
    let band = |i: usize| ((MARGIN + i as f32 * PITCH) * scale) as u32..((MARGIN + (i + 1) as f32 * PITCH) * scale) as u32;
    rows.iter()
        .enumerate()
        .map(|(i, s)| {
            let total: u64 = band(i)
                .flat_map(|y| (0..a.width).map(move |x| (x, y)))
                .map(|(x, y)| (0..3).map(|c| u64::from(a.pixel(x, y)[c].abs_diff(b.pixel(x, y)[c]))).sum::<u64>())
                .sum();
            (total as f32 / (PITCH * scale), format!("{:?} {}", s.family, s.text))
        })
        .collect()
}

/// Draws the page with both renderers into `screenshots/render/`, to compare by eye, and prints
/// the rows that differ most.
#[test]
#[expect(clippy::print_stderr, reason = "a report to read")]
#[ignore = "writes PNGs to look at; run with --run-ignored only"]
fn a_page_of_every_font_drawn_by_both() {
    load_iced_fonts();
    let rows = page();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("screenshots/render");
    for scale in [1.0, 1.5] {
        let (mine, theirs) = (ours(&rows, scale), iced_page(&rows, scale));
        assert_eq!((mine.width, mine.height), (theirs.width, theirs.height));
        mine.save(&dir.join(format!("text-{scale}-ours.png"))).unwrap();
        theirs.save(&dir.join(format!("text-{scale}-iced.png"))).unwrap();
        difference(&mine, &theirs).save(&dir.join(format!("text-{scale}-difference.png"))).unwrap();
        let mut worst = row_differences(&mine, &theirs, &rows, scale);
        worst.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (d, row) in worst.iter().take(12) {
            eprintln!("scale {scale}: {d:8.1} {row}");
        }
    }
}
