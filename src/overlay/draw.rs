//! Drawing the overlays: each panel as an iced element, sized by its style's `Fit` (see
//! `fit.rs`) and laid out where its position puts it.

mod glyphs;
mod log_panel;
mod look;
mod radial;

use std::f32::consts::TAU;

pub use glyphs::icon_svg;
pub use log_panel::log_panel;

use glyphs::{segment_element, state_icon};
use look::{Look, lit_cell_style, panel_style};
use radial::radial;

use iced::{
    Alignment, Border, Color, Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    widget::{column, container, progress_bar, row, scrollable, space, text},
};
use iced::Font;

use super::{KeyboardView, fit::Fit};
use crate::{
    config::{MenuKind, OverlayStyle, Paint, ScreenPosition},
    info::{InfoView, Segment},
    keyboard, media,
    media::MediaView,
    menu::MenuView,
    motion::Anim,
    offer::OfferView,
};

const UNIT: f32 = 46.0;
const GAP: f32 = 4.0;
/// The keyboard's and numpad's padding inside their box, at scale 1.
const KEYBOARD_PADDING: f32 = 16.0;

/// The width of a keyboard's body at scale 1: the legend's or the keys', whichever is wider.
fn keyboard_body_width(board: crate::keyboard::Board) -> f32 {
    if board == crate::keyboard::Board::Numpad {
        // Numpad keys are fewer, so bigger; its body has a floor of 230 at any scale.
        (board.units_wide() * (UNIT * 1.5 + GAP)).max(230.0)
    } else {
        board.units_wide() * (UNIT + GAP)
    }
}
/// Distance from the screen edge for edge and corner positions.
const EDGE_MARGIN: f32 = 40.0;

/// What a menu is drawn with besides its own style: the controller in use (for its button
/// glyphs), the color-blind tints, and how the menu is moving.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuLook {
    pub colorblind: bool,
    pub family: crate::info::PadFamily,
    pub nintendo_layout: bool,
    pub anim: Anim,
}

fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: f32, y: f32| x + (y - x) * t;
    Color { r: mix(a.r, b.r), g: mix(a.g, b.g), b: mix(a.b, b.b), a: mix(a.a, b.a) }
}

#[derive(Clone, Copy)]
pub struct Colors {
    anim: Anim,
    font: Font,
    /// The controller in use, for button glyphs.
    family: crate::info::PadFamily,
    /// Face buttons drawn with the Nintendo layout's labels.
    nintendo_layout: bool,
    /// Tints for added and removed rows in colors color-blind people can tell apart.
    colorblind: bool,
    background: Color,
    background_text: Color,
    muted: Color,
    item: Color,
    item_text: Color,
    selected: Color,
    selected_text: Color,
    /// 0 square .. 1 round, from the style's corners setting.
    corners: f32,
    look: Look,
}

fn paint(p: &Paint, fallback: [u8; 3]) -> Color {
    let [r, g, b] = p.rgb().unwrap_or(fallback);
    Color::from_rgba8(r, g, b, p.opacity.clamp(0.0, 1.0))
}

/// White on dark colors, black on light ones.
fn text_on(c: Color) -> Color {
    let luminance = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    if luminance > 0.6 { Color::from_rgb(0.08, 0.08, 0.1) } else { Color::WHITE }
}

impl Colors {
    /// How lit item `i` is, 0 to 1: the cursor's item, and the one it came from as it slides away.
    pub fn lit(&self, m: &MenuView, i: usize) -> f32 {
        self.anim.lit(m.selected == Some(i), self.anim.timing.from == Some(i as u32))
    }

