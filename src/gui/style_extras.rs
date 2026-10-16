//! The style editor's rows for an overlay's typography, ornaments and background picture: how
//! its text is set, the lines and frames over and around its panel, and a picture behind it.

use std::rc::Rc;

use super::{style_editor::{OnStyle, Slot, optional, paint_row, stepped_slider, stroke_rows}, *};
use crate::config::{BackgroundImage, Bevel, Camo, CamoScheme, Case, Frame, FrameKind, Grain, ImageFit, Pixels, Scanlines, Stroke, Texture, TextureKind, Tint, Typography, Vignette};
use iced::widget::{button, checkbox, column, row};

/// How the text is set: its case, letter spacing and size.
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
    column![
        text("    Type").size(14).color(MUTED_COLOR),
        labeled("        Case", row![case("As typed", Case::None), case("CAPITALS", Case::Upper), case("lower", Case::Lower)].spacing(4).into()),
        labeled("        Spacing", spacing),
        labeled("        Size", size),
        // A hint at the size the editor's other notes use, under the controls it's about.
        labeled("", note("Bold, italic, underline and color go inside the text: {b}…{/b}, {i}…{/i}, {u}…{/u}, {color:red}…{/color}.").into()),
    ]
    .spacing(8)
    .into()
}

/// The panel's picture, its lines between items, its corner frame and vignette.
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
    column![text("    Ornaments").size(14).color(MUTED_COLOR), picture, bevel_rows(style, on_change), dividers, frame, vignette, filter_rows(style, on_change)]
    .spacing(8)
    .into()
}

/// The filters on the panel: scanlines, grain, pixel blocks, a paper or metal texture, camo and a
/// color wash.
fn filter_rows<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
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
    let grain = optional(
        &Slot { label: "Grain", get: |s| s.ornament.grain.as_ref(), put: |s, v| s.ornament.grain = v, start: |_| Grain { opacity: 0.15, size: 1.0, animated: false } },
        |g, set| grain_rows(g, &set),
        style,
        on_change,
    );
    let pixels = optional(
        &Slot { label: "Pixel blocks", get: |s| s.ornament.pixels.as_ref(), put: |s, v| s.ornament.pixels = v, start: |_| Pixels { size: 6.0, opacity: 0.25 } },
        |p, set| pixel_rows(p, &set),
        style,
        on_change,
    );
    let texture = optional(
        &Slot {
            label: "Paper or brushed metal",
            get: |s| s.ornament.texture.as_ref(),
            put: |s, v| s.ornament.texture = v,
            start: |_| Texture { kind: TextureKind::Paper, opacity: 0.5 },
        },
        |t, set| texture_rows(t, &set),
        style,
        on_change,
    );
    let camo = optional(
        &Slot { label: "Camo", get: |s| s.ornament.camo.as_ref(), put: |s, v| s.ornament.camo = v, start: |_| Camo { scheme: CamoScheme::Woodland, opacity: 0.3, size: 60.0 } },
        |c, set| camo_rows(c, &set),
        style,
        on_change,
    );
    let tint = optional(
        &Slot { label: "Color wash", get: |s| s.ornament.tint.as_ref(), put: |s, v| s.ornament.tint = v, start: |_| Tint { color: "#ffa040".into(), opacity: 0.12 } },
        |t, set| {
            let color = {
                let (t, set) = (t.clone(), set.clone());
                paint_row(&Paint::new(&t.color, t.opacity), true, &(Rc::new(move |p: Paint| set(Tint { color: p.color, opacity: p.opacity })) as Rc<dyn Fn(Paint) -> Message + 'a>))
            };
            vec![labeled("        Color", color)]
        },
        style,
        on_change,
    );
    column![text("    Filters").size(14).color(MUTED_COLOR), scanlines, grain, pixels, texture, camo, tint].spacing(8).into()
}

/// The camo's colors, blotch size and strength.
fn camo_rows<'a>(camo: &Camo, set: &Rc<dyn Fn(Camo) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let scheme = |label: &'static str, scheme: CamoScheme| {
        let (c, set) = (camo.clone(), set.clone());
        button(text(label).size(13)).style(if camo.scheme == scheme { button::primary } else { style::secondary }).on_press_with(move || set(Camo { scheme, ..c.clone() }))
    };
    let size = {
        let (c, set) = (camo.clone(), set.clone());
        stepped_slider(16.0..=200.0, camo.size, format!("{:.0} px", camo.size), move |v| set(Camo { size: v.round(), ..c.clone() }))
    };
    let strength = {
        let (c, set) = (camo.clone(), set.clone());
        percent_of(camo.opacity, move |v| set(Camo { opacity: v, ..c.clone() }))
    };
    let schemes = row![scheme("Woodland", CamoScheme::Woodland), scheme("Desert", CamoScheme::Desert), scheme("Urban", CamoScheme::Urban), scheme("Arctic", CamoScheme::Arctic)].spacing(4);
    vec![labeled("        Colors", schemes.into()), labeled("        Blotch size", size), labeled("        Strength", strength)]
}

