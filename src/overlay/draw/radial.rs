//! The radial menu: items as arcs of a circle sized by their weights, or as boxes at equal
//! angles around a hub, with the stick that aims it named in the middle.

use iced::{
    Border, Color, Element,
    widget::{container, pin, space, stack},
};

use super::*;
use crate::{config::MenuKind, menu::MenuView};

pub(super) fn radial<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    match m.kind {
        MenuKind::Radial { boxes: true, .. } => radial_boxes(m, c, s),
        _ => radial_arcs(m, c, s),
    }
}

/// The menu as a circle divided into arcs, one per item, sized by their weights.
fn radial_arcs<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let weights: Vec<f32> = m.items.iter().map(|item| item.weight).collect();
    let arcs = crate::radial::arcs(&weights);
    let hub = 48.0 * s;
    let inner = hub / 2.0 + 6.0 * s;
    let outer = arc_outer_radius(&arcs, inner, s);
    let size = 2.0 * outer + 16.0 * s;
    let center = size / 2.0;
    let point = |degrees: f32, r: f32| {
        let (sin, cos) = degrees.to_radians().sin_cos();
        (center + sin * r, center - cos * r)
    };
    let (cell_w, cell_h) = (120.0 * s, 44.0 * s);
    let mut layers = arc_layers(m, c, s, &arcs, (inner, outer));
    for (i, &(start, end)) in arcs.iter().enumerate() {
        // Each label sits in the middle of its arc.
        let (x, y) = point((start + end) / 2.0, (inner + outer) / 2.0);
        let (label, _, _) = item_cell(m, i, c, 15.0 * s);
        let content = container(label).center_x(cell_w).center_y(cell_h);
        layers.push(pin(content).x(x - cell_w / 2.0).y(y - cell_h / 2.0).into());
    }
    layers.push(radial_hub(m, c, s, hub, size));
    stack(layers).width(size).height(size).into()
}

/// The arcs, drawn as layers of SVG that rarely change as the menu moves: every arc in the
/// items' color in one, then the lit arcs in the selected color and the picked one in white, each
/// covering only its arc's box, faded in as far as it is lit and flashed. Only their opacity
/// animates, so each is rasterized once rather than every frame. The base stays one picture so the
/// software renderer, which draws only what changed and drops pictures it didn't draw, keeps it.
fn arc_layers<'a, M: 'a>(m: &MenuView, c: Colors, s: f32, arcs: &[(f32, f32)], (inner, outer): (f32, f32)) -> Vec<Element<'a, M>> {
    let size = 2.0 * outer + 16.0 * s;
    let center = size / 2.0;
    let shapes: Vec<String> = match arcs {
        // One item takes the whole circle, which an arc can't draw: a ring.
        [_] => vec![format!(
            r#"<circle cx="{center}" cy="{center}" r="{}" fill="none" stroke="FILL" stroke-opacity="ALPHA" stroke-width="{:.1}"/>"#,
            (inner + outer) / 2.0,
            outer - inner
        )],
        _ => arcs.iter().map(|&arc| format!(r#"<path d="{}" fill="FILL" fill-opacity="ALPHA"/>"#, arc_path(center, arc, (inner, outer)))).collect(),
    };
    // The lines between the arcs, in the panel's color, on top of the arcs on either side of them.
    let line = |degrees: f32| {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let (x0, y0) = (center + sin * (inner - 1.0), center - cos * (inner - 1.0));
        let (x1, y1) = (center + sin * (outer + 1.0), center - cos * (outer + 1.0));
        format!(
            r#"<path d="M{x0:.1} {y0:.1} L{x1:.1} {y1:.1}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
            hex(c.background),
            c.background.a,
            2.0 * s
        )
    };
    let lines = |of: &[(f32, f32)]| -> String { of.iter().filter(|_| arcs.len() > 1).flat_map(|&(a, b)| [line(a), line(b)]).collect() };
    let paint = |shape: &str, color: Color, alpha: f32| shape.replace("FILL", &hex(color)).replace("ALPHA", &format!("{alpha:.2}"));
    let picture = |body: String, [x, y, w, h]: [f32; 4], opacity: f32| -> Element<'a, M> {
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.1}" height="{h:.1}" viewBox="{x:.1} {y:.1} {w:.1} {h:.1}">{body}</svg>"#);
        let image = iced::widget::svg(iced::widget::svg::Handle::from_memory(svg.into_bytes())).width(w).height(h).opacity(opacity);
        pin(image).x(x).y(y).into()
    };
    let base: String = shapes.iter().enumerate().map(|(i, shape)| paint(shape, c.item, c.item.a * c.anim.item_fade(i))).collect();
    let mut layers = vec![picture(base + &lines(arcs), [0.0, 0.0, size, size], 1.0)];
    // One slot per arc for its highlight and one for the flash, filled or empty, so the number of
    // layers stays the same as the cursor moves: when it changes, the software renderer redraws
    // the whole panel, its shadow included, rather than the arcs that changed.
    let mut flash = space().into();
    for (i, shape) in shapes.iter().enumerate() {
        let bounds = if arcs.len() > 1 { arc_box(center, arcs[i], (inner, outer)) } else { [0.0, 0.0, size, size] };
        let fade = c.anim.item_fade(i);
        let edges = lines(&arcs[i..=i]);
        let lit = c.lit(m, i);
        layers.push(if lit > 0.0 { picture(paint(shape, c.selected, 1.0) + &edges, bounds, lit * c.selected.a * fade) } else { space().into() });
        let pick = if m.selected == Some(i) { c.anim.flash() * 0.5 } else { 0.0 };
        if pick > 0.0 {
            flash = picture(paint(shape, Color::WHITE, 1.0) + &edges, bounds, pick * c.item.a * fade);
        }
    }
    layers.push(flash);
    layers
}

