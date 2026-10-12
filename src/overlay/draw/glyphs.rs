//! Button glyphs, D-pads, stick presses and icons, for info overlays, logs and menu rows.

use iced::{
    Border, Color, Element,
    widget::{column, container, row, space, text},
};

use super::Colors;
use crate::{
    info::{Charge, FormFactor, Icon, PadFamily, Segment},
    media::PlayState,
};

/// One piece of an info overlay's line, or of a menu row's glyphs.
pub(super) fn segment_element<'a, M: 'a>(seg: &Segment, c: Colors, s: f32, opacity: f32) -> Element<'a, M> {
    match seg {
        Segment::Text(t) => Element::from(text(t.clone()).font(c.font).size(16.0 * s).color(c.background_text)),
        Segment::Glyph { label, fill, round } => glyph(label, *fill, *round, c, s, opacity),
        Segment::Dpad(lit) => dpad_glyph(*lit, c, s),
        Segment::StickClick { right } => stick_click_glyph(*right, c, s),
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

/// A stick press: a round stick cap marked L or R, with a down arrow for the push.
fn stick_click_glyph<'a, M: 'a>(right: bool, c: Colors, s: f32) -> Element<'a, M> {
    let size = 26.0 * s;
    let label = if right { "R" } else { "L" };
    let cap = container(row![
        text(label).font(c.font).size(12.0 * s).color(c.item_text),
        text("↓").font(c.font).size(12.0 * s).color(c.item_text),
    ])
    .center_x(size)
    .center_y(size);
    cap.style(move |_: &iced::Theme| container::Style {
        background: Some(c.item.into()),
        border: Border { width: 2.0, radius: (size / 2.0).into(), color: c.item_text.scale_alpha(0.6) },
        ..container::Style::default()
    })
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
    let hex = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    let [r, g, b, _] = ink.into_rgba8();
    let c = hex([r, g, b]);
    let body = match icon {
        Icon::Form(FormFactor::Desktop) => format!(
            "<rect x='2' y='3' width='20' height='13' rx='2' fill='none' stroke='{c}' stroke-width='2'/>\
             <path d='M8 21 H16 M12 16 V21' fill='none' stroke='{c}' stroke-width='2' stroke-linecap='round'/>"
        ),
        Icon::Form(FormFactor::Laptop) => format!(
            "<rect x='5' y='4' width='14' height='11' rx='1.5' fill='none' stroke='{c}' stroke-width='2'/>\
             <path d='M2 18 H22 L21 20.5 H3 Z' fill='{c}'/>"
        ),
        Icon::Form(FormFactor::Handheld) => format!(
            "<rect x='2' y='6' width='20' height='12' rx='4' fill='none' stroke='{c}' stroke-width='2'/>\
             <rect x='8.5' y='9' width='7' height='6' rx='1' fill='{c}'/>"
        ),
        Icon::Controller(family) => {
            let tint = match family {
                PadFamily::Xbox => hex([0x3c, 0xa0, 0x3c]),
                PadFamily::PlayStation => hex([0x5a, 0x8c, 0xdc]),
                PadFamily::Nintendo => c.clone(),
            };
            format!(
                "<path d='M7 7 H17 C20.5 7 22.5 9.5 22.5 13.5 C22.5 17.5 20.8 18.5 19.2 18.5 \
                 C17.8 18.5 16.9 17.2 15.9 15.8 H8.1 C7.1 17.2 6.2 18.5 4.8 18.5 \
                 C3.2 18.5 1.5 17.5 1.5 13.5 C1.5 9.5 3.5 7 7 7 Z' fill='{tint}'/>"
            )
        }
        Icon::Battery(charge) => battery_svg(*charge, &c),
        Icon::Wifi(percent) => wifi_svg(*percent, &c),
    };
    format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' opacity='{}'>{body}</svg>", ink.a)
}

/// A battery outline filled to its level; an estimate is faded.
fn battery_svg(charge: Charge, c: &str) -> String {
    let percent = charge.percent.min(100);
    let width = 14.0 * f32::from(percent) / 100.0;
    let level = match percent {
        50.. => [0x3c, 0xa0, 0x3c],
        20..50 => [0xc8, 0xa0, 0x1e],
        _ => [0xc8, 0x3c, 0x3c],
    };
    let level = format!("#{:02x}{:02x}{:02x}", level[0], level[1], level[2]);
    let opacity = if charge.estimated { 0.6 } else { 1.0 };
    format!(
        "<rect x='1.5' y='6.5' width='18' height='11' rx='2.5' fill='none' stroke='{c}' stroke-width='2'/>\
         <rect x='21' y='10' width='2' height='4' rx='1' fill='{c}'/>\
         <rect x='4' y='9' width='{width:.2}' height='6' rx='1' fill='{level}' fill-opacity='{opacity}'/>"
    )
}

/// Wi-Fi bars, lit as the signal passes each quarter.
fn wifi_svg(percent: u8, c: &str) -> String {
    let lit: u8 = match percent {
        0 => 0,
        1..=25 => 1,
        26..=50 => 2,
        51..=75 => 3,
        _ => 4,
    };
    (0u8..4)
        .map(|i| {
            let height = 5.0 + 4.0 * f32::from(i);
            let opacity = if i < lit { 1.0 } else { 0.25 };
            format!(
                "<rect x='{:.1}' y='{:.1}' width='3.5' height='{height:.1}' rx='1' fill='{c}' fill-opacity='{opacity}'/>",
                2.0 + 5.5 * f32::from(i),
                21.0 - height
            )
        })
        .collect()
}

/// A play, pause or stop symbol, drawn so it doesn't depend on the font having the glyph.
pub(super) fn state_icon<'a, M: 'a>(state: PlayState, color: Color, size: f32) -> Element<'a, M> {
    let [r, g, b, _] = color.into_rgba8();
    let shape = match state {
        PlayState::Playing => "<path d='M6 3 L21 12 L6 21 Z'/>",
        PlayState::Paused => "<rect x='5' y='3' width='5' height='18' rx='1'/><rect x='14' y='3' width='5' height='18' rx='1'/>",
        PlayState::Stopped => "<rect x='5' y='5' width='14' height='14' rx='2'/>",
    };
    let svg_text = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#{r:02x}{g:02x}{b:02x}' fill-opacity='{}'>{shape}</svg>", color.a);
    iced::widget::svg(iced::widget::svg::Handle::from_memory(svg_text.into_bytes())).width(size).height(size).into()
}
