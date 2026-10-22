//! The Guide overlay's Notes and Mappings panels. Mappings lists what the profile does in one of
//! the game's modes (the one that's on, until Left and Right turn to another), and after it, a
//! section for each layer an input of that mode holds, with what the layer changes.

use crate::config::{Button, ButtonAction, GuideInput, InfoOverlay, Layer, MappingRow, Mode, OverlayStyle, Profile, RowShape, ScreenPosition, TriggerAction, Trigger, mapping_rows};

use super::input_token;

/// What the default mode (no mode switched to) is called on screen.
pub const DEFAULT_MODE: &str = "Default";

/// Rows of the Guide overlay's Mappings shown at once, layer sections included. The hints under
/// them (other modes, more pages) come on top.
const MAPPING_ROWS: usize = 14;

/// What the names of the Guide overlay's panels start with.
const PANEL_PREFIX: &str = "guide ";

/// Whether the info overlay named `name` is one of the Guide overlay's panels.
pub fn is_guide_panel(name: &str) -> bool {
    name.starts_with(PANEL_PREFIX)
}

/// Size of the Guide overlay's panels, relative to the default, so they read from across a room.
const PANEL_SCALE: f32 = 1.4;
/// The most of the display's width Notes may take (a percentage), so long lines wrap before they
/// reach the menu in the middle.
const NOTES_MAX_WIDTH: f32 = 30.0;

/// What the Guide overlay's panels show: a profile in one of its game's modes, and which page.
pub struct GuideView<'a> {
    pub profile: &'a Profile,
    /// The game's own modes. The built-in ones are left out: Guide mode is what's on screen, and
    /// Mouse mode is the same in every game.
    pub modes: &'a [Mode],
    /// The game's layers. Only these get a section; the built-in ones are left out.
    pub layers: &'a [Layer],
    /// The mode on show: 0 is the default, then the modes in order.
    pub shown: usize,
    /// The mode that's on, counted the same way.
    pub current: usize,
    pub page: usize,
}

/// A layer an input holds, as the Mappings show it.
struct Section {
    title: String,
    rows: Vec<Vec<String>>,
}

impl Section {
    /// Rows it takes on a page: its heading, its rows and the gap above it.
    fn height(&self) -> usize {
        self.rows.len() + 2
    }
}

/// Which mapping rows and sections a page shows.
struct PageLayout {
    rows: std::ops::Range<usize>,
    sections: Vec<usize>,
}

