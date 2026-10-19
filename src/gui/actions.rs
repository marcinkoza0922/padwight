//! The action editor (what an input does, nested in Multi, Toggle and Turbo) and summaries of actions.

use iced::widget::{column, row};

use super::*;

/// The "+ New layer" choice in a layer picker; replaced by a new layer's name when chosen.
pub(super) const NEW_LAYER: &str = "+ New layer";
/// The "+ New mode" choice in a mode picker, likewise.
pub(super) const NEW_MODE: &str = "+ New mode";

/// One-line description of an action, for collapsed rows and the controller drawing.
pub(super) fn summarize(action: &ButtonAction) -> String {
    action.summary()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActionKind {
    Disabled,
    Gamepad,
    PadTrigger,
    PadStick,
    Keys,
    Mouse,
    Wheel,
    Overlay,
    Numpad,
    Screenshot,
    Recording,
    Media,
    QuickSettings,
    ForceQuit,
    GyroToggle,
    GyroClutch,
    GyroHold,
    GyroRecenter,
    Toggle,
    Turbo,
    Macro,
    Menu,
    Info,
    Log,
    Layer,
    Mode,
    Multiple,
}

impl fmt::Display for ActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ActionKind::Disabled => "Disabled",
            ActionKind::Gamepad => "Gamepad button",
            ActionKind::PadTrigger => "Gamepad trigger",
            ActionKind::PadStick => "Gamepad stick",
            ActionKind::Keys => "Keys",
            ActionKind::Mouse => "Mouse click",
            ActionKind::Wheel => "Scroll wheel",
            ActionKind::Overlay => "On-screen keyboard",
            ActionKind::Numpad => "On-screen numpad",
            ActionKind::Screenshot => "Screenshot",
            ActionKind::Recording => "Start or stop recording",
            ActionKind::Media => "Media controls",
            ActionKind::QuickSettings => "Quick Settings",
            ActionKind::ForceQuit => "Force quit (hold)",
            ActionKind::GyroToggle => "Gyro on / off",
            ActionKind::GyroClutch => "Gyro clutch (off while held)",
            ActionKind::GyroHold => "Gyro on while held",
            ActionKind::GyroRecenter => "Recenter gyro",
            ActionKind::Toggle => "Toggle on / off…",
            ActionKind::Macro => "Macro…",
            ActionKind::Menu => "Open menu…",
            ActionKind::Info => "Info overlay…",
            ActionKind::Log => "Log overlay…",
            ActionKind::Layer => "Hold layer…",
            ActionKind::Mode => "Switch mode…",
            ActionKind::Turbo => "Turbo…",
            ActionKind::Multiple => "Several at once…",
        })
    }
}

/// Every kind, for a top-level action.
pub(super) const ACTION_KINDS: [ActionKind; 27] = [
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
    ActionKind::Overlay,
    ActionKind::Numpad,
    ActionKind::Screenshot,
    ActionKind::Recording,
    ActionKind::Media,
    ActionKind::QuickSettings,
    ActionKind::ForceQuit,
    ActionKind::GyroToggle,
    ActionKind::GyroClutch,
    ActionKind::GyroHold,
    ActionKind::GyroRecenter,
    ActionKind::Toggle,
    ActionKind::Turbo,
    ActionKind::Macro,
    ActionKind::Menu,
    ActionKind::Info,
    ActionKind::Log,
    ActionKind::Layer,
    ActionKind::Mode,
    ActionKind::Multiple,
];

/// Radial menu items: everything but opening another menu.
pub(super) const RADIAL_ITEM_KINDS: &[ActionKind] = &[
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
    ActionKind::Overlay,
    ActionKind::Numpad,
    ActionKind::Toggle,
    ActionKind::Macro,
    ActionKind::Mode,
    ActionKind::Multiple,
];

