use super::*;
use crate::{
    config::{Ornament, OverlayStyle, Texture},
    debug::scenes,
    menu::ItemView,
    motion::Anim,
    overlay::{
        OverlayView,
        render::{self, Align, balanced, sample},
    },
};

/// A menu of `n` plain items called A, B, C… with the cursor on `selected`.
fn menu(kind: MenuKind, n: usize, selected: Option<usize>) -> MenuView {
    let items = (0..n).map(|i| ItemView { label: ((b'A' + i as u8) as char).to_string(), ..ItemView::plain() }).collect();
    MenuView {
        title: "Radial".into(),
        kind,
        items,
        selected,
        picks: 0,
        crumbs: Vec::new(),
        hint: String::new(),
        about: String::new(),
        style: OverlayStyle::default(),
        touch: false,
        aim: None,
        prompt: None,
        one_column: false,
        figure: None,
    }
}

fn arcs_of(n: usize, selected: Option<usize>) -> MenuView {
    menu(MenuKind::Radial { stick: Stick::Right, boxes: false }, n, selected)
}

fn boxes_of(n: usize, selected: Option<usize>) -> MenuView {
    menu(MenuKind::Radial { stick: Stick::Left, boxes: true }, n, selected)
}

fn laid(node: &Node) -> (Point, Vec<Op>) {
    let (size, ops) = node.ops(Point::ZERO, 2000.0);
    assert!(balanced(&ops));
    (size, ops)
}

fn paths(ops: &[Op]) -> Vec<&Op> {
    ops.iter().filter(|op| matches!(op, Op::Path { .. })).collect()
}

fn texts(ops: &[Op]) -> usize {
    ops.iter().filter(|op| matches!(op, Op::Text { .. })).count()
}

#[test]
fn the_ring_is_a_square_that_holds_the_arcs() {
    let m = arcs_of(4, None);
    let style = Style::of(&m.style, None);
    let (size, _) = laid(&radial(&m, &style, 1.0));
    assert_eq!((size.x, size.y), (2.0 * 170.0 + 16.0, 2.0 * 170.0 + 16.0));
    // The scale scales it all.
    let (twice, _) = laid(&render::radial(&m, &style, 2.0));
    assert_eq!(twice.x, 2.0 * size.x);
}

#[test]
fn each_arc_is_a_path_with_a_line_after_it() {
    let m = arcs_of(4, Some(1));
    let style = Style::of(&m.style, None);
    let (_, ops) = laid(&radial(&m, &style, 1.0));
    let paths = paths(&ops);
    let strokes = paths.iter().filter(|op| matches!(op, Op::Path { fill: None, stroke: Some(_), .. })).count();
    // Four lines between the arcs.
    assert_eq!(strokes, 4);
    // The panel's disc, four arcs, the cursor's arc lit over its own, and the hub.
    let fills = paths.iter().filter(|op| matches!(op, Op::Path { fill: Some(_), .. })).count();
    assert_eq!(fills, 1 + 4 + 1 + 1);
    // A label for each arc and the stick in the middle.
    assert!(texts(&ops) >= 4);
}

#[test]
fn the_lit_arc_is_blended_toward_the_selected_color() {
    let m = arcs_of(3, Some(0));
    let style = Style::of(&m.style, None);
    let lit = |lit: f32| {
        let anim = Anim::still();
        assert_eq!(anim.lit(true, false), 1.0);
        let style = Style { anim, ..style };
        let mut node = radial(&m, &style, 1.0);
        node = if lit > 0.0 { node } else { radial(&arcs_of(3, None), &style, 1.0) };
        let (_, ops) = laid(&node);
        ops.iter()
            .filter(|op| matches!(op, Op::Path { fill: Some(Fill::Solid(c)), .. } if *c == style.selected))
            .count()
    };
    assert_eq!(lit(1.0), 1);
    assert_eq!(lit(0.0), 0);
}