    /// These colors for item `i`: its highlight blended in, its pick flashed, and its stagger fade.
    pub fn lit_item(&self, m: &MenuView, i: usize) -> Colors {
        let lit = self.lit(m, i);
        let flash = if m.selected == Some(i) { self.anim.flash() } else { 0.0 };
        let fade = self.anim.item_fade(i);
        let faded = |c: Color| Color { a: c.a * fade, ..c };
        Colors {
            item: faded(blend(blend(self.item, self.selected, lit), Color::WHITE, flash * 0.5)),
            item_text: faded(blend(self.item_text, self.selected_text, lit)),
            ..*self
        }
    }

    /// Every color at `opacity` times its own, for fading out.
    fn faded(self, opacity: f32) -> Colors {
        let f = |c: Color| Color { a: c.a * opacity.clamp(0.0, 1.0), ..c };
        Colors {
            background: f(self.background),
            background_text: f(self.background_text),
            muted: f(self.muted),
            item: f(self.item),
            item_text: f(self.item_text),
            selected: f(self.selected),
            selected_text: f(self.selected_text),
            look: self.look.faded(f),
            ..self
        }
    }
}

impl Colors {
    /// These colors with button glyphs drawn as `glyphs` says.
    fn with_glyphs(self, glyphs: crate::info::Glyphs) -> Colors {
        Colors { family: glyphs.family, nintendo_layout: glyphs.nintendo_layout, ..self }
    }
}

/// Text whose controller tokens (`{south}`, `{lt}`…) are drawn as the pad's glyphs, sized for
/// text of `size`. Plain text stays one text widget.
fn rich_text<'a, M: 'a>(t: &str, c: Colors, size: f32, color: Color) -> Element<'a, M> {
    let glyphs = crate::info::Glyphs { family: c.family, nintendo_layout: c.nintendo_layout };
    let segments = crate::info::glyph_text(t, glyphs);
    let plain = |t: &str| -> Element<'a, M> { text(t.to_string()).font(c.font).size(size).color(color).into() };
    match segments.as_slice() {
        [] => plain(""),
        [Segment::Text(t)] => plain(t),
        _ => {
            let mut line = row![].spacing(size * 0.3).align_y(Alignment::Center);
            for seg in &segments {
                line = line.push(match seg {
                    Segment::Text(t) => plain(t.trim()),
                    glyph => segment_element(glyph, c, size / 17.0, c.anim.opacity()),
                });
            }
            line.into()
        }
    }
}

fn colors(style: &OverlayStyle, font: Font) -> Colors {
    let background = paint(&style.background, [0x16, 0x18, 0x1c]);
    let item = paint(&style.items, [0x30, 0x34, 0x3c]);
    let selected = paint(&style.selected, [0x2f, 0x5d, 0xb0]);
    let background_text = style.text.as_deref().and_then(crate::config::parse_hex).map_or_else(|| text_on(background), |[r, g, b]| Color::from_rgb8(r, g, b));
    Colors {
        anim: Anim::still(),
        font,
        family: crate::info::PadFamily::default(),
        nintendo_layout: false,
        colorblind: false,
        background,
        background_text,
        muted: Color { a: 0.75, ..background_text },
        item,
        item_text: text_on(item),
        selected,
        selected_text: text_on(selected),
        corners: style.corners,
        look: Look::of(style, style.scale.clamp(0.5, 2.0), background_text),
    }
}

/// Positions an overlay panel on the full-screen surface, moved by its animation and by the
/// style's offset.
pub fn place<'a, M: 'a>(panel: Element<'a, M>, style: &OverlayStyle, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let (col, row) = style.position.cell();
    let horizontal = [Horizontal::Left, Horizontal::Center, Horizontal::Right][col];
    let vertical = [Vertical::Top, Vertical::Center, Vertical::Bottom][row];
    let margin = if style.position == ScreenPosition::Center { 0.0 } else { EDGE_MARGIN };
    let (x, y) = anim.offset();
    let (left, right) = sides(margin, x + fit.x, col);
    let (top, bottom) = sides(margin, y + fit.y, row);
    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(horizontal)
        .align_y(vertical)
        .padding(Padding { top, right, bottom, left })
        .into()
}

