//! A look's decorations in Edit Controls: the panel's outline, shadow and gradient, the items'
//! outline, the selected item's glow, and the ornaments (dividers, a frame, scanlines and a
//! vignette). Each is off until turned on, and starts as the settings window starts it.

use crate::config::{Config, FilterOn, Frame, FrameKind, Gradient, OverlayStyle, Paint, Scanlines, Shade, Stroke, Vignette};

use super::{
    pages::Page,
    rows::{Number, Row, set},
    style::{PaintOf, StyleAt, paint_name, style, style_mut},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decor {
    Border,
    Shadow,
    Gradient,
    ItemBorder,
    Glow,
    Dividers,
    Frame,
    Scanlines,
    Vignette,
}

const DECORS: [Decor; 9] = [Decor::Border, Decor::Shadow, Decor::Gradient, Decor::ItemBorder, Decor::Glow, Decor::Dividers, Decor::Frame, Decor::Scanlines, Decor::Vignette];

impl Decor {
    pub fn name(self) -> &'static str {
        match self {
            Decor::Border => "Panel outline",
            Decor::Shadow => "Panel shadow",
            Decor::Gradient => "Gradient",
            Decor::ItemBorder => "Item outline",
            Decor::Glow => "Glow on the selected item",
            Decor::Dividers => "Lines between items",
            Decor::Frame => "Corner frame",
            Decor::Scanlines => "Scanlines",
            Decor::Vignette => "Vignette",
        }
    }

    fn is_on(self, s: &OverlayStyle) -> bool {
        match self {
            Decor::Border => s.border.is_some(),
            Decor::Shadow => s.shadow.is_some(),
            Decor::Gradient => s.gradient.is_some(),
            Decor::ItemBorder => s.item_border.is_some(),
            Decor::Glow => s.glow.is_some(),
            Decor::Dividers => s.ornament.dividers.is_some(),
            Decor::Frame => s.ornament.frame.is_some(),
            Decor::Scanlines => s.ornament.scanlines.is_some(),
            Decor::Vignette => s.ornament.vignette.is_some(),
        }
    }

    /// Turns it on, as the settings window starts it, or off.
    fn turn(self, s: &mut OverlayStyle, on: bool) -> bool {
        if self.is_on(s) == on {
            return false;
        }
        match self {
            Decor::Border => s.border = on.then(Stroke::default),
            Decor::Shadow => s.shadow = on.then(Shade::panel),
            Decor::Gradient => s.gradient = on.then(Gradient::default),
            Decor::ItemBorder => s.item_border = on.then_some(Stroke { width: 1.0, ..Stroke::default() }),
            Decor::Glow => s.glow = on.then(|| Shade::glow(&s.selected.color)),
            Decor::Dividers => s.ornament.dividers = on.then_some(Stroke { width: 1.0, ..Stroke::default() }),
            Decor::Frame => s.ornament.frame = on.then(|| Frame { kind: FrameKind::Brackets, size: 14.0, width: 2.0, color: "#ffffff".into(), opacity: 1.0 }),
            Decor::Scanlines => s.ornament.scanlines = on.then(|| Scanlines { spacing: 4.0, opacity: 0.2, color: "#000000".into(), drift: false, on: FilterOn::Both }),
            Decor::Vignette => s.ornament.vignette = on.then(|| Vignette { color: "#000000".into(), opacity: 0.6, reach: 25.0 }),
        }
        true
    }
}

/// The color of decoration `d`, while it's on.
pub(super) fn decor_paint(s: &OverlayStyle, d: Decor) -> Option<Paint> {
    match d {
        Decor::Border => s.border.as_ref().map(Stroke::paint),
        Decor::ItemBorder => s.item_border.as_ref().map(Stroke::paint),
        Decor::Dividers => s.ornament.dividers.as_ref().map(Stroke::paint),
        Decor::Shadow => s.shadow.as_ref().map(Shade::paint),
        Decor::Glow => s.glow.as_ref().map(Shade::paint),
        Decor::Gradient => s.gradient.as_ref().map(|g| Paint::new(&g.color, g.opacity)),
        Decor::Frame => s.ornament.frame.as_ref().map(|f| Paint::new(&f.color, f.opacity)),
        Decor::Scanlines => s.ornament.scanlines.as_ref().map(|l| Paint::new(&l.color, l.opacity)),
        Decor::Vignette => s.ornament.vignette.as_ref().map(|v| Paint::new(&v.color, v.opacity)),
    }
}

