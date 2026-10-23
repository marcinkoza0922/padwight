//! The mapping labels beside the drawing: which column each goes in, where its leader line
//! meets the part it names, and how its text is shortened to fit.

use std::fmt::Write;

use super::{ACTIVE, Spot, layout::Layout};
use crate::{
    config::{Button, Trigger},
    info::PadModel,
};

/// Margin on each side of the controller for mapping labels: room for a menu's name.
pub const MARGIN: f32 = 140.0;
/// Size of the whole drawing, in SVG units (shown 1:1 in logical pixels).
pub const WIDTH: f32 = 420.0 + 2.0 * MARGIN;
pub const HEIGHT: f32 = 275.0;
/// Width of each label column; label lines end at its inner edge.
pub const LABEL_COLUMN: f32 = MARGIN - 6.0;
/// Where the right label column starts.
pub const RIGHT_COLUMN_X: f32 = MARGIN + 426.0;
const LABEL_SPACING: f32 = 29.0;
/// Font size of label text, in logical pixels.
pub const LABEL_TEXT_SIZE: f32 = 11.0;
/// Room for label text inside its pill: the column less the pill's padding and border.
const LABEL_TEXT_WIDTH: f32 = LABEL_COLUMN - 14.0;

/// What ends the label of a button that also has a double tap, triple tap or long press. It
/// survives the label being cut, and the picture draws it in blue.
pub const GESTURE_MARK: &str = " +";

/// A mapping label's text and row; it sits in the left or right column.
pub struct PlacedLabel {
    pub spot: Spot,
    pub text: String,
    pub y: f32,
    pub right: bool,
    anchor: (f32, f32),
}

/// Where each label's leader line meets its part (before the margin offset), and the column its
/// label goes in. Buttons with a letter on them are met at their edge so the line doesn't cover
/// the letter.
fn anchors(l: &Layout) -> Vec<(Spot, (f32, f32), bool)> {
    use Button::{DpadDown, DpadLeft, DpadRight, DpadUp, Guide, LeftBumper, LeftStick, RightBumper, RightStick, Select, Start};
    let (lx, rx) = l.shoulders;
    let quarter = l.shoulder_w / 4.0;
    // Meet each bar in the middle of what shows above the body.
    let trigger_y = l.trigger_top() + super::TRIGGER_HEIGHT / 2.0;
    let bumper_y = l.bumper_top() + super::BUMPER_ABOVE / 2.0;
    let (dx, dy) = l.dpad;
    let arm = l.dpad_reach * 18.0 / 26.0;
    let (fx, fy) = l.face;
    let [south, east, west, north] = l.face_offsets();
    let at = |(x, y, _): (f32, f32, f32)| (x, y);
    vec![
        (Spot::Trigger(Trigger::Left), (lx - quarter, trigger_y), false),
        (Spot::Button(LeftBumper), (lx, bumper_y), false),
        (Spot::Button(Select), at(l.select), false),
        (Spot::Button(LeftStick), l.left_stick, false),
        (Spot::Button(DpadUp), (dx, dy - arm), false),
        (Spot::Button(DpadLeft), (dx - arm, dy), false),
        (Spot::Button(DpadRight), (dx + arm, dy), false),
        (Spot::Button(DpadDown), (dx, dy + arm), false),
        (Spot::Trigger(Trigger::Right), (rx + quarter, trigger_y), true),
        (Spot::Button(RightBumper), (rx, bumper_y), true),
        (Spot::Button(Guide), at(l.guide), true),
        (Spot::Button(Button::North), (fx + north.0 + l.face_r, fy + north.1), true),
        (Spot::Button(Button::East), (fx + east.0 + l.face_r, fy + east.1), true),
        (Spot::Button(Button::South), (fx + south.0 + l.face_r, fy + south.1), true),
        (Spot::Button(Button::West), (fx + west.0, fy + west.1 + l.face_r), true),
        (Spot::Button(Start), at(l.start), true),
        (Spot::Button(RightStick), l.right_stick, true),
    ]
}

