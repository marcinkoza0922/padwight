//! The iced-free renderer's colors and look give the values this module's functions do.

use iced::{Background, Color, Font};

use super::{Colors, Look as IcedLook, item_fill, item_glow, item_outline, item_shadow, panel_style};
use crate::{
    config::{Gradient, OverlayStyle, Paint, Shade, Stroke},
    menu::Tone,
    overlay::render::{self as new, Rgba},
};

fn same(a: Rgba, c: Color) -> bool {
    (a.r, a.g, a.b, a.a) == (c.r, c.g, c.b, c.a)
}

fn iced(c: Rgba) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a: c.a }
}

fn outline_same(a: Option<new::Outline>, b: Option<(f32, Color)>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some((w, c))) => a.width == w && same(a.color, c),
        _ => false,
    }
}

fn shadow_same(a: new::Shadow, b: iced::Shadow) -> bool {
    same(a.color, b.color) && a.offset == (b.offset.x, b.offset.y) && a.blur == b.blur_radius
}

fn shadow_opt_same(a: Option<new::Shadow>, b: Option<iced::Shadow>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => shadow_same(a, b),
        _ => false,
    }
}

fn decorated() -> OverlayStyle {
    let mut style = OverlayStyle {
        scale: 1.5,
        corners: 0.7,
        background: Paint::new("#203040", 0.9),
        items: Paint::new("#e8e0b0", 0.8),
        border: Some(Stroke { width: 3.0, color: "#ff8800".into(), opacity: 0.7 }),
        shadow: Some(Shade { color: "#000010".into(), opacity: 0.5, blur: 30.0, x: 4.0, y: -8.0, pulse: false }),
        gradient: Some(Gradient { color: "#880044".into(), opacity: 0.6, angle: 70.0, drift: true }),
        item_border: Some(Stroke { width: 2.0, color: "#33ccff".into(), opacity: 0.9 }),
        glow: Some(Shade { color: "#ffee00".into(), opacity: 0.8, blur: 12.0, x: 0.0, y: 0.0, pulse: true }),
        ..OverlayStyle::default()
    };
    style.ornament.bevel = Some(crate::config::Bevel { depth: 0.8 });
    style.typography.letter_spacing = 2.0;
    style.typography.size = 1.2;
    style
}

fn styles() -> Vec<OverlayStyle> {
    let mut plain = decorated();
    plain.lite.plain = true;
    let mut calm = decorated();
    calm.lite.calm = true;
    let mut bevel_only = OverlayStyle::default();
    bevel_only.ornament.bevel = Some(crate::config::Bevel { depth: 0.5 });
    vec![OverlayStyle::default(), decorated(), plain, calm, bevel_only]
}

fn colors_same(n: &new::Style, c: &Colors) {
    let pairs = [
        (n.background, c.background),
        (n.background_text, c.background_text),
        (n.muted, c.muted),
        (n.item, c.item),
        (n.item_text, c.item_text),
        (n.selected, c.selected),
        (n.selected_text, c.selected_text),
    ];
    for (a, b) in pairs {
        assert!(same(a, b), "{a:?} vs {b:?}");
    }
    assert_eq!(n.corners, c.corners);
    assert_eq!(n.fade, c.fade);
    assert_eq!((n.typ.spacing, n.typ.sized(1.0)), (c.typ.spacing(), c.typ.size(1.0)));
    
}

fn look_same(n: &new::Look, l: &IcedLook) {
    assert!(outline_same(n.border, l.border));
    assert!(outline_same(n.item_border, l.item_border));
    assert!(shadow_opt_same(n.shadow, l.shadow));
    assert!(shadow_opt_same(n.glow, l.glow));
    assert_eq!(n.glow_pulse, l.glow_pulse);
    assert_eq!(n.bevel, l.bevel);
    assert_eq!(n.plain, l.plain);
    match (n.gradient, l.gradient) {
        (None, None) => {}
        (Some((c, a, d)), Some((ic, ia, id))) => assert!(same(c, ic) && a == ia && d == id),
        _ => panic!("gradient"),
    }
}