pub(super) fn set_decor_paint(s: &mut OverlayStyle, d: Decor, p: &Paint) -> bool {
    let both = |color: &mut String, opacity: &mut f32| set(color, p.color.clone()) | set(opacity, p.opacity);
    match d {
        Decor::Border => s.border.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::ItemBorder => s.item_border.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Dividers => s.ornament.dividers.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Shadow => s.shadow.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Glow => s.glow.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Gradient => s.gradient.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Frame => s.ornament.frame.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Scanlines => s.ornament.scanlines.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
        Decor::Vignette => s.ornament.vignette.as_mut().is_some_and(|x| both(&mut x.color, &mut x.opacity)),
    }
}

/// The outline or line that decoration `d` is, if it is one and is on.
fn stroke(s: &OverlayStyle, d: Decor) -> Option<&Stroke> {
    match d {
        Decor::Border => s.border.as_ref(),
        Decor::ItemBorder => s.item_border.as_ref(),
        Decor::Dividers => s.ornament.dividers.as_ref(),
        _ => None,
    }
}

fn stroke_mut(s: &mut OverlayStyle, d: Decor) -> Option<&mut Stroke> {
    match d {
        Decor::Border => s.border.as_mut(),
        Decor::ItemBorder => s.item_border.as_mut(),
        Decor::Dividers => s.ornament.dividers.as_mut(),
        _ => None,
    }
}

/// Every decoration, with whether it's on.
pub(super) fn list_rows(config: &Config, at: StyleAt) -> Vec<Row> {
    let Some(s) = style(config, at) else { return Vec::new() };
    // Info, log and media overlays and name labels have no items to outline, light up or separate.
    let text_only = matches!(at, StyleAt::Info(_) | StyleAt::Log(_) | StyleAt::Indicator(..) | StyleAt::Game(super::style::Look::Media));
    DECORS.into_iter().filter(|d| !(text_only && matches!(d, Decor::ItemBorder | Decor::Glow | Decor::Dividers))).map(|d| Row::open(format!("{}: {}", d.name(), if d.is_on(s) { "on" } else { "off" }), Page::Decor(at, d))).collect()
}

const STROKE: Number = Number::new(1.0, (0.0, 8.0), 0, " px");
const BLUR: Number = Number::new(5.0, (0.0, 80.0), 0, " px");
const DROP: Number = Number::new(1.0, (-30.0, 30.0), 0, " px");
const ANGLE: Number = Number::new(15.0, (0.0, 345.0), 0, "°");
const FRAME_SIZE: Number = Number::new(1.0, (4.0, 80.0), 0, " px");
const FRAME_WIDTH: Number = Number::new(1.0, (1.0, 8.0), 0, " px");
const LINE_SPACING: Number = Number::new(1.0, (2.0, 12.0), 0, " px");
const REACH: Number = Number::new(1.0, (5.0, 50.0), 0, "%");

/// A number of a decoration, changed by `field`, which says whether it changed.
fn num(at: StyleAt, label: &str, value: f32, n: &Number, field: fn(&mut OverlayStyle, f32) -> bool) -> Row {
    super::style::num(at, label, value, n, field)
}

