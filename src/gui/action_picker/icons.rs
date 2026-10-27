//! Each action kind's icon in the action dialog and on the button showing the pick.

use super::*;
use crate::{config::Keyword, keyword_icon};

/// The kind's icon: a keyword's own, in its color, as the summaries draw it, or a plain one.
pub(super) fn icon<'a>(k: ActionKind, size: f32) -> Element<'a, Message> {
    let keyword = match k {
        ActionKind::Toggle => Some(Keyword::Toggle),
        ActionKind::Turbo => Some(Keyword::Turbo),
        ActionKind::Macro => Some(Keyword::Macro),
        ActionKind::Cycle => Some(Keyword::Cycle),
        ActionKind::Menu => Some(Keyword::Menu),
        ActionKind::Info => Some(Keyword::Info),
        ActionKind::Log => Some(Keyword::Log),
        ActionKind::Layer => Some(Keyword::Layer),
        ActionKind::Mode => Some(Keyword::Mode),
        _ => None,
    };
    let svg_text = keyword.map_or_else(|| plain_icon(k), keyword_icon::svg);
    svg(svg::Handle::from_memory(svg_text.into_bytes())).width(size).height(size).into()
}

/// The kinds without a keyword: a line drawing in a neutral gray that reads in either theme.
fn plain_icon(k: ActionKind) -> String {
    let body = match k {
        ActionKind::Keys => "<rect x='3' y='6' width='18' height='12' rx='2.5'/><path d='M7 10h.01M11 10h.01M15 10h.01M8 14h8'/>",
        ActionKind::Overlay => "<rect x='2' y='5' width='20' height='14' rx='2.5'/><path d='M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12.5h.01M18 12.5h.01M8 16h8'/>",
        ActionKind::Mouse => "<rect x='6' y='3' width='12' height='18' rx='6'/><path d='M12 3v7'/>",
        ActionKind::Wheel => "<path d='M12 4v16M8 8l4-4 4 4M8 16l4 4 4-4'/>",
        ActionKind::Gamepad => "<path d='M7 7h10a5 5 0 0 1 5 5v1.5a3 3 0 0 1-5.6 1.5L15 13H9l-1.4 2A3 3 0 0 1 2 13.5V12a5 5 0 0 1 5-5Z'/><path d='M7 9.5v3M5.5 11h3M16 10h.01M18 12h.01'/>",
        ActionKind::PadTrigger => "<path d='M7 3h10v6a5 5 0 0 1-10 0Z'/><path d='M12 9v5M9 21h6'/>",
        ActionKind::PadStick => "<circle cx='12' cy='12' r='9'/><circle cx='15' cy='9' r='3' fill='#8a8f98'/><path d='M12 12l3-3'/>",
        ActionKind::Disabled => "<circle cx='12' cy='12' r='8.5'/><path d='M6 18 18 6'/>",
        ActionKind::Numpad => "<path d='M6 6h.01M12 6h.01M18 6h.01M6 12h.01M12 12h.01M18 12h.01M6 18h.01M12 18h.01M18 18h.01' stroke-width='3.5'/>",
        ActionKind::QuickSettings => "<path d='M4 7h16M4 17h16'/><circle cx='9' cy='7' r='2.2' fill='#8a8f98'/><circle cx='15' cy='17' r='2.2' fill='#8a8f98'/>",
        ActionKind::Media => "<path d='M8 5v14l11-7Z'/>",
        // Two outputs stacked: one press, both sent.
        ActionKind::Multiple => "<rect x='3' y='8' width='13' height='13' rx='2'/><path d='M8 8V5a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2h-3'/>",
        ActionKind::GyroToggle | ActionKind::GyroClutch | ActionKind::GyroHold | ActionKind::GyroRecenter => {
            "<circle cx='12' cy='12' r='8.5'/><ellipse cx='12' cy='12' rx='8.5' ry='3.5'/><circle cx='12' cy='12' r='1.5' fill='#8a8f98'/>"
        }
        ActionKind::Screenshot => "<path d='M4 8h3l2-3h6l2 3h3v11H4Z'/><circle cx='12' cy='13' r='3.5'/>",
        ActionKind::Recording => "<circle cx='12' cy='12' r='8.5'/><circle cx='12' cy='12' r='4' fill='#8a8f98'/>",
        ActionKind::ForceQuit => "<path d='M12 3v8M6.3 6.3a8 8 0 1 0 11.4 0'/>",
        _ => "",
    };
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='#8a8f98' stroke-width='2' \
         stroke-linecap='round' stroke-linejoin='round'>{body}</svg>"
    )
}
