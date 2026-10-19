//! The overlay on the stage, drawn with the overlay process's own code.

use super::*;

impl App {
    /// The overlay as the overlay process draws it, at its real size on this display, with
    /// made-up contents where the real ones come from the game (a log's presses, the media playing).
    pub(super) fn stage_panel<'a>(&self, target: StageTarget, style: &OverlayStyle, fit: &Fit, anim: &Anim) -> Option<Element<'a, Message>> {
        use crate::overlay::draw;
        let font = self.preview_font();
        let look = crate::overlay::draw::MenuLook { anim: *anim, ..self.menu_look() };
        Some(match target {
            StageTarget::Menu(i) => {
                let menu = self.menus().get(i)?;
                let menus = std::slice::from_ref(menu);
                let opener = Opener { buttons: vec![Button::LeftBumper], ..Opener::default() };
                match MenuSession::open(menus, &menu.name, opener).and_then(|s| s.view(menus)) {
                    Some(view) => draw::menu_panel(&view, font, look, fit),
                    None => placeholder("This menu has nothing to show yet: give it a name and an item."),
                }
            }
            StageTarget::Info(i) => {
                let live = crate::info::Live::sample(self.config.info_glyphs).with_layout(self.nintendo_layout());
                draw::info_panel(&crate::info::resolve(self.infos().get(i)?, &live), font, fit, anim)
            }
            StageTarget::Log(i) => {
                let o = self.logs().get(i)?;
                let crate::config::LogSource::Input(source) = &o.source;
                let sample = crate::inputlog::Inputs::sample();
                let now = sample.now.unwrap_or_else(Instant::now);
                draw::log_panel(&crate::inputlog::log_view(&sample.entries(None), source, style, self.glyphs(), now), font, fit, anim)
            }
            StageTarget::Panel(panel) | StageTarget::Setup(panel) => match panel {
                Panel::Keyboard | Panel::Numpad => {
                    let board = match panel {
                        Panel::Numpad => crate::keyboard::Board::Numpad,
                        _ if self.config.full_keyboard => crate::keyboard::Board::Full,
                        _ => crate::keyboard::Board::Basic,
                    };
                    let view = crate::overlay::KeyboardView {
                        board,
                        style: style.clone(),
                        cursor: crate::keyboard::find(board, "KEY_H").unwrap_or_default(),
                        latched: Vec::new(),
                        pressed: None,
                        closing: 0.0,
                        caps: false,
                        typed: String::new(),
                        field: None,
                    };
                    draw::keyboard_panel(&view, font, self.glyphs(), fit, anim)
                }
                Panel::Media => draw::media_panel(&crate::media::MediaView::sample(style.clone()), font, self.glyphs(), fit, anim),
                Panel::InGameMenu => draw::menu_panel(&crate::system_menu::main_page(style.clone(), 0), font, look, fit),
            },
        })
    }
}

/// Something to hold on to where an overlay draws nothing yet.
fn placeholder<'a>(note: &'a str) -> Element<'a, Message> {
    container(text(note).size(16).color(Color::WHITE))
        .padding(24)
        .style(|_: &iced::Theme| container::Style { background: Some(Color::from_rgba(0.1, 0.1, 0.12, 0.85).into()), ..container::Style::default() })
        .into()
}