/// What a menu item can do: anything a button can, including opening a submenu.
pub(super) const MENU_ITEM_KINDS: &[ActionKind] = &[
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
    ActionKind::Overlay,
    ActionKind::Numpad,
    ActionKind::Screenshot,
    ActionKind::GyroToggle,
    ActionKind::GyroRecenter,
    ActionKind::Toggle,
    ActionKind::Macro,
    ActionKind::Menu,
    // A menu item is a tap: it can't hold a layer, but it can switch modes.
    ActionKind::Mode,
    ActionKind::Multiple,
];

/// What an item of a menu without a close button can do: anything but open another menu, which
/// it would have no way back from.
pub(super) const NO_SUBMENU_ITEM_KINDS: &[ActionKind] = &[
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
    ActionKind::Overlay,
    ActionKind::Numpad,
    ActionKind::Screenshot,
    ActionKind::GyroToggle,
    ActionKind::GyroRecenter,
    ActionKind::Toggle,
    ActionKind::Macro,
    ActionKind::Mode,
    ActionKind::Multiple,
];

/// What a macro step can press: plain outputs only.
pub(super) const MACRO_STEP_KINDS: &[ActionKind] =
    &[ActionKind::Keys, ActionKind::Mouse, ActionKind::Wheel, ActionKind::Gamepad];

/// Entries of a Multiple list.
pub(super) const MULTI_ENTRY_KINDS: &[ActionKind] = &[
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
];

/// What a Toggle can hold down: basic outputs, a turbo (toggleable auto-fire) or several. Not a
/// layer: one that stays on is a mode.
pub(super) const TOGGLE_INNER_KINDS: &[ActionKind] = &[
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
    ActionKind::Turbo,
    // A repeating macro toggled on loops until toggled off.
    ActionKind::Macro,
    // Shown until toggled off.
    ActionKind::Info,
    ActionKind::Log,
    ActionKind::Multiple,
];

/// What a Turbo can repeat.
pub(super) const TURBO_INNER_KINDS: &[ActionKind] = &[
    ActionKind::Disabled,
    ActionKind::Gamepad,
    ActionKind::PadTrigger,
    ActionKind::PadStick,
    ActionKind::Keys,
    ActionKind::Mouse,
    ActionKind::Wheel,
    // Played once every so often, rather than looping.
    ActionKind::Macro,
    ActionKind::Multiple,
];

pub(super) const DEFAULT_TURBO_RATE: f32 = 10.0;

/// The width of an action's kind button and of the picker beside it, so they line up row to row.
pub(super) const VALUE_WIDTH: f32 = 200.0;

/// Callback that turns an edited action into a message; lets editors nest inside `Multi`.
pub(super) type OnAction<'a> = Rc<dyn Fn(ButtonAction) -> Message + 'a>;

pub(super) fn set_action<'a>(target: Target) -> OnAction<'a> {
    Rc::new(move |a| Message::SetAction(target, a))
}

/// Editor for one action. `nested` editors (entries of a Multiple list) can't be Multiple.
#[expect(clippy::too_many_arguments, reason = "predates the size lints")]
pub(super) fn action_editor<'a>(
    action: &'a ButtonAction,
    default_button: Button,
    kinds: &'static [ActionKind],
    on_change: OnAction<'a>,
    field: KeyField,
    names: &Names,
) -> Element<'a, Message> {
    let kind_picker = super::action_picker::kind_button(action, kinds, default_button, names, on_change.clone());
    let value = action_value(action, default_button, on_change, field, names);
    // Where there isn't room beside the kind, the value goes below it.
    row![kind_picker, value].spacing(8).align_y(Alignment::Start).wrap().vertical_spacing(6).into()
}

