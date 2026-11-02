//! One window state checked: text that wraps, clips or leaves the window, and text too faint
//! against what it is drawn on.

use std::collections::HashMap;

use anyhow::Result;
use iced::{Color, Rectangle, Size};

use super::{
    Finding, Kind, drawn,
    contrast::{drawn_colors, ratio},
    probe::{Probe, Seen, natural},
};
use crate::{
    gui::App,
    snapshot::{self, Image},
};

/// Labels at most this long are meant to sit on one line.
const LABEL_CHARS: usize = 48;
/// WCAG AA for normal text, and for large text (24 px and up).
const AA: f32 = 4.5;
const AA_LARGE: f32 = 3.0;
/// The width a window is laid out at to measure text unwrapped.
const WIDE: f32 = 3600.0;
/// The tallest image drawn; the page is a scrolled column, so this only bounds a long one.
const MAX_HEIGHT: f32 = 12_000.0;

/// Checks `app` in a window `size.width` wide (the height follows the content, since the page
/// scrolls). Returns the findings and the image they were found in.
pub fn scan(app: &App, scene: &str, theme_name: &'static str, size: Size) -> Result<(Vec<Finding>, Image)> {
    let mut first = Probe::new();
    snapshot::probe(drawn(app).0, size, &mut first)?;
    let bottom = first.texts.iter().map(|t| t.bounds.y + t.bounds.height).fold(size.height, f32::max);
    let tall = Size::new(size.width, (bottom + 24.0).min(MAX_HEIGHT));
    let mut narrow = Probe::new();
    let (element, theme, background) = drawn(app);
    let image = snapshot::render_probed(element, tall, 1.0, background, &theme, &mut narrow)?;
    let mut wide = Probe::new();
    snapshot::probe(drawn(app).0, Size::new(WIDE, tall.height), &mut wide)?;
    let natural = natural(&wide.texts);
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut found = Vec::new();
    for seen in &narrow.texts {
        let n = counts.entry(&seen.text).or_default();
        let alone = natural.get(&(seen.text.as_str(), *n)).copied();
        *n += 1;
        if seen.text.trim().is_empty() {
            continue;
        }
        let mut add = |kind, detail: String| found.push(Finding::new(scene, theme_name, kind, &seen.text, detail, seen.bounds));
        layout_findings(seen, alone, size.width, &mut add);
        if let Some(detail) = contrast_finding(&image, seen, alone) {
            add(Kind::LowContrast, detail);
        }
    }
    Ok((found, image))
}

fn layout_findings(seen: &Seen, alone: Option<Rectangle>, window: f32, add: &mut dyn FnMut(Kind, String)) {
    let b = seen.bounds;
    let chars = seen.text.chars().count();
    if let Some(alone) = alone.filter(|a| a.height > 0.0) {
        let lines = (b.height / alone.height).round().max(1.0);
        if lines > 1.5 {
            let squeezed = chars as f32 / lines < 12.0;
            if (chars <= LABEL_CHARS && !seen.text.contains('\n')) || squeezed {
                add(Kind::Wrapped, format!("{lines} lines, {:.0} px wide; one line needs {:.0} px", b.width, alone.width));
            }
        }
    }
    if b.x + b.width > window + 0.5 || b.x < -0.5 {
        add(Kind::OffScreen, format!("spans {:.0} to {:.0} px in a {window:.0} px window", b.x, b.x + b.width));
    } else if let Some(v) = seen.visible.filter(|v| v.width < b.width - 1.0 && v.width > 0.0) {
        add(Kind::Clipped, format!("{:.0} of {:.0} px wide shows", v.width, b.width));
    }
    if let Some(p) = seen.parent {
        let (right, bottom) = (b.x + b.width - (p.x + p.width), b.y + b.height - (p.y + p.height));
        if right > 1.5 || bottom > 1.5 || p.x - b.x > 1.5 || p.y - b.y > 1.5 {
            add(Kind::OutOfParent, format!("sticks out of its container by {:.0} px right, {:.0} px below", right.max(0.0), bottom.max(0.0)));
        }
    }
}

fn contrast_finding(image: &Image, seen: &Seen, alone: Option<Rectangle>) -> Option<String> {
    let v = seen.visible?;
    let area = [v.x.ceil() + 1.0, v.y.ceil() + 1.0, (v.x + v.width).floor() - 1.0, (v.y + v.height).floor() - 1.0];
    if area[2] - area[0] < 3.0 || area[3] - area[1] < 3.0 || area[0] < 0.0 || area[1] < 0.0 {
        return None;
    }
    let (bg, fg) = drawn_colors(image, area.map(|a| a as u32))?;
    let size = alone.map_or(seen.bounds.height, |a| a.height) / 1.3;
    // Large text, and one-character icons and badges (non-text contrast), need 3:1.
    let need = if size >= 24.0 || seen.text.trim().chars().count() <= 2 { AA_LARGE } else { AA };
    let got = ratio(fg, bg);
    (got < need).then(|| format!("{got:.2}:1 (needs {need}); {} on {}", hex(fg), hex(bg)))
}

pub fn hex(c: Color) -> String {
    let [r, g, b, _] = c.into_rgba8();
    format!("#{r:02x}{g:02x}{b:02x}")
}
