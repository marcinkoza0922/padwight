use super::*;
use crate::{
    config::{Bevel, FilterOn, Ornament, Shade, Stroke, Texture},
    debug::scenes,
    menu::Tone,
    overlay::{OverlayView, render::{self, Item, balanced, blend, item, sample, text::Spec}},
};

fn body(style: &Style) -> Node {
    Node::text(Spec::new("Resume", 16.0), style.item_text).center_x(190.0).center_y(52.0)
}

fn laid(node: &Node) -> (Point, Vec<Op>) {
    node.ops(Point::ZERO, 1000.0)
}

fn boxes(ops: &[Op]) -> Vec<(&Option<Fill>, &Option<render::Outline>)> {
    ops.iter().filter_map(|op| if let Op::Rect { fill, border, .. } = op { Some((fill, border)) } else { None }).collect()
}

/// The item for `config` in `state`, as ops.
fn drawn(config: &OverlayStyle, state: Item, tone: Tone) -> Vec<Op> {
    let style = Style::of(config, None).toned(tone);
    let colors = state.colors(&style);
    let (_, ops) = laid(&item(body(&colors), &colors, &state, config, 26.0));
    assert!(balanced(&ops));
    ops
}

fn lit() -> Item {
    Item { selected: true, lit: 1.0, ..Item::REST }
}

fn on_items(mut texture: Texture) -> Texture {
    texture.on = FilterOn::Items;
    texture
}

#[test]
fn an_item_at_rest_is_its_body_in_a_faintly_outlined_box() {
    let config = OverlayStyle::default();
    let style = Style::of(&config, None);
    let (size, ops) = laid(&item(body(&style), &style, &Item::REST, &config, 26.0));
    assert_eq!((size.x, size.y), (190.0, 52.0));
    // No layer, no shadow: nothing moves and nothing glows.
    assert!(!ops.iter().any(|op| matches!(op, Op::Layer(_) | Op::Shadow { .. })));
    let [(fill, border)] = boxes(&ops)[..] else { panic!("one box") };
    assert_eq!(*fill, Some(Fill::Solid(style.item)));
    let border = border.unwrap();
    assert_eq!(border.width, 1.0);
    assert_eq!(border.color, style.item_text.scale_alpha(0.12));
}

#[test]
fn the_selected_item_is_outlined_in_its_text_color() {
    let config = OverlayStyle::default();
    let ops = drawn(&config, lit(), Tone::Normal);
    let border = boxes(&ops)[0].1.unwrap();
    let style = Style::of(&config, None);
    assert_eq!((border.width, border.color), (2.0, style.selected_text));
}

#[test]
fn lighting_and_flashing_blend_the_colors_as_iced_does() {
    let style = Style::of(&OverlayStyle::default(), None);
    let half = Item { lit: 0.5, flash: 0.4, ..Item::REST }.colors(&style);
    assert_eq!(half.item, blend(blend(style.item, style.selected, 0.5), Rgba::WHITE, 0.2));
    assert_eq!(half.item_text, blend(style.item_text, style.selected_text, 0.5));
    // The stagger fade is the layer's, not the colors'.
    assert_eq!(Item { fade: 0.25, ..Item::REST }.colors(&style).item, style.item);
    // `Style::lit_item` still fades the colors for those that want it.
    assert_eq!(style.lit_item(0.0, 0.0, 0.25).item.a, style.item.a * 0.25);
}

#[test]
fn a_stagger_fade_draws_the_whole_item_as_one_layer() {
    let config = OverlayStyle { ornament: Ornament { wood: Some(on_items(Texture::new(0.5))), ..Ornament::default() }, ..OverlayStyle::default() };
    let state = Item { fade: 0.4, scale: 1.1, shift: 12.0, ..Item::REST };
    let ops = drawn(&config, state, Tone::Normal);
    let Op::Layer(layer) = ops[0] else { panic!("the item opens with its layer, got {:?}", ops[0]) };
    assert_eq!((layer.opacity, layer.scale, layer.shift), (0.4, 1.1, Point::new(12.0, 0.0)));
    assert_eq!(layer.about, Rect::new(0.0, 0.0, 190.0, 52.0).center());
    assert!(matches!(ops.last(), Some(Op::Unlayer)));
    // The box, its filter and its text are all inside it, drawn at full strength.
    let inside = &ops[1..ops.len() - 1];
    assert!(inside.iter().any(|op| matches!(op, Op::Pattern { opacity, .. } if *opacity > 0.0)));
    assert!(inside.iter().any(|op| matches!(op, Op::Text { .. })));
    assert_eq!(boxes(inside)[0].0, &Some(Fill::Solid(Style::of(&config, None).item)));
    assert_eq!(inside.iter().filter(|op| matches!(op, Op::Layer(_))).count(), 0);
}

