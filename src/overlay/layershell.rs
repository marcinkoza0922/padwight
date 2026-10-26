//! The overlay window on Linux: a layer-shell surface drawing the frames the daemon streams.

use std::time::{Duration, Instant};

use iced::{
    Color, Element, Subscription, Task,
    futures::{SinkExt, channel::mpsc},
    widget::{responsive, space, stack},
};
use iced_layershell::{
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings, StartMode},
    to_layer_message,
};

use super::{
    OverlayFrame,
    timing::{present_log, timing_enabled, timing_log, unix_ns},
    ui::{Overlay, layers},
};
use crate::ipc::{self, Request};

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    State(OverlayFrame),
    /// The daemon went away: nothing left to draw for.
    Disconnected,
    /// A frame of animation.
    Tick,
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
        Message::State(frame) => {
            let was_shown = state.shown();
            state.receive(frame);
            return resize(was_shown, state.shown());
        }
        Message::Tick => {
            timing_log(&format!("tick {}", unix_ns()));
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
    let mut watch = Subscription::run(watch);
    if timing_enabled() {
        // Logs each redraw, to see when frames are shown; makes no messages.
        let redraws = iced::event::listen_raw(|event, _, _| {
            if let iced::Event::Window(iced::window::Event::RedrawRequested(at)) = event {
                present_log(at);
            }
            None
        });
        watch = Subscription::batch([watch, redraws]);
    }
    if state.busy(Instant::now()) {
        Subscription::batch([watch, iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)])
    } else {
        watch
    }
}

fn watch() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, async |mut output: mpsc::Sender<Message>| {
        if let Ok(mut lines) = ipc::watch(&Request::WatchOverlay) {
            while let Some(line) = iced::futures::StreamExt::next(&mut lines).await {
                if let Ok(state) = serde_json::from_str::<OverlayFrame>(&line) {
                    let _ = output.send(Message::State(state)).await;
                }
            }
        }
        let _ = output.send(Message::Disconnected).await;
        // Keep the stream alive until iced exits.
        iced::futures::future::pending::<()>().await;
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

