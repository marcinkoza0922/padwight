use super::*;
use crate::{
    config::{Case, OverlayStyle, Typography},
    debug::scenes,
    info::{Charge, Glyphs, Icon, PadFamily},
    overlay::render::{self, Fill, Op, Point, Rect, balanced, bounds, sample, text},
};

fn style() -> Style {
    Style::of(&OverlayStyle::default(), None)
}

fn laid(node: &Node, width: f32) -> (Point, Vec<Op>) {
    node.ops(Point::ZERO, width)
}

fn texts(ops: &[Op]) -> Vec<(Point, Rgba, Rect)> {
    ops.iter().filter_map(|op| if let Op::Text { at, color, ink, .. } = op { Some((*at, *color, *ink)) } else { None }).collect()
}

/// The boxes drawn with a border, in order.
fn outlined(ops: &[Op]) -> Vec<(Rect, Corners)> {
    ops.iter().filter_map(|op| if let Op::Rect { rect, radii, border: Some(_), .. } = op { Some((*rect, *radii)) } else { None }).collect()
}

/// The height of a line of text of `size`.
fn line_height(size: f32) -> f32 {
    text::shape(&text::Spec::new("Jump", size)).size.y
}

#[test]
fn a_disc_is_a_square_and_a_tag_is_as_tall_with_room_beside_its_label() {
    let style = style();
    for s in [1.0, 2.0] {
        let disc = render::glyph("A", (Some([0x3c, 0xa0, 0x3c]), true), &style, s, 1.0);
        assert_eq!(disc.measure(f32::INFINITY), Point::new(24.0 * s, 24.0 * s));
        let (_, ops) = laid(&disc, f32::INFINITY);
        let (rect, radii) = outlined(&ops)[0];
        assert_eq!((rect.w, rect.h, radii), (24.0 * s, 24.0 * s, Corners::all(12.0 * s)));
        let tag = render::glyph("LT", (None, false), &style, s, 1.0);
        let label = text::shape(&text::Spec::new("LT", 13.0 * s)).size;
        assert_eq!(tag.measure(f32::INFINITY), Point::new(label.x + 14.0 * s, 24.0 * s));
        // A label of three letters is a tag even on a round button.
        let long = render::glyph("Esc", (Some([1, 2, 3]), true), &style, s, 1.0);
        assert_eq!(outlined(&laid(&long, f32::INFINITY).1)[0].1, Corners::all(6.0 * s));
    }
}

#[test]
fn a_glyph_fill_fades_with_the_line() {
    let ops = laid(&render::glyph("A", (Some([200, 60, 60]), true), &style(), 1.0, 0.5), f32::INFINITY).1;
    let fill = ops.iter().find_map(|op| if let Op::Rect { fill: Some(Fill::Solid(c)), .. } = op { Some(*c) } else { None });
    assert_eq!(fill, Some(Rgba::from_rgba8(200, 60, 60, 0.5)));
}

#[test]
fn the_dpad_is_three_cells_square_with_its_lit_arms_selected() {
    let style = style();
    let pad = render::dpad([true, false, false, true], &style, 2.0);
    assert_eq!(pad.measure(100.0), Point::new(48.0, 48.0));
    let (_, ops) = laid(&pad, 100.0);
    let fills: Vec<Rgba> = ops.iter().filter_map(|op| if let Op::Rect { fill: Some(Fill::Solid(c)), .. } = op { Some(*c) } else { None }).collect();
    // Up, left, middle, right, down; the gaps draw nothing.
    assert_eq!(fills.iter().map(|c| *c == style.selected).collect::<Vec<_>>(), [true, false, false, true, false]);
}

#[test]
fn icons_are_square_and_a_missing_one_shows_its_token() {
    let style = style();
    let icon = render::segment(&Segment::Icon(Icon::Battery(Charge { percent: 50, estimated: false })), &style, 1.5, 1.0);
    assert_eq!(icon.measure(100.0), Point::new(33.0, 33.0));
    let (_, ops) = laid(&rich("{icon:sword} Melee", &style, 17.0, style.background_text), 400.0);
    let svg = ops.iter().find_map(|op| if let Op::Svg { rect, .. } = op { Some(*rect) } else { None });
    assert_eq!(svg.map(|r| (r.w, r.h)), Some((17.0 * 1.25, 17.0 * 1.25)));
    let (_, ops) = laid(&rich("{icon:no_such_icon} Melee", &style, 17.0, style.background_text), 400.0);
    assert!(!ops.iter().any(|op| matches!(op, Op::Svg { .. })));
    let runs = texts(&ops);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].1.a, style.background_text.a * 0.7, "the token is drawn dimmer");
}

