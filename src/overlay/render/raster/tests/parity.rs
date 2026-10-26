//! The same boxes drawn by iced's tiny-skia renderer (through `snapshot::render`) and by ours,
//! compared pixel by pixel.

use std::f32::consts::FRAC_PI_4;

use iced::{
    Background, Border, Color, Element, Gradient, Length, Padding, Size, Vector,
    gradient,
    widget::{Space, container},
};

use super::{BACKGROUND, drawn, worst};
use crate::{
    overlay::render::{Corners, Fill, Linear, Op, Outline, Rect, Rgba, Shadow},
    snapshot::{Image, render},
};

fn color(c: Rgba) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a: c.a }
}

fn radius(c: Corners) -> iced::border::Radius {
    iced::border::Radius { top_left: c.top_left, top_right: c.top_right, bottom_right: c.bottom_right, bottom_left: c.bottom_left }
}

/// One box with its shadow, as iced draws a styled container.
struct Quad {
    rect: Rect,
    radii: Corners,
    fill: Fill,
    border: Outline,
    shadow: Shadow,
}

impl Quad {
    fn ops(&self) -> Vec<Op> {
        vec![
            Op::Shadow { rect: self.rect, radii: self.radii, shadow: self.shadow, spread: 0.0 },
            Op::Rect { rect: self.rect, radii: self.radii, fill: Some(self.fill.clone()), border: Some(self.border) },
        ]
    }

    fn iced(&self, size: Size, scale: f32) -> Image {
        let background = match &self.fill {
            Fill::Solid(c) => Background::Color(color(*c)),
            Fill::Linear(l) => {
                let linear = l.stops.iter().fold(gradient::Linear::new(l.angle), |g, s| g.add_stop(s.offset, color(s.color)));
                Background::Gradient(Gradient::Linear(linear))
            }
        };
        let style = container::Style {
            background: Some(background),
            border: Border { color: color(self.border.color), width: self.border.width, radius: radius(self.radii) },
            shadow: iced::Shadow { color: color(self.shadow.color), offset: Vector::new(self.shadow.offset.0, self.shadow.offset.1), blur_radius: self.shadow.blur },
            ..container::Style::default()
        };
        let quad = container(Space::new().width(Length::Fixed(self.rect.w)).height(Length::Fixed(self.rect.h))).style(move |_| style);
        let at = Padding { top: self.rect.y, left: self.rect.x, ..Padding::ZERO };
        let screen: Element<'_, ()> = container(quad).padding(at).width(Length::Fill).height(Length::Fill).into();
        let [r, g, b] = BACKGROUND;
        render(screen, size, scale, Color::from_rgb8(r, g, b)).unwrap()
    }
}

#[test]
fn a_shadowed_box_matches_iced_within_two_levels() {
    let quads = [
        Quad {
            rect: Rect::new(20.0, 16.0, 60.0, 40.0),
            radii: Corners::all(12.0),
            fill: Fill::Solid(Rgba::from_rgba8(0x30, 0x5d, 0xb0, 0.6)),
            border: Outline { width: 2.0, color: Rgba::from_rgba8(255, 255, 255, 0.4) },
            shadow: Shadow { color: Rgba::from_rgba8(0, 0, 0, 0.6), offset: (3.0, 5.0), blur: 10.0 },
        },
        Quad {
            rect: Rect::new(24.0, 20.0, 50.0, 30.0),
            radii: Corners { top_left: 2.0, top_right: 14.0, bottom_right: 0.0, bottom_left: 8.0 },
            fill: Fill::Linear(Linear::new(FRAC_PI_4).stop(0.0, Rgba::from_rgb8(0xe0, 0x60, 0x40)).stop(1.0, Rgba::from_rgba8(0x20, 0x40, 0x90, 0.8))),
            border: Outline { width: 1.0, color: Rgba::from_rgb8(0xf0, 0xe0, 0xc0) },
            shadow: Shadow { color: Rgba::from_rgba8(0x10, 0, 0x20, 0.8), offset: (-2.0, 4.0), blur: 16.0 },
        },
    ];
    let size = Size::new(110.0, 80.0);
    let mut report = Vec::new();
    for (i, quad) in quads.iter().enumerate() {
        for scale in [1.0, 1.5, 2.0] {
            let ours = drawn(&quad.ops(), size.width as u32, size.height as u32, scale);
            let theirs = quad.iced(size, scale);
            assert!(theirs.drawn(BACKGROUND) > 2000, "iced drew the box");
            let off = worst(&ours, &theirs);
            if off > 2 || std::env::var_os("PADWIGHT_BLESS").is_some() {
                let out = super::root().join("screenshots/render");
                ours.save(&out.join(format!("iced-{i}@{scale}-ours.png"))).unwrap();
                theirs.save(&out.join(format!("iced-{i}@{scale}-iced.png"))).unwrap();
            }
            report.push((i, scale, off));
        }
    }
    assert!(report.iter().all(|&(_, _, off)| off <= 2), "levels off iced (box, scale, levels): {report:?}");
}
