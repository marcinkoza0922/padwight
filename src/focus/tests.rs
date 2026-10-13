use super::*;
use crate::config::Rule;

/// A config with one game per (game, rules) entry, each rule naming profile "P".
fn config(games: &[(&str, &[(RuleKind, &str)])]) -> Config {
    let mut config = Config::default();
    for (name, rules) in games {
        let mut game = crate::config::Game::new(name, vec![crate::config::Profile::passthrough("P")]);
        game.rules = rules.iter().map(|(kind, value)| Rule::new(*kind, *value, "P")).collect();
        config.games.push(game);
    }
    config
}

fn at(game: &str) -> Option<ProfileRef> {
    Some(ProfileRef::new(Some(game), "P"))
}

#[test]
fn first_matching_rule_wins() {
    let mut config = config(&[
        ("Souls", &[(RuleKind::SteamAppId, "1245620")]),
        ("Other", &[(RuleKind::Executable, "ELDENRING.EXE")]),
        ("Factorio", &[(RuleKind::WindowClass, "factorio")]),
    ]);
    let elden = WindowInfo {
        class: "steam_app_1245620".into(),
        exe: "eldenring.exe".into(),
        steam_app_id: Some("1245620".into()),
        ..Default::default()
    };
    assert_eq!(profile_for(&config, &elden), at("Souls"));
    let factorio = WindowInfo { class: "Factorio".into(), exe: "factorio".into(), ..Default::default() };
    assert_eq!(profile_for(&config, &factorio), at("Factorio"));

    let browser = WindowInfo { class: "firefox".into(), exe: "firefox".into(), ..Default::default() };
    assert_eq!(profile_for(&config, &browser), None, "the default is the daemon's call");
    assert!(has_rules(&config, "Souls") && !has_rules(&config, "General"));

    config.games[0].rules[0].enabled = false;
    assert_eq!(profile_for(&config, &elden), at("Other"), "switched-off rules are skipped");
}

#[test]
fn empty_rule_values_never_match() {
    let config = config(&[("X", &[(RuleKind::WindowClass, " ")])]);
    assert_eq!(profile_for(&config, &WindowInfo::default()), None);
}

#[test]
fn process_scan_uses_rule_order() {
    let config = config(&[("A", &[(RuleKind::Executable, "game.exe")]), ("B", &[(RuleKind::Executable, "bash")])]);
    let procs = |names: &[&str]| -> Vec<WindowInfo> {
        names.iter().map(|n| WindowInfo { exe: n.to_string(), ..Default::default() }).collect()
    };
    assert_eq!(profile_for_processes(&config, &procs(&["bash", "game.exe"])), at("A"));
    assert_eq!(profile_for_processes(&config, &procs(&["bash"])), at("B"));
    assert_eq!(profile_for_processes(&config, &procs(&["zsh"])), None);
}

#[test]
fn launches_come_from_rules_with_the_matching_process() {
    let mut config = config(&[("Doom", &[(RuleKind::Executable, "doom.exe")])]);
    config.auto_switch.default_profile = Some(ProfileRef::new(None, "Desktop"));
    let doom = WindowInfo { exe: "doom.exe".into(), pid: 42, ..Default::default() };
    assert_eq!(game_launch(&config, &doom), Some(("Doom".into(), 42)));
    let shell = WindowInfo { exe: "bash".into(), pid: 7, ..Default::default() };
    assert_eq!(game_launch(&config, &shell), None, "the default profile isn't a launch");
    assert_eq!(game_launch_in(&config, &[shell, doom]), Some(("Doom".into(), 42)));
}

#[test]
fn rules_naming_folders_match_the_end_of_the_path() {
    let pop = WindowInfo {
        exe: "gpp.exe".into(),
        path: "X:\\Games\\Heroic\\Prince of Persia - The Sands of Time\\gpp.exe".into(),
        ..Default::default()
    };
    assert!(rule_matches(RuleKind::Executable, "Prince of Persia - The Sands of Time/gpp.exe", &pop));
    assert!(rule_matches(RuleKind::Executable, "prince of persia - the sands of time\\GPP.EXE", &pop));
    assert!(rule_matches(RuleKind::Executable, "gpp.exe", &pop));
    assert!(!rule_matches(RuleKind::Executable, "Sands of Time/gpp.exe", &pop), "whole folder names only");
    assert!(!rule_matches(RuleKind::Executable, "Warrior Within/gpp.exe", &pop));
    let unknown = WindowInfo { path: String::new(), ..pop };
    assert!(!rule_matches(RuleKind::Executable, "Prince of Persia - The Sands of Time/gpp.exe", &unknown));
}

#[test]
fn a_rule_naming_the_folder_wins_over_an_earlier_name_only_rule() {
    let config = config(&[
        ("Any", &[(RuleKind::Executable, "gpp.exe")]),
        ("Sands", &[(RuleKind::Executable, "Sands of Time/gpp.exe")]),
    ]);
    let window = |path: &str| WindowInfo { exe: "gpp.exe".into(), path: path.into(), pid: 9, ..Default::default() };
    let sands = window("/g/Sands of Time/gpp.exe");
    assert_eq!(matching_rule(&config, &sands).map(|(g, _)| g), Some("Sands"));
    assert_eq!(game_launch(&config, &sands), Some(("Sands".into(), 9)));
    assert_eq!(matching_rule(&config, &window("/g/Other/gpp.exe")).map(|(g, _)| g), Some("Any"));
    let running = [window("/g/Other/gpp.exe"), sands];
    assert_eq!(matching_process(&config, &running).map(|(g, _, p)| (g, p.path.as_str())), Some(("Sands", "/g/Sands of Time/gpp.exe")));
}

#[test]
fn the_search_stops_at_a_match_nothing_can_beat() {
    let name = Rule::new(RuleKind::Executable, "gpp.exe", "P");
    let folder = Rule::new(RuleKind::Executable, "Sands/gpp.exe", "P");
    let rest = std::iter::from_fn(|| -> Option<(&str, &Rule, u8)> { panic!("searched past the best match") });
    let found = [("A", &name, 1), ("B", &folder, 2)].into_iter().chain(rest);
    assert_eq!(best(found).map(|(g, _, n)| (g, n)), Some(("B", 2)));
}

#[test]
fn rules_from_windows_add_the_folder_when_the_exe_name_is_taken() {
    let window = WindowInfo { exe: "gpp.exe".into(), path: "X:\\Games\\Sands of Time\\gpp.exe".into(), ..Default::default() };
    let rule = |config: &Config| rule_for_window(config, &window, "P".into()).value;
    assert_eq!(rule(&Config::default()), "gpp.exe");
    let taken = config(&[("Other", &[(RuleKind::Executable, "GPP.EXE")])]);
    assert_eq!(rule(&taken), "Sands of Time/gpp.exe");
    let bare = WindowInfo { path: "X:\\gpp.exe".into(), ..window.clone() };
    assert_eq!(rule_for_window(&taken, &bare, "P".into()).value, "gpp.exe", "a drive is no folder");
}
