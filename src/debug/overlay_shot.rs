//! `padwight debug overlay`: draws the overlay to PNG files with no display, compositor or
//! overlay window. A scene is a made-up frame (see `scenes.rs`); `live` is the frame a running
//! daemon is showing; a `.json` file is a frame as the daemon sends it (`--json` writes one).

use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use iced::{Color, Element, Size, widget::{column, container, row}};

use super::{glyphs, scenes};
use crate::{
    ipc::{self, Request, UnixStream},
    motion::Anim,
    overlay::{self, OverlayFrame, draw, fit::Fit},
};

pub const USAGE: &str = "
Drawing the overlay without a display (no compositor or overlay window needed):
  overlay list                    list the scenes
  overlay <what>... [options]     draw each to <out>/<name>.png and print the paths
     <what> is a scene, `all` (every scene), `glyphs` (every glyph the overlays draw),
     `live` (what a running daemon's overlay shows: the first frame with something
     in it, within --wait), a frame saved as JSON, or a .padpack (each of its menus,
     its keyboard and numpad, and its info overlays, in its own styles and icons)
  options: --out <dir> (default screenshots/overlay)   --size <w>x<h> (default 1920x1080)
           --scale <n> (default 1)   --background <#rrggbb> (default #3c4650)
           --wait <ms> (default 3000, for live)   --json (print the frame instead of drawing)
";

