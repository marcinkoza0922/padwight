//! `expect overlay` and `debug overlay state`: what the overlay shows, asked as text. The first
//! test needs no daemon; the rest need one started with `--debug` (see the top of `main.rs`).

use super::Env;

#[test]
fn expect_overlay_fails_clearly_without_a_daemon() {
    let env = Env::new();
    let _session = env.session(&[]);
    let reply = env.debug(&["expect", "overlay", "showing", "guide"]);
    assert!(!reply.ok && reply.err.contains("no daemon"), "{}", reply.err);
    let reply = env.debug(&["expect", "overlay", "showing"]);
    assert!(!reply.ok && reply.err.contains("expect what"), "{}", reply.err);
}

#[cfg(target_os = "linux")]
mod with_daemon {
    use std::{process::Command, thread, time::Duration};

    use serde_json::Value;

    use super::Env;
    use crate::{
        common::{hold_config, serial},
        daemon::debug_daemon,
    };
    use super::super::BIN;

    /// The config with two menus, a list and a radial one, for `padwight menu <name>`.
    fn menus_config() -> String {
        let text = hold_config().replacen("menus = []", "", 1);
        format!(
            "{text}
[[general.menus]]
name = \"Belt\"
kind = \"list\"

[[general.menus.items]]
label = \"Potion\"
action = {{ keys = [\"KEY_1\"] }}

[[general.menus.items]]
label = \"Scroll\"
action = {{ keys = [\"KEY_2\"] }}

[[general.menus]]
name = \"Wheel\"

[general.menus.kind.radial]
stick = \"Right\"

[[general.menus.items]]
label = \"North\"
action = {{ keys = [\"KEY_3\"] }}

[[general.menus.items]]
label = \"South\"
action = {{ keys = [\"KEY_4\"] }}
"
        )
    }

    fn state(env: &Env) -> Value {
        let out = Command::new(BIN).args(["debug", "overlay", "state"]).env("XDG_RUNTIME_DIR", env.run()).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        serde_json::from_slice(&out.stdout).unwrap()
    }

    #[test]
    #[ignore]
    fn debug_the_guide_overlay_opens_and_its_selection_moves() {
        let _serial = serial();
        let env = Env::new();
        let _daemon = debug_daemon(&env, &hold_config());
        let _session = env.session(&["--attach"]);

        env.ok(&["expect", "overlay", "showing", "none"]);
        env.ok(&["tap", "guide"]);
        env.ok(&["expect", "overlay", "showing", "guide"]);
        env.ok(&["expect", "overlay", "guide.selected", "Quick Settings"]);
        env.ok(&["expect", "overlay", "menu.index", "0"]);
        env.ok(&["expect", "overlay", "guide.labels", "On-screen keyboard"]);

        env.ok(&["tap", "dpad-down"]);
        env.ok(&["tap", "dpad-down"]);
        env.ok(&["expect", "overlay", "guide.selected", "\"On-screen keyboard\""]);
        env.ok(&["expect", "overlay", "menu.index", "2"]);

        // A wrong expectation says what is there.
        let reply = env.debug(&["expect", "overlay", "guide.selected", "Quick Settings"]);
        assert!(!reply.ok && reply.err.contains("On-screen keyboard"), "{}", reply.err);
        let reply = env.debug(&["expect", "overlay", "guide.nope", "x"]);
        assert!(!reply.ok && reply.err.contains("it has"), "{}", reply.err);

        env.ok(&["tap", "east"]);
        env.ok(&["expect", "overlay", "showing", "none"]);
    }

    #[test]
    #[ignore]
    fn debug_the_keyboard_shows_its_key_and_latches() {
        let _serial = serial();
        let env = Env::new();
        let _daemon = debug_daemon(&env, &hold_config());
        let _session = env.session(&["--attach"]);

        env.ok(&["tap", "guide"]);
        env.ok(&["tap", "dpad-down"]);
        env.ok(&["tap", "dpad-down"]);
        env.ok(&["tap", "south"]);
        env.ok(&["expect", "overlay", "showing", "keyboard"]);
        env.ok(&["expect", "overlay", "keyboard.latched", "none"]);

        // Walk to Shift, then latch it for the next key.
        for _ in 0..20 {
            if state(&env)["keyboard"]["key"].as_str().is_some_and(|k| k.contains("SHIFT")) {
                break;
            }
            env.ok(&["tap", "dpad-left"]);
            thread::sleep(Duration::from_millis(30));
            if !state(&env)["keyboard"]["key"].as_str().is_some_and(|k| k.contains("SHIFT")) {
                env.ok(&["tap", "dpad-down"]);
            }
        }
        let key = state(&env)["keyboard"]["key"].as_str().unwrap().to_string();
        assert!(key.contains("SHIFT"), "the cursor never reached Shift, it is on {key}");
        env.ok(&["tap", "south"]);
        env.ok(&["expect", "overlay", "keyboard.latched", &key]);
    }

    #[test]
    #[ignore]
    fn debug_menus_of_different_styles_list_their_rows() {
        let _serial = serial();
        let env = Env::new();
        let _daemon = debug_daemon(&env, &menus_config());
        let _session = env.session(&["--attach"]);
        // The menu opens on the controller used last.
        env.ok(&["tap", "start"]);

        for (name, kind) in [("Belt", "list"), ("Wheel", "radial")] {
            let out = env.command(&["menu", name]).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
            env.ok(&["expect", "overlay", "showing", &name.to_lowercase()]);
            env.ok(&["expect", "overlay", "menu.kind", kind]);
            env.ok(&["expect", "overlay", "menu.title", name]);
            env.ok(&["expect", "overlay", "menu.items.0.label", if name == "Belt" { "Potion" } else { "North" }]);
            assert!(env.command(&["close-overlays"]).output().unwrap().status.success());
            env.ok(&["expect", "overlay", "menu", "none"]);
        }
    }
}