#[test]
fn an_arc_fades_in_with_its_stagger() {
    let m = arcs_of(3, None);
    let mut style = Style::of(&m.style, None);
    style.anim = Anim::still();
    let alphas = |style: &Style| -> Vec<f32> {
        let (_, ops) = laid(&radial(&m, style, 1.0));
        ops.iter()
            .filter_map(|op| match op {
                Op::Path { fill: Some(Fill::Solid(c)), .. } if c.r == style.item.r && c.g == style.item.g && c.b == style.item.b => Some(c.a),
                _ => None,
            })
            .collect()
    };
    let whole = alphas(&style);
    assert!(whole.iter().all(|&a| a == style.item.a));
    // Fading the whole panel fades the arcs by as much, in colors and not as separate layers.
    let half = alphas(&style.faded(0.5));
    assert!(half.iter().all(|&a| (a - style.item.a * 0.5).abs() < 1e-6), "{half:?}");
}

#[test]
fn a_single_item_is_a_ring_not_an_arc() {
    let m = arcs_of(1, Some(0));
    let style = Style::of(&m.style, None);
    let (_, ops) = laid(&radial(&m, &style, 1.0));
    let ring = ops.iter().find_map(|op| match op {
        Op::Path { fill: None, stroke: Some(stroke), .. } if stroke.width > 10.0 => Some(stroke.width),
        _ => None,
    });
    assert_eq!(ring, Some(Ring::new(&[1.0], 1.0).outer - Ring::new(&[1.0], 1.0).inner));
    // No lines between arcs when there is only one.
    assert_eq!(ops.iter().filter(|op| matches!(op, Op::Path { stroke: Some(s), .. } if s.width == 2.0)).count(), 0);
}

#[test]
fn filters_are_cut_to_each_arc() {
    let mut m = arcs_of(4, None);
    m.style.ornament = Ornament { wood: Some(render_texture()), ..Ornament::default() };
    let style = Style::of(&m.style, None);
    let (_, ops) = laid(&radial(&m, &style, 1.0));
    let cuts: Vec<_> = ops
        .iter()
        .filter_map(|op| if let Op::Pattern { cut, rect, .. } = op { Some((*cut, *rect)) } else { None })
        .collect();
    assert_eq!(cuts.len(), 4);
    let ring = Ring::new(&[1.0; 4], 1.0);
    for (i, (cut, rect)) in cuts.iter().enumerate() {
        let (x, y, w, h) = ring.arc_box(i);
        assert_eq!((rect.x, rect.y, rect.w, rect.h), (x, y, w, h));
        let crate::overlay::art::Shape::Arc { start, end, gap, inner, outer, center } = *cut else { panic!("{cut:?}") };
        assert_eq!((start, end), ring.arcs[i]);
        assert_eq!((gap, inner, outer), (2.0, ring.inner, ring.outer));
        assert_eq!(center, (ring.size() / 2.0 - x, ring.size() / 2.0 - y));
    }
}

fn render_texture() -> Texture {
    let mut texture = Texture::new(0.6);
    texture.on = crate::config::FilterOn::Items;
    texture
}