/// The padding on the two sides of a panel, for a panel `shift` pixels off where it sits in
/// `slot` (0 start, 1 center, 2 end) of a `margin` from the edge.
fn sides(margin: f32, shift: f32, slot: usize) -> (f32, f32) {
    match slot {
        0 => ((margin + shift).max(0.0), margin),
        2 => (margin, (margin - shift).max(0.0)),
        // A centered panel sits halfway between its two paddings.
        _ => (margin + (2.0 * shift).max(0.0), margin + (-2.0 * shift).max(0.0)),
    }
}

/// A panel's box: its colors and padding, at the size `fit` gives. The contents scroll, so a
/// box held to its max length scrolls rather than spilling, and the box clips anything wider.
fn framed<'a, M: 'a>(body: Element<'a, M>, c: Colors, padding: impl Into<Padding>, fit: &Fit) -> Element<'a, M> {
    let height = if fit.height.is_some() { Length::Fill } else { Length::Shrink };
    let contents = scrollable(body).width(Length::Shrink).height(height);
    let mut boxed = container(contents)
        .padding(padding)
        .align_x(Horizontal::Center)
        .clip(true)
        .max_width(fit.max_width)
        .max_height(fit.max_height)
        .style(panel_style(c));
    if let Some(w) = fit.width {
        boxed = boxed.width(w);
    }
    if let Some(h) = fit.height {
        boxed = boxed.height(h);
    }
    boxed.into()
}

/// The colors of a row of this kind: adding rows are tinted green and removing ones red, so
/// they stand out from the items they act on.
fn toned(c: Colors, tone: crate::menu::Tone) -> Colors {
    // Lime with white text stays readable at this mix (about 4.7:1 on the default items).
    let (tint, share) = match (tone, c.colorblind) {
        (crate::menu::Tone::Normal, _) => return c,
        (crate::menu::Tone::Add, false) => (Color::from_rgb8(0x5a, 0x9a, 0x1a), 0.75),
        (crate::menu::Tone::Remove, false) => (Color::from_rgb8(0xd0, 0x64, 0x64), 0.4),
        (crate::menu::Tone::Add, true) => (Color::from_rgb8(0x4a, 0x8f, 0xe0), 0.4),
        (crate::menu::Tone::Remove, true) => (Color::from_rgb8(0xe8, 0x96, 0x2e), 0.4),
    };
    let mix = |a: f32, b: f32| a + (b - a) * share;
    let item = Color { r: mix(c.item.r, tint.r), g: mix(c.item.g, tint.g), b: mix(c.item.b, tint.b), a: c.item.a };
    Colors { item, item_text: text_on(item), ..c }
}

/// An info overlay: its cells in a table, so the rows line up across the columns. A row is as
/// tall as its tallest cell, and a cell's segments wrap onto the next line when the panel is
/// held to its max width, so the whole row grows rather than the cell alone.
pub fn info_panel<'a, M: 'a>(v: &InfoView, font: Font, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let opacity = v.opacity * anim.opacity();
    let c = colors(&v.style, font).faded(opacity);
    let s = v.style.scale.clamp(0.5, 2.0);
    let columns = v.rows.iter().map(Vec::len).max().unwrap_or(0);
    let cols = (0..columns).map(|j| {
        iced::widget::table::column(space(), move |i: usize| -> Element<'a, M> {
            match v.rows[i].get(j) {
                Some(segments) => container(info_cell(segments, c, s, opacity)).align_y(Vertical::Center).into(),
                None => space().into(),
            }
        })
    });
    // Columns are as wide as their widest cell; the table's padding is half the gap between them.
    let grid = iced::widget::table(cols, 0..v.rows.len()).padding_x(10.0 * s).padding_y(2.0 * s).separator(0.0);
    let body: Element<'a, M> = match &v.title {
        Some(title) => column![info_cell(title, c, s, opacity), grid].spacing(8.0 * s).into(),
        None => grid.into(),
    };
    framed(body, c, [12.0 * s, 16.0 * s], fit)
}

