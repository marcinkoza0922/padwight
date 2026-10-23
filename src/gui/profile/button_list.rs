//! The Buttons tab's list: a row per button with its glyph and name, then what it does, with
//! its gestures in rows underneath.

use iced::widget::{column, row};

use super::*;
use crate::config::GuideInput;

/// The buttons the list shows, in groups set apart by a gap: face buttons, bumpers, Back and
/// Start, stick clicks, D-pad. Guide is built in: the Guide tab says what it does, so it has no
/// row here.
const GROUPS: [&[Button]; 5] = [
    &[Button::South, Button::East, Button::North, Button::West],
    &[Button::LeftBumper, Button::RightBumper],
    &[Button::Select, Button::Start],
    &[Button::LeftStick, Button::RightStick],
    &[Button::DpadUp, Button::DpadDown, Button::DpadLeft, Button::DpadRight],
];

/// The name column: chevron, glyph and name.
const NAME_WIDTH: f32 = 250.0;
/// The glyph's slot, so the names after the glyphs line up however wide each glyph is.
const GLYPH_WIDTH: f32 = 64.0;
const CHEVRON_WIDTH: f32 = 12.0;
/// In a layer, the slot at the end of each row for "Back to base" or "Override".
const LAYER_WIDTH: f32 = 110.0;

pub(in crate::gui) fn button_rows<'a>(p: &'a Profile, ui: &Ui) -> Vec<Element<'a, Message>> {
    let searching = !ui.query.trim().is_empty();
    let groups: Vec<Vec<Button>> =
        GROUPS.iter().map(|g| g.iter().copied().filter(|b| row_matches(p, *b, ui)).collect::<Vec<_>>()).filter(|g| !g.is_empty()).collect();
    let shown: Vec<Button> = groups.iter().flatten().copied().collect();
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    rows.extend(layer_note(&shown, ui));
    for (n, group) in groups.iter().enumerate() {
        let list = column(group.iter().flat_map(|b| button_row(p, *b, ui))).spacing(10);
        // The gap before a group is a little wider than the one between its rows.
        rows.push(container(list).padding(iced::Padding::default().top(if n > 0 { 6 } else { 0 })).into());
    }
    // The timings only shape gestures, so they sit under the rows rather than ahead of them.
    // Layers use their profile's.
    if ui.layer.is_none() && !searching {
        rows.push(rule::horizontal(1).into());
        rows.push(value_slider("Tap window", 100.0..=600.0, p.tap_window_ms as f32, 10.0, "ms", Message::SetTapWindow));
        rows.push(value_slider("Long press after", 200.0..=1500.0, p.long_press_ms as f32, 50.0, "ms", Message::SetLongPress));
    }
    rows
}

/// The back paddles' mappings, with the box saying whether the profile needs them. Layers
/// can override them like any button, so the box is left out there.
pub(in crate::gui) fn paddle_rows<'a>(p: &'a Profile, ui: &Ui) -> Vec<Element<'a, Message>> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    if ui.layer.is_none() && ui.query.trim().is_empty() {
        if !ui.any_paddles {
            rows.push(
                note("None of your managed controllers has back paddles. These mappings apply once one is connected.").into(),
            );
        }
        rows.extend(requirement_rows(p, Feature::BackPaddles));
    }
    let shown: Vec<Button> = Button::PADDLES.into_iter().filter(|b| row_matches(p, *b, ui)).collect();
    rows.extend(layer_note(&shown, ui));
    for b in shown {
        rows.extend(button_row(p, b, ui));
    }
    rows
}

/// Whether button `b`'s row has every word the search box holds: in its name, its action or a gesture's.
pub(super) fn row_matches(p: &Profile, b: Button, ui: &Ui) -> bool {
    if ui.query.trim().is_empty() {
        return true;
    }
    let base = ui.layer.map_or(p, |m| m.base);
    let mut haystack = format!("{} {} {}", button_name(b, ui.in_use), short_button(b), summarize(p.button(b)));
    for profile in [p, base] {
        haystack.push(' ');
        haystack.push_str(&summarize(profile.button(b)));
        for (kind, action) in GestureKind::ALL.into_iter().filter_map(|k| profile.gestures.get(&b).and_then(|g| g.get(k)).map(|a| (k, a))) {
            haystack.push_str(&format!(" {kind} {}", summarize(action)));
        }
    }
    matches_query(ui.query, &haystack)
}

/// The profile a row shows: in a layer that doesn't override the button, the profile it's over.
fn source<'p>(p: &'p Profile, b: Button, layer: Option<LayerMarks<'p>>) -> &'p Profile {
    match layer {
        Some(marks) if !marks.overrides(LayerPart::Button(b)) => marks.base,
        _ => p,
    }
}

