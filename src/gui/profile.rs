//! The profile editor (buttons, combos, gyro), also used for layers, and the controller drawing.

use iced::widget::{column, row};

use super::*;
use crate::info::{Glyphs, button_name};
use super::guide_tab::{self, GuideView};
use crate::pad_widget::{Callouts, controller_drawing};

mod button_list;
mod callouts;
use callouts::{drawing_labels, drawing_tips};
pub(super) use button_list::{button_rows, paddle_rows};
use button_list::row_matches;

/// The label column of the Edit profile card, so its Profile and Name fields line up.
const PROFILE_LABEL: f32 = 60.0;

/// Sections of the profile editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProfileTab {
    Buttons,
    Sticks,
    Combos,
    Gyro,
    Touchpad,
    Guide,
}

/// What the free view functions need to know about the app beyond the profile itself.
pub(super) struct Ui<'a> {
    pub(super) names: &'a Names,
    pub(super) expanded: &'a HashSet<Target>,
    /// The push the mouse stick's response graphs show (see `App::stick_probe`).
    pub(super) stick_probe: f32,
    /// Row picked by "Find by pressing", highlighted and open.
    pub(super) found: Option<Button>,
    pub(super) analog_triggers: bool,
    pub(super) any_gyro: bool,
    pub(super) any_paddles: bool,
    pub(super) any_touchpad: bool,
    /// Live stick positions (left, right) for previews, when a controller is connected.
    pub(super) live_sticks: Option<[(f32, f32); 2]>,
    /// The gyro's live output and recent aim, for the Gyro tab's pictures.
    pub(super) live_gyro: Option<crate::ipc::GyroOut>,
    pub(super) gyro_trail: &'a super::gyro_graph::Trail,
    /// Why the Gyro tab's live pictures don't show the settings on screen, when they don't.
    pub(super) gyro_note: Option<&'static str>,
    /// Whose button glyphs to draw (see `App::glyph_family`).
    pub(super) family: PadFamily,
    /// Whose button names the controller in use has (see `App::shown_pad`).
    pub(super) in_use: PadFamily,
    /// How the Nintendo layout shows on this profile's face buttons (see `App::swap_for`).
    pub(super) swap: Swap,
    /// Set while editing a layer: the profile shown is the layer over `base`.
    pub(super) layer: Option<LayerMarks<'a>>,
    /// The menus the profile can use, for the touchpad's picture.
    pub(super) menus: Vec<&'a Menu>,
    /// Fingers on the touchpad of the controller in use, and whether it is clicked.
    pub(super) live_touch: Option<(Vec<(f32, f32)>, bool)>,
    /// The touchpad's height in widths, for its picture.
    pub(super) touchpad_height: f32,
    /// What the search box holds; the Buttons and Combos tabs list only rows that match.
    pub(super) query: &'a str,
}

/// A layer being edited, over the profile it's compared with.
#[derive(Clone, Copy)]
pub(super) struct LayerMarks<'a> {
    pub(super) layer: &'a crate::config::Layer,
    pub(super) base: &'a Profile,
}

/// Something a layer overrides as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LayerPart {
    /// A button's action and gestures.
    Button(Button),
    Stick(Stick),
    /// A trigger's action and zones.
    Trigger(Trigger),
    Gyro,
    Touchpad,
}

impl LayerMarks<'_> {
    pub(super) fn overrides(&self, part: LayerPart) -> bool {
        let l = self.layer;
        match part {
            LayerPart::Button(b) => l.buttons.contains_key(&b) || l.gestures.contains_key(&b),
            LayerPart::Stick(Stick::Left) => l.left_stick.is_some(),
            LayerPart::Stick(Stick::Right) => l.right_stick.is_some(),
            LayerPart::Trigger(Trigger::Left) => l.left_trigger.is_some(),
            LayerPart::Trigger(Trigger::Right) => l.right_trigger.is_some(),
            LayerPart::Gyro => l.gyro.is_some(),
            LayerPart::Touchpad => l.touchpad.is_some(),
        }
    }
}

/// In a layer, a part it doesn't override reads "Same as Gameplay: …" with an Override
/// button; an overridden one gets its editor and "Back to base". Outside layers, just the
/// editor.
#[expect(clippy::needless_pass_by_value, reason = "labels go into `text`, which needs them owned for 'a")]
pub(super) fn layer_part<'a>(ui: &Ui, part: LayerPart, label: String, summary: String, editor: impl FnOnce() -> Vec<Element<'a, Message>>) -> Vec<Element<'a, Message>> {
    let Some(marks) = ui.layer else { return editor() };
    if marks.overrides(part) {
        let mut rows = vec![
            row![
                text(label).size(13).color(style_accent()),
                space::horizontal(),
                button(text("Back to base").size(13)).style(button::text).on_press(Message::RevertInput(part)),
            ]
            .align_y(Alignment::Center)
            .into(),
        ];
        rows.extend(editor());
        return rows;
    }
    vec![
        row![
            text(label).width(LABEL_WIDTH).color(MUTED_COLOR),
            text(format!("Same as {}: {summary}", marks.base.name)).color(MUTED_COLOR),
            space::horizontal(),
            button(text("Override").size(13)).style(style::secondary).on_press(Message::OverrideInput(part)),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into(),
    ]
}

impl Ui<'_> {
    pub(super) fn is_open(&self, target: Target) -> bool {
        self.expanded.contains(&target) || matches!(target, Target::Button(b) if self.found == Some(b))
    }
}

