use super::*;

#[test]
fn every_kind_has_a_cue_of_its_length_within_range() {
    for kind in SoundKind::ALL {
        let samples = SoundSpec::new(kind, 1.0, 1.0).samples();
        assert_eq!(samples.len(), (kind.seconds() * RATE as f32) as usize, "{kind:?}");
        assert!(samples.iter().all(|s| s.abs() <= 1.0), "{kind:?}");
        assert!(samples.iter().any(|s| s.abs() > 0.5), "{kind:?} is silent");
    }
}

#[test]
fn length_stretches_the_cue() {
    let normal = SoundSpec::new(SoundKind::Knock, 1.0, 1.0).samples().len();
    let long = SoundSpec { length: 2.0, ..SoundSpec::new(SoundKind::Knock, 1.0, 1.0) }.samples().len();
    let short = SoundSpec { length: 0.5, ..SoundSpec::new(SoundKind::Knock, 1.0, 1.0) }.samples().len();
    assert!((long as f32 / normal as f32 - 2.0).abs() < 0.01);
    assert!((short as f32 / normal as f32 - 0.5).abs() < 0.01);
}

#[test]
fn volume_scales_the_peak_and_zero_is_silent() {
    let loud = SoundSpec::new(SoundKind::Chime, 1.0, 1.0).samples();
    let quiet = SoundSpec::new(SoundKind::Chime, 0.5, 1.0).samples();
    let peak = |v: &[f32]| v.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
    assert!((peak(&loud) - PEAK).abs() < 1e-3);
    assert!((peak(&quiet) - PEAK / 2.0).abs() < 1e-3);
    assert!(SoundSpec::new(SoundKind::Pop, 0.0, 1.0).samples().iter().all(|s| *s == 0.0));
}

#[test]
fn higher_pitch_means_more_cycles_in_a_click() {
    let zero_crossings = |v: &[f32]| v.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    let low = zero_crossings(&SoundSpec::new(SoundKind::Chime, 1.0, 0.5).samples());
    let high = zero_crossings(&SoundSpec::new(SoundKind::Chime, 1.0, 2.0).samples());
    assert!(high > low * 2, "{low} vs {high}");
}

#[test]
fn cues_end_quietly() {
    for kind in SoundKind::ALL {
        let samples = SoundSpec::new(kind, 1.0, 1.0).samples();
        assert!(samples.last().is_some_and(|s| s.abs() < 1e-3), "{kind:?}");
    }
}

#[test]
fn overlay_sounds_default_on_and_load_from_nothing() {
    let set: SoundSet = toml::from_str("").unwrap();
    assert_eq!(set, SoundSet::default());
    for overlay in SoundOverlay::ALL {
        let sounds = set.get(overlay);
        assert!(sounds.enabled, "{overlay:?}");
        assert!(sounds.step.volume < sounds.pick.volume, "steps are fainter than picks");
        assert!(sounds.open.volume < sounds.step.volume, "popping in is fainter than a step");
        assert!(sounds.close.volume < sounds.step.volume, "popping out is fainter than a step");
    }
}

#[test]
fn a_table_sets_one_overlay_and_leaves_the_rest() {
    let set: SoundSet = toml::from_str("[menu]\nenabled = false\nstep = { kind = \"knock\", length = 1.5 }").unwrap();
    assert!(!set.menu.enabled);
    assert_eq!(set.menu.step.kind, SoundKind::Knock);
    assert_eq!(set.menu.step.length, 1.5);
    assert_eq!(set.menu.pick, OverlaySounds::default().pick);
    assert_eq!(set.keyboard, OverlaySounds::default());
}

#[test]
fn a_silenced_overlay_plays_nothing_and_others_do() {
    let mut set = SoundSet::default();
    set.menu.enabled = false;
    assert_eq!(set.cue(SoundOverlay::Menu, Feedback::Step), None);
    assert_eq!(set.cue(SoundOverlay::Menu, Feedback::Pick), None);
    assert_eq!(set.cue(SoundOverlay::Menu, Feedback::Open), None);
    assert_eq!(set.cue(SoundOverlay::Menu, Feedback::Close), None);
    assert_eq!(set.cue(SoundOverlay::Keyboard, Feedback::Pick), Some(set.keyboard.pick));
    assert_eq!(set.cue(SoundOverlay::Keyboard, Feedback::Open), Some(set.keyboard.open));
    assert!(!SoundSet::default().silenced().keyboard.enabled);
}

#[test]
fn feedback_prefers_a_pick() {
    assert_eq!(Feedback::of(true, true), Some(Feedback::Pick));
    assert_eq!(Feedback::of(false, true), Some(Feedback::Step));
    assert_eq!(Feedback::of(false, false), None);
}

#[test]
fn the_guide_menus_have_their_own_calm_sounds_even_in_an_older_set() {
    let set: SoundSet = toml::from_str("[menu]\nenabled = false").unwrap();
    assert_eq!(set.guide, OverlaySounds::guide());
    assert_ne!(set.guide, OverlaySounds::default());
    assert_eq!(set.cue(SoundOverlay::Guide, Feedback::Open).map(|s| s.kind), Some(SoundKind::Rise));
    assert_eq!(set.cue(SoundOverlay::Guide, Feedback::Close).map(|s| s.kind), Some(SoundKind::Fall));
}

#[test]
fn a_rise_ends_higher_than_it_starts_and_a_fall_lower() {
    let zero_crossings = |v: &[f32]| v.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    let ends = |kind| {
        let samples = SoundSpec::new(kind, 1.0, 1.0).samples();
        let quarter = samples.len() / 4;
        (zero_crossings(&samples[..quarter]), zero_crossings(&samples[samples.len() - quarter..]))
    };
    let (start, end) = ends(SoundKind::Rise);
    assert!(end > start, "rise: {start} then {end}");
    let (start, end) = ends(SoundKind::Fall);
    assert!(end < start, "fall: {start} then {end}");
}

/// Writes every kind to `sound-previews/<kind>.wav`, to listen to while tuning them.
#[test]
#[ignore]
fn render_previews() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sound-previews");
    std::fs::create_dir_all(&dir).unwrap();
    for kind in SoundKind::ALL {
        let samples = SoundSpec::new(kind, 0.8, 1.0).samples();
        let data: Vec<u8> = samples.iter().flat_map(|s| ((s * i16::MAX as f32) as i16).to_le_bytes()).collect();
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        for field in [16_u32.to_le_bytes(), [1, 0, 1, 0], RATE.to_le_bytes(), (RATE * 2).to_le_bytes(), [2, 0, 16, 0]] {
            wav.extend_from_slice(&field);
        }
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        std::fs::write(dir.join(format!("{}.wav", kind.label().to_lowercase())), wav).unwrap();
    }
}
