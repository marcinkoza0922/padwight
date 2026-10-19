//! The menus: their panels and the layouts of their items (a list, buttons, a D-pad or face
//! cluster, a carousel, a grid), with each item's label, icon and quick-select button.

use iced::{
    Alignment, Color, Element, Font, Length,
    widget::{column, container, row, space, text},
};

use super::*;

/// How much of a menu shows through under a question asked over it.
const UNDER_PROMPT: f32 = 0.35;

/// An item's glyphs (the same as the info overlays'), its label, and a ▸ for submenus, at the
/// row's right end when it `fills` one. A text badge (a stick's LS, say) is drawn as a small tag.
fn item_text<'a, M: 'a>(c: Colors, item: &crate::menu::ItemView, label: &str, size: f32, fills: bool) -> Element<'a, M> {
    let (font, fg) = (c.font, c.item_text);
    // Glyphs, badges and icons fade in and out with the label.
    let fade = fg.a;
    let mut line = row![].spacing(size * 0.5).align_y(Alignment::Center);
    // The glyphs are sized from the label, which is 17 units when the scale is 1.
    for b in &item.buttons {
        line = line.push(segment_element(&crate::info::button_glyph(*b, c.family, c.nintendo_layout), c, size / 17.0, fade));
    }
    if let Some(b) = item.button.as_deref().filter(|b| !b.is_empty()) {
        line = line.push(
            container(text(b.to_string()).font(font).size(size - 2.0).color(Color::BLACK.scale_alpha(fade)))
                .padding([1.0, size * 0.4])
                .style(move |_| container::Style {
                    background: Some(Color::from_rgb(0.85, 0.87, 0.9).scale_alpha(fade).into()),
                    border: Border { radius: 5.0.into(), ..Border::default() },
                    ..container::Style::default()
                }),
        );
    }
    if let Some(k) = item.keyword {
        let icon = iced::widget::svg::Handle::from_memory(crate::keyword_icon::svg(k).into_bytes());
        line = line.push(iced::widget::svg(icon).width(size).height(size).opacity(fade));
    }
    line = line.push(rich_text(label, c, size, fg));
    if item.submenu {
        if fills {
            line = line.push(space().width(Length::Fill));
        }
        line = line.push(text("▸").font(font).size(size).color(fg.scale_alpha(0.7)));
    }
    line.into()
}

pub fn menu_panel<'a, M: 'a>(m: &MenuView, font: Font, look: MenuLook, fit: &Fit) -> Element<'a, M> {
    let c = Colors {
        anim: look.anim,
        family: look.family,
        nintendo_layout: look.nintendo_layout,
        colorblind: look.colorblind,
        ..colors(&m.style, font)
    };
    let solid = c;
    // Under a question, the menu is dimmed so the question is what stands out.
    let c = c.faded(look.anim.opacity() * if m.prompt.is_some() { UNDER_PROMPT } else { 1.0 });
    let s = m.style.scale.clamp(0.5, 2.0);
    let body: Element<'a, M> = match m.kind {
        MenuKind::Radial { .. } => radial(m, solid, s),
        MenuKind::Directional { .. } => directional(m, c, s),
        MenuKind::List | MenuKind::Buttons => list(m, c, s),
        MenuKind::Carousel { .. } => carousel(m, c, s),
        MenuKind::Grid { .. } => grid(m, c, s),
    };
    // The in-game editor's picture of what its page sets, beside the list.
    let body = match &m.figure {
        Some(fig) => row![body, super::figure::figure(fig, c, s)].spacing(24.0 * s).align_y(Alignment::Center).into(),
        None => body,
    };
    // Where this page is in the menus, above its title: "Menu › Edit Controls › A button".
    let title = typeset::line(c.typ, c.font, &m.title, 20.0 * s, c.background_text);
    let heading: Element<'a, M> = if m.crumbs.is_empty() {
        title
    } else {
        let crumbs = typeset::line(c.typ, c.font, &m.crumbs.join(" › "), 13.0 * s, c.muted);
        column![crumbs, title].spacing(4.0 * s).align_x(Alignment::Center).into()
    };
    let contents = column![
        heading,
        body,
        rich_text(&m.hint, c, 13.0 * s, c.muted),
    ]
    .spacing(14.0 * s)
    .align_x(Alignment::Center);
    framed(contents.into(), c, &m.style, 22.0 * s, fit)
}

/// An item's label, its colors (highlighted, and faded in when the style staggers its items) and whether the
/// cursor is on it.
pub(super) fn item_cell<'a, M: 'a>(m: &MenuView, i: usize, c: Colors, size: f32) -> (Element<'a, M>, Colors, bool) {
    let item = &m.items[i];
    let selected = m.selected == Some(i);
    let lit = toned(c, item.tone).lit_item(m, i);
    // A list's rows are as wide as the list, so their arrows line up at the right.
    let fills = matches!(m.kind, MenuKind::List | MenuKind::Buttons);
    (item_text(lit, item, &toned_label(c, item), size, fills), lit, selected)
}

/// An item's label, with a + or − in front of adding and removing rows in color-blind mode.
fn toned_label(c: Colors, item: &crate::menu::ItemView) -> String {
    match (c.colorblind, item.tone) {
        (true, crate::menu::Tone::Add) => format!("+ {}", item.label),
        (true, crate::menu::Tone::Remove) => format!("− {}", item.label),
        _ => item.label.clone(),
    }
}