#[expect(clippy::too_many_lines, reason = "predates the size lints")]
pub(super) fn view_profile<'a>(p: &'a Profile, ui: &Ui, tab: ProfileTab, guide: GuideView<'a>) -> Element<'a, Message> {
    let names = ui.names;
    let sections: Vec<Element<'a, Message>> = match tab {
        ProfileTab::Buttons => {
            let (buttons, paddles) = (button_rows(p, ui), paddle_rows(p, ui));
            let searching = !ui.query.trim().is_empty();
            let mut sections = Vec::new();
            if !searching || !buttons.is_empty() {
                sections.push(section(
                "Buttons",
                Some(
                    "Click a button's name to edit it, and add a double tap, triple tap or long press \
                     with the \"+\" buttons under its actions. A button with gestures waits until the gesture \
                     is clear: a quick tap fires after the tap window; held past it, the button's own action \
                     holds until release (or the long press takes over, if set). \
                     In the summaries, keys are gray, mouse buttons teal and controller buttons show their \
                     glyphs, as on the overlays; the colored icon before a word such as Menu or Toggle marks \
                     the kind of action."
                        .into(),
                ),
                buttons,
            ));
            }
            if !searching || !paddles.is_empty() {
                sections.push(section(
                "Back paddles",
                Some(
                    "The extra buttons on the back of some controllers (Xbox Elite, DualSense Edge, Steam Deck). \
                     Map them like any button."
                        .into(),
                ),
                paddles,
            ));
            }
            if sections.is_empty() {
                sections.push(note("No button matches.").into());
            }
            sections
        }
        ProfileTab::Sticks => {
            let mut sticks = Vec::new();
            for s in [Stick::Left, Stick::Right] {
                let base = ui.layer.map_or(p.stick(s), |m| m.base.stick(s));
                sticks.push(device_box(layer_part(ui, LayerPart::Stick(s), s.to_string(), stick_summary(base), || vec![stick_editor(s, p.stick(s), ui)])));
            }
            let mut triggers = Vec::new();
            for t in [Trigger::Left, Trigger::Right] {
                let base = ui.layer.map_or(p, |m| m.base);
                let base = if t == Trigger::Left { &base.left_trigger } else { &base.right_trigger };
                triggers.extend(layer_part(ui, LayerPart::Trigger(t), t.to_string(), trigger_summary(base), || {
                    vec![trigger_editor(t, p.trigger_config(t), ui.analog_triggers, names)]
                }));
            }
            vec![
                section(
                    "Sticks",
                    Some(
                        "A stick's directions (Left Stick Up, …) act as buttons on top of the stick's mode, \
                         pressing at its \"Directions press at\" threshold. They have no actions of their own, \
                         but work in combos (e.g. LB + Right Stick Right), in macros, and as other buttons' \
                         output. For an action per direction, use a button ring."
                            .into(),
                    ),
                    sticks,
                ),
                section("Triggers", None, triggers),
            ]
        }
        ProfileTab::Combos => {
            let mut sections = vec![section(
                if ui.layer.is_some() { "This layer's combos" } else { "Combos" },
                Some(format!(
                    "Buttons pressed within {} ms of each other act as one input. A button in a combo waits \
                     that long before acting alone, unless it's Disabled: then it's only a modifier, with no \
                     time limit.",
                    p.combo_window_ms
                )),
                combo_rows(p, ui),
            )];
            if let Some(marks) = ui.layer {
                sections.push(base_combos(marks));
            }
            sections
        }
        ProfileTab::Guide => guide_tab::sections(p, guide, ui.layer.is_some()),
        ProfileTab::Touchpad => touchpad::touchpad_sections(p, ui),
        ProfileTab::Gyro => gyro::gyro_sections(p, ui),
    };
    let searching = !ui.query.trim().is_empty();
    let sections = match tab_words(tab) {
        words if searching && !words.is_empty() => {
            let kept: Vec<_> = sections.into_iter().zip(words).filter(|(_, w)| matches_query(ui.query, w)).map(|(e, _)| e).collect();
            if kept.is_empty() { vec![note("Nothing here matches.").into()] } else { kept }
        }
        _ => sections,
    };
    column(sections).spacing(16).into()
}

/// What each section of a tab can be found by when searching: its title and the labels in it, one
/// string per section in the order `view_profile` lists them. Empty for the tabs that search their
/// rows instead.
fn tab_words(tab: ProfileTab) -> &'static [&'static str] {
    match tab {
        ProfileTab::Buttons | ProfileTab::Combos => &[],
        ProfileTab::Sticks => &[
            "sticks left stick right stick mode mouse gamepad flick ring sectors first sector sector 1 wasd arrows scroll dead zone deadzone shape anti-deadzone hold buffer dead angle curve response speed vertical smoothing turn size invert directions press at stickiness fill with test turn",
            "triggers left trigger right trigger zones add zone walk modifier shift partial push range from",
        ],
        ProfileTab::Gyro => &[
            "gyro motion aim mode off mouse stick steering flick sensitivity invert horizontal vertical axes turning response curve slow fast shape start switched off rumble pause pushing advanced jitter noise smoothing drift full tilt lock edge",
        ],
        ProfileTab::Touchpad => &[
            "touchpad touch pad mouse stick menu swipe split left right half tap click two-finger scroll deadzone sensitivity response glide flick center finger speed",
        ],
        ProfileTab::Guide => &["guide overlay notes tips mapping guide rows add a row show button"],
    }
}

/// Whether `tab` has something the search box finds.
fn tab_has_match(p: &Profile, tab: ProfileTab, ui: &Ui) -> bool {
    match tab {
        ProfileTab::Buttons => Button::ALL.into_iter().chain(Button::PADDLES).any(|b| row_matches(p, b, ui)),
        ProfileTab::Combos => p.combos.iter().any(|c| combo_matches(c, ui)),
        _ => tab_words(tab).iter().any(|w| matches_query(ui.query, w)),
    }
}