/// Lays out mapping labels in each margin column: each as near the height of its part as the
/// rows allow, ordered so that no two leader lines cross.
pub fn place_labels(labels: &[(Spot, String)], model: Option<PadModel>) -> Vec<PlacedLabel> {
    let text_for = |b: Spot| labels.iter().find(|(l, _)| *l == b).map(|(_, t)| t.as_str());
    let layout = Layout::of(model);
    let obstacles = obstacles(layout);
    let mut anchors = anchors(layout);
    anchors.sort_by(|a, b| a.1.1.total_cmp(&b.1.1).then(a.1.0.total_cmp(&b.1.0)));
    let mut placed = Vec::new();
    for right in [false, true] {
        let mut column: Vec<PlacedLabel> = anchors
            .iter()
            .filter(|a| a.2 == right)
            .filter_map(|(spot, (ax, ay), _)| {
                let label = text_for(*spot)?;
                Some(PlacedLabel { spot: *spot, text: fit_label(label), y: 0.0, right, anchor: (ax + MARGIN, *ay) })
            })
            .collect();
        let rows = column_rows(&column.iter().map(|l| l.anchor.1 - LABEL_HALF).collect::<Vec<_>>());
        for (label, y) in column.iter_mut().zip(rows) {
            label.y = y;
        }
        uncross(&mut column, &obstacles);
        placed.extend(column);
    }
    placed
}

/// Half a label's height: a label is centered on its line's end.
const LABEL_HALF: f32 = 10.0;
/// Where the first and last label rows may sit.
const FIRST_ROW: f32 = 4.0;
const LAST_ROW: f32 = HEIGHT - 2.0 * LABEL_HALF - 4.0;

/// Rows for labels that want to sit at `wanted` (sorted), at least a row apart and inside the
/// picture: pushed down past the one above, then back up from the bottom if they overflow.
fn column_rows(wanted: &[f32]) -> Vec<f32> {
    let mut rows: Vec<f32> = Vec::with_capacity(wanted.len());
    for &w in wanted {
        let below = rows.last().map_or(FIRST_ROW, |r| r + LABEL_SPACING);
        rows.push(w.max(below).max(FIRST_ROW));
    }
    let mut limit = LAST_ROW;
    for r in rows.iter_mut().rev() {
        *r = r.min(limit);
        limit = *r - LABEL_SPACING;
    }
    rows
}

/// A round part a leader line should keep clear of: x, y and radius, before the margin offset.
type Obstacle = (f32, f32, f32);

/// The round parts on the face of the controller that a leader line mustn't run across.
fn obstacles(l: &Layout) -> Vec<Obstacle> {
    let (fx, fy) = l.face;
    let mut all: Vec<Obstacle> = l.face_offsets().iter().map(|(ox, oy)| (fx + ox + MARGIN, fy + oy, l.face_r)).collect();
    all.push((l.dpad.0 + MARGIN, l.dpad.1, l.dpad_reach));
    all.push((l.left_stick.0 + MARGIN, l.left_stick.1, l.stick_r));
    all.push((l.right_stick.0 + MARGIN, l.right_stick.1, l.stick_r));
    all.extend([l.select, l.guide, l.start].map(|(x, y, r)| (x + MARGIN, y, r)));
    all
}

/// How far a line keeps from a part it doesn't name.
const CLEARANCE: f32 = 3.0;

/// Distance from `c` to the segment `a`–`b`.
fn distance_to_segment(c: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 { 0.0 } else { (((c.0 - a.0) * dx + (c.1 - a.1) * dy) / len2).clamp(0.0, 1.0) };
    (c.0 - (a.0 + t * dx)).hypot(c.1 - (a.1 + t * dy))
}