#[test]
fn the_resolved_style_matches() {
    for style in styles() {
        let old = super::super::colors(&style, Font::DEFAULT);
        let now = new::Style::of(&style, None);
        colors_same(&now, &old);
        look_same(&now.look, &old.look);
        for opacity in [0.0, 0.4, 1.0, 1.5] {
            let (n, o) = (now.faded(opacity), old.faded(opacity));
            colors_same(&n, &o);
            look_same(&n.look, &o.look);
        }
        for (tone, blind) in [(Tone::Normal, false), (Tone::Add, false), (Tone::Remove, false), (Tone::Note, false), (Tone::Add, true), (Tone::Remove, true)] {
            let (mut n, mut o) = (now, old);
            n.colorblind = blind;
            o.colorblind = blind;
            colors_same(&n.toned(tone), &super::super::toned(o, tone));
        }
        for (lit, flash, fade) in [(0.0, 0.0, 1.0), (0.5, 0.3, 0.6), (1.0, 1.0, 0.0)] {
            let mut o = old;
            let faded = |c: Color| Color { a: c.a * fade, ..c };
            o.item = faded(super::super::blend(super::super::blend(o.item, o.selected, lit), Color::WHITE, flash * 0.5));
            o.item_text = faded(super::super::blend(o.item_text, o.selected_text, lit));
            colors_same(&now.lit_item(lit, flash, fade), &o);
        }
    }
}

#[test]
fn paint_and_text_color_match() {
    for (hex, opacity) in [("#102030", 0.5), ("#ffffff", 1.0), ("#000000", 0.0), ("bad", 0.3), ("#e0e0a0", 2.0)] {
        let p = Paint::new(hex, opacity);
        assert!(same(new::paint(&p, [9, 8, 7]), super::super::paint(&p, [9, 8, 7])));
    }
    for c in [Rgba::WHITE, Rgba::BLACK, Rgba::from_rgb8(0x2f, 0x5d, 0xb0), Rgba::from_rgb8(0xe0, 0xe0, 0x90), Rgba::rgb(0.6, 0.6, 0.6)] {
        assert!(same(new::text_on(c), super::super::text_on(iced(c))));
    }
    let (a, b) = (Rgba::from_rgba8(10, 200, 90, 0.4), Rgba::from_rgb8(250, 3, 128));
    for t in [-1.0, 0.0, 0.3, 0.77, 1.0, 2.0] {
        assert!(same(new::blend(a, b, t), super::super::blend(iced(a), iced(b), t)));
    }
}

#[test]
fn item_decorations_match() {
    for style in styles() {
        let mut old = super::super::colors(&style, Font::DEFAULT);
        let now = new::Style::of(&style, None);
        old.anim.clock = 1.7;
        for (selected, lit) in [(false, 0.0), (true, 1.0), (false, 0.4), (true, 0.8)] {
            let (o, n) = (item_outline(&old, selected, lit, 5.0), now.look.item_outline(now.item_text, now.selected_text, selected, lit));
            assert!(n.width == o.width && same(n.color, o.color));
            let g = now.look.item_glow(lit, 1.7);
            assert!(shadow_same(g, item_glow(&old, lit)));
            let s = now.look.item_shadow(now.item, lit, 1.7);
            assert!(shadow_same(s, item_shadow(&old, lit)));
            match (now.look.item_fill(now.item, lit), item_fill(&old, lit)) {
                (None, Background::Color(c)) => assert!(same(now.item, c)),
                (Some((top, bottom)), Background::Gradient(iced::Gradient::Linear(l))) => {
                    assert!(same(top, l.stops[0].unwrap().color) && same(bottom, l.stops[1].unwrap().color));
                }
                _ => panic!("fill"),
            }
        }
    }
}

#[test]
fn the_panel_box_matches() {
    for style in styles() {
        let mut old = super::super::colors(&style, Font::DEFAULT);
        let now = new::Style::of(&style, None);
        old.anim.clock = 2.3;
        let boxed = panel_style(old)(&iced::Theme::Dark);
        assert_eq!(boxed.border.radius, (now.corners * new::PANEL_CORNER).into());
        let (w, c) = now.look.border.map_or((0.0, Rgba::TRANSPARENT), |o| (o.width, o.color));
        assert!(boxed.border.width == w && same(c, boxed.border.color));
        let shadow = now.look.panel_shadow(now.background.a);
        assert!(shadow_same(shadow.unwrap_or_default(), boxed.shadow));
        match (now.look.gradient_angle(2.3), boxed.background) {
            (None, Some(Background::Color(c))) => assert!(same(now.background, c)),
            (Some(angle), Some(Background::Gradient(iced::Gradient::Linear(l)))) => {
                assert_eq!(angle, l.angle.0);
                assert!(same(now.background, l.stops[0].unwrap().color));
            }
            _ => panic!("background"),
        }
    }
}