/// Whether a combo's buttons and action have every word the search box holds.
fn combo_matches(combo: &crate::config::Combo, ui: &Ui) -> bool {
    let name = combo.buttons.iter().map(|b| short_button(*b)).collect::<Vec<_>>().join(" + ");
    matches_query(ui.query, &format!("{name} {}", summarize(&combo.action)))
}

/// The compared profile's combos, each of which a layer can switch off while it's on.
pub(super) fn base_combos<'a>(marks: LayerMarks<'_>) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for combo in &marks.base.combos {
        let key = crate::config::combo_key(&combo.buttons);
        let off = marks.layer.disabled_combos.iter().any(|d| crate::config::combo_key(d) == key);
        let replaced = marks.layer.combos.iter().any(|c| crate::config::combo_key(&c.buttons) == key);
        let name = combo.buttons.iter().map(|b| short_button(*b)).collect::<Vec<_>>().join(" + ");
        let mut line = row![text(name).width(LABEL_WIDTH), text(summarize(&combo.action)).color(MUTED_COLOR), space::horizontal()]
            .spacing(10)
            .align_y(Alignment::Center);
        line = if replaced {
            line.push(text("replaced by this layer's").size(12).color(MUTED_COLOR))
        } else {
            line.push(checkbox(off).label("Off in this layer").on_toggle(move |_| Message::ToggleBaseCombo(key.clone())))
        };
        rows.push(line.into());
    }
    if rows.is_empty() {
        rows.push(text(format!("{} has no combos.", marks.base.name)).size(13).color(MUTED_COLOR).into());
    }
    section(
        "The profile's combos",
        Some("They stay on while the layer is held, unless switched off here or replaced by a layer combo on the same buttons.".into()),
        rows,
    )
}

impl ProfileTab {
    pub(super) const ALL: [ProfileTab; 6] = [ProfileTab::Buttons, ProfileTab::Sticks, ProfileTab::Combos, ProfileTab::Gyro, ProfileTab::Touchpad, ProfileTab::Guide];
}

impl fmt::Display for ProfileTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProfileTab::Buttons => "Buttons",
            ProfileTab::Sticks => "Sticks & triggers",
            ProfileTab::Combos => "Combos",
            ProfileTab::Gyro => "Gyro",
            ProfileTab::Touchpad => "Touchpad",
            ProfileTab::Guide => "Guide",
        })
    }
}

/// A button pressed (or stick pushed firmly) in `now` that wasn't in `before`, for
/// "Find by pressing".
pub(super) fn newly_pressed(before: Option<&InputSnapshot>, now: &InputSnapshot) -> Option<Button> {
    let was_down = |b: &Button| before.is_some_and(|s| s.buttons.contains(b));
    if let Some(b) = now.buttons.iter().find(|b| !was_down(b)) {
        return Some(*b);
    }
    const FIRM: f32 = 0.7;
    let pushed = |s: &InputSnapshot, stick: Stick| -> Option<Button> {
        let (x, y) = if stick == Stick::Left { s.left_stick } else { s.right_stick };
        let [up, down, left, right] = Button::stick_directions(stick);
        [(up, -y), (down, y), (left, -x), (right, x)].into_iter().find(|(_, v)| *v > FIRM).map(|(b, _)| b)
    };
    [Stick::Left, Stick::Right]
        .into_iter()
        .find_map(|stick| pushed(now, stick).filter(|b| before.and_then(|s| pushed(s, stick)) != Some(*b)))
}

/// Where the profile's author says what it can't be played without. Never worked out from
/// the settings: a profile that adds gyro aiming to a scheme that works without it leaves
/// the box unticked.
pub(super) fn requirement_rows(p: &Profile, f: Feature) -> Vec<Element<'_, Message>> {
    let has = p.requires.contains(&f);
    let toggled = move |on: bool| {
        let mut all: Vec<Feature> = p.requires.iter().copied().filter(|x| *x != f).collect();
        all.extend(on.then_some(f));
        Message::SetRequires(all)
    };
    let example = match f {
        Feature::Gyro => "a flick stick setup that turns vertically with gyro",
        Feature::Touchpad => "a build menu that's only on the pad",
        Feature::BackPaddles => "a jump that's only on a paddle",
    };
    let label = f.label();
    // The same explanation for every feature, behind an ⓘ rather than a paragraph under each box.
    let hint = format!(
        "This isn't detected: tick it if the profile can't be played without {label} ({example}), \
         and leave it unticked if it's only an extra. Players whose controller has no {label} aren't \
         offered the profile, and Guide skips it."
    );
    vec![
        row![checkbox(has).label(format!("Can't be played without {label}")).on_toggle(toggled), help(hint)]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
    ]
}

/// The label column of a collapsible row: click to open or close it.
pub(super) fn row_toggle<'a>(label: &str, target: Target, open: bool, problem: bool) -> Element<'a, Message> {
    let chevron = if open { "▾" } else { "▸" };
    let label = text(label.to_owned());
    let label = if problem { label.color(ERROR_COLOR) } else { label };
    button(row![text(chevron).size(14), label].spacing(4).align_y(Alignment::Center))
        .style(button::text)
        .padding([4, 0])
        .width(LABEL_WIDTH)
        .on_press(Message::ToggleExpanded(target))
        .into()
}

