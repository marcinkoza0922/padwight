//! Every token and formatting tag a text can use, grouped for the token picker: what to insert,
//! a sample to preview it with, and what it does.

use super::PadFamily;

/// One token (or tag) in the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// What is put into the text, braces included.
    pub insert: String,
    /// The text the preview shows; the token itself when it shows by itself.
    pub sample: String,
    pub about: &'static str,
}

/// A group of entries the picker folds away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fold {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    /// Whether the entries are drawn for each kind of controller, side by side.
    pub per_controller: bool,
    /// Whether the entries are icons, shown as a grid.
    pub icons: bool,
    pub entries: Vec<Entry>,
}

impl Fold {
    /// The controllers the entries are drawn for, when they are drawn for each.
    pub fn families(&self) -> &'static [PadFamily] {
        if self.per_controller { &PadFamily::ALL } else { &[] }
    }
}

fn entry(insert: &str, about: &'static str) -> Entry {
    Entry { insert: format!("{{{insert}}}"), sample: format!("{{{insert}}}"), about }
}

/// A tag, previewed on `sample` text.
fn tag(insert: &str, sample: &str, about: &'static str) -> Entry {
    Entry { insert: format!("{{{insert}}}"), sample: sample.to_string(), about }
}

/// The controller's buttons, sticks and D-pad.
fn buttons() -> Vec<Entry> {
    vec![
        entry("south", "Bottom face button (A / ✕ / B)"),
        entry("east", "Right face button (B / ○ / A)"),
        entry("west", "Left face button (X / □ / Y)"),
        entry("north", "Top face button (Y / △ / X)"),
        entry("lb", "Left Bumper"),
        entry("rb", "Right Bumper"),
        entry("lt", "Left Trigger"),
        entry("rt", "Right Trigger"),
        entry("select", "Select / View / Share / −"),
        entry("start", "Start / Menu / Options / +"),
        entry("guide", "Guide / PS / Home (or whatever opens the Guide overlay)"),
        entry("confirm", "The button that confirms in menus"),
        entry("back", "The button that goes back in menus"),
        entry("ls", "Left Stick"),
        entry("rs", "Right Stick"),
        entry("l3", "Left Stick Click (L3)"),
        entry("r3", "Right Stick Click (R3)"),
        entry("dpad", "D-pad"),
        entry("up", "D-pad Up"),
        entry("down", "D-pad Down"),
        entry("left", "D-pad Left"),
        entry("right", "D-pad Right"),
        entry("leftright", "D-pad Left and Right"),
        entry("updown", "D-pad Up and Down"),
    ]
}

/// Keys, mouse buttons and the virtual pad's buttons.
fn other_inputs() -> Vec<Entry> {
    vec![
        entry("pad:south", "A button as the controller the game sees (the virtual pad) labels it; any button token works after pad:"),
        entry("keyboard:a", "A keyboard key by name: keyboard:esc, keyboard:leftshift, keyboard:f1…"),
        entry("mouse:leftclick", "A mouse button: leftclick, rightclick, middleclick, backclick, forwardclick"),
    ]
}

/// Values that change as the overlay shows.
fn live_values() -> Vec<Entry> {
    vec![
        entry("time", "Time (24-hour)"),
        entry("time12", "Time (12-hour)"),
        entry("date", "Date"),
        entry("profile", "Active profile"),
        entry("layer", "Active layers"),
        entry("mode", "Active mode"),
        entry("app", "Focused program's executable"),
        entry("title", "Focused window's title"),
        entry("pid", "Focused program's process ID"),
        entry("cpu", "CPU usage"),
        entry("ram", "Memory in use"),
        entry("gpu", "GPU usage (AMD)"),
        entry("system_battery", "Laptop or handheld battery charge"),
        entry("system_battery:icon", "The same, as a gauge"),
        entry("controller_battery", "Charge of the controller in use (~ means estimated)"),
        entry("controller_battery:icon", "The same, as a gauge"),
        entry("wifi", "Wi-Fi signal: icon and strength"),
        entry("pc", "This machine: a monitor, or a laptop or handheld shape"),
        entry("controller", "Controller in use, drawn for its kind (Xbox, PlayStation, Nintendo)"),
        entry("controller:name", "Controller in use, by name"),
        entry("current_input", "Buttons just pressed (set what it follows under Input, below the cells)"),
    ]
}

