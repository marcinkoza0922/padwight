//! The style editor's rows for an overlay's typography, ornaments and background picture: how
//! its text is set, the lines and frames over and around its panel, and a picture behind it.

use std::rc::Rc;

use super::{style_editor::{OnStyle, Slot, optional, paint_row, stepped_slider, stroke_rows}, *};
use crate::config::{BackgroundImage, Case, Frame, FrameKind, ImageFit, Scanlines, Stroke, Typography, Vignette};
use iced::widget::{button, checkbox, column, row};

/// How the text is set: its case, letter spacing, size, weight, slant and color.
pub(super) fn typography_rows<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
    let t = style.typography.clone();
    let set = set_field(on_change.clone(), style, |s: &mut OverlayStyle, t: Typography| s.typography = t);
    let case = |label: &'static str, case: Case| {
        button(text(label).size(13))
            .style(if t.case == case { button::primary } else { style::secondary })
            .on_press(set(Typography { case, ..t.clone() }))
    };
    let spacing = {
        let (t, set) = (t.clone(), set.clone());
        stepped_slider(0.0..=12.0, t.letter_spacing, format!("{:.0} px", t.letter_spacing), move |v| set(Typography { letter_spacing: v.round(), ..t.clone() }))
    };
    let size = {
        let (t, set) = (t.clone(), set.clone());
        stepped_slider(50.0..=200.0, t.size * 100.0, format!("{:.0}%", t.size * 100.0), move |v| set(Typography { size: (v / 5.0).round() * 0.05, ..t.clone() }))
    };
    let bold = {
        let (t, set) = (t.clone(), set.clone());
        checkbox(t.bold).label("Bold").on_toggle(move |on| set(Typography { bold: on, ..t.clone() }))
    };
    let italic = {
        let (t, set) = (t.clone(), set.clone());
        checkbox(t.italic).label("Italic").on_toggle(move |on| set(Typography { italic: on, ..t.clone() }))
    };
    let color = optional(
        &Slot { label: "Color for all text", get: |s| s.typography.color.as_ref(), put: |s, v| s.typography.color = v, start: |_| "#ffffff".to_string() },
        |hex, set| vec![labeled("        Color", paint_row(&Paint::new(hex, 1.0), false, &(Rc::new(move |p: Paint| set(p.color)) as Rc<dyn Fn(Paint) -> Message + 'a>)))],
        style,
        on_change,
    );
    column![
        text("    Type").size(14).color(MUTED_COLOR),
        labeled("        Case", row![case("As typed", Case::None), case("CAPITALS", Case::Upper), case("lower", Case::Lower)].spacing(4).into()),
        labeled("        Spacing", spacing),
        labeled("        Size", size),
        labeled("        Style", row![bold, italic].spacing(16).into()),
        color,
    ]
    .spacing(8)
    .into()
}

/// The panel's picture, its lines between items, its corner frame, scanlines and vignette.
pub(super) fn ornament_rows<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
    let picture = optional(
        &Slot {
            label: "Background picture",
            get: |s| s.background_image.as_ref(),
            put: |s, v| s.background_image = v,
            start: |_| BackgroundImage { file: String::new(), opacity: 1.0, fit: ImageFit::Cover },
        },
        |p, set| picture_rows(p, &set),
        style,
        on_change,
    );
    let dividers = optional(
        &Slot { label: "Lines between items", get: |s| s.ornament.dividers.as_ref(), put: |s, v| s.ornament.dividers = v, start: |_| Stroke { width: 1.0, ..Stroke::default() } },
        |stroke, set| stroke_rows(stroke, 0.0..=6.0, &set),
        style,
        on_change,
    );
    let frame = optional(
        &Slot {
            label: "Corner frame",
            get: |s| s.ornament.frame.as_ref(),
            put: |s, v| s.ornament.frame = v,
            start: |_| Frame { kind: FrameKind::Brackets, size: 14.0, width: 2.0, color: "#ffffff".into(), opacity: 1.0 },
        },
        |f, set| frame_rows(f, &set),
        style,
        on_change,
    );
    let scanlines = optional(
        &Slot {
            label: "Scanlines",
            get: |s| s.ornament.scanlines.as_ref(),
            put: |s, v| s.ornament.scanlines = v,
            start: |_| Scanlines { spacing: 4.0, opacity: 0.2, color: "#000000".into(), drift: false },
        },
        |l, set| scanline_rows(l, &set),
        style,
        on_change,
    );
    let vignette = optional(
        &Slot {
            label: "Vignette",
            get: |s| s.ornament.vignette.as_ref(),
            put: |s, v| s.ornament.vignette = v,
            start: |_| Vignette { color: "#000000".into(), opacity: 0.6, reach: 25.0 },
        },
        |v, set| vignette_rows(v, &set),
        style,
        on_change,
    );
    column![
        text("    Ornaments").size(14).color(MUTED_COLOR),
        picture,
        dividers,
        frame,
        scanlines,
        vignette,
    ]
    .spacing(8)
    .into()
}

/// The picture's file (a name in the backgrounds folder, or a path), how far it shows and how it
/// fills the panel.
fn picture_rows<'a>(picture: &BackgroundImage, set: &Rc<dyn Fn(BackgroundImage) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let file = {
        let (p, set) = (picture.clone(), set.clone());
        field("file name or path", &picture.file).on_input(move |file| set(BackgroundImage { file, ..p.clone() })).width(220)
    };
    let fit = |label: &'static str, fit: ImageFit| {
        let (p, set) = (picture.clone(), set.clone());
        button(text(label).size(13)).style(if picture.fit == fit { button::primary } else { style::secondary }).on_press_with(move || set(BackgroundImage { fit, ..p.clone() }))
    };
    let opacity = {
        let (p, set) = (picture.clone(), set.clone());
        percent_of(picture.opacity, move |v| set(BackgroundImage { opacity: v, ..p.clone() }))
    };
    vec![
        labeled("        File", file.into()),
        labeled("        Fit", row![fit("Cover", ImageFit::Cover), fit("Contain", ImageFit::Contain), fit("Stretch", ImageFit::Stretch)].spacing(4).into()),
        labeled("        Opacity", opacity),
    ]
}

