//! The radial menu: items as arcs of a circle sized by their weights, or as boxes at equal
//! angles around a hub, with the stick that aims it named in the middle.

use iced::{
    Border, Color, Element,
    widget::{container, pin, space, stack},
};

use super::*;
use crate::{config::MenuKind, menu::MenuView};

/// `c` is at full strength: the menu's fade is applied here.
pub(super) fn radial<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    match m.kind {
        MenuKind::Radial { boxes: true, .. } => radial_boxes(m, c.faded(c.anim.opacity()), s),
        _ => radial_arcs(m, c, s),
    }
}

/// The menu as a circle divided into arcs, one per item, sized by their weights.
fn radial_arcs<'a, M: 'a>(m: &MenuView, solid: Colors, s: f32) -> Element<'a, M> {
    let c = solid.faded(solid.anim.opacity());
    let weights: Vec<f32> = m.items.iter().map(|item| item.weight).collect();
    let arcs = crate::radial::arcs(&weights);
    let hub = 48.0 * s;
    let inner = hub / 2.0 + 6.0 * s;
    let shape = Ring { inner, outer: arc_outer_radius(&arcs, inner, s), hub, s, arcs };
    let (inner, outer, size, center) = (shape.inner, shape.outer, shape.size(), shape.size() / 2.0);
    let point = |degrees: f32, r: f32| {
        let (sin, cos) = degrees.to_radians().sin_cos();
        (center + sin * r, center - cos * r)
    };
    let (cell_w, cell_h) = (120.0 * s, 44.0 * s);
    let mut layers = vec![ring(m, solid, &shape)];
    for (i, &(start, end)) in shape.arcs.iter().enumerate() {
        // Each label sits in the middle of its arc.
        let (x, y) = point((start + end) / 2.0, (inner + outer) / 2.0);
        let (label, _, _) = item_cell(m, i, c, 15.0 * s);
        let content = container(label).center_x(cell_w).center_y(cell_h);
        layers.push(pin(content).x(x - cell_w / 2.0).y(y - cell_h / 2.0).into());
    }
    layers.push(radial_hub(m, c, s, (hub, size), false));
    stack(layers).width(size).height(size).into()
}

/// Where a radial menu's arcs go: their angles, the radii between which they're drawn, and the
/// hub's size, at scale `s`.
struct Ring {
    arcs: Vec<(f32, f32)>,
    inner: f32,
    outer: f32,
    hub: f32,
    s: f32,
}

impl Ring {
    /// The width and height of the square it's drawn in.
    fn size(&self) -> f32 {
        2.0 * self.outer + 16.0 * self.s
    }
}

/// The ring as one picture: the panel's color under it, every arc in the items' color, the lit ones blended toward the
/// selected color and the picked one flashed white, the lines between them, and the hub's disc.
/// It is drawn from `c` at full strength and faded as a whole, so its parts don't show through
/// each other as the menu opens and closes: a picture per part, each faded on its own, builds up
/// where they overlap, and the lit arc and the hub would show before the rest.
fn ring<'a, M: 'a>(m: &MenuView, c: Colors, shape: &Ring) -> Element<'a, M> {
    let Ring { ref arcs, inner, outer, hub, s } = *shape;
    let arcs = arcs.as_slice();
    let size = shape.size();
    let center = size / 2.0;
    let shape = |i: usize| match arcs {
        // One item takes the whole circle, which an arc can't draw: a ring.
        [_] => format!(
            r#"<circle cx="{center}" cy="{center}" r="{}" fill="none" stroke="FILL" stroke-opacity="ALPHA" stroke-width="{:.1}"/>"#,
            (inner + outer) / 2.0,
            outer - inner
        ),
        _ => format!(r#"<path d="{}" fill="FILL" fill-opacity="ALPHA"/>"#, arc_path(center, arcs[i], (inner, outer))),
    };
    let paint = |shape: &str, color: Color, alpha: f32| shape.replace("FILL", &hex(color)).replace("ALPHA", &format!("{:.3}", alpha.clamp(0.0, 1.0)));
    // The panel's color under it all, so the gap around the hub fades with the arcs rather than
    // ahead of them, over the panel alone.
    let mut body = format!(r#"<circle cx="{center}" cy="{center}" r="{outer:.1}" fill="{}" fill-opacity="{:.2}"/>"#, hex(c.background), c.background.a);
    for i in 0..arcs.len() {
        let shape = shape(i);
        let fade = c.anim.item_fade(i);
        body += &paint(&shape, c.item, c.item.a * fade);
        let lit = c.lit(m, i);
        if lit > 0.0 {
            body += &paint(&shape, c.selected, lit * c.selected.a * fade);
        }
        let flash = if m.selected == Some(i) { c.anim.flash() * 0.5 } else { 0.0 };
        if flash > 0.0 {
            body += &paint(&shape, Color::WHITE, flash * c.item.a * fade);
        }
    }
    // The lines between the arcs, in the panel's color, on top of the arcs on either side of them.
    for &(start, _) in arcs.iter().filter(|_| arcs.len() > 1) {
        let (sin, cos) = start.to_radians().sin_cos();
        let (x0, y0) = (center + sin * (inner - 1.0), center - cos * (inner - 1.0));
        let (x1, y1) = (center + sin * (outer + 1.0), center - cos * (outer + 1.0));
        body += &format!(
            r#"<path d="M{x0:.1} {y0:.1} L{x1:.1} {y1:.1}" stroke="{}" stroke-opacity="{:.2}" stroke-width="{:.1}"/>"#,
            hex(c.background),
            c.background.a,
            2.0 * s
        );
    }
    body += &format!(
        r#"<circle cx="{center}" cy="{center}" r="{:.1}" fill="{}" fill-opacity="{:.2}" stroke="{}" stroke-opacity="{:.2}" stroke-width="1"/>"#,
        hub / 2.0 - 0.5,
        hex(c.item),
        c.item.a,
        hex(c.item_text),
        0.25 * c.item_text.a
    );
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size:.1}" height="{size:.1}" viewBox="0 0 {size:.1} {size:.1}">{body}</svg>"#);
    iced::widget::svg(iced::widget::svg::Handle::from_memory(svg.into_bytes()))
        .width(size)
        .height(size)
        .opacity(c.anim.opacity())
        .into()
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

/// The circle in the middle of a radial menu, naming the stick that aims it. Without `disc`
/// only the name and the aim dot are drawn, over a disc drawn with the ring.
fn radial_hub<'a, M: 'a>(m: &MenuView, c: Colors, s: f32, (hub, size): (f32, f32), disc: bool) -> Element<'a, M> {
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
        .style(move |_: &iced::Theme| {
            if !disc {
                return container::Style::default();
            }
            container::Style {
                background: Some(c.item.into()),
                border: Border { width: 1.0, radius: (hub / 2.0).into(), color: c.item_text.scale_alpha(0.25) },
                ..container::Style::default()
            }
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
    layers.push(radial_hub(m, c, s, (48.0 * s, size), true));
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
