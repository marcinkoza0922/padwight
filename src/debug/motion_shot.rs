//! `padwight debug motion`: plays a scripted run of an overlay in each motion style and draws
//! it, with no display: an animated PNG to watch, a strip of frames to compare side by side,
//! and with `--trace` the numbers the drawing reads at each moment.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use iced::{Color, Size};

use super::scenes;
use crate::{
    motion::{MotionSet, MotionStyle, PICK_HOLD, Presence},
    overlay::{self, OverlayFrame, OverlayView},
    snapshot::Image,
};

pub const USAGE: &str = "
Playing the overlay's motion styles without a display:
  motion [<style>... | all] [options]
     plays a run (open, move twice, pick, move, pick and close) in each style, and writes
     <out>/<scene>-<style>.png (animated: open it in a browser) and
     <out>/<scene>-<style>-strip.png (frames left to right, top to bottom)
  options: --scene <name> (default menu-list; a menu scene, keyboard or numpad)
           --out <dir> (default screenshots/motion)   --fps <n> (default 50, the animation's)
           --every <ms> (default 40, between strip frames)   --from <s> --until <s> (the strip's span)
           --size <w>x<h> (default 960x540)   --trace (print the numbers instead of drawing)
";

/// Where the cursor goes on the keyboard, step by step.
const KEYS: [&str; 4] = ["KEY_H", "KEY_J", "KEY_K", "KEY_L"];
/// The run: when the cursor moves to the next place, when picks land, and when it closes.
const MOVES: [(f32, usize); 3] = [(0.55, 1), (0.7, 2), (1.6, 3)];
const PICKS: [f32; 2] = [1.15, 2.0];
/// A keyboard key is let go this long after it is pressed.
const PRESS: f32 = 0.1;
/// How long the run goes on after the close has played.
const TAIL: f32 = 0.15;
const BACKGROUND: [u8; 3] = [0x3c, 0x46, 0x50];

struct Options {
    styles: Vec<MotionStyle>,
    scene: String,
    out: PathBuf,
    fps: f32,
    every: f32,
    from: f32,
    until: Option<f32>,
    size: Size,
    trace: bool,
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    let options = parse(args)?;
    let base = scenes::frame(&options.scene)?;
    for &style in &options.styles {
        let script = script(&base, style)?;
        let end = length(&script, style);
        if options.trace {
            print!("{}", trace(&script, style, end));
            continue;
        }
        let name = format!("{}-{}", options.scene, style.to_string().to_lowercase());
        let times = |from: f32, step: f32, until: f32| (0..).map(move |i| from + i as f32 * step).take_while(move |&t| t <= until).collect::<Vec<_>>();
        let [r, g, b] = BACKGROUND;
        let background = Color::from_rgb8(r, g, b);
        let film = overlay::film(&script, &times(0.0, 1.0 / options.fps, end), options.size, 1.0, background)?;
        let strip = overlay::film(&script, &times(options.from, options.every / 1000.0, options.until.unwrap_or(end)), options.size, 1.0, background)?;
        let area = drawn_area(film.iter().chain(&strip), 12).context("nothing was drawn")?;
        let animated = options.out.join(format!("{name}.png"));
        save_animation(&film.iter().map(|i| crop(i, area)).collect::<Vec<_>>(), options.fps, &animated)?;
        let strip_path = options.out.join(format!("{name}-strip.png"));
        tile(&strip.iter().map(|i| crop(i, area)).collect::<Vec<_>>(), 6).save(&strip_path)?;
        println!("{}\n{}", animated.display(), strip_path.display());
    }
    Ok(())
}

fn parse(args: &[&str]) -> Result<Options> {
    let mut options = Options {
        styles: Vec::new(),
        scene: "menu-list".into(),
        out: PathBuf::from("screenshots/motion"),
        fps: 50.0,
        every: 40.0,
        from: 0.0,
        until: None,
        size: Size::new(960.0, 540.0),
        trace: false,
    };
    let mut it = args.iter();
    while let Some(&arg) = it.next() {
        let mut value = || it.next().copied().with_context(|| format!("{arg} needs a value"));
        match arg {
            "--scene" => options.scene = value()?.into(),
            "--out" => options.out = PathBuf::from(value()?),
            "--fps" => options.fps = value()?.parse()?,
            "--every" => options.every = value()?.parse()?,
            "--from" => options.from = value()?.parse()?,
            "--until" => options.until = Some(value()?.parse()?),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').with_context(|| format!("--size takes <w>x<h>, not {v}"))?;
                options.size = Size::new(w.parse()?, h.parse()?);
            }
            "--trace" => options.trace = true,
            "all" => options.styles.extend(MotionStyle::ALL.into_iter().filter(|s| *s != MotionStyle::Off)),
            _ if arg.starts_with("--") => bail!("unknown option {arg}{USAGE}"),
            _ => {
                let style = MotionStyle::ALL.into_iter().find(|s| s.to_string().eq_ignore_ascii_case(arg));
                options.styles.push(style.with_context(|| format!("no motion style called {arg}{USAGE}"))?);
            }
        }
    }
    if options.styles.is_empty() {
        bail!("no style to play{USAGE}");
    }
    if options.fps <= 0.0 || options.every <= 0.0 {
        bail!("--fps and --every must be above 0");
    }
    Ok(options)
}

/// The frames of the run in `style`, each with when it arrives, from the scene's frame.
fn script(base: &OverlayFrame, style: MotionStyle) -> Result<Vec<(f32, OverlayFrame)>> {
    let Some(view) = base.active.clone() else { bail!("the scene has no menu or keyboard to move") };
    let shown = |view: OverlayView| OverlayFrame { active: Some(view), motion: MotionSet::all(style), ..base.clone() };
    let mut cursor = 0;
    let mut picks = 0;
    let at = |cursor: usize, picks: u32, pressed: bool| -> Result<OverlayView> {
        Ok(match view.clone() {
            OverlayView::Menu(mut m) => {
                m.selected = Some(cursor % m.items.len().max(1));
                m.picks = picks;
                OverlayView::Menu(m)
            }
            OverlayView::Keyboard(mut k) => {
                k.cursor = crate::keyboard::find(k.board, KEYS[cursor]).unwrap_or(k.cursor);
                k.pressed = pressed.then(|| crate::keyboard::key_at(k.board, k.cursor).code.to_string());
                OverlayView::Keyboard(k)
            }
            _ => bail!("the scene has no menu or keyboard to move"),
        })
    };
    let mut script = vec![(0.0, shown(at(0, 0, false)?))];
    let mut events: Vec<(f32, Option<usize>)> = MOVES.iter().map(|&(t, c)| (t, Some(c))).chain(PICKS.iter().map(|&t| (t, None))).collect();
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (t, event) in events {
        match event {
            Some(c) => {
                cursor = c;
                script.push((t, shown(at(cursor, picks, false)?)));
            }
            None => {
                picks += 1;
                script.push((t, shown(at(cursor, picks, true)?)));
                if matches!(view, OverlayView::Keyboard(_)) {
                    script.push((t + PRESS, shown(at(cursor, picks, false)?)));
                }
            }
        }
    }
    // The last pick closes a menu, as picking an action does.
    let last = PICKS[PICKS.len() - 1];
    let close = if matches!(view, OverlayView::Menu(_)) { last + 0.01 } else { last + 2.0 * PRESS };
    script.push((close, OverlayFrame { active: None, motion: MotionSet::all(style), ..base.clone() }));
    Ok(script)
}

/// When the run is over: the close and its hold played, and a moment more.
fn length(script: &[(f32, OverlayFrame)], style: MotionStyle) -> f32 {
    let closed = script.last().map_or(0.0, |(t, _)| *t);
    closed + PICK_HOLD + style.feel().close + TAIL
}

/// The numbers the drawing reads through the run, every 20 ms.
fn trace(script: &[(f32, OverlayFrame)], style: MotionStyle, end: f32) -> String {
    let start = std::time::Instant::now();
    let when = |t: f32| start + std::time::Duration::from_secs_f32(t);
    let motion = MotionSet::all(style);
    let mut shown: Presence<OverlayView> = Presence::single();
    let mut next = script.iter().peekable();
    let mut out = format!("{style}\n    t  opacity     x      y  scale   lit  item  flash  fade0  shift0 shift3\n");
    for i in 0.. {
        let t = i as f32 * 0.02;
        if t > end {
            break;
        }
        while let Some((arrives, frame)) = next.next_if(|(arrives, _)| *arrives <= t) {
            shown.update(when(*arrives), frame.active.iter().cloned().collect());
        }
        shown.prune(when(t), &motion);
        let Some((_, anim)) = shown.shown(when(t), &motion).last() else {
            out += &format!("{t:5.2}  (gone)\n");
            continue;
        };
        let (x, y) = anim.offset();
        out += &format!(
            "{t:5.2}  {:7.3} {x:6.1} {y:6.1}  {:5.3}  {:4.2}  {:4.3}  {:5.2}  {:5.2}  {:6.1} {:6.1}\n",
            anim.opacity(),
            anim.scale(),
            anim.lit(true, false),
            anim.item_scale(true),
            anim.flash(),
            anim.item_fade(0),
            anim.item_shift(0),
            anim.item_shift(3),
        );
    }
    out
}

/// A box around everything drawn off the background in any of `images`, `margin` pixels wider:
/// (x, y, width, height).
fn drawn_area<'a>(images: impl Iterator<Item = &'a Image>, margin: u32) -> Option<(u32, u32, u32, u32)> {
    let mut area: Option<(u32, u32, u32, u32)> = None;
    for image in images {
        for y in 0..image.height {
            for x in 0..image.width {
                if image.pixel(x, y)[..3] != BACKGROUND {
                    let (x0, y0, x1, y1) = area.unwrap_or((x, y, x, y));
                    area = Some((x0.min(x), y0.min(y), x1.max(x), y1.max(y)));
                }
            }
        }
    }
    let (x0, y0, x1, y1) = area?;
    let (x0, y0) = (x0.saturating_sub(margin), y0.saturating_sub(margin));
    Some((x0, y0, x1 + margin - x0 + 1, y1 + margin - y0 + 1))
}