/// A cell's segments in a row that wraps them onto the next line when they're too wide.
fn info_cell<'a, M: 'a>(segments: &[Segment], c: Colors, s: f32, opacity: f32) -> Element<'a, M> {
    let mut line = row![].spacing(2.0 * s).align_y(Alignment::Center);
    for seg in segments {
        line = line.push(segment_element(seg, c, s, opacity));
    }
    line.wrap().vertical_spacing(2.0 * s).into()
}

#[expect(clippy::too_many_lines, reason = "predates the size lints")]
pub fn keyboard_panel<'a, M: 'a>(v: &KeyboardView, font: Font, glyphs: crate::info::Glyphs, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let c = colors(&v.style, font).with_glyphs(glyphs).faded(anim.opacity());
    let numpad = v.board == crate::keyboard::Board::Numpad;
    // A set width fits the keys (and their text) to it; otherwise the style's scale applies.
    let s = match fit.width {
        Some(w) => w / (keyboard_body_width(v.board) + 2.0 * KEYBOARD_PADDING),
        None => v.style.scale.clamp(0.5, 2.0),
    };
    // Numpad keys are fewer, so bigger.
    let unit = if numpad { UNIT * 1.5 } else { UNIT } * s;
    let gap = GAP * s;
    let cursor_code = keyboard::key_at(v.board, v.cursor).code;
    let shifted = v.latched.iter().any(|l| l == "KEY_LEFTSHIFT" || l == "KEY_RIGHTSHIFT");
    let label_size = if numpad { 22.0 } else { 14.0 };
    let mut rows = column![].spacing(gap);
    // Keys are numbered in reading order, as the motion tracking numbers them.
    let mut slot = 0u32;
    for keys in v.board.rows().iter() {
        let mut line = row![].spacing(gap);
        for key in keys.iter() {
            let here = slot;
            slot += 1;
            let width = key.width * (unit + gap) - gap;
            if key.code.is_empty() {
                line = line.push(space().width(width).height(unit));
                continue;
            }
            let cursor = key.code == cursor_code;
            let pressed = v.pressed.as_deref() == Some(key.code);
            let selected = cursor || pressed;
            // Caps Lock lights up like a latched key while it's on.
            let latched = v.latched.iter().any(|l| l == key.code) || (v.caps && key.code == "KEY_CAPSLOCK");
            // The cursor's key lights up and the one it left goes dark, sliding between them.
            let lit = if pressed { 1.0 } else { anim.lit(cursor, anim.timing.from == Some(here)) };
            let round = c.corners * unit / 2.0;
            let (bg, fg, border, glow) = if latched && !selected {
                let green = Color::from_rgb8(0x2e, 0x7d, 0x46);
                (green, Color::WHITE, Border { width: 1.0, radius: round.into(), color: Color::from_rgb8(0x3f, 0xb9, 0x50) }, iced::Shadow::default())
            } else {
                let flash = if pressed { anim.flash() * 0.5 } else { 0.0 };
                (
                    blend(blend(c.item, c.selected, lit), Color::WHITE, flash),
                    blend(c.item_text, c.selected_text, lit),
                    look::item_outline(&c, selected, lit, round),
                    look::item_glow(&c, lit),
                )
            };
            let cap = container(text(keyboard::cap_label(key, v.board, shifted, v.caps)).font(c.font).size(if selected { label_size + 2.0 } else { label_size } * s).color(fg))
                .center_x(width)
                .center_y(unit)
                .style(move |_| container::Style { background: Some(bg.into()), border, shadow: glow, ..container::Style::default() });
            line = line.push(cap);
        }
        rows = rows.push(line);
    }
    let hint = |t: &'static str| rich_text::<M>(t, c, 14.0 * s, c.background_text);
    let (legend, width): (Element<'a, M>, f32) = if numpad {
        let legend = row(super::NUMPAD_LEGEND.iter().map(|t| hint(t))).spacing(14.0 * s).wrap().vertical_spacing(4.0 * s).align_x(Horizontal::Center);
        (legend.into(), (v.board.units_wide() * (unit + gap)).max(230.0 * s))
    } else {
        let legend = row(super::KEYBOARD_LEGEND.iter().map(|t| hint(t)))
        .spacing(18.0 * s)
        .align_y(Alignment::Center)
        .wrap()
        .vertical_spacing(4.0 * s)
        .align_x(Horizontal::Center);
        (legend.into(), v.board.units_wide() * (unit + gap))
    };
    let mut body = column![].spacing(12.0 * s).width(width).align_x(Alignment::Center);
    // What's been typed, above the keys, for when the game's own text box is hidden behind them.
    if !v.typed.is_empty() {
        body = body.push(text(format!("{}▏", v.typed)).font(c.font).size(18.0 * s).color(c.background_text));
    }
    body = body.push(rows).push(legend);
    if v.closing > 0.0 {
        body = body.push(progress_bar(0.0..=1.0, v.closing).girth(4.0 * s));
    }
    framed(body.into(), c, KEYBOARD_PADDING * s, fit)
}

