#[test]
fn a_filling_toast_draws_its_bar_and_redraws_while_it_fills() {
    let t0 = Instant::now();
    let toast = Toast::filling(vec!["Keep holding".into()], t0, Duration::from_secs(2));
    let bar = |at| toast.view(at).unwrap().rows.last().unwrap()[0][0].clone();
    assert_eq!(bar(t0), Segment::Bar(0.0));
    assert_eq!(bar(t0 + Duration::from_secs(1)), Segment::Bar(0.5));
    assert!(toast.next_redraw(t0, Duration::from_millis(33)) <= t0 + BAR_FRAME);
}

#[test]
fn the_guide_layer_sheet_lists_what_it_binds() {
    let sheet = layer_sheet(&Layer::guide());
    let cells: Vec<&String> = sheet.rows.iter().flatten().collect();
    let pair = |token: &str, label: &str| {
        let at = cells.iter().position(|c| c.as_str() == token).unwrap_or_else(|| panic!("{token} missing"));
        assert_eq!(cells[at + 1], label, "{token}");
    };
    pair("{west}", "On-screen keyboard");
    pair("{north}", "On-screen numpad");
    pair("{rb}", "Screenshot");
    pair("{select}", "Force quit focused window (hold)");
    pair("{start}", "Quick Settings");
    assert!(sheet.rows.iter().all(|r| r.len() % 2 == 0 && r.len() <= 4));
    assert!(!cells.iter().any(|c| c.as_str() == "{south}"), "swallowed buttons aren't listed");
    // Every cell parses into glyphs and text.
    let live = Live::sample(PadFamily::Xbox);
    assert!(!resolve(&sheet, &live).rows.is_empty());
}

use super::*;

#[test]
fn current_input_tokens_take_a_controller_and_timings() {
    let tok = |cell: &str| parse(cell).into_iter().find_map(Result::err);
    let input = |device, gap_ms, stay_ms| Some(Token::CurrentInput { device, gap_ms, stay_ms });
    assert_eq!(tok("{current_input}"), input(None, None, None));
    assert_eq!(tok("{current_input:400}"), input(None, Some(400), None));
    assert_eq!(tok("{current_input_device_1:400:2000}"), input(Some(1), Some(400), Some(2000)));
    for literal in ["{current_input:x}", "{current_input_device_x}", "{current_input:1:2:3}", "{time:5}"] {
        assert_eq!(parse(literal), vec![Ok(literal.to_string())], "{literal}");
    }
}

#[test]
fn a_token_overrides_the_overlays_input_settings() {
    let mut o = overlay(&[&["{current_input}", "{current_input_device_2:400}"]]);
    o.current_input.tracking.device = Some(1);
    o.current_input.stay_ms = 3000;
    let cells = input_tokens(&o);
    assert_eq!((cells[0].tracking.device, cells[0].tracking.gap_ms, cells[0].stay_ms), (Some(1), 250, 3000));
    assert_eq!((cells[1].tracking.device, cells[1].tracking.gap_ms, cells[1].stay_ms), (Some(2), 400, 3000));
}

#[test]
fn current_input_shows_the_sample_motion() {
    let o = overlay(&[&["{current_input}"]]);
    let cell = &resolve(&o, &Live::sample(PadFamily::Xbox)).rows[0][0];
    let labels: Vec<&str> = cell
        .iter()
        .filter_map(|s| match s {
            Segment::Glyph { label, .. } => Some(label.as_str()),
            Segment::Dpad(_) => Some("dpad"),
            Segment::StickClick { .. } => Some("stick"),
            Segment::Icon(_) | Segment::Text(_) | Segment::Bar(_) | Segment::Picture(_) | Segment::Mark(_) => None,
        })
        .collect();
    assert_eq!(labels, ["dpad", "dpad", "dpad", "B"]);
    assert_eq!(cell[1], Segment::Dpad([false, true, false, true]), "a diagonal lights two arms");
}

