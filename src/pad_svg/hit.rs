//! Which part of the drawing a point falls on, for the debug window's clickable controller.

use super::{
    BUMPER_ABOVE, BUMPER_HEIGHT, Spot, TRIGGER_HEIGHT, TRIGGER_REACH, face_positions,
    layout::{DpadKind, Dot, Layout},
};
use crate::{
    config::{Button, Stick, Trigger},
    info::PadModel,
};

/// A part of the controller that can be pressed or moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Part {
    Button(Button),
    /// A stick's well: dragging it moves the stick (see [`stick_value`]).
    Stick(Stick),
    /// A trigger, pulled to this much (0.0..1.0) at the point.
    Trigger(Trigger, f32),
}

/// The part of `model`'s drawing at (`x`, `y`), in drawing coordinates without the label margin.
pub fn hit(model: Option<PadModel>, x: f32, y: f32) -> Option<Part> {
    let l = Layout::of(model);
    shoulder(l, x, y)
        .or_else(|| stick_at(l, x, y))
        .or_else(|| face_positions(l).into_iter().find(|(_, c)| within(*c, l.face_r, x, y)).map(|(b, _)| Part::Button(b)))
        .or_else(|| dpad_at(l, x, y))
        .or_else(|| {
            [(Button::Select, l.select), (Button::Guide, l.guide), (Button::Start, l.start)]
                .into_iter()
                .find(|(_, dot)| within_dot(*dot, x, y))
                .map(|(b, _)| Part::Button(b))
        })
}

/// The box (left, top, width, height) `spot` covers on `model`'s drawing, in drawing
/// coordinates without the label margin, for pointing at it. A bumper is the part of it that
/// shows above the body. `None` for a button the drawing doesn't have (stick directions, paddles).
pub fn area(model: Option<PadModel>, spot: Spot) -> Option<(f32, f32, f32, f32)> {
    let l = Layout::of(model);
    let around = |(x, y): (f32, f32), r: f32| (x - r, y - r, 2.0 * r, 2.0 * r);
    let (lx, rx) = l.shoulders;
    let w = l.shoulder_w;
    let b = match spot {
        Spot::Trigger(t) => {
            let cx = if t == Trigger::Left { lx } else { rx };
            return Some((cx - w * TRIGGER_REACH, l.trigger_top(), 2.0 * w * TRIGGER_REACH, TRIGGER_HEIGHT));
        }
        Spot::Button(b) => b,
    };
    let arm = l.dpad_reach * 18.0 / 26.0;
    let (dx, dy) = l.dpad;
    let dot = |(x, y, r): Dot| around((x, y), r + 3.0);
    Some(match b {
        Button::LeftBumper | Button::RightBumper => {
            let cx = if b == Button::LeftBumper { lx } else { rx };
            (cx - w / 2.0, l.bumper_top(), w, BUMPER_ABOVE)
        }
        Button::LeftStick => around(l.left_stick, l.stick_r),
        Button::RightStick => around(l.right_stick, l.stick_r),
        Button::DpadUp => around((dx, dy - arm), l.dpad_reach * 0.35),
        Button::DpadDown => around((dx, dy + arm), l.dpad_reach * 0.35),
        Button::DpadLeft => around((dx - arm, dy), l.dpad_reach * 0.35),
        Button::DpadRight => around((dx + arm, dy), l.dpad_reach * 0.35),
        Button::Select => dot(l.select),
        Button::Start => dot(l.start),
        Button::Guide => dot(l.guide),
        _ => return face_positions(l).into_iter().find(|(f, _)| *f == b).map(|(_, c)| around(c, l.face_r)),
    })
}

/// Where a stick sits when the pointer is at (`x`, `y`): the cap follows the pointer, up to the
/// edge of its travel. Y is positive down, like the sticks' values.
pub fn stick_value(model: Option<PadModel>, stick: Stick, x: f32, y: f32) -> (f32, f32) {
    let l = Layout::of(model);
    let (cx, cy) = match stick {
        Stick::Left => l.left_stick,
        Stick::Right => l.right_stick,
    };
    let travel = l.stick_r * 13.0 / 27.0;
    let (dx, dy) = ((x - cx) / travel, (y - cy) / travel);
    let len = dx.hypot(dy);
    if len > 1.0 { (dx / len, dy / len) } else { (dx, dy) }
}

/// The center of a face button, for aiming at it in tests.
#[cfg(test)]
pub fn face_center(model: Option<PadModel>, button: Button) -> (f32, f32) {
    face_positions(Layout::of(model)).into_iter().find(|(b, _)| *b == button).map_or((0.0, 0.0), |(_, c)| c)
}

