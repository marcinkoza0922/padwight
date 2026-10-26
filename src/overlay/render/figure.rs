//! The picture beside a page of the in-game editor (`draw/figure.rs`): a stick's, trigger's or
//! gyro's pictures with their captions and notes, or the live touchpad with its labels.
//!
//! The figures' SVG has no text in it, so captions, notes and the touchpad's labels are text
//! nodes of our own, laid over the drawing where iced pins its widgets.

use super::{Align, Fit, Length, Markup, Node, Op, Pin, Point, Rect, Rgba, Style, Svg, text::Spec};
use crate::{
    figures::Figure,
    touchpad_widget::{LABEL_RGB, LABEL_SIZE, Label, PadPicture},
};

/// The touchpad picture's width at scale 1.
const PAD_WIDTH: f32 = 240.0;

/// The picture beside a page, for a style at `scale` (the style's scale setting).
pub fn figure(fig: &Figure, style: &Style, scale: f32) -> Node {
    match fig {
        Figure::Touchpad { pad, menus, touches, click, height } => {
            let menus: Vec<&crate::config::Menu> = menus.iter().collect();
            let picture = PadPicture { settings: Some((pad, &menus)), touches, click: *click, height: *height, width: PAD_WIDTH * scale };
            Node::column([caption("Touchpad (live)", style, scale), touchpad(&picture)]).spacing(4.0 * scale).align_x(Align::Center)
        }
        _ => {
            let pics = fig.pictures().into_iter().map(|p| {
                let image = drawing(p.svg, p.size.0 * scale, p.size.1 * scale);
                let mut one = Node::column([caption(p.title, style, scale), image]).spacing(4.0 * scale).align_x(Align::Center);
                if let Some(note) = p.note {
                    one = one.push(label(note, 12.0 * scale, style, style.muted));
                }
                one
            });
            Node::column(pics).spacing(10.0 * scale).align_x(Align::Center)
        }
    }
}

/// The live touchpad: its drawing with each label centered in its box over it.
pub fn touchpad(picture: &PadPicture<'_>) -> Node {
    let (w, h) = picture.size();
    let mut node = drawing(picture.svg(), w, h);
    for l in picture.labels() {
        node = node.pin(Pin::At(Point::new(l.at.0, l.at.1)), pad_label(&l));
    }
    node
}

/// A label over the touchpad: wrapped to its box, centered in it, and cut off at its edges.
fn pad_label(l: &Label) -> Node {
    let [r, g, b] = LABEL_RGB;
    let color = if l.picked { Rgba::WHITE } else { Rgba::rgb(r, g, b) };
    let spec = Spec { align: super::text::Align::Center, ..Spec::new(l.text.clone(), LABEL_SIZE) };
    Node::container(Node::text(spec, color)).center_x(l.size.0.max(1.0)).center_y(l.size.1.max(1.0)).clip(super::Corners::ZERO)
}

/// An SVG drawn `w` by `h`.
fn drawing(svg: String, w: f32, h: f32) -> Node {
    let markup = Markup::new(svg);
    Node::leaf(move |rect: Rect, ops: &mut Vec<Op>| {
        ops.push(Op::Svg { svg: Svg::Markup(markup.clone()), rect, fit: Fit::Contain, opacity: 1.0 });
    })
    .width(w)
    .height(h)
}

fn caption(title: &str, style: &Style, scale: f32) -> Node {
    label(title.to_string(), 13.0 * scale, style, style.background_text)
}

fn label(text: String, size: f32, style: &Style, color: Rgba) -> Node {
    Node::text(Spec { family: style.font, ..Spec::new(text, size) }, color).width(Length::Shrink)
}

#[cfg(test)]
mod tests;
