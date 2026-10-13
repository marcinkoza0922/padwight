//! The App settings card that hides Padwight's controller from Steam (see [`crate::steam_hide`]).

use iced::widget::column;

use super::*;
use crate::steam_hide::{self, Status};

impl App {
    pub(super) fn view_steam_card(&self) -> Element<'_, Message> {
        let about = "Some games, such as Slay the Spire 2, read the stick through Steam whenever Steam reports a \
                     controller, and get nothing from it while Steam Input is off for them. Hidden from Steam, \
                     Padwight's controller reaches those games directly. Games started from Steam still see it, \
                     but Steam's own menus and Big Picture don't, nor do Sony and Nintendo controllers Padwight \
                     doesn't manage.";
        let body: Element<'_, Message> = match steam_hide::status() {
            Status::Unavailable(why) => text(why).color(MUTED_COLOR).into(),
            status => {
                let hidden = status == Status::Hidden;
                column![
                    checkbox(hidden).label("Hide Padwight's controller from Steam").on_toggle(Message::SetHideFromSteam),
                    text("Takes effect the next time Steam starts.").size(13).color(MUTED_COLOR),
                ]
                .spacing(6)
                .into()
            }
        };
        section("Steam", Some(about.into()), vec![body])
    }

    pub(super) fn set_hide_from_steam(&mut self, hidden: bool) {
        self.message = Some(match steam_hide::set(hidden) {
            Ok(()) if hidden => ("Steam will leave Padwight's controller alone once it's restarted.".into(), false),
            Ok(()) => ("Steam will see Padwight's controller again once it's restarted.".into(), false),
            Err(e) => (format!("{e:#}"), true),
        });
    }
}