#[test]
fn plain_text_is_one_run_with_the_typography_applied() {
    let config = OverlayStyle { typography: Typography { case: Case::Upper, letter_spacing: 2.0, size: 1.5 }, ..OverlayStyle::default() };
    let style = Style::of(&config, None);
    let (size, ops) = laid(&rich("Plain words here", &style, 20.0, Rgba::WHITE), 1000.0);
    assert_eq!(texts(&ops).len(), 1);
    let spec = text::Spec { case: Case::Upper, spacing: 2.0, ..text::Spec::new("Plain words here", 30.0) };
    assert_eq!(size, text::shape(&spec).size);
}

#[test]
fn a_line_wraps_between_words_and_never_inside_one() {
    let style = style();
    let line = render::rich("{south} alpha beta gamma delta epsilon zeta", &style, 17.0, style.background_text);
    let (one, ops) = laid(&line, 2000.0);
    // The glyph's label and a run for each word.
    assert_eq!(texts(&ops).len(), 1 + 6);
    let longest = texts(&ops).iter().skip(1).map(|(_, _, ink)| ink.w).fold(0.0, f32::max);
    let (size, ops) = laid(&line, 140.0);
    assert!(size.y > one.y * 1.8, "{one:?} {size:?}");
    assert!(size.x <= 140.0 && size.x > longest);
    // The same runs, whole, on several rows.
    let rows: std::collections::BTreeSet<i32> = texts(&ops).iter().map(|(at, ..)| (at.y * 10.0) as i32).collect();
    assert!(rows.len() >= 3, "{rows:?}");
    assert_eq!(texts(&ops).len(), 7);
    // A word that fits nowhere beside the glyph goes below it, rather than breaking.
    let (size, ops) = laid(&rich("{south} Supercalifragilistic", &style, 17.0, style.background_text), 120.0);
    assert!(size.y > 24.0 + 17.0, "{size:?}");
    assert_eq!(texts(&ops).len(), 2);
}

#[test]
fn glyphs_and_words_are_centered_on_one_line() {
    let style = style();
    let (size, ops) = laid(&rich("{south} Jump {lt} Aim", &style, 17.0, style.background_text), 2000.0);
    let middle = size.y / 2.0;
    for (rect, _) in outlined(&ops) {
        assert!((rect.y + rect.h / 2.0 - middle).abs() < 0.01, "{rect:?} {middle}");
    }
    // The glyphs' labels, then the words.
    let runs = texts(&ops);
    assert_eq!(runs.len(), 4);
    for (at, ..) in [runs[1], runs[3]] {
        assert!((at.y + line_height(17.0) / 2.0 - middle).abs() < 0.01, "{at:?} {middle}");
    }
}

#[test]
fn tags_format_the_text_they_hold() {
    let style = style();
    let ink = style.background_text;
    let (_, ops) = laid(&rich("{b}Bold{/b} {i}slanted{/i} {color:red}red{/color} plain", &style, 17.0, ink), 2000.0);
    let runs = texts(&ops);
    assert_eq!(runs.len(), 4);
    assert_ne!(runs[2].1, ink);
    assert_eq!(runs[2].1.a, ink.a, "a color tag keeps the line's alpha");
    assert_eq!(runs[3].1, ink);
    // Bold is wider than the same word plain, and underlining changes no size.
    let size = |t: &str| laid(&rich(t, &style, 17.0, ink), 2000.0).0;
    assert!(size("{b}Hello{/b} x").x > size("Hello x").x);
    assert_eq!(size("{u}Hello{/u} x"), size("{color:red}Hello{/color} x"));
    // A tag left open holds to the end.
    assert_eq!(size("{b}Hello world"), size("{b}Hello world{/b}"));
}

#[test]
fn a_box_outlines_its_pieces_in_the_text_color() {
    let style = style();
    let (size, ops) = laid(&rich("{box}{west}{/box} go", &style, 17.0, style.background_text), 2000.0);
    let boxes = outlined(&ops);
    // The box, then the glyph in it.
    assert_eq!(boxes.len(), 2);
    let (edge, radii) = boxes[0];
    assert_eq!(radii, Corners::all(3.0 + 3.0 * style.corners));
    let glyph = boxes[1].0;
    assert!((edge.w - (glyph.w + 2.0 * 17.0 * 0.35)).abs() < 0.01 && (edge.h - (glyph.h + 2.0 * 17.0 * 0.05)).abs() < 0.01, "{edge:?} {glyph:?}");
    assert!(size.x > edge.w);
    assert!(balanced(&ops) && bounds(&ops).is_some());
}

