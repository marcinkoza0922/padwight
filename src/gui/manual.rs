//! The Manual page. The text is `docs/manual/*.md`, the same Markdown that GitHub shows, embedded at
//! build time. Each file starts with its `# Title`, and links between chapters are file names.

use iced::widget::{column, container, markdown, row, rule, text};

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
    ("troubleshooting.md", include_str!("../../docs/manual/troubleshooting.md")),
];

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
            markdown::view(self.manual[chosen].items(), settings).map(Message::ManualLink)
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
        for link in source.split("](").skip(1).filter_map(|rest| rest.split(')').next()) {
            if link.starts_with("http") || link.is_empty() {
                continue;
            }
            assert!(chapter_of(link).is_some(), "link to unknown chapter {link:?}");
        }
    }
}