/// LB + RB for a new combo, or the first pair of buttons no combo has yet.
fn free_combo(combos: &[Combo]) -> Vec<Button> {
    let pairs = Button::ALL.into_iter().enumerate().flat_map(|(i, a)| Button::ALL[i + 1..].iter().map(move |&b| vec![a, b]));
    std::iter::once(vec![Button::LeftBumper, Button::RightBumper])
        .chain(pairs)
        .find(|pair| !combos.iter().any(|c| crate::config::combo_key(&c.buttons) == crate::config::combo_key(pair)))
        .unwrap_or_else(|| vec![Button::LeftBumper, Button::RightBumper])
}

#[expect(clippy::too_many_lines, reason = "predates the size lints")]
pub(super) fn combo_rows<'a>(p: &'a Profile, ui: &Ui) -> Vec<Element<'a, Message>> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    // The window only means something once there is a combo to press within it.
    let searching = !ui.query.trim().is_empty();
    if p.combos.is_empty() {
        rows.push(text("This profile has no combos yet.").color(MUTED_COLOR).into());
    } else if ui.layer.is_none() && !searching {
        rows.push(value_slider("Combo window", 20.0..=300.0, p.combo_window_ms as f32, 5.0, "ms", Message::SetComboWindow));
    }
    for (i, combo) in p.combos.iter().enumerate() {
        let target = Target::Combo(i);
        let open = ui.is_open(target);
        let name = combo.buttons.iter().map(|b| short_button(*b)).collect::<Vec<_>>().join(" + ");
        let name = if name.is_empty() { "(no buttons)".to_string() } else { name };
        if !combo_matches(combo, ui) {
            continue;
        }
        let problem = combo_problem(&p.combos, i).or_else(|| action_problem(&combo.action, ui.names));
        let remove = delete_item("Delete combo", Message::RemoveCombo(i));
        if !open {
            let mut line = row![row_toggle(&name, target, false, problem.is_some()), text(summarize(&combo.action)), space::horizontal()]
                .spacing(10)
                .align_y(Alignment::Center);
            if let Some(problem) = problem {
                line = line.push(text(format!("⚠ {problem}")).size(12).color(ERROR_COLOR));
            }
            // Deleted from inside the open row, as every item is.
            rows.push(line.into());
            continue;
        }
        let mut members = row![].spacing(6).align_y(Alignment::Center);
        for (n, &b) in combo.buttons.iter().enumerate() {
            if n > 0 {
                members = members.push(text("+"));
            }
            members = members.push(
                button(text(format!("{} ✕", short_button(b))).size(13))
                    .style(style::secondary)
                    .on_press(Message::RemoveComboButton(i, b)),
            );
        }
        let remaining: Vec<Button> =
            Button::EVERY.into_iter().filter(|b| !combo.buttons.contains(b)).collect();
        members = members.push(
            dropdown(remaining, None::<Button>, move |b| Message::AddComboButton(i, b))
                .placeholder("+ Add button…")
                .style(style::action_dropdown)
                .width(170),
        );
        let mut body = column![
            row![
                row_toggle(&name, target, true, problem.is_some()),
                members,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            labeled("    Action", fill_x(action_editor(&combo.action, Button::South, &ACTION_KINDS, set_action(target), KeyField::root(target), ui.names))),
        ]
        .spacing(8);
        if let Some(offer) = menu_combos::offer(p, &combo.buttons, &combo.action, ui) {
            body = body.push(labeled("", offer));
        }
        if let Some(problem) = problem {
            body = body.push(labeled("", text(format!("⚠ {problem}")).size(12).color(ERROR_COLOR).into()));
        }
        // Removing sits beside the box rather than inside it, apart from the combo's own controls.
        rows.push(
            row![
                container(body).padding(10).style(style::inset).width(Length::Fill),
                remove,
            ]
            .spacing(8)
            .into(),
        );
    }
    rows.push(button(text("+ Add combo").size(13)).style(style::secondary).on_press(Message::AddCombo).into());
    if searching && rows.is_empty() {
        rows.push(text("No combo matches.").color(MUTED_COLOR).into());
    }
    rows
}

/// Deletes the profile being edited. A setup's only profile can't go, and a tooltip says why.
fn delete_profile_button<'a>(can_delete: bool) -> Element<'a, Message> {
    let delete = button(text("Delete")).style(style::quiet_danger).on_press_maybe(can_delete.then_some(Message::DeleteProfile));
    if can_delete {
        return delete.into();
    }
    tooltip(
        delete,
        container(text("A setup needs at least one profile.").size(13)).padding(8).style(style::tooltip),
        tooltip::Position::Bottom,
    )
    .into()
}

pub(super) fn short_button(b: Button) -> &'static str {
    b.short_name()
}

/// The caption of a picture of `p` without callouts, which alone doesn't say whether anything is
/// mapped. Where the page doesn't name the profile (`named` is its setup), a bare "every input
/// passes through" would read as remapping doing nothing, so the caption names it.
fn passthrough_caption(p: &Profile, named: Option<&str>, caption: &str) -> String {
    match named {
        Some(setup) => format!("{setup} › {} passes every input through · {caption}", p.name),
        None => format!("Every input passes through unchanged · {caption}"),
    }
}