/// In a layer, what the muted rows are, when some of `shown` are the profile's own.
fn layer_note<'a>(shown: &[Button], ui: &Ui) -> Option<Element<'a, Message>> {
    let marks = ui.layer?;
    shown
        .iter()
        .any(|b| !marks.overrides(LayerPart::Button(*b)))
        .then(|| note(format!("Muted rows are the same as {}. Override one to change it while this layer is on.", marks.base.name)).into())
}

/// The action a button does when nothing remaps it: its own, or nothing for a paddle.
fn usual(b: Button) -> ButtonAction {
    if Button::PADDLES.contains(&b) || b.stick_direction().is_some() { ButtonAction::Disabled } else { ButtonAction::Gamepad(b) }
}

/// A row's name column: its glyph as the overlays draw it, then its name. `toggle` makes it a
/// button that opens and closes the row; without it, a chevron's width of space keeps it in line.
fn name_column<'a>(b: Button, ui: &Ui, toggle: Option<bool>, color: Option<Color>) -> Element<'a, Message> {
    let chevron: Element<'a, Message> = match toggle {
        Some(open) => text(if open { "▾" } else { "▸" }).size(14).width(CHEVRON_WIDTH).into(),
        None => space().width(CHEVRON_WIDTH).into(),
    };
    let glyph = container(input_chip(GuideInput::Button(b), ui.family, ui.swap)).center_x(GLYPH_WIDTH);
    let content = row![chevron, glyph, text(button_name(b, ui.in_use)).color_maybe(color)].spacing(4).align_y(Alignment::Center);
    let Some(_) = toggle else { return container(content).padding([4, 0]).width(NAME_WIDTH).into() };
    button(content).style(button::text).padding([4, 0]).width(NAME_WIDTH).on_press(Message::ToggleExpanded(Target::Button(b))).into()
}

/// One button: collapsed to its columns, or open with its action editor, gesture rows and a
/// "+ Double tap" style button for each gesture not yet set.
fn button_row<'a>(p: &'a Profile, b: Button, ui: &Ui) -> Vec<Element<'a, Message>> {
    let from_base = ui.layer.is_some_and(|marks| !marks.overrides(LayerPart::Button(b)));
    let target = Target::Button(b);
    if from_base || !ui.is_open(target) {
        return collapsed_row(p, b, ui, from_base);
    }
    open_row(p, b, ui)
}

/// A closed row: the name and the button's action, then a row under it per gesture. In a layer that
/// doesn't override it, the profile's own, muted, with "Override".
fn collapsed_row<'a>(p: &'a Profile, b: Button, ui: &Ui, from_base: bool) -> Vec<Element<'a, Message>> {
    let shown = source(p, b, ui.layer);
    let gestures = shown.gestures.get(&b);
    let gesture = |k: GestureKind| gestures.and_then(|g| g.get(k));
    let problem = if from_base {
        None
    } else {
        std::iter::once(shown.button(b)).chain(GestureKind::ALL.into_iter().filter_map(gesture)).find_map(|a| action_problem(a, ui.names))
    };
    let unchanged = *shown.button(b) == usual(b) && gestures.is_none_or(crate::config::Gestures::is_empty);
    let name_color = if problem.is_some() {
        Some(ERROR_COLOR)
    } else {
        (unchanged || from_base).then_some(MUTED_COLOR)
    };
    // A button left as itself would show only its own glyph, which reads like the button's icon
    // rather than what it does, so it says so in words.
    let press: Element<'a, Message> = if *shown.button(b) == ButtonAction::Gamepad(b) {
        text("Same button").color(MUTED_COLOR).into()
    } else {
        piece_wrap(shown.button(b).pieces(), ui.family, ui.swap, from_base || matches!(shown.button(b), ButtonAction::Disabled))
    };
    let mut line = row![
        name_column(b, ui, (!from_base).then_some(false), name_color),
        container(press).width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if let Some(marks) = ui.layer {
        let (label, style, message): (_, fn(&iced::Theme, button::Status) -> button::Style, _) = if from_base {
            ("Override", style::secondary, Message::OverrideInput(LayerPart::Button(b)))
        } else {
            ("Back to base", button::text, Message::RevertInput(LayerPart::Button(b)))
        };
        let control = button(text(label).size(13)).style(style).on_press(message);
        let control: Element<'a, Message> = if from_base {
            tooltip(
                control,
                container(text(format!("Same as {}. Override to change it in this layer.", marks.base.name)).size(13)).padding(8).style(style::tooltip),
                tooltip::Position::Top,
            )
            .into()
        } else {
            control.into()
        };
        line = line.push(container(control).width(LAYER_WIDTH).align_right(LAYER_WIDTH));
    }
    // Each gesture sits under the action it extends, indented to the glyph and named in muted text.
    let mut lines: Vec<Element<'a, Message>> = vec![line.into()];
    for (kind, action) in GestureKind::ALL.into_iter().filter_map(|k| gesture(k).map(|a| (k, a))) {
        let name = container(text(kind.to_string()).size(13).color(MUTED_COLOR)).padding(iced::Padding::default().left(CHEVRON_WIDTH + 24.0)).width(NAME_WIDTH);
        let mut line = row![name, container(piece_wrap(action.pieces(), ui.family, ui.swap, from_base)).width(Length::Fill)]
            .spacing(10)
            .align_y(Alignment::Center);
        if ui.layer.is_some() {
            line = line.push(space().width(LAYER_WIDTH));
        }
        lines.push(line.into());
    }
    let mut rows = vec![column(lines).spacing(2).into()];
    // On a line of its own, so it doesn't push this row's columns out of line with the others.
    if let Some(problem) = problem {
        rows.push(labeled_in(NAME_WIDTH, "", text(format!("⚠ {problem}")).size(12).color(ERROR_COLOR).into()));
    }
    rows
}

