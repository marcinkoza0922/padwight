use super::*;
use super::action::{ActionAt, Slot};
use super::owner::{self, Owner};

mod game;
use crate::config::{Analog, ButtonAction, GyroActivation, GyroInput, Profile, StickAction, Trigger};

impl LayoutEditor {
    fn labels(&self, config: &Config) -> Vec<String> {
        self.rows(config).into_iter().map(|r| r.item.label).collect()
    }

    /// Puts the cursor on the first row whose label starts with `label`, and chooses it.
    fn pick(&mut self, label: &str, config: &mut Config) -> EditStep {
        let labels = self.labels(config);
        self.cursor = labels.iter().position(|l| l.starts_with(label)).unwrap_or_else(|| panic!("no row {label:?} on {:?}: {labels:?}", self.page));
        self.choose(config)
    }

    /// Puts the cursor on the row for button `b` (a button list, or a combo's page).
    fn pick_button(&mut self, b: Button, config: &mut Config) -> EditStep {
        let rows = self.rows(config);
        self.cursor = rows.iter().position(|r| r.item.buttons == vec![b]).unwrap_or_else(|| panic!("no row for {b:?}"));
        self.choose(config)
    }

    fn step_row(&mut self, label: &str, dir: i32, config: &mut Config) -> EditStep {
        let labels = self.labels(config);
        self.cursor = labels.iter().position(|l| l.starts_with(label)).unwrap_or_else(|| panic!("no row {label:?}: {labels:?}"));
        self.adjust(dir, config)
    }
}

fn config() -> Config {
    let mut config = Config::default();
    config.general.profiles = vec![Profile::passthrough("Gamepad")];
    config
}

fn button(config: &Config, b: Button) -> ButtonAction {
    config.active().unwrap().button(b).clone()
}

#[test]
fn choosing_a_gamepad_button_maps_it_and_returns_to_the_button_list() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Gamepad button", &mut config);
    assert_eq!(editor.pick_button(Button::East, &mut config), EditStep::Changed);
    assert_eq!(button(&config, Button::South), ButtonAction::Gamepad(Button::East));
    assert_eq!(editor.page, Page::Buttons(Owner::Profile));
}

#[test]
fn a_choice_sets_the_button_and_back_leaves_the_editor() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    assert_eq!(editor.pick("Screenshot", &mut config), EditStep::Changed);
    assert_eq!(button(&config, Button::South), ButtonAction::Screenshot);
    assert_eq!(editor.back(), EditStep::Stay);
    assert_eq!(editor.back(), EditStep::Leave);
}

#[test]
fn a_double_tap_gesture_is_set_from_the_gestures_page() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Gestures", &mut config);
    editor.pick("Double tap", &mut config);
    editor.pick("Screenshot", &mut config);
    let gestures = &config.active().unwrap().gestures[&Button::South];
    assert_eq!(gestures.double_tap, Some(ButtonAction::Screenshot));
    assert!(editor.labels(&config)[0].ends_with("Screenshot"), "back on the gestures page");
    editor.pick("Double tap", &mut config);
    editor.pick("Not set", &mut config);
    assert!(config.active().unwrap().gestures.is_empty());
}

#[test]
fn a_combo_is_added_given_buttons_and_an_action_and_then_deleted() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Combos", &mut config);
    assert_eq!(editor.pick("Add combo", &mut config), EditStep::Changed);
    editor.pick_button(Button::LeftBumper, &mut config);
    editor.pick_button(Button::RightBumper, &mut config);
    editor.pick("Action", &mut config);
    editor.pick("Screenshot", &mut config);
    let combo = &config.active().unwrap().combos[0];
    assert_eq!((combo.buttons.clone(), combo.action.clone()), (vec![Button::LeftBumper, Button::RightBumper], ButtonAction::Screenshot));
    // Deleting takes a second press.
    assert_eq!(editor.pick("Delete combo", &mut config), EditStep::Stay);
    assert!(editor.view(&config, &[]).items[editor.cursor].label.contains("again"));
    assert_eq!(editor.choose(&mut config), EditStep::Changed);
    assert!(config.active().unwrap().combos.is_empty());
    assert_eq!(editor.page, Page::Combos(Owner::Profile));
}

#[test]
fn keys_are_ticked_in_and_out_from_the_keyboards_rows() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Keys", &mut config);
    editor.pick("Tab Q W", &mut config);
    editor.pick("[ ] Q", &mut config);
    editor.pick("[ ] W", &mut config);
    assert_eq!(button(&config, Button::South), ButtonAction::Keys(vec!["KEY_Q".into(), "KEY_W".into()]));
    editor.pick("[x] Q", &mut config);
    assert_eq!(button(&config, Button::South), ButtonAction::Keys(vec!["KEY_W".into()]));
    editor.back();
    assert_eq!(editor.pick("Done", &mut config), EditStep::Stay);
    assert_eq!(editor.page, Page::Buttons(Owner::Profile));
}