/// The gyro and the touchpad.
fn motion() -> Vec<Entry> {
    vec![
        entry("gyro:p", "Gyro pitch: how fast the controller tips up and down, in degrees a second"),
        entry("gyro:r", "Gyro roll: how fast it tilts side to side"),
        entry("gyro:y", "Gyro yaw: how fast it turns left and right"),
        entry("trackpad", "Where the touchpad is touched (0 to 1 across and down), and “pressed” while it is clicked"),
    ]
}

/// The formatting tags, each with its closing tag.
fn formatting() -> Vec<Entry> {
    let mut format = vec![
        tag("b", "{b}Bold{/b}", "Bold text, up to {/b}"),
        tag("i", "{i}Italic{/i}", "Italic text, up to {/i}"),
        tag("u", "{u}Underlined{/u}", "Underlined text, up to {/u}"),
        tag("box", "{box}Boxed{/box}", "A light outline around what follows, up to {/box}"),
    ];
    format.extend(crate::info::COLOR_NAMES.iter().map(|(name, _)| tag(&format!("color:{name}"), &format!("{{color:{name}}}{name}{{/color}}"), "Colored text, up to {/color}; or any hex color, {color:#ff8800}")));
    format.extend(crate::font::names().iter().map(|name| tag(&format!("font:{name}"), &format!("{{font:{name}}}{name}{{/font}}"), "Text in this font; any installed family works too")));
    format.extend([
        tag("/font", "", "Ends the font"),
        tag("/b", "", "Ends bold"),
        tag("/i", "", "Ends italic"),
        tag("/u", "", "Ends underline"),
        tag("/box", "", "Ends the outline"),
        tag("/color", "", "Ends the color"),
    ]);
    format
}

/// The built-in icons, then the ones the game in use carries.
fn icons() -> Vec<Entry> {
    let mut icons: Vec<Entry> = crate::icons::built_in_names().iter().map(|n| entry(&format!("icon:{n}"), "Built in")).collect();
    icons.extend(crate::icons::pack_names().iter().map(|n| entry(&format!("icon:{n}"), "This game's own")));
    icons
}

/// Every fold, with the icons of the game in use among them.
pub fn folds() -> Vec<Fold> {
    let fold = |id, title, blurb, per_controller, entries| Fold { id, title, blurb, per_controller, icons: false, entries };
    vec![
        fold("buttons", "Controller buttons", "Drawn as the controller in use labels them.", true, buttons()),
        fold("inputs", "Keys, mouse and the game's pad", "Keycaps, mouse buttons and buttons as the virtual pad labels them.", false, other_inputs()),
        fold("live", "Live values", "Refreshed while the overlay shows.", false, live_values()),
        fold("motion", "Gyro and touchpad", "Follow the controller's movement as it happens.", false, motion()),
        fold("format", "Text formatting", "Put a tag before the text and its closing tag after it.", false, formatting()),
        Fold { id: "icons", title: "Icons", blurb: "Built in, and the ones this game's pack carries (those work only in this game).", per_controller: false, icons: true, entries: icons() },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info::{Segment, parse, preview};

    #[test]
    fn every_entry_is_a_token_the_text_understands() {
        for fold in folds() {
            for e in &fold.entries {
                let parsed = parse(&e.insert);
                assert!(parsed.iter().any(Result::is_err), "{} is not a token", e.insert);
            }
        }
    }

    #[test]
    fn a_glyph_entry_draws_differently_for_each_controller() {
        let south = |family| preview("{south}", family);
        assert_ne!(south(PadFamily::Xbox), south(PadFamily::PlayStation));
        assert_ne!(south(PadFamily::Xbox), south(PadFamily::Nintendo));
    }

    #[test]
    fn samples_show_something_where_there_is_one() {
        for fold in folds() {
            for e in fold.entries.iter().filter(|e| !e.sample.is_empty() && !e.insert.starts_with("{/")) {
                assert!(!preview(&e.sample, PadFamily::Xbox).is_empty(), "{} previews as nothing", e.sample);
            }
        }
        assert!(matches!(preview("{icon:bow}", PadFamily::Xbox).as_slice(), [Segment::Picture(_)]));
    }
}
