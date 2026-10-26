//! Button glyphs, D-pads and icons, for info overlays, logs and menu rows.

use iced::{
    Border, Color, Element,
    widget::{column, container, row, space, text},
};

use super::Colors;
use crate::{
    info::{Icon, Segment},
    media::PlayState,
    overlay::art,
};

/// One piece of an info overlay's line, or of a menu row's glyphs.
pub(super) fn segment_element<'a, M: 'a>(seg: &Segment, c: Colors, s: f32, opacity: f32) -> Element<'a, M> {
    match seg {
        Segment::Text(t) => Element::from(text(t.clone()).font(c.font).size(16.0 * s).color(c.background_text)),
        Segment::Glyph { label, fill, round } => glyph(label, *fill, *round, c, s, opacity),
        Segment::Dpad(lit) => dpad_glyph(*lit, c, s),
        Segment::Icon(icon) => icon_glyph(icon, c, s),
        // Formatting tags and icons by name are laid out by `marked::line`; one that gets here
        // on its own is drawn as nothing, or as the icon's own space.
        Segment::Mark(_) => space().width(0).into(),
        Segment::Picture(name) => text(format!("{{icon:{name}}}")).font(c.font).size(14.0 * s).color(c.muted).into(),
        Segment::Bar(done) => iced::widget::progress_bar(0.0..=1.0, *done).length(160.0 * s).girth(6.0 * s).into(),
    }
}

/// A cross-shaped D-pad with the pressed arms (`[up, down, left, right]`) lit.
fn dpad_glyph<'a, M: 'a>([up, down, left, right]: [bool; 4], c: Colors, s: f32) -> Element<'a, M> {
    let cell = 8.0 * s;
    let arm = |on: bool| -> Element<'a, M> {
        let color = if on { c.selected } else { c.item };
        container(space())
            .width(cell)
            .height(cell)
            .style(move |_: &iced::Theme| container::Style {
                background: Some(color.into()),
                border: Border { width: 1.0, radius: (1.5 * s).into(), color: c.item_text.scale_alpha(0.35) },
                ..container::Style::default()
            })
            .into()
    };
    let gap = || -> Element<'a, M> { space().width(cell).height(cell).into() };
    column![row![gap(), arm(up), gap()], row![arm(left), arm(false), arm(right)], row![gap(), arm(down), gap()]]
    .into()
}

/// A button glyph: a colored disc for face buttons, a rounded tag for the rest.
#[expect(clippy::too_many_arguments, reason = "predates the size lints")]
fn glyph<'a, M: 'a>(label: &str, fill: Option<[u8; 3]>, round: bool, c: Colors, s: f32, opacity: f32) -> Element<'a, M> {
    let (bg, fg) = match fill {
        Some([r, g, b]) => (Color::from_rgba8(r, g, b, opacity), Color { a: opacity, ..Color::WHITE }),
        None => (c.item, c.item_text),
    };
    let size = 24.0 * s;
    let disc = round && label.chars().count() <= 2;
    let body = container(text(label.to_string()).font(c.font).size(13.0 * s).color(fg));
    let body = if disc {
        body.center_x(size).center_y(size)
    } else {
        body.padding([0.0, 7.0 * s]).height(size).center_y(size)
    };
    body.style(move |_: &iced::Theme| container::Style {
        background: Some(bg.into()),
        border: Border {
            width: 1.0,
            radius: if disc { size / 2.0 } else { 6.0 * s }.into(),
            color: fg.scale_alpha(0.2),
        },
        ..container::Style::default()
    })
    .into()
}

/// An icon in an info cell, drawn as SVG so it doesn't depend on the font having the glyph.
fn icon_glyph<'a, M: 'a>(icon: &Icon, c: Colors, s: f32) -> Element<'a, M> {
    let size = 22.0 * s;
    let svg_text = icon_svg(icon, c.background_text);
    iced::widget::svg(iced::widget::svg::Handle::from_memory(svg_text.into_bytes())).width(size).height(size).into()
}

/// The SVG for an info icon, in `ink`. A controller is drawn in its kind's color, where it
/// has one; a battery's fill is green, amber or red by its level.
pub fn icon_svg(icon: &Icon, ink: Color) -> String {
    let [r, g, b, _] = ink.into_rgba8();
    art::icons::info_svg(icon, [r, g, b], ink.a)
}

/// A play, pause or stop symbol, drawn so it doesn't depend on the font having the glyph.
pub(super) fn state_icon<'a, M: 'a>(state: PlayState, color: Color, size: f32) -> Element<'a, M> {
    let [r, g, b, _] = color.into_rgba8();
    let svg_text = art::icons::play_state_svg(state, [r, g, b], color.a);
    iced::widget::svg(iced::widget::svg::Handle::from_memory(svg_text.into_bytes())).width(size).height(size).into()
}