/// The outline of the arc from `start` to `end` degrees between two radii around `center`.
fn arc_path(center: f32, (start, end): (f32, f32), (inner, outer): (f32, f32)) -> String {
    let at = |degrees: f32, r: f32| {
        let (sin, cos) = degrees.to_radians().sin_cos();
        format!("{:.1} {:.1}", center + sin * r, center - cos * r)
    };
    let large = u8::from(end - start > 180.0);
    format!(
        "M{} A{outer:.1} {outer:.1} 0 {large} 1 {} L{} A{inner:.1} {inner:.1} 0 {large} 0 {} Z",
        at(start, outer),
        at(end, outer),
        at(end, inner),
        at(start, inner),
    )
}

/// The box around an arc, a pixel larger all round: its ends, and the outermost points of the
/// circle it passes (every 90 degrees).
fn arc_box(center: f32, (start, end): (f32, f32), (inner, outer): (f32, f32)) -> [f32; 4] {
    let quarters = (-4..8).map(|q| q as f32 * 90.0).filter(|a| *a > start && *a < end);
    let angles: Vec<f32> = [start, end].into_iter().chain(quarters).collect();
    let points = angles.iter().flat_map(|&a| {
        let (sin, cos) = a.to_radians().sin_cos();
        [inner, outer].map(|r| (center + sin * r, center - cos * r))
    });
    let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in points {
        (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
    }
    [left - 1.0, top - 1.0, right - left + 2.0, bottom - top + 2.0]
}

/// The circle in the middle of a radial menu, naming the stick that aims it.
fn radial_hub<'a, M: 'a>(m: &MenuView, c: Colors, s: f32, hub: f32, size: f32) -> Element<'a, M> {
    // The middle names the stick that aims the menu.
    let stick = match m.kind {
        MenuKind::Radial {
            stick: crate::config::Stick::Left,
            ..
        } => "{ls}",
        _ => "{rs}",
    };
    let center = container(rich_text(stick, c, 16.0 * s, c.item_text))
        .center_x(hub)
        .center_y(hub)
        .style(move |_: &iced::Theme| container::Style {
            background: Some(c.item.into()),
            border: Border {
                width: 1.0,
                radius: (hub / 2.0).into(),
                color: Color {
                    a: 0.25,
                    ..c.item_text
                },
            },
            ..container::Style::default()
        });
    let center: Element<'a, M> = pin(center).x(size / 2.0 - hub / 2.0).y(size / 2.0 - hub / 2.0).into();
    // A dot just outside the hub shows where the stick points.
    let Some(degrees) = m.aim else { return center };
    let dot = 11.0 * s;
    let (sin, cos) = degrees.to_radians().sin_cos();
    let reach = hub / 2.0 + 10.0 * s;
    let marker = container(space()).width(dot).height(dot).style(move |_: &iced::Theme| container::Style {
        background: Some(c.selected_text.into()),
        border: Border { width: 1.0, radius: (dot / 2.0).into(), color: c.selected },
        ..container::Style::default()
    });
    let placed = pin(marker).x(size / 2.0 + sin * reach - dot / 2.0).y(size / 2.0 - cos * reach - dot / 2.0);
    stack![center, placed].width(size).height(size).into()
}

