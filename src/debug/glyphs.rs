//! Every glyph the overlays draw, as info overlays drawn the way the overlay draws them: each
//! controller family's buttons, every keyboard key, the mouse buttons and the icons. The debug
//! window shows them, and `padwight debug overlay glyphs` draws them to an image.

use std::collections::HashSet;

use iced::{
    Alignment, Element,
    widget::{button, checkbox, column, pick_list, row, text},
};

use crate::{
    config::{Button, GestureKind, MouseButton, OverlayStyle, Trigger},
    info::{self, Charge, FormFactor, Icon, InfoView, PadFamily, Segment},
    keyboard,
    motion::Anim,
    overlay::{draw, fit::Fit},
};

/// How many keycaps a row of the keyboard sheet holds.
const KEYS_PER_ROW: usize = 12;

/// The debug window's glyph section: every sheet, in a font and layout picked there.
#[derive(Default)]
pub struct GlyphPanel {
    shown: bool,
    nintendo_layout: bool,
    font: FontChoice,
}

#[derive(Debug, Clone, Copy)]
pub enum GlyphMessage {
    Show(bool),
    NintendoLayout(bool),
    Font(FontChoice),
}

/// A font the overlays can be drawn in: the system's, or a bundled one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontChoice(Option<&'static str>);

impl std::fmt::Display for FontChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.unwrap_or("System font"))
    }
}

impl GlyphPanel {
    pub fn update(&mut self, message: GlyphMessage) {
        match message {
            GlyphMessage::Show(on) => self.shown = on,
            GlyphMessage::NintendoLayout(on) => self.nintendo_layout = on,
            GlyphMessage::Font(font) => self.font = font,
        }
    }

    pub fn view(&self) -> Element<'_, GlyphMessage> {
        let fonts: Vec<FontChoice> = std::iter::once(FontChoice(None)).chain(crate::font::BUNDLED.iter().map(|b| FontChoice(Some(b.name)))).collect();
        let controls = row![
            text("Glyphs").size(20),
            button(if self.shown { "Hide" } else { "Show" }).on_press(GlyphMessage::Show(!self.shown)),
            checkbox(self.nintendo_layout).label("Nintendo layout").on_toggle(GlyphMessage::NintendoLayout),
            pick_list(fonts, Some(self.font), GlyphMessage::Font),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        if !self.shown {
            return controls.into();
        }
        let font = crate::font::resolve(self.font.0);
        let sheets = sheets(self.nintendo_layout).into_iter().map(|v| draw::info_panel(&v, font, &Fit::contents(), &Anim::still()));
        column![
            controls,
            text("Every glyph the overlays draw, as info overlays draw them: each controller's buttons, every key, the mouse buttons and the icons. padwight debug overlay glyphs draws the same to a PNG.").size(12),
            row(sheets).spacing(12).wrap().vertical_spacing(12),
        ]
        .spacing(8)
        .into()
    }
}

/// Every sheet: one per controller family (face buttons swapped when `nintendo_layout`), then the
/// keyboard, the mouse and the icons.
pub fn sheets(nintendo_layout: bool) -> Vec<InfoView> {
    let mut sheets: Vec<InfoView> = PadFamily::ALL.iter().map(|&family| buttons(family, nintendo_layout)).collect();
    sheets.extend([keys(), mouse(), gestures(), icons()]);
    sheets
}

/// A sheet: a title, then rows of a name and its glyphs.
fn sheet(title: &str, rows: Vec<(String, Vec<Segment>)>) -> InfoView {
    InfoView {
        name: title.into(),
        style: OverlayStyle { scale: 1.0, ..OverlayStyle::info() },
        title: Some(vec![Segment::Text(title.into())]),
        rows: rows.into_iter().map(|(name, glyphs)| vec![vec![Segment::Text(name)], glyphs]).collect(),
        opacity: 1.0,
    }
}

