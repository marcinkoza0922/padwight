//! The Manual page. The text is `docs/manual/*.md`, the same Markdown that GitHub shows, embedded at
//! build time. Each file starts with its `# Title`, and links between chapters are file names.

use std::sync::OnceLock;

use iced::widget::{column, container, markdown, row, rule, svg, text};

use super::*;

/// The chapters, in the order listed on the page.
const CHAPTERS: &[(&str, &str)] = &[
    ("welcome.md", include_str!("../../docs/manual/welcome.md")),
    ("settings-window.md", include_str!("../../docs/manual/settings-window.md")),
    ("first-change.md", include_str!("../../docs/manual/first-change.md")),
    ("words.md", include_str!("../../docs/manual/words.md")),
    ("setups.md", include_str!("../../docs/manual/setups.md")),
    ("buttons.md", include_str!("../../docs/manual/buttons.md")),
    ("sticks.md", include_str!("../../docs/manual/sticks.md")),
    ("gyro.md", include_str!("../../docs/manual/gyro.md")),
    ("touchpad.md", include_str!("../../docs/manual/touchpad.md")),
    ("modes.md", include_str!("../../docs/manual/modes.md")),
    ("layers.md", include_str!("../../docs/manual/layers.md")),
    ("macros.md", include_str!("../../docs/manual/macros.md")),
    ("menus.md", include_str!("../../docs/manual/menus.md")),
    ("overlays.md", include_str!("../../docs/manual/overlays.md")),
    ("guide-button.md", include_str!("../../docs/manual/guide-button.md")),
    ("in-game-menu.md", include_str!("../../docs/manual/in-game-menu.md")),
    ("sharing.md", include_str!("../../docs/manual/sharing.md")),
    ("tutorial-first-setup.md", include_str!("../../docs/manual/tutorial-first-setup.md")),
    ("tutorial-buttons.md", include_str!("../../docs/manual/tutorial-buttons.md")),
    ("tutorial-sticks.md", include_str!("../../docs/manual/tutorial-sticks.md")),
    ("tutorial-gyro.md", include_str!("../../docs/manual/tutorial-gyro.md")),
    ("tutorial-touchpad.md", include_str!("../../docs/manual/tutorial-touchpad.md")),
    ("tutorial-modes.md", include_str!("../../docs/manual/tutorial-modes.md")),
    ("tutorial-layers.md", include_str!("../../docs/manual/tutorial-layers.md")),
    ("tutorial-macros.md", include_str!("../../docs/manual/tutorial-macros.md")),
    ("tutorial-menus.md", include_str!("../../docs/manual/tutorial-menus.md")),
    ("tutorial-overlays.md", include_str!("../../docs/manual/tutorial-overlays.md")),
    ("tutorial-guide.md", include_str!("../../docs/manual/tutorial-guide.md")),
    ("tutorial-sharing.md", include_str!("../../docs/manual/tutorial-sharing.md")),
    ("tutorial-deus-ex.md", include_str!("../../docs/manual/tutorial-deus-ex.md")),
    ("troubleshooting.md", include_str!("../../docs/manual/troubleshooting.md")),
];

/// `IMAGES` entries from picture names, each embedding `docs/images/manual/<name>.png`.
macro_rules! images {
    ($($name:literal),* $(,)?) => {
        [$((concat!($name, ".png"), include_bytes!(concat!("../../docs/images/manual/", $name, ".png")) as &[u8])),*]
    };
}

/// The pictures the chapters show, by file name. They are in `docs/images/manual/`, and the chapters name
/// them as `../images/manual/<file>` so that GitHub shows them too.
const IMAGES: &[(&str, &[u8])] = &images![
    "buttons-row", "deus-buttons", "deus-flick", "deus-overview", "deus-picture", "first-setup-add", "first-setup-rules", "guide-overlay", "guide-tab", "gyro-panel", "in-game-menu",
    "info-overlay", "info-tab", "keyboard-overlay", "layers-tab", "macros-tab", "menu-list", "menu-radial", "menus-tab",
    "modes-tab", "sharing-export", "sticks-ring", "sticks-zones", "touchpad-panel",
];

