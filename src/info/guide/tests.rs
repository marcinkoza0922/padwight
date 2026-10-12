use super::*;
use crate::config::{Gestures, RowEdit, RowId};

fn keys(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

fn view<'a>(profile: &'a Profile, modes: &'a [Mode], layers: &'a [Layer], shown: usize) -> GuideView<'a> {
    GuideView { profile, modes, layers, shown, current: 0, page: 0 }
}

fn row(glyph: &str, text: &str) -> Vec<String> {
    vec![glyph.to_string(), text.to_string()]
}

#[test]
fn the_notes_show_only_when_there_are_some() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::South, keys("KEY_E"));
    let overlays = view(&p, &[], &[], 0).overlays();
    assert_eq!(overlays.len(), 1, "no notes, so only the mappings");
    assert_eq!(overlays[0].name, "guide mappings");
    assert_eq!(overlays[0].title.as_deref(), Some("Mappings"), "no modes, so no mode in the title");
    assert!(overlays[0].rows.contains(&row("{south}", "{keyboard:e}")));
    p.guide.notes = "Hold LB for the radial menu.\nEsc is Start.".into();
    let overlays = view(&p, &[], &[], 0).overlays();
    assert_eq!(overlays[0].name, "guide notes");
    assert_eq!(overlays[0].rows.len(), 2, "one row per line");
}

#[test]
fn a_long_mappings_list_shows_a_page_at_a_time() {
    let mut p = Profile::passthrough("p");
    let names = ["KEY_A", "KEY_B", "KEY_C", "KEY_D", "KEY_E", "KEY_F", "KEY_G", "KEY_H", "KEY_I", "KEY_J", "KEY_K", "KEY_L", "KEY_M", "KEY_N", "KEY_O"];
    for (b, key) in Button::ALL.into_iter().zip(names) {
        p.set_button(b, keys(key));
    }
    for (i, key) in ["KEY_P", "KEY_Q", "KEY_R"].into_iter().enumerate() {
        p.gestures.insert(Button::ALL[i], Gestures { long_press: Some(keys(key)), ..Default::default() });
    }
    let pages = view(&p, &[], &[], 0).pages();
    assert!(pages > 1);
    let first = &view(&p, &[], &[], 0).overlays()[0];
    assert_eq!(first.rows.len(), MAPPING_ROWS);
    assert!(first.title.as_deref().is_some_and(|t| t.starts_with("Mappings 1/")));
    let last = &GuideView { page: 99, ..view(&p, &[], &[], 0) }.overlays()[0];
    assert!(last.title.as_deref().is_some_and(|t| t.starts_with(&format!("Mappings {pages}/"))), "past the end shows the last page");
}

#[test]
fn the_mappings_show_one_mode_at_a_time() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::South, keys("KEY_SPACE"));
    p.set_button(Button::East, keys("KEY_C"));
    p.guide.edits.insert(RowId::Input(GuideInput::Button(Button::South)), RowEdit { text: Some("Jump".into()), hidden: false });
    p.guide.edits.insert(RowId::Input(GuideInput::Button(Button::East)), RowEdit { text: Some("Crouch".into()), hidden: false });
    let mut driving = Mode::new("Driving");
    driving.buttons.insert(Button::South, keys("KEY_W"));
    let modes = [driving];

    let default = &view(&p, &modes, &[], 0).overlays()[0];
    assert!(default.rows.contains(&row("{south}", "Jump")));
    assert_eq!(default.title.as_deref(), Some("Mappings · Default mode · {left} {right} modes"));

    let shown = &view(&p, &modes, &[], 1).overlays()[0];
    assert!(shown.rows.contains(&row("{south}", "{keyboard:w}")), "the mode's mapping, not the text written for Jump: {:?}", shown.rows);
    assert!(shown.rows.contains(&row("{east}", "Crouch")), "what the mode leaves alone keeps its text");
    assert_eq!(shown.title.as_deref(), Some("Mappings · Driving mode (not on) · {left} {right} modes"));
    let on = &GuideView { current: 1, ..view(&p, &modes, &[], 1) }.overlays()[0];
    assert!(on.title.as_deref().is_some_and(|t| t.contains("Driving mode ·")), "{:?}", on.title);
}

#[test]
fn a_held_layer_gets_a_section_of_what_it_changes() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::LeftBumper, ButtonAction::Layer("Hotkeys".into()));
    p.gestures.insert(Button::North, Gestures { long_press: Some(ButtonAction::Layer("Hotkeys".into())), ..Default::default() });
    // Held only in a mode: no section outside it.
    p.set_button(Button::RightBumper, keys("KEY_R"));
    let mut hotkeys = Layer::new("Hotkeys");
    hotkeys.buttons.insert(Button::South, keys("KEY_F1"));
    let mut lean = Layer::new("Lean");
    lean.buttons.insert(Button::West, keys("KEY_Q"));
    let mut sneaking = Mode::new("Sneaking");
    sneaking.buttons.insert(Button::RightBumper, ButtonAction::Layer("Lean".into()));
    let (modes, layers) = ([sneaking], [hotkeys, lean]);

    // The pad's own buttons fill the first page; the section follows them.
    let overlays = GuideView { page: 99, ..view(&p, &modes, &layers, 0) }.overlays();
    assert_eq!(overlays.len(), 2, "the mappings and one section");
    assert_eq!(overlays[1].title.as_deref(), Some("Hotkeys layer · {lb}, {north} hold"));
    assert_eq!(overlays[1].rows, [row("{south}", "{keyboard:f1}")], "only what the layer sets");
    assert_eq!(overlays[1].style.position, ScreenPosition::CenterRight, "under the mappings");

    let overlays = GuideView { page: 99, ..view(&p, &modes, &layers, 1) }.overlays();
    let titles: Vec<_> = overlays.iter().filter_map(|o| o.title.clone()).collect();
    assert!(titles.contains(&"Lean layer · {rb}".to_string()), "{titles:?}");
}

#[test]
fn sections_that_dont_fit_go_to_the_next_page() {
    let section = |rows: usize| Section { title: String::new(), rows: vec![Vec::new(); rows] };
    let pages = layout(10, &[section(1), section(3)]);
    assert_eq!(pages.len(), 2, "10 rows + 3 fit, the second section doesn't");
    assert_eq!((pages[0].rows.clone(), pages[0].sections.clone()), (0..10, vec![0]));
    assert_eq!((pages[1].rows.clone(), pages[1].sections.clone()), (10..10, vec![1]));
    // A section taller than a page gets a page of its own, not an empty one before it.
    let pages = layout(0, &[section(20)]);
    assert_eq!(pages.len(), 1);
}