/// An item's glyphs (the same as the info overlays'), its label, and a ▸ for submenus. A
/// text badge (a stick's LS, say) is drawn as a small tag.
fn item_text<'a, M: 'a>(c: Colors, item: &crate::menu::ItemView, label: &str, size: f32, fg: Color) -> Element<'a, M> {
    let font = c.font;
    let mut line = row![].spacing(size * 0.5).align_y(Alignment::Center);
    // The glyphs are sized from the label, which is 17 units when the scale is 1.
    for b in &item.buttons {
        line = line.push(segment_element(&crate::info::button_glyph(*b, c.family, c.nintendo_layout), c, size / 17.0, 1.0));
    }
    if let Some(b) = item.button.as_deref().filter(|b| !b.is_empty()) {
        line = line.push(
            container(text(b.to_string()).font(font).size(size - 2.0).color(Color::BLACK))
                .padding([1.0, size * 0.4])
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.85, 0.87, 0.9).into()),
                    border: Border { radius: 5.0.into(), ..Border::default() },
                    ..container::Style::default()
                }),
        );
    }
    if let Some(k) = item.keyword {
        let icon = iced::widget::svg::Handle::from_memory(crate::keyword_icon::svg(k).into_bytes());
        line = line.push(iced::widget::svg(icon).width(size).height(size));
    }
    line = line.push(rich_text(label, c, size, fg));
    if item.submenu {
        line = line.push(text("▸").font(font).size(size).color(Color { a: 0.7, ..fg }));
    }
    line.into()
}

