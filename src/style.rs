//! Shared widget styles for the GUI. iced's defaults draw dropdowns, their menus and the
//! section cards in the same color, so fields disappear inside cards; these keep them apart
//! in both light and dark themes.

use iced::{
    Background, Border, Color, Shadow, Theme, Vector,
    widget::{button, checkbox, container, overlay::menu, pick_list, text_input, toggler},
};

/// Section cards: a step above the page, with a subtle edge.
pub fn card(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weakest.color.into()),
        text_color: Some(p.background.weakest.text),
        border: Border { width: 1.0, radius: 8.0.into(), color: p.background.weak.color },
        ..container::Style::default()
    }
}

/// Grouped rows inside a card (a combo, a zone, a macro step).
pub fn inset(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.base.color.into()),
        text_color: Some(p.background.base.text),
        border: Border { width: 1.0, radius: 6.0.into(), color: p.background.weak.color },
        ..container::Style::default()
    }
}

/// The outputs of a "Several at once" action, and the action a Toggle or Turbo wraps: a block
/// inside an inset box, drawn as every such block is (see `panel`).
pub fn output_list(theme: &Theme) -> container::Style {
    panel(theme)
}

/// A sub-section inside an inset box (an "Advanced" fold, a stick's pictures): the card's color,
/// a step off the box around it, so it reads as its own block.
pub fn panel(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weakest.color.into()),
        text_color: Some(p.background.weakest.text),
        border: Border { width: 1.0, radius: 6.0.into(), color: p.background.strong.color },
        ..container::Style::default()
    }
}

/// A banner across the top of the window for something that needs fixing: the danger color,
/// tinted enough to read in light and dark themes.
pub fn warning_banner(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Color { a: 0.25, ..p.danger.base.color }.into()),
        text_color: Some(p.background.base.text),
        border: Border { width: 0.0, radius: 0.0.into(), color: p.danger.base.color },
        ..container::Style::default()
    }
}

/// The open look of an overlay or menu: tinted with the accent over the page, so it reads as a
/// group apart from the rows around it.
pub fn tinted(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    let accent = p.primary.base.color;
    container::Style {
        background: Some(Color { a: 0.08, ..accent }.into()),
        text_color: Some(p.background.base.text),
        border: Border { width: 1.0, radius: 6.0.into(), color: Color { a: 0.4, ..accent } },
        ..container::Style::default()
    }
}

/// The header of a fold: accent text that marks it as something to open, tinted on hover.
pub fn fold_header(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let accent = p.primary.base.color;
    let text_color = self::accent(theme);
    button::Style {
        background: hovered.then(|| Color { a: 0.08, ..accent }.into()),
        text_color,
        border: Border { radius: 5.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

/// Dialogs: raised over the dimmed page with an edge and a shadow. In dark mode the surface is a
/// step lighter than the cards, since a shadow doesn't show on a dark page.
pub fn dialog(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    let surface = if p.is_dark { p.background.weak } else { p.background.base };
    container::Style {
        background: Some(surface.color.into()),
        text_color: Some(surface.text),
        border: Border { width: 1.0, radius: 10.0.into(), color: p.background.stronger.color },
        shadow: popup_shadow(),
        ..container::Style::default()
    }
}

/// Tooltip bubbles.
pub fn tooltip(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.strong.color.into()),
        text_color: Some(p.background.strong.text),
        border: Border { width: 1.0, radius: 6.0.into(), color: p.background.stronger.color },
        shadow: popup_shadow(),
        ..container::Style::default()
    }
}

/// Sidebar entries: plain text, with the open page (and the hovered one) on a tinted
/// background, so they read as navigation rather than as buttons.
pub fn nav(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = theme.extended_palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        // The open page on a soft tint of the accent, which marks it without outweighing the page.
        let background = if selected {
            Some(Color { a: 0.45, ..p.primary.weak.color }.into())
        } else if hovered {
            Some(p.background.weak.color.into())
        } else {
            None
        };
        let text_color = p.background.base.text;
        button::Style {
            background,
            text_color,
            border: Border { radius: 6.0.into(), ..Border::default() },
            ..button::Style::default()
        }
    }
}