#[test]
fn a_bar_is_a_track_with_its_share_filled() {
    let (size, ops) = laid(&render::segment(&Segment::Bar(0.25), &style(), 2.0, 1.0), 1000.0);
    assert_eq!(size, Point::new(320.0, 12.0));
    let widths: Vec<f32> = ops.iter().filter_map(|op| if let Op::Rect { rect, .. } = op { Some(rect.w) } else { None }).collect();
    assert_eq!(widths, [320.0, 80.0]);
}

#[test]
fn the_pad_family_picks_the_labels() {
    let xbox = style().with_glyphs(Glyphs { family: PadFamily::Xbox, nintendo_layout: false });
    let nintendo = style().with_glyphs(Glyphs { family: PadFamily::Xbox, nintendo_layout: true });
    let drawn = |s: &Style| laid(&rich("{south} {lt}", s, 17.0, Rgba::WHITE), 500.0).1;
    assert_eq!(texts(&drawn(&xbox)).len(), 2);
    assert_ne!(drawn(&xbox), drawn(&nintendo));
}

/// An info scene's cells drawn like the info panel: a table with columns as wide as their widest
/// cell, in a framed panel.
fn info_panel(view: &crate::info::InfoView) -> Node {
    let config = &view.style;
    let style = Style::of(config, None).with_glyphs(Glyphs { family: PadFamily::Xbox, nintendo_layout: false });
    let s = config.scale.clamp(0.5, 2.0);
    let cell = |segments: &[Segment]| render::info_cell(segments, &style, s, view.opacity);
    let columns = view.rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<f32> = (0..columns)
        .map(|j| view.rows.iter().filter_map(|r| r.get(j)).map(|c| cell(c).measure(f32::INFINITY).x).fold(0.0, f32::max))
        .collect();
    let rows = view.rows.iter().map(|row| {
        let cells = widths.iter().enumerate().map(|(j, w)| {
            let inner = row.get(j).map_or_else(Node::space, |c| cell(c));
            Node::container(inner).padding([2.0 * s, 10.0 * s]).width(w + 20.0 * s)
        });
        Node::row(cells).align_y(render::Align::Center)
    });
    let mut body = Node::column(rows);
    if let Some(title) = &view.title {
        body = Node::column([cell(title), body]).spacing(8.0 * s);
    }
    render::framed(body, &style, config, [12.0 * s, 16.0 * s], &crate::overlay::fit::Fit::contents())
}

/// Draws the token scenes' cells with the new renderer, to look at beside
/// `screenshots/overlay/<scene>.png` (`cargo run -q -- debug overlay info info-formatted`).
#[test]
#[ignore = "writes pictures to screenshots/render/"]
fn the_token_scenes_drawn_with_rich_text() {
    for name in ["info", "info-formatted"] {
        let frame = scenes::frame(name).unwrap();
        if let Some(icons) = &frame.pack_icons {
            crate::icons::set_pack_icons(icons);
        }
        for scale in [1.0, 2.0] {
            let panel = info_panel(&frame.info[0]);
            sample::save(&format!("rich-{name}-x{scale}"), &sample::draw(&panel, 1000.0, scale));
        }
    }
}

#[test]
fn the_pieces_other_panels_call_are_exported() {
    let style = style();
    let ink = Rgba::WHITE;
    let setting = render::Setting { size: 16.0, color: ink, scale: 1.0, gap: 2.0, opacity: 1.0, typed: false, trim: false };
    let segments = [Segment::Text("a ".into()), Segment::Bar(0.5)];
    assert_eq!(render::line(&segments, &style, setting).measure(500.0), render::info_cell(&segments, &style, 1.0, 1.0).measure(500.0));
    assert_eq!(render::plain("Hi", &style, 16.0, ink).measure(100.0), render::rich("Hi", &style, 16.0, ink).measure(100.0));
    let play = render::state_icon(crate::media::PlayState::Playing, ink, 30.0);
    assert_eq!(play.measure(100.0), Point::new(30.0, 30.0));
    assert!(render::icon_svg(&Icon::Wifi(80), ink).contains("<svg"));
    assert_eq!(render::bar_of(1.0, 50.0, 4.0).measure(100.0), Point::new(50.0, 4.0));
}