/// The part of `image` in `(x, y, width, height)`, with background past its edges.
fn crop(image: &Image, (x0, y0, width, height): (u32, u32, u32, u32)) -> Image {
    let [r, g, b] = BACKGROUND;
    let mut rgba = Vec::with_capacity((4 * width * height) as usize);
    for y in y0..y0 + height {
        for x in x0..x0 + width {
            rgba.extend(if x < image.width && y < image.height { image.pixel(x, y) } else { [r, g, b, 255] });
        }
    }
    Image { width, height, rgba }
}

/// `images` (all one size) in rows of `columns`, a thin dark line between them.
fn tile(images: &[Image], columns: usize) -> Image {
    const LINE: u32 = 2;
    let (w, h) = images.first().map_or((1, 1), |i| (i.width, i.height));
    let columns = columns.min(images.len()).max(1);
    let rows = images.len().div_ceil(columns);
    let (width, height) = (columns as u32 * (w + LINE), rows as u32 * (h + LINE));
    let mut rgba = [0x10, 0x12, 0x14, 255].repeat((width * height) as usize);
    for (n, image) in images.iter().enumerate() {
        let (left, top) = ((n % columns) as u32 * (w + LINE), (n / columns) as u32 * (h + LINE));
        for y in 0..h {
            let from = (4 * y * w) as usize;
            let to = (4 * ((top + y) * width + left)) as usize;
            rgba[to..to + 4 * w as usize].copy_from_slice(&image.rgba[from..from + 4 * w as usize]);
        }
    }
    Image { width, height, rgba }
}