/// The center of a stick's well, for aiming at it in tests.
#[cfg(test)]
pub fn stick_center(model: Option<PadModel>, stick: Stick) -> (f32, f32) {
    let l = Layout::of(model);
    if stick == Stick::Left { l.left_stick } else { l.right_stick }
}

/// How far a trigger is pulled when the pointer is at height `y`: the fill rises from the
/// bottom of the trigger, so the pointer is where the top of it ends up.
pub fn trigger_value(model: Option<PadModel>, y: f32) -> f32 {
    trigger_value_at(Layout::of(model).trigger_top(), y)
}

/// The pull at height `y` for a trigger whose top edge is at `top`.
fn trigger_value_at(top: f32, y: f32) -> f32 {
    ((top + TRIGGER_HEIGHT - y) / TRIGGER_HEIGHT).clamp(0.0, 1.0)
}

fn within((cx, cy): (f32, f32), r: f32, x: f32, y: f32) -> bool {
    (x - cx).hypot(y - cy) <= r
}

fn within_dot((cx, cy, r): Dot, x: f32, y: f32) -> bool {
    // These are small; give a finger-sized margin.
    within((cx, cy), r + 3.0, x, y)
}

fn stick_at(l: &Layout, x: f32, y: f32) -> Option<Part> {
    [(Stick::Left, l.left_stick), (Stick::Right, l.right_stick)]
        .into_iter()
        .find(|(_, c)| within(*c, l.stick_r, x, y))
        .map(|(s, _)| Part::Stick(s))
}

/// Triggers (the outlines above the bumpers) and bumpers.
fn shoulder(l: &Layout, x: f32, y: f32) -> Option<Part> {
    let w = l.shoulder_w;
    let (trigger_top, bumper_top) = (l.trigger_top(), l.bumper_top());
    let sides = [(l.shoulders.0, Trigger::Left, Button::LeftBumper), (l.shoulders.1, Trigger::Right, Button::RightBumper)];
    sides.into_iter().find_map(|(cx, trigger, bumper)| {
        if (cx - w * TRIGGER_REACH..=cx + w * TRIGGER_REACH).contains(&x) && (trigger_top..=trigger_top + TRIGGER_HEIGHT).contains(&y) {
            return Some(Part::Trigger(trigger, trigger_value_at(trigger_top, y)));
        }
        let on_bumper = (cx - w / 2.0..=cx + w / 2.0).contains(&x) && (bumper_top..=bumper_top + BUMPER_HEIGHT).contains(&y);
        on_bumper.then_some(Part::Button(bumper))
    })
}

