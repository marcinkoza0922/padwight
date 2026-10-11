//! The profile's Touchpad tab: what fingers on the pad do (mouse, a stick, a menu laid out on
//! the pad, or swipes), on the whole pad or on each half of a split one, what the click does,
//! and a picture of the pad with the fingers on it.

use iced::widget::{Column, column};

use super::*;
use crate::{
    config::{CurvePreset, SpeedCurve, SwipeDirection, TouchChoose, TouchRegion, TouchSide, TouchSlot, TouchpadConfig, TouchpadMotion},
    touchpad_widget::{PadPicture, touchpad_picture},
};

/// A finger's speed in pad widths/second is read by acceleration curves made for degrees/second
/// of gyro turning at this scale.
const CURVE_SCALE: f32 = 0.025;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TouchModeKind {
    Off,
    Mouse,
    Joystick,
    Menu,
    Swipe,
}

impl TouchModeKind {
    const ALL: [TouchModeKind; 5] = [TouchModeKind::Mouse, TouchModeKind::Joystick, TouchModeKind::Menu, TouchModeKind::Swipe, TouchModeKind::Off];

    fn of(motion: &TouchpadMotion) -> Self {
        match motion {
            TouchpadMotion::Off => TouchModeKind::Off,
            TouchpadMotion::Mouse { .. } => TouchModeKind::Mouse,
            TouchpadMotion::Joystick { .. } => TouchModeKind::Joystick,
            TouchpadMotion::Menu { .. } => TouchModeKind::Menu,
            TouchpadMotion::Swipe { .. } => TouchModeKind::Swipe,
        }
    }

    /// The mode with its starting settings; a menu starts on the first menu there is.
    fn motion(self, menus: &[String]) -> TouchpadMotion {
        match self {
            TouchModeKind::Off => TouchpadMotion::Off,
            TouchModeKind::Mouse => TouchpadMotion::mouse(crate::config::DEFAULT_TOUCHPAD_SPEED),
            TouchModeKind::Joystick => TouchpadMotion::Joystick { stick: Stick::Right, relative: true, radius: 0.2, deadzone: 0.1 },
            TouchModeKind::Menu => TouchpadMotion::Menu { menu: menus.first().cloned().unwrap_or_default(), choose: TouchChoose::Click, overlay: true },
            TouchModeKind::Swipe => TouchpadMotion::Swipe {
                up: ButtonAction::Disabled,
                down: ButtonAction::Disabled,
                left: ButtonAction::Disabled,
                right: ButtonAction::Disabled,
                distance: crate::config::DEFAULT_SWIPE_DISTANCE,
            },
        }
    }
}

impl fmt::Display for TouchModeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TouchModeKind::Off => "Nothing",
            TouchModeKind::Mouse => "Mouse pointer",
            TouchModeKind::Joystick => "Gamepad stick",
            TouchModeKind::Menu => "Menu zones",
            TouchModeKind::Swipe => "Swipes",
        })
    }
}

/// A short description of the touchpad's settings, for "Same as …" in layers.
pub(super) fn touchpad_summary(cfg: &TouchpadConfig) -> String {
    let one = |m: &TouchpadMotion| match m {
        TouchpadMotion::Off => "nothing".to_string(),
        TouchpadMotion::Mouse { .. } => "mouse".to_string(),
        TouchpadMotion::Joystick { stick, .. } => stick.to_string().to_lowercase(),
        TouchpadMotion::Menu { menu, .. } => format!("menu “{menu}”"),
        TouchpadMotion::Swipe { .. } => "swipes".to_string(),
    };
    match &cfg.right {
        Some(r) => format!("split: {} | {}", one(&cfg.motion), one(&r.motion)),
        None => one(&cfg.motion),
    }
}

/// What's wrong with the touchpad settings: a menu mode without a menu that exists.
pub(super) fn touchpad_problem(cfg: &TouchpadConfig, names: &Names) -> Option<String> {
    let missing = cfg.menus().find(|m| !names.menus.contains(m))?;
    Some(if missing.is_empty() { "pick a menu to lay out on the pad".into() } else { format!("there is no menu “{missing}”") })
}

