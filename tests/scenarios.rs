//! Scenario tests: each `tests/scenarios/*.scenario` runs a script against a real daemon on a
//! clock it is handed, and compares the daemon's transcript with the one saved in the file.
//!
//! A scenario file has a script and, after a line of `---`, the expected transcript:
//!
//! ```text
//! # Comments and blank lines are skipped.
//! setup packs/deus-ex.padpack Gameplay    # a library pack to import, and the profile to start in
//! model dualsense                         # the controller (default: generic)
//! press lb                                # every other line is a `padwight debug` word
//! tap south
//! wait 300
//! release lb
//! ---
//! key KEY_A down @0
//! ```
//!
//! A `cli <args>` line in the script runs `padwight <args>` against the daemon.
//!
//! Header lines (before the first debug word) pick the configuration: `setup <pack> [profile]`
//! imports a pack from the repository, `config hold` is the default config with South as KEY_A and
//! East as the left mouse button, `config <file>` installs a config in the form
//! `tests/common` writes, and without either the default config is used. `model`, `full-keyboard` (the
//! on-screen keyboard with every key), `steam running` and `steam not-running` (the daemon runs
//! alone in a pid namespace, with a process named steam or without) and `append <file>` (a TOML fragment added to the default
//! config, such as a `[shortcuts]` table) are header lines too. The daemon runs `--debug --fake-clock`, so `wait`, `tap` and `advance` move its
//! clock exactly and the `@ms` stamps in the transcript never vary, and with its output only
//! recorded it needs neither /dev/uinput nor /dev/input.
//!
//! `UPDATE_SCENARIOS=1 cargo nextest run scenarios` rewrites the expected parts instead of
//! failing; review the diff of the files.

#[path = "common/mod.rs"]
mod common;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use common::{Daemon, install_config, spawn_daemon_alone, spawn_daemon_env, temp_root, wait_for};

const BIN: &str = env!("CARGO_BIN_EXE_padwight");

/// A scenario file, split into its parts.
struct Scenario {
    name: String,
    path: PathBuf,
    setup: Option<(String, Option<String>)>,
    config: Option<String>,
    model: Option<String>,
    full_keyboard: bool,
    /// Run the daemon alone in a pid namespace, with a process named `steam` in it or without.
    steam: Option<bool>,
    /// TOML files from the repository appended to the default config.
    append: Vec<String>,
    script: Vec<String>,
    expected: String,
}

fn parse(path: &Path) -> Scenario {
    let text = fs::read_to_string(path).unwrap();
    let (head, expected) = match text.split_once("\n---\n") {
        Some((head, expected)) => (head.to_string(), expected.to_string()),
        None => (text.trim_end_matches("---").to_string(), String::new()),
    };
    let mut s = Scenario {
        name: path.file_stem().unwrap().to_string_lossy().into_owned(),
        path: path.to_path_buf(),
        setup: None,
        config: None,
        model: None,
        full_keyboard: false,
        steam: None,
        append: Vec::new(),
        script: Vec::new(),
        expected,
    };
    for line in head.lines() {
        let words: Vec<&str> = line.split('#').next().unwrap_or_default().split_whitespace().collect();
        match words.as_slice() {
            [] => {}
            ["setup", pack] => s.setup = Some((pack.to_string(), None)),
            ["setup", pack, profile @ ..] => s.setup = Some((pack.to_string(), Some(profile.join(" ")))),
            ["config", file] => s.config = Some(file.to_string()),
            ["model", name] => s.model = Some(name.to_string()),
            ["full-keyboard"] => s.full_keyboard = true,
            ["steam", "running"] => s.steam = Some(true),
            ["steam", "not-running"] => s.steam = Some(false),
            ["append", file] => s.append.push(file.to_string()),
            _ => s.script.push(words.join(" ")),
        }
    }
    s
}

/// Runs `padwight debug <args>`, with `stdin` for a script.
type Debug<'a> = dyn Fn(&[&str], Option<&str>) -> Result<String, String> + 'a;

