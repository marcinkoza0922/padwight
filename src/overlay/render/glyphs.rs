//! Button glyphs, D-pads, icons and bars, for info overlays, logs and menu rows: `draw/glyphs.rs`
//! with its numbers.

use super::{Corners, Fit, Length, Markup, Node, Op, Outline, Rect, Rgba, Style, Svg, text::Spec};
use crate::{
    info::{Icon, Segment},
    media::PlayState,
    overlay::art,
};

/// `content` on a rounded box of its own size, filled and bordered inside, as iced's container
/// with a background and a border.
pub(super) fn backed(content: Node, radii: Corners, fill: Option<Rgba>, border: Option<Outline>) -> Node {
    let back = Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        ops.push(Op::Rect { rect, radii, fill: fill.map(Into::into), border });
    })
    .width(Length::Fill)
    .height(Length::Fill);
    Node::stack(content).under(back)
}

/// A run of plain text in the panel's font, as iced's `text` widget sets it.
pub(super) fn label(style: &Style, text: &str, size: f32, color: Rgba) -> Node {
    Node::text(Spec { family: style.font, ..Spec::new(text, size) }, color)
}

/// One piece of an info overlay's line, or of a menu row's glyphs, at glyph scale `s`.
pub fn segment(seg: &Segment, style: &Style, s: f32, opacity: f32) -> Node {
    match seg {
        Segment::Text(t) => label(style, t, 16.0 * s, style.background_text),
        Segment::Glyph { label, fill, round } => glyph(label, (*fill, *round), style, s, opacity),
        Segment::Dpad(lit) => dpad(*lit, style, s),
        Segment::Icon(icon) => icon_glyph(icon, style, s),
        // Formatting tags and icons by name are laid out by `rich::line`; one that gets here
        // on its own is drawn as nothing, or as the icon's own token.
        Segment::Mark(_) => Node::space().width(0.0),
        Segment::Picture(name) => label(style, &format!("{{icon:{name}}}"), 14.0 * s, style.muted),
        Segment::Bar(done) => bar(*done, s),
    }
}

/// A cross-shaped D-pad with the pressed arms (`[up, down, left, right]`) lit.
pub fn dpad([up, down, left, right]: [bool; 4], style: &Style, s: f32) -> Node {
    let cell = 8.0 * s;
    let arm = |on: bool| {
        let color = if on { style.selected } else { style.item };
        let edge = Outline { width: 1.0, color: style.item_text.scale_alpha(0.35) };
        backed(Node::space().width(cell).height(cell), Corners::all(1.5 * s), Some(color), Some(edge))
    };
    let gap = || Node::space().width(cell).height(cell);
    Node::column([
        Node::row([gap(), arm(up), gap()]),
        Node::row([arm(left), arm(false), arm(right)]),
        Node::row([gap(), arm(down), gap()]),
    ])
}

/// A button glyph: a colored disc for face buttons, a rounded tag for the rest.
pub fn glyph(text: &str, (fill, round): (Option<[u8; 3]>, bool), style: &Style, s: f32, opacity: f32) -> Node {
    let (bg, fg) = match fill {
        Some([r, g, b]) => (Rgba::from_rgba8(r, g, b, opacity), Rgba { a: opacity, ..Rgba::WHITE }),
        None => (style.item, style.item_text),
    };
    let size = 24.0 * s;
    let disc = round && text.chars().count() <= 2;
    let body = Node::container(label(style, text, 13.0 * s, fg));
    let body = if disc { body.center_x(size).center_y(size) } else { body.padding([0.0, 7.0 * s]).center_y(size) };
    let radii = Corners::all(if disc { size / 2.0 } else { 6.0 * s });
    backed(body, radii, Some(bg), Some(Outline { width: 1.0, color: fg.scale_alpha(0.2) }))
}

/// An SVG `size` square, drawn to fit.
pub(super) fn picture(markup: String, size: f32, opacity: f32) -> Node {
    let svg = Svg::Markup(Markup::new(markup));
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| ops.push(Op::Svg { svg: svg.clone(), rect, fit: Fit::Contain, opacity }))
        .width(size)
        .height(size)
}

/// An icon in an info cell, drawn as SVG so it doesn't depend on the font having the glyph.
fn icon_glyph(icon: &Icon, style: &Style, s: f32) -> Node {
    picture(icon_svg(icon, style.background_text), 22.0 * s, 1.0)
}

/// The SVG for an info icon, in `ink`.
pub fn icon_svg(icon: &Icon, ink: Rgba) -> String {
    let [r, g, b, _] = ink.to_u8();
    art::icons::info_svg(icon, [r, g, b], ink.a)
}

/// A play, pause or stop symbol, drawn so it doesn't depend on the font having the glyph.
pub fn state_icon(state: PlayState, color: Rgba, size: f32) -> Node {
    let [r, g, b, _] = color.to_u8();
    picture(art::icons::play_state_svg(state, [r, g, b], color.a), size, 1.0)
}

/// iced's default progress bar on its dark theme: a track and a bar, rounded by 2, filled to `done`.
fn bar(done: f32, s: f32) -> Node {
    bar_of(done, 160.0 * s, 6.0 * s)
}

/// A progress bar `length` by `girth`.
pub fn bar_of(done: f32, length: f32, girth: f32) -> Node {
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        let radii = Corners::all(2.0);
        ops.push(Op::Rect { rect, radii, fill: Some(Rgba::from_rgb8(80, 84, 93).into()), border: None });
        let w = rect.w * done;
        if w > 0.0 {
            ops.push(Op::Rect { rect: Rect { w, ..rect }, radii, fill: Some(Rgba::from_rgb8(88, 101, 242).into()), border: None });
        }
    })
    .width(length)
    .height(girth)
}