pub(super) fn action_kind(action: &ButtonAction) -> ActionKind {
    match action {
        ButtonAction::Disabled => ActionKind::Disabled,
        ButtonAction::Gamepad(_) => ActionKind::Gamepad,
        ButtonAction::PadTrigger { .. } => ActionKind::PadTrigger,
        ButtonAction::PadStick { .. } => ActionKind::PadStick,
        ButtonAction::Keys(_) => ActionKind::Keys,
        ButtonAction::Mouse(_) => ActionKind::Mouse,
        ButtonAction::Wheel(_) => ActionKind::Wheel,
        ButtonAction::ToggleOverlay => ActionKind::Overlay,
        ButtonAction::ToggleNumpad => ActionKind::Numpad,
        ButtonAction::Screenshot => ActionKind::Screenshot,
        ButtonAction::ToggleRecording => ActionKind::Recording,
        ButtonAction::ToggleMedia => ActionKind::Media,
        ButtonAction::QuickSettings => ActionKind::QuickSettings,
        ButtonAction::ForceQuit => ActionKind::ForceQuit,
        ButtonAction::GyroToggle => ActionKind::GyroToggle,
        ButtonAction::GyroClutch => ActionKind::GyroClutch,
        ButtonAction::GyroHold => ActionKind::GyroHold,
        ButtonAction::GyroRecenter => ActionKind::GyroRecenter,
        ButtonAction::Multi(_) => ActionKind::Multiple,
        ButtonAction::Toggle(_) => ActionKind::Toggle,
        ButtonAction::Turbo { .. } => ActionKind::Turbo,
        ButtonAction::Macro { .. } => ActionKind::Macro,
        ButtonAction::OpenMenu(_) => ActionKind::Menu,
        ButtonAction::ShowInfo(_) => ActionKind::Info,
        ButtonAction::ShowLog(_) => ActionKind::Log,
        ButtonAction::Layer(_) => ActionKind::Layer,
        ButtonAction::Mode(_) => ActionKind::Mode,
    }
}

/// A fresh action of kind `k`, replacing `current`.
pub(super) fn new_action(k: ActionKind, default_button: Button, current: &ButtonAction, names: &Names) -> ButtonAction {
    // When wrapping in Toggle/Turbo, keep a simple existing action as the thing wrapped.
    let wrappable = match current {
        ButtonAction::Gamepad(_) | ButtonAction::PadTrigger { .. } | ButtonAction::PadStick { .. } | ButtonAction::Keys(_) | ButtonAction::Mouse(_) | ButtonAction::Wheel(_) => {
            Some(current.clone())
        }
        _ => None,
    };
    match k {
        ActionKind::Disabled => ButtonAction::Disabled,
        // A stick direction isn't a button the pad can press, so those rows start on South.
        ActionKind::Gamepad => ButtonAction::Gamepad(if default_button.stick_direction().is_some() { Button::South } else { default_button }),
        ActionKind::PadTrigger => ButtonAction::PadTrigger { trigger: Trigger::Right, amount: 1.0 },
        ActionKind::PadStick => ButtonAction::PadStick { stick: Stick::Left, angle: 0.0, amount: 1.0 },
        ActionKind::Keys => ButtonAction::Keys(Vec::new()),
        ActionKind::Mouse => ButtonAction::Mouse(MouseButton::Left),
        ActionKind::Wheel => ButtonAction::Wheel(WheelDirection::Up),
        ActionKind::Overlay => ButtonAction::ToggleOverlay,
        ActionKind::Numpad => ButtonAction::ToggleNumpad,
        ActionKind::Screenshot => ButtonAction::Screenshot,
        ActionKind::Recording => ButtonAction::ToggleRecording,
        ActionKind::Media => ButtonAction::ToggleMedia,
        ActionKind::QuickSettings => ButtonAction::QuickSettings,
        ActionKind::ForceQuit => ButtonAction::ForceQuit,
        ActionKind::GyroToggle => ButtonAction::GyroToggle,
        ActionKind::GyroClutch => ButtonAction::GyroClutch,
        ActionKind::GyroHold => ButtonAction::GyroHold,
        ActionKind::GyroRecenter => ButtonAction::GyroRecenter,
        // Keep what was there as the first entry.
        ActionKind::Multiple => ButtonAction::Multi(wrappable.into_iter().collect()),
        ActionKind::Toggle => ButtonAction::toggle(wrappable.unwrap_or(ButtonAction::Keys(Vec::new()))),
        ActionKind::Turbo => ButtonAction::Turbo {
            action: Box::new(wrappable.unwrap_or(ButtonAction::Mouse(MouseButton::Left))),
            rate: DEFAULT_TURBO_RATE,
            every_ms: 0,
        },
        ActionKind::Macro => ButtonAction::Macro { name: names.macros.first().cloned().unwrap_or_default(), repeat: false },
        ActionKind::Menu => ButtonAction::OpenMenu(names.menus.first().cloned().unwrap_or_default()),
        ActionKind::Info => ButtonAction::ShowInfo(names.infos.first().cloned().unwrap_or_default()),
        ActionKind::Log => ButtonAction::ShowLog(names.logs.first().cloned().unwrap_or_default()),
        // With no layers yet, picking the kind makes one.
        ActionKind::Layer => ButtonAction::Layer(names.layers.first().cloned().unwrap_or_else(|| NEW_LAYER.into())),
        ActionKind::Mode => ButtonAction::Mode(switchable_modes(names).first().cloned().unwrap_or_else(|| NEW_MODE.into())),
    }
}

