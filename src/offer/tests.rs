use super::*;
use crate::{
    config::{Game, Profile, Rule, RuleKind},
    pack,
};

fn entry(name: &str, exe: &str) -> Entry {
    let mut game = Game::new(name, vec![Profile::passthrough("P")]);
    game.rules = vec![Rule::new(RuleKind::Executable, exe, "P")];
    let pack = pack::export(&game, &pack::draft(&game, false)).pack;
    Entry { file: "x.padpack", pack }
}

fn listings(library: &[Entry]) -> Vec<Listing> {
    library.iter().map(Listing::of).collect()
}

fn window(exe: &str) -> WindowInfo {
    WindowInfo { exe: exe.into(), class: "game".into(), ..Default::default() }
}

fn press(s: &mut OfferSession, b: Button) -> Option<OfferOutcome> {
    s.handle(InputEvent::Button(b, true), Instant::now())
}

/// The list on show, if the offer is at one, and its highlighted row.
fn choices(s: &OfferSession) -> (Vec<String>, Option<usize>) {
    match s.view(&OverlayStyle::default()) {
        crate::overlay::OverlayView::Menu(m) => (m.items.into_iter().map(|i| i.label).collect(), m.selected),
        _ => (Vec::new(), None),
    }
}

#[test]
fn packs_naming_the_folder_beat_packs_sharing_the_program_name() {
    let library = vec![entry("Other", "gpp.exe"), entry("Sands", "Prince of Persia - The Sands of Time/gpp.exe")];
    let config = Config::default();
    let pop = WindowInfo { path: "/games/Prince of Persia - The Sands of Time/gpp.exe".into(), ..window("gpp.exe") };
    assert_eq!(candidates(&config, &listings(&library), &pop, None), [1]);
    let other = WindowInfo { path: "/games/Something Else/gpp.exe".into(), ..window("gpp.exe") };
    assert_eq!(candidates(&config, &listings(&library), &other, None), [0]);
}

#[test]
fn only_unknown_library_games_are_offered() {
    let library = vec![entry("Doom", "doom.exe"), entry("Doom Remix", "DOOM.EXE"), entry("Quake", "quake.exe")];
    let mut config = Config::default();
    assert_eq!(candidates(&config, &listings(&library), &window("doom.exe"), None), [0, 1], "both packs fit");
    assert!(candidates(&config, &listings(&library), &window("other.exe"), None).is_empty());

    config.declined_packs.push(library[1].pack.pack.id.clone());
    assert_eq!(candidates(&config, &listings(&library), &window("doom.exe"), None), [0]);

    let plan = pack::plan(&config, library[0].pack.clone(), true);
    pack::apply(&mut config, &plan, &pack::Choices::default());
    assert!(candidates(&config, &listings(&library), &window("doom.exe"), None).is_empty(), "a setup with rules is the config's");
}

#[test]
fn answers_and_picking() {
    let library = vec![entry("Doom", "doom.exe"), entry("Doom Remix", "doom.exe")];
    let one = |i: usize| OfferSession::new(&library, &[i], None, Instant::now());
    assert_eq!(press(&mut one(0), Button::South), Some(OfferOutcome::Add(0, None)));
    assert_eq!(press(&mut one(0), Button::East), Some(OfferOutcome::No));
    assert_eq!(press(&mut one(0), Button::West), Some(OfferOutcome::Never));
    assert_eq!(press(&mut one(0), Button::North), None);

    let mut two = OfferSession::new(&library, &[0, 1], None, Instant::now());
    assert_eq!(press(&mut two, Button::South), None, "yes leads to the list");
    assert_eq!(choices(&two).0, ["Doom", "Doom Remix"]);
    press(&mut two, Button::DpadDown);
    press(&mut two, Button::DpadDown);
    press(&mut two, Button::DpadUp);
    assert_eq!(press(&mut two, Button::South), Some(OfferOutcome::Add(1, None)));

    // A pack with several profiles asks which one.
    let mut multi = entry("Doom", "doom.exe");
    multi.pack.profiles.push(Profile::passthrough("Alt"));
    let library = vec![multi];
    let mut s = OfferSession::new(&library, &[0], None, Instant::now());
    assert_eq!(press(&mut s, Button::South), None);
    assert_eq!(choices(&s).0, ["P", "Alt"]);
    press(&mut s, Button::DpadDown);
    assert_eq!(press(&mut s, Button::South), Some(OfferOutcome::Add(0, Some(1))));
    press(&mut s, Button::East);
    assert!(choices(&s).0.is_empty(), "B from the profile list goes back to the question");

}

