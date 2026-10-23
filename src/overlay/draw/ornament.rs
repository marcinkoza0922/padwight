//! The ornaments drawn over and around a panel: a picture behind its contents, a tint, a
//! vignette, and the lines between a list's items (the frame's corners are in `frame`). Each is a
//! layer of a stack the size of the panel (see `framed`), except the dividers, which sit between rows.

use iced::{
    Background, Color, ContentFit, Element, Length,
    gradient,
    widget::{container, image, space, svg},
};

use crate::{
    assets,
    config::{BackgroundImage, ImageFit, Paint, Stroke, Tint, Vignette},
};

fn color_of(p: &Paint, fallback: [u8; 3], fade: f32) -> Color {
    let [r, g, b] = p.rgb().unwrap_or(fallback);
    Color::from_rgba8(r, g, b, (p.opacity * fade).clamp(0.0, 1.0))
}

/// The picture behind a panel's contents, covering the panel as its fit says. A PNG or JPEG
/// is drawn as a picture, an SVG as a drawing.
pub fn picture<'a, M: 'a>(bg: &BackgroundImage, fade: f32) -> Element<'a, M> {
    let path = assets::locate("backgrounds", &bg.file);
    let fit = match bg.fit {
        ImageFit::Cover => ContentFit::Cover,
        ImageFit::Contain => ContentFit::Contain,
        ImageFit::Stretch => ContentFit::Fill,
    };
    let opacity = (bg.opacity * fade).clamp(0.0, 1.0);
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
        svg(svg::Handle::from_path(path)).content_fit(fit).width(Length::Fill).height(Length::Fill).opacity(opacity).into()
    } else {
        image(image::Handle::from_path(path)).content_fit(fit).width(Length::Fill).height(Length::Fill).opacity(opacity).into()
    }
}

/// Darkens each edge of the panel: one layer per edge, each a gradient from the vignette's color
/// at the edge to none at its reach.
pub fn vignette<'a, M: 'a>(v: &Vignette, fade: f32) -> Vec<Element<'a, M>> {
    let reach = (v.reach / 100.0).clamp(0.05, 0.5);
    let edge = color_of(&Paint::new(&v.color, v.opacity), [0, 0, 0], fade);
    let clear = Color { a: 0.0, ..edge };
    // Gradient angles run from the start edge: 180 is top to bottom, so it darkens the top.
    [180.0_f32, 0.0, 90.0, 270.0]
        .into_iter()
        .map(|angle| {
            let fill = gradient::Linear::new(angle.to_radians()).add_stop(0.0, edge).add_stop(reach, clear).add_stop(1.0, clear);
            container(space())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style { background: Some(Background::Gradient(fill.into())), ..container::Style::default() })
                .into()
        })
        .collect()
}

/// A line between two rows of a list: `width` thick at scale 1, across the panel's width.
pub fn divider<'a, M: 'a>(stroke: &Stroke, s: f32, fade: f32) -> Element<'a, M> {
    let color = color_of(&stroke.paint(), [0xff, 0xff, 0xff], fade);
    container(space())
        .width(Length::Fill)
        .height(Length::Fixed(stroke.width.max(0.5) * s))
        .style(move |_| container::Style { background: Some(color.into()), ..container::Style::default() })
        .into()
}

/// A flat wash of the tint's color, rounded `round` pixels at its corners.
pub fn tint<'a, M: 'a>(t: &Tint, round: f32, fade: f32) -> Element<'a, M> {
    let color = color_of(&Paint::new(&t.color, t.opacity), [0xff, 0xa0, 0x40], fade);
    let style = move |_: &iced::Theme| container::Style { background: Some(color.into()), border: iced::Border { radius: round.into(), ..iced::Border::default() }, ..container::Style::default() };
    container(space()).width(Length::Fill).height(Length::Fill).style(style).into()
}