fn dpad_at(l: &Layout, x: f32, y: f32) -> Option<Part> {
    let (cx, cy) = l.dpad;
    let (dx, dy) = (x - cx, y - cy);
    let reach = l.dpad_reach;
    let arm = reach * 18.0 / 26.0;
    let direction = |dx: f32, dy: f32| match (dx.abs() >= dy.abs(), dx < 0.0, dy < 0.0) {
        (true, true, _) => Button::DpadLeft,
        (true, false, _) => Button::DpadRight,
        (false, _, true) => Button::DpadUp,
        (false, _, false) => Button::DpadDown,
    };
    match l.dpad_kind {
        DpadKind::Cross | DpadKind::Arrows if dx.abs() <= reach && dy.abs() <= reach => Some(Part::Button(direction(dx, dy))),
        DpadKind::Disc if dx.hypot(dy) <= reach => Some(Part::Button(direction(dx, dy))),
        DpadKind::Buttons => [Button::DpadUp, Button::DpadDown, Button::DpadLeft, Button::DpadRight]
            .into_iter()
            .zip([(0.0, -arm), (0.0, arm), (-arm, 0.0), (arm, 0.0)])
            .find(|(_, (ax, ay))| (dx - ax).hypot(dy - ay) <= reach * 0.35 + 2.0)
            .map(|(b, _)| Part::Button(b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn models() -> impl Iterator<Item = Option<PadModel>> {
        std::iter::once(None).chain(PadModel::ALL.map(Some))
    }

    #[test]
    fn sticks_faces_and_center_buttons_can_be_hit_at_their_centers() {
        for model in models() {
            let l = Layout::of(model);
            let at = |(x, y): (f32, f32)| hit(model, x, y);
            assert_eq!(at(l.left_stick), Some(Part::Stick(Stick::Left)), "{model:?}");
            assert_eq!(at(l.right_stick), Some(Part::Stick(Stick::Right)), "{model:?}");
            for (b, c) in face_positions(l) {
                assert_eq!(at(c), Some(Part::Button(b)), "{model:?}");
            }
            for (b, (x, y, _)) in [(Button::Select, l.select), (Button::Guide, l.guide), (Button::Start, l.start)] {
                assert_eq!(at((x, y)), Some(Part::Button(b)), "{model:?}");
            }
        }
    }

    #[test]
    fn dpad_arms_can_be_hit_on_every_model() {
        for model in models() {
            let l = Layout::of(model);
            let arm = l.dpad_reach * 18.0 / 26.0;
            let (dx, dy) = l.dpad;
            let arms = [
                (Button::DpadUp, (dx, dy - arm)),
                (Button::DpadDown, (dx, dy + arm)),
                (Button::DpadLeft, (dx - arm, dy)),
                (Button::DpadRight, (dx + arm, dy)),
            ];
            for (b, (x, y)) in arms {
                assert_eq!(hit(model, x, y), Some(Part::Button(b)), "{model:?}");
            }
        }
    }

    #[test]
    fn bumpers_and_triggers_can_be_hit_on_every_model() {
        for model in models() {
            let l = Layout::of(model);
            let (lx, rx) = l.shoulders;
            let (trigger_top, bumper_top) = (l.trigger_top(), l.bumper_top());
            let at = |x: f32, y: f32| hit(model, x, y);
            assert_eq!(at(lx, bumper_top + 3.0), Some(Part::Button(Button::LeftBumper)), "{model:?}");
            assert_eq!(at(rx, bumper_top + 3.0), Some(Part::Button(Button::RightBumper)), "{model:?}");
            assert!(matches!(at(lx, trigger_top + 5.0), Some(Part::Trigger(Trigger::Left, _))), "{model:?}");
            assert!(matches!(at(rx, trigger_top + 5.0), Some(Part::Trigger(Trigger::Right, _))), "{model:?}");
            // The bars meet: there is no dead gap between the trigger and its bumper.
            assert!(matches!(at(lx, trigger_top + TRIGGER_HEIGHT), Some(Part::Trigger(Trigger::Left, _))), "{model:?}");
        }
    }

    #[test]
    fn each_drawn_part_is_hit_inside_its_area() {
        let part = |p: Part| match p {
            Part::Button(b) => Some(b),
            Part::Stick(Stick::Left) => Some(Button::LeftStick),
            Part::Stick(Stick::Right) => Some(Button::RightStick),
            Part::Trigger(..) => None,
        };
        for model in models() {
            for b in Button::ALL {
                let (x, y, w, h) = area(model, Spot::Button(b)).unwrap_or_else(|| panic!("{b:?} on {model:?}"));
                assert_eq!(hit(model, x + w / 2.0, y + h / 2.0).and_then(part), Some(b), "{model:?}");
            }
        }
        assert_eq!(area(None, Spot::Button(Button::LeftPaddle)), None, "paddles aren't drawn");
    }

    #[test]
    fn empty_space_hits_nothing_and_triggers_follow_the_pointer() {
        assert_eq!(hit(None, -50.0, 300.0), None);
        let l = Layout::of(None);
        let top = l.trigger_top();
        let Some(Part::Trigger(_, pulled)) = hit(None, l.shoulders.0, top + 1.0) else { panic!("trigger") };
        assert!(pulled > 0.9);
        let Some(Part::Trigger(_, resting)) = hit(None, l.shoulders.0, top + TRIGGER_HEIGHT - 1.0) else { panic!("trigger") };
        assert!(resting < 0.1);
        assert_eq!(trigger_value(None, top), 1.0, "at the top it reads fully pulled");
        assert_eq!(trigger_value(None, top + TRIGGER_HEIGHT), 0.0, "at the bottom it reads released");
    }

    #[test]
    fn stick_follows_the_pointer_and_stops_at_the_edge() {
        let (cx, cy) = Layout::of(None).left_stick;
        assert_eq!(stick_value(None, Stick::Left, cx, cy), (0.0, 0.0));
        let (x, y) = stick_value(None, Stick::Left, cx + 500.0, cy);
        assert!((x - 1.0).abs() < 1e-4 && y.abs() < 1e-4);
        let (x, y) = stick_value(None, Stick::Left, cx, cy - 500.0);
        assert!(x.abs() < 1e-4 && (y + 1.0).abs() < 1e-4, "up is negative");
    }
}
