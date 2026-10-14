//! The Guide overlay pages of Edit Controls: the profile's notes for the Notes overlay, line by
//! line, and the rows of its Mappings list: each row's text, hiding it, sharing it with other
//! inputs, and rows of the author's own.

use crate::{
    config::{Button, Config, EditableRow, GuideInput, RowId, RowKey, Stick, Trigger, default_text, editable_rows},
    menu::Tone,
    system_menu::active_profile_mut,
};

use super::{
    pages::Page,
    rows::{Act, Nav, Row},
};

/// The Guide page: the notes, and the Mappings rows.
pub(super) fn guide_rows(config: &Config) -> Vec<Row> {
    let Some(p) = config.active() else { return Vec::new() };
    let lines = p.guide.notes.lines().count();
    let rows = editable_rows(p);
    let hidden = rows.iter().filter(|r| r.hidden).count();
    vec![
        Row::open(format!("Notes ({lines} lines)"), Page::GuideNotes),
        Row::open(format!("Mappings ({} rows, {hidden} hidden)", rows.len()), Page::GuideRows),
    ]
}

fn notes(config: &Config) -> Vec<String> {
    config.active().map(|p| p.guide.notes.lines().map(str::to_string).collect()).unwrap_or_default()
}

fn set_notes(config: &mut Config, lines: &[String]) -> bool {
    let Some(p) = active_profile_mut(config) else { return false };
    let notes = lines.join("\n");
    if p.guide.notes == notes {
        return false;
    }
    p.guide.notes = notes;
    true
}

/// The notes, a line a row: each line typed on its own, adding one, removing the last.
pub(super) fn notes_rows(config: &Config) -> Vec<Row> {
    let lines = notes(config);
    let mut rows: Vec<Row> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            Row::text(if line.is_empty() { "(empty line)".to_string() } else { line.clone() }, format!("Notes, line {}", i + 1), line.clone(), move |c, typed| {
                let mut lines = notes(c);
                match lines.get_mut(i) {
                    Some(line) => *line = typed.to_string(),
                    None => return false,
                }
                set_notes(c, &lines)
            })
        })
        .collect();
    let next = lines.len();
    rows.push(
        Row::text("Add a line", format!("Notes, line {}", next + 1), "", move |c, typed| {
            let mut lines = notes(c);
            lines.push(typed.to_string());
            set_notes(c, &lines)
        })
        .tone(Tone::Add),
    );
    if !lines.is_empty() {
        rows.push(Row::apply("Remove the last line", |c| {
            let mut lines = notes(c);
            lines.pop();
            Act::changed_if(set_notes(c, &lines))
        }).tone(Tone::Remove));
    }
    rows
}

/// The label of a guide input: a button's glyph (in `buttons`), or a trigger's or stick's token.
fn input_token(input: GuideInput) -> Option<&'static str> {
    match input {
        GuideInput::Button(_) => None,
        GuideInput::Trigger(Trigger::Left) => Some("{lt}"),
        GuideInput::Trigger(Trigger::Right) => Some("{rt}"),
        GuideInput::Stick(Stick::Left) => Some("{ls}"),
        GuideInput::Stick(Stick::Right) => Some("{rs}"),
    }
}

/// A label naming `inputs`: the triggers' and sticks' tokens, then `text`; and the buttons, drawn
/// as glyphs in front.
fn inputs_label(inputs: &[GuideInput], text: &str) -> (String, Vec<Button>) {
    let buttons: Vec<Button> = inputs.iter().filter_map(|i| if let GuideInput::Button(b) = i { Some(*b) } else { None }).collect();
    let tokens: Vec<&str> = inputs.iter().filter_map(|i| input_token(*i)).collect();
    let label = if tokens.is_empty() { text.to_string() } else { format!("{} {text}", tokens.join(" ")) };
    (label, buttons)
}

/// The Mappings rows, each opening its own page, and a row to add one.
pub(super) fn mapping_rows(config: &Config) -> Vec<Row> {
    let Some(p) = config.active() else { return Vec::new() };
    let mut rows: Vec<Row> = editable_rows(p)
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let text = if r.hidden { format!("{} (hidden)", shown_text(r)) } else { shown_text(r) };
            let (label, buttons) = inputs_label(&r.inputs, &text);
            Row::open(label, Page::GuideRow(i)).buttons(buttons)
        })
        .collect();
    rows.push(Row::add("Add a row", |c| {
        let Some(p) = active_profile_mut(c) else { return Act::stay() };
        p.guide.add_custom(String::new());
        let at = editable_rows(p).len() - 1;
        Act::changed().then(Nav::Open(Page::GuideRow(at)))
    }));
    rows
}

