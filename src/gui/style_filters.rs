//! The style editor's Filters group: a material under the panel's contents (paper, brushed
//! metal, concrete, cloth, denim, leather, steel, wood) and the layers over them (scanlines, grain, pixel blocks, camo, a color wash). Each is
//! a button that turns it on and off, and each that's on gets its editor under its name.

use std::rc::Rc;

use super::{style_editor::{OnStyle, Slot, paint_row, stepped_slider}, *};
use crate::config::{Camo, CamoScheme, Grain, Pixels, Scanlines, Texture, Tint};
use iced::widget::{button, checkbox, column, row};

/// The filters' buttons, by what they do, and the editors of those that are on.
pub(super) fn filter_rows<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
    let materials: [Slot<Texture>; 8] = [
        material("Paper", |s| s.ornament.paper.as_ref(), |s, v| s.ornament.paper = v),
        material("Brushed metal", |s| s.ornament.brushed_metal.as_ref(), |s, v| s.ornament.brushed_metal = v),
        material("Concrete", |s| s.ornament.concrete.as_ref(), |s, v| s.ornament.concrete = v),
        material("Cloth", |s| s.ornament.cloth.as_ref(), |s, v| s.ornament.cloth = v),
        material("Denim", |s| s.ornament.denim.as_ref(), |s, v| s.ornament.denim = v),
        material("Leather", |s| s.ornament.leather.as_ref(), |s, v| s.ornament.leather = v),
        material("Steel", |s| s.ornament.steel.as_ref(), |s, v| s.ornament.steel = v),
        material("Wood", |s| s.ornament.wood.as_ref(), |s, v| s.ornament.wood = v),
    ];
    let materials = materials.iter().map(|slot| filter(slot, |t, set| texture_rows(t, &set), style, on_change)).collect();
    let scanlines = filter(
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
    let grain = filter(
        &Slot { label: "Grain", get: |s| s.ornament.grain.as_ref(), put: |s, v| s.ornament.grain = v, start: |_| Grain { opacity: 0.15, size: 1.0, animated: false } },
        |g, set| grain_rows(g, &set),
        style,
        on_change,
    );
    let pixels = filter(
        &Slot { label: "Pixel blocks", get: |s| s.ornament.pixels.as_ref(), put: |s, v| s.ornament.pixels = v, start: |_| Pixels { size: 6.0, opacity: 0.25 } },
        |p, set| pixel_rows(p, &set),
        style,
        on_change,
    );
    let camo = filter(
        &Slot { label: "Camo", get: |s| s.ornament.camo.as_ref(), put: |s, v| s.ornament.camo = v, start: |_| Camo { scheme: CamoScheme::Woodland, opacity: 0.3, size: 60.0 } },
        |c, set| camo_rows(c, &set),
        style,
        on_change,
    );
    let tint = filter(
        &Slot { label: "Color wash", get: |s| s.ornament.tint.as_ref(), put: |s, v| s.ornament.tint = v, start: |_| Tint { color: "#ffa040".into(), opacity: 0.12 } },
        |t, set| tint_rows(t, &set),
        style,
        on_change,
    );
    grouped([("        Material", materials), ("        Screen", vec![scanlines, grain, pixels]), ("        Color", vec![camo, tint])])
}

/// A material's slot, starting at half strength.
fn material(label: &'static str, get: fn(&OverlayStyle) -> Option<&Texture>, put: fn(&mut OverlayStyle, Option<Texture>)) -> Slot<Texture> {
    Slot { label, get, put, start: |_| Texture { opacity: 0.5 } }
}

/// Each group's buttons on a row under its name, then the editors of the filters that are on.
fn grouped<'a>(groups: [(&'static str, Vec<Filter<'a>>); 3]) -> Element<'a, Message> {
    let mut rows = column![text("    Filters").size(14).color(MUTED_COLOR)].spacing(8);
    let mut editors = Vec::new();
    for (label, filters) in groups {
        let mut buttons = Vec::new();
        for Filter { button, editor } in filters {
            buttons.push(button);
            editors.extend(editor);
        }
        rows = rows.push(labeled(label, iced::widget::Row::with_children(buttons).spacing(4).wrap().vertical_spacing(4).into()));
    }
    rows = rows.push(labeled("", note("A material lies under the panel's contents, the other filters over them: keep those faint.").into()));
    rows.extend(editors).into()
}

/// A filter's button, and its editor while it's on.
struct Filter<'a> {
    button: Element<'a, Message>,
    editor: Option<Element<'a, Message>>,
}

/// The button that turns the filter in `slot` on and off, ticked while it's on, and while it's
/// on, its rows from `editor` under its name.
fn filter<'a, T: Clone + 'a>(
    slot: &Slot<T>,
    editor: impl Fn(&T, Rc<dyn Fn(T) -> Message + 'a>) -> Vec<Element<'a, Message>>,
    style: &OverlayStyle,
    on_change: &OnStyle<'a>,
) -> Filter<'a> {
    let set = set_field(on_change.clone(), style, slot.put);
    let value = (slot.get)(style);
    let (label, look): (String, fn(&iced::Theme, button::Status) -> button::Style) = match value {
        Some(_) => (format!("✓ {}", slot.label), button::primary),
        None => (slot.label.to_string(), style::secondary),
    };
    let toggled = set(value.is_none().then(|| (slot.start)(style)));
    let button = button(text(label).size(13)).style(look).on_press(toggled).into();
    // Its name over a rule, in the editors' column, sets each filter's rows apart.
    let heading = row![text(slot.label).size(13).color(MUTED_COLOR), iced::widget::rule::horizontal(1)].spacing(8).align_y(Alignment::Center).width(Length::Fixed(540.0));
    let heading = container(heading).padding(iced::Padding::ZERO.top(6));
    let editor = value.map(|v| column![labeled("", heading.into())].extend(editor(v, Rc::new(move |v| set(Some(v))))).spacing(6).into());
    Filter { button, editor }
}

/// The material's strength.
fn texture_rows<'a>(texture: &Texture, set: &Rc<dyn Fn(Texture) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let set = set.clone();
    vec![labeled("        Strength", percent_of(texture.opacity, move |v| set(Texture { opacity: v })))]
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

/// The wash's color and how strong it is.
fn tint_rows<'a>(tint: &Tint, set: &Rc<dyn Fn(Tint) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let set = set.clone();
    let color = paint_row(&Paint::new(&tint.color, tint.opacity), true, &(Rc::new(move |p: Paint| set(Tint { color: p.color, opacity: p.opacity })) as Rc<dyn Fn(Paint) -> Message + 'a>));
    vec![labeled("        Color", color)]
}

/// A 0 to 1 value as a percentage slider.
pub(super) fn percent_of<'a>(value: f32, on_change: impl Fn(f32) -> Message + 'a) -> Element<'a, Message> {
    stepped_slider(0.0..=100.0, value * 100.0, format!("{:.0}%", value * 100.0), move |v| on_change((v / 100.0).clamp(0.0, 1.0)))
}