/// `IMAGES` ready to draw. The window draws SVG, not PNG, so each picture is wrapped in an SVG that holds it.
fn picture(file: &str) -> Option<(svg::Handle, u32)> {
    static HANDLES: OnceLock<Vec<(&str, svg::Handle, u32)>> = OnceLock::new();
    HANDLES
        .get_or_init(|| {
            IMAGES
                .iter()
                .filter_map(|&(name, png)| {
                    // The PNG's header holds its size, big-endian, after the 8-byte signature and the chunk's length and name.
                    let size = |at: usize| Some(u32::from_be_bytes(png.get(at..at + 4)?.try_into().ok()?));
                    let (w, h) = (size(16)?, size(20)?);
                    let document = format!(
                        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><image width=\"{w}\" height=\"{h}\" xlink:href=\"data:image/png;base64,{}\"/></svg>",
                        base64(png)
                    );
                    Some((name, svg::Handle::from_memory(document.into_bytes()), w))
                })
                .collect()
        })
        .iter()
        .find(|(name, ..)| *name == file)
        .map(|(_, handle, width)| (handle.clone(), *width))
}

fn base64(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= chunk.len() { char::from(DIGITS[(n >> (18 - 6 * i) & 63) as usize]) } else { '=' });
        }
    }
    out
}

/// Draws the chapters' Markdown, with pictures.
struct Viewer;

impl<'a> markdown::Viewer<'a, String> for Viewer {
    fn on_link_click(url: markdown::Uri) -> String {
        url
    }

    fn image(&self, settings: markdown::Settings, url: &'a markdown::Uri, title: &'a str, _alt: &markdown::Text) -> Element<'a, String> {
        let file = url.rsplit('/').next().unwrap_or(url);
        let Some((handle, width)) = picture(file) else {
            return container(text(format!("[picture {file} missing]"))).padding(settings.spacing.0).into();
        };
        let mut shown = column![svg(handle).width(Length::Fill)].spacing(4);
        if !title.is_empty() {
            shown = shown.push(text(title).size(12).color(MUTED_COLOR));
        }
        // As wide as the picture is, no wider, so small ones aren't blown up.
        container(shown).max_width(width.min(900) as f32).into()
    }
}

/// The chapters parsed for drawing, in the order of `CHAPTERS`.
pub(super) fn load() -> Vec<markdown::Content> {
    CHAPTERS.iter().map(|(_, source)| markdown::Content::parse(source)).collect()
}

/// The chapter a link points to, by the file it names.
pub(super) fn chapter_of(link: &str) -> Option<usize> {
    CHAPTERS.iter().position(|(file, _)| *file == link)
}

/// The chapter's title: the text of its first line.
fn title(source: &str) -> &str {
    source.lines().next().unwrap_or_default().trim_start_matches('#').trim()
}

/// The first line of `source` that has every word of `query`, trimmed of Markdown marks, for the
/// chapter list to show under the title.
fn snippet<'a>(source: &'a str, query: &str) -> Option<&'a str> {
    source
        .lines()
        .skip(1)
        .map(|l| l.trim().trim_start_matches(['#', '-', '*', '>', ' ']))
        .filter(|l| !l.is_empty())
        .filter_map(|l| Some((fuzzy(query, l)?, l)))
        .min_by_key(|(score, _)| *score)
        .map(|(_, l)| l)
}