/// The Touchpad tab's sections.
pub(super) fn touchpad_sections<'a>(p: &'a Profile, ui: &Ui) -> Vec<Element<'a, Message>> {
    let base = ui.layer.map_or(&p.touchpad, |m| &m.base.touchpad);
    let help = "DualShock 4 and DualSense controllers have a touchpad. It can move the mouse pointer, push a gamepad stick, \
                be divided into the items of one of your menus, or take swipes. Split it to give each half its own job.";
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    // Without a touchpad controller this is a status worth seeing, not just help.
    if !ui.any_touchpad {
        rows.push(
            text("None of your managed controllers have a touchpad. These settings take effect once one is connected.")
                .size(13)
                .color(MUTED_COLOR)
                .into(),
        );
    }
    rows.extend(layer_part(ui, LayerPart::Touchpad, "Touchpad".into(), touchpad_summary(base), || touchpad_rows(&p.touchpad, ui.names)));
    let mut sections = vec![section("Touchpad", Some(help.into()), rows)];
    let picture = PadPicture {
        settings: Some((&p.touchpad, &ui.menus)),
        touches: ui.live_touch.as_ref().map_or(&[], |(t, _)| t.as_slice()),
        click: ui.live_touch.as_ref().is_some_and(|(_, c)| *c),
        height: ui.touchpad_height,
        width: 420.0,
    };
    let help = "Where each side's job is on the pad. Fingers on the controller's touchpad show up here.";
    sections.push(section("Picture", Some(help.into()), vec![container(touchpad_picture(&picture)).center_x(Length::Fill).into()]));
    if ui.layer.is_none() {
        sections.push(section(
            "Controller requirements",
            Some("What the profile's author says it can't be played without. Set by the author, not worked out from the settings.".into()),
            requirement_rows(p, Feature::Touchpad),
        ));
    }
    sections
}

/// The settings: whether the pad is split, and each side's mode and click.
pub(super) fn touchpad_rows<'a>(cfg: &'a TouchpadConfig, names: &Names) -> Vec<Element<'a, Message>> {
    let with = move |f: &dyn Fn(&mut TouchpadConfig)| {
        let mut c = cfg.clone();
        f(&mut c);
        Message::SetTouchpad(c)
    };
    let mut rows: Vec<Element<'a, Message>> = vec![
        checkbox(cfg.right.is_some())
            .label("Split into a left and a right half, each with its own job")
            .on_toggle(move |on| {
                with(&|c| {
                    c.right = on.then(|| TouchRegion { motion: c.motion.clone(), click: c.click.clone() });
                })
            })
            .into(),
    ];
    if let Some(problem) = touchpad_problem(cfg, names) {
        rows.push(text(format!("⚠ {problem}")).size(12).color(ERROR_COLOR).into());
    }
    for &side in cfg.sides() {
        let mut col = Column::new().spacing(10);
        if cfg.right.is_some() {
            col = col.push(text(if side == TouchSide::Left { "Left half" } else { "Right half" }).size(15).color(style_accent()));
        }
        col = side_rows(col, cfg, side, names, with);
        rows.push(container(col).padding(10).style(style::inset).width(Length::Fill).into());
    }
    rows
}