/// The leader line from `start` (a label's column edge) to `anchor`: the shortest path that keeps
/// clear of every other part, bending around them where it must (a straight line when nothing is
/// in the way). The part the anchor sits on is skipped, but the line may not leave it by crossing it.
fn route(start: Point, anchor: Point, obstacles: &[Obstacle]) -> Vec<Point> {
    /// Cost of each bend, so a path with fewer of them wins over one a little shorter.
    const BEND: f32 = 8.0;
    let own = |o: &Obstacle| (o.0 - anchor.0).hypot(o.1 - anchor.1) <= o.2 + CLEARANCE;
    let others: Vec<&Obstacle> = obstacles.iter().filter(|o| !own(o)).collect();
    let clear = |a: Point, b: Point| others.iter().all(|o| distance_to_segment((o.0, o.1), a, b) > o.2 + CLEARANCE);
    // The last step into the anchor must come from the side away from the part it sits on.
    let leaves_part = |to: Point| {
        obstacles.iter().filter(|o| own(o)).all(|o| (to.0 - anchor.0) * (anchor.0 - o.0) + (to.1 - anchor.1) * (anchor.1 - o.1) >= 0.0)
    };
    if clear(start, anchor) && leaves_part(start) {
        return vec![start, anchor];
    }
    // Nodes: the ends, and eight points around each part, just outside it.
    let mut nodes = vec![start, anchor];
    for o in obstacles {
        let reach = (o.2 + CLEARANCE + 3.0) * 1.08;
        nodes.extend((0..8).map(|k| {
            let angle = k as f32 * std::f32::consts::FRAC_PI_4;
            (o.0 + reach * angle.cos(), o.1 + reach * angle.sin())
        }));
    }
    let mut best = vec![f32::INFINITY; nodes.len()];
    let mut from = vec![usize::MAX; nodes.len()];
    let mut done = vec![false; nodes.len()];
    best[0] = 0.0;
    while let Some(i) = (0..nodes.len()).filter(|&i| !done[i] && best[i].is_finite()).min_by(|&a, &b| best[a].total_cmp(&best[b])) {
        done[i] = true;
        if i == 1 {
            break;
        }
        for j in 0..nodes.len() {
            if done[j] || j == 0 {
                continue;
            }
            let cost = best[i] + (nodes[j].0 - nodes[i].0).hypot(nodes[j].1 - nodes[i].1) + BEND;
            if cost < best[j] && clear(nodes[i], nodes[j]) && (j != 1 || leaves_part(nodes[i])) {
                best[j] = cost;
                from[j] = i;
            }
        }
    }
    if from[1] == usize::MAX {
        return vec![start, anchor];
    }
    let mut path = vec![anchor];
    let mut at = 1;
    while from[at] != usize::MAX {
        at = from[at];
        path.push(nodes[at]);
    }
    path.reverse();
    path
}

fn column_start(l: &PlacedLabel) -> Point {
    (if l.right { RIGHT_COLUMN_X } else { LABEL_COLUMN }, l.y + LABEL_HALF)
}

/// True if any pieces of the two paths cross.
fn paths_cross(a: &[Point], b: &[Point]) -> bool {
    a.windows(2).any(|p| b.windows(2).any(|q| crosses((p[0], p[1]), (q[0], q[1]))))
}

/// The labels' leader lines: how many pairs of them cross, and their total length.
fn tangle(column: &[PlacedLabel], obstacles: &[Obstacle]) -> (usize, f32) {
    let paths: Vec<Vec<Point>> = column.iter().map(|l| route(column_start(l), l.anchor, obstacles)).collect();
    let crossing = (0..paths.len()).flat_map(|i| (i + 1..paths.len()).map(move |j| (i, j))).filter(|&(i, j)| paths_cross(&paths[i], &paths[j])).count();
    let length = paths.iter().flat_map(|p| p.windows(2)).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
    (crossing, length)
}

/// Swaps the texts and anchors of labels, as long as a swap leaves fewer leader lines crossing (or
/// the same number and shorter lines in total), so none cross where an order exists without.
fn uncross(column: &mut [PlacedLabel], obstacles: &[Obstacle]) {
    let better = |a: (usize, f32), b: (usize, f32)| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1 - 0.5);
    let mut now = tangle(column, obstacles);
    while now.0 > 0 {
        let mut improved = false;
        for i in 0..column.len() {
            for j in i + 1..column.len() {
                swap_labels(column, i, j);
                let tried = tangle(column, obstacles);
                if better(tried, now) {
                    now = tried;
                    improved = true;
                } else {
                    swap_labels(column, i, j);
                }
            }
        }
        if !improved {
            break;
        }
    }
}

fn swap_labels(column: &mut [PlacedLabel], i: usize, j: usize) {
    let (a, b) = column.split_at_mut(j);
    std::mem::swap(&mut a[i].text, &mut b[0].text);
    std::mem::swap(&mut a[i].anchor, &mut b[0].anchor);
}

type Point = (f32, f32);

