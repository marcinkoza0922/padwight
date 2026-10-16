//! Picking what an input does: a button showing the action's kind, which opens a dialog of every
//! kind the place allows, in groups, each with its icon and a line on what it does. A pick list
//! can't group its entries or draw icons, and twenty-five bare names don't say which is which.
//! A "Gamepad button" action picks its button the same way, from the glyphs the game will see.

use iced::widget::{center, column, container, mouse_area, opaque, row, scrollable, space, text};

use super::*;
use crate::{config::Keyword, info::Glyphs, keyword_icon};

/// The kinds in the dialog, by group. Each place's own list (`ACTION_KINDS`, a menu item's…) picks
/// from these, and groups it has nothing of are left out.
const GROUPS: [(&str, &[ActionKind]); 6] = [
    ("Press", &[ActionKind::Keys, ActionKind::Mouse, ActionKind::Wheel, ActionKind::Gamepad, ActionKind::Disabled]),
    ("Change how it presses", &[ActionKind::Toggle, ActionKind::Turbo, ActionKind::Multiple, ActionKind::Macro]),
    ("Open or switch", &[ActionKind::Menu, ActionKind::Layer, ActionKind::Mode, ActionKind::Info, ActionKind::Log]),
    ("On-screen tools", &[ActionKind::Overlay, ActionKind::Numpad, ActionKind::QuickSettings, ActionKind::Media]),
    ("Gyro", &[ActionKind::GyroToggle, ActionKind::GyroClutch, ActionKind::GyroHold, ActionKind::GyroRecenter]),
    ("System", &[ActionKind::Screenshot, ActionKind::Recording, ActionKind::ForceQuit]),
];

/// The controller buttons a "Gamepad button" action can press, by group: every one of
/// `Button::EVERY`.
const PAD_GROUPS: [(&str, &[Button]); 6] = [
    ("Face buttons", &[Button::South, Button::East, Button::West, Button::North]),
    ("Bumpers and center", &[Button::LeftBumper, Button::RightBumper, Button::Select, Button::Start, Button::Guide]),
    ("Stick clicks", &[Button::LeftStick, Button::RightStick]),
    ("D-pad", &[Button::DpadUp, Button::DpadDown, Button::DpadLeft, Button::DpadRight]),
    ("Left stick", &[Button::LeftStickUp, Button::LeftStickDown, Button::LeftStickLeft, Button::LeftStickRight]),
    ("Right stick", &[Button::RightStickUp, Button::RightStickDown, Button::RightStickLeft, Button::RightStickRight]),
];

/// A kind's tile width, so the tiles of every group line up in three columns.
const TILE_WIDTH: f32 = 250.0;
/// A controller button's tile: four to the same width.
const PAD_TILE_WIDTH: f32 = (TILE_WIDTH * 3.0 - 8.0) / 4.0;

/// What a tile offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Face {
    Kind(ActionKind),
    Pad(Button),
}

/// A tile: what it offers, whether the action has it now, and what picking it sends.
#[derive(Debug, Clone)]
struct Choice {
    face: Face,
    current: bool,
    pick: Message,
}

/// The open dialog: its tiles by group, and the glyphs its controller buttons are drawn with.
#[derive(Debug, Clone)]
pub(super) struct ActionPicker {
    title: &'static str,
    glyphs: Glyphs,
    groups: Vec<(&'static str, Vec<Choice>)>,
}

impl ActionPicker {
    /// The kinds dialog for an action that `on_change` writes back, offering `kinds`.
    fn kinds(action: &ButtonAction, kinds: &[ActionKind], default_button: Button, names: &Names, on_change: &OnAction<'_>) -> Self {
        let current = action_kind(action);
        let groups = GROUPS.map(|(title, group)| {
            let choices = group
                .iter()
                .filter(|k| kinds.contains(k))
                .map(|k| Choice { face: Face::Kind(*k), current: *k == current, pick: on_change(new_action(*k, default_button, action, names)) })
                .collect();
            (title, choices)
        });
        Self::new("Choose an action", names.glyphs, groups)
    }

    /// The dialog that picks the controller button a "Gamepad button" action presses.
    fn pad_buttons(current: Button, glyphs: Glyphs, on_change: &OnAction<'_>) -> Self {
        let groups = PAD_GROUPS.map(|(title, group)| {
            (title, group.iter().map(|b| Choice { face: Face::Pad(*b), current: *b == current, pick: on_change(ButtonAction::Gamepad(*b)) }).collect())
        });
        Self::new("Choose a controller button", glyphs, groups)
    }

