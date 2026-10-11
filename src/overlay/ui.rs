//! The overlay window: a layer-shell surface drawing the frames the daemon streams.

use std::time::{Duration, Instant};

use iced::{
    Color, Element, Length, Size, Subscription, Task,
    futures::{SinkExt, channel::mpsc},
    widget::{column, responsive, space, stack},
};
use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings, StartMode},
    to_layer_message,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::{
    OverlayFrame, OverlayView, draw,
    fit::Fit,
    timing::{frame_summary, menu_log, timing_log, unix_ns},
};
use crate::{
    config::ScreenPosition,
    info::InfoView,
    inputlog::LogView,
    ipc::{self, Request},
    motion::{Anim, MotionSet, MotionStyle, Presence},
};

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    State(OverlayFrame),
    /// The daemon went away: nothing left to draw for.
    Disconnected,
    /// A frame of animation.
    Tick,
}

/// What is on screen, and what is fading out, each with its own timing.
struct Overlay {
    frame: OverlayFrame,
    panels: Presence<OverlayView>,
    info: Presence<InfoView>,
    logs: Presence<LogView>,
}

impl Overlay {
    fn new() -> Self {
        Overlay { frame: OverlayFrame::default(), panels: Presence::single(), info: Presence::default(), logs: Presence::default() }
    }

    /// Whether anything is drawn: something shown, or something still fading out.
    fn shown(&self) -> bool {
        !self.panels.is_empty() || !self.info.is_empty() || !self.logs.is_empty()
    }

    /// Whether something is still moving at `now`, so the screen needs redrawing.
    fn busy(&self, now: Instant) -> bool {
        let motion = &self.frame.motion;
        self.panels.busy(now, motion) || self.info.busy(now, motion) || self.logs.busy(now, motion)
    }

    /// Drops what has finished closing.
    fn prune(&mut self, now: Instant) {
        let motion = &self.frame.motion;
        self.panels.prune(now, motion);
        self.info.prune(now, motion);
        self.logs.prune(now, motion);
    }
}

/// Idle: a 1×1 invisible surface in a corner. Showing: the whole screen, transparent
/// except for what is drawn, with clicks passing through.
fn idle() -> (Anchor, (u32, u32)) {
    (Anchor::Top | Anchor::Right, (1, 1))
}

fn full_screen() -> (Anchor, (u32, u32)) {
    (Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right, (0, 0))
}

pub fn run() -> anyhow::Result<()> {
    let (anchor, size) = idle();
    let app = iced_layershell::application(boot, namespace, update, view)
        .style(|_, _| iced::theme::Style { background_color: Color::TRANSPARENT, text_color: Color::WHITE });
    crate::font::BUNDLED
        .iter()
        .fold(app, |app, b| app.font(b.bytes))
        .subscription(subscription)
        .settings(Settings {
            layer_settings: LayerShellSettings {
                size: Some(size),
                exclusive_zone: -1,
                anchor,
                layer: Layer::Overlay,
                margin: (0, 0, 0, 0),
                // Never take keyboard focus: typed keys must reach the window underneath.
                keyboard_interactivity: KeyboardInteractivity::None,
                events_transparent: true,
                start_mode: StartMode::Active,
            },
            ..Default::default()
        })
        .run()
        .map_err(|e| anyhow::anyhow!("overlay: {e}"))
}

fn boot() -> (Overlay, Task<Message>) {
    (Overlay::new(), Task::none())
}

fn namespace() -> String {
    "padwight_overlay".into()
}

fn update(state: &mut Overlay, message: Message) -> Task<Message> {
    match message {
        Message::State(mut frame) => {
            timing_log(&format!("recv {} {}", unix_ns(), frame_summary(&frame)));
            frame.apply_lite();
            let now = Instant::now();
            let was_shown = state.shown();
            state.panels.update(now, frame.active.iter().cloned().collect());
            state.info.update(now, frame.info.clone());
            state.logs.update(now, frame.logs.clone());
            state.frame = frame;
            state.prune(now);
            return resize(was_shown, state.shown());
        }
        Message::Tick => {
            let was_shown = state.shown();
            state.prune(Instant::now());
            return resize(was_shown, state.shown());
        }
        Message::Disconnected => return iced::exit(),
        _ => {}
    }
    Task::none()
}

/// Goes to the whole screen when something starts to show, and back to the corner once
/// nothing is left to draw.
fn resize(was_shown: bool, shown: bool) -> Task<Message> {
    let (anchor, size) = match (was_shown, shown) {
        (false, true) => full_screen(),
        (true, false) => idle(),
        _ => return Task::none(),
    };
    Task::done(Message::AnchorSizeChange(anchor, size))
}

fn subscription(state: &Overlay) -> Subscription<Message> {
    let watch = Subscription::run(watch);
    if state.busy(Instant::now()) {
        Subscription::batch([watch, iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)])
    } else {
        watch
    }
}

