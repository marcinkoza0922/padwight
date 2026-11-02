//! Checks that come from colors alone, not from a laid-out window: the theme's color pairs, and
//! the menu tones with and without color-blind mode.

use iced::{Color, Rectangle};

use super::{
    Finding, Kind,
    contrast::{deuteranope, delta_e, ratio},
    scan::hex,
};
use crate::config::OverlayStyle;

/// Two tones that look alike under deuteranopia below this ΔE merge at a glance.
const APART: f32 = 10.0;
const AA: f32 = 4.5;

fn finding(scene: &str, theme: &'static str, kind: Kind, name: &str, detail: String) -> Finding {
    Finding::new(scene, theme, kind, name, detail, Rectangle::with_size(iced::Size::ZERO))
}

/// Every text/background pair the window's theme defines.
pub fn theme(dark: bool, name: &'static str) -> Vec<Finding> {
    let theme = <iced::Theme as iced::theme::Base>::default(if dark { iced::theme::Mode::Dark } else { iced::theme::Mode::Light });
    let p = theme.extended_palette();
    let pair = |n: &str, pair: iced::theme::palette::Pair| (n.to_string(), pair.text, pair.color);
    let mut pairs = vec![];
    for (n, bg) in [
        ("background weakest", p.background.weakest),
        ("background weak", p.background.weak),
        ("background base", p.background.base),
        ("background strong", p.background.strong),
        ("background stronger", p.background.stronger),
        ("background strongest", p.background.strongest),
    ] {
        pairs.push(pair(n, bg));
    }
    macro_rules! levels {
        ($($n:literal $set:expr),*) => {$(
            for (level, pr) in [("weak", $set.weak), ("base", $set.base), ("strong", $set.strong)] {
                pairs.push(pair(&format!("{} {level}", $n), pr));
            }
        )*};
    }
    levels!("primary" p.primary, "secondary" p.secondary, "success" p.success, "warning" p.warning, "danger" p.danger);
    pairs
        .into_iter()
        .filter_map(|(n, text, bg)| {
            let got = ratio(text, bg);
            (got < AA).then(|| finding("theme palette", name, Kind::Palette, &n, format!("{got:.2}:1 (needs {AA}); {} on {}", hex(text), hex(bg))))
        })
        .collect()
}

/// The menu's row tones: text contrast on each, and how far apart they look to a deuteranope. With
/// color-blind mode off the pairs are shown for comparison only; they are not failures.
pub fn colorblind() -> Vec<Finding> {
    let style = OverlayStyle::default();
    let mut out = Vec::new();
    for on in [true, false] {
        let (scene, theme) = (if on { "menu colors, color-blind mode" } else { "menu colors, normal mode (for comparison)" }, "-");
        let swatches = crate::overlay::draw::tone_swatches(&style, on);
        let mut note = |kind, name: &str, detail: String| {
            if on {
                out.push(finding(scene, theme, kind, name, detail));
            }
        };
        for (name, item, text) in &swatches {
            let got = ratio(*text, *item);
            if got < AA {
                note(Kind::Colorblind, name, format!("text {got:.2}:1 (needs {AA}); {} on {}", hex(*text), hex(*item)));
            }
        }
        let tone = |n: &str| swatches.iter().find(|s| s.0 == n).map(|s| s.1);
        let pairs = [("normal row", "adding row"), ("normal row", "removing row"), ("adding row", "removing row"), ("panel", "normal row"), ("normal row", "selected row"), ("adding row", "selected row"), ("removing row", "selected row")];
        for (a, b) in pairs {
            let (Some(ca), Some(cb)) = (tone(a), tone(b)) else { continue };
            let apart = delta_e(deuteranope(opaque(ca)), deuteranope(opaque(cb)));
            if apart < APART {
                note(Kind::Colorblind, &format!("{a} / {b}"), format!("look alike to a deuteranope: delta E {apart:.1} (needs {APART}); normal vision sees {:.1}", delta_e(opaque(ca), opaque(cb))));
            }
        }
    }
    out
}

fn opaque(c: Color) -> Color {
    Color { a: 1.0, ..c }
}