    /// Groups the place offers nothing of are left out.
    fn new(title: &'static str, glyphs: Glyphs, groups: [(&'static str, Vec<Choice>); 6]) -> Self {
        Self { title, glyphs, groups: groups.into_iter().filter(|(_, c)| !c.is_empty()).collect() }
    }

    /// The kinds dialog for the whole action at `target`, as its editor would open it.
    pub(super) fn for_target(target: Target, action: &ButtonAction, kinds: &[ActionKind], default_button: Button, names: &Names) -> Self {
        Self::kinds(action, kinds, default_button, names, &set_action(target))
    }

    /// The controller-button dialog for the "Gamepad button" action at `target`.
    pub(super) fn pad_for_target(target: Target, current: Button, glyphs: Glyphs) -> Self {
        Self::pad_buttons(current, glyphs, &set_action(target))
    }

    /// What picking `face` sends, if the dialog offers it.
    #[cfg(test)]
    fn pick(&self, face: Face) -> Option<&Message> {
        self.groups.iter().flat_map(|(_, c)| c).find(|c| c.face == face).map(|c| &c.pick)
    }
}

/// A controller button by where it sits, beside its glyph, so a glyph that reads differently on
/// another controller (B is East on an Xbox pad and South on a Nintendo one) can't be mistaken.
fn position_name(b: Button) -> &'static str {
    match b {
        Button::South => "South",
        Button::East => "East",
        Button::West => "West",
        Button::North => "North",
        Button::LeftBumper => "Left bumper",
        Button::RightBumper => "Right bumper",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::Guide => "Guide",
        Button::LeftStick => "Left stick click",
        Button::RightStick => "Right stick click",
        Button::DpadUp => "D-pad up",
        Button::DpadDown => "D-pad down",
        Button::DpadLeft => "D-pad left",
        Button::DpadRight => "D-pad right",
        Button::LeftStickUp | Button::RightStickUp => "Up",
        Button::LeftStickDown | Button::RightStickDown => "Down",
        Button::LeftStickLeft | Button::RightStickLeft => "Left",
        Button::LeftStickRight | Button::RightStickRight => "Right",
        Button::LeftPaddle => "Left paddle",
        Button::RightPaddle => "Right paddle",
        Button::LeftPaddle2 => "Left paddle 2",
        Button::RightPaddle2 => "Right paddle 2",
    }
}

/// The full name of a stick direction, for the button showing the pick, where no group title says
/// which stick it is.
fn picked_name(b: Button) -> String {
    match b.stick_direction() {
        Some((stick, _)) => format!("{stick} {}", position_name(b).to_lowercase()),
        None => position_name(b).to_string(),
    }
}

impl ActionKind {
    /// What the kind does, in a line under its name in the dialog.
    fn about(self) -> &'static str {
        match self {
            ActionKind::Keys => "A key, or a chord like Ctrl + C.",
            ActionKind::Mouse => "A mouse button.",
            ActionKind::Wheel => "One notch of the scroll wheel.",
            ActionKind::Gamepad => "A controller button, stick direction or the like.",
            ActionKind::Disabled => "Nothing at all.",
            ActionKind::Toggle => "Turns another action on or off with each press.",
            ActionKind::Turbo => "Repeats another action quickly while held.",
            ActionKind::Multiple => "Presses several outputs together.",
            ActionKind::Macro => "Plays a sequence of presses from the Macros tab.",
            ActionKind::Menu => "Opens a menu to pick from.",
            ActionKind::Layer => "Changes other buttons while held.",
            ActionKind::Mode => "Switches to another set of mappings.",
            ActionKind::Info => "Shows an info overlay.",
            ActionKind::Log => "Shows a log overlay.",
            ActionKind::Overlay => "Opens the on-screen keyboard to type with.",
            ActionKind::Numpad => "Opens an on-screen numpad.",
            ActionKind::QuickSettings => "Opens the Quick Settings menu.",
            ActionKind::Media => "Opens play, pause, volume and track controls.",
            ActionKind::GyroToggle => "Turns gyro aiming on or off with each press.",
            ActionKind::GyroClutch => "Stops the gyro while held, to reposition.",
            ActionKind::GyroHold => "Gyro aims only while held.",
            ActionKind::GyroRecenter => "Takes the controller's position now as center.",
            ActionKind::Screenshot => "Saves a picture of the screen.",
            ActionKind::Recording => "Starts or stops recording the screen.",
            ActionKind::ForceQuit => "Held for 2 seconds, closes the focused program.",
        }
    }

    /// The name without the "…" that says a pick list entry has more settings: the dialog's tiles
    /// and the button showing the pick don't need it.
    fn name(self) -> String {
        self.to_string().trim_end_matches('…').to_string()
    }
}