/// Lays out the scenario's configuration in `config_dir`.
fn install(s: &Scenario, config_dir: &Path, debug: &Debug) -> Result<(), String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    if s.config.as_deref() == Some("hold") {
        install_config(config_dir, &steady(&common::hold_config()));
    } else if let Some(file) = &s.config {
        install_config(config_dir, &fs::read_to_string(manifest.join(file)).unwrap());
    } else {
        let mut text = steady(&common::default_config());
        if s.full_keyboard {
            text = text.replacen("[active]\n", "full_keyboard = true\n\n[active]\n", 1);
        }
        for file in &s.append {
            text.push_str(&format!("\n{}\n", fs::read_to_string(manifest.join(file)).unwrap()));
        }
        install_config(config_dir, &text);
        if let Some((pack, profile)) = &s.setup {
            let pack = manifest.join(pack);
            let mut args = vec!["install-pack", pack.to_str().unwrap()];
            args.extend(profile.as_deref());
            debug(&args, None).map_err(|e| format!("install-pack failed: {e}"))?;
        }
    }
    Ok(())
}

/// Runs the script. `cli <args>` lines run `padwight <args>` against the daemon between the debug
/// words, for what only a command can ask for (`cli numpad-toggle`).
fn run_script(script: &[String], debug: &Debug, cli: &dyn Fn(&[&str]) -> std::process::Output) -> Result<String, String> {
    let run_chunk = |chunk: &mut Vec<&str>| {
        let text = chunk.join("\n");
        chunk.clear();
        debug(&["run"], Some(&text)).map_err(|e| format!("the script failed: {e}"))
    };
    let mut chunk: Vec<&str> = Vec::new();
    for line in script {
        if let Some(args) = line.strip_prefix("cli ") {
            run_chunk(&mut chunk)?;
            let args: Vec<&str> = args.split_whitespace().collect();
            let out = cli(&args);
            if !out.status.success() {
                return Err(format!("`padwight {}` failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr)));
            }
        } else {
            chunk.push(line);
        }
    }
    run_chunk(&mut chunk)
}

/// The transcript, unless the daemon panicked or logged trouble while the script ran.
fn verdict(transcript: Result<String, String>, log: &str) -> Result<String, String> {
    match transcript {
        Ok(t) if log.contains("panicked") => Err(format!("the daemon panicked:\n{log}\ntranscript:\n{t}")),
        // INS-9: a scripted session leaves no error, failure or warning in the daemon's log.
        Ok(_) if !common::log_problems(log).is_empty() => Err(format!("the daemon logged trouble:\n{}", common::log_problems(log).join("\n"))),
        other => other,
    }
}

/// What went wrong running a scenario's script.
fn run(s: &Scenario) -> Result<String, String> {
    let root = temp_root();
    let run_dir = root.join("run");
    fs::create_dir_all(&run_dir).unwrap();
    let config_home = root.join("config");
    let config_dir = config_home.join("padwight");
    let env_of = |command: &mut Command| {
        command.env("XDG_RUNTIME_DIR", &run_dir).env("XDG_CONFIG_HOME", &config_home).env_remove("PADWIGHT_DEBUG_SOCKET").stdin(Stdio::null());
    };
    let debug = |args: &[&str], stdin: Option<&str>| {
        let mut command = Command::new(BIN);
        env_of(&mut command);
        command.arg("debug").args(["--wait", "10000"]).args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        if stdin.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command.spawn().unwrap();
        if let Some(text) = stdin {
            child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
        }
        let out = child.wait_with_output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() { Ok(text) } else { Err(format!("{}\n{}", text, String::from_utf8_lossy(&out.stderr).trim())) }
    };

    install(s, &config_dir, &debug)?;

    // The daemon, and the controller attached to it.
    let _daemon: Daemon = {
        // No session bus either, so the desktop the tests run on can't switch profiles under a
        // scenario by focusing a window.
        let no_bus = Path::new("unix:path=/nonexistent/padwight-scenario-bus");
        let (args, env) = (["--debug", "--fake-clock"], [("DBUS_SESSION_BUS_ADDRESS", no_bus)]);
        let daemon = match s.steam {
            Some(running) => spawn_daemon_alone(&root, Some(&run_dir), &args, &env, running),
            None => spawn_daemon_env(&root, Some(&run_dir), &args, &env),
        };
        wait_for("the daemon to answer", || {
            let mut command = Command::new(BIN);
            env_of(&mut command);
            command.arg("status").output().ok()?.status.success().then_some(())
        });
        daemon
    };
    let mut session = Command::new(BIN);
    env_of(&mut session);
    session.args(["debug", "--headless", "--attach"]);
    if let Some(model) = &s.model {
        session.args(["--model", model]);
    }
    session.stdout(Stdio::null()).stderr(Stdio::null());
    let mut session = session.spawn().unwrap();

    let ran = run_script(&s.script, &debug, &|args| {
        let mut command = Command::new(BIN);
        env_of(&mut command);
        command.args(args).output().unwrap()
    });
    let transcript = ran.and_then(|_| debug(&["transcript"], None));
    let _ = debug(&["quit"], None);
    let _ = session.wait();
    let log = common::daemon_log(&root);
    let _ = fs::remove_dir_all(&root);
    verdict(transcript, &log)
}

/// Lines aligned so the `@ms` stamps line up in a column, for reading.
fn align(transcript: &str) -> String {
    let rows: Vec<(&str, &str)> = transcript.lines().map(|l| l.rsplit_once(" @").unwrap_or((l, ""))).collect();
    let width = rows.iter().map(|(text, _)| text.chars().count()).max().unwrap_or(0);
    let mut out = String::new();
    for (text, ms) in rows {
        if ms.is_empty() {
            out.push_str(text);
        } else {
            out.push_str(&format!("{text:<width$} @{ms}"));
        }
        out.push('\n');
    }
    out
}

/// The config with automatic profile switching off, so the profile a scenario starts in is the
/// one it keeps until the script changes it.
fn steady(config: &str) -> String {
    let steady = config.replacen("[auto_switch]\nenabled = true", "[auto_switch]\nenabled = false", 1);
    assert_ne!(steady, config, "the default config has automatic switching on");
    steady
}

fn normal(text: &str) -> String {
    text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n").trim().to_string()
}

#[test]
fn scenarios() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/scenarios");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "scenario")).collect();
    files.sort();
    assert!(!files.is_empty(), "no scenarios in {}", dir.display());
    let update = std::env::var_os("UPDATE_SCENARIOS").is_some();
    let only = std::env::var("SCENARIO").ok();
    let mut failures = Vec::new();
    for path in files {
        let s = parse(&path);
        if only.as_ref().is_some_and(|o| !s.name.contains(o.as_str())) {
            continue;
        }
        match run(&s) {
            Ok(transcript) if update => {
                let head = fs::read_to_string(&s.path).unwrap();
                let head = head.split_once("\n---\n").map_or(head.trim_end_matches("---\n").trim_end().to_string(), |(h, _)| h.to_string());
                fs::write(&s.path, format!("{head}\n---\n{}", align(&transcript))).unwrap();
            }
            Ok(transcript) => {
                if normal(&transcript) != normal(&s.expected) {
                    failures.push(format!("{}: the transcript changed (UPDATE_SCENARIOS=1 rewrites it)\n--- expected\n{}\n--- got\n{}", s.name, s.expected.trim(), align(&transcript).trim()));
                }
            }
            Err(e) => failures.push(format!("{}: {e}", s.name)),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn the_log_check_flags_trouble_and_lets_the_environment_through() {
    let log = "padwight daemon started\nrtkit: refused\nFailed to save the config\nthread 'main' panicked at x\nNo focus tracking\nAll fine\n";
    assert_eq!(common::log_problems(log), ["Failed to save the config", "thread 'main' panicked at x"]);
}
