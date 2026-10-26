//! Shaping a run as iced does: cosmic-text's buffer with lines 1.3 times the size apart,
//! wrapped between words, basic shaping for plain ASCII text and full shaping for the rest. Letter
//! spacing goes between the letters of a word and after each space, as iced's letter-by-letter
//! layout spaces them, and lines are aligned here rather than by cosmic-text.

use cosmic_text::{Attrs, Buffer, Family, FeatureTag, FontFeatures, FontSystem, LayoutGlyph, LetterSpacing, Metrics, Shaping, Style, Weight, Wrap};

use super::{Align, LINE_HEIGHT, Spec, engine::Glyphs};
use crate::overlay::render::{Point, Rect};

/// Smaller text than this, in logical pixels, draws nothing: cosmic-text needs a line height.
const MIN_SIZE: f32 = 0.5;

/// A run, shaped.
pub(super) struct Run {
    pub spec: Spec,
    /// The fonts' version it was shaped with (`font::version`).
    pub version: u64,
    pub size: Point,
    pub ink: Rect,
    pub lines: Vec<Line>,
}

/// One line of a run, in its box's coordinates.
pub(super) struct Line {
    /// Its glyphs, at their place in the box (alignment and letter spacing applied).
    pub glyphs: Vec<LayoutGlyph>,
    /// From the box's top to the line's baseline.
    pub baseline: f32,
    pub underline: Option<Rect>,
}

impl Run {
    pub fn shape(fonts: &mut FontSystem, spec: Spec, glyphs: &mut Glyphs) -> Run {
        let size = spec.size;
        if !(size.is_finite() && size >= MIN_SIZE) {
            let size = Point::new(0.0, if size.is_finite() { size.max(0.0) * LINE_HEIGHT } else { 0.0 });
            return Run { spec, version: 0, size, ink: Rect::default(), lines: Vec::new() };
        }
        let text = spec.case.apply(&spec.text);
        let spacing = if spec.spacing.is_finite() { spec.spacing.max(0.0) } else { 0.0 };
        let mut buffer = Buffer::new(fonts, Metrics::new(size, size * LINE_HEIGHT));
        buffer.set_wrap(fonts, Wrap::Word);
        buffer.set_size(fonts, spec.width.filter(|w| w.is_finite()).map(|w| w.max(0.0)), None);
        // Iced's `Shaping::Auto`, except that iced underlines through rich text, which it always
        // shapes in full (kerning included).
        let shaping = if text.is_ascii() && !spec.underline { Shaping::Basic } else { Shaping::Advanced };
        buffer.set_text(fonts, &text, &attrs(&spec, spacing), shaping, None);
        buffer.shape_until_scroll(fonts, false);

        let mut lines = Vec::new();
        let mut widths = Vec::new();
        let mut height: f32 = 0.0;
        for run in buffer.layout_runs() {
            let mut placed = run.glyphs.to_vec();
            let taken = if spacing > 0.0 { unspace(&mut placed, run.text, spacing) } else { 0.0 };
            widths.push(run.line_w - taken);
            let underline = spec.underline.then(|| underline(&placed, run.line_top, run.line_height, size)).flatten();
            lines.push(Line { glyphs: placed, baseline: run.line_y, underline });
            height = height.max(run.line_top + run.line_height);
        }
        let widest = widths.iter().copied().fold(0.0, f32::max);
        let width = match (spec.align, spec.width) {
            (Align::Center | Align::End, Some(w)) if w.is_finite() => w.max(0.0),
            _ => widest,
        };
        for (line, w) in lines.iter_mut().zip(widths) {
            let shift = match spec.align {
                Align::Start => 0.0,
                Align::Center => (width - w) / 2.0,
                Align::End => width - w,
            };
            line.glyphs.iter_mut().for_each(|g| g.x += shift);
            if let Some(u) = &mut line.underline {
                u.x += shift;
            }
        }
        let size = Point::new(width, if lines.is_empty() { size * LINE_HEIGHT } else { height });
        let ink = ink(&lines, glyphs, fonts);
        Run { spec, version: 0, size, ink, lines }
    }
}

fn attrs(spec: &Spec, spacing: f32) -> Attrs<'static> {
    let mut attrs = Attrs::new()
        .family(spec.family.map_or(Family::SansSerif, Family::Name))
        .weight(if spec.bold { Weight::BOLD } else { Weight::NORMAL })
        .style(if spec.italic { Style::Italic } else { Style::Normal });
    if spacing > 0.0 {
        attrs.letter_spacing_opt = Some(LetterSpacing(spacing / spec.size));
        // Iced draws spaced text a letter at a time, so nothing joins or kerns across letters.
        let mut features = FontFeatures::new();
        features.disable(FeatureTag::KERNING).disable(FeatureTag::STANDARD_LIGATURES).disable(FeatureTag::CONTEXTUAL_LIGATURES);
        attrs = attrs.font_features(features);
    }
    attrs
}

/// Takes the letter spacing cosmic-text puts after every glyph back off the last letter of each
/// word, and returns how much narrower the line is for it.
fn unspace(glyphs: &mut [LayoutGlyph], line: &str, spacing: f32) -> f32 {
    let blank = |g: &LayoutGlyph| line.get(g.start..g.end).is_some_and(|s| s.chars().all(char::is_whitespace));
    let mut taken = 0.0;
    for i in 0..glyphs.len() {
        let ends_word = !blank(&glyphs[i]) && glyphs.get(i + 1).is_none_or(blank);
        let glyph = &mut glyphs[i];
        glyph.x -= taken;
        if ends_word {
            glyph.w -= spacing;
            taken += spacing;
        }
    }
    taken
}

/// Iced's underline: a pixel tall, under the line's glyphs, 0.08 of the size above where the
/// baseline would be if the glyphs filled the size.
fn underline(glyphs: &[LayoutGlyph], top: f32, height: f32, size: f32) -> Option<Rect> {
    let left = glyphs.iter().map(|g| g.x).reduce(f32::min)?;
    let right = glyphs.iter().map(|g| g.x + g.w).reduce(f32::max)?;
    let y = top + (height + size) / 2.0 - size * 0.08;
    (right > left).then(|| Rect::new(left, y, right - left, 1.0))
}

/// Where the glyphs draw at scale 1, grown by a pixel for other scales' hinting and sub-pixel
/// places. Empty at the top-left when nothing draws.
fn ink(lines: &[Line], glyphs: &mut Glyphs, fonts: &mut FontSystem) -> Rect {
    let mut glyphs = glyphs.with(fonts);
    let mut ink: Option<Rect> = None;
    let mut add = |r: Rect| ink = Some(ink.map_or(r, |i| i.union(r)));
    for line in lines {
        let baseline = line.baseline.round();
        for glyph in &line.glyphs {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            if let Some(image) = glyphs.get(physical.cache_key) {
                let p = image.placement;
                add(Rect::new((physical.x + p.left) as f32, (physical.y - p.top) as f32 + baseline, p.width as f32, p.height as f32));
            }
        }
        if let Some(u) = line.underline {
            add(u);
        }
    }
    ink.map_or(Rect::default(), |i| i.outset(1.0))
}