/// The kind's icon: a keyword's own, in its color, as the summaries draw it, or a plain one.
fn icon<'a>(k: ActionKind, size: f32) -> Element<'a, Message> {
    let keyword = match k {
        ActionKind::Toggle => Some(Keyword::Toggle),
        ActionKind::Turbo => Some(Keyword::Turbo),
        ActionKind::Macro => Some(Keyword::Macro),
        ActionKind::Menu => Some(Keyword::Menu),
        ActionKind::Info => Some(Keyword::Info),
        ActionKind::Log => Some(Keyword::Log),
        ActionKind::Layer => Some(Keyword::Layer),
        ActionKind::Mode => Some(Keyword::Mode),
        _ => None,
    };
    let svg_text = keyword.map_or_else(|| plain_icon(k), keyword_icon::svg);
    svg(svg::Handle::from_memory(svg_text.into_bytes())).width(size).height(size).into()
}

/// The kinds without a keyword: a line drawing in a neutral gray that reads in either theme.
fn plain_icon(k: ActionKind) -> String {
    let body = match k {
        ActionKind::Keys => "<rect x='3' y='6' width='18' height='12' rx='2.5'/><path d='M7 10h.01M11 10h.01M15 10h.01M8 14h8'/>",
        ActionKind::Overlay => "<rect x='2' y='5' width='20' height='14' rx='2.5'/><path d='M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12.5h.01M18 12.5h.01M8 16h8'/>",
        ActionKind::Mouse => "<rect x='6' y='3' width='12' height='18' rx='6'/><path d='M12 3v7'/>",
        ActionKind::Wheel => "<path d='M12 4v16M8 8l4-4 4 4M8 16l4 4 4-4'/>",
        ActionKind::Gamepad => "<path d='M7 7h10a5 5 0 0 1 5 5v1.5a3 3 0 0 1-5.6 1.5L15 13H9l-1.4 2A3 3 0 0 1 2 13.5V12a5 5 0 0 1 5-5Z'/><path d='M7 9.5v3M5.5 11h3M16 10h.01M18 12h.01'/>",
        ActionKind::Disabled => "<circle cx='12' cy='12' r='8.5'/><path d='M6 18 18 6'/>",
        ActionKind::Numpad => "<path d='M6 6h.01M12 6h.01M18 6h.01M6 12h.01M12 12h.01M18 12h.01M6 18h.01M12 18h.01M18 18h.01' stroke-width='3.5'/>",
        ActionKind::QuickSettings => "<path d='M4 7h16M4 17h16'/><circle cx='9' cy='7' r='2.2' fill='#8a8f98'/><circle cx='15' cy='17' r='2.2' fill='#8a8f98'/>",
        ActionKind::Media => "<path d='M8 5v14l11-7Z'/>",
        // Two outputs stacked: one press, both sent.
        ActionKind::Multiple => "<rect x='3' y='8' width='13' height='13' rx='2'/><path d='M8 8V5a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-3'/>",
        ActionKind::GyroToggle | ActionKind::GyroClutch | ActionKind::GyroHold | ActionKind::GyroRecenter => {
            "<circle cx='12' cy='12' r='8.5'/><ellipse cx='12' cy='12' rx='8.5' ry='3.5'/><circle cx='12' cy='12' r='1.5' fill='#8a8f98'/>"
        }
        ActionKind::Screenshot => "<path d='M4 8h3l2-3h6l2 3h3v11H4Z'/><circle cx='12' cy='13' r='3.5'/>",
        ActionKind::Recording => "<circle cx='12' cy='12' r='8.5'/><circle cx='12' cy='12' r='4' fill='#8a8f98'/>",
        ActionKind::ForceQuit => "<path d='M12 3v8M6.3 6.3a8 8 0 1 0 11.4 0'/>",
        _ => "",
    };
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8a8f98' stroke-width='2' \
         stroke-linecap='round' stroke-linejoin='round'>{body}</svg>"
    )
}