impl GuideView<'_> {
    fn mode(&self) -> Option<&Mode> {
        self.shown.checked_sub(1).and_then(|i| self.modes.get(i))
    }

    /// The profile as it maps in the shown mode. Guide texts written for inputs the mode
    /// changes are dropped, since they describe what those inputs do outside it.
    fn mapped(&self) -> Profile {
        match self.mode() {
            None => self.profile.clone(),
            Some(mode) => {
                let mut p = self.profile.with_layers([mode]);
                p.guide = p.guide.without_changes_of(mode);
                p
            }
        }
    }

    /// The Notes and Mappings overlays, and the layer sections on the page shown. Notes are left
    /// out when the author wrote none, and the Mappings when nothing is mapped and there are no
    /// modes to turn to.
    pub fn overlays(&self) -> Vec<InfoOverlay> {
        let mut overlays: Vec<InfoOverlay> = notes_panel(&self.profile.guide.notes).into_iter().collect();
        let mapped = self.mapped();
        let rows: Vec<Vec<String>> = mapping_rows(&mapped).iter().map(cells).collect();
        let sections = self.sections(&mapped);
        let pages = layout(rows.len(), &sections);
        let at = self.page.min(pages.len() - 1);
        let page = &pages[at];
        // Nothing changed, and no other modes to turn to: nothing to show.
        if rows.is_empty() && sections.is_empty() && self.modes.is_empty() {
            return overlays;
        }
        let mut shown = rows[page.rows.clone()].to_vec();
        shown.extend(self.hints(pages.len()));
        overlays.push(panel(&format!("{PANEL_PREFIX}mappings"), ScreenPosition::CenterRight, self.title(at, pages.len()), shown));
        for &i in &page.sections {
            let section = &sections[i];
            overlays.push(panel(&format!("{PANEL_PREFIX}layer {i}"), ScreenPosition::CenterRight, section.title.clone(), section.rows.clone()));
        }
        overlays
    }

    /// How many pages the Mappings take in the shown mode.
    pub fn pages(&self) -> usize {
        let mapped = self.mapped();
        layout(mapping_rows(&mapped).len(), &self.sections(&mapped)).len()
    }


    /// "Mappings", then which page, then which mode it is when the game has modes. A mode that
    /// isn't on is a preview, and says what turns it on.
    fn title(&self, page: usize, pages: usize) -> String {
        let mut title = "Mappings".to_string();
        if pages > 1 {
            title.push_str(&format!(" {}/{pages}", page + 1));
        }
        if !self.modes.is_empty() {
            let name = self.mode().map_or(DEFAULT_MODE, |m| m.name.as_str());
            // "Mouse mode" is named a mode already.
            let mode = if name.to_lowercase().ends_with("mode") { "" } else { " mode" };
            title.push_str(&format!(" · {name}{mode}"));
            if self.shown != self.current {
                title.push_str(&match self.switches_to_shown().first() {
                    Some(input) => format!(" (preview; {input} turns it on)"),
                    None => " (preview)".into(),
                });
            }
        }
        title
    }

    /// The muted rows under the mappings that say what turns to other modes and pages, written
    /// like the mappings: the input, then what it does.
    fn hints(&self, pages: usize) -> Vec<Vec<String>> {
        let hint = |glyph: &str, text: &str| vec![glyph.to_string(), format!("{{color:gray}}{text}{{/color}}")];
        let mut hints = Vec::new();
        if !self.modes.is_empty() {
            hints.push(hint("{leftright}", "Other modes"));
        }
        if pages > 1 {
            hints.push(hint("{rs}", "Up or down: more mappings"));
        }
        hints
    }

    /// The inputs that switch to the shown mode from the mode that's on.
    fn switches_to_shown(&self) -> Vec<String> {
        let Some(target) = self.mode() else { return Vec::new() };
        let on = match self.current.checked_sub(1).and_then(|i| self.modes.get(i)) {
            Some(mode) => self.profile.with_layers([mode]),
            None => self.profile.clone(),
        };
        holders(&on, |a| match a {
            ButtonAction::Mode(name) => Some(name.as_str()),
            _ => None,
        })
        .into_iter()
        .filter(|(name, _)| *name == target.name)
        .map(|(_, input)| input)
        .collect()
    }

    /// A section for each of the game's layers that an input of `mapped` holds, in the order
    /// they're first held.
    fn sections(&self, mapped: &Profile) -> Vec<Section> {
        let mut held: Vec<(String, Vec<String>)> = Vec::new();
        let layers = holders(mapped, |a| match a {
            ButtonAction::Layer(name) => Some(name.as_str()),
            _ => None,
        });
        for (name, holder) in layers {
            match held.iter_mut().find(|(n, _)| *n == name) {
                Some((_, holders)) => holders.push(holder),
                None => held.push((name, vec![holder])),
            }
        }
        held.into_iter()
            .filter_map(|(name, holders)| {
                let layer = self.layers.iter().find(|l| l.name == name)?;
                let mut over = mapped.with_layers([layer]);
                over.guide = over.guide.without_changes_of(layer);
                let rows = mapping_rows(&over).iter().filter(|r| about(layer, r)).map(cells).collect();
                // Named as its Mappings row names it.
                Some(Section { title: format!("Layer “{name}” · {}", holders.join(", ")), rows })
            })
            .collect()
    }
}

/// Whether a row of the Mappings (with `layer` on) is about something `layer` sets.
fn about(layer: &Layer, row: &MappingRow) -> bool {
    match row.shape {
        RowShape::Combo => layer.sets_combo(&row.inputs),
        _ => row.inputs.iter().any(|i| layer.sets(*i)),
    }
}

