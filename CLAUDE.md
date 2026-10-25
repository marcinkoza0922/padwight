# Agent notes

## Commands

Keep output short. Use these exact commands, never plain `cargo build` or `cargo test`:

- Check: `cargo check --all-targets --message-format=short 2>&1 | head -40`
- Lint: `cargo clippy --all-targets --message-format=short`
- Test: `RUST_BACKTRACE=0 cargo nextest run --profile agent --hide-progress-bar --cargo-quiet [filter]`
- Overlay screenshots, no display needed: `cargo run -q -- debug overlay all glyphs` (PNGs in
  `screenshots/overlay/`; `debug overlay list` names the scenes, `live` captures a running daemon's overlay,
  and a `.padpack` path draws that pack's menus, keyboard, numpad and info overlays in its own look)
- Motion styles, no display needed: `cargo run -q -- debug motion <style>|all` (animated PNG + frame strip in
  `screenshots/motion/`; `--from/--until/--every` pick the strip's span, `--scene keyboard`, `--trace` prints numbers)
- Which overlay cue plays, no display or speakers needed: `scripts/sound-trace.sh [config.toml]` (a `--debug` daemon
  logs each cue as a `sound:` line); `cargo run -q -- debug sound list|<kind>|<overlay>` plays cues to hear them
- How loud and how grating cues are, no speakers needed: `python3 -I scripts/sound-audit.py [--level 60] path...`
  (loudness in sone, sharpness, roughness; writes a listening page to `sound-previews/audit/`). Render the
  kinds and default cues first with `PADWIGHT_PREVIEW_DIR=dir` and the ignored `sound::tests::render_previews`
- Game detection, no controller needed: `cargo run -q -- debug focus [pid [class]]|processes`;
  `scripts/sandbox-pid.sh` runs a program in its own pid namespace (see
  `docs/development/code-review-plan.md`, which also tracks the review passes)
- Input latency, CPU and memory (Docker, uhid pad): `scripts/profile.sh [filter]`
  (`PADWIGHT_PROFILE_OUT=dir`); a daemon with `PADWIGHT_INPUT_TIMING=file` logs each input's stages
- Overlay CPU and memory on a headless KWin: `PADWIGHT_HARNESS_RELEASE=1 scripts/overlay-harness.sh
  scripts/harness/resources.sh <out>` (`gui-idle.sh` for the settings window)
- Game facts (Steam ids, executables, controller support, config paths): local PCGamingWiki snapshot and
  Discord's detectable-games list in `pcgamingwiki/`, see its `README.md`; never read `games.jsonl` whole
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