/// The modes an action can switch to: the game's, and mouse mode. The Guide mode belongs to the
/// Guide button.
fn switchable_modes(names: &Names) -> Vec<String> {
    names.modes.iter().filter(|m| *m != crate::config::GUIDE_MODE).cloned().collect()
}

/// A Turbo's settings: a rate for keys, buttons and the like, or the gap between presses for a
/// macro, and the action it repeats (`turbo` is that action, its rate and its gap).
fn turbo_editor<'a>(
    turbo: (&'a ButtonAction, f32, u64),
    default_button: Button,
    on_change: OnAction<'a>,
    field: &KeyField,
    names: &Names,
) -> Element<'a, Message> {
    let (inner, rate, every_ms) = turbo;
    let parent = on_change.clone();
    // A macro put inside starts out taking as long as it does.
    let macro_ms_of = names.macro_ms.clone();
    let wrap: OnAction<'a> = Rc::new(move |a: ButtonAction| {
        let every = match &a {
            ButtonAction::Macro { name, .. } if every_ms == 0 => macro_ms_of.get(name).copied().unwrap_or(0),
            _ => every_ms,
        };
        parent(ButtonAction::Turbo { action: Box::new(a), rate, every_ms: every })
    });
    let pace: Element<'a, Message> = match inner {
        ButtonAction::Macro { name, .. } => {
            let macro_ms = names.macro_ms.get(name).copied().unwrap_or(0);
            let shown = crate::config::macro_turbo_ms(every_ms, macro_ms);
            let repeated = inner.clone();
            let set_every = move |ms: u64| on_change(ButtonAction::Turbo { action: Box::new(repeated.clone()), rate, every_ms: ms });
            let mut line = row![text("Press every").size(13), ms_field(every_ms, set_every)]
                .spacing(8)
                .align_y(Alignment::Center);
            if shown != every_ms {
                line = line
                    .push(text(format!("(raised to {shown} ms, because the macro takes that long to play)")).size(12).color(MUTED_COLOR));
            }
            line.into()
        }
        _ => {
            let repeated = inner.clone();
            let set_rate = move |r: f32| on_change(ButtonAction::Turbo { action: Box::new(repeated.clone()), rate: r, every_ms });
            row![
                slider(2.0..=30.0, rate, set_rate).step(1.0_f32).width(200),
                text(format!("{rate:.0} presses/s")).size(13),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
        }
    };
    let inside = Names { in_turbo: true, ..names.clone() };
    column![
        wrapped("Repeats while held:", action_editor(inner, default_button, TURBO_INNER_KINDS, wrap, field.child(0), &inside)),
        pace,
    ]
    .spacing(6)
    .into()
}

/// The action a Toggle or Turbo wraps, boxed like a "Several at once" list so it reads as sitting
/// inside the outer action rather than beside it.
fn wrapped<'a>(caption: &'a str, inner: Element<'a, Message>) -> Element<'a, Message> {
    container(column![text(caption).size(12).color(MUTED_COLOR), inner].spacing(4))
        .padding(8)
        .width(Length::Fill)
        .style(style::output_list)
        .into()
}