/// A decoration's page: turning it on or off, and while on, its settings and color.
pub(super) fn decor_rows(config: &Config, at: StyleAt, d: Decor) -> Vec<Row> {
    let Some(s) = style(config, at) else { return Vec::new() };
    let on = d.is_on(s);
    let mut rows = vec![Row::toggle("On", on, move |c, v| style_mut(c, at).is_some_and(|s| d.turn(s, v)))];
    if !on {
        return rows;
    }
    if let Some(paint) = decor_paint(s, d) {
        rows.push(Row::open(format!("Color: {}", paint_name(&paint)), Page::Paint(at, PaintOf::Decor(d))));
    }
    rows.extend(match d {
        Decor::Border | Decor::ItemBorder | Decor::Dividers => {
            let width = stroke(s, d).map_or(1.0, |x| x.width);
            vec![STROKE.row("Width", width, move |c, v| style_mut(c, at).and_then(|s| stroke_mut(s, d)).is_some_and(|x| set(&mut x.width, v)))]
        }
        Decor::Shadow => s.shadow.as_ref().map_or_else(Vec::new, |x| {
            vec![
                num(at, "Blur", x.blur, &BLUR, |s, v| s.shadow.as_mut().is_some_and(|x| set(&mut x.blur, v))),
                num(at, "Drop", x.y, &DROP, |s, v| s.shadow.as_mut().is_some_and(|x| set(&mut x.y, v))),
            ]
        }),
        Decor::Glow => s.glow.as_ref().map_or_else(Vec::new, |x| {
            vec![
                num(at, "Blur", x.blur, &BLUR, |s, v| s.glow.as_mut().is_some_and(|x| set(&mut x.blur, v))),
                Row::toggle("Pulse", x.pulse, move |c, v| style_mut(c, at).and_then(|s| s.glow.as_mut()).is_some_and(|x| set(&mut x.pulse, v))),
            ]
        }),
        Decor::Gradient => s.gradient.as_ref().map_or_else(Vec::new, |x| {
            vec![
                num(at, "Direction", x.angle, &ANGLE, |s, v| s.gradient.as_mut().is_some_and(|x| set(&mut x.angle, v))),
                Row::toggle("Drift, slowly swaying", x.drift, move |c, v| style_mut(c, at).and_then(|s| s.gradient.as_mut()).is_some_and(|x| set(&mut x.drift, v))),
            ]
        }),
        Decor::Frame => s.ornament.frame.as_ref().map_or_else(Vec::new, |x| {
            let kind = x.kind;
            vec![
                Row::step(format!("Shape: {}", kind.to_string().to_lowercase()), move |c, step| {
                    let at_kind = FrameKind::ALL.iter().position(|&k| k == kind).unwrap_or(0) as i32;
                    let next = FrameKind::ALL[(at_kind + step).rem_euclid(FrameKind::ALL.len() as i32) as usize];
                    style_mut(c, at).and_then(|s| s.ornament.frame.as_mut()).is_some_and(|x| {
                        x.size = next.natural_size();
                        set(&mut x.kind, next)
                    })
                }),
                num(at, "Size", x.size, &FRAME_SIZE, |s, v| s.ornament.frame.as_mut().is_some_and(|x| set(&mut x.size, v))),
                num(at, "Width", x.width, &FRAME_WIDTH, |s, v| s.ornament.frame.as_mut().is_some_and(|x| set(&mut x.width, v))),
            ]
        }),
        Decor::Scanlines => s.ornament.scanlines.as_ref().map_or_else(Vec::new, |x| {
            vec![
                num(at, "Spacing", x.spacing, &LINE_SPACING, |s, v| s.ornament.scanlines.as_mut().is_some_and(|x| set(&mut x.spacing, v))),
                Row::toggle("Drift down, slowly", x.drift, move |c, v| style_mut(c, at).and_then(|s| s.ornament.scanlines.as_mut()).is_some_and(|x| set(&mut x.drift, v))),
            ]
        }),
        Decor::Vignette => s.ornament.vignette.as_ref().map_or_else(Vec::new, |x| {
            vec![num(at, "Reach", x.reach, &REACH, |s, v| s.ornament.vignette.as_mut().is_some_and(|x| set(&mut x.reach, v)))]
        }),
    });
    rows
}