/// The button an action editor shows its kind with, drawn as a dropdown is. It opens the dialog
/// for the action that `on_change` writes back, offering `kinds`.
pub(super) fn kind_button<'a>(
    action: &ButtonAction,
    kinds: &'static [ActionKind],
    default_button: Button,
    names: &Names,
    on_change: OnAction<'a>,
) -> Element<'a, Message> {
    let kind = action_kind(action);
    let arrow = text("▾").size(14).style(|theme: &iced::Theme| text::Style { color: Some(theme.extended_palette().primary.base.color) });
    let (action, names) = (action.clone(), names.clone());
    // What each pick sends is worked out on the click, not on every redraw of every editor.
    button(row![icon(kind, 16.0), text(kind.name()).width(Length::Fill), arrow].spacing(8).align_y(Alignment::Center))
        .style(style::field_button)
        .padding([6, 10])
        .width(VALUE_WIDTH)
        .on_press_with(move || Message::OpenActionPicker(Box::new(ActionPicker::kinds(&action, kinds, default_button, &names, &on_change))))
        .into()
}

/// The button a "Gamepad button" action shows its button with: the glyph the game will see and
/// where the button sits. It opens the dialog of every controller button.
pub(super) fn pad_button<'a>(current: Button, glyphs: Glyphs, on_change: OnAction<'a>) -> Element<'a, Message> {
    let arrow = text("▾").size(14).style(|theme: &iced::Theme| text::Style { color: Some(theme.extended_palette().primary.base.color) });
    button(row![pad_chip(current, glyphs), text(picked_name(current)).width(Length::Fill), arrow].spacing(8).align_y(Alignment::Center))
        .style(style::field_button)
        .padding([4, 10])
        .width(VALUE_WIDTH)
        .on_press_with(move || Message::OpenActionPicker(Box::new(ActionPicker::pad_buttons(current, glyphs, &on_change))))
        .into()
}

/// One tile: a kind's icon, name and what it does, or a controller button's glyph and where it
/// sits. What the action has now is picked out.
fn tile<'a>(choice: &Choice, glyphs: Glyphs) -> Element<'a, Message> {
    let (content, width, height): (Element<'a, Message>, f32, f32) = match choice.face {
        Face::Kind(kind) => {
            let words = column![text(kind.name()).size(14), text(kind.about()).size(12).color(MUTED_COLOR)].spacing(2);
            (row![icon(kind, 20.0), words].spacing(10).align_y(Alignment::Center).into(), TILE_WIDTH, 62.0)
        }
        Face::Pad(b) => (row![pad_chip(b, glyphs), text(position_name(b)).size(14)].spacing(10).align_y(Alignment::Center).into(), PAD_TILE_WIDTH, 40.0),
    };
    let current = choice.current;
    button(content)
        .style(move |t, status| if current { style::segment(true)(t, status) } else { style::secondary(t, status) })
        .padding([8, 10])
        .width(width)
        .height(height)
        .on_press(Message::PickAction(Box::new(choice.pick.clone())))
        .into()
}

/// The dialog over a dimmed page; clicking beside it closes it.
pub(super) fn view_action_picker(picker: &ActionPicker) -> Element<'_, Message> {
    let mut groups = column![].spacing(14);
    for (title, choices) in &picker.groups {
        let tiles = choices.iter().map(|c| tile(c, picker.glyphs));
        groups = groups.push(column![text(*title).size(13).color(MUTED_COLOR), row(tiles).spacing(8).wrap().vertical_spacing(8)].spacing(6));
    }
    let dialog = container(
        column![
            row![text(picker.title).size(20), space::horizontal(), button(text("Cancel")).style(style::secondary).on_press(Message::CloseActionPicker)]
                .align_y(Alignment::Center),
            // Room on the right for the scrollbar, which shows in a short window.
            scrollable(groups.padding(iced::Padding::default().right(16))).height(Length::Shrink),
        ]
        .spacing(14),
    )
    .padding(20)
    .max_width(TILE_WIDTH * 3.0 + 8.0 * 2.0 + 16.0 + 40.0)
    .style(style::dialog);
    let backdrop = container(center(opaque(dialog)).padding(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style { background: Some(Color { a: 0.6, ..Color::BLACK }.into()), ..container::Style::default() });
    opaque(mouse_area(backdrop).on_press(Message::CloseActionPicker))
}

