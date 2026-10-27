use super::*;
use crate::{
    config::ScreenPosition,
    debug::scenes,
    overlay::render::{balanced, bounds},
};

const DISPLAY: Point = Point::new(1920.0, 1080.0);

fn ops_of(scene: &str) -> Vec<Op> {
    let frame = scenes::frame(scene).unwrap();
    screen_ops(&frame, &Shown::settled(&frame), DISPLAY)
}

fn ink(ops: &[Op]) -> Rect {
    bounds(ops).expect("something is drawn")
}

#[test]
fn every_scene_draws_balanced_ops() {
    for (name, _) in scenes::SCENES {
        let Ok(frame) = scenes::frame(name) else { continue };
        let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
        assert!(balanced(&ops), "{name}");
    }
}

#[test]
fn a_centered_menu_is_centered_on_the_screen() {
    let r = ink(&ops_of("menu-list"));
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    assert!((cx - 960.0).abs() < 40.0 && (cy - 540.0).abs() < 40.0, "{r:?}");
}

#[test]
fn an_info_panel_keeps_its_margin_from_the_edge() {
    let frame = scenes::frame("info").unwrap();
    let spot = frame.info[0].style.position;
    let r = ink(&screen_ops(&frame, &Shown::settled(&frame), DISPLAY));
    if spot == ScreenPosition::TopLeft {
        assert!(r.x >= place::EDGE_MARGIN - 1.0 && r.y >= place::EDGE_MARGIN - 1.0, "{r:?}");
    }
    assert!(r.w < 1920.0 && r.h < 1080.0, "{r:?}");
}

#[test]
fn the_guide_stands_in_a_row_with_notes_and_mappings() {
    let frame = scenes::frame("guide").unwrap();
    let shown = Shown::settled(&frame);
    let cx = Context::of(&frame, DISPLAY);
    assert!(stacks::guide_up(&frame, &shown, &cx));
    let r = ink(&screen_ops(&frame, &shown, DISPLAY));
    assert!(r.x < 100.0 && r.right() > 1920.0 - 100.0, "Notes at the left edge and Mappings at the right: {r:?}");
}

#[test]
fn the_guide_row_is_off_without_its_notes() {
    let mut frame = scenes::frame("guide").unwrap();
    frame.info.clear();
    let cx = Context::of(&frame, DISPLAY);
    assert!(!stacks::guide_up(&frame, &Shown::settled(&frame), &cx));
}

#[test]
fn peek_draws_no_panels() {
    let mut frame = scenes::frame("menu-list").unwrap();
    frame.peek = true;
    assert!(bounds(&screen_ops(&frame, &Shown::settled(&frame), DISPLAY)).is_none());
}

#[test]
fn a_prompt_is_drawn_over_its_menu() {
    let frame = scenes::frame("unsaved").unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    assert!(balanced(&ops));
    let mut bare = frame.clone();
    if let Some(OverlayView::Menu(m)) = &mut bare.active {
        m.prompt = None;
    }
    let without = screen_ops(&bare, &Shown::settled(&bare), DISPLAY);
    assert!(ops.len() > without.len(), "the question adds to the menu");
}

#[test]
fn a_panel_fades_and_zooms_as_a_layer_about_its_center() {
    let frame = scenes::frame("menu-list").unwrap();
    let mut shown = Shown::settled(&frame);
    let mut anim = Anim::faint(0.5);
    anim.feel.zoom = 0.5;
    shown.panels[0].1 = anim;
    let ops = screen_ops(&frame, &shown, DISPLAY);
    let layers: Vec<_> = ops.iter().filter_map(|op| if let Op::Layer(l) = op { Some(*l) } else { None }).collect();
    assert_eq!(layers.len(), 1);
    assert!(matches!(ops.first(), Some(Op::Layer(_))) && matches!(ops.last(), Some(Op::Unlayer)), "the panel is the whole list");
    assert!(layers[0].scale < 1.0 && (layers[0].opacity - 0.5).abs() < 1e-4, "{:?}", layers[0]);
    assert!((layers[0].about.x - 960.0).abs() < 40.0);
}

#[test]
fn every_panel_is_a_layer_of_its_own() {
    let frame = scenes::frame("guide").unwrap();
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    // Notes, the menu and Mappings: three layers at the top level, side by side.
    let mut depth = 0;
    let mut top = Vec::new();
    for op in &ops {
        match op {
            Op::Layer(_) | Op::Clip { .. } if depth == 0 => {
                assert!(matches!(op, Op::Layer(_)), "only panels at the top level");
                top.push(op);
                depth += 1;
            }
            Op::Layer(_) | Op::Clip { .. } => depth += 1,
            Op::Unlayer | Op::Unclip => depth -= 1,
            _ => assert!(depth > 0, "nothing drawn outside a panel: {op:?}"),
        }
    }
    assert!(top.len() >= 3, "{}", top.len());
}