/// The track behind a segmented control (the section switches inside a page).
pub fn segments(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        border: Border { width: 1.0, radius: 8.0.into(), color: p.background.strong.color },
        ..container::Style::default()
    }
}

/// One segment: the chosen one is a raised chip on the track, lighter than the track in either
/// theme (the page color would be darker than the track in dark mode), with an accent edge.
pub fn segment(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = theme.extended_palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let chip = if p.is_dark { p.background.strong.color } else { p.background.base.color };
        let background = if selected {
            Some(chip.into())
        } else if hovered {
            Some(Color { a: 0.5, ..p.background.strong.color }.into())
        } else {
            None
        };
        // The accent itself is too dark on the dark chip, so dark mode lifts it most of the way to white.
        let accent = p.primary.base.color;
        let selected_text = if p.is_dark {
            let lift = |c: f32| c + (1.0 - c) * 0.65;
            Color { r: lift(accent.r), g: lift(accent.g), b: lift(accent.b), ..accent }
        } else {
            accent
        };
        button::Style {
            background,
            text_color: if selected { legible(selected_text, chip) } else { p.background.weak.text },
            border: Border {
                width: if selected { 1.0 } else { 0.0 },
                radius: 6.0.into(),
                color: Color { a: 0.6, ..p.primary.base.color },
            },
            shadow: if selected {
                Shadow { color: Color { a: 0.25, ..Color::BLACK }, offset: Vector::new(0.0, 1.0), blur_radius: 3.0 }
            } else {
                Shadow::default()
            },
            ..button::Style::default()
        }
    }
}

/// Ordinary buttons. iced's own secondary is a flat mid-gray that looks the same as a
/// disabled button; this one is a light field-like surface with an edge, so it reads as
/// clickable.
pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let (background, border_color) = match status {
        button::Status::Active => (p.background.weak.color, p.background.strong.color),
        button::Status::Hovered => (p.background.strong.color, p.primary.base.color),
        button::Status::Pressed => (p.background.stronger.color, p.primary.base.color),
        button::Status::Disabled => return disabled(theme),
    };
    button::Style {
        background: Some(background.into()),
        text_color: p.background.weak.text,
        border: Border { width: 1.0, radius: 5.0.into(), color: border_color },
        ..button::Style::default()
    }
}

/// The main action of a page or dialog (Save & apply): iced's primary, with the shared
/// disabled look instead of its pale, hard-to-read blue.
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Disabled => disabled(theme),
        _ => readable(button::primary(theme, status)),
    }
}

/// A dialog's final destructive action (Delete, Discard): iced's solid danger, with the shared
/// disabled look.
pub fn danger(theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Disabled => disabled(theme),
        _ => readable(button::danger(theme, status)),
    }
}

/// WCAG relative luminance.
fn luminance(c: Color) -> f32 {
    let lin = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio, 1 to 21.
fn contrast(a: Color, b: Color) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The contrast aimed for between button text and its fill: WCAG AA is 4.5:1, and glyph edges
/// blend into the fill, so the pixels read a little lower than the colors do.
const AA: f32 = 4.8;

/// `text` moved toward white (on a dark fill) or black (on a light one) until it reads on `fill`.
pub fn legible(text: Color, fill: Color) -> Color {
    let toward = if luminance(fill) < 0.4 { 1.0 } else { 0.0 };
    let mut c = text;
    for _ in 0..20 {
        if contrast(c, fill) >= AA {
            break;
        }
        c = Color { r: c.r + (toward - c.r) * 0.15, g: c.g + (toward - c.g) * 0.15, b: c.b + (toward - c.b) * 0.15, ..c };
    }
    c
}

/// The accent for text and small marks on the page: the theme's primary, moved until it reads
/// on the lightest surface it sits on (the card's hover fill in dark mode, the page's panel in
/// light mode).
pub fn accent(theme: &Theme) -> Color {
    let p = theme.extended_palette();
    let surface = if p.is_dark { p.background.stronger.color } else { p.background.weak.color };
    legible(p.primary.base.color, surface)
}

/// A button that is only a colored label, like a "▸ Advanced" toggle.
pub fn link(theme: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Disabled => disabled(theme),
        _ => button::Style { text_color: accent(theme), ..button::Style::default() },
    }
}