fn overlay(rows: &[&[&str]]) -> InfoOverlay {
    InfoOverlay {
        name: "Keys".into(),
        always: true, on_start: None, linger: None, title: None, current_input: Default::default(),
        style: OverlayStyle::info(),
        rows: rows.iter().map(|r| r.iter().map(std::string::ToString::to_string).collect()).collect(),
    }
}

#[test]
fn tokens_become_glyphs_for_the_controller_in_use() {
    let o = overlay(&[&["{north} Reload", "{LT}+{south}"]]);
    let xbox = resolve(&o, &Live::sample(PadFamily::Xbox));
    assert_eq!(xbox.rows[0][0][0], Segment::Glyph { label: "Y".into(), fill: Some([0xc8, 0xa0, 0x1e]), round: true });
    assert_eq!(xbox.rows[0][0][1], Segment::Text(" Reload".into()));
    let ps = resolve(&o, &Live::sample(PadFamily::PlayStation));
    assert!(matches!(&ps.rows[0][0][0], Segment::Glyph { label, .. } if label == "△"));
    assert!(matches!(&ps.rows[0][1][0], Segment::Glyph { label, .. } if label == "L2"), "tokens are case-insensitive");
    let switch = resolve(&o, &Live::sample(PadFamily::Nintendo));
    assert!(matches!(&switch.rows[0][1][2], Segment::Glyph { label, .. } if label == "B"), "Nintendo's bottom button is B");
}

#[test]
fn pad_keyboard_and_mouse_tokens_draw_their_glyphs() {
    let o = overlay(&[&["{pad:south}", "{keyboard:leftshift}", "{mouse:leftclick}", "{keyboard:nope}"]]);
    let mut live = Live::sample(PadFamily::Xbox);
    live.pad_family = PadFamily::PlayStation;
    let view = resolve(&o, &live);
    let cells = &view.rows[0];
    assert_eq!(cells[0], vec![button_glyph(Button::South, PadFamily::PlayStation, false)], "the pad the daemon presents");
    assert_eq!(cells[1], vec![key_glyph("KEY_LEFTSHIFT")]);
    assert_eq!(cells[2], vec![mouse_glyph(MouseButton::Left)]);
    assert_eq!(cells[3], vec![Segment::Text("{keyboard:nope}".into())], "a key no layout draws stays text");
}

#[test]
fn plain_buttons_still_follow_the_controller_in_use() {
    let o = overlay(&[&["{south}"]]);
    let mut live = Live::sample(PadFamily::Xbox);
    live.pad_family = PadFamily::Nintendo;
    assert_eq!(resolve(&o, &live).rows[0][0], vec![button_glyph(Button::South, PadFamily::Xbox, false)]);
}

#[test]
fn button_names_follow_the_controller_in_use() {
    assert_eq!(button_name(Button::South, PadFamily::Xbox), "A (South)");
    assert_eq!(button_name(Button::South, PadFamily::PlayStation), "Cross (South)");
    assert_eq!(button_name(Button::Select, PadFamily::PlayStation), "Share");
    assert_eq!(button_name(Button::DpadUp, PadFamily::Nintendo), "D-pad Up");
}

