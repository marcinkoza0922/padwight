//! A line of segments: text, glyphs and icons, with the formatting tags among them (`{b}`,
//! `{i}`, `{u}`, `{color:…}`, `{font:…}`, `{box}`) applied to the text they hold between them.
//! This is `draw/marked.rs` and `rich_text` in `draw.rs`, with their numbers. The tags are parsed
//! by `info::glyph_text`, shared with iced.

use super::{
    Align, Corners, Node, Outline, Padding, Rgba, Style, Type,
    glyphs::{backed, picture, segment},
    text::Spec,
};
use crate::info::{Glyphs, Mark, Segment};

/// How a line is set: the size and color of its text, the scale of its glyphs, the gap between
/// pieces, and whether the typography (case, spacing) applies.
#[derive(Clone, Copy)]
pub struct Setting {
    pub size: f32,
    pub color: Rgba,
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

    /// `base` unless a color tag says another; the alpha stays the line's, so a fade still fades.
    fn ink(&self, base: Rgba) -> Rgba {
        self.color.map_or(base, |[r, g, b]| Rgba { a: base.a, ..Rgba::from_rgb8(r, g, b) })
    }

    /// A run of `t` as this formatting sets it, in the typography `ty`.
    fn run(&self, style: &Style, ty: Type, t: &str, set: Setting) -> Node {
        let family = self.font.as_deref().map_or(style.font, |name| crate::font::family(Some(name)));
        let spec = Spec {
            family,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            case: ty.case,
            spacing: ty.spacing,
            ..Spec::new(t, ty.sized(set.size))
        };
        Node::text(spec, self.ink(set.color))
    }
}

/// The pieces of `segments` in a row that wraps, formatted as its tags say. A tag left open
/// holds to the end.
pub fn line(segments: &[Segment], style: &Style, set: Setting) -> Node {
    let ty = if set.typed { style.typ } else { Type::PLAIN };
    let mut look = Look::default();
    let mut done: Vec<Node> = Vec::new();
    let mut boxed: Option<Vec<Node>> = None;
    let single = segments.iter().filter(|s| !matches!(s, Segment::Mark(_))).count() <= 1;
    for seg in segments {
        let node = match seg {
            Segment::Mark(Mark::Boxed(true)) => {
                boxed.get_or_insert_with(Vec::new);
                continue;
            }
            Segment::Mark(Mark::Boxed(false)) => {
                if let Some(inner) = boxed.take() {
                    done.push(outline(inner, style, set));
                }
                continue;
            }
            Segment::Mark(mark) => {
                look.apply(mark);
                continue;
            }
            Segment::Text(t) if boxed.is_none() && !single => {
                words(t, &look, style, set, &mut done);
                continue;
            }
            Segment::Text(t) => {
                let t = if set.trim && !single { t.trim() } else { t.as_str() };
                if t.is_empty() {
                    continue;
                }
                look.run(style, ty, t, set)
            }
            Segment::Picture(name) => icon(name, look.ink(set.color), set, style, ty),
            other => segment(other, style, set.scale, set.opacity),
        };
        match &mut boxed {
            Some(inner) => inner.push(node),
            None => done.push(node),
        }
    }
    if let Some(inner) = boxed {
        done.push(outline(inner, style, set));
    }
    Node::wrap(done).spacing(set.gap).line_spacing(set.gap).align_y(Align::Center)
}

/// Text outside a box, a piece per word, so the row wraps between words: a run too long to fit
/// beside a glyph would otherwise drop whole to the next line and leave the glyph alone there,
/// and a wrapped line can't start with a space. Where the line keeps its spaces (`trim` off), a
/// word followed by one, or the piece before text that starts with one, is padded by a space's width.
/// Spaced letters space their words as much again, as after any letter.
fn words(t: &str, look: &Look, style: &Style, set: Setting, done: &mut Vec<Node>) {
    let ty = if set.typed { style.typ } else { Type::PLAIN };
    // The gap between pieces is part of the space.
    let space = if set.trim { 0.0 } else { (ty.sized(set.size) * 0.25 - set.gap).max(0.0) } + ty.spacing;
    let spaced = |n: Node| Node::container(n).padding(Padding { right: space, ..Padding::ZERO });
    if space > 0.0
        && t.starts_with(char::is_whitespace)
        && let Some(before) = done.pop()
    {
        done.push(spaced(before));
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    let trailing = t.ends_with(char::is_whitespace);
    for (i, word) in words.iter().enumerate() {
        let node = look.run(style, ty, word, set);
        done.push(if space > 0.0 && (i + 1 < words.len() || trailing) { spaced(node) } else { node });
    }
}

/// The pieces of a `{box}` run inside a light outline in the text's color.
fn outline(inner: Vec<Node>, style: &Style, set: Setting) -> Node {
    let edge = Outline { width: 1.0, color: set.color.scale_alpha(0.55) };
    let radii = Corners::all(3.0 + 3.0 * style.corners);
    let row = Node::row(inner).spacing(set.gap).align_y(Align::Center).padding([set.size * 0.05, set.size * 0.35]);
    backed(row, radii, None, Some(edge))
}

/// Icon `name` drawn in `ink`, as tall as the text. An icon that doesn't exist shows its token
/// as written, so the mistake can be seen and fixed.
fn icon(name: &str, ink: Rgba, set: Setting, style: &Style, ty: Type) -> Node {
    let [r, g, b, _] = ink.to_u8();
    match crate::icons::svg(name, [r, g, b]) {
        Some(svg) => picture(svg, ty.sized(set.size) * 1.25, set.opacity * ink.a),
        None => {
            let spec = Spec { family: style.font, ..Spec::new(format!("{{icon:{name}}}"), set.size) };
            Node::text(spec, ink.scale_alpha(0.7))
        }
    }
}

/// Text whose tokens (`{south}`, `{lt}`, `{icon:bow}`…) are drawn as glyphs and icons, and whose
/// formatting tags (`{b}`, `{u}`, `{color:red}`…) are applied, sized for text of `size`. Plain
/// text stays one text node.
pub fn rich(t: &str, style: &Style, size: f32, color: Rgba) -> Node {
    let glyphs = Glyphs { family: style.family, nintendo_layout: style.nintendo_layout };
    let segments = crate::info::glyph_text(t, glyphs);
    match segments.as_slice() {
        [] => plain("", style, size, color),
        [Segment::Text(t)] => plain(t, style, size, color),
        _ => line(
            &segments,
            style,
            Setting { size, color, scale: size / 17.0, gap: size * 0.3, opacity: style.anim.opacity(), typed: true, trim: true },
        ),
    }
}

/// Text in the typography's case, size and letter spacing, in `color`. A run that wraps keeps wrapping.
pub fn plain(t: &str, style: &Style, size: f32, color: Rgba) -> Node {
    Look::default().run(style, style.typ, t, Setting { size, color, scale: 1.0, gap: 0.0, opacity: 1.0, typed: true, trim: false })
}

/// `segments` as an info overlay's cell draws them, at panel scale `s`: in a row that wraps them
/// onto the next line when they're too wide.
pub fn info_cell(segments: &[Segment], style: &Style, s: f32, opacity: f32) -> Node {
    line(segments, style, Setting { size: 16.0 * s, color: style.background_text, scale: s, gap: 2.0 * s, opacity, typed: false, trim: false })
}

#[cfg(test)]
mod tests;