/// One side's mode, its settings and its click.
fn side_rows<'a>(rows: Column<'a, Message>, cfg: &'a TouchpadConfig, side: TouchSide, names: &Names, with: impl Fn(&dyn Fn(&mut TouchpadConfig)) -> Message + Copy + 'a) -> Column<'a, Message> {
    let menus = names.menus.clone();
    let rows = rows.push(labeled(
        "Fingers",
        dropdown(TouchModeKind::ALL, Some(TouchModeKind::of(cfg.motion(side))), move |k| with(&|c| *c.motion_mut(side) = k.motion(&menus))).width(SETTING_WIDTH).into(),
    ));
    let set = move |m: TouchpadMotion| with(&|c| *c.motion_mut(side) = m.clone());
    let rows = match cfg.motion(side) {
        TouchpadMotion::Off => rows,
        TouchpadMotion::Mouse { speed, curve, tap, scroll, inertia } => mouse_rows(rows, (*speed, *curve, *tap, *scroll, *inertia), set),
        TouchpadMotion::Joystick { stick, relative, radius, deadzone } => joystick_rows(rows, (*stick, *relative, *radius, *deadzone), set),
        TouchpadMotion::Menu { menu, choose, overlay } => menu_rows(rows, (menu, *choose, *overlay), names, set),
        TouchpadMotion::Swipe { distance, .. } => {
            let mut rows = rows.push(value_slider("    Swipe length", 0.1..=0.6, *distance, 0.01, "pad widths", move |v| {
                with(&|c| {
                    if let TouchpadMotion::Swipe { distance, .. } = c.motion_mut(side) {
                        *distance = v;
                    }
                })
            }));
            for dir in SwipeDirection::ALL {
                rows = rows.push(slot_editor(cfg, TouchSlot::Swipe(side, dir), names));
            }
            rows
        }
    };
    let click_help = match cfg.motion(side) {
        TouchpadMotion::Menu { .. } => "    Click (off the items)",
        _ => "    Click",
    };
    rows.push(labeled(click_help, slot_action(cfg, TouchSlot::Click(side), names)))
}

/// The pointer's speed and acceleration, and tapping, scrolling and gliding.
fn mouse_rows<'a>(rows: Column<'a, Message>, (speed, curve, tap, scroll, inertia): (f32, SpeedCurve, bool, bool, bool), set: impl Fn(TouchpadMotion) -> Message + Copy + 'a) -> Column<'a, Message> {
    let mouse = move |speed: f32, curve: SpeedCurve, tap: bool, scroll: bool, inertia: bool| set(TouchpadMotion::Mouse { speed, curve, tap, scroll, inertia });
    let preset = curve.preset(CURVE_SCALE);
    let picker = curve_preset_picker(preset, move |p: CurvePreset| {
        let picked = p.speed_curve(CURVE_SCALE).unwrap_or_else(|| curve.custom_from(CURVE_SCALE));
        mouse(speed, picked, tap, scroll, inertia)
    });
    let mut rows = rows
        .push(value_slider("    Speed", 200.0..=4000.0, speed, 50.0, "px across the pad", move |v| mouse(v, curve, tap, scroll, inertia)))
        .push(labeled("    Acceleration", picker));
    if !curve.is_flat() {
        rows = rows
            .push(value_slider("      Slow fingers", 0.1..=3.0, curve.slow_gain, 0.05, "×", move |v| mouse(speed, SpeedCurve { slow_gain: v, ..curve }, tap, scroll, inertia)))
            .push(value_slider("      Fast fingers", 0.1..=4.0, curve.fast_gain, 0.05, "×", move |v| mouse(speed, SpeedCurve { fast_gain: v, ..curve }, tap, scroll, inertia)))
            .push(value_slider("      Shape", 0.25..=4.0, curve.power, 0.05, "(1 = straight, more = slow for longer)", move |v| {
                mouse(speed, SpeedCurve { power: v, ..curve }, tap, scroll, inertia)
            }));
    }
    let checks = column![
        checkbox(tap).label("Tap to click (runs the click below)").on_toggle(move |v| mouse(speed, curve, v, scroll, inertia)),
        checkbox(scroll).label("Two fingers scroll").on_toggle(move |v| mouse(speed, curve, tap, v, inertia)),
        checkbox(inertia).label("Keep gliding after a flick").on_toggle(move |v| mouse(speed, curve, tap, scroll, v)),
    ]
    .spacing(6);
    rows.push(labeled("", checks.into()))
}

