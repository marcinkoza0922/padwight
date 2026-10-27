//! Screen damage: a frame repaints only what changed, the rest of a reused buffer untouched, and
//! comes out the same as the screen drawn afresh.

use tiny_skia::Pixmap;

use super::{items::straight_patch, *};
use crate::{
    debug::scenes,
    info::Segment,
    overlay::render::Layer,
};

const DISPLAY: Point = Point::new(1280.0, 720.0);
const W: u32 = DISPLAY.x as u32;
const H: u32 = DISPLAY.y as u32;

fn blank() -> Pixmap {
    Pixmap::new(W, H).unwrap()
}

/// `ops` drawn afresh onto a clear screen.
fn afresh(ops: &[Op]) -> Pixmap {
    let mut p = blank();
    Screen::new().lay(ops, 1.0, &mut p.as_mut());
    p
}

fn worst(a: &Pixmap, b: &Pixmap) -> u8 {
    a.data().iter().zip(b.data()).map(|(a, b)| a.abs_diff(*b)).max().unwrap()
}

fn settled(frame: &OverlayFrame) -> Vec<Op> {
    screen_ops(frame, &Shown::settled(frame), DISPLAY)
}

/// `ops` with every panel (each layer at the top level) changed by `change`.
fn moved(ops: &[Op], change: impl Fn(&mut Layer)) -> Vec<Op> {
    let mut depth = 0;
    let mut ops = ops.to_vec();
    for op in &mut ops {
        match op {
            Op::Layer(layer) if depth == 0 => {
                change(layer);
                depth += 1;
            }
            Op::Layer(_) | Op::Clip { .. } => depth += 1,
            Op::Unlayer | Op::Unclip => depth -= 1,
            _ => {}
        }
    }
    ops
}

/// A menu in the middle of the screen with the cursor on item `selected`, and an info panel in
/// the top left corner whose first cell reads `cell`.
fn menu_and_info(selected: usize, cell: &str) -> OverlayFrame {
    let mut frame = scenes::frame("menu-list").unwrap();
    let mut info = scenes::frame("info").unwrap().info.remove(0);
    info.style.position = crate::config::ScreenPosition::TopLeft;
    info.rows[0][0] = vec![Segment::Text(cell.into())];
    frame.info = vec![info];
    if let Some(OverlayView::Menu(m)) = &mut frame.active {
        m.selected = Some(selected % m.items.len());
    }
    frame
}

/// The window's three buffers, each with the frame it last held.
struct Window {
    buffers: Vec<(Pixmap, Option<u64>)>,
    frames: u64,
    /// Whether a buffer never drawn into is known to be clear, as a new pool's are: it is then
    /// as old as the frame before the first (`Screen::forget_onto_clear`), else unknown.
    clear: bool,
}

impl Window {
    fn new() -> Window {
        Window { buffers: (0..3).map(|_| (blank(), None)).collect(), frames: 0, clear: false }
    }

    /// Paints `ops` into buffer `slot` as the window would, checking that nothing outside the
    /// damage was written, and returns the damage and the buffer.
    fn paint(&mut self, screen: &mut Screen, ops: &[Op], slot: usize) -> (Damage, &Pixmap) {
        let (buffer, held) = &mut self.buffers[slot];
        let age = match *held {
            Some(f) => self.frames + 1 - f,
            None if self.clear => self.frames + 1,
            None => 0,
        };
        let before = buffer.clone();
        let damage = screen.paint(ops, 1.0, &mut buffer.as_mut(), age);
        for (i, (now, was)) in buffer.pixels().iter().zip(before.pixels()).enumerate() {
            let (x, y) = ((i as u32 % W) as i32, (i as u32 / W) as i32);
            assert!(damage.contains(x, y) || now == was, "({x}, {y}) changed outside the damage");
        }
        if !damage.is_empty() {
            self.frames += 1;
            *held = Some(self.frames);
        }
        (damage, buffer)
    }
}