/// The grain's strength, grain size and whether it flickers.
fn grain_rows<'a>(grain: &Grain, set: &Rc<dyn Fn(Grain) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let strength = {
        let (g, set) = (grain.clone(), set.clone());
        percent_of(grain.opacity, move |v| set(Grain { opacity: v, ..g.clone() }))
    };
    let size = {
        let (g, set) = (grain.clone(), set.clone());
        stepped_slider(1.0..=6.0, grain.size, format!("{:.0} px", grain.size), move |v| set(Grain { size: v.round(), ..g.clone() }))
    };
    let animated = {
        let (g, set) = (grain.clone(), set.clone());
        checkbox(grain.animated).label("Flicker").on_toggle(move |on| set(Grain { animated: on, ..g.clone() }))
    };
    vec![labeled("        Strength", strength), labeled("        Size", size), labeled("", animated.into())]
}

/// The pixel blocks' size and strength.
fn pixel_rows<'a>(pixels: &Pixels, set: &Rc<dyn Fn(Pixels) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let size = {
        let (p, set) = (pixels.clone(), set.clone());
        stepped_slider(2.0..=24.0, pixels.size, format!("{:.0} px", pixels.size), move |v| set(Pixels { size: v.round(), ..p.clone() }))
    };
    let strength = {
        let (p, set) = (pixels.clone(), set.clone());
        percent_of(pixels.opacity, move |v| set(Pixels { opacity: v, ..p.clone() }))
    };
    vec![labeled("        Block size", size), labeled("        Strength", strength)]
}

/// The 3D buttons.
fn bevel_rows<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
    optional(
        &Slot { label: "3D buttons", get: |s| s.ornament.bevel.as_ref(), put: |s, v| s.ornament.bevel = v, start: |_| Bevel { depth: 0.5 } },
        |b, set| {
            let depth = {
                let set = set.clone();
                percent_of(b.depth, move |v| set(Bevel { depth: v }))
            };
            vec![labeled("        Depth", depth)]
        },
        style,
        on_change,
    )
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

/// The texture's material and strength.
fn texture_rows<'a>(texture: &Texture, set: &Rc<dyn Fn(Texture) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let kind = |label: &'static str, kind: TextureKind| {
        let (t, set) = (texture.clone(), set.clone());
        button(text(label).size(13)).style(if texture.kind == kind { button::primary } else { style::secondary }).on_press_with(move || set(Texture { kind, ..t.clone() }))
    };
    let strength = {
        let (t, set) = (texture.clone(), set.clone());
        percent_of(texture.opacity, move |v| set(Texture { opacity: v, ..t.clone() }))
    };
    vec![
        labeled("        Material", row![kind("Paper", TextureKind::Paper), kind("Brushed metal", TextureKind::BrushedMetal)].spacing(4).into()),
        labeled("        Strength", strength),
    ]
}

/// The frame's shape, size, line width and color.
fn frame_rows<'a>(frame: &Frame, set: &Rc<dyn Fn(Frame) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let shape = {
        let (f, set) = (frame.clone(), set.clone());
        // A new shape brings its own size, at which its detail reads.
        dropdown(FrameKind::ALL, Some(frame.kind), move |kind: FrameKind| set(Frame { kind, size: kind.natural_size(), ..f.clone() }))
    };
    let size = {
        let (f, set) = (frame.clone(), set.clone());
        stepped_slider(4.0..=80.0, frame.size, format!("{:.0} px", frame.size), move |v| set(Frame { size: v.round(), ..f.clone() }))
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
        labeled("        Shape", shape.into()),
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