/// A solid button whose text falls short of AA: the text is moved first, and when it is already
/// white (or black) the fill is deepened a little at a time.
fn readable(mut style: button::Style) -> button::Style {
    let Some(Background::Color(mut fill)) = style.background else { return style };
    style.text_color = legible(style.text_color, fill);
    for _ in 0..20 {
        if contrast(style.text_color, fill) >= AA {
            break;
        }
        let away = if luminance(style.text_color) > luminance(fill) { 0.0 } else { 1.0 };
        fill = Color { r: fill.r + (away - fill.r) * 0.03, g: fill.g + (away - fill.g) * 0.03, b: fill.b + (away - fill.b) * 0.03, ..fill };
    }
    style.background = Some(fill.into());
    style
}

/// Every disabled button, whatever its style: no fill and a faint edge, so it doesn't read as
/// clickable, with pale text that can still be read. Paler than a field's muted text: a disabled
/// arrow next to enabled ones must look out of use at a glance.
fn disabled(theme: &Theme) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: None,
        text_color: Color { a: 0.4, ..p.background.base.text },
        border: Border { width: 1.0, radius: 5.0.into(), color: p.background.weak.color },
        ..button::Style::default()
    }
}

/// Every "Delete …" on a page (a profile, a setup, an item): a light red button that turns dark
/// red once hovered. Only a dialog's final Delete is solid red.
pub fn quiet_danger(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    // At rest: the light red fill with a red edge, and the text in a red deep enough to read on it
    // (darkened in light mode, lifted toward white in dark mode, where the fill is dark).
    let red = p.danger.base.color;
    let lift = |c: f32| c + (1.0 - c) * 0.55;
    let rest_text = if p.is_dark { Color { r: lift(red.r), g: lift(red.g), b: lift(red.b), ..red } } else { Color { r: red.r * 0.6, g: red.g * 0.6, b: red.b * 0.6, ..red } };
    let (fill, text_color, edge) = match status {
        button::Status::Hovered | button::Status::Pressed => (p.danger.strong.color, p.danger.strong.text, p.danger.strong.color),
        button::Status::Disabled => return disabled(theme),
        button::Status::Active => (p.danger.weak.color, rest_text, red),
    };
    button::Style {
        background: Some(fill.into()),
        text_color: legible(text_color, fill),
        border: Border { width: 1.0, radius: 5.0.into(), color: edge },
        ..button::Style::default()
    }
}

/// Field surface shared by dropdowns and text inputs: the page's own color with an edge, set into
/// the cards. Buttons are filled, so a field beside a button never reads as another button.
fn field(theme: &Theme, emphasized: bool) -> (Background, Border, Color) {
    let p = theme.extended_palette();
    let border_color = if emphasized { p.primary.base.color } else { p.background.strong.color };
    (
        p.background.base.color.into(),
        Border { width: if emphasized { 2.0 } else { 1.0 }, radius: 5.0.into(), color: border_color },
        p.background.base.text,
    )
}

/// A button that opens a list of its own (the font picker), drawn as a dropdown is.
pub fn field_button(theme: &Theme, status: button::Status) -> button::Style {
    let emphasized = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (background, border, text_color) = field(theme, emphasized);
    button::Style { background: Some(background), text_color, border, ..button::Style::default() }
}