#[test]
fn moving_the_cursor_in_a_corner_info_panel_touches_only_that_corner() {
    let info_only = |cell: &str| {
        let mut frame = menu_and_info(0, cell);
        frame.active = None;
        straight_patch(&settled(&frame), 1.0).unwrap()
    };
    let corner = {
        let (a, b) = (info_only("cursor 120, 340"), info_only("cursor 121, 341"));
        Patch::new(a.x0.min(b.x0), a.y0.min(b.y0), a.x1.max(b.x1), a.y1.max(b.y1))
    };
    assert!(corner.x1 < W as i32 / 3 && corner.y1 < H as i32 / 3, "the info panel is in the top left corner: {corner:?}");
    let mut screen = Screen::new();
    let mut buffer = blank();
    let first = settled(&menu_and_info(0, "cursor 120, 340"));
    assert_eq!(screen.paint(&first, 1.0, &mut buffer.as_mut(), 0).patches(), &[Patch::new(0, 0, W as i32, H as i32)]);
    let before = buffer.clone();
    let next = settled(&menu_and_info(0, "cursor 121, 341"));
    let damage = screen.paint(&next, 1.0, &mut buffer.as_mut(), 1);
    assert!(!damage.is_empty());
    for p in damage.patches() {
        assert!(corner.holds(p), "{p:?} reaches outside the info panel {corner:?}");
    }
    for (i, (now, was)) in buffer.pixels().iter().zip(before.pixels()).enumerate() {
        let (x, y) = ((i as u32 % W) as i32, (i as u32 / W) as i32);
        if !corner.contains(x, y) {
            assert_eq!(now, was, "({x}, {y}) outside the corner changed");
        }
    }
    assert_eq!(worst(&buffer, &afresh(&next)), 0);
}

#[test]
fn an_unchanged_frame_repaints_nothing() {
    let ops = settled(&menu_and_info(1, "x"));
    let mut screen = Screen::new();
    let mut buffer = blank();
    screen.paint(&ops, 1.0, &mut buffer.as_mut(), 0);
    assert!(screen.paint(&ops, 1.0, &mut buffer.as_mut(), 1).is_empty());
}

#[test]
fn an_older_buffer_takes_the_damage_of_the_frames_since() {
    let frames = [settled(&menu_and_info(0, "a")), settled(&menu_and_info(1, "a")), settled(&menu_and_info(1, "b"))];
    let (mut screen, mut window) = (Screen::new(), Window::new());
    window.paint(&mut screen, &frames[0], 0);
    let (second, _) = window.paint(&mut screen, &frames[1], 1);
    let second_own = {
        // What the second frame changed: the menu's cursor, which a buffer one frame behind skips.
        let mut s = Screen::new();
        let mut p = blank();
        s.paint(&frames[0], 1.0, &mut p.as_mut(), 0);
        s.paint(&frames[1], 1.0, &mut p.as_mut(), 1)
    };
    assert_eq!(second.patches().len(), 1, "the second buffer was new: everything");
    let (third, buffer) = window.paint(&mut screen, &frames[2], 0);
    assert_eq!(worst(buffer, &afresh(&frames[2])), 0, "age 2 brings the buffer up to date");
    for p in second_own.patches() {
        assert!(third.patches().iter().any(|t| t.holds(p)), "{p:?}, changed a frame back, is repainted: {third:?}");
    }
    assert!(third.patches().iter().any(|t| t.x1 < W as i32 / 3), "and so is this frame's change in the corner");
}

#[test]
fn an_unknown_buffer_is_repainted_whole() {
    let ops = settled(&menu_and_info(2, "x"));
    let mut screen = Screen::new();
    let mut buffer = blank();
    screen.paint(&ops, 1.0, &mut buffer.as_mut(), 0);
    let mut garbage = blank();
    garbage.data_mut().fill(0x5a);
    let damage = screen.paint(&ops, 1.0, &mut garbage.as_mut(), 0);
    assert_eq!(damage.patches(), &[Patch::new(0, 0, W as i32, H as i32)]);
    assert_eq!(worst(&garbage, &afresh(&ops)), 0);
    // A new scale, size, or forgetting, repaints all too, whatever the buffer's age.
    let damage = screen.paint(&ops, 1.5, &mut garbage.as_mut(), 1);
    assert_eq!(damage.patches().len(), 1);
    screen.forget();
    assert_eq!(screen.paint(&ops, 1.0, &mut garbage.as_mut(), 1).patches(), &[Patch::new(0, 0, W as i32, H as i32)]);
}

