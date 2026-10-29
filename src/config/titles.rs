//! Titles: games that can be launched from the Games page, imported from Steam, Heroic and
//! Lutris. A title is not a setup (`Game`): it says what to start, and which setup plays it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{is_true, yes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Store {
    Steam,
    Heroic,
    Lutris,
    /// Not imported: a game `padwight run` started that no title names.
    Run,
}

impl Store {
    pub fn label(self) -> &'static str {
        match self {
            Store::Steam => "Steam",
            Store::Heroic => "Heroic",
            Store::Lutris => "Lutris",
            Store::Run => "Padwight",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Title {
    /// The store and its id, e.g. `steam-1245620`; what an import finds a title again by.
    pub id: String,
    pub name: String,
    pub source: Store,
    /// The id the store's link names.
    pub store_id: String,
    /// Heroic's runner: `legendary`, `gog`, `nile` or `sideload`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub runner: String,
    /// The setup that plays it; empty to match through the setups' rules, else General.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub setup: String,
    /// Imported but not wanted in the list.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// False once the store no longer lists it as installed.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub installed: bool,
    /// Variables set on the game when it is started through `padwight run`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Through `padwight run`, hides the controllers Padwight has taken over from SDL.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_controllers: bool,
}

impl Title {
    /// The store's own link that starts the game.
    pub fn link(&self) -> String {
        match self.source {
            Store::Steam => format!("steam://rungameid/{}", self.store_id),
            Store::Heroic => format!("heroic://launch?appName={}&runner={}", encode(&self.store_id), encode(&self.runner)),
            Store::Lutris => format!("lutris:rungameid/{}", self.store_id),
            Store::Run => String::new(),
        }
    }
}

/// Percent-encodes everything but unreserved characters.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { char::from(b).to_string() } else { format!("%{b:02X}") })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title(source: Store, id: &str, runner: &str) -> Title {
        Title { id: String::new(), name: String::new(), source, store_id: id.into(), runner: runner.into(), setup: String::new(), hidden: false, installed: true, env: BTreeMap::new(), hide_controllers: false }
    }

    #[test]
    fn links_name_the_store_and_game() {
        assert_eq!(title(Store::Steam, "1245620", "").link(), "steam://rungameid/1245620");
        assert_eq!(title(Store::Lutris, "7", "").link(), "lutris:rungameid/7");
        assert_eq!(title(Store::Heroic, "a b", "sideload").link(), "heroic://launch?appName=a%20b&runner=sideload");
    }

    #[test]
    fn launch_settings_are_saved_with_the_title() {
        let env = BTreeMap::from([("PROTON_LOG".to_string(), "1".to_string())]);
        let titles = vec![Title { env, hide_controllers: true, ..title(Store::Steam, "1", "") }];
        let config = super::super::Config { titles, ..super::super::Config::default() };
        let back: super::super::Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(back.titles, config.titles);
        assert!(back.titles[0].hide_controllers && back.titles[0].env["PROTON_LOG"] == "1");
    }

    #[test]
    fn titles_are_saved_in_the_config() {
        let titles = vec![Title { id: "heroic-gog-1".into(), name: "Factorio".into(), ..title(Store::Heroic, "1", "gog") }];
        let config = super::super::Config { titles, ..super::super::Config::default() };
        let text = toml::to_string(&config).unwrap();
        assert!(text.contains("[[titles]]") && text.contains("source = \"heroic\""), "{text}");
        assert!(!text.contains("hidden") && !text.contains("installed") && !text.contains("env") && !text.contains("hide_controllers"), "defaults aren't written: {text}");
        let back: super::super::Config = toml::from_str(&text).unwrap();
        assert_eq!(back.titles, config.titles);
    }
}