fn watch() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, async |mut output: mpsc::Sender<Message>| {
        let mut request = serde_json::to_string(&Request::WatchOverlay).unwrap_or_default();
        request.push('\n');
        if let Ok(mut stream) = tokio::net::UnixStream::connect(ipc::socket_path()).await
            && stream.write_all(request.as_bytes()).await.is_ok()
        {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(state) = serde_json::from_str::<OverlayFrame>(&line) {
                    let _ = output.send(Message::State(state)).await;
                }
            }
        }
        let _ = output.send(Message::Disconnected).await;
        // Keep the stream alive until iced exits.
        tokio::time::sleep(Duration::from_secs(3600)).await;
    })
}

fn view(state: &Overlay) -> Element<'_, Message> {
    if !state.shown() {
        return space().into();
    }
    // The surface covers the whole display, so each overlay sizes itself as shares of it.
    responsive(move |display| {
        let started = Instant::now();
        let built: Element<'_, Message> = stack(layers(state, display, Instant::now())).into();
        timing_log(&format!("built {} {} {}x{}", unix_ns(), started.elapsed().as_nanos(), display.width, display.height));
        built
    })
    .into()
}

/// The overlays on screen at `now`, sized for a display of `display`.
fn layers(state: &Overlay, display: Size, now: Instant) -> Vec<Element<'_, Message>> {
    let motion = &state.frame.motion;
    let font = crate::font::resolve(state.frame.font.as_deref());
    // Info and log overlays sharing a spot stack up there; the keyboard or a menu goes on top.
    let mut layers: Vec<Element<'_, Message>> = Vec::new();
    for spot in ScreenPosition::GRID {
        let infos: Vec<_> = state.info.shown(now, motion).filter(|(v, _)| v.style.position == spot).collect();
        let logs: Vec<_> = state.logs.shown(now, motion).filter(|(v, _)| v.style.position == spot).collect();
        let style = infos.first().map(|(v, _)| &v.style).or_else(|| logs.first().map(|(v, _)| &v.style));
        if let Some(style) = style {
            let panels = infos
                .iter()
                .map(|(v, anim)| draw::info_panel(v, font, &Fit::of(&v.style, display), anim))
                .chain(logs.iter().map(|(v, anim)| draw::log_panel(v, font, &Fit::of(&v.style, display), anim)))
                .collect::<Vec<Element<'_, Message>>>();
            let fit = Fit::of(style, display);
            layers.push(draw::place(column(panels).spacing(12).into(), style, &fit, &Anim::still()));
        }
    }
    let glyphs = crate::info::Glyphs { family: state.frame.family, nintendo_layout: state.frame.nintendo_layout };
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
                draw::place(draw::menu_panel(m, font, look, &fit), &m.style, &fit, &anim)
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

/// Draws `frame` as the overlay window shows it once everything has finished opening, on a
/// screen of `display` (logical pixels) over `background`: no window or compositor needed.
pub fn snapshot(mut frame: OverlayFrame, display: Size, scale: f32, background: Color) -> anyhow::Result<crate::snapshot::Image> {
    frame.motion = MotionSet::all(MotionStyle::Off);
    let mut state = Overlay::new();
    let _ = update(&mut state, Message::State(frame));
    // Well after now, so nothing is still on its way in.
    let settled = Instant::now() + Duration::from_secs(10);
    let screen = stack(layers(&state, display, settled)).width(Length::Fill).height(Length::Fill);
    crate::snapshot::render(screen.into(), display, scale, background)
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
            style: Default::default(),
            touch: false,
            aim: None,
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
        let _ = update(&mut state, Message::State(menu_frame(MotionStyle::Subtle)));
        assert!(state.shown());
        let _ = update(&mut state, Message::State(OverlayFrame { motion: menu_frame(MotionStyle::Subtle).motion, ..Default::default() }));
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
        let _ = update(&mut state, Message::State(menu_frame(MotionStyle::Off)));
        let mut picked = menu_frame(MotionStyle::Off);
        if let Some(OverlayView::Menu(m)) = &mut picked.active {
            m.picks = 1;
        }
        let _ = update(&mut state, Message::State(picked));
        let _ = update(&mut state, Message::State(OverlayFrame { motion: menu_frame(MotionStyle::Off).motion, ..Default::default() }));
        assert!(state.shown(), "held so the pick is seen");
        let later = Instant::now() + Duration::from_secs_f32(crate::motion::PICK_HOLD + 0.05);
        state.prune(later);
        assert!(!state.shown(), "then gone");
    }

    #[test]
    fn with_motion_off_a_closed_menu_goes_at_once() {
        let mut state = Overlay::new();
        let _ = update(&mut state, Message::State(menu_frame(MotionStyle::Off)));
        let closing = OverlayFrame { motion: menu_frame(MotionStyle::Off).motion, ..Default::default() };
        let _ = update(&mut state, Message::State(closing));
        assert!(!state.shown());
    }
}
