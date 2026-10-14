//! The macro pages of Edit Controls: the game's macros, one macro's name and steps, a step's
//! settings, and the steps that can be added (fighting-game motions among them).

use crate::{
    config::{ButtonAction, Config, ItemKind, Macro, MacroStep, Motion, Stick, StickPreset},
    menu::Tone,
};

use super::{
    action::{ActionAt, Slot},
    named,
    owner::game_mut,
    pages::{Page, stick_name},
    rows::{Act, Nav, Number, Row, cycle, set},
};

const TAP_MS: u64 = 50;
const WAIT_MS: u64 = 100;
const HOLD: Number = Number::new(10.0, (10.0, 1000.0), 0, " ms");
const WAIT: Number = Number::new(10.0, (10.0, 5000.0), 0, " ms");
const AXIS: Number = Number::new(0.05, (-1.0, 1.0), 2, "");

fn macro_mut(config: &mut Config, m: usize) -> Option<&mut Macro> {
    game_mut(config)?.macros.get_mut(m)
}

fn step_mut(config: &mut Config, m: usize, i: usize) -> Option<&mut MacroStep> {
    macro_mut(config, m)?.steps.get_mut(i)
}

/// The game's macros, and a row to add one.
pub(super) fn list_rows(config: &Config) -> Vec<Row> {
    let macros = &config.active_game().macros;
    let mut rows: Vec<Row> = macros.iter().enumerate().map(|(i, m)| Row::open(format!("{} ({} steps)", m.name, m.steps.len()), Page::Macro(i))).collect();
    let new = macros.len();
    rows.push(Row::add("Add macro", move |c| {
        let name = named::new_name(c, ItemKind::Macro, "Macro");
        let Some(game) = game_mut(c) else { return Act::stay() };
        game.macros.push(Macro { name, steps: Vec::new() });
        Act::changed().then(Nav::Open(Page::Macro(new)))
    }));
    rows
}

/// A macro's page: its name, its steps, adding one, and deleting the macro.
pub(super) fn macro_rows(config: &Config, m: usize) -> Vec<Row> {
    let Some(mac) = config.active_game().macros.get(m) else { return Vec::new() };
    let mut rows = vec![named::rename_row(config, ItemKind::Macro, m)];
    rows.extend(mac.steps.iter().enumerate().map(|(i, s)| Row::open(format!("{}. {}", i + 1, step_label(s)), Page::Step(m, i)).keyword(s.action().and_then(ButtonAction::keyword))));
    rows.push(Row::open("Add a step…", Page::AddStep(m)).tone(Tone::Add));
    rows.push(named::delete_row(config, ItemKind::Macro, m, move |c| {
        let Some(game) = game_mut(c).filter(|g| m < g.macros.len()) else { return false };
        game.macros.remove(m);
        true
    }));
    rows
}

pub(super) fn step_label(step: &MacroStep) -> String {
    match step {
        MacroStep::Tap { action, hold_ms } => format!("Tap {} ({hold_ms} ms)", action.summary()),
        MacroStep::Press(action) => format!("Press {}", action.summary()),
        MacroStep::Release(action) => format!("Release {}", action.summary()),
        MacroStep::Wait(ms) => format!("Wait {ms} ms"),
        MacroStep::Stick { stick, x, y } => format!("{} to {}", stick_name(*stick), direction(*x, *y)),
    }
}

/// Where a stick step points, by its preset's arrow and name, or its numbers.
fn direction(x: f32, y: f32) -> String {
    match StickPreset::of(x, y) {
        StickPreset::Custom => format!("({x:.2}, {:.2})", -y),
        preset => preset.to_string(),
    }
}

/// What a new step can be, each added at the end and opened. A motion adds its stick steps.
pub(super) fn add_rows(m: usize) -> Vec<Row> {
    let add = |label: &str, step: fn() -> MacroStep, opens_action: bool| {
        Row::add(label, move |c| {
            let Some(mac) = macro_mut(c, m) else { return Act::stay() };
            mac.steps.push(step());
            let i = mac.steps.len() - 1;
            let page = if opens_action { Page::Action(ActionAt::new(Slot::MacroStep(m, i))) } else { Page::Step(m, i) };
            Act::changed().then(Nav::BackOpen(1, page))
        })
    };
    let mut rows = vec![
        add("Tap…", || MacroStep::Tap { action: ButtonAction::Keys(Vec::new()), hold_ms: TAP_MS }, true),
        add("Hold down…", || MacroStep::Press(ButtonAction::Keys(Vec::new())), true),
        add("Let go of…", || MacroStep::Release(ButtonAction::Keys(Vec::new())), true),
        add("Wait", || MacroStep::Wait(WAIT_MS), false),
        add("Move a stick", || MacroStep::Stick { stick: Stick::Left, x: 1.0, y: 0.0 }, false),
    ];
    rows.extend(Motion::ALL.into_iter().map(|motion| {
        Row::add(motion.to_string(), move |c| {
            let Some(mac) = macro_mut(c, m) else { return Act::stay() };
            mac.steps.extend(motion.steps());
            Act::changed().then(Nav::Back(1))
        })
    }));
    rows
}