#[test]
fn breadcrumbs_follow_the_pages_opened() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    let view = editor.view(&config, &["Menu".into()]);
    assert_eq!(view.crumbs, ["Menu", "Edit Controls", "Buttons"]);
    assert_eq!(view.title, "A button");
}

#[test]
fn a_toggle_wraps_the_action_and_keeps_it_on_a_page_of_its_own() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Make it a toggle", &mut config);
    assert!(matches!(button(&config, Button::South), ButtonAction::Toggle(_)));
    editor.pick("Starts on", &mut config);
    let ButtonAction::Toggle(t) = button(&config, Button::South) else { panic!("not a toggle") };
    assert!(t.start_on);
    // What it toggles is changed on its own page, and stays toggled.
    editor.pick("Toggles", &mut config);
    editor.pick("Screenshot", &mut config);
    let ButtonAction::Toggle(t) = button(&config, Button::South) else { panic!("not a toggle") };
    assert_eq!(*t.action, ButtonAction::Screenshot);
    editor.pick("Stop toggling", &mut config);
    assert_eq!(button(&config, Button::South), ButtonAction::Screenshot);
}

#[test]
fn a_menu_a_layer_or_a_mode_cant_be_made_a_toggle() {
    let mut config = config();
    for action in [ButtonAction::OpenMenu("M".into()), ButtonAction::Layer("L".into()), ButtonAction::Mode("D".into())] {
        config.general.profiles[0].set_button(Button::South, action);
        let rows = action_page::rows(&config, ActionAt::new(Slot::Button(Owner::Profile, Button::South)));
        assert!(!rows.iter().any(|r| r.item.label == "Make it a toggle"));
    }
}

#[test]
fn a_turbos_rate_steps() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Make it turbo", &mut config);
    assert_eq!(editor.step_row("Rate", 1, &mut config), EditStep::Changed);
    assert!(matches!(button(&config, Button::South), ButtonAction::Turbo { rate, .. } if rate == 11.0));
}

#[test]
fn several_actions_at_once_are_built_part_by_part() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Several at once", &mut config);
    assert_eq!(editor.page, Page::Multi(ActionAt::new(Slot::Button(Owner::Profile, Button::South))));
    editor.pick("Add an action", &mut config);
    editor.pick("Screenshot", &mut config);
    assert_eq!(button(&config, Button::South), ButtonAction::Multi(vec![ButtonAction::Gamepad(Button::South), ButtonAction::Screenshot]));
    editor.pick("Keep only the first", &mut config);
    assert_eq!(button(&config, Button::South), ButtonAction::Gamepad(Button::South));
}

#[test]
fn gyro_activation_steps_through_its_kinds_and_takes_an_input() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Gyro", &mut config);
    editor.step_row("Mode", 1, &mut config);
    editor.step_row("Active", 1, &mut config);
    assert_eq!(config.active().unwrap().gyro.activation, GyroActivation::WhileHeld(GyroInput::LeftTrigger));
    editor.pick("With", &mut config);
    editor.pick_button(Button::RightBumper, &mut config);
    assert_eq!(config.active().unwrap().gyro.activation, GyroActivation::WhileHeld(GyroInput::Button(Button::RightBumper)));
}

#[test]
fn a_layer_is_added_bound_and_held_from_a_button() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Layers", &mut config);
    editor.pick("Add layer", &mut config);
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::South, &mut config);
    editor.pick("Screenshot", &mut config);
    assert_eq!(config.general.layers[0].buttons.get(&Button::South), Some(&ButtonAction::Screenshot));
    // A layer leaves its other buttons to the profile.
    assert!(editor.labels(&config).iter().filter(|l| *l == "not set").count() > 1);
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::LeftBumper, &mut config);
    editor.pick("Hold a layer", &mut config);
    editor.pick("Layer 1", &mut config);
    assert_eq!(button(&config, Button::LeftBumper), ButtonAction::Layer("Layer 1".into()));
}

#[test]
fn a_layer_overrides_a_stick_and_gives_it_back() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Layers", &mut config);
    editor.pick("Add layer", &mut config);
    editor.pick("{ls} Left stick: the profile's", &mut config);
    editor.pick("Set the left stick here", &mut config);
    editor.step_row("Kind", 1, &mut config);
    assert!(config.general.layers[0].left_stick.is_some());
    assert_ne!(config.general.layers[0].left_stick.as_ref().unwrap().action, config.active().unwrap().left_stick.action);
    editor.pick("Use the profile's", &mut config);
    assert!(config.general.layers[0].left_stick.is_none());
}