#[test]
fn a_bevel_lights_the_top_and_flips_when_pressed() {
    let config = OverlayStyle { ornament: Ornament { bevel: Some(Bevel { depth: 0.8 }), ..Ornament::default() }, ..OverlayStyle::default() };
    let style = Style::of(&config, None);
    let ends = |ops: &[Op]| match boxes(ops)[0].0 {
        Some(Fill::Linear(l)) => (l.angle, l.stops[0].color, l.stops[1].color),
        other => panic!("{other:?}"),
    };
    let (angle, top, bottom) = ends(&drawn(&config, Item::REST, Tone::Normal));
    assert_eq!(angle, std::f32::consts::PI);
    assert!(top.r > style.item.r && bottom.r < style.item.r);
    let (_, pressed_top, pressed_bottom) = ends(&drawn(&config, lit(), Tone::Normal));
    assert!(pressed_top.r < top.r && pressed_bottom.r > bottom.r);
    // A raised item casts a shadow and catches light along its edge.
    let rest = drawn(&config, Item::REST, Tone::Normal);
    assert!(matches!(rest[0], Op::Shadow { .. }));
    assert_eq!(boxes(&rest)[0].1.unwrap().width, 1.0);
    // The pressed one casts none.
    assert!(!drawn(&config, lit(), Tone::Normal).iter().any(|op| matches!(op, Op::Shadow { .. })));
}

#[test]
fn a_glow_follows_the_selected_item_and_pulses() {
    let config = OverlayStyle { glow: Some(Shade { pulse: true, ..Shade::glow("#ffcc40") }), ..OverlayStyle::default() };
    let alpha = |clock: f32, state: Item| {
        let style = Style { anim: Anim { clock, ..Anim::still() }, ..Style::of(&config, None) };
        let colors = state.colors(&style);
        let (_, ops) = laid(&item(body(&colors), &colors, &state, &config, 26.0));
        ops.iter().find_map(|op| if let Op::Shadow { shadow, .. } = op { Some(shadow.color.a) } else { None })
    };
    assert_eq!(alpha(0.0, Item::REST), None);
    let (low, high) = (alpha(0.0, lit()).unwrap(), alpha(0.625, lit()).unwrap());
    // At a quarter of 2.5 s the breath is 1.0, at 0 it is 0.8 of the glow.
    assert!((low / high - 0.8).abs() < 1e-5, "{low} {high}");
}

#[test]
fn item_outlines_follow_the_style() {
    let stroke = Some(Stroke { width: 3.0, color: "#ff0000".into(), opacity: 1.0 });
    let config = OverlayStyle { item_border: stroke, ..OverlayStyle::default() };
    let border = boxes(&drawn(&config, Item::REST, Tone::Normal))[0].1.unwrap();
    assert_eq!((border.width, border.color), (3.0, Rgba::rgb(1.0, 0.0, 0.0)));
    // A width of 0 is no outline at all.
    let none = Some(Stroke { width: 0.0, color: "#ff0000".into(), opacity: 1.0 });
    assert!(boxes(&drawn(&OverlayStyle { item_border: none, ..OverlayStyle::default() }, lit(), Tone::Normal))[0].1.is_none());
    // A plain style's items are bare, the selected one outlined.
    let mut plain = OverlayStyle::default();
    plain.lite.plain = true;
    assert!(boxes(&drawn(&plain, Item::REST, Tone::Normal))[0].1.is_none());
    assert_eq!(boxes(&drawn(&plain, lit(), Tone::Normal))[0].1.unwrap().width, 2.0);
}

