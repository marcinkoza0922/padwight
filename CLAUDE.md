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
- Game detection and the Steam hand-off, no controller needed: `cargo run -q -- debug focus [pid [class]]|processes`,
  `debug steam [app id]`; `scripts/sandbox-pid.sh` runs a program in its own pid namespace (see
  `docs/development/code-review-plan.md`, which also tracks the review passes)
- Long files: `scripts/long-files.sh` (lists files over 400 lines; split them when you work in them)

Filter tests to the module you touched (e.g. `engine::`) unless the change is broad.
A hook runs `cargo check` after each edit to a `.rs` file and reports only failures.

## Rules

- Keep clippy at zero warnings. Fix new warnings rather than allowing them; use
  `#[expect(..., reason = "...")]` only where the lint is wrong for that code.
  Functions marked "predates the size lints" are a baseline: shrink them when you touch them.
- Don't open `Cargo.lock` or anything under `target/`.
- Never send screenshots to /tmp; they should always go into an untracked directory in the repository directory
