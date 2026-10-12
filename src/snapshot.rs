//! Drawing iced elements into images with no window and no display: the CPU renderer
//! (tiny-skia) draws offscreen, the same widgets the overlay and the windows draw. For checking
//! what the overlay looks like in tests and from `padwight debug overlay`.

use std::{path::Path, time::Instant};

use anyhow::{Context, Result};
use iced::{
    Color, Element, Event, Font, Pixels, Size, Theme,
    advanced::{
        clipboard,
        graphics::text::font_system,
        mouse,
        renderer::{self, Headless},
    },
    window,
};
use iced_runtime::user_interface::{Cache, UserInterface};

/// A drawn image: `width` × `height` pixels, four bytes each (RGBA).
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = 4 * (y * self.width + x) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    /// How many pixels differ from `color`: zero when nothing was drawn on the background.
    #[cfg(test)]
    pub fn drawn(&self, [r, g, b]: [u8; 3]) -> usize {
        self.rgba.as_chunks::<4>().0.iter().filter(|p| p[..3] != [r, g, b]).count()
    }

    /// The part of the image with something drawn on `background`, with `margin` pixels of
    /// background kept around it. A blank image stays as it is.
    pub fn trimmed(&self, [r, g, b]: [u8; 3], margin: u32) -> Image {
        let drawn = |x: u32, y: u32| self.pixel(x, y)[..3] != [r, g, b];
        let rows: Vec<u32> = (0..self.height).filter(|&y| (0..self.width).any(|x| drawn(x, y))).collect();
        let cols: Vec<u32> = (0..self.width).filter(|&x| (0..self.height).any(|y| drawn(x, y))).collect();
        let (Some(&top), Some(&bottom), Some(&left), Some(&right)) = (rows.first(), rows.last(), cols.first(), cols.last()) else {
            return Image { width: self.width, height: self.height, rgba: self.rgba.clone() };
        };
        let (x0, y0) = (left.saturating_sub(margin), top.saturating_sub(margin));
        let (x1, y1) = ((right + margin).min(self.width - 1), (bottom + margin).min(self.height - 1));
        let width = x1 - x0 + 1;
        let rgba = (y0..=y1).flat_map(|y| {
            let start = 4 * (y * self.width + x0) as usize;
            self.rgba[start..start + 4 * width as usize].iter().copied()
        });
        Image { width, height: y1 - y0 + 1, rgba: rgba.collect() }
    }

    /// Writes the image as a PNG, making its folder if need be.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).with_context(|| format!("cannot make {}", dir.display()))?;
        }
        let file = std::fs::File::create(path).with_context(|| format!("cannot write {}", path.display()))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.rgba)?;
        writer.finish()?;
        Ok(())
    }
}

/// Draws `element` on a `size` screen (logical pixels) over `background`, at `scale` physical
/// pixels per logical one. Text is in the bundled fonts or the system's, as on screen.
pub fn render<M>(element: Element<'_, M>, size: Size, scale: f32, background: Color) -> Result<Image> {
    render_themed(element, size, scale, background, &Theme::Dark)
}

/// Like `render`, with the widgets styled for `theme`.
pub fn render_themed<M>(element: Element<'_, M>, size: Size, scale: f32, background: Color, theme: &Theme) -> Result<Image> {
    load_fonts();
    let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")))
        .context("the offscreen renderer did not start")?;
    let mut ui = UserInterface::build(element, size, Cache::default(), &mut renderer);
    // A redraw request lets widgets that animate or measure on redraw settle first.
    let _ = ui.update(&[Event::Window(window::Event::RedrawRequested(Instant::now()))], mouse::Cursor::Unavailable, &mut renderer, &mut clipboard::Null, &mut Vec::new());
    ui.draw(&mut renderer, theme, &renderer::Style { text_color: theme.palette().text }, mouse::Cursor::Unavailable);
    let physical = Size::new((size.width * scale).round() as u32, (size.height * scale).round() as u32);
    let rgba = renderer.screenshot(physical, scale, background);
    Ok(Image { width: physical.width, height: physical.height, rgba })
}

/// The bundled fonts, loaded once into the renderer's font list as the windows load them.
fn load_fonts() {
    let mut fonts = font_system().write().unwrap_or_else(std::sync::PoisonError::into_inner);
    // Loading the same bytes again is skipped by the font list itself.
    for bundled in &crate::font::BUNDLED {
        fonts.load_font(bundled.bytes.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{container, text};

    #[test]
    fn draws_a_widget_offscreen() {
        let background = Color::from_rgb8(0x20, 0x20, 0x20);
        let panel: Element<'_, ()> = container(text("Hello").size(30).color(Color::WHITE)).padding(10).into();
        let image = render(panel, Size::new(200.0, 100.0), 1.0, background).unwrap();
        assert_eq!((image.width, image.height), (200, 100));
        assert!(image.drawn([0x20, 0x20, 0x20]) > 50, "the text left marks");
        assert_eq!(image.pixel(199, 99), [0x20, 0x20, 0x20, 0xff], "the corner is background");
    }
}