/// The kinds a step can be switched between, keeping its action where it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tap,
    Press,
    Release,
}

/// A step's page: its action and timing, or where it moves a stick; moving it; deleting it.
pub(super) fn step_rows(config: &Config, m: usize, i: usize) -> Vec<Row> {
    let Some(step) = config.active_game().macros.get(m).and_then(|x| x.steps.get(i)) else { return Vec::new() };
    let len = config.active_game().macros[m].steps.len();
    let mut rows = match step {
        MacroStep::Tap { action, hold_ms } => vec![
            kind_row(m, i, Kind::Tap, action),
            action_row(m, i, action),
            HOLD.row_u("Held for", *hold_ms, move |c, v| match step_mut(c, m, i) {
                Some(MacroStep::Tap { hold_ms, .. }) => set(hold_ms, v),
                _ => false,
            }),
        ],
        MacroStep::Press(action) => vec![kind_row(m, i, Kind::Press, action), action_row(m, i, action)],
        MacroStep::Release(action) => vec![kind_row(m, i, Kind::Release, action), action_row(m, i, action)],
        MacroStep::Wait(ms) => vec![WAIT.row_u("Wait", *ms, move |c, v| step_mut(c, m, i).is_some_and(|s| set(s, MacroStep::Wait(v))))],
        MacroStep::Stick { stick, x, y } => stick_rows(m, i, *stick, *x, *y),
    };
    for (label, to) in [("Move up", i.checked_sub(1)), ("Move down", (i + 1 < len).then_some(i + 1))] {
        let Some(to) = to else { continue };
        rows.push(Row::apply(label, move |c| {
            let Some(mac) = macro_mut(c, m) else { return Act::stay() };
            mac.steps.swap(i, to);
            Act::changed().then(Nav::Follow(Page::Step(m, to), to + 1))
        }));
    }
    rows.push(Row::delete("Delete step", move |c| {
        let Some(mac) = macro_mut(c, m).filter(|x| i < x.steps.len()) else { return false };
        mac.steps.remove(i);
        true
    }));
    rows
}

fn kind_row(m: usize, i: usize, kind: Kind, action: &ButtonAction) -> Row {
    let name = match kind {
        Kind::Tap => "Tap",
        Kind::Press => "Hold down",
        Kind::Release => "Let go",
    };
    let action = action.clone();
    Row::step(format!("Kind: {name}"), move |c, dir| {
        let next = match cycle(&[Kind::Tap, Kind::Press, Kind::Release], kind, dir) {
            Kind::Tap => MacroStep::Tap { action: action.clone(), hold_ms: TAP_MS },
            Kind::Press => MacroStep::Press(action.clone()),
            Kind::Release => MacroStep::Release(action.clone()),
        };
        step_mut(c, m, i).is_some_and(|s| set(s, next))
    })
}

fn action_row(m: usize, i: usize, action: &ButtonAction) -> Row {
    Row::open(format!("Presses: {}", action.summary()), Page::Action(ActionAt::new(Slot::MacroStep(m, i)))).keyword(action.keyword())
}

/// A stick step: which stick, a direction from the presets, and the exact position.
fn stick_rows(m: usize, i: usize, stick: Stick, x: f32, y: f32) -> Vec<Row> {
    let preset = StickPreset::of(x, y);
    let presets: Vec<StickPreset> = StickPreset::ALL.into_iter().filter(|p| *p != StickPreset::Custom).collect();
    let set_step = move |c: &mut Config, stick: Stick, x: f32, y: f32| step_mut(c, m, i).is_some_and(|s| set(s, MacroStep::Stick { stick, x, y }));
    vec![
        Row::step(format!("Stick: {}", stick_name(stick)), move |c, _| {
            let other = if stick == Stick::Left { Stick::Right } else { Stick::Left };
            set_step(c, other, x, y)
        }),
        Row::step(format!("Direction: {}", direction(x, y)), move |c, dir| {
            let next = if preset == StickPreset::Custom { StickPreset::Center } else { cycle(&presets, preset, dir) };
            let (x, y) = next.position().unwrap_or_default();
            set_step(c, stick, x, y)
        }),
        AXIS.row("Across", x, move |c, v| set_step(c, stick, v, y)),
        // Shown with up as positive, as the settings window does.
        AXIS.row("Up", -y, move |c, v| set_step(c, stick, x, -v)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_motion_adds_its_stick_steps_and_a_step_moves_and_goes() {
        let mut config = Config::default();
        list_rows(&config).pop().map(|row| match row.choose {
            super::super::rows::Choose::Apply(apply) => apply(&mut config),
            _ => panic!("not an add row"),
        });
        let motion_row = add_rows(0).into_iter().find(|r| r.item.label.starts_with("Quarter circle forward")).unwrap();
        let super::super::rows::Choose::Apply(apply) = motion_row.choose else { panic!("not an add row") };
        apply(&mut config);
        let steps = &config.active_game().macros[0].steps;
        assert_eq!(steps.len(), Motion::QuarterCircleForward.steps().len());
        assert_eq!(step_label(&steps[0]), "Left stick to ↓ Down");
    }
}
