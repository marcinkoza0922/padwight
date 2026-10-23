//! The character picker: a window of emoji and symbols by group, searchable by name, that puts
//! the one clicked at the end of the text field it was opened from.

use iced::widget::{button, center, column, container, mouse_area, opaque, row, scrollable, space, text};

use super::{tokens::TokenTarget, *};

/// Groups of characters, each as the character and the words that find it. All single code
/// points, so one click is one glyph whatever the font.
const GROUPS: [(&str, &[(&str, &str)]); 8] = [
    ("Faces", &[
        ("😀", "grin happy"), ("😃", "smile open"), ("😄", "smile eyes"), ("😁", "beam"), ("😆", "laugh squint"), ("😅", "sweat smile"),
        ("😂", "joy tears laugh"), ("🙂", "slight smile"), ("😉", "wink"), ("😊", "blush"), ("😇", "angel halo"), ("😍", "heart eyes love"),
        ("😘", "kiss"), ("😋", "yum tasty"), ("😛", "tongue"), ("😜", "wink tongue"), ("🤪", "zany crazy"), ("🤔", "thinking hmm"),
        ("😐", "neutral"), ("😑", "expressionless"), ("🙄", "eye roll"), ("😏", "smirk"), ("😴", "sleep zzz"), ("😎", "cool sunglasses"),
        ("🤓", "nerd"), ("😕", "confused"), ("😟", "worried"), ("😮", "surprised open mouth"), ("😲", "astonished"), ("😳", "flushed"),
        ("😢", "cry sad"), ("😭", "sob loud cry"), ("😱", "scream fear"), ("😖", "confounded"), ("😞", "disappointed"), ("😤", "triumph huff"),
        ("😡", "angry mad"), ("🤬", "swearing cursing"), ("💀", "skull dead"), ("☠", "skull crossbones poison"), ("👻", "ghost"), ("👽", "alien"),
        ("🤖", "robot"), ("😈", "devil imp"), ("👿", "angry devil"), ("🤡", "clown"), ("💩", "poo"), ("🥶", "cold freezing"), ("🥵", "hot"),
    ]),
    ("Hands & people", &[
        ("👍", "thumbs up like yes"), ("👎", "thumbs down dislike no"), ("👌", "ok"), ("✌", "peace victory"), ("🤞", "fingers crossed luck"),
        ("🤘", "rock horns"), ("👈", "point left"), ("👉", "point right"), ("👆", "point up"), ("👇", "point down"), ("☝", "index up"),
        ("✋", "hand stop"), ("👋", "wave hello"), ("👏", "clap"), ("🙌", "raised hands"), ("🙏", "pray please thanks"), ("💪", "muscle strong"),
        ("✊", "fist"), ("👊", "punch"), ("🧠", "brain"), ("👀", "eyes look"), ("👁", "eye"), ("👂", "ear"), ("👃", "nose"), ("👄", "mouth"),
        ("🧙", "mage wizard"), ("🧛", "vampire"), ("🧟", "zombie"), ("🥷", "ninja"), ("🦸", "hero"), ("🦹", "villain"), ("👑", "crown king"),
    ]),
    ("Game & fantasy", &[
        ("⚔", "swords crossed attack"), ("🗡", "dagger knife"), ("🏹", "bow arrow"), ("🛡", "shield defense"), ("🔫", "gun pistol"),
        ("💣", "bomb"), ("🧨", "dynamite firecracker"), ("🪓", "axe"), ("🔨", "hammer"), ("⛏", "pick mine"), ("🪄", "wand magic"),
        ("🔮", "crystal ball magic"), ("🧪", "potion flask"), ("⚗", "alembic"), ("📜", "scroll"), ("📖", "book"), ("🗝", "key old"),
        ("🔑", "key"), ("🔒", "lock"), ("🔓", "unlock"), ("💎", "gem diamond"), ("💰", "money bag gold"), ("🪙", "coin"), ("🏆", "trophy win"),
        ("🥇", "gold medal first"), ("🥈", "silver medal second"), ("🥉", "bronze medal third"), ("🎮", "gamepad controller game"),
        ("🕹", "joystick"), ("🎲", "dice"), ("🎯", "target bullseye"), ("🧩", "puzzle piece"), ("♟", "chess pawn"), ("🃏", "joker card"),
        ("🐉", "dragon"), ("🦄", "unicorn"), ("🐺", "wolf"), ("🦇", "bat"), ("🕷", "spider"), ("🐍", "snake"), ("🦅", "eagle"),
        ("🏰", "castle"), ("⛺", "tent camp"), ("🔥", "fire flame hot"), ("💧", "water drop"), ("⚡", "lightning bolt energy"),
        ("❄", "snowflake ice"), ("🌪", "tornado"), ("🌊", "wave sea"), ("🌋", "volcano"), ("⭐", "star favorite"), ("🌙", "moon night"),
        ("☀", "sun day"), ("🍖", "meat food"), ("🍎", "apple food"), ("🍺", "beer drink"), ("🍷", "wine drink"), ("🚗", "car drive"),
        ("🚀", "rocket fast"), ("✈", "plane airplane"), ("⛵", "boat sail"), ("🏠", "house home"),
    ]),
    ("Hearts & marks", &[
        ("❤", "red heart love"), ("🧡", "orange heart"), ("💛", "yellow heart"), ("💚", "green heart"), ("💙", "blue heart"), ("💜", "purple heart"),
        ("🖤", "black heart"), ("🤍", "white heart"), ("💔", "broken heart"), ("💯", "hundred"), ("✅", "check done yes"), ("✔", "tick check"),
        ("❌", "cross no wrong"), ("✖", "multiply cross"), ("❓", "question"), ("❗", "exclamation warning"), ("⚠", "warning caution"),
        ("⛔", "no entry stop"), ("🚫", "prohibited"), ("ℹ", "info"), ("✨", "sparkles new"), ("💫", "dizzy"), ("💥", "boom collision"),
        ("💢", "anger"), ("💤", "sleep zzz"), ("🔔", "bell alert"), ("🔕", "bell off mute"), ("🔊", "speaker loud sound"), ("🔇", "muted"),
        ("🎵", "music note"), ("🎶", "music notes"), ("📌", "pin"), ("📍", "location pin"), ("🔖", "bookmark"), ("🏁", "finish flag"),
        ("🚩", "red flag"), ("🏳", "white flag"),
    ]),
    ("Arrows", &[
        ("←", "left arrow"), ("↑", "up arrow"), ("→", "right arrow"), ("↓", "down arrow"), ("↔", "left right arrow"), ("↕", "up down arrow"),
        ("↖", "up left arrow"), ("↗", "up right arrow"), ("↘", "down right arrow"), ("↙", "down left arrow"), ("⇐", "double left arrow"),
        ("⇑", "double up arrow"), ("⇒", "double right arrow"), ("⇓", "double down arrow"), ("⇔", "double left right arrow"), ("↩", "return left hook"),
        ("↪", "right hook"), ("↺", "anticlockwise rotate undo"), ("↻", "clockwise rotate redo"), ("⟲", "anticlockwise gapped"), ("⟳", "clockwise gapped"),
        ("➜", "heavy arrow"), ("➤", "arrowhead"), ("➔", "wide arrow"), ("▲", "triangle up"), ("▼", "triangle down"), ("◀", "triangle left"),
        ("▶", "triangle right"), ("△", "outline triangle up"), ("▽", "outline triangle down"), ("◁", "outline triangle left"), ("▷", "outline triangle right"),
        ("⏎", "return enter"), ("⇥", "tab"), ("⇤", "back tab"), ("⌫", "backspace delete"), ("⌦", "delete forward"), ("⇧", "shift"),
        ("⌃", "control ctrl"), ("⌥", "option alt"), ("⌘", "command"), ("⎋", "escape"),
    ]),
    ("Shapes & stars", &[
        ("●", "circle filled dot"), ("○", "circle outline"), ("◉", "fisheye"), ("◎", "bullseye"), ("◐", "half circle"), ("◑", "half circle right"),
        ("■", "square filled"), ("□", "square outline"), ("▣", "square dot"), ("◆", "diamond filled"), ("◇", "diamond outline"), ("◈", "diamond dot"),
        ("★", "star filled"), ("☆", "star outline"), ("✦", "star four"), ("✧", "star four outline"), ("✪", "star circled"), ("✯", "star pinwheel"),
        ("✱", "asterisk"), ("✶", "star six"), ("❂", "sun circle"), ("❖", "diamond four"), ("◢", "triangle corner"), ("◣", "triangle corner left"),
        ("•", "bullet dot"), ("‣", "triangular bullet"), ("◦", "white bullet"), ("∙", "bullet operator"), ("·", "middle dot"), ("…", "ellipsis"),
        ("─", "line horizontal"), ("│", "line vertical"), ("┌", "corner top left"), ("┐", "corner top right"), ("└", "corner bottom left"),
        ("┘", "corner bottom right"), ("├", "tee right"), ("┤", "tee left"), ("┬", "tee down"), ("┴", "tee up"), ("┼", "cross"),
        ("═", "double horizontal"), ("║", "double vertical"), ("░", "light shade"), ("▒", "medium shade"), ("▓", "dark shade"), ("█", "full block"),
        ("▀", "upper half block"), ("▄", "lower half block"),
    ]),
    ("Math & units", &[
        ("±", "plus minus"), ("×", "times multiply"), ("÷", "divide"), ("−", "minus"), ("≈", "approximately"), ("≠", "not equal"),
        ("≤", "less equal"), ("≥", "greater equal"), ("∞", "infinity"), ("√", "square root"), ("∑", "sum"), ("∆", "delta increment"),
        ("π", "pi"), ("µ", "micro mu"), ("°", "degree"), ("′", "prime minutes"), ("″", "double prime seconds"), ("‰", "per mille"),
        ("¼", "quarter"), ("½", "half"), ("¾", "three quarters"), ("⅓", "third"), ("⅔", "two thirds"), ("¹", "superscript one"),
        ("²", "squared superscript two"), ("³", "cubed superscript three"), ("₀", "subscript zero"), ("₁", "subscript one"), ("₂", "subscript two"),
        ("€", "euro"), ("£", "pound"), ("¥", "yen"), ("¢", "cent"), ("©", "copyright"), ("®", "registered"), ("™", "trademark"),
        ("§", "section"), ("¶", "paragraph"), ("†", "dagger"), ("‡", "double dagger"), ("№", "number"),
    ]),
    ("Letters", &[
        ("á", "a acute"), ("à", "a grave"), ("â", "a circumflex"), ("ä", "a umlaut diaeresis"), ("ã", "a tilde"), ("å", "a ring"), ("æ", "ae ligature"),
        ("ç", "c cedilla"), ("é", "e acute"), ("è", "e grave"), ("ê", "e circumflex"), ("ë", "e umlaut"), ("í", "i acute"), ("î", "i circumflex"),
        ("ï", "i umlaut"), ("ñ", "n tilde"), ("ó", "o acute"), ("ô", "o circumflex"), ("ö", "o umlaut"), ("õ", "o tilde"), ("ø", "o slash"),
        ("œ", "oe ligature"), ("ú", "u acute"), ("û", "u circumflex"), ("ü", "u umlaut"), ("ß", "sharp s eszett"), ("ł", "l stroke"),
        ("ą", "a ogonek"), ("ę", "e ogonek"), ("ć", "c acute"), ("ń", "n acute"), ("ś", "s acute"), ("ź", "z acute"), ("ż", "z dot"),
        ("š", "s caron"), ("č", "c caron"), ("ž", "z caron"), ("ř", "r caron"), ("ð", "eth"), ("þ", "thorn"),
        ("α", "alpha"), ("β", "beta"), ("γ", "gamma"), ("δ", "delta"), ("λ", "lambda"), ("Ω", "omega ohm"), ("Σ", "sigma"), ("Φ", "phi"),
        ("«", "left guillemet quote"), ("»", "right guillemet quote"), ("‘", "left single quote"), ("’", "right single quote apostrophe"),
        ("“", "left double quote"), ("”", "right double quote"), ("–", "en dash"), ("—", "em dash"), ("¡", "inverted exclamation"), ("¿", "inverted question"),
    ]),
];

