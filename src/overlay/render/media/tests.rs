use super::*;
use crate::{
    debug::scenes,
    overlay::{OverlayView, render::{self, Fill, Op, Point, Rect, Rgba, sample}},
};

fn laid(node: &Node) -> (Point, Vec<Op>) {
    node.ops(Point::ZERO, f32::INFINITY)
}

/// The rectangles filled with `color`.
fn filled(ops: &[Op], color: Rgba) -> Vec<Rect> {
    ops.iter()
        .filter_map(|op| match op {
            Op::Rect { rect, fill: Some(Fill::Solid(c)), .. } if *c == color => Some(*rect),
            _ => None,
        })
        .collect()
}

fn offer_view() -> OfferView {
    OfferView {
        title: "Add controller profiles?".into(),
        detail: "A short description of the pack that wraps across the panel.".into(),
        answers: vec![("{confirm}".into(), "Yes".into()), ("{cancel}".into(), "Not now".into()), ("{x}".into(), "Never".into())],
        hint: "Not now asks again next time.".into(),
        style: OverlayStyle::default(),
    }
}

#[test]
fn the_media_panel_is_560_wide_with_its_padding() {
    let view = MediaView::sample(OverlayStyle::default());
    let (size, _) = laid(&media(&view, None, Glyphs::default(), &Fit::contents(), &Anim::still()));
    assert_eq!(size.x, 560.0 + 2.0 * 18.0);
}

#[test]
fn the_progress_bar_fills_the_share_of_the_track_that_has_played() {
    let view = MediaView::sample(OverlayStyle::default());
    let (_, ops) = laid(&media(&view, None, Glyphs::default(), &Fit::contents(), &Anim::still()));
    let fill = filled(&ops, Rgba::from_rgb8(88, 101, 242));
    assert_eq!(fill.len(), 1);
    let progress = view.position_ms as f32 / view.length_ms as f32;
    assert!((fill[0].w - 560.0 * progress).abs() < 0.01, "{:?}", fill[0]);
}

#[test]
fn without_a_player_there_is_no_bar() {
    let view = MediaView { player: None, ..MediaView::sample(OverlayStyle::default()) };
    let (_, ops) = laid(&media(&view, None, Glyphs::default(), &Fit::contents(), &Anim::still()));
    assert!(filled(&ops, Rgba::from_rgb8(88, 101, 242)).is_empty());
    assert!(ops.iter().any(|op| matches!(op, Op::Text { .. })));
}

#[test]
fn the_offer_is_520_of_detail_wide_with_its_padding() {
    let (size, _) = laid(&offer(&offer_view(), None, Glyphs::default(), &Fit::contents(), &Anim::still()));
    assert_eq!(size.x, 520.0 + 2.0 * 22.0);
}

#[test]
fn each_answer_is_an_item_box_and_only_the_confirm_answer_is_selected() {
    let view = offer_view();
    let (_, ops) = laid(&offer(&view, None, Glyphs::default(), &Fit::contents(), &Anim::still()));
    // One box for each answer, filled with the items' color.
    let item = render::Style::of(&view.style, None).item;
    let boxes: Vec<Option<_>> = ops
        .iter()
        .filter_map(|op| match op {
            Op::Rect { fill: Some(Fill::Solid(c)), border, .. } if *c == item => Some(*border),
            _ => None,
        })
        .collect();
    assert_eq!(boxes.len(), view.answers.len());
    // The default answer's outline is the selected look, the others' is not.
    assert!(boxes[0] != boxes[1]);
    assert_eq!(boxes[1], boxes[2]);
}

/// Each media and offer scene, drawn as its panel.
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn the_media_and_offer_scenes_drawn_as_panels() {
    for name in ["media", "offer"] {
        let Some(view) = scenes::frame(name).unwrap().active else { continue };
        let fit = Fit::contents();
        let panel = match view {
            OverlayView::Media(m) => render::media(&m, None, Glyphs::default(), &fit, &Anim::still()),
            OverlayView::Offer(o) => render::offer(&o, None, Glyphs::default(), &fit, &Anim::still()),
            _ => continue,
        };
        sample::save(&format!("{name}-{name}"), &sample::draw(&panel, 1000.0, 1.0));
    }
}
