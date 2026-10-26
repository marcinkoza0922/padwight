//! `padwight debug overlay compare`: draws each scene with iced and with the overlay's own
//! renderer (`overlay/render`), and writes the two side by side with a difference image.
//! See `docs/development/overlay-renderer-plan.md`, step 3.10.

use std::path::Path;

use anyhow::{Result, bail};
use iced::Color;

use super::overlay_shot as shot;
use crate::{overlay, overlay::{OverlayFrame, render::Rgba}, snapshot::Image};

pub const USAGE: &str = "
  overlay compare <what>... [options]
     draws each with iced and with the new renderer, and writes iced | new | difference to
     <out>/<name>.png (default screenshots/compare), worst first in the report. <what> and the
     options are as above (not live or --json). The difference image is |iced - new| per channel,
     times 8. Score: `over` is the share of pixels where any channel differs by more than 8
     levels (the goal is a few levels per pixel, text edges aside), then the mean and the
     largest difference in levels. Scenes the new renderer can't draw yet are listed as not ported.
";

/// A difference this many levels (of 255) or less on every channel counts as the same.
pub const THRESHOLD: u8 = 8;
/// How much the difference image is brightened, so small differences show.
const AMPLIFY: u32 = 8;
/// Space between the pictures, in pixels.
const GUTTER: u32 = 8;
const GUTTER_COLOR: [u8; 4] = [0x80, 0x00, 0x80, 0xff];

/// Draws `frame` with the new renderer on a `size` screen (logical pixels) over `background`,
/// at `scale`, as `overlay::snapshot` does for iced; `None` while it can't draw something in
/// the frame.
///
/// The whole screen is `render::screen` (step 4.1); `None` only when the picture can't be made.
fn draw_new(frame: &OverlayFrame, size: iced::Size, scale: f32, background: Color) -> Option<Image> {
    let background = Rgba { r: background.r, g: background.g, b: background.b, a: 1.0 };
    overlay::render::screen(frame, overlay::render::Point::new(size.width, size.height), scale, background)
}

/// How far apart two same-sized pictures are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    /// The share of pixels (0 to 1) where some channel differs by more than `THRESHOLD`.
    pub over: f32,
    /// The mean over all pixels of the largest channel difference, in levels.
    pub mean: f32,
    /// The largest channel difference anywhere, in levels.
    pub max: u8,
}

