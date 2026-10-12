//! The Guide overlay's Notes and Mappings panels. Mappings lists what the profile does in one of
//! the game's modes (the one that's on, until Left and Right turn to another), and after it, a
//! section for each layer an input of that mode holds, with what the layer changes.

use crate::config::{Button, ButtonAction, GuideInput, InfoOverlay, Layer, MappingRow, Mode, OverlayStyle, Profile, RowShape, ScreenPosition, TriggerAction, Trigger, mapping_rows};

use super::input_token;

/// What the default mode (no mode switched to) is called on screen.
pub const DEFAULT_MODE: &str = "Default";

/// Rows of the Guide overlay's Mappings shown at once, layer sections included.
const MAPPING_ROWS: usize = 14;

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
    /// out when the author wrote none.
    pub fn overlays(&self) -> Vec<InfoOverlay> {
        let mut overlays = Vec::new();
        if !self.profile.guide.notes.trim().is_empty() {
            let rows = self.profile.guide.notes.lines().map(|line| vec![line.to_string()]).collect();
            let mut notes = panel("guide notes", ScreenPosition::CenterLeft, "Notes".into(), rows);
            notes.style.max_width = NOTES_MAX_WIDTH;
            overlays.push(notes);
        }
        let mapped = self.mapped();
        let rows: Vec<Vec<String>> = mapping_rows(&mapped).iter().map(cells).collect();
        let sections = self.sections(&mapped);
        let pages = layout(rows.len(), &sections);
        let at = self.page.min(pages.len() - 1);
        let page = &pages[at];
        overlays.push(panel("guide mappings", ScreenPosition::CenterRight, self.title(at, pages.len()), rows[page.rows.clone()].to_vec()));
        for &i in &page.sections {
            let section = &sections[i];
            overlays.push(panel(&format!("guide layer {i}"), ScreenPosition::CenterRight, section.title.clone(), section.rows.clone()));
        }
        overlays
    }

    /// How many pages the Mappings take in the shown mode.
    pub fn pages(&self) -> usize {
        let mapped = self.mapped();
        layout(mapping_rows(&mapped).len(), &self.sections(&mapped)).len()
    }

    /// "Mappings", then which mode it is when the game has modes, then which page.
    fn title(&self, page: usize, pages: usize) -> String {
        let mut title = "Mappings".to_string();
        if pages > 1 {
            title.push_str(&format!(" {}/{pages} · {{rs}} more", page + 1));
        }
        if !self.modes.is_empty() {
            let name = self.mode().map_or(DEFAULT_MODE, |m| m.name.as_str());
            // "Mouse mode" is named a mode already.
            let mode = if name.to_lowercase().ends_with("mode") { "" } else { " mode" };
            let off = if self.shown == self.current { "" } else { " (not on)" };
            title.push_str(&format!(" · {name}{mode}{off} · {{left}} {{right}} modes"));
        }
        title
    }

    /// A section for each of the game's layers that an input of `mapped` holds, in the order
    /// they're first held.
    fn sections(&self, mapped: &Profile) -> Vec<Section> {
        let mut held: Vec<(String, Vec<String>)> = Vec::new();
        for (name, holder) in layer_holders(mapped) {
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
                Some(Section { title: format!("{name} layer · {}", holders.join(", ")), rows })
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

/// Every layer an input of `p` holds, with that input's glyphs (a gesture's or combo's marked
/// like their Mappings rows), in the order of the Mappings: buttons, gestures, combos, triggers.
fn layer_holders(p: &Profile) -> Vec<(String, String)> {
    let glyph = |b: Button| input_token(GuideInput::Button(b)).map(|t| format!("{{{t}}}"));
    let mut holders = Vec::new();
    let mut add = |action: &ButtonAction, shape: RowShape, glyphs: Option<Vec<String>>| {
        let Some(glyphs) = glyphs else { return };
        action.walk(&mut |a| {
            if let ButtonAction::Layer(name) = a {
                holders.push((name.clone(), shape.join(&glyphs)));
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
