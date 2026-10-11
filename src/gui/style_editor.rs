//! The overlay style editor: where an overlay sits, its size, shape and colors, and the
//! decorations that give one game's overlays a look of their own.

use std::ops::RangeInclusive;

use iced::widget::{column, row};

use super::*;
use crate::config::{Gradient, Shade, Stroke};

pub(super) type OnStyle<'a> = Rc<dyn Fn(OverlayStyle) -> Message + 'a>;

/// Colors offered as swatches; any other color can be typed as #rrggbb.
pub(super) const SWATCHES: [&str; 12] = [
    "#000000", "#16181c", "#30343c", "#5a606b", "#e6e8eb", "#ffffff", "#2f5db0", "#1f7a7a", "#2e7d46", "#6a3fb0",
    "#a83232", "#c26a1d",
];

/// Position, size, colors and decorations of an overlay.
#[expect(clippy::needless_pass_by_value, reason = "editors take their callbacks by value")]
pub(super) fn style_editor<'a>(style: &OverlayStyle, on_change: OnStyle<'a>) -> Element<'a, Message> {
    let set = |f: fn(&mut OverlayStyle, Paint)| setter(&on_change, style, f);
    let scale = style.scale;
    let size = stepped_slider(50.0..=200.0, scale * 100.0, format!("{:.0}%", scale * 100.0), {
        let set = set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: f32| s.scale = v);
        move |v| set((v / 100.0 * 20.0).round() / 20.0)
    });
    let shape = stepped_slider(0.0..=100.0, style.corners * 100.0, format!("{:.0}% · {}", style.corners * 100.0, corner_name(style.corners)), {
        let set = set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: f32| s.corners = v);
        move |v| set((v / 100.0 * 20.0).round() / 20.0)
    });
    let width = optional_slider(
        "Fixed",
        style.width,
        Span { default: 50.0, range: 10.0..=100.0, unit: "%" },
        set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: Option<f32>| s.width = v),
    );
    let height = optional_slider(
        "Fixed",
        style.height,
        Span { default: 50.0, range: 10.0..=100.0, unit: "%" },
        set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: Option<f32>| s.height = v),
    );
    let offset_x = percent_slider(style.x_offset, -50.0..=50.0, set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: f32| s.x_offset = v));
    let offset_y = percent_slider(style.y_offset, -50.0..=50.0, set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: f32| s.y_offset = v));
    let max_width = percent_slider(style.max_width, 10.0..=100.0, set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: f32| s.max_width = v));
    let max_height = percent_slider(style.max_height, 10.0..=100.0, set_field(on_change.clone(), style, |s: &mut OverlayStyle, v: f32| s.max_height = v));

    column![
        labeled("    Position", position_grid(style, &on_change)),
        labeled("    Size", size),
        labeled("    Width", width),
        labeled("    Height", height),
        labeled("    Offset X", offset_x),
        labeled("    Offset Y", offset_y),
        labeled("    Max width", max_width),
        labeled("    Max height", max_height),
        labeled("    Corners", shape),
        labeled("    Background", paint_row(&style.background, true, &set(|s, p| s.background = p))),
        labeled("    Items", paint_row(&style.items, true, &set(|s, p| s.items = p))),
        labeled("    Selected item", paint_row(&style.selected, true, &set(|s, p| s.selected = p))),
        decorations(style, &on_change),
    ]
    .spacing(8)
    .into()
}

/// A small "screen" of 3×3 spots to click, and the name of the chosen one.
fn position_grid<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
    let mut grid = column![].spacing(3);
    for r in 0..3 {
        let mut line = row![].spacing(3);
        for c in 0..3 {
            let pos = ScreenPosition::GRID[r * 3 + c];
            let chosen = pos == style.position;
            line = line.push(tooltip(
                button(space().width(22).height(12))
                    .padding(2)
                    .style(if chosen { button::primary } else { style::secondary })
                    .on_press(on_change(OverlayStyle { position: pos, ..style.clone() })),
                container(text(pos.to_string()).size(13)).padding(6).style(style::tooltip),
                tooltip::Position::Top,
            ));
        }
        grid = grid.push(line);
    }
    row![grid, text(style.position.to_string()).size(13).color(MUTED_COLOR)].spacing(12).align_y(Alignment::Center).into()
}

/// Where an optional decoration lives in a style, what it's called, and what it starts as when
/// it's ticked.
struct Slot<T> {
    label: &'static str,
    get: fn(&OverlayStyle) -> Option<&T>,
    put: fn(&mut OverlayStyle, Option<T>),
    start: fn(&OverlayStyle) -> T,
}

