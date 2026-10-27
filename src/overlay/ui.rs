//! What the overlay window shows: the frames the daemon streams, and whatever is still fading
//! out, drawn as layers over the screen. The window around it is a layer-shell surface on Linux
//! (`layershell.rs`) and a layered window on Windows (`windows.rs`).

use std::time::{Duration, Instant};

use iced::{
    Color, Element, Length, Size,
    widget::{column, stack},
};

#[cfg(target_os = "linux")]
use super::render::Shown;
use super::{
    OverlayFrame, OverlayView, draw,
    fit::Fit,
    timing::{frame_summary, menu_log, timing_log, unix_ns},
};
use crate::{
    config::ScreenPosition,
    info::InfoView,
    inputlog::LogView,
    motion::{Anim, MotionSet, MotionStyle, Presence},
};

/// What is on screen, and what is fading out, each with its own timing.
pub(super) struct Overlay {
    frame: OverlayFrame,
    panels: Presence<OverlayView>,
    info: Presence<InfoView>,
    logs: Presence<LogView>,
}

impl Overlay {
    pub(super) fn new() -> Self {
        Overlay { frame: OverlayFrame::default(), panels: Presence::single(), info: Presence::default(), logs: Presence::default() }
    }

    /// Whether anything is drawn: something shown, or something still fading out.
    pub(super) fn shown(&self) -> bool {
        !self.panels.is_empty() || !self.info.is_empty() || !self.logs.is_empty()
    }

    /// Whether something is still moving at `now`, so the screen needs redrawing.
    pub(super) fn busy(&self, now: Instant) -> bool {
        let motion = &self.frame.motion;
        self.frame.loops() || self.panels.busy(now, motion) || self.info.busy(now, motion) || self.logs.busy(now, motion)
    }

    /// The daemon's latest frame, as received.
    #[cfg(target_os = "linux")]
    pub(super) fn frame(&self) -> &OverlayFrame {
        &self.frame
    }

    /// What is on screen at `now`, each panel with how it moves, for the overlay's own renderer
    /// (`render::screen_ops` with `frame`): what `layers` draws for iced. Each menu's `menu`
    /// line goes in the timing log, as `layers` writes it.
    #[cfg(target_os = "linux")]
    pub(super) fn shown_at(&self, now: Instant) -> Shown<'_> {
        let motion = &self.frame.motion;
        let panels: Vec<_> = self.panels.shown(now, motion).collect();
        for (view, anim) in &panels {
            if let OverlayView::Menu(m) = view {
                menu_log(m, anim);
            }
        }
        Shown { panels, info: self.info.shown(now, motion).collect(), logs: self.logs.shown(now, motion).collect() }
    }

    /// Drops what has finished closing.
    pub(super) fn prune(&mut self, now: Instant) {
        let motion = &self.frame.motion;
        self.panels.prune(now, motion);
        self.info.prune(now, motion);
        self.logs.prune(now, motion);
    }

    /// Takes the daemon's latest frame: what opens starts its way in, what closes its way out.
    pub(super) fn receive(&mut self, frame: OverlayFrame) {
        self.receive_at(frame, Instant::now());
    }

    /// Takes a frame as though it came at `now`.
    fn receive_at(&mut self, mut frame: OverlayFrame, now: Instant) {
        timing_log(&format!("recv {} {}", unix_ns(), frame_summary(&frame)));
        frame.apply_lite();
        self.panels.update(now, frame.active.iter().cloned().collect());
        self.info.update(now, frame.info.clone());
        self.logs.update(now, frame.logs.clone());
        crate::info::set_opener(frame.opener);
        crate::info::set_menu_keys(frame.menu_keys);
        crate::info::set_panel_keys(&frame.panel_keys);
        if let Some(icons) = &frame.pack_icons {
            crate::icons::set_pack_icons(icons);
        }
        crate::font::set_pack_font_files(&frame.pack_fonts);
        self.frame = frame;
        self.prune(now);
    }
}

