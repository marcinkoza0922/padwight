//! Where panels sit on the full-screen surface: at their style's spot, or side by side.

use iced::{
    Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget::{container, row},
};

use super::{EDGE_MARGIN, Fit, transform};
use crate::{config::OverlayStyle, motion::Anim, overlay::render::{Margins, margins}};

/// Positions an overlay panel on the full-screen surface, moved by its animation and by the
/// style's offset.
pub fn place<'a, M: 'a>(panel: Element<'a, M>, style: &OverlayStyle, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let (col, row) = style.position.cell();
    let horizontal = [Horizontal::Left, Horizontal::Center, Horizontal::Right][col];
    let vertical = [Vertical::Top, Vertical::Center, Vertical::Bottom][row];
    let (x, y) = anim.offset();
    let Margins { top, right, bottom, left } = margins(style.position, (x + fit.x, y + fit.y));
    container(transform(panel, anim.scale(), iced::Vector::ZERO))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(horizontal)
        .align_y(vertical)
        .padding(Padding { top, right, bottom, left })
        .into()
}

/// A centered panel with the stacks of panels at the middle of the screen's left and right
/// edges beside it, their tops in a line and the three centered on the screen as one, as the
/// Guide overlay draws its Notes, menu and Mappings. The panel moves by its animation.
pub fn beside<'a, M: 'a>(left: Option<Element<'a, M>>, panel: Element<'a, M>, right: Option<Element<'a, M>>, anim: &Anim) -> Element<'a, M> {
    let (x, y) = anim.offset();
    let side = |stack: Option<Element<'a, M>>, align: Horizontal, padding: Padding| {
        container(stack.unwrap_or_else(|| iced::widget::space().into())).width(Length::Fill).align_x(align).padding(padding)
    };
    let line = row![
        side(left, Horizontal::Left, Padding { left: EDGE_MARGIN, ..Padding::ZERO }),
        transform(panel, anim.scale(), iced::Vector::new(x, y)),
        side(right, Horizontal::Right, Padding { right: EDGE_MARGIN, ..Padding::ZERO }),
    ]
    .align_y(Vertical::Top);
    container(line).width(Length::Fill).height(Length::Fill).align_y(Vertical::Center).into()
}