/// A family's buttons, triggers, sticks and D-pad, as `{tokens}` and menus draw them.
fn buttons(family: PadFamily, swapped: bool) -> InfoView {
    let glyphs = |list: &[Button]| list.iter().map(|&b| info::button_glyph(b, family, swapped)).collect::<Vec<_>>();
    let mut shoulders = glyphs(&[Button::LeftBumper, Button::RightBumper]);
    shoulders.extend([Trigger::Left, Trigger::Right].map(|t| info::trigger_glyph(t, family)));
    // `{ls}`, `{rs}` and `{dpad}` draw these whatever the family.
    let mut sticks = vec![info::glyph("LS", None, true), info::glyph("RS", None, true)];
    sticks.extend(glyphs(&[Button::LeftStick, Button::RightStick]));
    let mut dpad = vec![info::glyph("✚", None, false)];
    dpad.extend(glyphs(&[Button::DpadUp, Button::DpadDown, Button::DpadLeft, Button::DpadRight]));
    // Diagonals, as the input log lights two arms.
    dpad.extend([[true, false, true, false], [true, false, false, true], [false, true, true, false], [false, true, false, true]].map(Segment::Dpad));
    let rows = vec![
        ("Face".into(), glyphs(&Button::FACE)),
        ("Shoulders".into(), shoulders),
        ("Middle".into(), glyphs(&[Button::Select, Button::Start, Button::Guide])),
        ("Sticks".into(), sticks),
        ("D-pad".into(), dpad),
        ("Stick directions".into(), glyphs(&Button::EVERY[Button::ALL.len()..])),
        ("Paddles".into(), glyphs(&Button::PADDLES)),
    ];
    let layout = if swapped { ", Nintendo layout" } else { "" };
    sheet(&format!("{family}{layout}"), rows)
}

/// The gesture badges a row's mark draws: `{gesture:…}`.
fn gestures() -> InfoView {
    let rows = [("Double tap", GestureKind::DoubleTap), ("Triple tap", GestureKind::TripleTap), ("Long press", GestureKind::LongPress)]
        .map(|(name, kind)| (name.to_string(), vec![info::gesture_glyph(kind)]))
        .to_vec();
    sheet("Gestures", rows)
}

/// Every key a keyboard layout draws, each once, as `{keyboard:…}` draws it.
fn keys() -> InfoView {
    let mut seen = HashSet::new();
    let codes: Vec<&str> = keyboard::drawn_keys().map(|k| k.code).filter(|code| seen.insert(*code)).collect();
    let rows = codes
        .chunks(KEYS_PER_ROW)
        .enumerate()
        .map(|(i, chunk)| (format!("{}–{}", i * KEYS_PER_ROW + 1, i * KEYS_PER_ROW + chunk.len()), chunk.iter().map(|c| info::key_glyph(c)).collect()))
        .collect();
    sheet(&format!("Keyboard keys ({})", codes.len()), rows)
}

fn mouse() -> InfoView {
    sheet("Mouse buttons", vec![("Buttons".into(), MouseButton::ALL.map(info::mouse_glyph).to_vec())])
}

/// The drawn icons: form factors, controller kinds, battery levels and Wi-Fi bars.
fn icons() -> InfoView {
    let charge = |percent, estimated| Icon::Battery(Charge { percent, estimated });
    let rows: Vec<(String, Vec<Icon>)> = vec![
        ("This machine".into(), vec![Icon::Form(FormFactor::Desktop), Icon::Form(FormFactor::Laptop), Icon::Form(FormFactor::Handheld)]),
        ("Controller".into(), PadFamily::ALL.map(Icon::Controller).to_vec()),
        ("Battery".into(), vec![charge(100, false), charge(45, false), charge(10, false), charge(60, true)]),
        ("Wi-Fi".into(), [0, 25, 50, 75, 100].map(Icon::Wifi).to_vec()),
    ];
    sheet("Icons", rows.into_iter().map(|(name, icons)| (name, icons.into_iter().map(Segment::Icon).collect())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_drawn_key_is_on_the_keyboard_sheet_once() {
        let sheet = keys();
        let caps: Vec<&Segment> = sheet.rows.iter().flat_map(|row| &row[1]).collect();
        let unique: HashSet<&str> = keyboard::drawn_keys().map(|k| k.code).collect();
        assert_eq!(caps.len(), unique.len());
    }

    #[test]
    fn each_family_and_layout_has_a_sheet() {
        let titles: Vec<String> = sheets(true).iter().chain(&sheets(false)).map(|s| s.name.clone()).collect();
        assert!(titles.contains(&"Nintendo, Nintendo layout".to_string()));
        assert!(titles.contains(&"PlayStation".to_string()));
    }
}