#[test]
fn a_mode_is_added_and_switched_to_from_a_button() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Modes", &mut config);
    editor.pick("Add mode", &mut config);
    let mut editor = LayoutEditor::new();
    editor.pick("Buttons", &mut config);
    editor.pick_button(Button::Select, &mut config);
    editor.pick("Switch to a mode", &mut config);
    editor.pick("Mode 1", &mut config);
    assert_eq!(button(&config, Button::Select), ButtonAction::Mode("Mode 1".into()));
}

#[test]
fn a_macro_is_renamed_everywhere_and_kept_while_in_use() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Macros", &mut config);
    editor.pick("Add macro", &mut config);
    assert!(editor.labels(&config).iter().any(|l| l == "Delete macro"));
    config.general.profiles[0].set_button(Button::North, ButtonAction::Macro { name: "Macro 1".into(), repeat: false });
    assert!(!editor.labels(&config).iter().any(|l| l == "Delete macro"), "in use");
    assert_eq!(editor.pick("Name", &mut config), EditStep::Text { title: "Name of the macro".into(), text: "Macro 1".into() });
    assert_eq!(editor.set_text(&mut config, "Reload"), EditStep::Changed);
    assert_eq!(button(&config, Button::North), ButtonAction::Macro { name: "Reload".into(), repeat: false });
}

#[test]
fn a_trigger_zone_is_added_and_given_an_action() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("{lt} {rt} Triggers", &mut config);
    editor.pick("{lt} Left trigger", &mut config);
    editor.pick("Zones", &mut config);
    editor.pick("Add zone", &mut config);
    editor.pick("Action", &mut config);
    editor.pick("Screenshot", &mut config);
    assert_eq!(owner::zones(&config, Owner::Profile, Analog::Trigger(Trigger::Left))[0].action, ButtonAction::Screenshot);
}

#[test]
fn a_stick_becomes_a_mouse_with_its_response_settings() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("{ls} {rs} Sticks", &mut config);
    editor.pick("{rs} Right stick", &mut config);
    editor.step_row("Kind", 1, &mut config);
    assert!(matches!(config.active().unwrap().right_stick.action, StickAction::Mouse { .. }));
    assert!(!editor.two_columns(&config), "a page of values stays in one column");
    assert!(editor.view(&config, &[]).one_column);
    editor.step_row("Speed", 1, &mut config);
    assert!(matches!(config.active().unwrap().right_stick.action, StickAction::Mouse { speed, .. } if speed == 1250.0));
}

#[test]
fn guide_isnt_listed_and_paddles_are_when_the_controller_has_them() {
    let config = config();
    let buttons = |editor: &LayoutEditor| inputs::button_rows(&editor.cx(&config), Owner::Profile).into_iter().flat_map(|r| r.item.buttons).collect::<Vec<_>>();
    let without = buttons(&LayoutEditor::new());
    assert!(!without.contains(&Button::Guide));
    assert!(!without.contains(&Button::LeftPaddle));
    assert!(buttons(&LayoutEditor::new().with_paddles(true)).contains(&Button::LeftPaddle));
}

#[test]
fn pages_about_a_stick_trigger_gyro_or_menu_have_their_picture() {
    use crate::figures::{Figure, Live};
    let mut config = config();
    let live = Live { sticks: [(0.5, 0.0), (0.0, 0.0)], triggers: [0.25, 0.0], ..Live::default() };
    let mut editor = LayoutEditor::new();
    assert!(editor.figure(&config, &live).is_none(), "the top page has none");
    editor.pick("{ls} {rs} Sticks", &mut config);
    editor.pick("{ls} Left stick", &mut config);
    assert!(matches!(editor.figure(&config, &live), Some(Figure::Stick { live: Some((x, _)), .. }) if x == 0.5));
    assert_eq!(editor.live_stick(), Some(crate::config::Stick::Left));
    let mut editor = LayoutEditor::new();
    editor.pick("{lt} {rt} Triggers", &mut config);
    editor.pick("{lt} Left trigger", &mut config);
    assert!(matches!(editor.figure(&config, &live), Some(Figure::Trigger { live: Some(v), .. }) if v == 0.25));
    assert_eq!(editor.live_stick(), None);
    let mut editor = LayoutEditor::new();
    editor.pick("Info overlays", &mut config);
    editor.pick("Add info overlay", &mut config);
    assert_eq!(editor.preview(), Some(Preview::Info(0)), "an info overlay shows where it goes");
}