/// The frame's shape, size, line width and color.
fn frame_rows<'a>(frame: &Frame, set: &Rc<dyn Fn(Frame) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let shape = |label: &'static str, kind: FrameKind| {
        let (f, set) = (frame.clone(), set.clone());
        button(text(label).size(13)).style(if frame.kind == kind { button::primary } else { style::secondary }).on_press_with(move || set(Frame { kind, ..f.clone() }))
    };
    let size = {
        let (f, set) = (frame.clone(), set.clone());
        stepped_slider(4.0..=40.0, frame.size, format!("{:.0} px", frame.size), move |v| set(Frame { size: v.round(), ..f.clone() }))
    };
    let width = {
        let (f, set) = (frame.clone(), set.clone());
        stepped_slider(1.0..=8.0, frame.width, format!("{:.0} px", frame.width), move |v| set(Frame { width: v.round(), ..f.clone() }))
    };
    let color = {
        let (f, set) = (frame.clone(), set.clone());
        paint_row(&frame_paint(frame), true, &(Rc::new(move |p: Paint| set(Frame { color: p.color, opacity: p.opacity, ..f.clone() })) as Rc<dyn Fn(Paint) -> Message + 'a>))
    };
    vec![
        labeled("        Shape", row![shape("Brackets", FrameKind::Brackets), shape("Chamfer", FrameKind::Chamfer)].spacing(4).into()),
        labeled("        Size", size),
        labeled("        Width", width),
        labeled("        Color", color),
    ]
}

fn frame_paint(frame: &Frame) -> Paint {
    Paint::new(&frame.color, frame.opacity)
}

/// The scanlines' spacing, opacity, color and whether they drift.
fn scanline_rows<'a>(lines: &Scanlines, set: &Rc<dyn Fn(Scanlines) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let spacing = {
        let (l, set) = (lines.clone(), set.clone());
        stepped_slider(2.0..=12.0, lines.spacing, format!("{:.0} px", lines.spacing), move |v| set(Scanlines { spacing: v.round(), ..l.clone() }))
    };
    let opacity = {
        let (l, set) = (lines.clone(), set.clone());
        percent_of(lines.opacity, move |v| set(Scanlines { opacity: v, ..l.clone() }))
    };
    let color = {
        let (l, set) = (lines.clone(), set.clone());
        paint_row(&Paint::new(&lines.color, 1.0), false, &(Rc::new(move |p: Paint| set(Scanlines { color: p.color, ..l.clone() })) as Rc<dyn Fn(Paint) -> Message + 'a>))
    };
    let drift = {
        let (l, set) = (lines.clone(), set.clone());
        checkbox(lines.drift).label("Drift down, slowly").on_toggle(move |on| set(Scanlines { drift: on, ..l.clone() }))
    };
    vec![
        labeled("        Spacing", spacing),
        labeled("        Opacity", opacity),
        labeled("        Color", color),
        labeled("", drift.into()),
    ]
}

/// The vignette's color, opacity and how far in from the edges it reaches.
fn vignette_rows<'a>(vignette: &Vignette, set: &Rc<dyn Fn(Vignette) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let reach = {
        let (v, set) = (vignette.clone(), set.clone());
        stepped_slider(5.0..=50.0, vignette.reach, format!("{:.0}%", vignette.reach), move |r| set(Vignette { reach: r.round(), ..v.clone() }))
    };
    let color = {
        let (v, set) = (vignette.clone(), set.clone());
        paint_row(&Paint::new(&vignette.color, vignette.opacity), true, &(Rc::new(move |p: Paint| set(Vignette { color: p.color, opacity: p.opacity, ..v.clone() })) as Rc<dyn Fn(Paint) -> Message + 'a>))
    };
    vec![labeled("        Reach", reach), labeled("        Color", color)]
}

/// A 0 to 1 value as a percentage slider.
fn percent_of<'a>(value: f32, on_change: impl Fn(f32) -> Message + 'a) -> Element<'a, Message> {
    stepped_slider(0.0..=100.0, value * 100.0, format!("{:.0}%", value * 100.0), move |v| on_change((v / 100.0).clamp(0.0, 1.0)))
}
