//! A game's own look for the on-screen keyboard and numpad, shown on its Details tab.

use iced::widget::column;

use super::*;
use crate::{
    keyboard::Layout,
    motion::{MotionSet, MotionStyle, OverlayKind},
    overlay::KeyboardView,
};

const FONT_HELP: &str = "One font for every overlay, menu and the on-screen keyboard and numpad. Pick one of the \
                         free fonts that come with the app. (A pack can also name any font installed on your system.)";

impl App {
    /// The font overlay previews are drawn in: the shown game's, else the global one.
    pub(super) fn preview_font(&self) -> iced::Font {
        let own = match &self.page {
            Page::Game(_) => self.game().overlay_font.as_deref(),
            _ => None,
        };
        crate::font::resolve(own.or(self.config.overlay_font.as_deref()))
    }

    /// The App settings page's font card: used by every game that doesn't pick its own.
    pub(super) fn view_font_card(&self) -> Element<'_, Message> {
        section(
            "Overlay font",
            Some(FONT_HELP.into()),
            vec![labeled("Font", self.font_picker(FontSlot::Global, "System default", self.config.overlay_font.as_deref(), Message::SetOverlayFont))],
        )
    }

    /// How menu previews are drawn: the color-blind setting, and the controller the info
    /// glyphs are drawn for when none is detected.
    /// What this game could draw at `position` at the same time as something there. The
    /// keyboard, numpad, media controls and menus are never up together, but any of them can be
    /// up with info and log overlays: so a `panel` shares its spot with the overlays there, and
    /// an overlay with the panels (overlays at one spot stack rather than overlap).
    pub(super) fn sharing_spot(&self, position: crate::config::ScreenPosition, panel: bool) -> Vec<String> {
        let game = self.game();
        let found: Vec<(String, crate::config::ScreenPosition)> = if panel {
            let info = game.info.iter().map(|o| (format!("info overlay “{}”", o.name), o.style.position));
            info.chain(game.logs.iter().map(|o| (format!("log “{}”", o.name), o.style.position))).collect()
        } else {
            let fixed = [
                ("the on-screen keyboard", game.keyboard_style.as_ref().unwrap_or(&self.config.keyboard_style)),
                ("the on-screen numpad", game.numpad_style.as_ref().unwrap_or(&self.config.numpad_style)),
                ("the media controls", game.media_style.as_ref().unwrap_or(&self.config.media_style)),
                ("the in-game menu", game.menu_style.as_ref().unwrap_or(&self.config.menu_style)),
            ]
            .map(|(name, style)| (name.to_string(), style.position));
            fixed.into_iter().chain(game.menus.iter().map(|m| (format!("menu “{}”", m.name), m.style.position))).collect()
        };
        found.into_iter().filter(|(_, at)| *at == position).map(|(name, _)| name).collect()
    }

    /// A note under an Appearance section when something that can be on screen at the same time
    /// is drawn at the same spot. Not an error: the two may never be up at once.
    pub(super) fn spot_note<'a>(&self, position: crate::config::ScreenPosition, panel: bool) -> Option<Element<'a, Message>> {
        let others = self.sharing_spot(position, panel);
        (!others.is_empty()).then(|| {
            let note = format!("⚠ {position} also has {}: they overlap if they're on screen together.", others.join(", "));
            text(note).size(12).color(WARNING_COLOR).into()
        })
    }

    /// How previews draw button glyphs: the Settings page's glyphs, with the Nintendo layout.
    pub(super) fn glyphs(&self) -> crate::info::Glyphs {
        crate::info::Glyphs { family: self.config.info_glyphs, nintendo_layout: self.nintendo_layout() }
    }

    pub(super) fn menu_look(&self) -> crate::overlay::draw::MenuLook {
        crate::overlay::draw::MenuLook {
            anim: Default::default(),
            colorblind: self.config.colorblind_tones,
            family: self.config.info_glyphs,
            nintendo_layout: self.nintendo_layout(),
        }
    }

    /// The App settings page's color card: the tints of added and removed rows.
    pub(super) fn view_color_card(&self) -> Element<'_, Message> {
        section(
            "Menu colors",
            Some("Rows that add to a list are tinted, and rows that remove from one are tinted red. Color-blind mode uses blue and orange instead, and puts a + or − in front of each row.".into()),
            vec![toggler(self.config.colorblind_tones).label("Color-blind mode").on_toggle(Message::SetColorblindTones).into()],
        )
    }

    /// How each kind of overlay moves, for every setup that doesn't set its own.
    pub(super) fn view_motion_card(&self) -> Element<'_, Message> {
        section(
            "Overlay motion",
            Some(format!("How overlays move as they open and close, as the cursor moves and as something is picked. Each kind of overlay has its own style, and a setup can set its own on its Details tab. {}", motion_guide())),
            {
                let mut rows = motion_rows(self.config.motion, Message::SetMotion);
                rows.push(self.view_motion_demo());
                rows
            },
        )
    }

    /// The setup's own font. An empty name is its own choice of the system default, which `None`
    /// (follow App settings) can't say.
    pub(super) fn view_game_font(&self) -> Element<'_, Message> {
        let own = self.game().overlay_font.as_deref();
        let global = self.config.overlay_font.clone();
        let mut rows = own_rows("font", "the font set on the App settings page", own.is_some(), move |on| Message::SetGameOverlayFont(on.then(|| global.clone().unwrap_or_default())));
        if let Some(own) = own {
            let picker = self.font_picker(FontSlot::Game, "System default", Some(own).filter(|f| !f.is_empty()), |f| Message::SetGameOverlayFont(Some(f.unwrap_or_default())));
            rows.push(labeled("Font", picker));
        }
        section("Overlay font", Some(FONT_HELP.into()), rows)
    }

    /// Sets (or with `None` clears) the shown game's own style for a layout.
    pub(super) fn set_game_overlay_style(&mut self, layout: Layout, style: Option<OverlayStyle>) {
        let game = self.game_mut();
        match layout {
            Layout::Keyboard => game.keyboard_style = style,
            Layout::Numpad => game.numpad_style = style,
        }
    }

    /// The keyboard, numpad, media controls and in-game menu cards: each follows the Settings
    /// page until given its own look.
    pub(super) fn view_game_overlays(&self) -> Element<'_, Message> {
        let game = self.game();
        let font = self.preview_font();
        let look = self.menu_look();
        let glyphs = self.glyphs();
        column![
            game_overlay_card(
                stage::Panel::Keyboard,
                game.keyboard_style.as_ref(),
                &self.config.keyboard_style,
                |s| Message::SetGameOverlayStyle(Layout::Keyboard, s),
                move |style| keyboard_preview(Layout::Keyboard, style, font, glyphs),
            ),
            game_overlay_card(
                stage::Panel::Numpad,
                game.numpad_style.as_ref(),
                &self.config.numpad_style,
                |s| Message::SetGameOverlayStyle(Layout::Numpad, s),
                move |style| keyboard_preview(Layout::Numpad, style, font, glyphs),
            ),
            game_overlay_card(
                stage::Panel::Media,
                game.media_style.as_ref(),
                &self.config.media_style,
                Message::SetGameMediaStyle,
                move |style| preview(crate::overlay::draw::media_panel(&crate::media::MediaView::sample(preview_style(style)), font, glyphs, &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still())),
            ),
            game_overlay_card(
                stage::Panel::InGameMenu,
                game.menu_style.as_ref(),
                &self.config.menu_style,
                Message::SetGameMenuStyle,
                move |style| {
                    let sample = crate::system_menu::main_page(preview_style(style), 0);
                    preview(crate::overlay::draw::menu_panel(&sample, font, look, &crate::overlay::fit::Fit::contents()))
                },
            ),
        ]
        .spacing(16)
        .into()
    }
}

