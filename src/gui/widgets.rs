//! Small widgets in the app's style, shared by every page: dropdowns, fields, labeled rows, sections, ⓘ tips and overlay previews.

use std::ops::RangeInclusive;

use iced::widget::{column, row};

use super::*;

/// A "▸ Label" / "▾ Label" toggle for an optional section.
pub(super) fn disclosure<'a>(label: &str, open: bool, message: Message) -> Element<'a, Message> {
    let chevron = if open { "▾" } else { "▸" };
    button(text(format!("{chevron} {label}")).size(14)).style(button::text).padding([4, 0]).on_press(message).into()
}

/// The preview keeps the real colors but caps the size so it fits the window.
pub(super) fn preview_style(style: &OverlayStyle) -> OverlayStyle {
    OverlayStyle { scale: style.scale.min(1.0), ..style.clone() }
}

/// How much smaller than on screen a menu's preview is drawn: a full-size radial menu is about
/// 800 px tall. Any smaller and its item labels can't be read.
const MENU_PREVIEW_SCALE: f32 = 0.8;

/// A menu's preview: smaller than on screen, keeping its real colors.
pub(super) fn menu_preview_style(style: &OverlayStyle) -> OverlayStyle {
    OverlayStyle { scale: style.scale.min(1.0) * MENU_PREVIEW_SCALE, ..style.clone() }
}

/// A live preview on a "screen" in the window's own tones, so transparency shows without a dark
/// slab in light mode.
pub(super) fn preview<'a>(panel: Element<'a, Message>) -> Element<'a, Message> {
    column![
        text("Preview").size(12).color(MUTED_COLOR),
        container(container(panel).center_x(Length::Fill))
            .padding(16)
            .width(Length::Fill)
            .style(|theme: &iced::Theme| container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                border: iced::Border { radius: 8.0.into(), ..iced::Border::default() },
                ..container::Style::default()
            }),
    ]
    .spacing(4)
    .into()
}

/// A checkbox for an optional number of seconds, with a slider while it's ticked.
pub(super) fn seconds_option<'a>(label: &'static str, value: Option<f32>, default: f32, on_change: impl Fn(Option<f32>) -> Message + Clone + 'a) -> Element<'a, Message> {
    optional_slider(label, value, Span { default, range: 1.0..=60.0, unit: " s" }, on_change)
}

/// How an optional slider runs: where it starts when ticked, its range, and the unit it shows.
pub(super) struct Span {
    pub default: f32,
    pub range: RangeInclusive<f32>,
    pub unit: &'static str,
}

/// A checkbox for an optional number, with a slider while it's ticked. Unticked, the number is
/// unset: `value` is `None`.
pub(super) fn optional_slider<'a>(label: &'static str, value: Option<f32>, span: Span, on_change: impl Fn(Option<f32>) -> Message + Clone + 'a) -> Element<'a, Message> {
    let toggle = on_change.clone();
    let default = span.default;
    let mut line = row![checkbox(value.is_some()).label(label).on_toggle(move |on| toggle(on.then_some(default)))]
        .spacing(10)
        .align_y(Alignment::Center);
    if let Some(number) = value {
        line = line
            .push(slider(span.range, number, move |v| on_change(Some(v))).step(1.0_f32).width(180))
            .push(text(format!("{number:.0}{}", span.unit)).size(13));
    }
    line.into()
}

/// A message for `on_change` with one field of `style` set to the value it's given.
pub(super) fn set_field<'a, T: 'a>(on_change: OnStyle<'a>, style: &OverlayStyle, set: fn(&mut OverlayStyle, T)) -> impl Fn(T) -> Message + Clone + 'a {
    let style = style.clone();
    move |v| {
        let mut s = style.clone();
        set(&mut s, v);
        on_change(s)
    }
}

/// A switch in the app's style, which stays visible while off in dark mode.
pub(super) fn toggler<'a>(on: bool) -> iced::widget::Toggler<'a, Message> {
    iced::widget::toggler(on).style(style::switch)
}

/// A checkbox in the app's style, whose empty box shows on cards in either theme.
pub(super) fn checkbox<'a>(on: bool) -> iced::widget::Checkbox<'a, Message> {
    iced::widget::checkbox(on).style(style::tickbox)
}

/// A dropdown in the app's style: stands out from cards, with a raised menu.
pub(super) fn dropdown<'a, T, L, V>(options: L, selected: Option<V>, on_select: impl Fn(T) -> Message + 'a) -> iced::widget::PickList<'a, T, L, V, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
{
    pick_list(options, selected, on_select)
        .style(style::dropdown)
        .menu_style(style::dropdown_menu)
        .padding([6, 10])
}