/// The stick the finger pushes and how.
fn joystick_rows<'a>(rows: Column<'a, Message>, (stick, relative, radius, deadzone): (Stick, bool, f32, f32), set: impl Fn(TouchpadMotion) -> Message + Copy + 'a) -> Column<'a, Message> {
    let joystick = move |stick: Stick, relative: bool, radius: f32, deadzone: f32| set(TouchpadMotion::Joystick { stick, relative, radius, deadzone });
    let mut rows = rows
        .push(labeled("    Output stick", dropdown([Stick::Left, Stick::Right], Some(stick), move |s| joystick(s, relative, radius, deadzone)).width(170).into()))
        .push(labeled(
            "",
            checkbox(relative)
                .label("Center where the finger lands (otherwise the middle of the pad is center)")
                .on_toggle(move |v| joystick(stick, v, radius, deadzone))
                .into(),
        ));
    if relative {
        rows = rows.push(value_slider("    Full push at", 0.05..=0.5, radius, 0.01, "pad widths from center", move |v| joystick(stick, relative, v, deadzone)));
    }
    rows.push(value_slider("    Deadzone", 0.0..=0.5, deadzone, 0.01, "of a full push", move |v| joystick(stick, relative, radius, v)))
}

/// The menu laid out on the pad, and how its items are chosen.
fn menu_rows<'a>(rows: Column<'a, Message>, (menu, choose, overlay): (&'a String, TouchChoose, bool), names: &Names, set: impl Fn(TouchpadMotion) -> Message + Copy + 'a) -> Column<'a, Message> {
    let entry = move |menu: String, choose: TouchChoose, overlay: bool| set(TouchpadMotion::Menu { menu, choose, overlay });
    let picked = names.menus.contains(menu).then(|| menu.clone());
    let menu_name = menu.clone();
    let rows = if names.menus.is_empty() {
        rows.push(labeled("    Menu", note("Make a menu on the Menus tab first, then pick it here.").into()))
    } else {
        rows.push(labeled(
            "    Menu",
            dropdown(names.menus.clone(), picked, move |m| entry(m, choose, overlay)).placeholder("Pick a menu…").width(SETTING_WIDTH).into(),
        ))
    };
    let name = menu_name.clone();
    rows.push(labeled("    Choose by", dropdown(TouchChoose::ALL, Some(choose), move |c| entry(name.clone(), c, overlay)).width(SETTING_WIDTH).into()))
        .push(labeled("", checkbox(overlay).label("Show the menu while a finger is on the pad").on_toggle(move |v| entry(menu_name.clone(), choose, v)).into()))
        .push(labeled(
            "",
            note(
                "The pad is divided the way the menu is drawn: a radial menu by direction from the middle, a grid into its cells, \
                 a list into rows, a carousel from left to right. A clicked item stays pressed until the click lets go. The game \
                 keeps getting input while the menu shows.",
            )
            .into(),
        ))
}

/// A labeled action editor for one of the touchpad's action slots.
fn slot_editor<'a>(cfg: &'a TouchpadConfig, slot: TouchSlot, names: &Names) -> Element<'a, Message> {
    let label = match slot {
        TouchSlot::Click(_) => "    Click".to_string(),
        TouchSlot::Swipe(_, dir) => format!("    {dir}"),
    };
    labeled(label, slot_action(cfg, slot, names))
}

fn slot_action<'a>(cfg: &'a TouchpadConfig, slot: TouchSlot, names: &Names) -> Element<'a, Message> {
    let Some(action) = cfg.slot(slot) else { return text("").into() };
    let target = Target::Touch(slot);
    fill_x(action_editor(action, Button::South, &ACTION_KINDS, set_action(target), KeyField::root(target), names))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_builds_on_a_split_pad_and_a_missing_menu_is_a_problem() {
        let names = Names { menus: vec!["Build".into()], ..Names::default() };
        for left in TouchModeKind::ALL {
            for right in TouchModeKind::ALL {
                let cfg = TouchpadConfig {
                    motion: left.motion(&names.menus),
                    click: ButtonAction::Disabled,
                    right: Some(TouchRegion { motion: right.motion(&[]), click: ButtonAction::Disabled }),
                };
                assert!(!touchpad_rows(&cfg, &names).is_empty());
                let problem = touchpad_problem(&cfg, &names);
                assert_eq!(problem.is_some(), right == TouchModeKind::Menu, "{left:?} | {right:?}");
            }
        }
        assert_eq!(touchpad_summary(&TouchpadConfig::default()), "mouse");
    }
}