/// The menu being edited, which sits under the editor.
fn backdrop_layer<'a, M: 'a>(state: &'a Overlay, font: iced::Font, display: Size) -> Option<Element<'a, M>> {
    let m = state.frame.backdrop.as_ref()?;
    let look = draw::MenuLook { colorblind: state.frame.colorblind, family: state.frame.family, nintendo_layout: state.frame.nintendo_layout, anim: Anim::still() };
    let fit = Fit::of(&m.style, display);
    Some(draw::place(draw::menu_panel(m, font, look, &fit), &m.style, &fit, &Anim::still()))
}

/// The overlays on screen at `now`, sized for a display of `display`.
pub(super) fn layers<'a, M: 'a>(state: &'a Overlay, display: Size, now: Instant) -> Vec<Element<'a, M>> {
    let motion = &state.frame.motion;
    let font = crate::font::resolve(state.frame.font.as_deref());
    // The keyboard or a menu goes on top of the info and log overlays.
    let guide = guide_up(state, display, now);
    let (mut layers, [mut left, mut right]) = stacks(state, font, display, now, guide);
    let glyphs = crate::info::Glyphs { family: state.frame.family, nintendo_layout: state.frame.nintendo_layout };
    layers.extend(backdrop_layer(state, font, display));
    if state.frame.peek {
        return layers;
    }
    for (view, anim) in state.panels.shown(now, motion) {
        layers.push(match view {
            OverlayView::Keyboard(k) => {
                let fit = Fit::of(&k.style, display);
                draw::place(draw::keyboard_panel(k, font, glyphs, &fit, &anim), &k.style, &fit, &anim)
            }
            OverlayView::Menu(m) => {
                let look = draw::MenuLook {
                    colorblind: state.frame.colorblind,
                    family: state.frame.family,
                    nintendo_layout: state.frame.nintendo_layout,
                    anim,
                };
                menu_log(m, &anim);
                let fit = Fit::of(&m.style, display);
                let panel = draw::menu_panel(m, font, look, &fit);
                let menu = if guide { draw::beside(left.take(), panel, right.take(), &anim) } else { draw::place(panel, &m.style, &fit, &anim) };
                // A question asked over the menu stands on top of it, in the middle of the screen.
                match &m.prompt {
                    Some(o) => {
                        let fit = Fit::of(&o.style, display);
                        let prompt = draw::place(draw::offer_panel(o, font, glyphs, &fit, &Anim::still()), &o.style, &fit, &Anim::still());
                        stack![menu, prompt].into()
                    }
                    None => menu,
                }
            }
            OverlayView::Media(m) => {
                let fit = Fit::of(&m.style, display);
                draw::place(draw::media_panel(m, font, glyphs, &fit, &anim), &m.style, &fit, &anim)
            }
            OverlayView::Offer(o) => {
                let fit = Fit::of(&o.style, display);
                draw::place(draw::offer_panel(o, font, glyphs, &fit, &anim), &o.style, &fit, &anim)
            }
        });
    }
    layers
}

/// The info and log overlays, those sharing a spot stacked up there. With the Guide overlay up,
/// the stacks at the middle of the left and right edges (its Notes and Mappings) are handed back
/// instead, to stand in a row with its menu.
fn stacks<'a, M: 'a>(state: &'a Overlay, font: iced::Font, display: Size, now: Instant, guide: bool) -> (Vec<Element<'a, M>>, [Option<Element<'a, M>>; 2]) {
    let motion = &state.frame.motion;
    let mut layers = Vec::new();
    let mut sides = [None, None];
    for spot in ScreenPosition::GRID {
        let infos: Vec<_> = state.info.shown(now, motion).filter(|(v, _)| v.style.position == spot).collect();
        let logs: Vec<_> = state.logs.shown(now, motion).filter(|(v, _)| v.style.position == spot).collect();
        let style = infos.first().map(|(v, _)| &v.style).or_else(|| logs.first().map(|(v, _)| &v.style));
        let Some(style) = style else { continue };
        let panels = infos
            .iter()
            .map(|(v, anim)| draw::info_panel(v, font, &Fit::of(&v.style, display), anim))
            .chain(logs.iter().map(|(v, anim)| draw::log_panel(v, font, &Fit::of(&v.style, display), anim)))
            .collect::<Vec<Element<'_, M>>>();
        let stack: Element<'_, M> = column(panels).spacing(12).into();
        match spot {
            ScreenPosition::CenterLeft if guide => sides[0] = Some(stack),
            ScreenPosition::CenterRight if guide => sides[1] = Some(stack),
            _ => layers.push(draw::place(stack, style, &Fit::of(style, display), &Anim::still())),
        }
    }
    (layers, sides)
}

/// Whether the Guide overlay is up with its menu alone in the middle of the screen, its size and
/// place the style's own, so Notes and Mappings can line up beside it.
fn guide_up(state: &Overlay, display: Size, now: Instant) -> bool {
    let motion = &state.frame.motion;
    let mut panels = state.panels.shown(now, motion);
    let (Some((OverlayView::Menu(menu), _)), None) = (panels.next(), panels.next()) else { return false };
    let fit = Fit::of(&menu.style, display);
    !state.frame.peek
        && menu.style.position == ScreenPosition::Center
        && (fit.width, fit.height, fit.x, fit.y) == (None, None, 0.0, 0.0)
        && state.info.shown(now, motion).any(|(v, _)| crate::info::is_guide_panel(&v.name))
}

/// Draws `frame` as the overlay window shows it once everything has finished opening, on a
/// screen of `display` (logical pixels) over `background`: no window or compositor needed.
pub fn snapshot(mut frame: OverlayFrame, display: Size, scale: f32, background: Color) -> anyhow::Result<crate::snapshot::Image> {
    frame.motion = MotionSet::all(MotionStyle::Off);
    let mut state = Overlay::new();
    state.receive(frame);
    // Well after now, so nothing is still on its way in.
    let settled = Instant::now() + Duration::from_secs(10);
    let screen = stack(layers::<()>(&state, display, settled)).width(Length::Fill).height(Length::Fill);
    crate::snapshot::render(screen.into(), display, scale, background)
}

/// Draws a run of frames as the overlay window shows it, with no window: each frame in `script`
/// arrives that many seconds into the run, and a picture is drawn at each time in `at`.
pub fn film(script: &[(f32, OverlayFrame)], at: &[f32], display: Size, scale: f32, background: Color) -> anyhow::Result<Vec<crate::snapshot::Image>> {
    let start = Instant::now();
    let when = |secs: f32| start + Duration::from_secs_f32(secs.max(0.0));
    let mut state = Overlay::new();
    let mut next = script.iter().peekable();
    let mut pictures = Vec::with_capacity(at.len());
    for &t in at {
        while let Some((arrives, frame)) = next.next_if(|(arrives, _)| *arrives <= t) {
            state.receive_at(frame.clone(), when(*arrives));
        }
        state.prune(when(t));
        let screen = stack(layers::<()>(&state, display, when(t))).width(Length::Fill).height(Length::Fill);
        pictures.push(crate::snapshot::render(screen.into(), display, scale, background)?);
    }
    Ok(pictures)
}

#[cfg(test)]
mod motion_tests {
    use super::*;
    use crate::{config::MenuKind, menu::MenuView, motion::MotionStyle};

    fn menu_frame(motion: MotionStyle) -> OverlayFrame {
        let view = MenuView {
            title: "Belt".into(),
            kind: MenuKind::List,
            items: Vec::new(),
            selected: Some(0),
            picks: 0,
            crumbs: Vec::new(),
            hint: String::new(),
            about: String::new(),
            style: Default::default(),
            touch: false,
            aim: None,
            prompt: None,
            one_column: false,
            figure: None,
        };
        OverlayFrame {
            active: Some(OverlayView::Menu(view)),
            motion: crate::motion::MotionSet { menu: motion, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn a_menu_that_closes_stays_up_for_its_close() {
        let mut state = Overlay::new();
        state.receive(menu_frame(MotionStyle::Subtle));
        assert!(state.shown());
        state.receive(OverlayFrame { motion: menu_frame(MotionStyle::Subtle).motion, ..Default::default() });
        assert!(state.shown(), "fading out, so the surface stays up");
        assert!(state.busy(Instant::now()));
    }

    #[test]
    fn a_frame_stands_in_for_an_older_one_only_when_nothing_opened_closed_or_was_picked() {
        let open = menu_frame(MotionStyle::Subtle);
        let mut moved = open.clone();
        let mut picked = open.clone();
        if let (Some(OverlayView::Menu(m)), Some(OverlayView::Menu(p))) = (&mut moved.active, &mut picked.active) {
            m.selected = Some(1);
            p.picks = 1;
        }
        assert!(moved.stands_in_for(&open), "a cursor move can be skipped in a burst");
        assert!(!picked.stands_in_for(&open), "the frame before a pick still shows");
        assert!(!OverlayFrame::default().stands_in_for(&picked), "a pick isn't lost to the close after it");
        assert!(!open.stands_in_for(&OverlayFrame::default()), "nor an opening");
    }

    #[test]
    fn a_menu_closed_by_a_pick_holds_its_pick_a_moment_even_with_motion_off() {
        let mut state = Overlay::new();
        state.receive(menu_frame(MotionStyle::Off));
        let mut picked = menu_frame(MotionStyle::Off);
        if let Some(OverlayView::Menu(m)) = &mut picked.active {
            m.picks = 1;
        }
        state.receive(picked);
        state.receive(OverlayFrame { motion: menu_frame(MotionStyle::Off).motion, ..Default::default() });
        assert!(state.shown(), "held so the pick is seen");
        let later = Instant::now() + Duration::from_secs_f32(crate::motion::PICK_HOLD + 0.05);
        state.prune(later);
        assert!(!state.shown(), "then gone");
    }

    #[test]
    fn with_motion_off_a_closed_menu_goes_at_once() {
        let mut state = Overlay::new();
        state.receive(menu_frame(MotionStyle::Off));
        let closing = OverlayFrame { motion: menu_frame(MotionStyle::Off).motion, ..Default::default() };
        state.receive(closing);
        assert!(!state.shown());
    }
}