/// True if the segments properly cross (touching ends don't count).
fn crosses((p1, p2): (Point, Point), (q1, q2): (Point, Point)) -> bool {
    let side = |a: Point, b: Point, c: Point| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let (d1, d2) = (side(q1, q2, p1), side(q1, q2, p2));
    let (d3, d4) = (side(p1, p2, q1), side(p1, p2, q2));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Leader lines from each label's column edge to its part.
pub fn draw_labels(s: &mut String, labels: &[(Spot, String)], model: Option<PadModel>) {
    let obstacles = obstacles(Layout::of(model));
    for l in place_labels(labels, model) {
        let (ax, ay) = l.anchor;
        let points: Vec<String> = route(column_start(&l), l.anchor, &obstacles).iter().map(|(x, y)| format!("{x},{y}")).collect();
        let _ = write!(
            s,
            r#"<polyline points="{}" fill="none" stroke="{ACTIVE}" stroke-width="1.5" stroke-opacity="0.85" stroke-linejoin="round"/><circle cx="{ax}" cy="{ay}" r="2.5" fill="{ACTIVE}"/>"#,
            points.join(" ")
        );
    }
}

/// The leader lines alone, as a drawing the size of the whole picture: laid over the controller
/// so they keep full strength while the controller is dimmed.
pub fn leaders(labels: &[(Spot, String)], model: Option<PadModel>) -> String {
    let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}">"#);
    draw_labels(&mut s, labels, model);
    s.push_str("</svg>");
    s
}

/// Shortens `label` with an ellipsis so it fits on one line of its column. A quoted name that
/// is cut keeps its closing quote: “Menu “Augmentat…””, and a [`GESTURE_MARK`] stays at the end.
fn fit_label(label: &str) -> String {
    if let Some(rest) = label.strip_suffix(GESTURE_MARK) {
        let mark: f32 = GESTURE_MARK.chars().map(glyph_width).sum();
        return fit_within(rest, LABEL_TEXT_WIDTH / LABEL_TEXT_SIZE - mark) + GESTURE_MARK;
    }
    fit_within(label, LABEL_TEXT_WIDTH / LABEL_TEXT_SIZE)
}

/// [`fit_label`] for a `budget` in ems.
fn fit_within(label: &str, budget: f32) -> String {
    if label.chars().map(glyph_width).sum::<f32>() <= budget {
        return label.to_string();
    }
    // What follows the cut: the ellipsis, and the closing quote when the cut falls inside quotes.
    let tail = label.find('“').filter(|_| label.contains('”')).map_or("…", |_| "…”");
    let mut used: f32 = tail.chars().map(glyph_width).sum();
    let mut text: String = label
        .chars()
        .take_while(|&c| {
            used += glyph_width(c);
            used <= budget && c != '”'
        })
        .collect();
    text.truncate(text.trim_end().len());
    text.push_str(tail);
    text
}