/// The decorations, each off until ticked: the panel's outline, shadow and gradient, the items'
/// outline, the selected item's glow and the panel's text color.
fn decorations<'a>(style: &OverlayStyle, on_change: &OnStyle<'a>) -> Element<'a, Message> {
    let border = Slot { label: "Panel outline", get: |s| s.border.as_ref(), put: |s, v| s.border = v, start: |_| Stroke::default() };
    let shadow = Slot { label: "Panel shadow", get: |s| s.shadow.as_ref(), put: |s, v| s.shadow = v, start: |_| Shade::panel() };
    let gradient = Slot { label: "Gradient", get: |s| s.gradient.as_ref(), put: |s, v| s.gradient = v, start: |_| Gradient::default() };
    let item_border = Slot {
        label: "Item outline",
        get: |s| s.item_border.as_ref(),
        put: |s, v| s.item_border = v,
        start: |_| Stroke { width: 1.0, ..Stroke::default() },
    };
    let glow = Slot { label: "Glow on the selected item", get: |s| s.glow.as_ref(), put: |s, v| s.glow = v, start: |s| Shade::glow(&s.selected.color) };
    let text_color = Slot { label: "Text color", get: |s| s.text.as_ref(), put: |s, v| s.text = v, start: |_| "#ffffff".to_string() };
    column![
        // Indented like the editor's labels above, so it reads as part of the editor.
        text("    Decorations").size(14).color(MUTED_COLOR),
        optional(&border, |stroke, set| stroke_rows(stroke, 0.0..=8.0, &set), style, on_change),
        optional(&shadow, |shade, set| shade_rows(shade, &set), style, on_change),
        optional(&gradient, gradient_rows, style, on_change),
        optional(&item_border, |stroke, set| stroke_rows(stroke, 0.0..=6.0, &set), style, on_change),
        optional(&glow, |shade, set| shade_rows(shade, &set), style, on_change),
        optional(
            &text_color,
            |hex, set| vec![labeled("        Color", paint_row(&Paint::new(hex, 1.0), false, &(Rc::new(move |p: Paint| set(p.color)) as Rc<dyn Fn(Paint) -> Message + 'a>)))],
            style,
            on_change,
        ),
    ]
    .spacing(8)
    .into()
}

/// A checkbox for an optional decoration, and while it's ticked, the rows `editor` makes from it
/// and a message for a changed one.
fn optional<'a, T: Clone + 'a>(
    slot: &Slot<T>,
    editor: impl Fn(&T, Rc<dyn Fn(T) -> Message + 'a>) -> Vec<Element<'a, Message>>,
    style: &OverlayStyle,
    on_change: &OnStyle<'a>,
) -> Element<'a, Message> {
    let set = set_field(on_change.clone(), style, slot.put);
    let toggle = {
        let (set, start, style) = (set.clone(), slot.start, style.clone());
        move |on: bool| set(on.then(|| start(&style)))
    };
    let value = (slot.get)(style);
    let mut rows = column![labeled("", checkbox(value.is_some()).label(slot.label).on_toggle(toggle).into())].spacing(6);
    if let Some(value) = value {
        rows = rows.extend(editor(value, Rc::new(move |v| set(Some(v)))));
    }
    rows.into()
}

/// A gradient's second color and its direction.
fn gradient_rows<'a>(gradient: &Gradient, set: Rc<dyn Fn(Gradient) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let paint: Rc<dyn Fn(Paint) -> Message + 'a> = {
        let (g, set) = (gradient.clone(), set.clone());
        Rc::new(move |p: Paint| set(Gradient { color: p.color, opacity: p.opacity, ..g.clone() }))
    };
    let angle = {
        let g = gradient.clone();
        move |v: f32| set(Gradient { angle: v, ..g.clone() })
    };
    vec![
        labeled("        Fades to", paint_row(&gradient.paint(), true, &paint)),
        labeled("        Direction", stepped_slider(0.0..=345.0, gradient.angle, direction_name(gradient.angle).into(), angle)),
    ]
}

/// A stroke's width and color.
fn stroke_rows<'a>(stroke: &Stroke, widths: RangeInclusive<f32>, set: &Rc<dyn Fn(Stroke) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let paint = {
        let (s, set) = (stroke.clone(), set.clone());
        Rc::new(move |p: Paint| {
            let mut s = s.clone();
            s.set_paint(p);
            set(s)
        }) as Rc<dyn Fn(Paint) -> Message + 'a>
    };
    let width = {
        let (s, set) = (stroke.clone(), set.clone());
        move |v: f32| set(Stroke { width: (v * 2.0).round() / 2.0, ..s.clone() })
    };
    vec![
        labeled("        Width", stepped_slider(widths, stroke.width, format!("{} px", stroke.width), width)),
        labeled("        Color", paint_row(&stroke.paint(), true, &paint)),
    ]
}