/// A picker of acceleration presets, with what the picked one does; it shows Custom when the
/// settings match no preset.
pub(super) fn curve_preset_picker<'a>(selected: crate::config::CurvePreset, on_pick: impl Fn(crate::config::CurvePreset) -> Message + 'a) -> Element<'a, Message> {
    let mut choices = crate::config::CurvePreset::NAMED.to_vec();
    choices.push(crate::config::CurvePreset::Custom);
    row![dropdown(choices, Some(selected), on_pick).width(170), text(selected.about()).size(12).color(MUTED_COLOR)]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

/// An exact milliseconds entry beside a slider (sliders can't hit 17 ms on a 5 s range).
pub(super) fn ms_field<'a>(ms: u64, on_change: impl Fn(u64) -> Message + 'a) -> Element<'a, Message> {
    row![
        field("ms", &ms.to_string())
            .on_input(move |s| {
                let digits: String = s.chars().filter(char::is_ascii_digit).take(6).collect();
                on_change(digits.parse().unwrap_or(0))
            })
            .width(80),
        text("ms").size(13),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

/// The height of a text box that shows `lines` whole lines (16 px text, 1.3 line height, 5 px
/// padding). Boxes grow with their text between two such heights, so no line is cut through.
pub(super) const fn editor_lines(lines: f32) -> f32 {
    lines * 20.8 + 10.0
}

/// The width of a setting's dropdown on the settings pages and the header, so the dropdowns
/// on one page line up.
pub(super) const SETTING_WIDTH: f32 = 260.0;

/// A sentence or more explaining a setting, in the theme's text color softened rather than the
/// faint grey of captions, and at a size that reads as prose.
pub(super) fn note<'a>(s: impl text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(s).size(14).style(|theme: &iced::Theme| text::Style { color: Some(theme.extended_palette().background.base.text.scale_alpha(0.7)) })
}

/// A text input in the app's style.
pub(super) fn field<'a>(placeholder: &str, value: &str) -> iced::widget::TextInput<'a, Message> {
    text_input(placeholder, value).style(style::text_field).padding([6, 10])
}

/// A search box with a button to clear it. `on_change` gets the text as typed.
pub(super) fn search_field<'a>(placeholder: &str, query: &str, on_change: fn(String) -> Message) -> Element<'a, Message> {
    let clear: Element<'a, Message> = if query.is_empty() {
        space().into()
    } else {
        button(text("✕").size(13)).style(style::secondary).on_press(on_change(String::new())).into()
    };
    row![field(placeholder, query).on_input(on_change).width(Length::Fill), clear].spacing(6).align_y(Alignment::Center).into()
}

/// A part of the Overview in a card, like a `section` but keeping the part's own heading.
pub(super) fn overview_card<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).padding(14).width(Length::Fill).style(style::card).into()
}

pub(super) fn section<'a>(title: &'a str, help_text: Option<String>, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut heading = row![text(title).size(18)].spacing(8).align_y(Alignment::Center);
    if let Some(h) = help_text {
        heading = heading.push(help(h));
    }
    container(column![heading].extend(rows).spacing(10))
        .padding(14)
        .width(Length::Fill)
        .style(style::card)
        .into()
}

/// The card at the top of an items tab (layers, macros, menus, info and log overlays): the add
/// buttons, a line on what the items are, the copy link and the tab's help. Every tab has this
/// layout, so the controls sit in the same places. The add buttons and the blurb under them take
/// the width the copy button leaves, and wrap there, so nothing is pushed out of a narrow window.
/// `empty` is the note for a setup that has none yet, kept inside the card with the add button as
/// the Combos card keeps its own.
pub(super) fn items_header<'a>(
    kind: ItemKind,
    add: Element<'a, Message>,
    blurb: &'static str,
    explain: String,
    empty: Option<&'static str>,
) -> Element<'a, Message> {
    let mut left = column![add, text(blurb).size(13).color(MUTED_COLOR)].spacing(6);
    if let Some(empty) = empty {
        left = left.push(text(empty).color(MUTED_COLOR));
    }
    container(
        row![
            container(left).width(Length::Fill),
            // The link sits under the copy button, so it takes no width from the blurb.
            column![
                row![button(text("Copy from another setup…").wrapping(text::Wrapping::None)).style(style::secondary).on_press(Message::OpenBrowse(kind)), help(explain)]
                    .spacing(12)
                    .align_y(Alignment::Center),
                tutorial_link(item_tutorial(kind)),
            ]
            .align_x(Alignment::End),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(14)
    .width(Length::Fill)
    .style(style::card)
    .into()
}

/// A bar of tabs (`segments`, made with `segment`) that wraps onto another line when the window is
/// too narrow for one, so no tab is pushed out of reach.
pub(super) fn segment_bar<'a>(segments: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    container(iced::widget::Row::with_children(segments).spacing(2).wrap().vertical_spacing(2))
        .padding(3)
        .style(style::segments)
        .into()
}

