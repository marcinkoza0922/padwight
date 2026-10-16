//! Picking what an input does: a button showing the action's kind, which opens a dialog of every
//! kind the place allows, in groups, each with its icon and a line on what it does. A pick list
//! can't group its entries or draw icons, and twenty-five bare names don't say which is which.

use iced::widget::{center, column, container, mouse_area, opaque, row, scrollable, space, text};

use super::*;
use crate::{config::Keyword, keyword_icon};

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

/// A tile's width, so the tiles of every group line up in columns.
const TILE_WIDTH: f32 = 250.0;

/// The open dialog: the kind the action has now, and what picking each kind sends.
#[derive(Debug, Clone)]
pub(super) struct ActionPicker {
    current: ActionKind,
    choices: Vec<(ActionKind, Message)>,
}

impl ActionPicker {
    /// The dialog for an action that `on_change` writes back, offering `kinds`.
    fn new(action: &ButtonAction, kinds: &[ActionKind], default_button: Button, names: &Names, on_change: &OnAction<'_>) -> Self {
        let choices = kinds.iter().map(|k| (*k, on_change(new_action(*k, default_button, action, names)))).collect();
        Self { current: action_kind(action), choices }
    }

    /// The dialog for the whole action at `target`, as its editor would open it.
    pub(super) fn for_target(target: Target, action: &ButtonAction, kinds: &[ActionKind], default_button: Button, names: &Names) -> Self {
        Self::new(action, kinds, default_button, names, &set_action(target))
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
        .width(200)
        .on_press_with(move || Message::OpenActionPicker(Box::new(ActionPicker::new(&action, kinds, default_button, &names, &on_change))))
        .into()
}

/// One kind in the dialog: its icon, name and what it does. The action's kind now is picked out.
fn tile<'a>(kind: ActionKind, current: bool, pick: Message) -> Element<'a, Message> {
    let words = column![text(kind.name()).size(14), text(kind.about()).size(12).color(MUTED_COLOR)].spacing(2);
    button(row![icon(kind, 20.0), words].spacing(10).align_y(Alignment::Center))
        .style(move |t, status| if current { style::segment(true)(t, status) } else { style::secondary(t, status) })
        .padding([8, 10])
        .width(TILE_WIDTH)
        .height(Length::Fill)
        .on_press(Message::PickAction(Box::new(pick)))
        .into()
}

/// The dialog over a dimmed page; clicking beside it closes it.
pub(super) fn view_action_picker(picker: &ActionPicker) -> Element<'_, Message> {
    let mut groups = column![].spacing(14);
    for (title, kinds) in GROUPS {
        let tiles: Vec<Element<'_, Message>> = kinds
            .iter()
            .filter_map(|k| picker.choices.iter().find(|(c, _)| c == k))
            .map(|(k, pick)| container(tile(*k, *k == picker.current, pick.clone())).height(62).into())
            .collect();
        if tiles.is_empty() {
            continue;
        }
        groups = groups.push(column![text(title).size(13).color(MUTED_COLOR), row(tiles).spacing(8).wrap().vertical_spacing(8)].spacing(6));
    }
    let dialog = container(
        column![
            row![text("Choose an action").size(20), space::horizontal(), button(text("Cancel")).style(style::secondary).on_press(Message::CloseActionPicker)]
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
        let pick = app.action_picker.as_ref().unwrap().choices.iter().find(|(k, _)| *k == ActionKind::Mouse).unwrap().1.clone();
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
        let offered: Vec<ActionKind> = picker.choices.iter().map(|(k, _)| *k).collect();
        assert_eq!(offered, MACRO_STEP_KINDS);
    }
}