struct Options {
    out: PathBuf,
    size: Size,
    scale: f32,
    background: [u8; 3],
    wait: Duration,
    json: bool,
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    if args == ["list"] {
        for (name, about) in scenes::SCENES {
            println!("{name:<20}{about}");
        }
        println!("{:<20}every glyph the overlays draw (not a frame)", "glyphs");
        return Ok(());
    }
    let (targets, options) = parse(args)?;
    for target in targets {
        if target == "glyphs" {
            if options.json {
                bail!("glyphs is not a frame, so it has no JSON");
            }
            let path = options.out.join("glyphs.png");
            glyph_sheets(&options)?.save(&path)?;
            println!("{}", path.display());
            continue;
        }
        let frames = if target.ends_with(".padpack") { super::pack_scenes::frames(Path::new(&target))? } else { vec![load(&target, options.wait)?] };
        for (name, frame) in frames {
            if options.json {
                println!("{}", serde_json::to_string_pretty(&frame)?);
                continue;
            }
            let [r, g, b] = options.background;
            let image = overlay::snapshot(frame, options.size, options.scale, Color::from_rgb8(r, g, b))?;
            let path = options.out.join(format!("{name}.png"));
            image.save(&path)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// What to draw, with `all` spelled out, and the options.
fn parse(args: &[&str]) -> Result<(Vec<String>, Options)> {
    let mut options = Options {
        out: PathBuf::from("screenshots/overlay"),
        size: Size::new(1920.0, 1080.0),
        scale: 1.0,
        background: [0x3c, 0x46, 0x50],
        wait: Duration::from_secs(3),
        json: false,
    };
    let mut targets = Vec::new();
    let mut it = args.iter();
    while let Some(&arg) = it.next() {
        let mut value = || it.next().copied().with_context(|| format!("{arg} needs a value"));
        match arg {
            "--out" => options.out = PathBuf::from(value()?),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').with_context(|| format!("--size takes <w>x<h>, not {v}"))?;
                options.size = Size::new(w.parse()?, h.parse()?);
            }
            "--scale" => options.scale = value()?.parse()?,
            "--background" => {
                let v = value()?;
                options.background = crate::config::parse_hex(v).with_context(|| format!("--background takes #rrggbb, not {v}"))?;
            }
            "--wait" => options.wait = Duration::from_millis(value()?.parse()?),
            "--json" => options.json = true,
            "all" => targets.extend(scenes::SCENES.map(|(name, _)| name.to_string())),
            _ if arg.starts_with("--") => bail!("unknown option {arg}{USAGE}"),
            _ => targets.push(arg.to_string()),
        }
    }
    if targets.is_empty() {
        bail!("nothing to draw{USAGE}");
    }
    Ok((targets, options))
}

/// The frame `target` names, and the name its image is saved under.
fn load(target: &str, wait: Duration) -> Result<(String, OverlayFrame)> {
    if target == "live" {
        return Ok(("live".into(), live(&ipc::socket_path(), wait)?));
    }
    if target.ends_with(".json") {
        let path = Path::new(target);
        let text = std::fs::read_to_string(path).with_context(|| format!("cannot read {target}"))?;
        let frame = serde_json::from_str(&text).with_context(|| format!("{target} is not an overlay frame"))?;
        let name = path.file_stem().map_or("frame".into(), |s| s.to_string_lossy().into_owned());
        return Ok((name, frame));
    }
    Ok((target.to_string(), scenes::frame(target)?))
}

/// The frame the overlay of the daemon at `socket` is showing: the first it sends that shows
/// something, waiting up to `wait` for one.
fn live(socket: &Path, wait: Duration) -> Result<OverlayFrame> {
    let mut stream = UnixStream::connect(socket).with_context(|| format!("no daemon at {}", socket.display()))?;
    let mut request = serde_json::to_string(&Request::WatchOverlay)?;
    request.push('\n');
    stream.write_all(request.as_bytes())?;
    let deadline = Instant::now() + wait;
    let mut lines = BufReader::new(stream);
    // The icons come only in the frames where they change, which may show nothing.
    let mut icons = None;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            bail!("the overlay showed nothing for {} ms", wait.as_millis());
        }
        lines.get_ref().set_read_timeout(Some(left))?;
        let mut line = String::new();
        match lines.read_line(&mut line) {
            Ok(0) => bail!("the daemon stopped sending overlay frames"),
            Ok(_) => {
                let mut frame: OverlayFrame = serde_json::from_str(&line).context("the daemon sent something that isn't a frame")?;
                if frame.pack_icons.is_some() {
                    icons.clone_from(&frame.pack_icons);
                } else {
                    frame.pack_icons.clone_from(&icons);
                }
                if frame.active.is_some() || !frame.info.is_empty() || !frame.logs.is_empty() {
                    return Ok(frame);
                }
            }
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

/// Every glyph sheet, both layouts, side by side, sized to fit them.
fn glyph_sheets(options: &Options) -> Result<crate::snapshot::Image> {
    let font = iced::Font::DEFAULT;
    let panel = |v: &crate::info::InfoView| -> Element<'static, ()> { container(draw::info_panel(v, font, &Fit::contents(), &Anim::still())).into() };
    let sheets = glyphs::sheets(false);
    let swapped = glyphs::sheets(true);
    let (pads, rest) = sheets.split_at(3);
    let page = column![
        row(pads.iter().map(panel)).spacing(16),
        row(swapped[..3].iter().map(panel)).spacing(16),
        row(rest.iter().map(panel)).spacing(16),
    ]
    .spacing(16)
    .padding(16);
    let [r, g, b] = options.background;
    // Tall enough for every sheet; the empty space below is trimmed off.
    let image = crate::snapshot::render(page.into(), Size::new(options.size.width.max(1500.0), 2400.0), options.scale, Color::from_rgb8(r, g, b))?;
    Ok(image.trimmed(options.background, (16.0 * options.scale) as u32))
}

#[cfg(test)]
mod tests {
    use crate::ipc::UnixListener;

    use super::*;

    const BACKGROUND: [u8; 3] = [0x3c, 0x46, 0x50];

    #[test]
    fn every_scene_draws_something() {
        let [r, g, b] = BACKGROUND;
        for (name, _) in scenes::SCENES {
            let image = overlay::snapshot(scenes::frame(name).unwrap(), Size::new(960.0, 540.0), 1.0, Color::from_rgb8(r, g, b)).unwrap();
            assert!(image.drawn(BACKGROUND) > 500, "{name} drew nothing");
        }
    }

    #[test]
    fn the_glyph_sheets_are_trimmed_to_what_they_draw() {
        let (_, options) = parse(&["glyphs"]).unwrap();
        let image = glyph_sheets(&options).unwrap();
        assert!(image.height < 2400, "the space below the sheets is cut off");
        assert!(image.drawn(BACKGROUND) > (image.width * image.height / 4) as usize);
    }

    #[test]
    fn live_waits_for_a_frame_that_shows_something() {
        let dir = std::env::temp_dir().join(format!("padwight-overlay-shot-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("daemon.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let daemon = std::thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(&conn).read_line(&mut request).unwrap();
            assert!(request.contains("WatchOverlay"));
            for frame in [OverlayFrame::default(), scenes::frame("media").unwrap()] {
                writeln!(conn, "{}", serde_json::to_string(&frame).unwrap()).unwrap();
            }
        });
        let frame = live(&socket, Duration::from_secs(5)).unwrap();
        daemon.join().unwrap();
        assert!(matches!(frame.active, Some(overlay::OverlayView::Media(_))), "the empty frame was skipped");
        let _ = std::fs::remove_dir_all(dir);
    }
}