pub fn dropdown(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let p = theme.extended_palette();
    let emphasized = matches!(status, pick_list::Status::Hovered | pick_list::Status::Opened { .. });
    let (background, border, text_color) = field(theme, emphasized);
    pick_list::Style {
        text_color,
        placeholder_color: muted(p.background.weak.text),
        // The arrow in the accent color marks every dropdown as one at a glance.
        handle_color: p.primary.base.color,
        background,
        border,
    }
}

/// A dropdown that runs an action ("+ New profile from…") rather than holding a value: its
/// placeholder is its label, so it's drawn in the text color instead of the faint placeholder
/// gray, which would make it look disabled.
pub fn action_dropdown(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    pick_list::Style { placeholder_color: theme.extended_palette().background.weak.text, ..dropdown(theme, status) }
}

/// Open dropdown menus: a raised surface with an accent edge and a shadow.
pub fn dropdown_menu(theme: &Theme) -> menu::Style {
    let p = theme.extended_palette();
    menu::Style {
        background: p.background.neutral.color.into(),
        border: Border { width: 1.0, radius: 6.0.into(), color: p.primary.base.color },
        text_color: p.background.neutral.text,
        selected_text_color: p.primary.strong.text,
        selected_background: p.primary.strong.color.into(),
        shadow: popup_shadow(),
    }
}

pub fn text_field(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let p = theme.extended_palette();
    let emphasized = matches!(status, text_input::Status::Hovered | text_input::Status::Focused { .. });
    let (background, border, value) = field(theme, emphasized);
    text_input::Style {
        background,
        border,
        icon: muted(p.background.weak.text),
        placeholder: muted(p.background.weak.text),
        value,
        selection: p.primary.weak.color,
    }
}

/// Readable but clearly secondary text on a field.
fn muted(text: Color) -> Color {
    Color { a: 0.6, ..text }
}

fn popup_shadow() -> Shadow {
    Shadow { color: Color { a: 0.45, ..Color::BLACK }, offset: Vector::new(0.0, 4.0), blur_radius: 14.0 }
}

/// Switches: iced's, with an edge on the track and, in dark mode, a light knob while off. Off,
/// iced's dark knob on a dark track all but disappears on a dark card.
pub fn switch(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let p = theme.extended_palette();
    let base = toggler::default(theme, status);
    let on = matches!(status, toggler::Status::Active { is_toggled: true } | toggler::Status::Hovered { is_toggled: true } | toggler::Status::Disabled { is_toggled: true });
    if on {
        return base;
    }
    let edge = Color { a: 0.45, ..p.background.base.text };
    let foreground = if p.is_dark { Color { a: 0.75, ..p.background.base.text }.into() } else { base.foreground };
    toggler::Style { background_border_width: 1.0, background_border_color: edge, foreground, ..base }
}

/// Checkboxes: iced's, with an unticked box's edge drawn from the text color so it shows on
/// cards in either theme.
pub fn tickbox(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let p = theme.extended_palette();
    let base = checkbox::primary(theme, status);
    match status {
        checkbox::Status::Active { is_checked: false } | checkbox::Status::Hovered { is_checked: false } => {
            checkbox::Style { border: Border { color: Color { a: 0.5, ..p.background.base.text }, ..base.border }, ..base }
        }
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_text_is_moved_until_it_reads() {
        let fill = Color::from_rgb8(0x9d, 0x3b, 0x3a);
        let text = legible(Color::from_rgb8(0xe3, 0xa9, 0xa8), fill);
        assert!(contrast(text, fill) >= AA);
        // Text that already reads is left alone.
        assert_eq!(legible(Color::WHITE, Color::from_rgb8(0x20, 0x20, 0x20)), Color::WHITE);
    }

    #[test]
    fn a_fill_is_deepened_when_white_text_still_falls_short() {
        let style = readable(button::Style { background: Some(Color::from_rgb8(0x58, 0x65, 0xf2).into()), text_color: Color::WHITE, ..button::Style::default() });
        let Some(Background::Color(fill)) = style.background else { panic!("a solid fill") };
        assert!(contrast(style.text_color, fill) >= AA);
    }
}