/// The largest difference between any channel (alpha included) of two pixels.
fn pixel_diff(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

/// How `a` and `b` differ; `None` if they aren't the same size.
pub fn score(a: &Image, b: &Image) -> Option<Score> {
    if (a.width, a.height) != (b.width, b.height) || a.rgba.len() != b.rgba.len() {
        return None;
    }
    let count = (a.rgba.len() / 4).max(1) as f64;
    let (mut over, mut sum, mut max) = (0usize, 0u64, 0u8);
    for (p, q) in a.rgba.as_chunks::<4>().0.iter().zip(b.rgba.as_chunks::<4>().0) {
        let d = pixel_diff(p, q);
        over += usize::from(d > THRESHOLD);
        sum += u64::from(d);
        max = max.max(d);
    }
    Some(Score { over: (over as f64 / count) as f32, mean: (sum as f64 / count) as f32, max })
}

/// The difference between `a` and `b` (the same size), each channel's difference times
/// `AMPLIFY`, opaque.
pub fn difference(a: &Image, b: &Image) -> Image {
    let mut rgba = Vec::with_capacity(a.rgba.len());
    for (p, q) in a.rgba.as_chunks::<4>().0.iter().zip(b.rgba.as_chunks::<4>().0) {
        rgba.extend(p[..3].iter().zip(&q[..3]).map(|(x, y)| (u32::from(x.abs_diff(*y)) * AMPLIFY).min(255) as u8));
        rgba.push(255);
    }
    Image { width: a.width, height: a.height, rgba }
}

/// The pictures left to right, `GUTTER` apart, on a row as tall as the tallest.
pub fn side_by_side(pictures: &[&Image]) -> Image {
    let height = pictures.iter().map(|p| p.height).max().unwrap_or(0);
    let width = pictures.iter().map(|p| p.width).sum::<u32>() + GUTTER * pictures.len().saturating_sub(1) as u32;
    let mut rgba = GUTTER_COLOR.repeat((width * height) as usize);
    let mut x0 = 0;
    for p in pictures {
        for y in 0..p.height {
            let from = (y * p.width * 4) as usize;
            let to = 4 * (y * width + x0) as usize;
            rgba[to..to + 4 * p.width as usize].copy_from_slice(&p.rgba[from..from + 4 * p.width as usize]);
        }
        x0 += p.width + GUTTER;
    }
    Image { width, height, rgba }
}

/// What came of comparing one scene.
#[derive(Debug)]
pub enum Outcome {
    /// Compared: the score and where the picture was saved.
    Compared(Score, std::path::PathBuf),
    /// The pictures differ in size (iced's, then the new one's).
    SizeMismatch((u32, u32), (u32, u32), std::path::PathBuf),
    /// The new renderer can't draw it yet.
    NotPorted,
}

/// Worst first: size mismatches, then the larger share over the threshold, then the larger mean.
/// Not ported scenes go last, in the order given.
pub fn worst_first(results: &mut [(String, Outcome)]) {
    let rank = |o: &Outcome| match o {
        Outcome::SizeMismatch(..) => (0, 0.0, 0.0),
        Outcome::Compared(s, _) => (1, -s.over, -s.mean),
        Outcome::NotPorted => (2, 0.0, 0.0),
    };
    results.sort_by(|(_, a), (_, b)| rank(a).partial_cmp(&rank(b)).unwrap_or(std::cmp::Ordering::Equal));
}

/// Compares one frame: draws it both ways, saves `<out>/<name>.png`.
fn compare(name: &str, iced: &Image, new: Option<Image>, out: &Path) -> Result<Outcome> {
    let Some(new) = new else {
        return Ok(Outcome::NotPorted);
    };
    let path = out.join(format!("{name}.png"));
    let Some(s) = score(iced, &new) else {
        side_by_side(&[iced, &new]).save(&path)?;
        return Ok(Outcome::SizeMismatch((iced.width, iced.height), (new.width, new.height), path));
    };
    side_by_side(&[iced, &new, &difference(iced, &new)]).save(&path)?;
    Ok(Outcome::Compared(s, path))
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    let (targets, options) = shot::parse_in(args, "screenshots/compare")?;
    if options.json || targets.iter().any(|t| t == "live") {
        bail!("compare draws scenes, frames saved as JSON and packs, not live or --json{USAGE}");
    }
    let [r, g, b] = options.background;
    let background = Color::from_rgb8(r, g, b);
    let mut results = Vec::new();
    for target in targets {
        if target == "glyphs" {
            // The glyph page is laid out by iced alone; nothing of it is ported.
            results.push((target, Outcome::NotPorted));
            continue;
        }
        for (name, frame) in shot::frames_of(&target, &options)? {
            let new = draw_new(&frame, options.size, options.scale, background);
            let outcome = if new.is_some() {
                compare(&name, &overlay::snapshot(frame, options.size, options.scale, background)?, new, &options.out)?
            } else {
                Outcome::NotPorted
            };
            results.push((name, outcome));
        }
    }
    worst_first(&mut results);
    println!("{:<28}{:>8}{:>8}{:>6}", "scene", "over", "mean", "max");
    let mut skipped = Vec::new();
    for (name, outcome) in &results {
        match outcome {
            Outcome::Compared(s, path) => println!("{name:<28}{:>7.2}%{:>8.2}{:>6}  {}", s.over * 100.0, s.mean, s.max, path.display()),
            Outcome::SizeMismatch(a, b, path) => println!("{name:<28}size differs: iced {}x{}, new {}x{}  {}", a.0, a.1, b.0, b.1, path.display()),
            Outcome::NotPorted => skipped.push(name.as_str()),
        }
    }
    if !skipped.is_empty() {
        println!("not ported ({}): {}", skipped.len(), skipped.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(width: u32, height: u32, px: [u8; 4]) -> Image {
        Image { width, height, rgba: px.repeat((width * height) as usize) }
    }

    #[test]
    fn identical_pictures_score_zero() {
        let a = flat(4, 3, [10, 20, 30, 255]);
        assert_eq!(score(&a, &a), Some(Score { over: 0.0, mean: 0.0, max: 0 }));
    }

    #[test]
    fn small_differences_are_under_the_threshold_and_big_ones_over() {
        let a = flat(2, 2, [100, 100, 100, 255]);
        let mut b = flat(2, 2, [100, 100, 100, 255]);
        b.rgba[0] = 100 + THRESHOLD; // exactly the threshold: still the same
        b.rgba[4 + 1] = 100 - THRESHOLD - 1; // one past it
        b.rgba[8 + 2] = 200;
        let s = score(&a, &b).unwrap();
        assert!((s.over - 0.5).abs() < 1e-6, "two of four pixels are over: {s:?}");
        assert_eq!(s.max, 100);
        assert!((s.mean - (8.0 + 9.0 + 100.0) / 4.0).abs() < 1e-5);
    }

    #[test]
    fn alpha_counts() {
        let s = score(&flat(1, 1, [0, 0, 0, 255]), &flat(1, 1, [0, 0, 0, 100])).unwrap();
        assert_eq!((s.over, s.max), (1.0, 155));
    }

    #[test]
    fn pictures_of_different_sizes_have_no_score() {
        assert_eq!(score(&flat(2, 2, [0; 4]), &flat(3, 2, [0; 4])), None);
    }

    #[test]
    fn the_difference_is_amplified_and_clamped() {
        let d = difference(&flat(1, 2, [10, 50, 0, 255]), &flat(1, 2, [12, 0, 0, 0]));
        assert_eq!(d.pixel(0, 0), [16, 255, 0, 255], "2 levels show as 16, 50 clamp, alpha is opaque");
    }

    #[test]
    fn pictures_sit_side_by_side_with_a_gutter() {
        let a = flat(2, 3, [1, 1, 1, 255]);
        let b = flat(3, 2, [2, 2, 2, 255]);
        let row = side_by_side(&[&a, &b]);
        assert_eq!((row.width, row.height), (2 + GUTTER + 3, 3));
        assert_eq!(row.pixel(1, 2), [1, 1, 1, 255]);
        assert_eq!(row.pixel(2, 0), GUTTER_COLOR);
        assert_eq!(row.pixel(2 + GUTTER, 1), [2, 2, 2, 255]);
        assert_eq!(row.pixel(2 + GUTTER, 2), GUTTER_COLOR, "the shorter picture leaves the rest of its column empty");
    }

    #[test]
    fn the_report_puts_the_worst_first_and_unported_last() {
        let s = |over, mean| Outcome::Compared(Score { over, mean, max: 0 }, "x.png".into());
        let mut results = vec![
            ("skipped".to_string(), Outcome::NotPorted),
            ("good".to_string(), s(0.01, 1.0)),
            ("bad".to_string(), s(0.2, 5.0)),
            ("size".to_string(), Outcome::SizeMismatch((1, 1), (2, 2), "x.png".into())),
            ("ok".to_string(), s(0.01, 0.5)),
        ];
        worst_first(&mut results);
        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["size", "bad", "good", "ok", "skipped"]);
    }

    #[test]
    fn a_scene_the_new_renderer_cannot_draw_is_not_ported() {
        let iced = flat(2, 2, [0, 0, 0, 255]);
        assert!(matches!(compare("x", &iced, None, Path::new("unused")).unwrap(), Outcome::NotPorted));
    }

    #[test]
    fn a_compared_scene_is_saved() {
        let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("screenshots/compare-test");
        let a = flat(4, 4, [0, 0, 0, 255]);
        let b = flat(4, 4, [30, 0, 0, 255]);
        let Outcome::Compared(s, path) = compare("unit", &a, Some(b), &out).unwrap() else { panic!("compared") };
        assert_eq!((s.over, s.max), (1.0, 30));
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(out);
    }
}