/// How far the arcs reach: far enough that the middle of each arc has room for its label,
/// up to a limit for very many items.
fn arc_outer_radius(arcs: &[(f32, f32)], inner: f32, s: f32) -> f32 {
    let label = 120.0 * s;
    let fits = |outer: f32| {
        let mid = (inner + outer) / 2.0;
        arcs.len() < 2
            || arcs
                .iter()
                .all(|&(a, b)| 2.0 * mid * ((b - a).to_radians() / 2.0).sin() >= label)
    };
    let mut outer = 170.0 * s;
    while !fits(outer) && outer < 420.0 * s {
        outer += 4.0 * s;
    }
    outer
}

/// `#rrggbb` for an SVG attribute.
fn hex(c: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8
    )
}

/// The menu as boxes at equal angles around a hub.
fn radial_boxes<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let (cell_w, cell_h) = (120.0 * s, 44.0 * s);
    let n = m.items.len().max(1) as f32;
    let radius = radial_radius(m.items.len(), 120.0, 44.0) * s;
    let size = 2.0 * radius + 120.0 * s;
    let mut layers: Vec<Element<'a, M>> = vec![space().width(size).height(size).into()];
    for i in 0..m.items.len() {
        // First item at the top, then clockwise.
        let angle = i as f32 / n * TAU;
        let (x, y) = (
            size / 2.0 + radius * angle.sin(),
            size / 2.0 - radius * angle.cos(),
        );
        let (label, lit, selected) = item_cell(m, i, c, 15.0 * s);
        let content = container(label)
            .center_x(cell_w)
            .center_y(cell_h)
            .style(lit_cell_style(lit, selected, cell_h));
        layers.push(pin(content).x(x - cell_w / 2.0).y(y - cell_h / 2.0).into());
    }
    layers.push(radial_hub(m, c, s, 48.0 * s, size));
    stack(layers).width(size).height(size).into()
}

/// The smallest ring (at least 150) on which neighbouring `w`×`h` cells don't touch.
fn radial_radius(n: usize, w: f32, h: f32) -> f32 {
    let gap = 8.0;
    let step = TAU / n.max(1) as f32;
    let overlaps = |r: f32| {
        (0..n).any(|i| {
            let (a, b) = (i as f32 * step, (i + 1) as f32 * step);
            let dx = (r * a.sin() - r * b.sin()).abs();
            let dy = (r * a.cos() - r * b.cos()).abs();
            n > 1 && dx < w + gap && dy < h + gap
        })
    };
    let mut r = 150.0;
    while overlaps(r) && r < 600.0 {
        r += 5.0;
    }
    r
}
