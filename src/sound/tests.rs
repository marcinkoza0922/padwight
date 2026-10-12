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
    assert_eq!(set.cue(SoundOverlay::Guide, Feedback::Mode).map(|s| s.kind), Some(SoundKind::Swish));
    assert_ne!(set.cue(SoundOverlay::Guide, Feedback::Mode), set.cue(SoundOverlay::Guide, Feedback::Step));
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

#[test]
fn echo_and_reverb_add_a_tail_that_ends_quietly_within_range() {
    let dry = SoundSpec::new(SoundKind::Click, 1.0, 1.0);
    for wet in [SoundSpec { echo: 1.0, ..dry }, SoundSpec { reverb: 1.0, ..dry }, SoundSpec { echo: 0.5, reverb: 0.5, bass: 12.0, ..dry }] {
        let samples = wet.samples();
        assert!(samples.len() > dry.samples().len() * 10, "{wet:?}");
        assert_eq!(samples.len(), (wet.duration().as_secs_f32() * RATE as f32).round() as usize, "{wet:?}");
        assert!(samples.iter().all(|s| s.abs() <= PEAK + 1e-4), "{wet:?}");
        assert!(samples.last().is_some_and(|s| s.abs() < 1e-3), "{wet:?}");
        // Something is heard after the click itself has died.
        assert!(samples[RATE as usize / 5..].iter().any(|s| s.abs() > 0.01), "{wet:?}");
    }
}

#[test]
fn bass_and_treble_move_the_weight_of_a_cue() {
    let zero_crossings = |v: &[f32]| v.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
    let crunch = SoundSpec::new(SoundKind::Crunch, 1.0, 1.0);
    let dark = zero_crossings(&SoundSpec { bass: 12.0, treble: -12.0, ..crunch }.samples());
    let bright = zero_crossings(&SoundSpec { bass: -12.0, treble: 12.0, ..crunch }.samples());
    assert!(bright > dark, "{dark} vs {bright}");
    assert_eq!(SoundSpec { bass: 0.0, ..crunch }.samples().len(), crunch.samples().len());
}

#[test]
fn drive_thickens_a_tone_without_raising_its_peak() {
    // A struck sine dies away; driven, its quiet end is lifted toward its peak.
    let beep = SoundSpec::new(SoundKind::Beep, 1.0, 1.0);
    let energy = |v: &[f32]| v.iter().map(|s| s * s).sum::<f32>();
    let clean = beep.samples();
    let driven = SoundSpec { drive: 1.0, ..beep }.samples();
    assert_eq!(driven.len(), clean.len());
    assert!(energy(&driven) > energy(&clean) * 1.5);
    assert!(driven.iter().all(|s| s.abs() <= PEAK + 1e-4));
}

#[test]
fn effects_load_and_save_only_when_set() {
    let set: SoundSet = toml::from_str("[menu]\npick = { kind = \"chime\", echo = 0.4, bass = -3 }").unwrap();
    assert_eq!((set.menu.pick.echo, set.menu.pick.bass, set.menu.pick.reverb), (0.4, -3.0, 0.0));
    let saved = toml::to_string(&SoundSet::default()).unwrap();
    assert!(!saved.contains("echo") && !saved.contains("reverb"), "{saved}");
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

#[test]
fn a_cue_can_play_a_pack_s_sound_file_and_falls_back_to_its_kind() {
    let wav = crate::config::encode_base64(&{
        let data: Vec<u8> = [0i16, 16000, -16000, 16000, 0].iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend([1, 0, 1, 0]);
        out.extend(RATE.to_le_bytes());
        out.extend((RATE * 2).to_le_bytes());
        out.extend([2, 0, 16, 0]);
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    });
    let spec = SoundSpec { file: Some(SoundFile::new("clap-test")), ..SoundSpec::new(SoundKind::Pop, 1.0, 1.0) };
    let synthesized = spec.samples().len();
    assert_eq!(synthesized, (SoundKind::Pop.seconds() * RATE as f32) as usize, "no pack in use: the kind plays");
    set_pack_sounds(&std::collections::BTreeMap::from([("clap-test".to_string(), wav)]));
    assert_eq!(spec.samples().len(), 5);
    let octave_up = SoundSpec { pitch: 2.0, ..spec };
    assert_eq!(octave_up.samples().len(), 2);
    let text = toml::to_string(&spec).unwrap();
    assert!(text.contains("file = \"clap-test\""));
    assert_eq!(toml::from_str::<SoundSpec>(&text).unwrap(), spec);
}
