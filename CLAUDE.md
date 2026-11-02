# Agent notes

## Commands

Keep output short. Use these exact commands, never plain `cargo build` or `cargo test`:

- Check: `cargo check --all-targets --message-format=short 2>&1 | head -40`
- Lint: `cargo clippy --all-targets --message-format=short`
- Test: `RUST_BACKTRACE=0 cargo nextest run --profile agent --hide-progress-bar --cargo-quiet [filter]`
- Overlay screenshots, no display needed: `cargo run -q -- debug overlay all glyphs` (PNGs in
  `screenshots/overlay/`; `debug overlay list` names the scenes, `live` captures a running daemon's overlay,
  and a `.padpack` path draws that pack's menus, keyboard, numpad and info overlays in its own look)
- What the overlay shows, as text: `cargo run -q -- debug overlay state` (JSON of a running daemon's overlay);
  in debug sessions `expect overlay <path> <value>` (shape in `src/debug/overlay_state.rs`)
- Old vs new overlay renderer: `cargo run -q -- debug overlay compare all|<scene>` (iced | new | difference PNGs and
  a score in `screenshots/compare/`, worst first; scenes the new renderer can't draw yet are listed as not ported)
- Picture regression, no display needed: `cargo run -q -- debug overlay baseline <dir>` (scenes, glyphs, motion strips,
  settings tabs light and dark; `debug gui baseline <dir>` for the settings window alone), later
  `debug overlay diff <dir>` (changed pictures worst first, difference PNGs in `screenshots/regression/diff`, exit 1 if any)
- Motion styles, no display needed: `cargo run -q -- debug motion <style>|all` (animated PNG + frame strip in
  `screenshots/motion/`; `--from/--until/--every` pick the strip's span, `--scene keyboard`, `--trace` prints numbers)
- Which overlay cue plays, no display or speakers needed: `scripts/sound-trace.sh [config.toml]` (a `--debug` daemon
  logs each cue as a `sound:` line); `cargo run -q -- debug sound list|<kind>|<overlay>` plays cues to hear them
- How loud and how grating cues are, no speakers needed: `python3 -I scripts/sound-audit.py [--level 60] path...`
  (loudness in sone, sharpness, roughness; writes a listening page to `sound-previews/audit/`). Render the
  kinds and default cues first with `PADWIGHT_PREVIEW_DIR=dir` and the ignored `sound::tests::render_previews`
- Real input: `cargo run -q -- debug record <file> [--device name]` saves a controller's events and capabilities
  (no grab); `debug replay <file> --uhid` plays them back as a HID pad (also reads `evemu-record` files)
- Game detection, no controller needed: `cargo run -q -- debug focus [pid [class]]|processes`;
  `scripts/sandbox-pid.sh` runs a program in its own pid namespace (see
  `docs/development/code-review-plan.md`, which also tracks the review passes)
- Soak and chaos run (Docker, seeded, `PADWIGHT_SOAK_SECONDS`/`_SEED`/`_STEPS`): `scripts/soak.sh`; see `docs/development/qa-tooling.md` T-6
- Scenario tests, no display or devices needed: `RUST_BACKTRACE=0 cargo nextest run --profile agent --hide-progress-bar --cargo-quiet scenarios`
  runs `tests/scenarios/*.scenario` (script plus saved transcript); `UPDATE_SCENARIOS=1` rewrites them, `SCENARIO=name` picks some.
  They run a `--debug --fake-clock` daemon: `advance <ms>`/`wait <ms>` in a debug script move its clock exactly, `debug transcript` prints what it did
- Input latency, CPU and memory (Docker, uhid pad): `scripts/profile.sh [filter]`
  (`PADWIGHT_PROFILE_OUT=dir`); a daemon with `PADWIGHT_INPUT_TIMING=file` logs each input's stages
- Overlay CPU and memory on a headless KWin: `PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh
  scripts/harness/resources.sh <out>` (`gui-idle.sh` for the settings window)
- Performance regressions: `python3 -I scripts/profile-compare.py old/ new/` over two `PADWIGHT_PROFILE_OUT` folders
- Weekly desktop VM run and the qa-plan table: `scripts/desktop-weekly.sh [--dry-run]`
- Mutation testing: `scripts/mutants.sh engine|menu|gui-checks|config-store`; coverage: `cargo llvm-cov nextest`
- Game facts (Steam ids, executables, controller support, config paths): local PCGamingWiki snapshot and
  Discord's detectable-games list in `pcgamingwiki/`, see its `README.md`; never read `games.jsonl` whole
- Pack identity audit (Steam ids, executables, release, controller support vs the PCGamingWiki snapshot; keeps the
  manual columns of `docs/development/pack-qa.tsv`): `python3 -I scripts/pack-audit.py [-v|--check|--self-test]`
- Pack keys against the game's own bindings: `python3 -I scripts/binding-compare.py list|compare [game]|self-test`
  (readers for Unreal `User.ini`, Dark engine `.bnd`, id Tech/Source cfg, F.E.A.R. profile, Max Payne registry)
- Fuzzing (needs `cargo install cargo-fuzz` and nightly): `scripts/fuzz.sh [target [seconds]]` runs one or all seven
  targets in `fuzz/`; corpus in `fuzz/corpus`, crashes in `fuzz/artifacts` (neither in git); see `docs/development/qa-tooling.md` T-5
- Long files: `scripts/long-files.sh` (lists files over 400 lines; split them when you work in them)

Filter tests to the module you touched (e.g. `engine::`) unless the change is broad.
A hook runs `cargo check` after each edit to a `.rs` file and reports only failures.

## Rules

- Breaking changes are fine: the app has no outside users yet. Don't keep old config or pack fields
  readable, add migrations or bump formats just for compatibility; remove what's gone.
- Keep clippy at zero warnings. Fix new warnings rather than allowing them; use
  `#[expect(..., reason = "...")]` only where the lint is wrong for that code.
  Functions marked "predates the size lints" are a baseline: shrink them when you touch them.
- Don't open `Cargo.lock` or anything under `target/`.
- Creating or updating a setup pack: look the game up in `pcgamingwiki/` first (`games.tsv`, `executables.tsv`,
  the article's Input section) for its Steam app ids, executable names, release date and native controller
  support, before guessing or searching the web. Follow `docs/development/game-setup-guidelines.md`.
- Never send screenshots to /tmp; they should always go into an untracked directory in the repository directory