#[test]
fn the_screen_picture_is_the_display_at_scale() {
    let frame = scenes::frame("menu-list").unwrap();
    let image = screen(&frame, Point::new(400.0, 300.0), 2.0, Rgba::rgb(0.2, 0.3, 0.4)).unwrap();
    assert_eq!((image.width, image.height), (800, 600));
    assert_eq!(image.pixel(0, 0), [51, 77, 102, 255]);
}

/// Times a radial menu's highlight gliding round it, a new item every three frames, as the
/// harness's sweep does (run optimized: `--cargo-profile release`). With `BENCH_SLEEP` set it
/// waits a frame's time between frames, as the overlay does, which on a CPU that slows down
/// while idle is several times slower and closer to what the harness measures.
#[test]
#[ignore]
#[expect(clippy::print_stdout, reason = "a benchmark reports its timing")]
fn bench_a_radial_sweep() {
    use crate::motion::{Feel, Timing};
    let mut frame = scenes::frame("menu-radial").unwrap();
    let n = match &frame.active {
        Some(OverlayView::Menu(m)) => m.items.len(),
        _ => unreachable!(),
    };
    let feel = Feel { glide: 0.12, ..Feel::OFF };
    let mut screen = Screen::new();
    let mut buffer = tiny_skia::Pixmap::new(DISPLAY.x as u32, DISPLAY.y as u32).unwrap();
    let mut spent = std::time::Duration::ZERO;
    for f in 0..180usize {
        let (i, step) = (f / 3 % n, f % 3);
        if let Some(OverlayView::Menu(m)) = &mut frame.active {
            m.selected = Some(i);
            m.aim = Some(f as f32 * 7.0);
        }
        let timing = Timing { opened: 10.0, moved: step as f32 * 0.016, from: Some(((i + n - 1) % n) as u32), ..Timing::default() };
        let anim = Anim { feel, timing, clock: 0.0 };
        let shown = Shown { panels: frame.active.iter().map(|v| (v, anim)).collect(), info: Vec::new(), logs: Vec::new() };
        let ops = screen_ops(&frame, &shown, DISPLAY);
        if std::env::var_os("BENCH_SLEEP").is_some() {
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        let start = std::time::Instant::now();
        screen.paint(&ops, 1.0, &mut buffer.as_mut(), 1);
        if f >= 30 {
            spent += start.elapsed();
        }
    }
    let [diffed, tiled] = screen.spent();
    println!("{:.2} ms a frame; tiles {}, pictures {}; diff {diffed:?} draw {tiled:?} in all", spent.as_secs_f64() * 1000.0 / 150.0, screen.tiles(), screen.drawn());
}

#[test]
fn a_lens_shows_the_saved_picture_enlarged_about_its_center() {
    let mut frame = scenes::frame("menu-list").unwrap();
    let plain = ink(&screen_ops(&frame, &Shown::settled(&frame), DISPLAY));
    let shot = std::env::temp_dir().join(format!("padwight-lens-test-{}.png", std::process::id()));
    screen(&frame, DISPLAY, 1.0, Rgba::rgb(0.2, 0.3, 0.5)).unwrap().save(&shot).unwrap();
    frame.lens = Some(crate::overlay::Lens { x: 0.5, y: 0.5, zoom: 3.0, shot: Some(shot.clone()) });
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    assert!(balanced(&ops));
    assert!(matches!(ops.last(), Some(Op::Rect { border: Some(_), .. })), "the rim goes on top");
    let picture = ops.iter().find_map(|op| if let Op::Picture { rect, .. } = op { Some(*rect) } else { None });
    let rect = picture.expect("the lens draws the picture");
    assert_eq!((rect.w, rect.h), (3.0 * 1920.0, 3.0 * 1080.0));
    // The point under the lens's center stays where it is.
    assert!((rect.x + rect.w * 0.5 - 960.0).abs() < 0.5 && (rect.y + rect.h * 0.5 - 540.0).abs() < 0.5, "{rect:?}");
    assert!(plain.w > 0.0);
    if let Some(out) = std::env::var_os("PADWIGHT_LENS_OUT") {
        let image = screen(&frame, DISPLAY, 1.0, Rgba::rgb(0.2, 0.3, 0.5)).unwrap();
        image.save(std::path::Path::new(&out)).unwrap();
    }
    let _ = std::fs::remove_file(shot);
}

#[test]
fn a_lens_without_its_picture_yet_is_only_a_rim() {
    let mut frame = scenes::frame("menu-list").unwrap();
    frame.lens = Some(crate::overlay::Lens { x: 0.0, y: 1.0, zoom: 3.0, shot: None });
    let ops = screen_ops(&frame, &Shown::settled(&frame), DISPLAY);
    assert!(balanced(&ops));
    assert!(!ops.iter().any(|op| matches!(op, Op::Picture { .. })));
}
