//! `padwight debug gui-check`: lays out every tab of the settings window, light and dark, at a
//! small window size, and looks for text that wraps where it should be one line, text that is
//! clipped or leaves the window, and text too faint on what it is drawn on (WCAG AA). The menu
//! colors for color-blind people get the same contrast check and a simulated deuteranopia pass.
//! Nothing is read from or written to the user's own config.
//!
//! The layout comes from iced's widget tree (`probe`), not from pixels. Text colors are not in
//! that tree, so contrast reads the colors a text really drew in from the image of the same
//! layout. See `docs/development/qa-tooling.md`, T-11.

mod contrast;
mod palette;
mod probe;
mod report;
mod scan;
mod scenes;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use iced::{Color, Rectangle, Size, Theme};

use super::*;

pub const USAGE: &str = "
Layout and contrast checks of the settings window (no display or daemon needed):
  gui-check [options]            check every tab in light and dark, print what is wrong
  options: --size <w>x<h>        the window (default 900x600; only the width matters, the page scrolls)
           --scene <text>        only the scenes whose name has this text   --list  name the scenes
           --light | --dark      only that theme
           --out <dir>           where pictures with the findings boxed go (default screenshots/gui-check)
           --strict              exit with an error if anything is found";

/// What kind of problem a finding is, worst first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Text drawn past the window's edge.
    OffScreen,
    /// Text cut off by a viewport.
    Clipped,
    /// Text outside the container it is in.
    OutOfParent,
    /// A short label broken over several lines.
    Wrapped,
    /// Text below WCAG AA against its background.
    LowContrast,
    /// A theme color pair below WCAG AA.
    Palette,
    /// Menu tones that merge under deuteranopia, or text on them that is too faint.
    Colorblind,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::OffScreen => "off-screen",
            Kind::Clipped => "clipped",
            Kind::OutOfParent => "out-of-container",
            Kind::Wrapped => "wrapped-label",
            Kind::LowContrast => "low-contrast",
            Kind::Palette => "palette",
            Kind::Colorblind => "color-blind",
        }
    }
}

/// One problem, in one scene and theme.
#[derive(Debug, Clone)]
pub struct Finding {
    pub scene: String,
    pub theme: &'static str,
    pub kind: Kind,
    pub text: String,
    pub detail: String,
    pub bounds: Rectangle,
}

impl Finding {
    /// A finding in `scene` drawn in `theme` (the pair says where it was seen).
    pub fn new((scene, theme): (&str, &'static str), kind: Kind, text: &str, detail: String, bounds: Rectangle) -> Self {
        Finding { scene: scene.into(), theme, kind, text: text.into(), detail, bounds }
    }
}

/// What the window draws now: its element, the theme, and the color behind it.
fn drawn(app: &App) -> (Element<'_, Message>, Theme, Color) {
    let theme = app.theme();
    if app.stage.is_some() {
        let game = Color::from_rgb8(0x3a, 0x4d, 0x63);
        let stage_theme = stage::stage_theme(&theme);
        return (app.view_stage(), stage_theme, game);
    }
    let background = theme.extended_palette().background.base.color;
    (app.view(), theme, background)
}

struct Options {
    size: Size,
    out: PathBuf,
    scene: Option<String>,
    themes: Vec<bool>,
    list: bool,
    strict: bool,
}

fn parse(args: &[&str]) -> Result<Options> {
    let mut o = Options { size: Size::new(900.0, 600.0), out: PathBuf::from("screenshots/gui-check"), scene: None, themes: Vec::new(), list: false, strict: false };
    let mut it = args.iter().copied();
    while let Some(arg) = it.next() {
        match arg {
            "--size" => {
                let v = it.next().context("--size needs <w>x<h>")?;
                let (w, h) = v.split_once('x').with_context(|| format!("--size takes <w>x<h>, not {v}"))?;
                o.size = Size::new(w.parse()?, h.parse()?);
            }
            "--out" => o.out = PathBuf::from(it.next().context("--out needs a folder")?),
            "--scene" => o.scene = Some(it.next().context("--scene needs a name")?.to_lowercase()),
            "--light" => o.themes.push(false),
            "--dark" => o.themes.push(true),
            "--list" => o.list = true,
            "--strict" => o.strict = true,
            other => bail!("unknown option {other:?}{USAGE}"),
        }
    }
    if o.themes.is_empty() {
        o.themes = vec![false, true];
    }
    Ok(o)
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    let o = parse(args)?;
    let scenes: Vec<_> = scenes::all().into_iter().filter(|s| o.scene.as_ref().is_none_or(|w| s.name.to_lowercase().contains(w))).collect();
    if o.list {
        scenes.iter().for_each(|s| println!("{}", s.name));
        return Ok(());
    }
    let mut findings = Vec::new();
    let mut skipped = Vec::new();
    for &dark in &o.themes {
        let theme_name = if dark { "dark" } else { "light" };
        for scene in &scenes {
            match scenes::checked(scene, dark, theme_name, &o.out, o.size) {
                Ok(f) => findings.extend(f),
                Err(e) => skipped.push(format!("{} ({theme_name}): {e:#}", scene.name)),
            }
        }
        findings.extend(palette::theme(dark, theme_name));
    }
    findings.extend(palette::colorblind());
    print!("{}", report::text(&findings, &skipped, scenes.len() * o.themes.len(), o.size));
    if o.strict && !findings.is_empty() {
        bail!("{} findings", report::unique(&findings).len());
    }
    Ok(())
}