#[test]
fn a_fading_panel_damages_its_rect_and_leaves_it_clear() {
    let ops = settled(&scenes::frame("menu-list").unwrap());
    let rect = straight_patch(&ops, 1.0).unwrap();
    let (mut screen, mut buffer) = (Screen::new(), blank());
    screen.paint(&ops, 1.0, &mut buffer.as_mut(), 0);
    for opacity in [0.8, 0.5, 0.2] {
        let faded = moved(&ops, |l| l.opacity = opacity);
        let damage = screen.paint(&faded, 1.0, &mut buffer.as_mut(), 1);
        assert!(damage.patches().iter().any(|p| p.holds(&rect.intersection(&Patch::new(0, 0, W as i32, H as i32)).unwrap())), "{damage:?}");
        assert_eq!(worst(&buffer, &afresh(&faded)), 0);
    }
    let damage = screen.paint(&[], 1.0, &mut buffer.as_mut(), 1);
    assert!(!damage.is_empty());
    assert!(buffer.data().iter().all(|b| *b == 0), "once it's gone its area is clear");
}

/// The menu alone with the cursor on item `selected`, at `opacity`.
fn menu_at(selected: usize, opacity: f32) -> Vec<Op> {
    let mut frame = menu_and_info(selected, "");
    frame.info.clear();
    moved(&settled(&frame), |l| l.opacity = opacity)
}

#[test]
fn overlapping_panels_are_composited_again_where_one_changes() {
    // The info panel slid under the menu, seen through it; then the other way round, the menu
    // under a see-through info panel. Only the menu changes.
    let mut frame = scenes::frame("info").unwrap();
    frame.info[0].style.position = crate::config::ScreenPosition::Center;
    let info = moved(&settled(&frame), |l| (l.shift, l.opacity) = (Point::new(60.0, 40.0), 0.9));
    assert!(straight_patch(&info, 1.0).unwrap().intersection(&straight_patch(&menu_at(0, 1.0), 1.0).unwrap()).is_some(), "they overlap");
    let mut screen = Screen::new();
    let mut window = Window::new();
    for (n, i) in [0, 1, 2, 3, 2].into_iter().enumerate() {
        let under: Vec<Op> = info.iter().cloned().chain(menu_at(i, 0.8)).collect();
        let (_, buffer) = window.paint(&mut screen, &under, n % 3);
        assert_eq!(worst(buffer, &afresh(&under)), 0, "menu over info, frame {n}");
    }
    for (n, i) in [0, 1, 2, 3].into_iter().enumerate() {
        let over: Vec<Op> = menu_at(i, 1.0).into_iter().chain(info.iter().cloned()).collect();
        let (_, buffer) = window.paint(&mut screen, &over, n % 2);
        assert_eq!(worst(buffer, &afresh(&over)), 0, "info over menu, frame {n}");
    }
}

/// A radial menu's highlight gliding once round it and on past the first item, a new item every
/// other frame, so arcs are part lit and the first change moves back to the start.
fn radial_sweep() -> Vec<Vec<Op>> {
    use crate::motion::{Feel, Timing};
    let mut frame = scenes::frame("menu-radial").unwrap();
    let n = match &frame.active {
        Some(OverlayView::Menu(m)) => m.items.len(),
        _ => unreachable!(),
    };
    let feel = Feel { glide: 0.12, ..Feel::OFF };
    (0..2 * n + 4)
        .map(|f| {
            let (i, step) = (f / 2 % n, f % 2);
            if let Some(OverlayView::Menu(m)) = &mut frame.active {
                m.selected = Some(i);
                m.aim = Some(f as f32 * 11.0);
            }
            let timing = Timing { opened: 10.0, moved: step as f32 * 0.03, from: Some(((i + n - 1) % n) as u32), ..Timing::default() };
            let anim = Anim { feel, timing, clock: 0.0 };
            let shown = Shown { panels: frame.active.iter().map(|v| (v, anim)).collect(), info: Vec::new(), logs: Vec::new() };
            screen_ops(&frame, &shown, DISPLAY)
        })
        .collect()
}