/// Writes `frames` as an animated PNG playing at `fps`, looping.
fn save_animation(frames: &[Image], fps: f32, path: &Path) -> Result<()> {
    let Some(first) = frames.first() else { bail!("no frames") };
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("cannot make {}", dir.display()))?;
    }
    let file = std::fs::File::create(path).with_context(|| format!("cannot write {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), first.width, first.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(frames.len() as u32, 0)?;
    encoder.set_frame_delay(1000, (fps * 1000.0) as u16)?;
    let mut writer = encoder.write_header()?;
    for frame in frames {
        writer.write_image_data(&frame.rgba)?;
    }
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_menu_run_moves_picks_and_closes() {
        let script = script(&scenes::frame("menu-list").unwrap(), MotionStyle::Playful).unwrap();
        assert!(script.windows(2).all(|w| w[0].0 <= w[1].0), "in order");
        assert!(script.last().unwrap().1.active.is_none(), "ends closed");
        let picks: Vec<u32> = script.iter().filter_map(|(_, f)| match &f.active {
            Some(OverlayView::Menu(m)) => Some(m.picks),
            _ => None,
        }).collect();
        assert_eq!(picks.last(), Some(&2));
    }

    #[test]
    fn the_trace_ends_with_the_menu_gone() {
        let script = script(&scenes::frame("menu-list").unwrap(), MotionStyle::Subtle).unwrap();
        let out = trace(&script, MotionStyle::Subtle, length(&script, MotionStyle::Subtle));
        assert!(out.trim_end().ends_with("(gone)"), "{out}");
    }

    #[test]
    fn a_strip_lays_frames_out_in_rows() {
        let image = Image { width: 3, height: 2, rgba: vec![255; 24] };
        let strip = tile(&[image.clone_like(), image.clone_like(), image], 2);
        assert_eq!((strip.width, strip.height), (2 * 5, 2 * 4));
    }

    impl Image {
        fn clone_like(&self) -> Image {
            Image { width: self.width, height: self.height, rgba: self.rgba.clone() }
        }
    }
}
