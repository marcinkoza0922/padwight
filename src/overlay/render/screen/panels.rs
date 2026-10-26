//! Each kind of panel built for the screen, in the style the overlay draws it in.

use super::Context;
use crate::{
    config::MenuKind,
    menu::MenuView,
    motion::Anim,
    offer::OfferView,
    overlay::{
        OverlayView,
        render::{self, Node, Style, UNDER_PROMPT},
    },
};

/// A menu, dimmed under the question asked over it. A radial menu's circle is framed like the
/// other kinds' bodies.
pub(super) fn menu(m: &MenuView, cx: &Context, anim: Anim) -> Node {
    let solid = render::menu_style(m, cx.font, anim, cx.glyphs, cx.colorblind);
    let fit = cx.fit(&m.style);
    render::menu_panel(m, &solid, &fit).unwrap_or_else(|| {
        let s = m.style.scale.clamp(0.5, 2.0);
        debug_assert!(matches!(m.kind, MenuKind::Radial { .. }));
        let dim = if m.prompt.is_some() { UNDER_PROMPT } else { 1.0 };
        let style: Style = solid.faded(anim.opacity() * dim);
        render::menu_frame(m, render::radial(m, &solid, s), &style, s, &fit)
    })
}

/// The question asked over a menu, or offered on its own.
pub(super) fn offer(o: &OfferView, cx: &Context, anim: &Anim) -> Node {
    render::offer(o, cx.font, cx.glyphs, &cx.fit(&o.style), anim)
}

/// A panel of any kind.
pub(super) fn any(view: &OverlayView, cx: &Context, anim: &Anim) -> Node {
    match view {
        OverlayView::Keyboard(k) => {
            let style = Style { anim: *anim, ..Style::of(&k.style, cx.font).with_glyphs(cx.glyphs) }.faded(anim.opacity());
            render::keyboard_panel(k, &style, &cx.fit(&k.style))
        }
        OverlayView::Menu(m) => menu(m, cx, *anim),
        OverlayView::Media(m) => render::media(m, cx.font, cx.glyphs, &cx.fit(&m.style), anim),
        OverlayView::Offer(o) => offer(o, cx, anim),
    }
}
