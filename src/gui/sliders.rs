//! Labeled sliders with their value and unit, shared by every page.

use iced::widget::row;

use super::*;

/// A slider's value and unit: "250 ms", but "45°", "120°/s turning" and "50%", since a degree or
/// percent sign goes against its number.
fn slider_value(value: f32, step: f32, unit: &str) -> String {
    let number = if step >= 1.0 { format!("{value:.0}") } else { format!("{value:.2}") };
    if unit.is_empty() || unit.starts_with(['°', '%']) { format!("{number}{unit}") } else { format!("{number} {unit}") }
}

/// A share of something (0..1) as a percentage, then what it's a share of: "25% of the way
/// out", "12%".
fn fraction_value(value: f32, unit: &str) -> String {
    let percent = format!("{:.0}%", value * 100.0);
    if unit.is_empty() { percent } else { format!("{percent} {unit}") }
}

/// A slider over a share (0..1), shown as a percentage: the way every share in the window reads.
pub(super) fn fraction_editor<'a>(
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    step: f32,
    unit: &'a str,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    row![slider(range, value, on_change).step(step).width(300), text(fraction_value(value, unit)).size(13)]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

#[expect(clippy::too_many_arguments, reason = "predates the size lints")]
pub(super) fn value_slider<'a>(
    label: &'a str,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    step: f32,
    unit: &'a str,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    labeled(label, slider_editor(range, value, step, unit, on_change))
}

/// A slider with its value and unit: the editor part of a `value_slider` row.
pub(super) fn slider_editor<'a>(
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    step: f32,
    unit: &'a str,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    row![slider(range, value, on_change).step(step).width(300), text(slider_value(value, step, unit)).size(13)]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use super::slider_value;

    #[test]
    fn units_are_spaced_but_degrees_are_not() {
        assert_eq!(slider_value(250.0, 10.0, "ms"), "250 ms");
        assert_eq!(slider_value(0.0, 15.0, "° from up"), "0° from up");
        assert_eq!(slider_value(120.0, 10.0, "°/s turning"), "120°/s turning");
        assert_eq!(slider_value(0.5, 0.1, "°/s"), "0.50°/s");
        assert_eq!(slider_value(8.0, 1.0, ""), "8");
        assert_eq!(slider_value(50.0, 5.0, "%"), "50%");
    }

    #[test]
    fn shares_read_as_percentages() {
        assert_eq!(super::fraction_value(0.25, "of the way out"), "25% of the way out");
        assert_eq!(super::fraction_value(0.12, ""), "12%");
    }
}
