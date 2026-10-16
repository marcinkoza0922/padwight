//! Drawing the overlays: each panel as an iced element, sized by its style's `Fit` (see
//! `fit.rs`) and laid out where its position puts it.

mod figure;
mod frame;
mod glyphs;
mod log_panel;
mod look;
mod keyboard;
mod marked;
mod menus;
mod ornament;
mod texture;
mod radial;
mod transform;
mod typeset;

use std::f32::consts::TAU;

pub use glyphs::icon_svg;
pub use log_panel::log_panel;

use glyphs::{segment_element, state_icon};
use look::{Look, lit_cell_style, panel_style};
pub use keyboard::keyboard_panel;
pub use menus::menu_panel;
use menus::item_cell;
use radial::radial;
use transform::transform;
use typeset::Type;

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
    media,
    media::MediaView,
    menu::MenuView,
    motion::Anim,
    offer::OfferView,
};

const UNIT: f32 = 46.0;
const GAP: f32 = 4.0;

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
    /// The typography: case, letter spacing and size.
    typ: Type,
    /// How much of the panel is shown, 0 to 1: its fade in and out, applied to pictures and
    /// ornaments as it is to colors.
    fade: f32,
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

    /// Item `i`'s cell, swelling as the cursor lands on it and sliding in as it comes in.
    pub fn moving<'a, M: 'a>(&self, m: &MenuView, i: usize, cell: impl Into<Element<'a, M>>) -> Element<'a, M> {
        transform(cell, self.anim.item_scale(m.selected == Some(i)), iced::Vector::new(self.anim.item_shift(i), 0.0))
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
            fade: self.fade * opacity.clamp(0.0, 1.0),
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

/// Text whose tokens (`{south}`, `{lt}`, `{icon:bow}`…) are drawn as glyphs and icons, and whose
/// formatting tags (`{b}`, `{u}`, `{color:red}`…) are applied, sized for text of `size`. Plain
/// text stays one text widget.
fn rich_text<'a, M: 'a>(t: &str, c: Colors, size: f32, color: Color) -> Element<'a, M> {
    let glyphs = crate::info::Glyphs { family: c.family, nintendo_layout: c.nintendo_layout };
    let segments = crate::info::glyph_text(t, glyphs);
    match segments.as_slice() {
        [] => typeset::line(c.typ, c.font, "", size, color),
        [Segment::Text(t)] => typeset::line(c.typ, c.font, t, size, color),
        _ => marked::line(
            &segments,
            c,
            marked::Setting { size, color, scale: size / 17.0, gap: size * 0.3, opacity: c.anim.opacity(), typed: true, trim: true },
        ),
    }
}

fn colors(style: &OverlayStyle, font: Font) -> Colors {
    let background = paint(&style.background, [0x16, 0x18, 0x1c]);
    let item = paint(&style.items, [0x30, 0x34, 0x3c]);
    let selected = paint(&style.selected, [0x2f, 0x5d, 0xb0]);
    let scale = style.scale.clamp(0.5, 2.0);
    // Each text takes the color that reads on what it sits on.
    let background_text = text_on(background);
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
        look: Look::of(style, scale, background_text),
        typ: Type::of(&style.typography, scale),
        fade: 1.0,
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
    container(transform(panel, anim.scale(), iced::Vector::ZERO))
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

/// The style's filters, as layers over the contents: grain, pixel blocks, camo and a tint.
fn filters<'a, M: 'a>(c: Colors, style: &OverlayStyle, s: f32) -> Vec<Element<'a, M>> {
    let ornaments = &style.ornament;
    let mut layers = Vec::new();
        if let Some(grain) = &ornaments.grain {
        layers.push(texture::grain(grain, s, c.anim.clock, grain.animated && !style.lite.calm, c.fade));
        }
        if let Some(pixels) = &ornaments.pixels {
        layers.push(texture::pixels(pixels, s, c.fade));
        }
        if let Some(camo) = &ornaments.camo {
        layers.push(texture::camo(camo, s, c.fade));
        }
        if let Some(tint) = &ornaments.tint {
        layers.push(ornament::tint(tint, c.fade));
        }
    layers
}

/// A panel's box: its colors and padding, at the size `fit` gives. The contents scroll, so a
/// box held to its max length scrolls rather than spilling, and the box clips anything wider.
/// The style's picture sits behind the contents and its ornaments over them, all the panel's
/// size: the contents are the stack's base layer, which sets that size.
fn framed<'a, M: 'a>(body: Element<'a, M>, c: Colors, style: &OverlayStyle, padding: impl Into<Padding>, fit: &Fit) -> Element<'a, M> {
    let height = if fit.height.is_some() { Length::Fill } else { Length::Shrink };
    let contents = scrollable(body).width(Length::Shrink).height(height);
    let mut layers = iced::widget::Stack::new().push(container(contents).padding(padding).align_x(Horizontal::Center));
    if !style.lite.plain {
        let s = style.scale.clamp(0.5, 2.0);
        if let Some(picture) = &style.background_image {
            layers = layers.push_under(ornament::picture(picture, c.fade));
        }
        let ornaments = &style.ornament;
        // Concrete goes under metal, and metal under paper, should they all be on.
        for (material, t) in [
            (texture::Material::Paper, &ornaments.paper),
            (texture::Material::BrushedMetal, &ornaments.brushed_metal),
            (texture::Material::Concrete, &ornaments.concrete),
        ] {
            if let Some(t) = t {
                layers = layers.push_under(texture::layer(material, t, c.fade));
            }
        }
        if let Some(lines) = &ornaments.scanlines {
            let drift = lines.drift && !style.lite.calm;
            layers = layers.push(ornament::scanlines(lines, s, c.anim.clock, drift, c.fade));
        }
        for filter in filters(c, style, s) {
            layers = layers.push(filter);
        }
        if let Some(vignette) = &ornaments.vignette {
            for edge in ornament::vignette(vignette, c.fade) {
                layers = layers.push(edge);
            }
        }
        if let Some(frame) = &ornaments.frame {
            for mark in frame::marks(frame, s, c.fade) {
                layers = layers.push(mark);
            }
        }
    }
    let mut boxed = container(layers)
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
    // A table of no columns divides by zero, so an overlay with no cells yet (a new one) has none.
    let grid: Element<'a, M> = match columns {
        0 => space().into(),
        _ => iced::widget::table(cols, 0..v.rows.len()).padding_x(10.0 * s).padding_y(2.0 * s).separator(0.0).into(),
    };
    let body: Element<'a, M> = match &v.title {
        Some(title) => column![info_cell(title, c, s, opacity), grid].spacing(8.0 * s).into(),
        None => grid,
    };
    framed(body, c, &v.style, [12.0 * s, 16.0 * s], fit)
}

/// `segments` as an info overlay's cell draws them, on a chip of the default panel's color, for
/// the settings window's previews.
pub fn chip<'a, M: 'a>(segments: &[Segment], font: Font) -> Element<'a, M> {
    let c = colors(&OverlayStyle::default(), font);
    let background = Color { a: 1.0, ..c.background };
    container(info_cell(segments, c, 0.85, 1.0))
        .padding([3.0, 8.0])
        .style(move |_: &iced::Theme| container::Style {
            background: Some(background.into()),
            border: Border { radius: 6.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

/// A cell's segments in a row that wraps them onto the next line when they're too wide.
fn info_cell<'a, M: 'a>(segments: &[Segment], c: Colors, s: f32, opacity: f32) -> Element<'a, M> {
    marked::line(
        segments,
        c,
        marked::Setting { size: 16.0 * s, color: c.background_text, scale: s, gap: 2.0 * s, opacity, typed: false, trim: false },
    )
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
    framed(contents.into(), c, &m.style, 18.0 * s, fit)
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
    framed(contents.into(), c, &o.style, 22.0 * s, fit)
}
