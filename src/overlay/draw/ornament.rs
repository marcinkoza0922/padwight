//! The ornaments drawn over and around a panel: a picture behind its contents, a frame at its
//! corners, scanlines, a vignette, and the lines between a list's items. Each is a layer of a
//! stack the size of the panel (see `framed`), except the dividers, which sit between rows.

use iced::{
    Background, Color, ContentFit, Element, Length,
    gradient,
    widget::{column, container, image, responsive, space, svg},
};

use crate::{
    assets,
    config::{BackgroundImage, Frame, FrameKind, ImageFit, Paint, Scanlines, Stroke, Tint, Vignette},
};

/// How far a scanline drifts per second, in pixels at scale 1.
const DRIFT_SPEED: f32 = 14.0;

/// Where a corner's mark sits: which edges it is on, as the mark's offset from its corner.
#[derive(Clone, Copy)]
struct Corner {
    /// The corner's point within the mark's box, 0 or the box's size.
    x: f32,
    y: f32,
    /// Which way the mark's arms run from the corner: into the panel.
    dx: f32,
    dy: f32,
}

const CORNERS: [Corner; 4] = [
    Corner { x: 0.0, y: 0.0, dx: 1.0, dy: 1.0 },
    Corner { x: 1.0, y: 0.0, dx: -1.0, dy: 1.0 },
    Corner { x: 0.0, y: 1.0, dx: 1.0, dy: -1.0 },
    Corner { x: 1.0, y: 1.0, dx: -1.0, dy: -1.0 },
];

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

/// A mark at each of the panel's four corners: brackets (an L along the edges) or a chamfer (a
/// line across the corner). `s` is the style's scale.
pub fn frame<'a, M: 'a>(f: &Frame, s: f32, fade: f32) -> Vec<Element<'a, M>> {
    let reach = f.size.max(1.0) * s;
    let [r, g, b] = Paint::new(&f.color, 1.0).rgb().unwrap_or([0xff, 0xff, 0xff]);
    let ink = format!("#{r:02x}{g:02x}{b:02x}");
    let opacity = (f.opacity * fade).clamp(0.0, 1.0);
    CORNERS
        .iter()
        .map(|c| {
            let (px, py) = (c.x * reach, c.y * reach);
            let (far_x, far_y) = (px + c.dx * reach, py + c.dy * reach);
            let shape = match f.kind {
                FrameKind::Brackets => format!("M{far_x} {py} L{px} {py} L{px} {far_y}"),
                FrameKind::Chamfer => format!("M{far_x} {py} L{px} {far_y}"),
            };
            let caps = if f.kind == FrameKind::Brackets { "square" } else { "round" };
            let markup = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {reach} {reach}\"><path d=\"{shape}\" fill=\"none\" stroke=\"{ink}\" stroke-opacity=\"{opacity}\" stroke-width=\"{}\" stroke-linecap=\"{caps}\"/></svg>",
                f.width.max(0.5) * s,
            );
            let mark = svg(svg::Handle::from_memory(markup.into_bytes())).width(reach).height(reach);
            let (horizontal, vertical) = (if c.x == 0.0 { iced::alignment::Horizontal::Left } else { iced::alignment::Horizontal::Right }, if c.y == 0.0 { iced::alignment::Vertical::Top } else { iced::alignment::Vertical::Bottom });
            container(mark).width(Length::Fill).height(Length::Fill).align_x(horizontal).align_y(vertical).into()
        })
        .collect()
}

/// Thin lines across the panel, `spacing` apart. They drift down when `drift` is set. The lines
/// fill whatever height the panel turns out to be, so they're laid out once that is known.
pub fn scanlines<'a, M: 'a>(sc: &Scanlines, s: f32, clock: f32, drift: bool, fade: f32) -> Element<'a, M> {
    let gap = sc.spacing.max(2.0) * s;
    let thickness = (gap * 0.35).max(1.0);
    let color = color_of(&Paint::new(&sc.color, sc.opacity), [0, 0, 0], fade);
    let shift = if drift { (clock * DRIFT_SPEED * s) % gap } else { 0.0 };
    responsive(move |size| {
        let count = (size.height / gap).ceil() as usize + 1;
        let mut lines = column![].spacing(gap - thickness).width(Length::Fill);
        for _ in 0..count {
            lines = lines.push(
                container(space())
                    .width(Length::Fill)
                    .height(Length::Fixed(thickness))
                    .style(move |_| container::Style { background: Some(color.into()), ..container::Style::default() }),
            );
        }
        container(lines).width(Length::Fill).height(Length::Fill).clip(true).padding(iced::Padding { top: shift, ..iced::Padding::ZERO }).into()
    })
    .into()
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

/// A flat wash of the tint's color over the panel.
pub fn tint<'a, M: 'a>(t: &Tint, fade: f32) -> Element<'a, M> {
    let color = color_of(&Paint::new(&t.color, t.opacity), [0xff, 0xa0, 0x40], fade);
    container(space()).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(color.into()), ..container::Style::default() }).into()
}