/// An empty picker's note, with a link to the tab that makes what it would pick. `after` is any
/// text that follows the link.
fn tab_link<'a>(before: &str, link: &str, tab: GameTab, after: &str) -> Element<'a, Message> {
    let mut line = row![
        text(before.to_string()).size(12).color(MUTED_COLOR),
        button(text(link.to_string()).size(12)).style(button::text).padding(0).on_press(Message::SelectGameTab(tab)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if !after.is_empty() {
        line = line.push(text(after.to_string()).size(12).color(MUTED_COLOR));
    }
    line.into()
}

/// The settings to the right of an action's kind picker.
#[expect(clippy::too_many_lines, reason = "predates the size lints")]
pub(super) fn action_value<'a>(
    action: &'a ButtonAction,
    default_button: Button,
    on_change: OnAction<'a>,
    field: KeyField,
    names: &Names,
) -> Element<'a, Message> {
    match action {
        ButtonAction::Gamepad(b) => super::action_picker::pad_button(*b, names.glyphs, on_change),
        ButtonAction::PadTrigger { trigger, amount } => super::pad_push::trigger_editor(*trigger, *amount, on_change),
        ButtonAction::PadStick { stick, angle, amount } => super::pad_push::stick_editor(*stick, *angle, *amount, on_change),
        ButtonAction::Mouse(m) => dropdown(MouseButton::ALL, Some(*m), move |m| {
            on_change(ButtonAction::Mouse(m))
        })
        .width(VALUE_WIDTH)
        .into(),
        ButtonAction::Wheel(d) => dropdown(WheelDirection::ALL, Some(*d), move |d| {
            on_change(ButtonAction::Wheel(d))
        })
        .width(VALUE_WIDTH)
        .into(),
        ButtonAction::Keys(keys) => key_input(
            keys,
            "e.g. Left Ctrl+C",
            move |s| on_change(ButtonAction::Keys(text_to_keys(&s))),
            Message::OpenKeyPicker(field, keys.clone(), false),
        ),
        ButtonAction::Multi(list) => multi_editor(list, default_button, on_change, field, names),
        ButtonAction::Toggle(Toggled { action: inner, start_on }) => {
            let start_on = *start_on;
            let parent = on_change.clone();
            let wrap: OnAction<'a> =
                Rc::new(move |a| parent(ButtonAction::Toggle(Toggled { action: Box::new(a), start_on })));
            let starting = {
                let inner = inner.clone();
                move |on: bool| on_change(ButtonAction::Toggle(Toggled { action: inner.clone(), start_on: on }))
            };
            column![
                wrapped(
                    "Each press turns this on or off:",
                    action_editor(inner, default_button, TOGGLE_INNER_KINDS, wrap, field.child(0), names),
                ),
                tooltip(
                    checkbox(start_on).label("On when the game starts").size(14).text_size(12).on_toggle(starting),
                    container(
                        text(
                            "Switched on by itself the first time the game is focused after it starts, \
                             e.g. a controls overlay that stays up until this is pressed."
                        )
                        .size(13)
                    )
                    .padding(8)
                    .max_width(320.0)
                    .style(style::tooltip),
                    tooltip::Position::Top,
                ),
            ]
            .spacing(6)
            .into()
        }
        ButtonAction::Turbo { action: inner, rate, every_ms } => {
            turbo_editor((inner, *rate, *every_ms), default_button, on_change, &field, names)
        }
        ButtonAction::Macro { .. } if names.macros.is_empty() => {
            tab_link("No macros yet.", "Create one on the Macros tab", GameTab::Macros, "")
        }
        ButtonAction::Macro { name, repeat } => {
            let repeat = *repeat;
            let pick = {
                let on_change = on_change.clone();
                dropdown(names.macros.clone(), Some(name.clone()), move |n| on_change(ButtonAction::Macro { name: n, repeat }))
                    .width(VALUE_WIDTH)
            };
            let name = name.clone();
            let missing = !names.macros.contains(&name);
            let mut line = row![
                pick,
            ]
            .spacing(12)
            .align_y(Alignment::Center);
            let goto = goto_button(ItemKind::Macro, &name, !missing);
            // A macro in a turbo plays once per press, so there's nothing to loop.
            if !names.in_turbo {
                line = line.push(
                    checkbox(repeat)
                        .label("Repeat while held")
                        .on_toggle(move |r| on_change(ButtonAction::Macro { name: name.clone(), repeat: r })),
                );
            }
            if missing {
                line = line.push(text("missing macro").size(12).color(ERROR_COLOR));
            }
            line.push(goto).into()
        }
        ButtonAction::Disabled
        | ButtonAction::Screenshot
        | ButtonAction::ToggleRecording
        | ButtonAction::QuickSettings
        | ButtonAction::GyroToggle
        | ButtonAction::GyroClutch
        | ButtonAction::GyroHold
        | ButtonAction::GyroRecenter => space().into(),
        ButtonAction::ToggleMedia => text("D-pad left and right seek, up and down change the volume, bumpers change track, A plays or pauses, B closes.").size(12).color(MUTED_COLOR).into(),
        ButtonAction::ForceQuit => {
            text("Hold for 2 seconds to close the program in the focused window. The desktop is never closed.")
                .size(12)
                .color(MUTED_COLOR)
                .into()
        }
        ButtonAction::ToggleOverlay | ButtonAction::ToggleNumpad => {
            text("Hold B on the controller to close it.").size(12).color(MUTED_COLOR).into()
        }
        ButtonAction::ShowInfo(_) if names.infos.is_empty() => {
            tab_link(
                "No info overlays to show.",
                "Create one on the Info overlays tab",
                GameTab::Info,
                ", and turn off Always so an action can show it.",
            )
        }
        ButtonAction::ShowInfo(name) => {
            let mut line = row![dropdown(names.infos.clone(), Some(name.clone()), move |n| on_change(ButtonAction::ShowInfo(n))).width(VALUE_WIDTH)]
                .spacing(8)
                .align_y(Alignment::Center);
            if names.always_infos.contains(name) {
                line = line.push(text("always on screen, so it can't be triggered").size(12).color(ERROR_COLOR));
            } else if !names.infos.contains(name) {
                line = line.push(text("missing info overlay").size(12).color(ERROR_COLOR));
            }
            line.push(goto_button(ItemKind::Info, name, names.infos.contains(name) || names.always_infos.contains(name))).into()
        }
        ButtonAction::ShowLog(_) if names.logs.is_empty() => {
            tab_link(
                "No log overlays to show.",
                "Create one on the Log overlays tab",
                GameTab::Logs,
                ", and turn off Always so an action can show it.",
            )
        }
        ButtonAction::ShowLog(name) => {
            let mut line = row![dropdown(names.logs.clone(), Some(name.clone()), move |n| on_change(ButtonAction::ShowLog(n))).width(VALUE_WIDTH)]
                .spacing(8)
                .align_y(Alignment::Center);
            if names.always_logs.contains(name) {
                line = line.push(text("always on screen, so it can't be triggered").size(12).color(ERROR_COLOR));
            } else if !names.logs.contains(name) {
                line = line.push(text("missing log overlay").size(12).color(ERROR_COLOR));
            }
            line.push(goto_button(ItemKind::Log, name, names.logs.contains(name) || names.always_logs.contains(name))).into()
        }
        ButtonAction::OpenMenu(_) if names.menus.is_empty() => {
            tab_link("No menus to open.", "Create one on the Menus tab", GameTab::Menus, "")
        }
        ButtonAction::Layer(name) => {
            let mut options = names.layers.clone();
            options.push(NEW_LAYER.to_string());
            let mut line = row![dropdown(options, Some(name.clone()), move |n| on_change(ButtonAction::Layer(n))).width(VALUE_WIDTH)]
                .spacing(8)
                .align_y(Alignment::Center);
            if !names.layers.contains(name) {
                line = line.push(text("missing layer").size(12).color(ERROR_COLOR));
            }
            line.push(goto_button(ItemKind::Layer, name, names.layers.contains(name))).into()
        }
        ButtonAction::Mode(name) => {
            let mut options = switchable_modes(names);
            options.push(NEW_MODE.to_string());
            let mut line = row![dropdown(options, Some(name.clone()), move |n| on_change(ButtonAction::Mode(n))).width(VALUE_WIDTH)]
                .spacing(8)
                .align_y(Alignment::Center);
            if !names.modes.contains(name) {
                line = line.push(text("missing mode").size(12).color(ERROR_COLOR));
            }
            line.push(goto_button(ItemKind::Mode, name, names.modes.contains(name))).into()
        }
        ButtonAction::OpenMenu(name) => {
            let mut line = row![dropdown(names.menus.clone(), Some(name.clone()), move |n| on_change(ButtonAction::OpenMenu(n))).width(VALUE_WIDTH)]
                .spacing(8)
                .align_y(Alignment::Center);
            if !names.menus.contains(name) {
                line = line.push(text("missing menu").size(12).color(ERROR_COLOR));
            }
            line.push(goto_button(ItemKind::Menu, name, names.menus.contains(name))).into()
        }
    }
}