#[test]
fn nintendo_layout_swaps_face_labels_only() {
    // On an Xbox pad the bottom button shows B, and the right one A; the bindings don't move.
    let o = overlay(&[&["{south} {east} {west} {north} {LB}"]]);
    let swapped = resolve(&o, &Live::sample(PadFamily::Xbox).with_layout(true));
    let labels: Vec<&str> = swapped.rows[0][0]
        .iter()
        .filter_map(|s| match s {
            Segment::Glyph { label, .. } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["B", "A", "Y", "X", "LB"]);
    let plain = resolve(&o, &Live::sample(PadFamily::Xbox));
    assert!(matches!(&plain.rows[0][0][0], Segment::Glyph { label, .. } if label == "A"), "off by default");
    assert_eq!(nintendo_face(Button::South), Button::East);
    assert_eq!(nintendo_face(Button::LeftBumper), Button::LeftBumper);
}

#[test]
fn live_values_and_unknown_braces() {
    let o = overlay(&[&["CPU {cpu} · {app}", "{nope} {", "pid {pid}"]]);
    assert!(is_live(&o));
    let v = resolve(&o, &Live::sample(PadFamily::Xbox));
    assert_eq!(v.rows[0][0], vec![Segment::Text("CPU 23% · game.exe".into())]);
    assert_eq!(v.rows[0][1], vec![Segment::Text("{nope} {".into())]);
    assert_eq!(v.rows[0][2], vec![Segment::Text("pid 4242".into())]);
    assert!(!is_live(&overlay(&[&["{south} Jump"]])));
    assert_eq!(local_time("%Y").len(), 4);
    let batteries = overlay(&[&["{system_battery} {controller_battery}"]]);
    assert_eq!(
        resolve(&batteries, &Live::sample(PadFamily::Xbox)).rows[0][0],
        vec![Segment::Text("78% ~64%".into())]
    );
    let mut live = Live::sample(PadFamily::Xbox);
    live.system.battery = None;
    live.controller_battery = None;
    assert_eq!(resolve(&batteries, &live).rows[0][0], vec![Segment::Text("n/a n/a".into())]);

    let rec = recording_view();
    assert_eq!(rec.style.position, ScreenPosition::TopLeft);
    assert_eq!(rec.rows[0][0], vec![glyph("● REC", Some([0xc8, 0x3c, 0x3c]), false)]);

    let layer = overlay(&[&["Layer: {layer}"]]);
    let mut live = Live::sample(PadFamily::Xbox);
    live.layers = vec!["Hotkeys".into(), "Build".into()];
    assert_eq!(resolve(&layer, &live).rows[0][0], vec![Segment::Text("Layer: Hotkeys + Build".into())]);
    live.layers.clear();
    assert_eq!(resolve(&layer, &live).rows[0][0], vec![Segment::Text("Layer: —".into())]);
}

#[test]
fn overlays_fade_out_at_the_end_of_their_time() {
    let now = std::time::Instant::now();
    let s = std::time::Duration::from_secs_f32;
    assert_eq!(fade(now, now + s(5.0)), Some(1.0));
    let half = fade(now, now + FADE_OUT / 2).unwrap();
    assert!((half - 0.5).abs() < 0.01, "{half}");
    assert_eq!(fade(now, now), None);
    assert_eq!(fade(now + s(1.0), now), None);
}

#[test]
fn toasts_show_their_lines_then_fade_away() {
    let now = Instant::now();
    let toast = Toast::new(vec!["Doom".into(), "Play".into()], now);
    let view = toast.view(now).unwrap();
    assert_eq!(view.rows, [[[Segment::Text("Doom".into())]], [[Segment::Text("Play".into())]]]);
    assert_eq!(view.opacity, 1.0);
    assert_eq!(toast.next_redraw(now, Duration::from_millis(33)), now + TOAST_TIME - FADE_OUT);
    assert!(toast.view(now + TOAST_TIME - FADE_OUT / 2).unwrap().opacity < 1.0);
    assert_eq!(toast.view(now + TOAST_TIME), None);
}

#[test]
fn timed_overlays_show_at_start_and_linger_after_release() {
    let now = Instant::now();
    let s = Duration::from_secs_f32;
    let mut start = overlay(&[&["Loaded"]]);
    start.name = "Loaded".into();
    start.on_start = Some(3.0);
    let mut sheet = overlay(&[&["{south} Jump"]]);
    sheet.name = "Sheet".into();
    sheet.linger = Some(2.0);
    let overlays = [start, sheet];
    let mut t = Timers::default();

    t.game_started(&overlays, now);
    assert!(t.timed("Loaded") && !t.timed("Sheet"));
    assert_eq!(t.opacity("Loaded", now + s(1.0)), 1.0);
    assert!(t.next_redraw(now, s(0.03)).is_some_and(|w| w > now + s(2.0)), "wakes as it starts to fade");

    // Held, then let go: it lingers, fading at the end, and holding it again ends that.
    t.update(HashSet::from(["Sheet".to_string()]), &overlays, now);
    assert!(t.held("Sheet") && !t.timed("Sheet"));
    t.update(HashSet::new(), &overlays, now + s(1.0));
    assert!(t.timed("Sheet") && !t.held("Sheet"));
    assert!(t.opacity("Sheet", now + s(3.0) - FADE_OUT / 2) < 0.6);
    t.update(HashSet::from(["Sheet".to_string()]), &overlays, now + s(1.5));
    assert!(!t.timed("Sheet"));

    // Time's up for the start one.
    t.update(HashSet::new(), &overlays, now + s(4.0));
    assert!(!t.timed("Loaded"));
    assert_eq!(t.opacity("Loaded", now + s(4.0)), 0.0);
    t.clear();
    assert!(!t.timed("Sheet"));
}

#[test]
fn controller_battery_reads_the_hid_devices_power_supply() {
    let hid = std::env::temp_dir().join(format!("controller-battery-test-{}", std::process::id()));
    let supply = hid.join("power_supply/ps-controller-battery-1");
    fs::create_dir_all(&supply).unwrap();
    fs::write(supply.join("type"), "Battery\n").unwrap();
    fs::write(supply.join("capacity"), "64\n").unwrap();
    assert_eq!(controller_battery(&hid), Some(Charge { percent: 64, estimated: false }));
    assert_eq!(controller_battery(&hid.join("missing")), None);
    // A Switch controller reports only a level, which becomes an estimate.
    fs::remove_file(supply.join("capacity")).unwrap();
    fs::write(supply.join("capacity_level"), "High\n").unwrap();
    assert_eq!(controller_battery(&hid), Some(Charge { percent: 75, estimated: true }));
    fs::remove_dir_all(&hid).unwrap();
}

#[test]
fn wifi_strength_is_link_quality_out_of_70() {
    let text = "Inter-| sta-|   Quality        |\n face | tus | link level noise |\nwlp2s0: 0000   43.  -67.  -256 0 0\n";
    assert_eq!(parse_wireless(text), Some(61));
    assert_eq!(parse_wireless("Inter-| sta-|\n face | tus |\n"), None, "no wireless interface");
}

#[test]
fn form_factor_from_dmi() {
    assert_eq!(detect_form("Jupiter", "Valve", "3"), FormFactor::Handheld);
    assert_eq!(detect_form("RC71L", "ASUSTeK COMPUTER INC.", "10"), FormFactor::Handheld);
    assert_eq!(detect_form("Laptop 15", "Acme", "10"), FormFactor::Laptop);
    assert_eq!(detect_form("Convertible", "Acme", "31"), FormFactor::Laptop);
    assert_eq!(detect_form("MS-7D25", "Micro-Star", "3"), FormFactor::Desktop);
}

#[test]
fn controller_and_pc_tokens_and_gauges_parse() {
    assert_eq!(token("pc"), Some(Token::Icon(IconToken::Pc)));
    assert_eq!(token("controller"), Some(Token::Icon(IconToken::Controller)));
    assert_eq!(token("controller:name"), Some(Token::Stat(Stat::Controller)));
    assert_eq!(token("wifi"), Some(Token::Wifi));
    assert_eq!(token("wifi:icon"), None, "Wi-Fi always has its icon");
    assert_eq!(token("controller_battery"), Some(Token::Gauge { gauge: Gauge::ControllerBattery, icon: false }));
    assert_eq!(token("system_battery:bars"), None, "an unknown argument leaves the token as text");
}

#[test]
fn families_from_vendor_and_name() {
    assert_eq!(PadFamily::detect(0x054c, "Wireless Controller"), Some(PadFamily::PlayStation));
    assert_eq!(PadFamily::detect(0x057e, "Pro Controller"), Some(PadFamily::Nintendo));
    assert_eq!(PadFamily::detect(0x045e, "Xbox Wireless Controller"), Some(PadFamily::Xbox));
    assert_eq!(PadFamily::detect(0x0000, "Nintendo Switch Pro Controller"), Some(PadFamily::Nintendo));
    assert_eq!(PadFamily::detect(0x0000, "Generic X-Box pad"), Some(PadFamily::Xbox));
    assert_eq!(PadFamily::detect(0x28de, "Steam Deck"), Some(PadFamily::Xbox));
    assert_eq!(PadFamily::detect(0x2dc8, "8BitDo Ultimate 2C"), None, "unknown pads use the fallback setting");
}

#[test]
fn system_sampling_reads_proc() {
    let mut s = Sampler::default();
    s.sample();
    s.sample();
    assert!(s.stats.ram.is_some_and(|(used, total)| used > 0.0 && total >= used));
    assert!(s.stats.cpu.is_none_or(|c| (0.0..=100.0).contains(&c)));
}

#[test]
fn guide_says_whatever_opens_the_overlay() {
    use crate::config::{GestureKind, Shortcut, ShortcutInput, Shortcuts};
    let say = |opener: Option<ShortcutInput>| {
        let mut s = Shortcuts::default();
        s.set(Shortcut::OpenGuide, opener);
        set_opener(OpenerLabel::of(&s));
        parse("press {guide} to close")
    };
    let part = |b| Err(Token::Button(b));
    assert_eq!(say(Some(ShortcutInput::button(Button::Guide))), vec![Ok("press ".into()), part(Button::Guide), Ok(" to close".into())]);
    assert_eq!(say(Some(ShortcutInput::button(Button::Select)))[1], part(Button::Select));
    let combo = say(Some(ShortcutInput::combo(&[Button::Start, Button::Select])));
    assert_eq!(combo[1..4], [part(Button::Select), Ok(" + ".into()), part(Button::Start)]);
    let gesture = say(Some(ShortcutInput::gesture(Button::Select, GestureKind::LongPress)));
    assert_eq!(gesture[1..3], [Ok("long press ".into()), part(Button::Select)]);
    assert!(matches!(&say(None)[1], Ok(t) if t.contains("no Guide")));
    // {pad:guide} is the pad's own button, whatever opens the overlay.
    assert_eq!(parse("{pad:guide}"), vec![Err(Token::PadButton(Button::Guide))]);
}

#[test]
fn confirm_and_back_say_the_menu_buttons() {
    assert_eq!(parse("{confirm}{back}"), vec![Err(Token::Button(Button::South)), Err(Token::Button(Button::East))]);
    set_menu_keys(crate::config::MenuKeys { confirm: Button::Select, back: Button::LeftStick });
    assert_eq!(parse("{confirm} {back}"), vec![Err(Token::Button(Button::Select)), Ok(" ".into()), Err(Token::Button(Button::LeftStick))]);
    assert_eq!(parse("{south}"), vec![Err(Token::Button(Button::South))], "{{south}} stays the button");
}

#[test]
fn the_panel_keys_say_the_buttons_they_sit_on() {
    use crate::config::{Shortcut, ShortcutInput, Shortcuts};
    assert_eq!(parse("{play}{previous}{enter}"), vec![Err(Token::Button(Button::South)), Err(Token::Button(Button::LeftBumper)), Err(Token::Button(Button::Start))]);
    let mut shortcuts = Shortcuts::default();
    shortcuts.set(Shortcut::MediaPlay, Some(ShortcutInput::button(Button::Select)));
    set_panel_keys(&shortcuts);
    assert_eq!(parse("{play}"), vec![Err(Token::Button(Button::Select))]);
    assert_eq!(parse("{next}"), vec![Err(Token::Button(Button::RightBumper))], "the key it traded with stays where it was");
    set_panel_keys(&Shortcuts::default());
}

#[test]
fn formatting_tags_become_marks_and_unknown_ones_stay_text() {
    assert_eq!(
        parse("{b}Hit{/b} {color:red}x{/color} {color:#102030}y{/color}"),
        vec![
            Err(Token::Mark(Mark::Bold(true))),
            Ok("Hit".into()),
            Err(Token::Mark(Mark::Bold(false))),
            Ok(" ".into()),
            Err(Token::Mark(Mark::Color(Some([0xe8, 0x5a, 0x5a])))),
            Ok("x".into()),
            Err(Token::Mark(Mark::Color(None))),
            Ok(" ".into()),
            Err(Token::Mark(Mark::Color(Some([0x10, 0x20, 0x30])))),
            Ok("y".into()),
            Err(Token::Mark(Mark::Color(None))),
        ]
    );
    assert_eq!(parse("{u}{box}{i}"), vec![Err(Token::Mark(Mark::Underline(true))), Err(Token::Mark(Mark::Boxed(true))), Err(Token::Mark(Mark::Italic(true)))]);
    for text in ["{color}", "{color:nope}", "{/color:red}", "{b:1}", "{bold text}"] {
        assert_eq!(parse(text), vec![Ok(text.to_string())], "{text} is not a tag");
    }
}

#[test]
fn marks_and_icons_reach_the_segments_of_a_cell_and_of_a_hint() {
    let o = overlay(&[&["{b}Big{/b} {icon:bow}"]]);
    let cell = &resolve(&o, &Live::sample(PadFamily::Xbox)).rows[0][0];
    assert_eq!(cell[0], Segment::Mark(Mark::Bold(true)));
    assert_eq!(cell[1], Segment::Text("Big".into()));
    assert_eq!(cell[2], Segment::Mark(Mark::Bold(false)));
    assert_eq!(cell.last(), Some(&Segment::Picture("bow".into())));
    let hint = glyph_text("{u}{icon:key}{/u}", Glyphs::default());
    assert_eq!(hint, vec![Segment::Mark(Mark::Underline(true)), Segment::Picture("key".into()), Segment::Mark(Mark::Underline(false))]);
}

#[test]
fn an_icon_token_needs_a_plain_name() {
    assert_eq!(parse("{icon:bow}"), vec![Err(Token::Picture("bow".into()))]);
    assert_eq!(parse("{icon:../x}"), vec![Ok("{icon:../x}".into())]);
    assert_eq!(parse("{icon}"), vec![Ok("{icon}".into())]);
}

#[test]
fn gyro_and_trackpad_tokens_show_the_controller_s_movement() {
    let o = overlay(&[&["{gyro:p} {gyro:y} {gyro:r}", "{trackpad}"]]);
    let live = Live::sample(PadFamily::Xbox);
    let view = resolve(&o, &live);
    assert_eq!(view.rows[0][0], [Segment::Text("+12°/s -35°/s +3°/s".into())]);
    assert_eq!(view.rows[0][1], [Segment::Text("(0.42, 0.58) pressed".into())]);
    let still = Live { gyro: Some([0.2, -0.3, 0.0]), touchpad: Some(Touchpad::default()), ..live.clone() };
    let view = resolve(&o, &still);
    assert_eq!(view.rows[0][0], [Segment::Text("0°/s 0°/s 0°/s".into())]);
    assert_eq!(view.rows[0][1], [Segment::Text("(–, –)".into())]);
    let none = Live { gyro: None, touchpad: None, ..live };
    let view = resolve(&o, &none);
    assert_eq!(view.rows[0][0], [Segment::Text("n/a n/a n/a".into())]);
    assert_eq!(view.rows[0][1], [Segment::Text("n/a".into())]);
    assert!(is_live(&o) && is_fast(&o));
    assert!(!is_fast(&overlay(&[&["{cpu}"]])));
}

#[test]
fn the_font_tag_names_a_valid_font_and_closes() {
    assert_eq!(
        parse("{font:Press Start 2P}Go{/font}"),
        vec![Err(Token::Mark(Mark::Font(Some("Press Start 2P".into())))), Ok("Go".into()), Err(Token::Mark(Mark::Font(None)))]
    );
    for text in ["{font}", "{font:a/b}", "{/font:x}"] {
        assert_eq!(parse(text), vec![Ok(text.to_string())], "{text} is not a tag");
    }
}