/// One panel's card: whether the game has its own look (`own`), that look's editor and a
/// preview, or a note that it follows `global`. `set` makes the message for a new look.
fn game_overlay_card<'a>(
    panel: stage::Panel,
    own: Option<&'a OverlayStyle>,
    global: &'a OverlayStyle,
    set: impl Fn(Option<OverlayStyle>) -> Message + 'a,
    sample: impl Fn(&OverlayStyle) -> Element<'static, Message>,
) -> Element<'a, Message> {
    let set = Rc::new(set);
    let on_style = {
        let set = set.clone();
        Rc::new(move |s| set(Some(s))) as OnStyle<'a>
    };
    let mut rows = own_rows("appearance", "the appearance set on the App settings page", own.is_some(), move |on| set(on.then(|| global.clone())));
    if let Some(style) = own {
        rows.push(stage::edit_button(StageTarget::Setup(panel)));
        rows.push(style_editor(style, on_style));
        rows.push(sample(style));
    }
    section(panel.name(), Some("A setup can have its own look here, or follow the one set on the App settings page.".into()), rows)
}

/// A keyboard or numpad drawn with `style`, with the cursor on a key, for the card's preview.
fn keyboard_preview(layout: Layout, style: &OverlayStyle, font: iced::Font, glyphs: crate::info::Glyphs) -> Element<'static, Message> {
    let sample = KeyboardView {
        board: layout.into(),
        style: preview_style(style),
        cursor: match layout {
            Layout::Keyboard => crate::keyboard::find(layout, "KEY_H").unwrap_or_default(),
            Layout::Numpad => layout.home(),
        },
        latched: Vec::new(),
        pressed: None,
        closing: 0.0,
        caps: false,
        typed: String::new(),
        field: None,
    };
    preview(crate::overlay::draw::keyboard_panel(&sample, font, glyphs, &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still()))
}