impl App {
    #[expect(clippy::too_many_lines, clippy::cognitive_complexity, reason = "predates the size lints")]
    pub(super) fn update_profile(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetRequires(requires) => {
                if let Some(p) = self.profile_mut() {
                    p.requires = requires;
                }
            }
            Message::SetGyro(gyro) => {
                if let Some(p) = self.profile_mut() {
                    p.gyro = gyro;
                }
            }
            Message::SetTouchpad(touchpad) => {
                if let Some(p) = self.profile_mut() {
                    p.touchpad = touchpad;
                }
            }
            Message::EditProfile(name) => {
                if let Some(i) = self.game().profiles.iter().position(|p| p.name == name) {
                    self.editing = i;
                    self.reload_guide_notes();
                }
            }
            Message::RenameProfile(name) => {
                let editing = self.editing;
                let taken = self.game().profiles.iter().enumerate().any(|(i, p)| i != editing && p.name == name);
                let Some(old) = self.profile().map(|p| p.name.clone()).filter(|_| !taken) else { return Task::none() };
                let key = self.game_key().map(str::to_string);
                let game = self.game_mut();
                game.profiles[editing].name = name.clone();
                // Keep the game's rules, and the auto-switch default, pointing at it.
                for rule in game.rules.iter_mut().filter(|r| r.profile == old) {
                    rule.profile = name.clone();
                }
                let rename = Rename::Profile { game: key.clone(), old: old.clone(), new: name.clone() };
                if let Some(d) = self.config.auto_switch.default_profile.as_mut().filter(|d| d.game == key && d.profile == old) {
                    d.profile = name;
                }
                self.config.active = follow_renames(std::slice::from_ref(&rename), self.config.active.clone());
                self.renames.push(rename);
            }
            Message::AddProfile(template) => {
                let name = unique_name(template.base_name(), |n| self.game().profile(n).is_some());
                let profile = match template {
                    Template::Duplicate => {
                        let mut p = self.profile().cloned().unwrap_or_else(|| Profile::passthrough(""));
                        p.name = name;
                        p
                    }
                    other => other.make(&name),
                };
                self.game_mut().profiles.push(profile);
                self.editing = self.game().profiles.len() - 1;
            }
            Message::DeleteProfile => {
                let editing = self.editing;
                let game = self.game_mut();
                if game.profiles.len() > 1 && editing < game.profiles.len() {
                    game.profiles.remove(editing);
                    self.editing = editing.min(self.game().profiles.len() - 1);
                }
            }
            Message::SetAction(target, action) => {
                if let Some(p) = self.profile_mut() {
                    match target {
                        Target::Button(b) => p.set_button(b, action),
                        Target::Trigger(t) => {
                            if let TriggerAction::Button { action: a, .. } = p.trigger_mut(t) {
                                *a = action;
                            }
                        }
                        Target::Combo(i) => {
                            if let Some(c) = p.combos.get_mut(i) {
                                c.action = action;
                            }
                        }
                        Target::Gesture(b, kind) => {
                            *p.gestures.entry(b).or_default().slot(kind) = Some(action);
                        }
                        Target::TriggerGesture(t, kind) => {
                            *p.trigger_config_mut(t).gestures.slot(kind) = Some(action);
                        }
                        Target::Zone(a, i) => {
                            if let Some(z) = p.zones_mut(a).get_mut(i) {
                                z.action = action;
                            }
                        }
                        Target::RingSector(st, i) => {
                            if let Some(a) = p.stick_mut(st).action.ring_actions_mut().get_mut(i) {
                                *a = action;
                            }
                        }
                        Target::Touch(slot) => {
                            if let Some(a) = p.touchpad.slot_mut(slot) {
                                *a = action;
                            }
                        }
                        Target::MacroStep(..) | Target::CycleStep(..) | Target::MenuItem(..) | Target::StickAdvanced(_) | Target::GyroAdvanced => {}
                    }
                }
            }
            Message::AddGesture(b, kind) => {
                if let Some(p) = self.profile_mut() {
                    let slot = p.gestures.entry(b).or_default().slot(kind);
                    if slot.is_none() {
                        *slot = Some(ButtonAction::Keys(Vec::new()));
                    }
                }
            }
            Message::RemoveGesture(b, kind) => {
                if let Some(p) = self.profile_mut()
                    && let Some(g) = p.gestures.get_mut(&b)
                {
                    *g.slot(kind) = None;
                    if g.is_empty() {
                        p.gestures.remove(&b);
                    }
                }
            }
            Message::AddTriggerGesture(t, kind) => {
                if let Some(p) = self.profile_mut() {
                    let slot = p.trigger_config_mut(t).gestures.slot(kind);
                    if slot.is_none() {
                        *slot = Some(ButtonAction::Keys(Vec::new()));
                    }
                }
            }
            Message::RemoveTriggerGesture(t, kind) => {
                if let Some(p) = self.profile_mut() {
                    *p.trigger_config_mut(t).gestures.slot(kind) = None;
                }
            }
            Message::AddZone(a, preset) => {
                if let Some(p) = self.profile_mut() {
                    p.zones_mut(a).push(match preset {
                        ZonePreset::Empty => Zone { min: 0.0, max: 0.5, action: ButtonAction::Keys(Vec::new()) },
                        ZonePreset::Walk => Zone {
                            min: 0.0,
                            max: 0.75,
                            action: ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]),
                        },
                    });
                }
            }
            Message::RemoveZone(a, i) => {
                if let Some(p) = self.profile_mut()
                    && i < p.zones(a).len()
                {
                    p.zones_mut(a).remove(i);
                }
            }
            Message::SetZoneRange(a, i, min, max) => {
                if let Some(z) = self.profile_mut().and_then(|p| p.zones_mut(a).get_mut(i)) {
                    z.min = min;
                    z.max = max;
                }
            }
            Message::SelectProfileTab(tab) => {
                self.profile_tab = tab;
                self.found = None;
                if tab == ProfileTab::Guide {
                    self.reload_guide_notes();
                }
            }
            Message::Guide(msg) => self.update_guide(msg),
            Message::TogglePicture => self.picture_hidden = !self.picture_hidden,
            Message::ToggleExpanded(target) => {
                if !self.expanded.remove(&target) {
                    self.expanded.insert(target);
                }
                if let Target::Button(b) = target
                    && self.found == Some(b)
                {
                    self.found = None;
                    self.expanded.remove(&target);
                }
            }
            Message::ExpandAll(open) => {
                let targets: Vec<Target> = match self.profile_tab {
                    ProfileTab::Buttons => Button::ALL.into_iter().chain(Button::PADDLES).map(Target::Button).collect(),
                    ProfileTab::Sticks => Vec::new(),
                    ProfileTab::Combos => {
                        (0..self.profile().map_or(0, |p| p.combos.len())).map(Target::Combo).collect()
                    }
                    ProfileTab::Gyro | ProfileTab::Touchpad | ProfileTab::Guide => Vec::new(),
                };
                for t in targets {
                    if open {
                        self.expanded.insert(t);
                    } else {
                        self.expanded.remove(&t);
                    }
                }
                self.found = None;
            }
            Message::StartFind => {
                self.finding = true;
                self.found = None;
            }
            Message::CancelFind => self.finding = false,
            Message::SetTapWindow(ms) => {
                if let Some(p) = self.profile_mut() {
                    p.tap_window_ms = ms.round() as u64;
                }
            }
            Message::SetLongPress(ms) => {
                if let Some(p) = self.profile_mut() {
                    p.long_press_ms = ms.round() as u64;
                }
            }
            Message::AddCombo => {
                if let Some(p) = self.profile_mut() {
                    p.combos.push(Combo { buttons: free_combo(&p.combos), action: ButtonAction::Keys(Vec::new()) });
                    let new = Target::Combo(p.combos.len() - 1);
                    self.expanded.insert(new);
                }
            }
            Message::AddMenuCombos(combos) => {
                if let Some(p) = self.profile_mut() {
                    for c in combos {
                        if !p.combos.iter().any(|e| crate::config::combo_key(&e.buttons) == crate::config::combo_key(&c.buttons)) {
                            p.combos.push(c);
                        }
                    }
                }
            }
            Message::RemoveCombo(i) => {
                if let Some(p) = self.profile_mut()
                    && i < p.combos.len()
                {
                    p.combos.remove(i);
                    self.move_open_combos(open_cards::after_removing(i));
                }
            }
            Message::AddComboButton(i, b) => {
                if let Some(c) = self.profile_mut().and_then(|p| p.combos.get_mut(i))
                    && !c.buttons.contains(&b)
                {
                    c.buttons.push(b);
                }
            }
            Message::RemoveComboButton(i, b) => {
                if let Some(c) = self.profile_mut().and_then(|p| p.combos.get_mut(i)) {
                    c.buttons.retain(|x| *x != b);
                }
            }
            Message::SetComboWindow(ms) => {
                if let Some(p) = self.profile_mut() {
                    p.combo_window_ms = ms.round() as u64;
                }
            }
            Message::SetStick(s, cfg) => {
                if let Some(p) = self.profile_mut() {
                    *p.stick_mut(s) = cfg;
                }
            }
            Message::SetTrigger(t, action) => {
                if let Some(p) = self.profile_mut() {
                    *p.trigger_mut(t) = action;
                }
            }
            other => return self.update_actions(other),
        }
        Task::none()
    }

    /// Opens and highlights `b`'s row and scrolls near it.
    pub(super) fn jump_to(&mut self, b: Button) -> Task<Message> {
        self.finding = false;
        self.found = Some(b);
        if !matches!(self.game_tab, GameTab::Layers | GameTab::Modes) {
            self.game_tab = GameTab::Profiles;
        }
        let (tab, position) = match Button::ALL.iter().position(|x| *x == b) {
            Some(i) => (ProfileTab::Buttons, i as f32 / Button::ALL.len() as f32),
            None if Button::PADDLES.contains(&b) => {
                let i = Button::PADDLES.iter().position(|x| *x == b).unwrap_or_default();
                (ProfileTab::Buttons, (Button::ALL.len() + i) as f32 / (Button::ALL.len() + Button::PADDLES.len()) as f32)
            }
            None => {
                let right = b.stick_direction().is_some_and(|(s, _)| s == Stick::Right);
                (ProfileTab::Sticks, if right { 0.75 } else { 0.25 })
            }
        };
        self.profile_tab = tab;
        iced::widget::operation::snap_to("main", scrollable::RelativeOffset { x: Some(0.0), y: Some(position) })
    }

    /// The Nintendo layout's switch for the controller picked in the header (greyed out on all of them): which
    /// label each face button shows. The bindings don't move.
    fn view_nintendo_layout(&self) -> Element<'_, Message> {
        let pad = self.picked_controller().and_then(|d| self.status.as_ref()?.controllers.iter().find(|c| c.id == d.id));
        let note = "Swaps A with B and X with Y, to match games that draw Nintendo-layout prompts. On a Nintendo \
                    pad each face button then sends the letter on its label; remapping a face button's plain press \
                    turns that off, and they send their positions again. On other pads only the labels change. A \
                    setup's own choice overrides this.";
        let toggle = toggler(pad.is_some_and(|c| c.nintendo_layout)).label("Nintendo button layout");
        let toggle = if pad.is_some() { toggle.on_toggle(Message::SetNintendoLayout) } else { toggle };
        // It's the controller's setting, not the profile's, which the card's place on every profile doesn't say.
        let hint = pad.map_or_else(
            || "Set per controller, the same in every profile. Pick a controller in the header to change it.".to_string(),
            |c| format!("For {}, in every profile.", c.nickname_or_name()),
        );
        section("Button layout", Some(note.into()), vec![toggle.into(), text(hint).size(13).color(MUTED_COLOR).into()])
    }

    pub(super) fn view_profile_tab<'a>(&'a self, names: &Names) -> Element<'a, Message> {
        let col = column![self.view_profile_bar(), self.view_nintendo_layout()].spacing(16);
        let Some(p) = self.profile() else { return col.into() };
        col.push(self.view_profile_editor(p, names, None)).into()
    }

    /// The live drawing, "Find by pressing", and the Buttons, Sticks & triggers, Combos and
    /// Gyro tabs for `p`: a profile, or (with `layer`) a layer over one.
    pub(super) fn view_profile_editor<'a>(&'a self, p: &'a Profile, names: &Names, layer: Option<LayerMarks<'a>>) -> Element<'a, Message> {
        // The active profile's drawing also shows the layers that are on right now.
        let live = (layer.is_none() && self.profile_ref().is_some_and(|at| at == self.saved.active)).then(|| self.with_active_layers(p));
        let mut col = column![overview_card(self.view_live(Some(live.as_ref().unwrap_or(p)), None, true))].spacing(16);

        // The tabs take what the buttons beside them leave, and wrap there when it's too little.
        let mut tabs = row![container(self.profile_segments(p, names)).width(Length::Fill)]
            .spacing(6)
            .align_y(Alignment::Center);
        // Only where there are rows that open: every button, and the combos once there is one.
        let expandable = match self.profile_tab {
            ProfileTab::Buttons => true,
            ProfileTab::Combos => self.profile().is_some_and(|p| !p.combos.is_empty()),
            ProfileTab::Sticks | ProfileTab::Gyro | ProfileTab::Touchpad | ProfileTab::Guide => false,
        };
        let tutorial = match self.profile_tab {
            ProfileTab::Buttons | ProfileTab::Combos => "tutorial-buttons.md",
            ProfileTab::Sticks => "tutorial-sticks.md",
            ProfileTab::Gyro => "tutorial-gyro.md",
            ProfileTab::Touchpad => "tutorial-touchpad.md",
            ProfileTab::Guide => "tutorial-guide.md",
        };
        if expandable {
            tabs = tabs
                .push(button(text("Expand all").wrapping(text::Wrapping::None)).style(style::secondary).on_press(Message::ExpandAll(true)))
                .push(button(text("Collapse all").wrapping(text::Wrapping::None)).style(style::secondary).on_press(Message::ExpandAll(false)));
        }
        // Last, so it stays at the right end whether or not the buttons before it show.
        tabs = tabs.push(tutorial_link(tutorial));
        col = col.push(self.find_and_search()).push(tabs);

        let ui = Ui {
            names,
            expanded: &self.expanded,
            stick_probe: self.stick_probe,
            found: self.found,
            analog_triggers: self.analog_triggers(),
            any_gyro: self.any_gyro(),
            any_paddles: self.any_paddles(),
            any_touchpad: self.any_touchpad(),
            live_sticks: self.shown_input().map(|l| [l.left_stick, l.right_stick]),
            live_gyro: self.live.as_ref().and_then(|l| l.gyro_out),
            gyro_trail: &self.gyro_trail,
            gyro_note: self.gyro_note(p, layer.is_some()),
            family: self.glyph_family(),
            in_use: self.shown_pad().family,
            swap: self.swap_for(p),
            layer,
            menus: self.config.scope_of(self.game()).menus,
            live_touch: self.shown_input().map(|l| (l.touches.clone(), l.touch_click)),
            query: &self.button_query,
            touchpad_height: if self.shown_pad().model == Some(PadModel::DualShock4) { crate::input::DUALSHOCK4_HEIGHT } else { crate::input::DUALSENSE_HEIGHT },
        };
        let guide = GuideView { notes: &self.guide_notes, family: ui.family, swap: ui.swap, font: self.preview_font(), nintendo_layout: self.nintendo_layout() };
        if let Some(also) = self.also_in_tabs(p, &ui) {
            col = col.push(also);
        }
        col.push(view_profile(p, &ui, self.profile_tab, guide)).into()
    }

    /// "Find by pressing" and the search box, on one line.
    fn find_and_search<'a>(&self) -> Element<'a, Message> {
        let find: Element<'_, Message> = if self.finding {
            row![
                text("Press a button or push a stick on your controller…").color(style_accent()),
                button(text("Cancel")).style(style::secondary).on_press(Message::CancelFind),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                button(text("Find by pressing")).style(style::secondary).on_press_maybe(self.status.is_some().then_some(Message::StartFind)),
                help("Press a button or push a stick on your controller to jump to its mapping.".into()),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        };
        row![
            find,
            container(search_field("Search this profile…", &self.button_query, Message::SetButtonSearch)).max_width(420)
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    }

    /// While searching, the other tabs that have a match, as buttons to go there.
    fn also_in_tabs<'a>(&self, p: &Profile, ui: &Ui) -> Option<Element<'a, Message>> {
        if ui.query.trim().is_empty() {
            return None;
        }
        let others: Vec<ProfileTab> = ProfileTab::ALL.into_iter().filter(|t| *t != self.profile_tab && tab_has_match(p, *t, ui)).collect();
        let mut line = row![text(if others.is_empty() { "No other tab matches." } else { "Also found in" }).size(13).color(MUTED_COLOR)]
            .spacing(6)
            .align_y(Alignment::Center);
        for t in others {
            line = line.push(button(text(t.to_string()).size(13)).style(style::secondary).padding([3, 10]).on_press(Message::SelectProfileTab(t)));
        }
        Some(line.into())
    }

    /// Why the live gyro pictures don't show `p`'s gyro settings: the daemon runs the saved
    /// settings of the active profile only.
    fn gyro_note(&self, p: &Profile, in_layer: bool) -> Option<&'static str> {
        let at = self.profile_ref()?;
        if in_layer {
            None
        } else if at != self.saved.active {
            Some("These show the active profile's gyro. Make this profile active to see its settings live.")
        } else if self.saved.profile(&at).is_none_or(|saved| saved.gyro != p.gyro) {
            Some("These show the saved settings. Save & apply to see your changes live.")
        } else {
            None
        }
    }

    /// The profile's tabs (Buttons, Sticks & triggers, Combos, Gyro, Guide) as one segmented
    /// control, with a ⚠ on a tab that has a problem.
    fn profile_segments<'a>(&self, p: &Profile, names: &Names) -> Element<'a, Message> {
        let family = self.glyph_family();
        let segments = ProfileTab::ALL
            .into_iter()
            .map(|t| {
                let label = if section_has_problem(p, t, names) { format!("{t}  ⚠") } else { t.to_string() };
                segment(t.glyph(family), label, 14.0, self.profile_tab == t, Message::SelectProfileTab(t))
            })
            .collect();
        segment_bar(segments)
    }

    /// The live controller drawing, labeled with `labels_from`'s mappings.
    /// Without the daemon the picture is dimmed, as it shows no input. The controller is drawn
    /// dark, so on a dark window it's dimmed less or it would vanish.
    fn picture_opacity(&self) -> f32 {
        match (self.status.is_some(), self.theme().extended_palette().is_dark) {
            (true, _) => 1.0,
            (false, true) => 0.5,
            (false, false) => 0.3,
        }
    }

    /// The line under the live picture: the controller pressed last (and its gyro), the mode and the layers on.
    fn live_caption(&self) -> String {
        let caption = match (&self.live, &self.status) {
            (Some(live), _) => match live.gyro {
                Some([pitch, yaw, roll]) => {
                    let [pitch, yaw, roll] = [pitch, yaw, roll].map(|v| crate::monitor::signed(v, 0));
                    format!("{} · gyro pitch {pitch} yaw {yaw} roll {roll} °/s", live.device)
                }
                None => live.device.clone(),
            },
            (None, Some(_)) => "Press a button on a managed controller".into(),
            (None, None) => NEEDS_DAEMON.into(),
        };
        let layers = self.status.as_ref().map(|s| s.active_layers.as_slice()).unwrap_or_default();
        let caption = match self.status.as_ref().and_then(|s| s.active_mode.as_deref()) {
            Some(mode) => format!("{caption} · Mode: {mode}"),
            None => caption,
        };
        if layers.is_empty() { caption } else { format!("{caption} · Layers: {}", layers.join(" + ")) }
    }

    /// `named` is the setup of `labels_from`, given where the page doesn't already say which profile it is.
    pub(super) fn view_live(&self, labels_from: Option<&Profile>, named: Option<&str>, foldable: bool) -> Element<'_, Message> {
        let folded = foldable && self.picture_hidden;
        let title: Element<'_, Message> = if foldable {
            row![
                text("Live input").size(20),
                button(text(if folded { "Show" } else { "Hide" })).style(style::secondary).on_press(Message::TogglePicture),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            text("Live input").size(20).into()
        };
        if folded {
            return title;
        }
        let pad = self.shown_pad();
        let labels = labels_from.map(drawing_labels).unwrap_or_default();
        let tips = labels_from.map(drawing_tips).unwrap_or_default();
        let caption = match labels_from {
            Some(p) if labels.is_empty() => passthrough_caption(p, named, &self.live_caption()),
            _ => self.live_caption(),
        };
        column![
            title,
            // The labels are placed in the picture's own pixels, so it keeps its size in a narrow
            // window and scrolls sideways there, rather than shrinking out from under them.
            container(
                scrollable(controller_drawing(
                    self.shown_input(),
                    pad.model,
                    // The picture is the controller itself, so it keeps the controller's own labels.
                    Glyphs { family: pad.family, nintendo_layout: false },
                    Callouts { labels: &labels, tips: &tips },
                    self.picture_opacity(),
                ))
                .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
                .width(Length::Shrink)
            )
            .center_x(Length::Fill),
            container(text(caption).size(13).color(MUTED_COLOR).align_x(iced::alignment::Horizontal::Center)).center_x(Length::Fill),
        ]
        .spacing(8)
        .into()
    }

    pub(super) fn view_profile_bar(&self) -> Element<'_, Message> {
        let names: Vec<String> = self.game().profiles.iter().map(|p| p.name.clone()).collect();
        let current = self.profile().map(|p| p.name.clone());
        let can_delete = names.len() > 1;
        let active = self.profile_ref().is_some_and(|at| at == self.saved.active);
        // Nothing is active while the daemon is down, as the header's status says.
        let activate: Element<'_, Message> = if active && self.status.is_some() {
            text("● active").size(13).color(style_accent()).into()
        } else {
            space().into()
        };
        let bar = column![
            row![text("Edit profile").size(20), activate].spacing(12).align_y(Alignment::Center),
            row![
                text("Profile").width(PROFILE_LABEL),
                dropdown(names, current.clone(), Message::EditProfile).width(170),
                space::horizontal(),
                // A list of what to start from, so it's a dropdown that says so.
                dropdown(Template::NEW, None::<Template>, Message::AddProfile)
                    .placeholder("+ New profile from…")
                    .style(style::action_dropdown)
                    .width(220),
                button(text("Duplicate")).style(style::secondary).on_press(Message::AddProfile(Template::Duplicate)),
                delete_profile_button(can_delete),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Name").width(PROFILE_LABEL),
                field("Profile name", current.as_deref().unwrap_or("")).on_input(Message::RenameProfile).width(170),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8);
        container(bar).padding(14).width(Length::Fill).style(style::card).into()
    }

}

#[cfg(test)]
mod tests;
