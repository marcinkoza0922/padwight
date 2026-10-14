//! The picture beside a page of the in-game editor: a stick's, trigger's or gyro's pictures with
//! their captions, or the touchpad.

use iced::{
    Alignment, Element,
    widget::{column, svg, text},
};

use super::*;
use crate::{
    figures::Figure,
    touchpad_widget::{PadPicture, touchpad_picture},
};

/// The touchpad picture's width at scale 1.
const PAD_WIDTH: f32 = 240.0;

pub(super) fn figure<'a, M: 'a>(fig: &Figure, c: Colors, s: f32) -> Element<'a, M> {
    match fig {
        Figure::Touchpad { pad, menus, touches, click, height } => {
            let menus: Vec<&crate::config::Menu> = menus.iter().collect();
            let picture = PadPicture { settings: Some((pad, &menus)), touches, click: *click, height: *height, width: PAD_WIDTH * s };
            column![caption("Touchpad (live)", c, s), touchpad_picture(&picture)].spacing(4.0 * s).align_x(Alignment::Center).into()
        }
        _ => {
            let mut pics = column![].spacing(10.0 * s).align_x(Alignment::Center);
            for p in fig.pictures() {
                let image = svg(svg::Handle::from_memory(p.svg.into_bytes())).width(p.size.0 * s).height(p.size.1 * s);
                let mut one = column![caption(p.title, c, s), image].spacing(4.0 * s).align_x(Alignment::Center);
                if let Some(note) = p.note {
                    one = one.push(text(note).font(c.font).size(12.0 * s).color(c.muted));
                }
                pics = pics.push(one);
            }
            pics.into()
        }
    }
}

fn caption<'a, M: 'a>(title: &str, c: Colors, s: f32) -> Element<'a, M> {
    text(title.to_string()).font(c.font).size(13.0 * s).color(c.background_text).into()
}