/// What each motion style is like, for the settings to explain them.
pub(super) fn motion_guide() -> String {
    let styles = MotionStyle::ALL.iter().map(|style| format!("{style}: {}", style.describe())).collect::<Vec<_>>().join(" ");
    format!("{styles} Guide menus move as Menus do.")
}

/// One row per kind of overlay, each with a dropdown for its style. `change` makes the message
/// for a new style, so the same rows serve the global set and a setup's own.
pub(super) fn motion_rows<'a>(set: MotionSet, change: fn(OverlayKind, MotionStyle) -> Message) -> Vec<Element<'a, Message>> {
    OverlayKind::ALL
        .into_iter()
        .map(|kind| {
            let style = set.get(kind);
            labeled(
                kind.label(),
                dropdown(MotionStyle::ALL, Some(style), move |style| change(kind, style)).width(SETTING_WIDTH).into(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::ScreenPosition, gui::tests::*};

    #[test]
    fn panels_and_overlays_at_one_spot_are_pointed_out_but_panels_together_are_not() {
        let mut app = with_game();
        let bottom = app.config.keyboard_style.position;
        assert!(app.sharing_spot(bottom, true).is_empty(), "the numpad never shows with the keyboard");
        let _ = app.update(Message::NewInfo);
        let info = &mut app.game_mut().info[0];
        info.name = "Ammo".into();
        info.style.position = bottom;
        assert_eq!(app.sharing_spot(bottom, true), ["info overlay “Ammo”"]);
        assert!(app.sharing_spot(bottom, false).iter().any(|n| n == "the on-screen keyboard"));
    }

    #[test]
    fn a_game_can_override_the_keyboard_and_numpad_styles() {
        let mut app = with_game();
        let style = OverlayStyle { position: ScreenPosition::TopLeft, ..OverlayStyle::default() };
        let _ = app.update(Message::SetGameOverlayStyle(Layout::Keyboard, Some(style.clone())));
        let doom = app.config.games.iter().find(|g| g.name == "Doom").unwrap();
        assert_eq!(doom.keyboard_style, Some(style));
        assert_eq!(doom.numpad_style, None);
        let _ = app.update(Message::SetGameOverlayStyle(Layout::Keyboard, None));
        assert!(app.config.games.iter().find(|g| g.name == "Doom").unwrap().keyboard_style.is_none());
    }

    #[test]
    fn a_game_can_override_the_media_and_in_game_menu_looks() {
        let mut app = with_game();
        let style = OverlayStyle { position: ScreenPosition::BottomLeft, ..OverlayStyle::default() };
        let _ = app.update(Message::SetGameMediaStyle(Some(style.clone())));
        let _ = app.update(Message::SetGameMenuStyle(Some(style.clone())));
        let doom = app.config.games.iter().find(|g| g.name == "Doom").unwrap();
        assert_eq!((doom.media_style.clone(), doom.menu_style.clone()), (Some(style.clone()), Some(style.clone())));
        assert_eq!(doom.keyboard_style, None);
        let _ = app.update(Message::SetGameMediaStyle(None));
        assert!(app.config.games.iter().find(|g| g.name == "Doom").unwrap().media_style.is_none());
    }

    #[test]
    fn the_font_is_one_setting_per_game_over_a_global_one() {
        let mut app = with_game();
        let _ = app.update(Message::SetOverlayFont(Some("Inter".into())));
        assert_eq!(app.config.active_font(), Some("Inter"));
        let _ = app.update(Message::SetGameOverlayFont(Some("Comfortaa".into())));
        let doom = app.config.games.iter().find(|g| g.name == "Doom").unwrap();
        assert_eq!(doom.overlay_font.as_deref(), Some("Comfortaa"));
        assert_eq!(app.config.overlay_font.as_deref(), Some("Inter"));
    }

    #[test]
    fn a_game_can_pick_the_system_font_over_a_global_one() {
        let mut app = with_game();
        let _ = app.update(Message::SetOverlayFont(Some("Inter".into())));
        let _ = app.update(Message::SelectPage(Page::Game(Some("Doom".into()))));
        // The setup's own font picker gives back an empty name for "System default".
        let _ = app.update(Message::SetGameOverlayFont(Some(String::new())));
        assert_eq!(crate::font::resolve(app.game().overlay_font.as_deref().or(app.config.overlay_font.as_deref())), iced::Font::DEFAULT);
        let _ = app.update(Message::SetGameOverlayFont(None));
        assert_eq!(app.game().overlay_font, None);
    }

    #[test]
    fn a_games_own_motion_starts_as_the_global_one() {
        let mut app = with_game();
        let _ = app.update(Message::SelectPage(Page::Game(Some("Doom".into()))));
        let _ = app.update(Message::OwnGameMotion(true));
        assert_eq!(app.game().motion, Some(app.config.motion));
        let _ = app.update(Message::OwnGameMotion(false));
        assert_eq!(app.game().motion, None);
    }
}