/// One tab of a `segment_bar`: its glyph and name, kept on one line.
pub(super) fn segment<'a>(glyph: Element<'a, Message>, label: String, size: f32, selected: bool, message: Message) -> Element<'a, Message> {
    let padding = if size >= 15.0 { [6, 16] } else { [5, 14] };
    button(row![glyph, text(label).size(size).wrapping(text::Wrapping::None)].spacing(6).align_y(Alignment::Center))
        .style(style::segment(selected))
        .padding(padding)
        .on_press(message)
        .into()
}

/// The title of an item's card, with the chevron that opens it at the same size as a disclosure's.
pub(super) fn card_title<'a>(open: bool, title: iced::widget::Text<'a>) -> iced::widget::Button<'a, Message> {
    let chevron = text(if open { "▾" } else { "▸" }).size(14);
    button(row![chevron, title].spacing(8).align_y(Alignment::Center)).style(button::text).padding(0)
}

/// The top of a Details card that a setup can override: a toggle for "Use its own `what` in this
/// setup", and while it's off, a note that the card follows `follows` (such as "the font set on the
/// App settings page"). Every such card starts with this, and shows its editor only while the toggle is on.
pub(super) fn own_rows<'a>(what: &str, follows: &str, own: bool, on_toggle: impl Fn(bool) -> Message + 'a) -> Vec<Element<'a, Message>> {
    let mut rows = vec![toggler(own).label(format!("Use its own {what} in this setup")).on_toggle(on_toggle).into()];
    if !own {
        rows.push(text(format!("Following {follows}.")).size(13).color(MUTED_COLOR).into());
    }
    rows
}

/// Deletes a whole item, from its card's header.
pub(super) fn delete_item<'a>(label: &'static str, message: Message) -> Element<'a, Message> {
    button(text(label).size(13)).style(style::quiet_danger).on_press(message).into()
}

/// A small, quiet link to a tutorial in the Manual, for the corner of the panel it teaches. `file` is the
/// tutorial's chapter file.
pub(super) fn tutorial_link<'a>(file: &'static str) -> Element<'a, Message> {
    tooltip(
        button(text("Tutorial ↗").size(12).color(MUTED_COLOR)).style(button::text).padding([2, 4]).on_press(Message::OpenTutorial(file)),
        container(text("Open the step-by-step tutorial in the Manual").size(12)).padding(6).style(style::tooltip),
        tooltip::Position::Bottom,
    )
    .gap(4)
    .into()
}

/// The tutorial for each kind of item.
fn item_tutorial(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Macro => "tutorial-macros.md",
        ItemKind::Cycle => "tutorial-cycles.md",
        ItemKind::Menu => "tutorial-menus.md",
        ItemKind::Info | ItemKind::Log => "tutorial-overlays.md",
        ItemKind::Layer => "tutorial-layers.md",
        ItemKind::Mode => "tutorial-modes.md",
    }
}

/// An ⓘ that explains a section on hover, instead of a paragraph of gray text.
pub(super) fn help<'a>(explanation: String) -> Element<'a, Message> {
    tooltip(
        text("ⓘ").size(16).color(style_accent()),
        container(text(explanation).size(13)).padding(10).max_width(420.0).style(style::tooltip),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .into()
}

/// The accent color, for hints that should catch the eye.
pub(super) fn style_accent() -> Color {
    Color::from_rgb(0.35, 0.45, 0.95)
}

pub(super) fn labeled<'a>(label: impl text::IntoFragment<'a>, editor: Element<'a, Message>) -> Element<'a, Message> {
    labeled_in(LABEL_WIDTH, label, editor)
}

/// A labeled row whose label column is `width` wide, for rows in a narrower column than the rest.
pub(super) fn labeled_in<'a>(width: f32, label: impl text::IntoFragment<'a>, editor: Element<'a, Message>) -> Element<'a, Message> {
    row![text(label).width(width), editor]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

/// Gives an action editor the row's free width, so it wraps onto a second line there instead of
/// running off screen, and buttons placed after it (such as a remove button) line up at the right.
pub(super) fn fill_x<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content).width(Length::Fill).into()
}