/// The media controls: what is playing, how far along, and the volume.
pub fn media_panel<'a, M: 'a>(m: &MediaView, font: Font, glyphs: crate::info::Glyphs, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let c = colors(&m.style, font).with_glyphs(glyphs).faded(anim.opacity());
    let s = m.style.scale.clamp(0.5, 2.0);
    let width = 560.0 * s;
    let body: Element<'a, M> = match &m.player {
        None => text("No media player is running").font(c.font).size(18.0 * s).color(c.background_text).into(),
        Some(player) => {
            let title = if m.title.is_empty() { "Nothing playing" } else { &m.title };
            let by = [m.artist.as_str(), player.as_str()].iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" · ");
            let progress = if m.length_ms > 0 { (m.position_ms as f32 / m.length_ms as f32).clamp(0.0, 1.0) } else { 0.0 };
            let times = format!("{} / {}", media::clock(m.position_ms), if m.length_ms > 0 { media::clock(m.length_ms) } else { "--:--".into() });
            let mut meta = row![text(times).font(c.font).size(14.0 * s).color(c.muted)].spacing(16.0 * s);
            // The system's sound, which the D-pad changes, then the player's own volume.
            match (m.system_volume, m.muted) {
                (Some(_), true) => meta = meta.push(text("Sound muted").font(c.font).size(14.0 * s).color(c.muted)),
                (Some(v), false) => meta = meta.push(text(format!("Sound {:.0}%", v * 100.0)).font(c.font).size(14.0 * s).color(c.muted)),
                (None, _) => {}
            }
            if let Some(v) = m.volume {
                let what = if m.system_volume.is_some() { "Player" } else { "Volume" };
                meta = meta.push(text(format!("{what} {:.0}%", v * 100.0)).font(c.font).size(14.0 * s).color(c.muted));
            }
            if m.players > 1 {
                meta = meta.push(text(format!("{} players", m.players)).font(c.font).size(14.0 * s).color(c.muted));
            }
            column![
                row![
                    state_icon(m.state, c.background_text, 28.0 * s),
                    column![
                        text(title.to_string()).font(c.font).size(20.0 * s).color(c.background_text),
                        text(by).font(c.font).size(14.0 * s).color(c.muted),
                    ]
                    .spacing(2.0 * s),
                ]
                .spacing(14.0 * s)
                .align_y(Alignment::Center),
                progress_bar(0.0..=1.0, progress).girth(6.0 * s),
                meta.wrap().vertical_spacing(4.0 * s),
            ]
            .spacing(10.0 * s)
            .into()
        }
    };
    let contents = column![body, rich_text(&m.hint, c, 12.0 * s, c.muted)]
        .spacing(12.0 * s)
        .width(width)
        .align_x(Alignment::Center);
    framed(contents.into(), c, 18.0 * s, fit)
}

/// The offer to add a library game: the question and its answers. (Its lists are menus.)
pub fn offer_panel<'a, M: 'a>(o: &OfferView, font: Font, glyphs: crate::info::Glyphs, fit: &Fit, anim: &Anim) -> Element<'a, M> {
    let c = colors(&o.style, font).with_glyphs(glyphs).faded(anim.opacity());
    let s = o.style.scale.clamp(0.5, 2.0);
    let cell = |label: String, lit: Colors, selected: bool| {
        container(rich_text::<M>(&label, lit, 16.0 * s, lit.item_text))
            .padding([8.0 * s, 16.0 * s])
            .style(lit_cell_style(lit, selected, 36.0 * s))
    };
    let answers = o.answers.iter().map(|(button, what)| cell(format!("{button}  {what}"), c, false).into());
    let body: Element<'a, M> = row(answers).spacing(10.0 * s).into();
    let contents = column![
        text(o.title.clone()).font(c.font).size(20.0 * s).color(c.background_text),
        text(o.detail.clone()).font(c.font).size(14.0 * s).color(c.muted).width(520.0 * s).align_x(Alignment::Center),
        body,
        rich_text(&o.hint, c, 12.0 * s, c.muted),
    ]
    .spacing(14.0 * s)
    .align_x(Alignment::Center);
    framed(contents.into(), c, 22.0 * s, fit)
}