fn shown_text(r: &EditableRow) -> String {
    if r.text.is_empty() { "(no text)".into() } else { r.text.clone() }
}

fn row_at(config: &Config, i: usize) -> Option<EditableRow> {
    editable_rows(config.active()?).into_iter().nth(i)
}

/// A Mappings row's page: its text, whether it's shown, sharing it, and splitting or removing it.
pub(super) fn row_rows(config: &Config, i: usize) -> Vec<Row> {
    let Some(r) = row_at(config, i) else { return Vec::new() };
    let key = r.key.clone();
    let k = key.clone();
    let mut rows = vec![Row::text(format!("Text: {}", shown_text(&r)), "Text of the row (empty for the usual)", r.text.clone(), move |c, typed| {
        let Some(p) = active_profile_mut(c) else { return false };
        let before = p.guide.clone();
        p.guide.set_text(k.clone(), typed.trim().to_string());
        p.guide != before
    })];
    if !matches!(key, RowKey::Custom(_)) {
        let k = key.clone();
        rows.push(Row::toggle("Shown", !r.hidden, move |c, on| {
            let Some(p) = active_profile_mut(c) else { return false };
            let before = p.guide.clone();
            p.guide.set_hidden(k.clone(), !on);
            p.guide != before
        }));
    }
    if matches!(key, RowKey::Row(RowId::Input(_)) | RowKey::Merged(_) | RowKey::Custom(_)) {
        rows.push(Row::open("Share it with another input…", Page::GuideShare(i)));
    }
    match key {
        RowKey::Merged(m) => rows.push(Row::apply("Split it into rows of their own", move |c| {
            let Some(p) = active_profile_mut(c) else { return Act::stay() };
            p.guide.split(m);
            Act::changed().then(Nav::Back(1))
        }).tone(Tone::Remove)),
        RowKey::Custom(m) => rows.push(Row::delete("Remove the row", move |c| {
            let Some(p) = active_profile_mut(c) else { return false };
            p.guide.remove_custom(m);
            true
        })),
        RowKey::Row(_) => {}
    }
    rows
}

/// Every input a row can name: the buttons (Guide excepted), the triggers and the sticks.
fn all_inputs() -> Vec<GuideInput> {
    let mut inputs: Vec<GuideInput> = Button::ALL.into_iter().filter(|b| *b != Button::Guide).map(GuideInput::Button).collect();
    inputs.extend([Trigger::Left, Trigger::Right].map(GuideInput::Trigger));
    inputs.extend([Stick::Left, Stick::Right].map(GuideInput::Stick));
    inputs
}

/// The inputs row `i` can share with: those with a row of their own, or for an author's row, any.
pub(super) fn share_rows(config: &Config, i: usize) -> Vec<Row> {
    let (Some(p), Some(r)) = (config.active(), row_at(config, i)) else { return Vec::new() };
    let rows = editable_rows(p);
    let others: Vec<(GuideInput, String)> = match r.key {
        RowKey::Custom(_) => all_inputs().into_iter().filter(|x| !r.inputs.contains(x)).map(|x| (x, String::new())).collect(),
        _ => rows
            .iter()
            .filter_map(|o| match o.key {
                RowKey::Row(RowId::Input(input)) if !r.inputs.contains(&input) => Some((input, o.default_label.clone())),
                _ => None,
            })
            .collect(),
    };
    if others.is_empty() {
        return vec![Row::note("No other input to share it with")];
    }
    others
        .into_iter()
        .map(|(other, label)| {
            let key = r.key.clone();
            let (label, buttons) = inputs_label(&[other], &label);
            Row::apply(label, move |c| share(c, &key, other)).buttons(buttons)
        })
        .collect()
}

/// Adds `other` to the row `key`: a merged or author's row takes it in, and an input's own row
/// becomes a merged row of both.
fn share(config: &mut Config, key: &RowKey, other: GuideInput) -> Act {
    let Some(p) = active_profile_mut(config) else { return Act::stay() };
    match key {
        RowKey::Merged(i) => p.guide.add_to_merge(*i, other),
        RowKey::Custom(i) => p.guide.add_custom_input(*i, other),
        RowKey::Row(RowId::Input(input)) => {
            let text = default_text(p, *input);
            p.guide.merge(vec![*input, other], text);
        }
        RowKey::Row(_) => return Act::stay(),
    }
    // The row may have moved in the list: back to the list, which shows where it went.
    Act::changed().then(Nav::Back(2))
}