/// A shadow's or glow's color, blur and drop.
fn shade_rows<'a>(shade: &Shade, set: &Rc<dyn Fn(Shade) -> Message + 'a>) -> Vec<Element<'a, Message>> {
    let field = |f: fn(&mut Shade, f32)| {
        let (s, set) = (shade.clone(), set.clone());
        move |v: f32| {
            let mut s = s.clone();
            f(&mut s, v.round());
            set(s)
        }
    };
    let paint = {
        let (s, set) = (shade.clone(), set.clone());
        Rc::new(move |p: Paint| {
            let mut s = s.clone();
            s.set_paint(p);
            set(s)
        }) as Rc<dyn Fn(Paint) -> Message + 'a>
    };
    vec![
        labeled("        Color", paint_row(&shade.paint(), true, &paint)),
        labeled("        Blur", stepped_slider(0.0..=80.0, shade.blur, format!("{:.0} px", shade.blur), field(|s, v| s.blur = v))),
        labeled("        Drop", stepped_slider(-30.0..=30.0, shade.y, format!("{:.0} px", shade.y), field(|s, v| s.y = v))),
    ]
}

/// A message for `on_change` with one color of `style` replaced.
fn setter<'a>(on_change: &OnStyle<'a>, style: &OverlayStyle, set: fn(&mut OverlayStyle, Paint)) -> Rc<dyn Fn(Paint) -> Message + 'a> {
    let (on_change, style) = (on_change.clone(), style.clone());
    Rc::new(move |p| {
        let mut s = style.clone();
        set(&mut s, p);
        on_change(s)
    })
}

/// Swatches, a #rrggbb field and (when `opacity`) an opacity slider for one color.
pub(super) fn paint_row<'a>(paint: &Paint, opacity: bool, on_paint: &Rc<dyn Fn(Paint) -> Message + 'a>) -> Element<'a, Message> {
    let mut swatches = row![].spacing(4).align_y(Alignment::Center);
    for hex in SWATCHES {
        let [r, g, b] = crate::config::parse_hex(hex).unwrap_or_default();
        let color = Color::from_rgb8(r, g, b);
        let chosen = paint.color.eq_ignore_ascii_case(hex);
        swatches = swatches.push(
            button(space().width(16).height(16))
                .padding(0)
                .style(move |_, _| button::Style {
                    background: Some(color.into()),
                    border: iced::Border {
                        width: if chosen { 3.0 } else { 1.0 },
                        radius: 4.0.into(),
                        color: if chosen { Color::from_rgb8(0x4e, 0xa1, 0xff) } else { Color::from_rgb(0.5, 0.5, 0.5) },
                    },
                    ..button::Style::default()
                })
                .on_press(on_paint(Paint { color: hex.into(), ..paint.clone() })),
        );
    }
    let typed = {
        let (on_paint, alpha) = (on_paint.clone(), paint.opacity);
        move |hex: String| on_paint(Paint { color: hex, opacity: alpha })
    };
    let mut line = row![swatches, field("#rrggbb", &paint.color).on_input(typed).width(90)].spacing(8).align_y(Alignment::Center);
    if opacity {
        let (color, on_paint) = (paint.color.clone(), on_paint.clone());
        let slide = move |v: f32| on_paint(Paint { color: color.clone(), opacity: (v / 100.0).clamp(0.0, 1.0) });
        line = line
            .push(text("Opacity").size(13))
            .push(slider(0.0..=100.0, paint.opacity * 100.0, slide).step(5.0_f32).width(110))
            .push(text(format!("{:.0}%", paint.opacity * 100.0)).size(13));
    }
    if paint.rgb().is_none() {
        line = line.push(text("not a #rrggbb color").size(12).color(ERROR_COLOR));
    }
    line.into()
}

/// A slider in steps of 5 with its value written beside it.
fn stepped_slider<'a>(range: RangeInclusive<f32>, value: f32, shown: String, on_change: impl Fn(f32) -> Message + 'a) -> Element<'a, Message> {
    row![slider(range, value, on_change).step(5.0_f32).width(220), text(shown).size(13)].spacing(10).align_y(Alignment::Center).into()
}

/// A slider for a number of percent of the display.
fn percent_slider<'a>(value: f32, range: RangeInclusive<f32>, on_change: impl Fn(f32) -> Message + 'a) -> Element<'a, Message> {
    row![slider(range, value, on_change).step(1.0_f32).width(220), text(format!("{value:.0}%")).size(13)]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

/// What a corners setting looks like, from square to circle.
fn corner_name(corners: f32) -> &'static str {
    match corners {
        c if c < 0.1 => "Square",
        c if c < 0.5 => "Rounded",
        c if c < 0.95 => "Squircle",
        _ => "Circle",
    }
}

/// Which way a gradient at `angle` degrees runs, to the nearest eighth of a turn.
fn direction_name(angle: f32) -> &'static str {
    const NAMES: [&str; 8] = ["Upward", "To top right", "Rightward", "To bottom right", "Downward", "To bottom left", "Leftward", "To top left"];
    NAMES[((angle.rem_euclid(360.0) + 22.5) / 45.0) as usize % 8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_directions_are_named_by_the_nearest_eighth() {
        assert_eq!(direction_name(0.0), "Upward");
        assert_eq!(direction_name(180.0), "Downward");
        assert_eq!(direction_name(100.0), "Rightward");
        assert_eq!(direction_name(350.0), "Upward");
    }
}