pub fn menu_panel<'a, M: 'a>(m: &MenuView, font: Font, look: MenuLook, fit: &Fit) -> Element<'a, M> {
    let c = Colors {
        anim: look.anim,
        family: look.family,
        nintendo_layout: look.nintendo_layout,
        colorblind: look.colorblind,
        ..colors(&m.style, font)
    };
    let c = c.faded(look.anim.opacity());
    let s = m.style.scale.clamp(0.5, 2.0);
    let body: Element<'a, M> = match m.kind {
        MenuKind::Radial { .. } => radial(m, c, s),
        MenuKind::Directional { .. } => directional(m, c, s),
        MenuKind::List | MenuKind::Buttons => list(m, c, s),
        MenuKind::Carousel { .. } => carousel(m, c, s),
        MenuKind::Grid { .. } => grid(m, c, s),
    };
    // Where this page is in the menus, above its title: "Menu › Edit Controls › A button".
    let title = text(m.title.clone()).font(c.font).size(20.0 * s).color(c.background_text);
    let heading: Element<'a, M> = if m.crumbs.is_empty() {
        title.into()
    } else {
        let crumbs = text(m.crumbs.join(" › ")).font(c.font).size(13.0 * s).color(c.muted);
        column![crumbs, title].spacing(4.0 * s).align_x(Alignment::Center).into()
    };
    let contents = column![
        heading,
        body,
        rich_text(&m.hint, c, 13.0 * s, c.muted),
    ]
    .spacing(14.0 * s)
    .align_x(Alignment::Center);
    framed(contents.into(), c, 22.0 * s, fit)
}

/// An item's label, its colors (highlighted, and faded in under Stagger) and whether the
/// cursor is on it.
fn item_cell<'a, M: 'a>(m: &MenuView, i: usize, c: Colors, size: f32) -> (Element<'a, M>, Colors, bool) {
    let item = &m.items[i];
    let selected = m.selected == Some(i);
    let lit = toned(c, item.tone).lit_item(m, i);
    (item_text(lit, item, &toned_label(c, item), size, lit.item_text), lit, selected)
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
                container(label).center_x(w).center_y(h).style(lit_cell_style(lit, selected, h)).into()
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

/// Items in rows of equal cells, read left to right; a short last row keeps to the left.
fn grid<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let columns = m.kind.grid_columns().unwrap_or(1);
    let (w, h) = (150.0 * s, 64.0 * s);
    let mut rows = column![].spacing(8.0 * s);
    for start in (0..m.items.len()).step_by(columns) {
        let mut line = row![].spacing(8.0 * s);
        for i in start..(start + columns).min(m.items.len()) {
            let (label, lit, selected) = item_cell(m, i, c, 16.0 * s);
            line = line.push(container(label).center_x(w).center_y(h).padding([0.0, 6.0 * s]).style(lit_cell_style(lit, selected, h)));
        }
        rows = rows.push(line);
    }
    rows.into()
}

/// A list: one column while it is short, two once it is longer. Two columns fill top to
/// bottom, the left one first, and scroll together with the cursor.
fn list<'a, M: 'a>(m: &MenuView, c: Colors, s: f32) -> Element<'a, M> {
    let len = m.items.len();
    let cursor = m.selected.unwrap_or(0);
    let cell = |i: usize| -> Element<'a, M> {
        let (content, lit, selected) = item_cell(m, i, c, 17.0 * s);
        container(content).padding([10.0 * s, 14.0 * s]).width(Length::Fill).style(lit_cell_style(lit, selected, 40.0 * s)).into()
    };
    let marker = |t: &'static str| text(t).font(c.font).size(13.0 * s).color(c.muted);
    let start = crate::menu::list_start(cursor, len);
    let half = if crate::menu::list_columns(len) == 2 { len.div_ceil(2) } else { len };
    let shown = |from: usize, to: usize| (from..to.min(from + crate::menu::VISIBLE_ROWS)).map(&cell).collect::<Vec<Element<'a, M>>>();
    let column_of = |items: Vec<Element<'a, M>>, width: f32| items.into_iter().fold(column![].spacing(6.0 * s).width(width * s), iced::widget::Column::push);
    let left = column_of(shown(start, half), 300.0);
    let grid: Element<'a, M> = if half < len {
        row![left, column_of(shown(half + start, len), 300.0)].spacing(14.0 * s).into()
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
        line = line.push(
            container(label)
                .center_x(if big { 170.0 } else { 120.0 } * s)
                .center_y(height)
                .style(lit_cell_style(lit, big, height)),
        );
    }
    line.push(arrow("▶")).into()
}