#[test]
fn the_hub_names_the_stick_and_the_aim_dot_sits_just_outside_it() {
    let mut m = arcs_of(4, None);
    let style = Style::of(&m.style, None);
    let dots = |m: &MenuView| {
        let (_, ops) = laid(&radial(m, &style, 1.0));
        ops.iter()
            .filter_map(|op| match op {
                Op::Rect { rect, fill: Some(Fill::Solid(c)), .. } if *c == style.selected_text && rect.w == 11.0 => Some(*rect),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(dots(&m).is_empty());
    m.aim = Some(90.0);
    let [dot] = dots(&m)[..] else { panic!("one dot") };
    let size = Ring::new(&[1.0; 4], 1.0).size();
    // Straight right of the middle, at the hub's radius and 10 more.
    assert_eq!((dot.x + 5.5 - size / 2.0, dot.y + 5.5 - size / 2.0), (24.0 + 10.0, 0.0));
}

#[test]
fn boxes_sit_at_equal_angles_around_a_hub_with_a_disc() {
    let m = boxes_of(4, Some(0));
    let style = Style::of(&m.style, None);
    let (size, ops) = laid(&radial(&m, &style, 1.0));
    let (side, radius) = crate::radial::box_square(4, 1.0);
    assert_eq!((size.x, size.y), (side, side));
    // The first at the top, the second to its right.
    let boxes: Vec<Rect> =
        ops.iter().filter_map(|op| if let Op::Rect { rect, radii, .. } = op { (rect.w == 120.0 && radii.top_left > 0.0).then_some(*rect) } else { None }).collect();
    assert_eq!(boxes.len(), 4);
    assert_eq!(boxes[0].center(), Point::new(side / 2.0, side / 2.0 - radius));
    assert!((boxes[1].center().x - (side / 2.0 + radius)).abs() < 1e-3);
    // The hub's disc is a rounded box of its own, and the selected box has the thicker outline.
    assert!(ops.iter().any(|op| matches!(op, Op::Rect { rect, radii, .. } if rect.w == 48.0 && radii.top_left == 24.0)));
    let widths: Vec<f32> = ops.iter().filter_map(|op| if let Op::Rect { rect, border: Some(b), .. } = op { (rect.w == 120.0).then_some(b.width) } else { None }).collect();
    assert_eq!(widths, [2.0, 1.0, 1.0, 1.0]);
    // Boxes neither swell nor slide in here, but their stagger is a layer round each.
    let faded = Style { anim: Anim { clock: 0.0, ..Anim::still() }, ..style };
    let (_, ops) = laid(&radial(&m, &faded, 1.0));
    assert!(balanced(&ops));
}

#[test]
fn a_notes_label_is_muted_and_has_no_box() {
    let mut m = boxes_of(2, Some(1));
    m.items[1].tone = crate::menu::Tone::Note;
    let style = Style::of(&m.style, None);
    assert!(!label::selected(&m, 1));
    let (_, ops) = laid(&radial(&m, &style, 1.0));
    let widths: Vec<f32> = ops.iter().filter_map(|op| if let Op::Rect { rect, border: Some(b), .. } = op { (rect.w == 120.0).then_some(b.width) } else { None }).collect();
    assert_eq!(widths[0], 1.0);
}

#[test]
fn labels_carry_glyphs_tags_and_a_submenu_arrow() {
    use crate::config::Button;
    let mut m = arcs_of(2, None);
    m.items[0].buttons = vec![Button::South];
    m.items[0].button = Some("X".into());
    m.items[0].submenu = true;
    let style = Style::of(&m.style, None);
    let plain = laid(&radial(&arcs_of(2, None), &style, 1.0)).1;
    let (_, ops) = laid(&radial(&m, &style, 1.0));
    assert!(texts(&ops) >= texts(&plain) + 3, "the glyph, the tag and the arrow");
}

/// A menu as a framed panel, with its title and hint, to look at.
fn panel(m: &MenuView) -> Node {
    let style = Style { anim: Anim { clock: 0.625, ..Anim::still() }, ..Style::of(&m.style, None) };
    let s = m.style.scale.clamp(0.5, 2.0);
    let title = render::rich(&m.title, &style, 20.0 * s, style.background_text);
    let hint = render::rich(&m.hint, &style, 13.0 * s, style.muted);
    let body = Node::column([title, radial(m, &style, s), hint]).spacing(14.0 * s).align_x(Align::Center);
    render::framed(body, &style, &m.style, 22.0 * s, &crate::overlay::fit::Fit::contents())
}

/// Every radial scene, and the same with the stick aimed, to look at beside
/// `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn every_radial_scene_drawn() {
    for name in ["menu-radial", "menu-radial-boxes", "menu-radial-plaid"] {
        let Some(OverlayView::Menu(mut menu)) = scenes::frame(name).unwrap().active else { panic!("{name} is a menu") };
        sample::save(&format!("radial-{name}"), &sample::draw(&panel(&menu), 2000.0, 1.0));
        menu.aim = Some(135.0);
        sample::save(&format!("radial-{name}-aimed"), &sample::draw(&panel(&menu), 2000.0, 1.0));
    }
}
