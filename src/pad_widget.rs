//! The controller drawing as an iced widget, shared by the settings window and the debug window.

use iced::{
    Color, Element,
    widget::{container, pin, row, space, stack, svg, text, tooltip},
};

use crate::{
    info::{Glyphs, PadModel},
    ipc::InputSnapshot,
    pad_svg::{self, Spot},
};

/// The sign a [`pad_svg::GESTURE_MARK`] is drawn as.
const GESTURE_SIGN: &str = "+";

/// What the picture says beside and over its parts: a mapping label per part, and a tooltip for
/// the parts that do more than a label can say.
#[derive(Default, Clone, Copy)]
pub struct Callouts<'a> {
    pub labels: &'a [(Spot, String)],
    pub tips: &'a [(Spot, String)],
}

/// The controller SVG with button letters and mapping-label pills placed over it. (iced's
/// SVG renderer may not draw SVG text, so text is real widgets pinned at drawing
/// coordinates; the drawing is shown at 1:1.) `opacity` dims the controller alone: the leader
/// lines and labels stay at full strength. Pointing at a part with a tip, or at its label, shows
/// the tip.
pub fn controller_drawing<'a, M: 'a>(
    input: Option<&InputSnapshot>,
    model: Option<PadModel>,
    glyphs: Glyphs,
    Callouts { labels, tips }: Callouts<'_>,
    opacity: f32,
) -> Element<'a, M> {
    let tip_for = |spot: Spot| tips.iter().find(|(s, _)| *s == spot).map(|(_, t)| t.clone());
    let body = svg::Handle::from_memory(pad_svg::render(input, model, glyphs, &[]).into_bytes());
    let lines = svg::Handle::from_memory(pad_svg::leaders(labels, model).into_bytes());
    let mut layers: Vec<Element<'a, M>> = vec![
        svg(body).width(pad_svg::WIDTH).height(pad_svg::HEIGHT).opacity(opacity).into(),
        svg(lines).width(pad_svg::WIDTH).height(pad_svg::HEIGHT).into(),
    ];

    for o in pad_svg::overlays(input, model, glyphs) {
        let [r, g, b] = o.color;
        // Light gray letters (LT, RT) are drawn for the dark controller body. Dimmed in light
        // mode the body turns pale, so there they take a dark gray to stay readable.
        let pale = r.min(g).min(b) >= 0xc0 && r.max(g).max(b) - r.min(g).min(b) < 0x20;
        let dimmed = opacity < 1.0;
        let letter = text(o.text).size(12).style(move |theme: &iced::Theme| {
            let light_page = !theme.extended_palette().is_dark;
            let color = if pale && dimmed && light_page { Color::from_rgb8(0x5a, 0x5f, 0x66) } else { Color::from_rgb8(r, g, b) };
            text::Style { color: Some(color) }
        });
        let letter = container(letter)
            .center_x(24)
            .center_y(20);
        layers.push(pin(letter).x(o.x - 12.0).y(o.y - 10.0).into());
    }
    for l in pad_svg::place_labels(labels, model) {
        // An input that does nothing is grayed, as the list below the picture shows it.
        let off = l.text.starts_with("Disabled");
        let (ink, edge) = if off { ([0x8a, 0x8f, 0x98], [0x5d, 0x64, 0x70]) } else { ([0xe6, 0xed, 0xf3], [0x4e, 0xa1, 0xff]) };
        let label = |s: String| text(s).size(pad_svg::LABEL_TEXT_SIZE).wrapping(text::Wrapping::None);
        // The gesture mark is drawn in blue, apart from the action, so it reads as a sign.
        let (words, mark) = match l.text.strip_suffix(pad_svg::GESTURE_MARK) {
            Some(words) => (words.to_string(), Some(label(GESTURE_SIGN.into()).color(Color::from_rgb8(0x4e, 0xa1, 0xff)))),
            None => (l.text, None),
        };
        let words = label(words).color(Color::from_rgb8(ink[0], ink[1], ink[2]));
        let pill = container(row![words].extend(mark.map(Element::from)).spacing(3))
            .padding([2, 6])
            .style(move |_: &iced::Theme| container::Style {
                background: Some(Color::from_rgb8(0x1d, 0x20, 0x26).into()),
                border: iced::Border { width: 1.0, radius: 5.0.into(), color: Color::from_rgb8(edge[0], edge[1], edge[2]) },
                ..container::Style::default()
            });
        let pill = with_tip(pill, tip_for(l.spot));
        // Each column is a box the width of the margin; pills hug its inner edge.
        let column = container(pill).width(pad_svg::LABEL_COLUMN);
        let (column, x) = if l.right {
            (column.align_x(iced::alignment::Horizontal::Left), pad_svg::RIGHT_COLUMN_X)
        } else {
            (column.align_x(iced::alignment::Horizontal::Right), 0.0)
        };
        layers.push(pin(column).x(x).y(l.y).into());
    }
    for (spot, tip) in tips {
        let Some((x, y, w, h)) = pad_svg::area(model, *spot) else { continue };
        layers.push(pin(with_tip(space().width(w).height(h), Some(tip.clone()))).x(x + pad_svg::MARGIN).y(y).into());
    }
    stack(layers).width(pad_svg::WIDTH).height(pad_svg::HEIGHT).into()
}

/// `content` with `tip` shown in a bubble while the pointer is on it.
fn with_tip<'a, M: 'a>(content: impl Into<Element<'a, M>>, tip: Option<String>) -> Element<'a, M> {
    match tip {
        Some(tip) => tooltip(content, container(text(tip).size(13)).padding(8).style(crate::style::tooltip), tooltip::Position::Bottom).into(),
        None => content.into(),
    }
}
