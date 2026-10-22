//! A line of segments: text, glyphs and icons, with the formatting tags among them (`{b}`,
//! `{i}`, `{u}`, `{color:…}`, `{box}`) applied to the text they hold between them.

use iced::{
    Alignment, Border, Color, Element, Font, Padding,
    font::{Style as Slant, Weight},
    widget::{container, row},
};

use super::{Colors, glyphs::segment_element, typeset};
use crate::info::{Mark, Segment};

/// How a line is set: the size and color of its text, the scale of its glyphs, the gap between
/// pieces, and whether the typography (case, spacing) applies.
#[derive(Clone, Copy)]
pub(super) struct Setting {
    pub size: f32,
    pub color: Color,
    /// What glyphs and icons are scaled by (1 is a 16-unit label).
    pub scale: f32,
    pub gap: f32,
    pub opacity: f32,
    pub typed: bool,
    /// Text in a multi-piece line has the space around it trimmed, the gap taking its place.
    pub trim: bool,
}

/// The formatting in force.
#[derive(Clone, Default)]
struct Look {
    bold: bool,
    italic: bool,
    underline: bool,
    color: Option<[u8; 3]>,
    font: Option<String>,
}

impl Look {
    fn apply(&mut self, mark: &Mark) {
        match mark {
            Mark::Bold(on) => self.bold = *on,
            Mark::Italic(on) => self.italic = *on,
            Mark::Underline(on) => self.underline = *on,
            Mark::Color(c) => self.color = *c,
            Mark::Font(name) => self.font.clone_from(name),
            Mark::Boxed(_) => {}
        }
    }

    fn font(&self, base: Font) -> Font {
        let base = self.font.as_deref().map_or(base, |name| crate::font::resolve(Some(name)));
        Font {
            weight: if self.bold { Weight::Bold } else { base.weight },
            style: if self.italic { Slant::Italic } else { base.style },
            ..base
        }
    }

    fn face(&self, base: Font, set: Setting) -> typeset::Face {
        typeset::Face { font: self.font(base), size: set.size, color: self.ink(set.color), underline: self.underline }
    }

    /// `base` unless a color tag says another; the alpha stays the line's, so a fade still fades.
    fn ink(&self, base: Color) -> Color {
        self.color.map_or(base, |[r, g, b]| Color { a: base.a, ..Color::from_rgb8(r, g, b) })
    }
}

/// The pieces of `segments` in a row that wraps, formatted as its tags say. A tag left open
/// holds to the end.
pub(super) fn line<'a, M: 'a>(segments: &[Segment], c: Colors, set: Setting) -> Element<'a, M> {
    let ty = if set.typed { c.typ } else { typeset::Type::PLAIN };
    let new_row = || -> Vec<Element<'a, M>> { Vec::new() };
    let mut look = Look::default();
    let mut done = new_row();
    let mut boxed: Option<Vec<Element<'a, M>>> = None;
    let single = segments.iter().filter(|s| !matches!(s, Segment::Mark(_))).count() <= 1;
    for seg in segments {
        let element = match seg {
            Segment::Mark(Mark::Boxed(true)) => {
                boxed.get_or_insert_with(new_row);
                continue;
            }
            Segment::Mark(Mark::Boxed(false)) => {
                if let Some(inner) = boxed.take() {
                    done.push(outline(inner, c, set));
                }
                continue;
            }
            Segment::Mark(mark) => {
                look.apply(mark);
                continue;
            }
            Segment::Text(t) if boxed.is_none() && !single => {
                words(t, look.face(c.font, set), ty, set, &mut done);
                continue;
            }
            Segment::Text(t) => {
                let t = if set.trim && !single { t.trim() } else { t.as_str() };
                if t.is_empty() {
                    continue;
                }
                typeset::underlined(ty, t, look.face(c.font, set))
            }
            Segment::Picture(name) => picture(name, look.ink(set.color), set, c, ty),
            other => segment_element(other, c, set.scale, set.opacity),
        };
        match &mut boxed {
            Some(inner) => inner.push(element),
            None => done.push(element),
        }
    }
    if let Some(inner) = boxed {
        done.push(outline(inner, c, set));
    }
    row(done).spacing(set.gap).align_y(Alignment::Center).wrap().vertical_spacing(set.gap).into()
}

/// Text outside a box, a piece per word, so the row wraps between words: a run too long to fit
/// beside a glyph would otherwise drop whole to the next line and leave the glyph alone there,
/// and a wrapped line can't start with a space. Where the line keeps its spaces (`trim` off), a
/// word followed by one, or the piece before text that starts with one, is padded by a space's width.
/// Spaced letters space their words as much again, as after any letter.
fn words<'a, M: 'a>(t: &str, face: typeset::Face, ty: typeset::Type, set: Setting, done: &mut Vec<Element<'a, M>>) {
    // The gap between pieces is part of the space.
    let space = if set.trim { 0.0 } else { (ty.size(set.size) * 0.25 - set.gap).max(0.0) } + ty.spacing();
    let spaced = |e: Element<'a, M>| -> Element<'a, M> { container(e).padding(Padding::ZERO.right(space)).into() };
    if space > 0.0
        && t.starts_with(char::is_whitespace)
        && let Some(before) = done.pop()
    {
        done.push(spaced(before));
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    let trailing = t.ends_with(char::is_whitespace);
    for (i, word) in words.iter().enumerate() {
        let element = typeset::underlined(ty, word, face);
        done.push(if space > 0.0 && (i + 1 < words.len() || trailing) { spaced(element) } else { element });
    }
}

/// The pieces of a `{box}` run inside a light outline in the text's color.
fn outline<'a, M: 'a>(inner: Vec<Element<'a, M>>, c: Colors, set: Setting) -> Element<'a, M> {
    let edge = set.color.scale_alpha(0.55);
    let radius = 3.0 + 3.0 * c.corners;
    container(row(inner).spacing(set.gap).align_y(Alignment::Center))
        .padding([set.size * 0.05, set.size * 0.35])
        .style(move |_: &iced::Theme| container::Style {
            border: Border { color: edge, width: 1.0, radius: radius.into() },
            ..container::Style::default()
        })
        .into()
}

/// Icon `name` drawn in `ink`, as tall as the text. An icon that doesn't exist shows its token
/// as written, so the mistake can be seen and fixed.
fn picture<'a, M: 'a>(name: &str, ink: Color, set: Setting, c: Colors, ty: typeset::Type) -> Element<'a, M> {
    let [r, g, b, _] = ink.into_rgba8();
    match crate::icons::svg(name, [r, g, b]) {
        Some(svg) => {
            let side = ty.size(set.size) * 1.25;
            let handle = iced::widget::svg::Handle::from_memory(svg.into_bytes());
            iced::widget::svg(handle).width(side).height(side).opacity(set.opacity * ink.a).into()
        }
        None => typeset::line(typeset::Type::PLAIN, c.font, &format!("{{icon:{name}}}"), set.size, ink.scale_alpha(0.7)),
    }
}