/// The chapters that match `query`, best first, by index, each with the line that matched. Words may be spread
/// over a chapter, so a chapter can match without any one line doing so.
fn find(query: &str) -> Vec<(usize, Option<&'static str>)> {
    let mut hits: Vec<(u32, usize, Option<&'static str>)> = CHAPTERS
        .iter()
        .enumerate()
        .filter_map(|(i, (_, source))| Some((fuzzy(query, source)?, i, snippet(source, query))))
        .collect();
    // Best matches first; the chapters keep their order among equals.
    hits.sort_by_key(|&(score, ..)| score);
    hits.into_iter().map(|(_, i, line)| (i, line)).collect()
}

impl App {
    /// The chapter list on the left, and the chosen chapter on the right. Searching narrows the list
    /// to the chapters that fuzzily mention every word typed, the closest first.
    pub(super) fn view_manual(&self) -> Element<'_, Message> {
        let query = self.manual_query.as_str();
        let hits = find(query);
        let searching = !query.trim().is_empty();
        // A chosen chapter that the search rules out gives way to the first one it keeps.
        let mut chosen = self.manual_chapter.min(CHAPTERS.len() - 1);
        if let Some(&(first, _)) = hits.first()
            && !hits.iter().any(|(i, _)| *i == chosen)
        {
            chosen = first;
        }
        let mut index = column![search_field("Search the manual…", query, Message::SetManualSearch)].spacing(4);
        for &(i, line) in &hits {
            let mut label = column![text(title(CHAPTERS[i].1)).size(14)].spacing(2);
            if searching && let Some(line) = line {
                label = label.push(text(line).size(12).color(MUTED_COLOR));
            }
            index = index.push(
                button(label).width(Length::Fill).padding([6, 10]).style(style::nav(i == chosen)).on_press(Message::SelectChapter(i)),
            );
        }
        if hits.is_empty() {
            index = index.push(text("No chapter matches.").size(13).color(MUTED_COLOR));
        }
        let body: Element<'_, Message> = if hits.is_empty() {
            space().into()
        } else {
            let settings = markdown::Settings::from(&self.theme());
            markdown::view_with(self.manual[chosen].items(), settings, &Viewer).map(Message::ManualLink)
        };
        row![
            container(index).width(260),
            rule::vertical(1),
            container(body).width(Length::Fill).padding([0, 8]),
        ]
        .spacing(16)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_have_titles_and_text() {
        let mut titles = HashSet::new();
        for (file, source) in CHAPTERS {
            assert!(source.starts_with("# "), "{file} doesn't start with a title");
            assert!(titles.insert(title(source)), "two chapters are titled {:?}", title(source));
        }
        for content in load() {
            assert!(!content.items().is_empty());
        }
    }

    #[test]
    fn search_narrows_the_chapters() {
        assert_eq!(find("").len(), CHAPTERS.len());
        assert!(find("zzzqqq").is_empty());
        let hits = find("GYRO");
        assert!(hits.iter().any(|(i, _)| CHAPTERS[*i].0 == "gyro.md"), "{hits:?}");
        assert!(hits.len() < CHAPTERS.len());
    }

    #[test]
    fn links_between_chapters_resolve() {
        let source: String = CHAPTERS.iter().map(|(_, s)| *s).collect();
        for link in source.split("](").skip(1).filter_map(|rest| rest.split([')', ' ']).next()) {
            if link.starts_with("http") || link.is_empty() || link.starts_with("../images/manual/") {
                continue;
            }
            assert!(chapter_of(link).is_some(), "link to unknown chapter {link:?}");
        }
    }

    #[test]
    fn every_picture_is_listed_and_used() {
        let source: String = CHAPTERS.iter().map(|(_, s)| *s).collect();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/images/manual");
        let on_disk: HashSet<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        let listed: HashSet<String> = IMAGES.iter().map(|(name, _)| name.to_string()).collect();
        assert_eq!(on_disk, listed, "docs/images/manual and IMAGES differ");
        for (name, png) in IMAGES {
            assert!(source.contains(&format!("../images/manual/{name}")), "{name} is in no chapter");
            assert!(picture(name).is_some(), "{name} is not a PNG with a header: {} bytes", png.len());
        }
        for link in source.split("](../images/manual/").skip(1).filter_map(|rest| rest.split([')', ' ']).next()) {
            assert!(listed.contains(link), "a chapter shows {link}, which isn't in IMAGES");
        }
    }

    #[test]
    fn tutorial_links_from_panels_resolve() {
        for file in [
            "tutorial-first-setup.md", "tutorial-buttons.md", "tutorial-sticks.md", "tutorial-gyro.md", "tutorial-touchpad.md",
            "tutorial-modes.md", "tutorial-layers.md", "tutorial-macros.md", "tutorial-menus.md", "tutorial-overlays.md",
            "tutorial-guide.md", "tutorial-sharing.md",
        ] {
            assert!(chapter_of(file).is_some(), "no chapter {file}");
        }
    }
}