#[test]
fn tones_tint_the_box_and_notes_have_none() {
    let config = OverlayStyle::default();
    let fill = |tone, colorblind: bool| {
        let style = Style { colorblind, ..Style::of(&config, None) }.toned(tone);
        let (_, ops) = laid(&item(body(&style), &style, &Item::REST, &config, 26.0));
        match boxes(&ops)[0].0 {
            Some(Fill::Solid(c)) => *c,
            other => panic!("{other:?}"),
        }
    };
    let normal = fill(Tone::Normal, false);
    let (add, remove) = (fill(Tone::Add, false), fill(Tone::Remove, false));
    assert!(add.g > add.r && remove.r > remove.g);
    // Color-blind tints are blue and orange.
    let (add_b, remove_b) = (fill(Tone::Add, true), fill(Tone::Remove, true));
    assert!(add_b.b > add_b.r && remove_b.r > remove_b.b);
    assert_ne!(add, add_b);
    assert_ne!(normal, add);
    assert_eq!(fill(Tone::Note, false), Rgba::TRANSPARENT);
}

#[test]
fn item_filters_take_their_picture_from_the_panels_corner() {
    let mut config = OverlayStyle::default();
    config.ornament.wood = Some(on_items(Texture::new(0.8)));
    let style = Style::of(&config, None);
    let framed = render::framed(item(body(&style), &style, &Item::REST, &config, 26.0), &style, &config, 30.0, &crate::overlay::fit::Fit::contents());
    let (_, ops) = laid(&framed);
    let Some(Op::Pattern { origin, rect, cut, .. }) = ops.iter().find(|op| matches!(op, Op::Pattern { .. })) else { panic!("no filter on the item") };
    assert_eq!((origin.x, origin.y), (0.0, 0.0));
    assert!(rect.x > 0.0 && rect.y > 0.0);
    assert_eq!(*cut, art::Shape::Rounded(26.0));
    assert_eq!((rect.w, rect.h), (190.0, 52.0));
}

#[test]
fn an_item_follows_the_animation() {
    let anim = Anim::still();
    let rest = Item::of(&anim, 0, false);
    assert_eq!((rest.lit, rest.flash, rest.fade, rest.scale, rest.shift), (0.0, 0.0, 1.0, 1.0, 0.0));
    assert_eq!(Item::of(&anim, 0, true).lit, 1.0);
}

/// Every state of an item, in the styles the menu scenes use, to look at beside
/// `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay <scene>`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn item_boxes_in_each_style_and_state() {
    let states: [(&str, Item, Tone, bool); 9] = [
        ("rest", Item::REST, Tone::Normal, false),
        ("lit", lit(), Tone::Normal, false),
        ("glide", Item { lit: 0.5, ..Item::REST }, Tone::Normal, false),
        ("flash", Item { flash: 0.7, ..lit() }, Tone::Normal, false),
        ("stagger", Item { fade: 0.5, shift: 14.0, ..Item::REST }, Tone::Normal, false),
        ("swell", Item { scale: 1.12, ..lit() }, Tone::Normal, false),
        ("add", Item::REST, Tone::Add, false),
        ("remove", Item::REST, Tone::Remove, true),
        ("note", Item::REST, Tone::Note, false),
    ];
    let names = ["menu-list", "menu-styled", "menu-metal", "menu-wood", "menu-plaid-items", "menu-paper", "menu-leather", "menu-denim"];
    for name in names {
        let Some(OverlayView::Menu(menu)) = scenes::frame(name).unwrap().active else { continue };
        let config = menu.style;
        let base = Style { anim: Anim { clock: 0.625, ..Anim::still() }, ..Style::of(&config, None) };
        let rows: Vec<Node> = states
            .iter()
            .map(|&(label, state, tone, colorblind)| {
                let style = Style { colorblind, ..base }.toned(tone);
                let colors = state.colors(&style);
                let text = Node::text(Spec::new(label, 16.0), colors.item_text).center_x(250.0).center_y(52.0);
                item(text, &colors, &state, &config, base.corners * 26.0)
            })
            .collect();
        let panel = render::framed(Node::column(rows).spacing(10.0).width(250.0), &base, &config, 26.0, &crate::overlay::fit::Fit::contents());
        sample::save(&format!("item-{name}"), &sample::draw(&panel, 1000.0, 1.0));
    }
}