fn directional<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let (w, h) = (190.0 * s, 52.0 * s);
    // The slot picked last stays lit, and flashes as it's picked.
    let slot = |i: usize| -> Element<'a, M> {
        match m.items.get(i).filter(|item| !item.label.is_empty()) {
            Some(_) => {
                let (label, lit, selected) = item_cell(m, i, c, 16.0 * s);
                c.moving(m, i, container(label).center_x(w).center_y(h).style(lit_cell_style(lit, selected, h)))
            }
            None => space().width(w).height(h).into(),
        }
    };
    column![
        slot(0),
        row![slot(3), space().width(40.0 * s), slot(1)].align_y(Alignment::Center),
        slot(2),
    ]
    .spacing(10.0 * s)
    .align_x(Alignment::Center)
    .into()
}

/// Items in rows of equal cells, read left to right; a short last row keeps to the left. A
/// merged item covers several cells, and the items under it aren't drawn.
fn grid<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let spans: Vec<_> = m.items.iter().map(|item| item.span).collect();
    let layout = crate::menu::GridLayout::new(&spans, m.kind.grid_columns().unwrap_or(1));
    let (w, h, gap) = (150.0 * s, 64.0 * s, 8.0 * s);
    // How far `n` cells reach, with the gaps between them.
    let reach = |n: usize, size: f32| n as f32 * size + n.saturating_sub(1) as f32 * gap;
    let mut cells = iced::widget::Stack::new().width(reach(layout.columns, w)).height(reach(layout.rows, h));
    for i in 0..m.items.len() {
        let Some(at) = layout.cells(i) else { continue };
        let (label, lit, selected) = item_cell(m, i, c, 16.0 * s);
        let (cw, ch) = (reach(at.columns, w), reach(at.rows, h));
        let cell = container(label).center_x(cw).center_y(ch).padding([0.0, 6.0 * s]).style(lit_cell_style(lit, selected, h));
        let (x, y) = (at.column as f32 * (w + gap), at.row as f32 * (h + gap));
        cells = cells.push(column![space().height(y), row![space().width(x), c.moving(m, i, cell)]]);
    }
    cells.into()
}

/// A list: one column while it is short, two once it is longer. Two columns fill top to
/// bottom, the left one first, and scroll together with the cursor.
fn list<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let len = m.items.len();
    let cursor = m.selected.unwrap_or(0);
    let cell = |i: usize| -> Element<'a, M> {
        let (content, lit, selected) = item_cell(m, i, c, 17.0 * s);
        c.moving(m, i, container(content).padding([10.0 * s, 14.0 * s]).width(Length::Fill).style(lit_cell_style(lit, selected, 40.0 * s)))
    };
    let marker = |t: &'static str| text(t).font(c.font).size(13.0 * s).color(c.muted);
    let columns = m.columns();
    let start = crate::menu::list_start_in(cursor, len, columns);
    let half = if columns == 2 { len.div_ceil(2) } else { len };
    let shown = |from: usize, to: usize| (from..to.min(from + crate::menu::VISIBLE_ROWS)).map(&cell).collect::<Vec<Element<'a, M>>>();
    // Between rows, the style's dividers, if it has any.
    let divider = if m.style.lite.plain { None } else { m.style.ornament.dividers.as_ref() };
    let column_of = |items: Vec<Element<'a, M>>, width: f32| {
        let mut col = column![].spacing(6.0 * s).width(width * s);
        for (k, item) in items.into_iter().enumerate() {
            if let Some(line) = divider.filter(|_| k > 0) {
                col = col.push(ornament::divider(line, s, c.fade));
            }
            col = col.push(item);
        }
        col
    };
    // Wide enough that a long label stays on one line: a wrapped row is taller, and a full page of
    // them overflows the box.
    let longest = m.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0);
    let width = (longest as f32 * 9.0 + 60.0).clamp(300.0, 440.0);
    let left = column_of(shown(start, half), width);
    let grid: Element<'a, M> = if half < len {
        row![left, column_of(shown(half + start, len), width)].spacing(14.0 * s).into()
    } else {
        left.into()
    };
    let mut body = column![].spacing(6.0 * s);
    if start > 0 {
        body = body.push(marker("▲"));
    }
    body = body.push(grid);
    if start + crate::menu::VISIBLE_ROWS < half {
        body = body.push(marker("▼"));
    }
    body.into()
}

fn carousel<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let n = m.items.len();
    let selected = m.selected.unwrap_or(0);
    let arrow = |t: &'static str| text(t).font(c.font).size(22.0 * s).color(c.muted);
    let mut line = row![arrow("◀")].spacing(12.0 * s).align_y(Alignment::Center);
    // The selected item in the middle, with up to two neighbors on each side. An even count
    // leaves the right side one short, so a blank keeps the selected item centered.
    let shown = n.min(5) as i32;
    let (before, after) = (shown / 2, shown - 1 - shown / 2);
    // On a touchpad the items stay put, all of them in order, the way the pad lays them out.
    let slots: Vec<Option<usize>> = if m.touch {
        (0..n).map(Some).collect()
    } else {
        (-before..=before).map(|offset| (offset <= after).then(|| (selected as i32 + offset).rem_euclid(n as i32) as usize)).collect()
    };
    for slot in slots {
        let Some(i) = slot else {
            line = line.push(space().width(120.0 * s).height(60.0 * s));
            continue;
        };
        let big = if m.touch { m.selected == Some(i) } else { i == selected };
        let height = if big { 80.0 } else { 60.0 } * s;
        let (label, lit, _) = item_cell(m, i, c, if big { 19.0 } else { 14.0 } * s);
        line = line.push(c.moving(
            m,
            i,
            container(label).center_x(if big { 170.0 } else { 120.0 } * s).center_y(height).style(lit_cell_style(lit, big, height)),
        ));
    }
    line.push(arrow("▶")).into()
}