/// Rough advance width of `c` in ems (measured on Noto Sans), erring wide so a label never
/// needs a second line.
fn glyph_width(c: char) -> f32 {
    match c {
        'i' | 'j' | 'l' | '.' | ',' | ':' | ';' | '\'' | '!' | ' ' => 0.32,
        'f' | 't' | 'r' | 'I' | '(' | ')' | '[' | ']' | '“' | '”' | '‘' | '’' | '"' | '-' => 0.45,
        'm' | 'w' | 'M' | 'W' | '@' | '%' | '…' => 0.95,
        c if c.is_ascii_lowercase() => 0.62,
        _ => 0.8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_fill_columns_top_down_and_truncate() {
        let labels = [
            (Spot::Button(Button::South), "Left click".to_string()),
            (Spot::Trigger(Trigger::Left), "A very long mapping description".to_string()),
            (Spot::Button(Button::DpadUp), "Up".to_string()),
        ];
        let placed = place_labels(&labels, None);
        let left: Vec<&str> = placed.iter().filter(|l| !l.right).map(|l| l.text.as_str()).collect();
        let right: Vec<&str> = placed.iter().filter(|l| l.right).map(|l| l.text.as_str()).collect();
        assert_eq!(left, ["A very long mappi…", "Up"]);
        assert_eq!(right, ["Left click"]);
        // Rows never overlap: the second label in a column sits at least a row below the first.
        let ys: Vec<f32> = placed.iter().filter(|l| !l.right).map(|l| l.y).collect();
        assert!(ys[1] - ys[0] >= LABEL_SPACING);
    }

    #[test]
    fn wide_letters_are_cut_sooner_than_narrow_ones() {
        assert_eq!(fit_label("Menu “Augmentations”"), "Menu “Augmentat…”", "a cut name keeps its closing quote");
        assert_eq!(fit_label("Menu “Belt” +"), "Menu “Belt” +");
        assert_eq!(fit_label("Menu “Augmentations” +"), "Menu “Augment…” +", "the gesture mark outlives the cut");
        assert_eq!(fit_label("Hold: fill all"), "Hold: fill all");
        assert_eq!(fit_label("Keypad PLUS"), "Keypad PLUS");
    }

    #[test]
    fn leader_lines_cross_rarely_even_with_every_input_mapped() {
        let every: Vec<(Spot, String)> = anchors(Layout::of(None)).into_iter().map(|(spot, ..)| (spot, "Mapped".to_string())).collect();
        for model in [None, Some(PadModel::ProController)] {
            let placed = place_labels(&every, model);
            let obstacles = obstacles(Layout::of(model));
            let lines: Vec<Vec<Point>> = placed.iter().map(|l| route(column_start(l), l.anchor, &obstacles)).collect();
            // With the Start line forced to pass the face buttons, one crossing can be unavoidable.
            let crossing = (0..lines.len()).flat_map(|i| (i + 1..lines.len()).map(move |j| (i, j))).filter(|&(i, j)| paths_cross(&lines[i], &lines[j])).count();
            assert!(crossing <= 1, "{model:?}: {crossing} pairs of leader lines cross");
            assert!(placed.iter().all(|a| (FIRST_ROW..=LAST_ROW).contains(&a.y)), "a row is outside the picture");
        }
    }

    #[test]
    fn leader_lines_of_a_typical_setup_do_not_cross() {
        use Button::{DpadDown, DpadLeft, DpadRight, DpadUp, LeftBumper, LeftStick, North, RightBumper, RightStick, Select, South, Start, West};
        let spots = [LeftBumper, RightBumper, Select, Start, LeftStick, RightStick, DpadUp, DpadDown, DpadLeft, DpadRight, South, West, North, Button::East];
        let labels: Vec<(Spot, String)> = spots.into_iter().map(|b| (Spot::Button(b), "Mapped".to_string())).collect();
        let obstacles = obstacles(Layout::of(None));
        let placed = place_labels(&labels, None);
        let lines: Vec<Vec<Point>> = placed.iter().map(|l| route(column_start(l), l.anchor, &obstacles)).collect();
        for i in 0..lines.len() {
            for j in i + 1..lines.len() {
                assert!(!paths_cross(&lines[i], &lines[j]), "{:?} crosses {:?}", placed[i].anchor, placed[j].anchor);
            }
        }
    }

    #[test]
    fn leader_lines_keep_clear_of_other_parts() {
        let every: Vec<(Spot, String)> = anchors(Layout::of(None)).into_iter().map(|(spot, ..)| (spot, "Mapped".to_string())).collect();
        for model in [None, Some(PadModel::ProController), Some(PadModel::DualSense), Some(PadModel::XboxSeries)] {
            let obstacles = obstacles(Layout::of(model));
            for l in place_labels(&every, model) {
                let path = route(column_start(&l), l.anchor, &obstacles);
                for o in obstacles.iter().filter(|o| (o.0 - l.anchor.0).hypot(o.1 - l.anchor.1) > o.2 + CLEARANCE) {
                    for w in path.windows(2) {
                        assert!(distance_to_segment((o.0, o.1), w[0], w[1]) > o.2, "{model:?}: line to {:?} crosses a part at {:?}", l.anchor, (o.0, o.1));
                    }
                }
            }
        }
    }

    #[test]
    fn lines_follow_the_models_layout() {
        let select = [(Spot::Button(Button::Select), "Map".to_string())];
        let anchor = |model| place_labels(&select, model)[0].anchor;
        assert_ne!(anchor(None), anchor(Some(PadModel::ProController)), "Minus sits elsewhere on a Pro Controller");
    }
}