/// The open picker.
pub(super) struct CharPicker {
    target: TokenTarget,
    group: usize,
    query: String,
}

/// The small button that opens the character picker for `target`.
pub(super) fn char_button<'a>(target: TokenTarget) -> Element<'a, Message> {
    tooltip(
        button(text("☺").size(14)).style(style::secondary).on_press(Message::OpenCharPicker(target)),
        container(text("Emoji and symbols").size(13)).padding(8).style(style::tooltip),
        tooltip::Position::Top,
    )
    .into()
}

/// The characters to show: every group's that match `query` when there is one, else `group`'s.
fn shown(group: usize, query: &str) -> Vec<(&'static str, &'static str)> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return GROUPS[group.min(GROUPS.len() - 1)].1.to_vec();
    }
    GROUPS.iter().flat_map(|(_, chars)| chars.iter().copied()).filter(|(c, words)| *c == query || words.contains(&query)).collect()
}

impl App {
    pub(super) fn update_chars(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenCharPicker(target) => self.char_picker = Some(CharPicker { target, group: 0, query: String::new() }),
            Message::CloseCharPicker => self.char_picker = None,
            Message::CharGroup(group) => {
                if let Some(p) = &mut self.char_picker {
                    p.group = group;
                    p.query.clear();
                }
            }
            Message::CharSearch(query) => {
                if let Some(p) = &mut self.char_picker {
                    p.query = query;
                }
            }
            Message::PickChar(c) => {
                if let Some(target) = self.char_picker.as_ref().map(|p| p.target.clone()) {
                    self.insert_text(&target, &c, false);
                }
            }
            other => return self.update_layers(other),
        }
        Task::none()
    }

    pub(super) fn view_char_picker<'a>(&'a self, picker: &'a CharPicker) -> Element<'a, Message> {
        let current = self.token_target_text(&picker.target);
        let mine: Element<'a, Message> = if current.is_empty() { text("(empty)").size(13).color(MUTED_COLOR).into() } else { text(current).size(15).into() };
        let tabs = iced::widget::row(GROUPS.iter().enumerate().map(|(i, (name, _))| {
            let on = picker.query.trim().is_empty() && i == picker.group;
            let b = button(text(*name).size(13)).on_press(Message::CharGroup(i));
            if on { b.into() } else { b.style(style::secondary).into() }
        }))
        .spacing(6)
        .wrap()
        .vertical_spacing(6);
        let found = shown(picker.group, &picker.query);
        let body: Element<'a, Message> = if found.is_empty() {
            text("Nothing by that name.").size(13).color(MUTED_COLOR).into()
        } else {
            let cells = found.into_iter().map(|(c, words)| {
                tooltip(
                    button(container(text(c).size(22)).center_x(Length::Fill)).style(style::secondary).padding([6, 0]).width(44).on_press(Message::PickChar(c.into())),
                    container(text(words).size(12)).padding(6).style(style::tooltip),
                    tooltip::Position::Top,
                )
                .into()
            });
            scrollable(iced::widget::row(cells).spacing(4).wrap().vertical_spacing(4)).height(Length::Fixed(300.0)).width(Length::Fill).into()
        };
        let dialog = container(
            column![
                text("Insert an emoji or symbol").size(20),
                row![text("Your text").size(13).color(MUTED_COLOR), mine].spacing(10).align_y(Alignment::Center),
                field("Search by name, e.g. heart, arrow, sword", &picker.query).on_input(Message::CharSearch).width(Length::Fill),
                tabs,
                body,
                row![
                    text("Click a character to add it to the end of your text. Emoji show in color if the system has an emoji font.").size(12).color(MUTED_COLOR).width(Length::Fill),
                    space::horizontal(),
                    button(text("Done")).on_press(Message::CloseCharPicker),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(12),
        )
        .padding(20)
        .width(Length::Fixed(640.0))
        .style(style::dialog);
        let backdrop = container(center(opaque(dialog)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style { background: Some(Color { a: 0.6, ..Color::BLACK }.into()), ..container::Style::default() });
        opaque(mouse_area(backdrop).on_press(Message::CloseCharPicker))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    #[test]
    fn every_entry_is_one_character_that_is_found_by_its_words() {
        for (_, chars) in GROUPS {
            for (c, words) in chars {
                assert_eq!(c.chars().filter(|ch| !matches!(ch, '\u{fe0f}')).count(), 1, "{c} is more than one code point");
                assert!(!words.is_empty());
                assert!(shown(0, words.split(' ').next().unwrap()).iter().any(|(found, _)| found == c));
            }
        }
        assert!(shown(0, "zzzz").is_empty());
    }

    #[test]
    fn characters_go_on_the_end_of_the_text_with_no_space() {
        let mut app = with_game();
        let _ = app.update(Message::NewInfo);
        let _ = app.update(Message::SetInfoTitle(0, "Ammo".into()));
        let _ = app.update(Message::OpenCharPicker(TokenTarget::InfoTitle(0)));
        let _ = app.update(Message::PickChar("🔥".into()));
        let _ = app.update(Message::PickChar("★".into()));
        assert_eq!(app.game().info[0].title.as_deref(), Some("Ammo🔥★"));
        let _ = app.update(Message::CharSearch("heart".into()));
        let _ = app.view();
        let _ = app.update(Message::CharGroup(3));
        let _ = app.view();
        let _ = app.update(Message::CloseCharPicker);
        assert!(app.char_picker.is_none());
    }

    #[test]
    fn menu_labels_take_characters_too() {
        let mut app = with_game();
        let _ = app.update(Message::NewMenu(crate::config::MenuKindTag::List));
        let _ = app.update(Message::OpenCharPicker(TokenTarget::MenuLabel(0, 0)));
        let _ = app.update(Message::PickChar("⚔".into()));
        assert!(app.game().menus[0].items[0].label.ends_with('⚔'));
    }
}
