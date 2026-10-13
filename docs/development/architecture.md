# Architecture

This is for people working on the code. For what the app does, see the [README](../../README.md). The user-facing docs are the [pack format reference](../pack-format.md) and the [pack tutorial](../manual/tutorial-deus-ex.md).

## Processes

One binary, `padwight`, runs as several processes. `main.rs` picks the role from the first argument:

| Command | Role | Notes |
|---|---|---|
| `padwight` / `padwight gui` | Settings window (iced) | Talks to the daemon over IPC. If no daemon is running, it edits `config.toml` directly. |
| `padwight daemon` | The long-running service | The only process that grabs controllers and writes to uinput. The systemd unit in `dist/padwight.service` runs this. |
| `padwight overlay` | On-screen overlay (layer-shell surface) | Spawned by the daemon (`daemon.rs`, `current_exe()` + `overlay`). It stays running invisibly between uses so menus appear instantly. |
| `padwight debug` | Debug mode (CLI only) | A virtual controller of any model, in a window or `--headless`, driven by clicks or by `padwight debug <command>` from another terminal. Never touches real controllers or the daemon. See [Debug mode](#debug-mode). |
| `padwight status`, `profile <name>`, `reload`, `menu <name>`, … | One-shot CLI | Sends one request to the daemon and exits. |

Everything the daemon knows about the desktop comes from focus trackers (see below). Everything the GUI shows comes from the daemon or from the config file.

## Data flow

```
 evdev (physical pad)                              uinput (virtual devices)
        │                                                  ▲
        ▼                                                  │
  input.rs ── normalized InputEvent ──► daemon.rs ── OutEvent ──► output.rs
  (+ input/linux/touchpad.rs)            │    ▲                (Xbox 360-style pad,
                                         │    │                 keyboard, mouse)
                       ┌─────────────────┤    │
                       ▼                 ▼    │
                 menu.rs / offer.rs   engine.rs (per device)
                 overlay.rs           ├─ engine/stick.rs   sticks, zones, mouse/scroll ticks
                 (route input while   ├─ engine/flick.rs  flick stick
                  a menu/overlay      └─ engine/touchpad.rs
                  is up)
                       ▲
                       │ focus changes
        focus/ (sway, hyprland, kwin, gnome, wlroots, identify.rs → /proc)

 gui.rs ◄──── IPC: $XDG_RUNTIME_DIR/padwight.sock (ipc.rs, one JSON line per request)
   │
   └──── ~/.config/padwight/  (config.toml, setups/, general/; daemon is the only writer while it runs)
```

1. **Input.** `input/linux.rs` reads each physical pad's evdev node and turns raw events into normalized buttons (following the Linux gamepad spec), sticks in −1..1, and triggers in 0..1. Touchpads have their own node (`input/linux/touchpad.rs`) and reach the engine like buttons.
2. **Daemon.** `daemon.rs` grabs the physical devices so nothing else sees them, keeps one `engine` state per controller, and decides who gets each event: an open menu (`menu.rs`), the on-screen keyboard or numpad (`overlay.rs`), a library-game offer (`offer.rs`), or the mapping engine.
3. **Engine.** `engine.rs` turns normalized input into output for the active profile: buttons, gestures (double/triple tap, long press), combos, toggles, turbo, macros, layers, and zones. `engine/stick.rs` handles the sticks' continuous output (mouse, scroll, rings), `engine/flick.rs` the flick stick, and `engine/touchpad.rs` the touchpad.
4. **Output.** `output.rs` writes to virtual uinput devices: a pad (an Xbox 360-style one, or another controller a game supports, see `pad_identity.rs`), a keyboard and a mouse. Each physical controller's pad is rebuilt when the active game changes its identity. `rumble.rs` mirrors force-feedback uploads from games back to the physical pad.
5. **Focus.** `focus/` finds the focused window and its game, and the daemon picks the profile from the game's rules (see [Focus tracking](#focus-tracking)).

Stick output and timers (tap windows, long presses, turbo) run on the daemon's tick, not on input events alone, so a held stick keeps moving without new events.

## Module map

Line counts are approximate and change; `scripts/long-files.sh` lists the files over 400 lines.

### Core

| Path | Responsibility |
|---|---|
| `main.rs` | Command-line dispatch. |
| `config.rs` (and `config/`) | The config types: buttons, actions, profiles, games, rules, layers, menus, overlays, gyro, and the load/save/migration code. `config/summary.rs` writes short descriptions for the UI and logs. `config/touchpad.rs`, `config/log.rs` hold the touchpad and log-overlay settings. `config/modes.rs` holds modes (stored like layers) and the conversion of toggled layers into them. `config/style.rs` is how an overlay looks (`OverlayStyle`, its decorations, typography, ornaments and background picture); `config/performance.rs` the App settings' Performance card; `config/rumble_cues.rs` the rumble cues of each overlay. |
| `ipc.rs` | The `Request` and `Response` enums and the socket client/server helpers. |
| `daemon.rs` (and `daemon/`) | The service: device discovery and grabbing, the main loop, routing, and output dispatch. `daemon/evdev.rs` finds and reads the controllers; `daemon/logs.rs` records the input log; `daemon/touchpad.rs` runs touchpad readers. |
| `monitor.rs` | The one-line live status shown when the daemon runs in a terminal. |

### Input and mapping

| Path | Responsibility |
|---|---|
| `input.rs`, `input/linux.rs`, `input/linux/touchpad.rs` | evdev → normalized input (`input.rs` has the normalized types). Handles xpad's swapped X/Y labels. Touchpads report each finger by its contact slot, at 0..1 across the pad. |
| `engine.rs` | Per-device mapping state for the active profile. One of the largest files; its tests are in the same file. |
| `engine/stick.rs` | Sticks: gamepad feeds, zones, mouse and scroll output, rings. |
| `engine/flick.rs` | Flick stick. |
| `engine/touchpad.rs` | Touchpad fingers, each on the side of the pad it landed on: mouse (tap, two-finger scroll, gliding), a virtual stick, menu zones and swipes, and the click. Menu zones never take the controller: the engine picks the item and the daemon shows the menu in the overlay. |
| `menu.rs` | On-screen menus: all controller input while one is open. `menu/touch.rs` lays a menu out on the touchpad: which item is under a finger, and the overlay's view of it. |
| `offer.rs` | The "add this game from the library?" panel when Guide is tapped over an unknown game. |
| `system_menu.rs` | The Guide + Start menu over the game: Quick Settings and Edit Controls. See [in-game-editor.md](in-game-editor.md). |
| `layout_editor.rs` | The Edit Controls page of that menu: button actions of the active profile. |
| `rumble.rs` | Forwards rumble from the virtual pad to the physical one, and plays the rumble cues of overlays (`cue`). |
| `icons.rs` | `{icon:name}` icons: the built-in SVGs in `assets/icons`, and the active game's own (`set_pack_icons`, set by every process that draws text). |
| `assets.rs` | The user's `backgrounds/` folder beside `config.toml`, and where a file named in a pack is. |
| `config/assets.rs` | `PackAssets`: the icons (SVG) and sounds (base64 WAV) a game carries, which travel in its pack and work only in that game. |
| `info/catalog.rs` | Every token and formatting tag, grouped for the token picker (`gui/tokens.rs`). |
| `output.rs`, `output/uinput.rs` | The uinput devices. The pad can present itself as another controller (`pad_identity.rs`). |
| `pad_identity.rs` | Which controller the pad presents itself as: Xbox 360 unless the game ticks the physical controller's kind on its Details tab. USB IDs, name and button labels per identity. |
| `quit.rs` | ForceQuit: ends the focused game's process tree, never the desktop. |
| `capture.rs`, `record.rs` | Screenshot and screen recording, using whichever tool the desktop has. |
| `media.rs`, `media/mpris.rs` | MPRIS media controls on a worker thread, so a stuck player can't stall the daemon. |
| `keyboard.rs` | The on-screen key picker used in the GUI. |

### Focus

| Path | Responsibility |
|---|---|
| `focus/mod.rs` | Picks the active game from the focus event and the rules. Falls back to matching running processes when no tracker exists. |
| `focus/kwin.rs`, `focus/dbus.rs` | KDE Plasma: a KWin script reports focus changes over D-Bus. |
| `focus/sway.rs` | Sway (and i3-IPC compositors) over `$SWAYSOCK`. |
| `focus/hyprland.rs` | Hyprland's event and command sockets. |
| `focus/wlroots.rs` | wlroots compositors (labwc, Wayfire, river, …) over `wlr-foreign-toplevel-management`. Matches windows by class only, since the protocol doesn't give the process. |
| `focus/gnome.rs` | GNOME Shell: installs and enables the extension in `gnome-extension/`, which reports focus over the same D-Bus service. |
| `focus/identify.rs` | Identifies a window's executable, Steam App ID and class from `/proc`, including Wine/Proton `.exe` names. |
| `launchers.rs` | Which games are installed (Steam, Heroic, Lutris) or running, for the library picker. |

### Debug mode

| Path | Responsibility |
|---|---|
| `debug.rs` | `padwight debug`: starts a session (window or `--headless`), or sends commands and scripts (`run`) to a running one. |
| `debug/pad.rs` | The virtual controller's state; `snapshot()` gives the same `InputSnapshot` the daemon reports. |
| `debug/protocol.rs` | Request/response types and the command words that parse into them, including `expect` assertions. |
| `debug/bridge.rs` | `--attach`: has the daemon run the virtual controller as a controller of its own, sending each change as it is made. |
| `daemon/inject.rs` | The daemon's end: controllers no hardware backs (`DebugAttach`, `DebugInput`, `DebugOutput`, `DebugDetach`), with their output recorded. Only a daemon started with `--debug` accepts them. |
| `debug/server.rs` | The session socket (`padwight-debug.sock`, or `$PADWIGHT_DEBUG_SOCKET`) and its client. |
| `debug/window.rs` | The iced window: clickable virtual controller plus every model's drawing. |
| `debug/glyphs.rs` | Every glyph the overlays draw (each family's buttons, both face layouts, every key, the mouse buttons, the icons) as info sheets, shown in the debug window's Glyphs section. |
| `debug/overlay_shot.rs`, `debug/scenes.rs` | `padwight debug overlay`: draws made-up overlay frames (one per kind of overlay and menu), the glyph sheets, a saved frame, or a running daemon's live overlay to PNG with no display. |
| `debug/motion_shot.rs` | `padwight debug motion`: plays a scripted run (open, move, pick, close) of a scene in each motion style and writes an animated PNG, a strip of frames, or a trace of the motion values. |
| `debug/sound.rs`, `debug/sound_panel.rs` | `padwight debug sound`: plays cues from the command line (every kind, a kind with chosen volume/pitch/length/effects, or an overlay's default cues as a visit plays them), to hear them while tuning. The debug window has the same as a panel. Needs an audio device, no daemon. It plays the built-in defaults, not your config: to see what a running daemon plays, use `scripts/sound-trace.sh`. |
| `debug/focus.rs` | `padwight debug focus` and `padwight debug steam`: what the daemon makes of the window in front, of any pid (a sandboxed one too), of running processes, and of Steam's app cache. It shows the program, the rule that matches, controller support in Steam and who has the controllers, through the same code the daemon uses. See [the code review plan](code-review-plan.md#tools-for-checking-findings). |
| `snapshot.rs` | Draws any iced element offscreen with the CPU renderer (tiny-skia) into an image; what the overlay snapshots and their tests use. |
| `pad_svg/hit.rs` | Which part of a model's drawing a point falls on, for clicking. |
| `pad_widget.rs` | The drawing as an iced widget, shared with the settings window. |
| `touchpad_widget.rs` | The touchpad as a picture (each side's job, a menu's zones, the fingers), shared by the Touchpad tab and the debug window, where it can be dragged on. |

**Testing mappings end to end.** Run `padwight daemon --debug` (stop the service first, or give the test daemon its own `XDG_RUNTIME_DIR` and `XDG_CONFIG_HOME`), then `padwight debug --headless --attach`. The daemon treats the virtual controller like a real one: the active profile's mappings, layers, menus and overlays all react, and `padwight debug output` lists what the mappings produced (`key KEY_A down`, `pad South up`, `mouse-move 3 -2`, ...). That output is only recorded; `--live` also sends it to the real virtual keyboard, mouse and pad. Mapping actions that aren't output (screenshots, recording, force quit, the on-screen keyboard) happen either way, which is why the daemon needs `--debug`. A `--debug` daemon saves screenshots and recordings under `$TMPDIR/padwight-debug-<uid>/` (the same `Screenshots/<game>/` layout) instead of Pictures and Videos. Each command returns once the daemon has handled its input, so `expect output ...` right after `press ...` doesn't race. `padwight debug detach-all` removes a controller a crashed session left behind. `padwight debug identify <name|auto>` forces every virtual pad on a `--debug` daemon to present as another controller, whatever the game and the controller say; `auto` goes back to choosing.

`tests/debug_session.rs` uses all of this. Its session tests (commands, scripts, expectations, every model's drawing, sockets) need only the binary and run with the rest. Its daemon tests start a `--debug` daemon of their own and drive it through an attached session; they need `/dev/uinput`, and two of them read the virtual keyboard from `/dev/input`, so they're opt-in: `cargo test --test debug_session -- --ignored`, or `scripts/kernel-test-docker.sh debug_`.

**Checking which sound plays.** A `--debug` daemon logs every overlay cue it plays as a `sound:` line (`sound: guide Step -> pulse volume=0.1 pitch=1 length=1 file=None`), after the scope's sound set (the active setup's own, else App settings, with Performance's "No sounds" applied) has chosen it. `scripts/sound-trace.sh [config.toml]` starts such a daemon on throwaway config and runtime folders, attaches a virtual controller, drives the Guide overlay, the menu it leads to and Guide + Start, and prints those lines, so a sound setting that doesn't take effect shows up as the wrong kind there. `SETUP_TOML=file` appends a `[sounds...]` table to General's `setup.toml`. It needs `/dev/input` for the daemon's own scan but no speakers, display or uinput write access. Note that the Guide overlay and every menu it opens (Quick Settings, Guide + Start) use the `guide` overlay's sounds; only a setup's own menus use `menu`.

**Looking at the overlay without a display.** `padwight debug overlay all glyphs` writes a PNG of every scene and of the glyph sheets to `screenshots/overlay/` (untracked), drawn by the overlay's own layer code (`overlay/ui.rs`, settled, no motion) on a virtual screen (`--size`, default 1920x1080). `padwight debug overlay list` names the scenes. `--json` prints a scene's frame, which can be edited and drawn again by passing the `.json` file. `live` draws what a running daemon's overlay shows, so with a `--debug` daemon and an attached session, `padwight debug press ...` followed by `padwight debug overlay live` captures a real menu or keyboard. None of it needs Wayland, a compositor or a GPU. `padwight debug motion <style>|all` does the same for motion: it plays a run of a menu or the keyboard on a made-up clock and writes it to `screenshots/motion/` (see [Motion runs](overlay-debugging.md#motion-runs)).

Quick test loop: `padwight debug --headless &`, then `padwight debug --wait 3000 press south`, `padwight debug expect pressed south` (non-zero exit on failure), `padwight debug quit`. `padwight debug help` lists every command.

### Overlays and display

| Path | Responsibility |
|---|---|
| `overlay.rs` (and `overlay/ui.rs`, `overlay/draw/`) | The overlay process: keyboard, numpad, menus, info and log panels. `OverlayController` owns the state inside the daemon; `ui.rs` holds what is on screen and draws its layers, `layershell.rs` is the layer-shell window, and `snapshot` draws a frame offscreen as that window would. `draw/look.rs` resolves a style's decorations into container styles; `draw/glyphs.rs` draws button glyphs and icons (or text, under the performance settings). `draw/typeset.rs` sets text in a style's typography (case, letter spacing, size, weight, slant). `draw/ornament.rs` draws the background picture, corner frames, scanlines, vignettes and the lines between items. A panel's contents are the base of a `Stack`, and its picture and ornaments are layered under and over it, so they're the panel's size. Looping effects keep the overlay redrawing (`OverlayFrame::loops`) and read `Anim::clock`. |
| `info.rs` | Info overlays: `{token}` expansion into button glyphs and live values, and the generated bindings sheet for layers. |
| `font.rs` | Bundled fonts (`assets/fonts`) and installed families by name. |
| `style.rs` | Shared widget styles for the GUI. |
| `pad_svg.rs` | The SVG drawing of a live controller. Text is drawn by the GUI on top. |
| `inputlog.rs`, `inputlog/` | The input log: each press and what it did, grouped into sequences for log overlays. |

### Packs and library

| Path | Responsibility |
|---|---|
| `pack.rs` | `.padpack` parse, check, export, import planning and apply, versioning (`FORMAT`). Includes most of the pack tests. |
| `library.rs` | The built-in library, embedded at build time, and library update detection. Its checks run in `cargo test`. |
| `build.rs` | Embeds every `packs/*.padpack` into the binary. |
| `packs/` | The shipped library packs. |

### GUI (`gui.rs` and `gui/`)

| Path | Responsibility |
|---|---|
| `gui.rs` | The iced application shell: state, messages, and the daemon connection. The sidebar and game pages are in `gui/games.rs`. |
| `gui/profile.rs` | The profile editor (buttons, sticks and triggers, combos, gyro) and the controller drawing. Shared with the Layers tab. `gui/touchpad.rs` is its Touchpad tab. |
| `gui/actions.rs` | The action editor, including nested Multi, Toggle and Turbo. |
| `gui/layers.rs` | The Layers tab. |
| `gui/items.rs` | Macros, menus and info overlays: cards and editors. |
| `gui/games.rs` | Sidebar, a game's page and Details, Settings, controller list. |
| `gui/packs.rs` | Library picker, import preview, export, delete, copy-from-another-game. |
| `gui/checks.rs` | What "Save & apply" checks before writing: names, references, and rules. |
| `gui/widgets.rs`, `gui/style_editor.rs`, `gui/style_extras.rs`, `gui/tracking.rs`, `gui/overlays.rs`, `gui/ring_preview.rs`, `gui/logs.rs` | Shared widgets and smaller tabs. `style_extras.rs` holds the style editor's typography, ornament and background picture rows. |
| `gui/tab_icons.rs` | The glyph before each editor tab; Profiles and Buttons take the controller in use's family colors. |
| `pad_svg.rs`, `pad_svg/layout.rs`, `pad_svg/models.rs`, `pad_svg/parts.rs`, `pad_svg/labels.rs` | The live controller picture. `models.rs` holds each model's outline (mirrored, so both halves match), its shell color, and where its sticks, D-pad, buttons and marks sit, traced from the pictures in `references/controller images`; `layout.rs` defines what a model can have; `parts.rs` draws the shell, shoulders, sticks, D-pad and buttons; `labels.rs` places the mapping labels and their leader lines. |

## Configuration and state

- **Files.** `~/.config/padwight/config.toml` (`Config::path()`) holds the app-wide settings. `src/config/store.rs` reads and writes the rest: `general/` (the built-in **General** setup) and `setups/NNN-name/` (one per game). A `shared/` folder from older versions is folded into the setups that use it on load and removed on the next save. Each setup folder has `setup.toml` (name, rules, looks) and a subfolder per kind of item (`profiles/`, `layers/`, `macros/`, `menus/`, `info/`, `logs/`), with one numbered file per item. Each save rewrites the tree through a staging folder.
- **Single writer.** While the daemon runs, it is the only process that writes the file. The GUI sends `SetConfig` over IPC, and the daemon saves. After a hand edit, `padwight reload` tells the daemon to re-read it.
- **Unsaved edits.** The GUI keeps edits in memory until **Save & apply**. Revert drops them.
- **No migration.** Older single-file configs aren't read; the format can change freely. `Config::ensure_guide_layer` still adds the Guide layer to games that lack one.
- **Names.** Items are unique by kind within a game. See [specs.md](specs.md).

## Packs

A pack is a game in a file: TOML in the same shapes as the config, extension `.padpack`. `pack.rs` owns it:

- `FORMAT` is the version this build writes. Newer formats are refused with a message; older formats are upgraded on load. Bump it only for breaking shape changes, and document what changed in the comment above it.
- `parse` is strict (`deny_unknown_fields` on the top level). A malformed pack fails with the line and error, never half-imports.
- `check` rejects dangling references (macros, menus, info overlays, layers, profiles named by rules).
- `export` lists references to missing items, and keeps the pack ID on re-export.
- `plan` and `apply` do an import: they report name clashes and rule clashes, and `apply` writes the chosen result into the config.

Library packs are read at build time (`build.rs`) and checked by `cargo test` (`library.rs`). Adding a file to `packs/` is enough for it to appear in the picker; the tests say what it must satisfy.

## Testing and checks

| Command | What it does |
|---|---|
| `cargo check --all-targets` | Compiles everything, tests included. |
| `cargo clippy --all-targets` | Lints. The crate sets extra lints in `Cargo.toml` (`too_many_lines`, `cognitive_complexity`, and others). |
| `cargo test` | All unit tests, including every library pack check (`library.rs`, `pack.rs`). |
| `cargo nextest run` | The same tests under nextest. `.config/nextest.toml` has a quiet `agent` profile. |
| `scripts/profile.sh` | The profiling suite (`tests/profile/`): input latency, CPU and memory, start-up. Two parts need the host instead of the container: `profile_overlay_styles` (a Wayland session with layer-shell; what overlay looks, motion and the two renderers cost) and `profile_sound` (an audio device; what an open output stream costs). Run them with `cargo test --release --test profile -- --ignored --test-threads=1 --nocapture <name>`. Settings: `PADWIGHT_PROFILE_DIR` (where the JSON results go; default `target/profile`), `PADWIGHT_LATENCY_REPEATS` (sessions per latency run), `PADWIGHT_PROFILE_CASES` (run only the overlay style cases whose names contain it, such as `radial`). |
| `scripts/overlay-harness.sh` | Runs the real daemon and overlay on a headless KWin, drives them with the debug controller, and logs every frame sent and drawn. With the fade frames test and its check, see [overlay-debugging.md](overlay-debugging.md). |
| `tests/gnome_focus.rs` | GNOME focus tracking against a stand-in for GNOME Shell (`tests/fixtures/gnome_shell_mock.js`) on a private session bus: the daemon installs and enables the extension, reports GNOME Shell as its tracker, and accepts the extension's `WindowActivated` call. It doesn't prove real GNOME Shell loads the extension. Needs `gjs`, so it's opt-in: `dbus-run-session -- cargo nextest run --run-ignored ignored-only gnome_`. |
| `scripts/fake-pad.py <fifo>` | A gamepad made through `/dev/uhid`, so the daemon sees a HID device like a real controller (the same layout as `tests/common/uhid.rs`). `echo press > <fifo>` presses South once, `echo quit > <fifo>` removes it. The kernel drops reports until the daemon has opened the device, so send `press` again until it shows up. Needs write access to `/dev/uhid`. `scripts/desktop-test.sh` uses it on the test VMs. |
| `cargo test -- --ignored rumble` | The end-to-end rumble check. It needs `/dev/uinput`, so it is opt-in. |
| `python3 scripts/check-project.py` | Checks that the docs match the code: CLI commands, the pack format number, links and anchors, `flatpak/cargo-sources.json` against `Cargo.lock`, and bundled font licenses. CI runs it. |
| `scripts/kernel-test-docker.sh` | Runs the kernel-level tests in a Docker container, so the account needs no `input` group membership: `nothing_stays` (uinput), `daemon_` (the daemon-exit tests, which also need `/dev/uhid`) `socket_fallback` (run as root, since it makes a second user). `hotplug_` covers unplug and replug with a simulated controller. The KWin unit test (`focus::kwin`) needs the desktop session bus, so it runs on the host, not in the container. |
| `scripts/check-install.sh` | Validates the installed files: the systemd units, the udev rule, the desktop entry, the metainfo and the Flatpak manifest. Errors if a validator is missing. CI runs it. |
| `cargo audit` | Known vulnerabilities in `Cargo.lock`. CI runs it. |

A few conventions to know:

- **Formatting.** The code is not rustfmt-formatted as a whole, so running `cargo fmt` rewrites many unrelated files. Format only what you change, by hand.
- **File size.** Files over about 400 lines are candidates for splitting. `scripts/long-files.sh` lists them. Several of the largest (`config.rs`, `daemon.rs`, `engine.rs`, `gui/profile.rs`) are already over that, so put new code in a submodule rather than adding to them.
- **Warnings.** Clippy is kept at zero warnings. Fix a new warning rather than allowing it; use `#[expect(..., reason = "...")]` only where the lint is wrong for that code.
- **CI.** `.github/workflows/ci.yml` runs check, clippy with warnings denied, the tests, the project checks and `cargo audit` on every push and pull request.
- **Agents.** `CLAUDE.md` holds the commands and rules that automated agents in this repo follow.

## Focus tracking

The daemon picks the profile from the focused window. The tracker is chosen by the desktop:

1. **Sway** (`sway.rs`): subscribes to window events on `$SWAYSOCK`.
2. **Hyprland** (`hyprland.rs`): follows the event socket and asks the command socket for details.
3. **KDE Plasma** (`kwin.rs`): a small KWin script sends each focus change to the daemon's D-Bus service (`dbus.rs`).
4. **GNOME** (`gnome.rs`): a GNOME Shell extension sends each focus change over the same D-Bus service.
5. **wlroots compositors** (`wlroots.rs`): the `wlr-foreign-toplevel-management` protocol (labwc, Wayfire, river, and others). Windows match by class only.
6. **Fallback**: no tracker. Rules apply while a matching process runs.

`identify.rs` resolves a window to an executable name, Steam App ID (from `SteamAppId`/`STEAM_COMPAT_APP_ID` or Proton's `steam_app_<id>` class) and window class. Wine/Proton games are matched by their `.exe`, not the Wine loader. GNOME has no layer-shell, so the overlays can't be shown there. Focus tracking works through the GNOME Shell extension in `gnome-extension/`.

## Windows

Everything above describes Linux. The same binary builds for Windows, where the engine, config, packs, menus, overlays' drawing and the GUI are shared, and each place that talks to the system has a Windows file beside the Linux one. The `mod` line picks the file (`#[cfg_attr(windows, path = "…")]`), or the module declares both under `#[cfg]`:

| Linux | Windows | What it does there |
|---|---|---|
| `daemon/evdev.rs` | `daemon/xinput.rs` | The daemon's controllers (`scan_devices`): XInput's four slots, polled on a thread each, skipping the slots our own virtual pads take. |
| `input/linux.rs` | `input/xinput.rs` | Raw input to `InputEvent`. The Guide button and USB IDs come from `xinput1_4.dll`'s unnamed exports. |
| `output/uinput.rs` | `output/windows.rs` | The virtual pad through ViGEmBus (`vigem-client`; without the driver its output is dropped), the keyboard and mouse through `SendInput`, as scancodes. |
| `rumble.rs` | `rumble/windows.rs` | ViGEmBus reports the game's rumble; `XInputSetState` plays it. |
| `daemon/unix.rs` | `daemon/windows.rs` | Signals become the console's Ctrl+C and close events; the socket needs no permission bits in `%LOCALAPPDATA%`. |
| `overlay/layershell.rs` | `overlay/windows.rs` | A layered window (`WS_EX_LAYERED`, `WS_EX_TRANSPARENT`, `WS_EX_NOACTIVATE`): `overlay/ui.rs`'s layers drawn by the CPU renderer (`snapshot.rs`) and handed over with `UpdateLayeredWindow`. |
| `focus/*.rs` | `focus/windows.rs` | A WinEvent hook on foreground changes; processes by `QueryFullProcessImageNameW`; Steam App IDs from `steamapps\common\<folder>` and its manifest. |
| `keyboard/xkb.rs` | `keyboard/windows.rs` | Key labels from the active layout (`ToUnicodeEx`). |
| `tray.rs` | `tray/windows.rs` | `Shell_NotifyIconW` on a hidden window. |
| `media/mpris.rs`, `media/mixer.rs` | `media/windows.rs`, `media/windows_mixer.rs` | The system media session (GSMTC) and the default output's endpoint volume. |
| `capture.rs`, `record.rs` | `capture/windows.rs`, `record/windows.rs` | GDI screenshots; recording with `ffmpeg`. |
| `quit.rs` | `quit/windows.rs` | `WM_CLOSE` to the game's windows, then `TerminateProcess`. |
| `info.rs` | `info/windows.rs` | The clock, CPU, memory and battery from Win32. |

Key codes stay Linux's everywhere (`keycode.rs`: evdev's `KeyCode` on Linux, the same table generated from it elsewhere), so configs and packs are the same files on both. `ipc.rs` uses AF_UNIX sockets on both, through `uds_windows` on Windows; `ipc::watch` reads the streaming requests on a thread, the same on both. The integration tests that need uinput, uhid or D-Bus are Linux only; `tests/debug_session.rs`'s session tests run on both. CI builds, lints and tests on both.

To check Linux code from Windows, `cargo check --target x86_64-unknown-linux-gnu` works once `PKG_CONFIG` points at a stub that answers for `xkbcommon` (nothing is linked).

## Where to start reading

- To follow one button press: `input.rs` → `daemon.rs` (routing) → `engine.rs` (button, gesture and combo handling) → `output.rs`.
- To change what a profile can do: `config.rs` (types and defaults), then `engine.rs`, then the editor in `gui/profile.rs`, then `pack.rs` if packs need it.
- To add a built-in action: `ButtonAction` in `config.rs`, its summary in `config/summary.rs`, the engine, the action picker in `gui/actions.rs`, and the pack tests in `pack.rs`.
- Design notes for larger features are in this folder: [specs.md](specs.md) for games and packs, [layers.md](layers.md), [sticks.md](sticks.md) and [guide.md](guide.md).