/// List of simultaneous actions, each with its own editor and a remove button.
#[expect(clippy::needless_pass_by_value, reason = "editors take their callbacks by value")]
pub(super) fn multi_editor<'a>(
    list: &'a [ButtonAction],
    default_button: Button,
    on_change: OnAction<'a>,
    field: KeyField,
    names: &Names,
) -> Element<'a, Message> {
    let with = |f: &dyn Fn(&mut Vec<ButtonAction>)| {
        let mut v = list.to_vec();
        f(&mut v);
        on_change(ButtonAction::Multi(v))
    };
    let mut col = column![].spacing(6);
    for (i, sub) in list.iter().enumerate() {
        let parent = on_change.clone();
        let entry: OnAction<'a> = Rc::new(move |a| {
            let mut v = list.to_vec();
            v[i] = a;
            parent(ButtonAction::Multi(v))
        });
        col = col.push(
            row![
                action_editor(sub, default_button, MULTI_ENTRY_KINDS, entry, field.child(i), names),
                button(text("✕").size(13)).style(style::secondary).on_press(with(&|v| {
                    v.remove(i);
                })),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    let col = col.push(
        button(text("+ Add output").size(13))
            .style(style::secondary)
            .on_press(with(&|v| v.push(ButtonAction::Keys(Vec::new())))),
    );
    container(col).padding(8).width(Length::Fill).style(style::output_list).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summaries_read_like_the_mapping() {
        let keys = |k: &[&str]| ButtonAction::Keys(k.iter().map(std::string::ToString::to_string).collect());
        assert_eq!(summarize(&keys(&["KEY_LEFTCTRL", "KEY_C"])), "Left Ctrl + C");
        assert_eq!(summarize(&ButtonAction::Disabled), "Disabled");
        assert_eq!(summarize(&keys(&["KEY_SEMICOLON"])), "“;”");
        assert_eq!(
            summarize(&ButtonAction::toggle(ButtonAction::Turbo {
                action: Box::new(ButtonAction::Mouse(MouseButton::Left)),
                rate: 12.0,
                every_ms: 0,
            })),
            "Toggle Turbo Left click (12/s)"
        );
        assert_eq!(summarize(&ButtonAction::Macro { name: "QCF".into(), repeat: true }), "Macro “QCF” (repeat)");
        assert_eq!(summarize(&ButtonAction::PadStick { stick: Stick::Right, angle: 90.0, amount: 0.5 }), "Right Stick right 50%");
        assert_eq!(summarize(&ButtonAction::PadTrigger { trigger: Trigger::Left, amount: 1.0 }), "Left Trigger");
    }
}