impl App {
    pub(super) fn update_action_picker(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenActionPicker(picker) => self.action_picker = Some(*picker),
            Message::CloseActionPicker => self.action_picker = None,
            Message::PickAction(pick) => {
                self.action_picker = None;
                return Task::done(*pick);
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn every_kind_is_in_exactly_one_group() {
        for k in ACTION_KINDS {
            assert_eq!(GROUPS.iter().filter(|(_, kinds)| kinds.contains(&k)).count(), 1, "{k:?}");
        }
        assert_eq!(GROUPS.iter().map(|(_, kinds)| kinds.len()).sum::<usize>(), ACTION_KINDS.len());
    }

    #[test]
    fn picking_a_kind_sets_it_and_closes_the_dialog() {
        let mut app = app();
        let target = Target::Button(Button::South);
        let current = app.profile().unwrap().button(Button::South).clone();
        let picker = ActionPicker::for_target(target, &current, &ACTION_KINDS, Button::South, &app.names());
        let _ = app.update(Message::OpenActionPicker(Box::new(picker)));
        let pick = app.action_picker.as_ref().unwrap().pick(Face::Kind(ActionKind::Mouse)).unwrap().clone();
        let task = app.update(Message::PickAction(Box::new(pick.clone())));
        drop(task);
        assert!(app.action_picker.is_none());
        let _ = app.update(pick);
        assert_eq!(app.profile().unwrap().button(Button::South), &ButtonAction::Mouse(MouseButton::Left));
    }

    #[test]
    fn a_place_offers_only_its_own_kinds() {
        let app = app();
        let picker = ActionPicker::for_target(Target::MacroStep(0, 0), &ButtonAction::Keys(Vec::new()), MACRO_STEP_KINDS, Button::South, &app.names());
        let mut offered: Vec<ActionKind> =
            picker.groups.iter().flat_map(|(_, c)| c).filter_map(|c| match c.face { Face::Kind(k) => Some(k), Face::Pad(_) => None }).collect();
        offered.sort_by_key(|k| MACRO_STEP_KINDS.iter().position(|m| m == k));
        assert_eq!(offered, MACRO_STEP_KINDS);
    }

    #[test]
    fn the_pad_dialog_offers_every_output_button_once() {
        let picker = ActionPicker::pad_for_target(Target::Button(Button::South), Button::East, Glyphs::default());
        let offered: Vec<Button> = picker.groups.iter().flat_map(|(_, c)| c).filter_map(|c| match c.face { Face::Pad(b) => Some(b), Face::Kind(_) => None }).collect();
        assert_eq!(offered.len(), Button::EVERY.len());
        assert!(Button::EVERY.iter().all(|b| offered.contains(b)));
        let current: Vec<Face> = picker.groups.iter().flat_map(|(_, c)| c).filter(|c| c.current).map(|c| c.face).collect();
        assert_eq!(current, [Face::Pad(Button::East)]);
    }

    #[test]
    fn pad_glyphs_follow_the_controller_the_game_is_shown() {
        let mut app = with_game();
        let pick = PadPick { model: Some(crate::info::PadModel::DualShock4), family: PadFamily::PlayStation };
        app.pad_shown = Some(PadShown { pick, since: std::time::Instant::now() });
        // A setup that doesn't list the DualShock gets an Xbox 360 pad, whatever is in hand.
        assert_eq!(app.names().glyphs.family, PadFamily::Xbox);
        app.game_mut().controllers = Some(crate::pad_identity::ControllerSupport { dualshock: true, ..Default::default() });
        assert_eq!(app.names().glyphs.family, PadFamily::PlayStation);
    }

    #[test]
    fn picking_a_pad_button_sets_it() {
        let mut app = app();
        let picker = ActionPicker::pad_for_target(Target::Button(Button::North), Button::South, Glyphs::default());
        let _ = app.update(picker.pick(Face::Pad(Button::RightStickLeft)).unwrap().clone());
        assert_eq!(app.profile().unwrap().button(Button::North), &ButtonAction::Gamepad(Button::RightStickLeft));
    }
}
