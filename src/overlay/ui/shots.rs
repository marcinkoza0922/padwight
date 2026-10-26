//! The overlay window's pictures without a window: one screen as it looks once everything has
//! opened, and a run of frames at chosen times, both drawn by the overlay's own renderer.

use std::time::{Duration, Instant};

use iced::{Color, Size};

use super::Overlay;
use crate::{
    motion::{MotionSet, MotionStyle},
    overlay::{
        OverlayFrame,
        render::{Display, Point, Screen},
    },
    snapshot::Image,
};

/// `state` at `now`, drawn on a `display` over the opaque `background`.
fn picture(screen: &mut Screen, state: &Overlay, now: Instant, display: Display, background: Color) -> anyhow::Result<Image> {
    let (size, scale) = (display.size, display.scale);
    let (w, h) = ((size.x * scale).round() as u32, (size.y * scale).round() as u32);
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(|| anyhow::anyhow!("a {w}x{h} picture can't be made"))?;
    pixmap.fill(tiny_skia::Color::from_rgba(background.r, background.g, background.b, 1.0).unwrap_or(tiny_skia::Color::BLACK));
    screen.draw(&state.frame, &state.showing(now), display, &mut pixmap.as_mut());
    let rgba = pixmap.data().as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    Ok(Image { width: w, height: h, rgba })
}

/// Draws `frame` as the overlay window shows it once everything has finished opening, on a
/// screen of `display` (logical pixels) over `background`: no window or compositor needed.
pub fn snapshot(mut frame: OverlayFrame, display: Size, scale: f32, background: Color) -> anyhow::Result<Image> {
    frame.motion = MotionSet::all(MotionStyle::Off);
    let mut state = Overlay::new();
    state.receive(frame);
    // Well after now, so nothing is still on its way in.
    let settled = Instant::now() + Duration::from_secs(10);
    picture(&mut Screen::new(), &state, settled, shown(display, scale), background)
}

/// Draws a run of frames as the overlay window shows it, with no window: each frame in `script`
/// arrives that many seconds into the run, and a picture is drawn at each time in `at`.
pub fn film(script: &[(f32, OverlayFrame)], at: &[f32], display: Size, scale: f32, background: Color) -> anyhow::Result<Vec<Image>> {
    let start = Instant::now();
    let when = |secs: f32| start + Duration::from_secs_f32(secs.max(0.0));
    let mut state = Overlay::new();
    let mut screen = Screen::new();
    let mut next = script.iter().peekable();
    let mut pictures = Vec::with_capacity(at.len());
    for &t in at {
        while let Some((arrives, frame)) = next.next_if(|(arrives, _)| *arrives <= t) {
            state.receive_at(frame.clone(), when(*arrives));
        }
        state.prune(when(t));
        pictures.push(picture(&mut screen, &state, when(t), shown(display, scale), background)?);
    }
    Ok(pictures)
}

/// A `display` of logical pixels drawn at `scale`.
fn shown(display: Size, scale: f32) -> Display {
    Display { size: Point::new(display.width, display.height), scale }
}
