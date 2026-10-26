//! The media controls and the offer to add a library game (`draw.rs`'s `media_panel` and
//! `offer_panel`) with their numbers: what is playing, how far along and the volume, over the
//! controls' hint; the offer's question and detail, its answers as item boxes, and its hint.

#[cfg(test)]
mod tests;

use super::{
    Align, Item, Node, Style,
    glyphs::{bar_of, label, state_icon},
    rich,
    text::{Align as TextAlign, Spec},
};
use crate::{
    config::OverlayStyle,
    info::Glyphs,
    media::{self, MediaView},
    motion::Anim,
    offer::OfferView,
    overlay::fit::Fit,
};

/// The media controls, drawn as the overlay shows them: `font` is the family (`None` for the
/// default), `glyphs` the controller in use, and `anim` how far the panel has faded in or out.
pub fn media(view: &MediaView, font: Option<&'static str>, glyphs: Glyphs, fit: &Fit, anim: &Anim) -> Node {
    let style = panel_style(&view.style, font, glyphs, anim);
    let s = view.style.scale.clamp(0.5, 2.0);
    let width = 560.0 * s;
    let body = match &view.player {
        None => label(&style, "No media player is running", 18.0 * s, style.background_text),
        Some(player) => now_playing(view, player, &style, s, width),
    };
    let hint = Node::column(view.hint.lines().map(|line| rich(line, &style, 13.0 * s, style.muted))).spacing(4.0 * s).align_x(Align::Center);
    let contents = Node::column([body, hint]).spacing(12.0 * s).width(width).align_x(Align::Center);
    super::framed(contents, &style, &view.style, 18.0 * s, fit)
}

/// What is playing: the title and artist with the play state, the progress bar, and the times
/// and volumes, wrapping under the bar.
fn now_playing(view: &MediaView, player: &str, style: &Style, s: f32, width: f32) -> Node {
    let title = if view.title.is_empty() { "Nothing playing" } else { &view.title };
    let by = if view.artist.is_empty() { "Unknown artist" } else { &view.artist };
    let progress = if view.length_ms > 0 { (view.position_ms as f32 / view.length_ms as f32).clamp(0.0, 1.0) } else { 0.0 };
    let length = if view.length_ms > 0 { media::clock(view.length_ms) } else { "--:--".into() };
    let small = |t: &str| label(style, t, 14.0 * s, style.muted);
    let mut meta = vec![small(&format!("{} / {length}", media::clock(view.position_ms)))];
    // The system's sound, which the D-pad changes, then the player's own volume under the
    // player's name.
    match (view.system_volume, view.muted) {
        (Some(_), true) => meta.push(small("System muted")),
        (Some(v), false) => meta.push(small(&format!("System {:.0}%", v * 100.0))),
        (None, _) => {}
    }
    let own = match view.volume {
        Some(v) => format!("{player} {:.0}%", v * 100.0),
        None => player.to_string(),
    };
    meta.push(small(&own));
    if view.players > 1 {
        meta.push(small(&format!("{} players", view.players)));
    }
    let track = Node::row([
        state_icon(view.state, style.background_text, 28.0 * s),
        Node::column([label(style, title, 20.0 * s, style.background_text), label(style, by, 14.0 * s, style.muted)]).spacing(2.0 * s),
    ])
    .spacing(14.0 * s)
    .align_y(Align::Center);
    Node::column([track, bar_of(progress, width, 6.0 * s), Node::wrap(meta).spacing(16.0 * s).line_spacing(4.0 * s)]).spacing(10.0 * s)
}

/// The offer to add a library game: the question, what the pack is, its answers as boxes in a
/// row, and the hint. `{confirm}`'s answer is the default, so it has the selected look.
pub fn offer(view: &OfferView, font: Option<&'static str>, glyphs: Glyphs, fit: &Fit, anim: &Anim) -> Node {
    let style = panel_style(&view.style, font, glyphs, anim);
    let s = view.style.scale.clamp(0.5, 2.0);
    let answers = view.answers.iter().map(|(button, what)| answer(&format!("{button}  {what}"), &style, &view.style, button == "{confirm}", s));
    let detail = Node::text(Spec { family: style.font, align: TextAlign::Center, ..Spec::new(view.detail.clone(), 14.0 * s) }, style.muted).width(520.0 * s);
    let contents = Node::column([
        label(&style, &view.title, 20.0 * s, style.background_text),
        detail,
        Node::row(answers).spacing(10.0 * s),
        rich(&view.hint, &style, 12.0 * s, style.muted),
    ])
    .spacing(14.0 * s)
    .align_x(Align::Center);
    super::framed(contents, &style, &view.style, 22.0 * s, fit)
}

/// One answer: its text in a box rounded to half its height at full corners.
fn answer(text: &str, style: &Style, config: &OverlayStyle, selected: bool, s: f32) -> Node {
    let body = rich(text, style, 16.0 * s, style.item_text).padding([8.0 * s, 16.0 * s]);
    super::item(body, style, &Item { selected, ..Item::REST }, config, style.corners * 18.0 * s)
}

/// The panel's colors: the overlay's style with its glyphs, faded as the panel is.
fn panel_style(config: &OverlayStyle, font: Option<&'static str>, glyphs: Glyphs, anim: &Anim) -> Style {
    Style::of(config, font).with_glyphs(glyphs).faded(anim.opacity())
}
