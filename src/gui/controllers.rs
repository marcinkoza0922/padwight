//! The remembered controllers on the Overview: nickname, colour, player label, passing through,
//! and forgetting one. They stay listed, greyed out, while disconnected.

use iced::widget::{column, row};

use super::*;
use crate::ipc::ControllerInfo;

/// Colours a controller can be marked with: (name, `#rrggbb`).
const COLORS: [(&str, &str); 5] = [("Blue", "#4f8cff"), ("Red", "#e5484d"), ("Green", "#30a46c"), ("Yellow", "#f5d90a"), ("Purple", "#8e4ec6")];

/// An entry of the colour list.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ColorChoice(Option<String>);

impl fmt::Display for ColorChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_deref().and_then(|hex| COLORS.iter().find(|(_, h)| *h == hex)) {
            Some((name, _)) => f.write_str(name),
            None => f.write_str("No colour"),
        }
    }
}

/// An entry of the player list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlayerChoice(Option<u8>);

impl fmt::Display for PlayerChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(n) => write!(f, "Player {n}"),
            None => f.write_str("No player"),
        }
    }
}

/// `#rrggbb` as a colour.
fn parse_hex(hex: &str) -> Option<Color> {
    let n = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
    Some(Color::from_rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

impl App {
    /// The list of remembered controllers, empty when there are none.
    pub(super) fn view_remembered(&self) -> Option<Element<'_, Message>> {
        let status = self.status.as_ref().filter(|s| !s.controllers.is_empty())?;
        let mut list = column![text("Remembered controllers").size(20)].spacing(8);
        for c in &status.controllers {
            list = list.push(self.view_remembered_one(c));
        }
        Some(list.into())
    }

    fn view_remembered_one<'a>(&'a self, c: &'a ControllerInfo) -> Element<'a, Message> {
        let shade = if c.connected { None } else { Some(MUTED_COLOR) };
        let name = if c.name.is_empty() { c.id.as_str() } else { c.name.as_str() };
        let state = match (c.connected, c.pending) {
            (false, _) => "not connected",
            (true, true) => "waiting for its set-up question",
            (true, false) => "connected",
        };
        let swatch = c.color.as_deref().and_then(parse_hex);
        let mut title = row![].spacing(8).align_y(Alignment::Center);
        if let Some(color) = swatch {
            title = title.push(text("●").color(color));
        }
        let mut label = text(name);
        if let Some(shade) = shade {
            label = label.color(shade);
        }
        title = title.push(label).push(text(state).size(12).color(MUTED_COLOR));

        let draft = self.nick_drafts.get(&c.id).unwrap_or(&c.nickname);
        let id = c.id.clone();
        let current = c.clone();
        let nickname = field("Nickname", draft)
            .on_input(move |s| Message::ControllerNick(id.clone(), s))
            .on_submit(Message::SetController(ControllerInfo { nickname: draft.clone(), ..current.clone() }))
            .size(14)
            .width(180);
        let colors: Vec<ColorChoice> = std::iter::once(ColorChoice(None)).chain(COLORS.iter().map(|(_, h)| ColorChoice(Some((*h).into())))).collect();
        let with = c.clone();
        let color = pick_list(colors, Some(ColorChoice(c.color.clone())), move |p| {
            Message::SetController(ControllerInfo { color: p.0, ..with.clone() })
        })
        .padding([6, 10]);
        let players: Vec<PlayerChoice> = std::iter::once(PlayerChoice(None)).chain((1..=4).map(|n| PlayerChoice(Some(n)))).collect();
        let with = c.clone();
        let player = pick_list(players, Some(PlayerChoice(c.player)), move |p| {
            Message::SetController(ControllerInfo { player: p.0, ..with.clone() })
        })
        .padding([6, 10]);
        let with = c.clone();
        let pass = checkbox(c.pass_through)
            .label("Pass through")
            .on_toggle(move |on| Message::SetController(ControllerInfo { pass_through: on, ..with.clone() }));
        let forget = button(text("Forget").size(13)).style(style::secondary).on_press(Message::ForgetController(c.id.clone()));
        column![title, row![nickname, color, player, pass, space::horizontal(), forget].spacing(8).align_y(Alignment::Center)]
            .spacing(6)
            .into()
    }
}