#[test]
fn profiles_the_controller_cannot_play_are_not_offered() {
    // Profiles that need a gyro are neither listed nor offered without one.
    let mut gyro = entry("Doom", "doom.exe");
    gyro.pack.profiles[0].requires = vec![Feature::Gyro];
    gyro.pack.profiles.push(Profile::passthrough("Plain"));
    gyro.pack.profiles.push(Profile::passthrough("Plain 2"));
    let library = vec![gyro];
    let mut s = OfferSession::new(&library, &[0], Some(&[]), Instant::now());
    press(&mut s, Button::South);
    assert_eq!(choices(&s).0, ["Plain", "Plain 2"]);
    press(&mut s, Button::DpadDown);
    assert_eq!(press(&mut s, Button::South), Some(OfferOutcome::Add(0, Some(2))), "the position in the pack");
    let mut s = OfferSession::new(&library, &[0], Some(&[Feature::Gyro]), Instant::now());
    press(&mut s, Button::South);
    assert_eq!(choices(&s).0, ["P", "Plain", "Plain 2"]);

    let mut one = entry("Doom", "doom.exe");
    one.pack.profiles[0].requires = vec![Feature::Gyro];
    one.pack.profiles.push(Profile::passthrough("Plain"));
    let library = vec![one];
    let mut s = OfferSession::new(&library, &[0], Some(&[]), Instant::now());
    assert_eq!(press(&mut s, Button::South), Some(OfferOutcome::Add(0, Some(1))), "a lone fit is used without asking");
    let mut gyro_only = entry("Doom", "doom.exe");
    gyro_only.pack.profiles[0].requires = vec![Feature::Gyro];
    let gyro_only = [Listing::of(&gyro_only)];
    assert!(candidates(&Config::default(), &gyro_only, &window("doom.exe"), Some(&[])).is_empty());
    assert_eq!(candidates(&Config::default(), &gyro_only, &window("doom.exe"), Some(&[Feature::Gyro])), [0]);
    assert_eq!(candidates(&Config::default(), &gyro_only, &window("doom.exe"), None), [0]);
}

#[test]
fn the_stick_moves_through_the_list_and_holding_repeats() {
    let library = vec![entry("A", "a.exe"), entry("B", "a.exe"), entry("C", "a.exe")];
    let t0 = Instant::now();
    let mut s = OfferSession::new(&library, &[0, 1, 2], None, t0);
    press(&mut s, Button::South);
    s.handle(InputEvent::Axis(crate::input::Axis::LeftY, 0.9), t0);
    assert_eq!(choices(&s).1.unwrap(), 1);
    assert!(s.tick(t0 + crate::nav::REPEAT_DELAY));
    assert_eq!(choices(&s).1.unwrap(), 2);
    s.handle(InputEvent::Axis(crate::input::Axis::LeftY, 0.0), t0);
    assert!(!s.tick(t0 + crate::nav::REPEAT_DELAY * 4));
}

#[test]
fn b_goes_back_to_the_question() {
    let library = vec![entry("Doom", "doom.exe"), entry("Doom Remix", "doom.exe")];
    let mut back = OfferSession::new(&library, &[0, 1], None, Instant::now());
    press(&mut back, Button::South);
    assert_eq!(press(&mut back, Button::East), None, "B goes back to the question");
    assert!(choices(&back).0.is_empty());
}

#[test]
fn a_restless_stick_doesnt_keep_the_offer_up() {
    use crate::input::Axis;
    let library = vec![entry("Doom", "doom.exe")];
    let t0 = Instant::now();
    let mut s = OfferSession::new(&library, &[0], None, t0);
    for second in 0..40 {
        let wobble = if second % 2 == 0 { 0.02 } else { -0.03 };
        for axis in [Axis::LeftY, Axis::RightX, Axis::RightTrigger] {
            s.handle(InputEvent::Axis(axis, wobble), t0 + Duration::from_secs(second));
        }
    }
    assert!(s.expired(t0 + IDLE_CLOSE));
    s.handle(InputEvent::Axis(Axis::LeftY, 0.9), t0 + IDLE_CLOSE);
    assert!(!s.expired(t0 + IDLE_CLOSE + Duration::from_secs(1)), "a step of the stick counts as input");
}