/// A row's cells: the glyphs of its inputs, then its text.
fn cells(row: &MappingRow) -> Vec<String> {
    let glyphs: Vec<String> = row.inputs.iter().filter_map(|i| input_token(*i)).map(|t| format!("{{{t}}}")).collect();
    vec![row.shape.join(&glyphs), row.text.clone()]
}

/// Every layer or mode (as `named` picks them out of actions) an input of `p` holds or switches
/// to, with that input's glyphs (a gesture's or combo's marked like their Mappings rows), in the
/// order of the Mappings: buttons, gestures, combos, triggers.
fn holders(p: &Profile, named: impl Fn(&ButtonAction) -> Option<&str>) -> Vec<(String, String)> {
    let glyph = |b: Button| input_token(GuideInput::Button(b)).map(|t| format!("{{{t}}}"));
    let mut holders = Vec::new();
    let mut add = |action: &ButtonAction, shape: RowShape, glyphs: Option<Vec<String>>| {
        let Some(glyphs) = glyphs else { return };
        action.walk(&mut |a| {
            if let Some(name) = named(a) {
                holders.push((name.to_string(), shape.join(&glyphs)));
            }
        });
    };
    for (b, action) in p.buttons.iter().filter(|(b, _)| **b != Button::Guide) {
        add(action, RowShape::Input, glyph(*b).map(|g| vec![g]));
    }
    for (b, gestures) in p.gestures.iter().filter(|(b, _)| **b != Button::Guide) {
        for kind in crate::config::GestureKind::ALL {
            if let Some(action) = gestures.get(kind) {
                add(action, RowShape::Gesture(kind), glyph(*b).map(|g| vec![g]));
            }
        }
    }
    for combo in &p.combos {
        add(&combo.action, RowShape::Combo, combo.buttons.iter().map(|b| glyph(*b)).collect());
    }
    for t in [Trigger::Left, Trigger::Right] {
        if let TriggerAction::Button { action, .. } = p.trigger(t) {
            let token = input_token(GuideInput::Trigger(t)).map(|t| vec![format!("{{{t}}}")]);
            add(action, RowShape::Input, token);
        }
    }
    holders
}

/// Splits `rows` mapping rows and the `sections` into pages: the rows a page at a time, then
/// the sections in the room left, each on a new page when it doesn't fit. A section taller than
/// a page gets one to itself.
fn layout(rows: usize, sections: &[Section]) -> Vec<PageLayout> {
    let mut pages: Vec<PageLayout> = (0..rows.div_ceil(MAPPING_ROWS).max(1))
        .map(|i| PageLayout { rows: i * MAPPING_ROWS..((i + 1) * MAPPING_ROWS).min(rows), sections: Vec::new() })
        .collect();
    let mut used = pages.last().map_or(0, |p| p.rows.len());
    for (i, section) in sections.iter().enumerate() {
        if used > 0 && used + section.height() > MAPPING_ROWS {
            pages.push(PageLayout { rows: rows..rows, sections: Vec::new() });
            used = 0;
        }
        if let Some(page) = pages.last_mut() {
            page.sections.push(i);
        }
        used += section.height();
    }
    pages
}

/// A panel of the Guide overlay.
/// The Guide overlay's Notes panel for `notes`, a row per line, or `None` when there are none.
/// The settings window previews the notes with it too.
pub fn notes_panel(notes: &str) -> Option<InfoOverlay> {
    if notes.trim().is_empty() {
        return None;
    }
    let rows = notes.lines().map(|line| vec![line.to_string()]).collect();
    let mut panel = panel(&format!("{PANEL_PREFIX}notes"), ScreenPosition::CenterLeft, "Notes".into(), rows);
    panel.style.max_width = NOTES_MAX_WIDTH;
    Some(panel)
}

fn panel(name: &str, position: ScreenPosition, title: String, rows: Vec<Vec<String>>) -> InfoOverlay {
    InfoOverlay {
        name: name.into(),
        always: true,
        on_start: None,
        linger: None,
        current_input: Default::default(),
        style: OverlayStyle { position, scale: PANEL_SCALE, ..OverlayStyle::info() },
        title: Some(title),
        rows,
    }
}

#[cfg(test)]
mod tests;