/// An open row: the name on its own line, then a box with the action's editor, a row per
/// gesture and the buttons that add the rest.
fn open_row<'a>(p: &'a Profile, b: Button, ui: &Ui) -> Vec<Element<'a, Message>> {
    let target = Target::Button(b);
    let gestures = p.gestures.get(&b);
    let set_gestures: Vec<(GestureKind, &'a ButtonAction)> =
        GestureKind::ALL.into_iter().filter_map(|k| gestures.and_then(|g| g.get(k)).map(|a| (k, a))).collect();
    let problem = std::iter::once(p.button(b)).chain(set_gestures.iter().map(|(_, a)| *a)).find_map(|a| action_problem(a, ui.names));
    let missing: Vec<GestureKind> = GestureKind::ALL.into_iter().filter(|k| gestures.and_then(|g| g.get(*k)).is_none()).collect();
    // The name gets a line of its own so the open editor reads as sitting under it.
    let mut header = row![name_column(b, ui, Some(true), problem.is_some().then_some(ERROR_COLOR)), space::horizontal()]
        .spacing(10)
        .align_y(Alignment::Center);
    if ui.layer.is_some() {
        header = header.push(button(text("Back to base").size(13)).style(button::text).on_press(Message::RevertInput(LayerPart::Button(b))));
    }
    let mut rows: Vec<Element<'a, Message>> = vec![labeled(
        "    Press",
        fill_x(action_editor(p.button(b), b, &ACTION_KINDS, set_action(target), KeyField::root(target), ui.names)),
    )];
    rows.extend(menu_combos::offer(p, &[b], p.button(b), ui).map(|o| labeled("", o)));

    for (kind, action) in set_gestures {
        rows.push(rule::horizontal(1).into());
        // Laid out like the Press row, with a ✕ right after the action it removes. The row takes
        // the free width, so the action still wraps there in a narrow window.
        let target = Target::Gesture(b, kind);
        rows.push(labeled(
            format!("    {kind}"),
            fill_x(
                row![
                    action_editor(action, b, &ACTION_KINDS, set_action(target), KeyField::root(target), ui.names),
                    button(text("✕").size(13)).style(style::secondary).on_press(Message::RemoveGesture(b, kind)),
                ]
                .spacing(8)
                .align_y(Alignment::Start),
            ),
        ));
        if kind == GestureKind::LongPress {
            rows.extend(menu_combos::offer(p, &[b], action, ui).map(|o| labeled("", o)));
        }
    }
    // Adding a gesture sits under the rows it extends, one click per kind.
    if !missing.is_empty() {
        rows.push(rule::horizontal(1).into());
        let adds = missing.into_iter().map(|k| {
            button(text(format!("+ {k}")).size(13)).style(style::secondary).padding([3, 10]).on_press(Message::AddGesture(b, k)).into()
        });
        rows.push(labeled("", row(adds).spacing(6).into()));
    }
    if let Some(problem) = problem {
        rows.push(labeled("", text(format!("⚠ {problem}")).size(12).color(ERROR_COLOR).into()));
    }
    // The details sit in a box under the name, like a combo's, picked out in the accent color
    // when the button was just found. The name stays outside so it doesn't move on opening.
    let found = ui.found == Some(b);
    let details = container(column(rows).spacing(8)).padding(10).width(Length::Fill).style(move |t| {
        let mut s = style::inset(t);
        if found {
            s.border.color = style_accent();
            s.border.width = 2.0;
        }
        s
    });
    vec![column![header, details].spacing(4).into()]
}