#[test]
fn a_damaged_sequence_equals_drawing_afresh() {
    let mut scenes_frames: Vec<(&str, Vec<Vec<Op>>)> = Vec::new();
    scenes_frames.push(("cursor", (0..6).map(|i| settled(&menu_and_info(i, &format!("cursor {}", i / 2)))).collect()));
    let menu = settled(&scenes::frame("menu-list").unwrap());
    let mut fade: Vec<_> = [0.2, 0.5, 0.8, 1.0, 1.0, 0.6, 0.3].iter().map(|&o| moved(&menu, |l| l.opacity = o)).collect();
    fade.push(Vec::new());
    scenes_frames.push(("fade", fade));
    let guide = settled(&scenes::frame("guide").unwrap());
    let mut slide: Vec<_> = (0..4).map(|t| moved(&guide, |l| l.shift = Point::new(t as f32 * 13.3, -(t as f32) * 7.1))).collect();
    slide.extend((0..3).map(|t| moved(&guide, |l| (l.scale, l.about) = (0.8 + t as f32 * 0.07, Point::new(600.0, 300.0)))));
    scenes_frames.push(("slide-zoom", slide));
    let busy = scenes::frame("busy").unwrap();
    let opening = [0.1, 0.4, 0.7, 1.0].iter().map(|&o| {
        let anim = Anim::faint(o);
        let shown = Shown {
            panels: busy.active.iter().map(|v| (v, anim)).collect(),
            info: busy.info.iter().map(|v| (v, anim)).collect(),
            logs: busy.logs.iter().map(|v| (v, anim)).collect(),
        };
        screen_ops(&busy, &shown, DISPLAY)
    });
    scenes_frames.push(("busy", opening.collect()));
    scenes_frames.push(("radial-sweep", radial_sweep()));
    // Buffers of every age: 0 (new), 1, 2 and 3.
    let slots = [0, 1, 0, 2, 1, 1, 0, 2, 2, 0];
    for (name, frames) in scenes_frames {
        let (mut screen, mut window) = (Screen::new(), Window::new());
        let mut repainted = 0;
        for (n, ops) in frames.iter().enumerate() {
            let (damage, buffer) = window.paint(&mut screen, ops, slots[n % slots.len()]);
            let w = worst(buffer, &afresh(ops));
            assert_eq!(w, 0, "{name} frame {n}: {w}");
            if n > 3 {
                repainted += damage.patches().iter().map(|p| p.width() * p.height()).sum::<i32>();
            }
        }
        if name == "cursor" {
            // Once all three buffers hold a frame, only the menu and the corner are repainted:
            // less than a screen over the two frames left.
            assert!(repainted < (W * H) as i32, "{name}: {repainted} pixels");
        }
    }
}

#[test]
fn new_clear_buffers_are_painted_only_where_the_screen_draws() {
    let menu = settled(&scenes::frame("menu-list").unwrap());
    let rect = straight_patch(&menu, 1.0).unwrap();
    let mut screen = Screen::new();
    screen.forget_onto_clear((W, H), 1.0);
    let mut window = Window { clear: true, ..Window::new() };
    // An opening into the pool's three buffers in turn, the first two new.
    for (n, (opacity, slot)) in [(0.1, 0), (0.3, 1), (0.6, 0), (1.0, 1), (1.0, 2)].into_iter().enumerate() {
        let ops = moved(&menu, |l| l.opacity = opacity);
        let (damage, buffer) = window.paint(&mut screen, &ops, slot);
        assert_eq!(worst(buffer, &afresh(&ops)), 0, "frame {n}");
        for p in damage.patches() {
            assert!(rect.holds(p), "frame {n}: {p:?} is outside the menu {rect:?}");
        }
    }
}
