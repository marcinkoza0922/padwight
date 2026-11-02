# QA tooling to build

Tools and test functionality that [qa-plan.md](qa-plan.md) needs and that don't exist yet. Each
names the gaps it closes there. They build on what's already in the tree: the debug session
(`padwight debug`, `daemon/inject.rs`), the uhid test controller (`tests/common/uhid.rs`), the
container runner (`scripts/kernel-test-docker.sh`), the off-screen renderers (`debug overlay`,
`debug gui`), the overlay harness, the profiling suite and the `pcgamingwiki/` snapshot.

## Order of work

Ranked by what it catches per day spent. Effort: S is a day or two, M about a week, L more.

| Rank | Tool | Effort | Catches | Tier |
|---|---|---|---|---|
| 1 | [T-1](#t-1-scenario-tests) Scenario tests with recorded transcripts | M | Wrong output across feature combinations, in CI | T1 |
| 2 | [T-2](#t-2-a-clock-the-daemon-can-be-handed) A clock the daemon can be handed | M | Timing bugs at the daemon level, deterministically; makes T-1 fast and stable | T1 |
| 3 | [T-12](#t-12-scheduled-runs-and-performance-baselines) Scheduled runs and performance baselines | S | Regressions in the layers nobody remembers to run | all |
| 4 | [T-13](#t-13-pack-identity-audit-and-status-table) Pack identity audit | S | Wrong Steam IDs and executable names in 232 packs, offline | T1 |
| 5 | [T-5](#t-5-fuzzing) Fuzzing | S | Panics and hangs on hostile packs, configs, assets and IPC | T0, T3 |
| 6 | [T-3](#t-3-controllers-with-real-kernel-drivers) Controllers with real kernel drivers | M–L | Gyro, touchpad, battery and Nintendo/PlayStation driver paths without hardware | T0, T1 |
| 7 | [T-18](#t-18-record-and-replay-real-input) Record and replay real input | M | Turns a player's hardware bug into a test | T0, T1 |
| 8 | [T-10](#t-10-gui-tests-that-check-the-saved-config) GUI tests that check the saved config | M | Editors saving something other than what was shown | T3 |
| 9 | [T-4](#t-4-asserting-what-the-overlay-shows) Asserting what the overlay shows | S | Menus and overlays in the wrong state, checked without pictures | T2 |
| 10 | [T-8](#t-8-picture-regression) Picture regression | M | Unintended visual changes in overlays and the settings window | T4 |
| 11 | [T-6](#t-6-soak-and-chaos-runs) Soak and chaos runs | M | Leaks, stuck output and panics over hours of mixed use | T0 |
| 12 | [T-9](#t-9-fault-injection) Fault injection | M | Write errors, a full disk, a crash mid-save | T0 |
| 13 | [T-7](#t-7-stand-in-compositors) Stand-in compositors | M | Focus backends (Sway, Hyprland, niri, wlroots, COSMIC, KWin, Cinnamon) in CI | T1 |
| 14 | [T-19](#t-19-coverage-and-mutation-testing) Coverage and mutation testing | S | Code no test runs; tests that pass whatever the code does | all |
| 15 | [T-14](#t-14-binding-comparison) Binding comparison | M per engine | Pack keys that don't match the game's controls | T1 |
| 16 | [T-17](#t-17-bug-report-bundle) Bug report bundle | S | Missing facts in manual QA runs and user reports | — |
| 17 | [T-15](#t-15-protocol-conformance) Protocol conformance | S | `docs/protocol.md` drifting from the daemon | T3 |
| 18 | [T-11](#t-11-gui-layout-checks) GUI layout checks | M | Clipped and overflowing text, dark-theme contrast | T4 |

## T-1: Scenario tests

**Gap.** The engine has dense unit tests, but the daemon has 3 tests per thousand lines, and nothing
checks feature combinations end to end in CI: a gesture inside a layer inside a mode while a cycle
steps, a menu opened from a combo while a macro runs, a profile switch with a toggle on.

**What to build.** A folder of scenario files, `tests/scenarios/*.scenario`, run by one test
binary. Each file has three parts:

```text
# config: a setup tree, or a library pack to import, or inline TOML
setup  packs/deus-ex.padpack
profile "Gameplay"

# script: the debug session's command words, unchanged
press lb
tap south
wait 300
stick right 1 0
release lb

# expected: the transcript of output, written by the tool the first time
---
key KEY_LEFTSHIFT down     @0
key KEY_F1 down            @0
key KEY_F1 up              @50
...
```

The runner starts a `--debug` daemon on throwaway config and runtime folders (as
`scripts/sound-trace.sh` does), attaches a virtual controller, plays the script and compares
`debug output` with the expected part. `UPDATE_SCENARIOS=1` rewrites the expected part instead of
failing, so a change in behaviour shows up as a reviewable diff of the transcript (the
[insta](https://insta.rs) style; the crate itself can do the snapshot handling). Timestamps come
from T-2, so they are exact and the test is not timing-sensitive.

Recorded output covers keys, mouse and pad. Add the other things a mapping can do to the
transcript: profile, mode and layer changes, menus opened and items picked, overlays shown, toasts,
and rumble cues, so a scenario sees them too (they are already recorded for the GUI's *What's
happening* list).

**Start with.** One scenario per README feature heading, one per menu style, one per engine
gesture, and the library's tier-P1 packs ([qa-plan.md section 6](qa-plan.md#6-pack-qa)) through
their main actions. Every T1 bug fixed gets a scenario.

**Note.** Runs without `/dev/uinput` if the daemon's output is only recorded (it is, without
`--live`), and needs no controller scan. If the daemon can be started without `/dev/input` access,
this runs in plain CI.

**Status.** Built: `tests/scenarios.rs` runs every `tests/scenarios/*.scenario` (32 now) in one test,
`cargo nextest run --profile agent scenarios`; `SCENARIO=name` runs the ones whose name contains it,
`UPDATE_SCENARIOS=1` rewrites the expected parts. The header takes `setup <pack> [profile]` (a pack
from `packs/` or `tests/scenarios/packs/`, imported with the new `padwight debug install-pack`),
`config hold` (South is KEY_A, East the left button) or `config <file>`, and `model <name>`; every
other line goes to the debug session unchanged (`press`, `tap`, `wait`, `advance`, `stick`, `expect`
and whatever words are added to it later). The daemon runs `--debug --fake-clock`, so stamps are
exact and the whole set takes about 7 s. The expected part is the new `transcript` word's output: the
injected controller's output plus `profile`, `shifts` (mode and layers), `overlay` (the panel in front
and its selection), `toast`, `sound` and `rumble` lines, each with `@ms`. Pad axes sent a value they
already have are left out. Seed set: keys and mouse, each gesture, combo-window edges, toggle and
turbo, ring and zones, trigger button and zones, layers, modes, macros, cycles, the six menu styles,
five library packs (Deus Ex x3, Civilization VI, StarCraft, Doom) and the Guide overlay, keyboard
and Quick Settings. Left: typing on the on-screen keyboard is not in the transcript (it goes to the
virtual keyboard directly, not through an injected controller); info overlays show as a count;
rumble lines appear only where a setup defines a cue; `expect overlay` (T-4) will slot in as a
script word; more library packs and every fixed T1 bug still to be added.

## T-2: A clock the daemon can be handed

**Gap.** The engine takes `now: Instant` as an argument, which is why its timing tests are
reliable. The daemon reads the wall clock in its main loop, timers and tick, so anything through the
daemon (debug sessions, container tests) depends on real time: slow, and flaky under load at the
edges of a combo window or a long press.

**What to build.** A `Clock` the main loop reads (`now()`, and how long to sleep until the next
timer), with the real one by default and a manual one for a `--debug` daemon started with
`--fake-clock`. The debug protocol gains `advance <ms>`, which moves the clock and runs every tick
and timer that falls inside, then answers. Scripts then say `advance 59` and `advance 1` to test both
sides of the 60 ms combo window exactly.

**Closes.** Daemon-level timing tests; makes T-1 fast (no real sleeps) and stable.

**Status.** Built. `src/clock.rs` holds the clock (`clock::now()`; the daemon reads it everywhere it
used to read `Instant::now()`). `padwight daemon --debug --fake-clock` makes it manual, so time moves
only when a session says `advance <ms>` (runs every tick, timer and scan that falls inside, each at
its exact time, then answers) or `wait <ms>` (an `advance` on a fake clock, a real wait otherwise;
`tap`, `swipe` and `gyro <..> <ms>` use it too). On the fake clock the daemon scans for no real
controllers, games or overlay process, and its keyboard and mouse exist only in memory, so it needs
neither /dev/uinput nor /dev/input. The loop's body is `Daemon::step` (`daemon/main_loop.rs`);
`Daemon::advance` is the fake clock's only driver. Left: the overlay process and the settings window
still read the real clock (they are separate programs), and a few daemon threads (the update check,
the rumble and sound threads) sleep for real.

## T-3: Controllers with real kernel drivers

**Gap.** `tests/common/uhid.rs` makes a generic HID pad. The real controllers go through
`hid-playstation`, `hid-sony`, `hid-nintendo` and `xpad`, which create the motion-sensor and
touchpad nodes, report battery, and label buttons their own way. Gyro, the touchpad, the motion
node arriving after the pad (HW-4) and the gyro fallback on replug can't be tested without them.

**What to build, in three steps:**

1. **uhid replicas.** Give the uhid helper the vendor and product IDs and report descriptors of a
   DualSense, a DualShock 4 and a Switch Pro, and answer the feature reports their drivers ask for
   while probing (calibration, firmware, pairing info for the PlayStation drivers; the subcommand
   handshake for `hid-nintendo`). The kernel then binds the real driver, which makes the real
   gamepad, motion and touchpad nodes. [hid-tools](https://gitlab.freedesktop.org/libevdev/hid-tools)
   already has device descriptions and replies for Sony pads that its kernel tests use; start from
   those. Needs the drivers loaded on the host running the container.
2. **USB gadget for xpad.** `xpad` binds only to USB devices. `dummy_hcd` with `raw_gadget` makes
   a USB device in software, so an Xbox pad can be emulated through the real USB stack. Host
   modules again; worth it only once step 1 shows the pattern pays.
3. **A hardware rig.** A Raspberry Pi Pico (or similar) acting as a USB controller of several
   identities, driven over its serial port by the test, behind a hub whose ports can be switched off
   (`uhubctl`) for real unplugs. It tests the whole USB path on the real machine, and can measure
   press-to-output latency including USB polling, which the uhid profiling run leaves out.

**Status.** Step 1 is built for the DualSense. `HidPad::dualsense(n)` in `tests/common/uhid.rs`
(with `uhid/dualsense.rs`) creates a 054c:0ce6 pad on USB and answers the calibration, pairing and
firmware feature reports from a thread, so the host's `hid-playstation` binds and makes the
gamepad, "... Motion Sensors" (INPUT_PROP_ACCELEROMETER) and "... Touchpad" nodes. `DualSenseState`
builds input reports (gyro, accel, two fingers, click). The pad's unique id is its MAC,
`02:70:61:64:00:nn`, which `default_config` recognises as a test pad. Run it with
`scripts/kernel-test-docker.sh replica_` (`tests/replica.rs`): real-driver nodes, gyro and
touchpad reaching the engine (pointer motion, click), and the sensor pairing across four plugs.
Not done: the DualShock 4 (same driver; feature reports 0x02, 0x12, 0xa3 and a 64-byte input report
with three touch reports, laid out in `hid-playstation.c`) and the Switch Pro (`hid-nintendo`
needs a reply to each output subcommand). The motion node's lateness (HW-4) is a race inside the
kernel's probe, not forced, and the gyro-fallback on replug needs a profile that requires gyro.

**Closes.** HW-4, HW-5, HW-7's suspend and resume (with the rig), the gyro-fallback gap noted for
the hot-plug tests, and FEEL-5's measurement.

## T-4: Asserting what the overlay shows

**Gap.** Menus are checked by pictures. A test that wants to know whether the Guide overlay is up
with its second item selected has to compare pixels.

**What to build.** `padwight debug overlay state`: the live overlay's state as JSON (which panels
are showing, a menu's items and selection, the keyboard's key and latches, an info overlay's cells
after token expansion). The daemon already has this in `OverlayController`; `debug overlay live`
draws it. Then `expect overlay <path> <value>` in the debug protocol, for scripts and T-1:

```text
tap guide
expect overlay showing guide
press dpad-down
expect overlay guide.selected "Keyboard"
```

**Status.** Built. `padwight debug overlay state [--wait ms]` prints the state document of a
running daemon (any daemon, not only `--debug`; it reads the frame the overlay window would get,
so no new IPC request). `expect overlay <path> <value>` is a debug-session word that asks the same
document, retrying for half a second, and fails with what is actually there (or the keys a bad
path could have used). The document's shape is in the doc comment of `src/debug/overlay_state.rs`:
`showing` (list of names, `guide`, `keyboard`, `info:<name>` ...), `menu` (also under the menu's
title, so `guide.selected`), `keyboard` (`key`, `latched`, `typed` ...), `media`, `offer`, `info`
(cells as text after token expansion), `logs`. Values match strings exactly, numbers within 0.01,
`none` matches null or an empty list, and a list matches when one item does. Tests:
`overlay_state::` unit tests, `expect_overlay_fails_clearly_without_a_daemon`, and the ignored
daemon tests `debug_the_guide_overlay_*`, `debug_the_keyboard_*`, `debug_menus_of_different_styles_*`
in `tests/debug_session/overlay.rs`. Left: info-overlay and doodle/lens/peek assertions have no
daemon test; the document does not yet cover the settings/new-pad panels beyond their menu view.

## T-5: Fuzzing

**Gap.** The damaged-input sweeps (truncations, deletions) catch a lot, but they aren't guided by
coverage, and the newer parsers have no sweep at all.

**What to build.** `cargo fuzz` targets, each a few lines over an existing entry point:

| Target | Entry point | Why |
|---|---|---|
| `pack_parse` | `pack::parse`, then `check` and `plan` | Packs come from strangers |
| `pack_assets` | the SVG icon and base64 WAV loaders in `config/assets.rs` | Strangers' images and sounds, through resvg and a WAV decoder |
| `config_load` | `Config::load_from` over a generated folder tree | A hand-edited config must not stop the daemon |
| `ipc_request` | `ipc` request parsing | The socket is a public protocol now |
| `tokens` | info-overlay token and formatting-tag expansion (`info/`) | Text from packs |
| `calculator` | `tools/calculator.rs` | Typed by the player |
| `engine_input` | a sequence of `InputEvent`s and `now` steps into `Engine::handle` and `timers`, then `release_all`, asserting nothing is held | The T0 invariant, coverage-guided |

Seed each with the library packs, the fixtures and the debug scenes. Run each for a minute in CI
(T-12) and for an hour weekly; keep the corpus in a separate repository or cache, not in git.

**Status.** Built: the seven targets in the table, in `fuzz/` (a separate crate, outside the normal
build). The program is a binary crate, so `src/fuzz_lib.rs` is a second crate root with the same
module list as `main.rs` (keep the two equal), which `fuzz/Cargo.toml` builds as a library; the
entry points are in `src/fuzz_targets.rs`, which sees private items, and each target is a one-line
`fuzz_target!`. `fuzz/{packs,assets,protocols,build.rs}` are symlinks so the build script and the
protocol macros find their files. `config_load` reads a text of files, each started by a line
`#### relative/path`. Run `scripts/fuzz.sh [target [seconds]]` (needs `cargo install cargo-fuzz`
and nightly; set `CARGO_TARGET_DIR` to share a build folder). It builds the seed corpus on the
first run (`scripts/fuzz-seeds.py`: the library packs, built-in icons, sounds and fonts, token
strings of the packs and debug scenes, IPC lines; config folders come from the program itself) and
runs without a sanitizer (`-s none`) to keep the build short. Corpus and crashes stay in
`fuzz/corpus` and `fuzz/artifacts`, not in git. Left: the weekly hour-long run and a shared corpus
cache (T-12) and `touchpad` events in `engine_input`.

First runs, two minutes per target on two workers: `engine_input` (about 200k runs), `calculator`
(440k), `ipc_request` (17M), `tokens` (1.1M), `pack_parse` (250k) and `config_load` (1.3k, slow:
each run writes a folder) found nothing. `pack_assets` found two faults in the font parser, both
from a few bytes of a damaged TrueType collection in a pack: an assertion panic in `ttf-parser`
and a hang on a header claiming 875 million fonts. Fixed in `font::family_of` (the count must fit
the file; the parse is wrapped in `catch_unwind`), with regression tests in `font.rs`.

## T-6: Soak and chaos runs

**Gap.** The idle test runs 5 seconds. Leaks of file descriptors, threads or memory, and stuck
output after long mixed use, show only over hours.

**What to build.** A runner on a `--debug` daemon in the container that, for a set time, picks
random steps from: input on one to four virtual pads, hot-plug through uhid, profile, mode and
layer switches, `reload`, `SetConfig` with a changed config, opening and closing every overlay,
killing and `SIGSTOP`ping the `padwight overlay` process, and fake focus changes (T-7). Every few
minutes it releases everything and checks the invariants: nothing held in the kernel, no panic in
the log, resident memory, open descriptors and thread count within a bound of their start. It
saves its seed, so a failure can be replayed exactly. Run overnight weekly.

**Status.** Built: `tests/soak/` (an `#[ignore]` test, `soak_run`) and `scripts/soak.sh`, which runs it
in the Docker container against a `--debug` daemon. Seeded steps: input on 1-4 uhid pads
(`HidPad::with_uniq`), plug and unplug, `SetProfile`/`Activate`/`NextProfile`/lock, the Guide mode
(North held), `Reload`, `SetConfig` (changes every `long_press_ms`), disable/enable, keyboard and
numpad overlays, `CloseOverlays`, SIGKILL/SIGSTOP/SIGCONT of the `padwight overlay` child, and a
focus hook. At each check it releases everything, then asserts the kernel's key state on every
`Padwight Virtual` device is empty, the daemon answers, the log has no `panicked`/`output error`,
at most one overlay process, and RSS, fds and threads are within a bound of the start. Run:
`scripts/soak.sh` (60 s); `PADWIGHT_SOAK_SECONDS=600 PADWIGHT_SOAK_CHECK_SECONDS=60 scripts/soak.sh`;
overnight: `PADWIGHT_SOAK_SECONDS=28800 scripts/soak.sh` (not under the shared Docker lock; use it only
when no other run needs the container). Replay: `PADWIGHT_SOAK_SEED=n PADWIGHT_SOAK_STEPS=k`.
Output (`soak.csv` trend, `steps.log`, `daemon.log`) goes to `soak-runs/`. Settings are listed at
the top of `tests/soak/main.rs`. Focus changes: set `PADWIGHT_SOAK_FOCUS_HOOK` to a command that
takes a window class (T-7 plugs in there). Left: the container has no compositor, so the overlay
process exits at once and the kill/stop steps mostly find nothing (needs T-7's wlroots stand-in);
no menu is configured for `OpenMenu`; only key state (not axes) is checked in the kernel.

## T-7: Stand-in compositors

**Gap.** GNOME focus has a stand-in shell (`tests/fixtures/gnome_shell_mock.js`). Sway, Hyprland,
niri, wlroots, COSMIC, KWin and Cinnamon have none, so their backends are tested only in the VMs.
The overlay's choice of monitor on wlroots (`overlay/wayland/monitors.rs`) and the GNOME overlay's
`SetBuffer`/`Present` calls have no test outside a VM either.

**What to build.** Small servers in the test crate that speak just enough of each protocol:

- **Sway**: an i3-IPC socket at `$SWAYSOCK` that answers `subscribe` and sends `window` events.
- **Hyprland**: the `.socket.sock` command socket and `.socket2.sock` event socket.
- **KWin**: the D-Bus calls the KWin script makes, on a private session bus (as `gnome_` does).
- **niri**: a `$NIRI_SOCKET` that answers `EventStream` with window and focus events (with pids).
- **Cinnamon**: the extension's D-Bus calls, on a private session bus, as the GNOME mock does.
- **wlroots**: a minimal Wayland server offering `wlr-foreign-toplevel-management` (the
  `wayland-server` crate does this in a few hundred lines). Giving it two outputs also tests which
  one the overlay picks.
- **COSMIC**: the same server with `ext-foreign-toplevel-list` and `cosmic-toplevel-info`
  (`protocols/`) instead.
- **GNOME overlay**: the GNOME mock taking `SetBuffer` and `Present`, so a test can check the
  overlay hands over the right size and redraws only on change.

Each test starts the stand-in, starts the daemon, sends a focus change for a window whose pid is a
stand-in process named like a game, and checks the profile with `padwight status`. This also
drives T-6's focus changes.

**Status.** Built, all in `tests/common/standin/` behind one `Compositor` trait (`focus`, `unfocus`,
`close`, with a `Window` of id, pid, class and title): Sway, Hyprland and niri (socket servers),
KWin and Cinnamon (on a private `dbus-daemon` session bus; Cinnamon has a fake `gsettings` and
plays the extension, owning its bus name and serving the overlay object), wlroots (a
`wayland-server` with two outputs, `wlr-foreign-toplevel-management`, and enough layer shell,
`wl_shm` and compositor to run the overlay process) and COSMIC (`ext-foreign-toplevel-list` plus
`cosmic-toplevel-info`, generated from `tests/fixtures/cosmic-toplevel-info-server.xml`). The GNOME
overlay mock is the Cinnamon stand-in's overlay object (same D-Bus interface), recording
`SetBuffer`, `Present`, `Geometry` and `Hide`.

`tests/compositor_focus.rs` has nine tests. Seven walk a stand-in through game focused before the
daemon starts, desktop, game, nothing, game, close, checking `padwight status` each time (wlroots
and COSMIC carry no pid, so their game rule is a class rule). One checks the overlay's surface is
made on the focused window's output, for each of two outputs. One runs the real overlay process
against the Cinnamon stand-in and checks the picture is the monitor's size at its scale and is set
before the first `Present`. Run:
`cargo nextest run --profile agent --run-ignored ignored-only compositor_` (about two seconds).

They need no display, session bus on the host, or controller (the daemon gets a dead or private bus
address and no compositor variables; `spawn_daemon_env` scrubs `SWAYSOCK`, `NIRI_SOCKET`,
`HYPRLAND_INSTANCE_SIGNATURE`, `WAYLAND_DISPLAY`). They do need `/dev/uinput`, because the daemon
exits at start-up when it cannot create its virtual keyboard and mouse (checked with the device
masked: "Inappropriate ioctl for device"), so they stay `#[ignore]`d like the other daemon tests.
`dbus-daemon` is needed for the KWin and Cinnamon ones. Keep socket paths under 107 bytes (a long
`TMPDIR` breaks them). Shared helpers: `tests/common/focus.rs` (game config, `GameProcess`,
`status`, `cli`, `wait_for_status`).

Left: the overlay does not hide again in the wlroots stand-in after `close-overlays` (it never
asks for the 1x1 corner), so the overlay's monitor follow is tested once per daemon, not as a
window moves; whether that is the stand-in or the overlay is not known. The Cinnamon stand-in's
first-run path (daemon installs and enables the extension through `gsettings set`, the stand-in loads
it, the daemon looks again after its ten-second watchdog) is built but not tested. The Cinnamon
overlay keeps presenting about 16 times a second while the keyboard is open and still, so "redraws
only on change" is not asserted. The GNOME Shell stand-in (the daemon only trusts an
`org.gnome.Shell` owned by a process named `gnome-shell`) is not built.

## T-8: Picture regression

**Gap.** `debug overlay compare` checks the new renderer against iced, not against itself last
week. A change in spacing or color in one scene goes unnoticed until someone looks.

**What to build.** `padwight debug overlay baseline <dir>` writes every scene, glyph sheet and
motion strip; `debug overlay diff <dir>` draws them again and reports the scenes whose difference
passes a threshold, with difference pictures (the compare tool's scoring already does this). The
same for `debug gui` shots of every tab, light and dark. CI keeps the baseline from `main` as a
build artefact, runs the diff on a pull request, and posts the list of changed scenes. A change is
fine when it is meant; the point is to see it. The CPU renderer and the bundled fonts make the
pictures the same on every machine; check that once before trusting it.

## T-9: Fault injection

**Gap.** `output::tests` covers a refused uinput write. Nothing tests a full disk or a crash during
a save, a socket that closes mid-request, or an overlay process that never answers.

**What to build.** A `faults` module compiled only into test and `--debug` builds, read from
`PADWIGHT_FAULTS` (for example `save.rename=fail`, `uinput.write=fail:1/50`, `overlay.frame=hang`).
Each named point is one line at the call site. Tests then check that a failed save leaves the old
setups, that refused writes are sent again (as `output::tests` does), and that a hung overlay
doesn't stall input. For a crash between staging and swap, kill the daemon at the point instead of
failing, and check the folder on restart (SEC-5).

**Status.** Built: `src/faults.rs` (+ `faults/live.rs`). Spec grammar `point=action[+skip][:N/M]`,
comma-separated, `*` suffix matches a prefix; actions `fail`, `hang`, `close`, `crash` (abort). It is
compiled in under `cfg(any(test, debug_assertions))`, so release builds get empty inline functions and
ignore `PADWIGHT_FAULTS`; a debug build reads it only when the daemon starts with `--debug`. Points:
`save.write`, `save.fsync`, `save.set_aside`, `save.remove_old`, `save.rename`, `save.cleanup`,
`save.settings_rename`, `uinput.write`, `ipc.reply`, `overlay.frame`. Run:
`cargo nextest run --profile agent faults:: frames:: serve::`. The save is now flushed with `syncfs`
before the swap (it had no fsync). The crash test re-runs the test binary as a child that aborts at each
stage. Left: points on the gui-to-daemon client side, overlay process hang (only the daemon's stream
is covered), and a fault point per uinput device other than the shared frame writer.

## T-10: GUI tests that check the saved config

**Gap.** Every page builds, and screenshots show it, but no test clicks through an editor and
checks what Save & apply writes. `iced_test` is already a dev-dependency (used in
`gui/stage/handles.rs`).

**What to build.** Tests that drive the settings window by its messages or, with `iced_test`'s
simulator, by clicks and typing on labelled widgets, then call the save path into a temporary
folder and compare the config with the expected one:

- Add a library setup, change a button's action, save: the file has the new action, nothing else changed.
- Each editor (sticks, triggers, gyro, touchpad, combos, layers, modes, macros, cycles, menus,
  info and log overlays): one change each, saved and compared.
- Revert, and undo after a change, restore the config exactly.
- Each Save & apply check (GUI-5) blocks the save it should and only that.

The same approach fits the in-game editor: `layout_editor` rows driven by the input the menu
would get, then compared (EDIT-1, EDIT-2).

**Status.** Built for the settings window: `src/gui/harness.rs` (`Rig`) boots the app on a config
folder of its own under `screenshots/gui-save-tests/`, drives it by `Message`s, runs the real Save
checks (`Message::Save`), writes with `Config::save_to`, diffs the files line by line and reads them
back to compare with what the window holds. Tests are in `src/gui/save_tests.rs` (one change per
editor: buttons, library setup, sticks, triggers, gyro, touchpad, combos, layers, modes, macros,
cycles, menus, info, logs; Revert and Undo restoring the files exactly) and `save_tests/blocks.rs`
(ten Save & apply checks, each refusing with its own words, writing nothing, flagging its tab, and
passing once fixed). Run: `cargo nextest run --profile agent save_tests`. Left: the in-game editor
(`layout_editor` rows, EDIT-1/EDIT-2), the remaining checks (zones, two flick sticks, layer
indicator, duplicate profile names), and iced_test clicks on labelled widgets.

## T-11: GUI layout checks

**Gap.** Clipped text, wrapped labels and low contrast are found by eye.

**What to build.** A pass over the `debug gui` shots of every tab, light and dark, at the minimum
window size: for each text widget, check its laid-out text fits its bounds (from iced's layout
tree, not the pixels), and check the contrast of text against its background meets WCAG AA. The
color-blind menu colors get the same contrast check, plus a simulated deuteranopia pass.

## T-12: Scheduled runs and performance baselines

**Gap.** Layers L3 and L5 to L8 run by hand. `scripts/profile.sh` writes numbers but nothing
compares them with the last run.

**What to build.**

- A nightly CI workflow on `main`: the container suite (GitHub's Ubuntu runners can
  `modprobe uinput uhid` and run Docker), the fuzz targets for a minute each, T-8's diff, the
  scenario tests, and `scripts/desktop-versions.sh`.
- `scripts/profile-compare.py old/ new/`: reads two `PADWIGHT_PROFILE_OUT` folders and flags each
  measurement that got worse by more than its noise (from the repeat runs it already makes), say a
  latency median 20% up or idle CPU over 1% of a core. Run it in the weekly run against the last
  week's results, kept on the same machine; shared CI runners are too noisy for latency.
- The weekly desktop VM run as one script that runs `desktop-test.sh` on every VM and updates the
  table in [qa-plan.md section 8](qa-plan.md#8-desktop-session-runs).

**Status.** Built, except the weekly profile run itself, which stays manual on a fixed machine.
`.github/workflows/nightly.yml` (on `main`, 02:17 UTC and by hand) runs the container suite as one
job per filter (`scripts/kernel-test-docker.sh`, after `modprobe uinput uhid`),
`scripts/desktop-versions.sh`, and coverage (T-19). It also holds jobs for fuzz targets (T-5,
guarded on `fuzz/Cargo.toml`), scenario tests (T-1, guarded on `tests/scenarios/`) and the overlay
picture diff (T-8, guarded on the CLI having `baseline` and `diff`); each does nothing until its tool
is merged, and carries a TODO to check its command when it is. None of the workflow has run on
GitHub yet; the YAML parses. `python3 -I scripts/profile-compare.py old/ new/` compares two
`PADWIGHT_PROFILE_OUT` folders (limits and noise rules in its docstring; exit 1 when something got
worse, 2 when the machines differ) and was tried on synthetic folders only.
`scripts/desktop-weekly.sh [--dry-run] [--only names]` runs `desktop-test.sh` on every VM in `vms/`
(starting and stopping the ones that are off) and has `scripts/update-desktop-table.py` write the
result, date and kernel into the table of section 8; the dry run was checked, a real run was not.
Left: a weekly cron on the profiling machine that runs `scripts/profile.sh` into a dated folder and
then `profile-compare.py` against the last one; the Version column of the desktop table is still
filled in by hand.

## T-13: Pack identity audit and status table

**Gap.** Several pack families were written with unverified Steam IDs and executable names. The
facts are in `pcgamingwiki/` offline.

**What to build.** `scripts/pack-audit.py`: for each pack, look up its game in
`pcgamingwiki/games.tsv`, `executables.tsv` and the Discord detectable list, and report Steam
app IDs not listed for the game, executable names not seen for it, and a release date or native
controller support that disagrees with the pack's notes. It writes `pack-qa.tsv`: one row per pack
with the audit result and columns for the manual checks of [qa-plan.md section 6](qa-plan.md#6-pack-qa)
(bindings checked, last smoke, last full play), kept between runs. Never read `games.jsonl`
whole; use the TSV files and the article lookup its README describes.

**Status.** Built: `scripts/pack-audit.py` (Python standard library only). `python3 -I scripts/pack-audit.py`
audits every pack against the snapshot and rewrites `docs/development/pack-qa.tsv`; `-v` prints each finding,
`--check` fails if the table is stale, `--self-test` checks the matching logic on a tiny in-memory snapshot.
Matching is by pack name (title or redirect), then by Steam ids; several name matches or none are reported
(`ambiguous`, `no_match`), never picked. Findings: `id_wrong_game`, `id_other_game`, `id_unknown`,
`exe_unseen`, `exe_no_data`, `year_mismatch`, `passthrough_pre2006`, `passthrough_no_support`,
`mapped_but_native`, plus info kinds. The table is tracked next to the plan so the manual columns
(`bindings_checked`, `last_smoke`, `last_full_play`, `manual_notes`) are versioned and reviewed; reruns keep
them. Left: the snapshot is untracked, so the audit columns only refresh where it exists; the executable list
comes only from Discord's list (no second source); the controller check is a heuristic on the wiki's
`controller_support`/`xinput` fields and the pack's shape, not its prose.

## T-14: Binding comparison

**Gap.** Whether a pack's keys match the game's own controls is checked by a person reading the
game's settings.

**What to build.** Readers for the default bindings of the engines the library leans on, each
turning a game's file into `action → key`:

| Engine | File | Packs |
|---|---|---|
| Unreal 1 and 2 | `System/User.ini`, `DefUser.ini` | Deus Ex, Unreal, … |
| Dark engine | `*.bnd` | Thief, System Shock 2 |
| id Tech and GoldSrc/Source | `config.cfg`, `kb_def.lst` | Doom, Quake, Half-Life, … |
| Unity games with readable settings | per game | Timberborn, … |

A pack names a key with a label ("Use", "Lean left"); the comparison maps labels to the engine's
action names by a small table per game, and reports keys that don't match. Start with the
files already read by hand for the format 19 pack update: Deus Ex's `User.ini`, the Dark engine
`.bnd` files, F.E.A.R.'s profile and Max Payne's registry keys in its Proton prefix.

**Status.** Built: `scripts/binding-compare.py` with readers for Unreal 1 `User.ini`/`DefUser.ini`, Dark engine
`.bnd`, id Tech/GoldSrc/Source `config.cfg` and `kb_def.lst`, F.E.A.R.'s `Profile000.gdb` and Max Payne 1/2
registry scancodes (Wine `user.reg`). The per-game tables are in `scripts/binding-compare.toml` (pack input path or
Guide label to engine action names; Deus Ex, Thief Gold, Thief 2, System Shock 2, F.E.A.R., Max Payne 1 and 2,
VKQuake). `python3 -I scripts/binding-compare.py list` shows which games are installed here, `compare [game]`
reports mismatches (`--all` shows agreeing rows, `--file path` points at a file for a game that isn't installed),
`dump-game` and `dump-pack` print what each side reads, `self-test` runs every reader on the fixtures in
`tests/fixtures/bindings/` plus a pack comparison. Left: Unity games (Timberborn, Parkitect), Unreal 2 and
Lithtech games beyond F.E.A.R., id Tech 4 (`Doom 3`), Half-Life/Source packs (the installed HL2 and Portal packs are
gamepad passthrough, so there are no keys to compare), the Max Payne 1 action roles are inferred from default keys,
sticks other than the four cardinal ring sectors, and layers and modes.

## T-15: Protocol conformance

**Gap.** `ipc/tests.rs` pins the wire format from Rust. The examples in `docs/protocol.md` aren't
run, and no client in another language is tested.

**What to build.** A test that pulls each request and reply example out of `docs/protocol.md`,
parses it with the Rust types, and checks it serializes back the same; and an integration test that
runs `examples/dashboard/server.py`'s socket client against a `--debug` daemon and reads `Status`
and a few `WatchInput` lines. `check-project.py` can run the first.

**Status.** Built. `cargo nextest run --profile agent ipc::tests::protocol_doc` runs the document
test (`src/ipc/tests/protocol_doc.rs`): every `json request`, `json reply` and `json <Type>` fenced
block, and every JSON code span in the Requests and JSON shapes sections, must parse with the real
types and serialize back to the same value. Every `Request` and `Response` variant must have an
example (the `Debug*` requests, `Attached`, `Lines`, and the whole-configuration `SetConfig` and
`Config` only need to be shown), and every field of the reply structs must be named in the
document, so a new message or field can't go undocumented. `tests/dashboard_client.rs` starts a
`--debug` daemon on throwaway folders and runs `tests/dashboard_client/check.py`, which drives
`examples/dashboard/server.py`'s socket client and HTTP bridge (Version, Status, GetConfig, error
replies, a `WatchInput` line, `/api/status`, `/api/input`, the Host check). It needs neither
/dev/input nor /dev/uinput, so it runs with the ordinary tests; with no pad attached `WatchInput`
sends only `null`. `scripts/check-project.py` checks the doc's version against `src/ipc.rs` and
that the marked examples are JSON, but not the Rust parsing (it can't run the types). Left: a pad
attached through `debug --attach` (needs uinput) to read real snapshots, `SetConfig` examples, and
clients in other languages.

## T-17: Bug report bundle

**Gap.** Manual QA runs and user reports both need the same facts ([qa-plan.md section 5.0](qa-plan.md#50-recording-a-run)),
gathered by hand.

**What to build.** `padwight doctor --report <file>`: version and commit, OS, kernel, desktop and
version, focus backend, the doctor checks, the controllers with their IDs, drivers and nodes, who
else reads their hidraw nodes, the last thousand lines of the daemon log, and the active setup's
files with rule values kept and nothing else personal. One file to attach to an issue.

**Status.** Built: `padwight doctor --report <file>` (`src/doctor/report.rs`, `system.rs`, `scrub.rs`).
Each section is gathered on its own and a failing one says so; with no daemon the report says that
and reads the rest from the system. `scrub.rs` (unit-tested) turns the home folder into `~`, other
home folders and the user name into `<user>`, the host name into `<host>`, MAC and Bluetooth
addresses, UUIDs, machine ids, IPv4 addresses, e-mail addresses and Steam ids into placeholders,
replaces every serial or `uniq` string the machine reports, redacts values of keys such as
`nickname`, and hides quoted text on log lines that mention a window title. The focused window and
recent windows are left out. Not collected: display scale and refresh rate, controller firmware,
keyboard layout. `build.rs` now embeds the short commit as `PADWIGHT_COMMIT`.

## T-18: Record and replay real input

**Gap.** The code review plan's missing tool: a bug seen on a real controller can't be turned into
a test without that controller. `WatchInput` merges bursts to 60 lines a second, so it can't serve
as a recording.

**What to build.** `padwight debug record <file>`: reads a controller's evdev nodes (gamepad,
motion, touchpad) without grabbing them, and writes every event with its time, plus the device's
name, IDs and capabilities. `padwight debug replay <file>` makes a uhid or uinput device with the
same capabilities and plays the events at their times (or on T-2's clock, exactly). A recording
from a player's DualSense then runs in the container against the daemon, and with T-1 its output
becomes a test. Record the formats `evemu-record` writes too, so existing recordings can be used.

**Status.** Built (untested with a real controller; none was at hand). `padwight debug record <file>
[--device <node|name>] [--seconds <s>]` reads the gamepad node and the motion-sensor and touchpad nodes
the daemon would pair with it (`daemon::same_controller`), without grabbing, and writes a text file
(format in `src/debug/recording/format.rs`: device lines with IDs and capabilities including absinfo,
then `time device type code value` lines with the code name as a comment). `padwight debug replay <file>
[--uhid] [--speed x] [--lead-in ms] [--hold ms] [--uniq id]` makes uinput devices with the recorded
capabilities, or with `--uhid` a HID gamepad built from them (buttons, six axes, GAS/BRAKE, one D-pad
hat), which is what the daemon grabs; it reads `evemu-record` files too. Use a lead-in of 3 s or more
for a daemon to grab the pad before the first frame. `replay::play` takes a `Clock` and a `Sink`, the
seam for T-2's fake clock. Test: `scripts/kernel-test-docker.sh record_replay` (records a uinput pad,
replays it as uhid, checks a daemon types the mapped key); unit tests cover the file format, the evemu
parser, the scheduler and the HID report. Left: motion sensors and touchpads replay through uinput only
(a daemon ignores those), uinput can't set the unique ID or force feedback, back paddles have no HID
usage here, and no `--loop` or fake-clock mode yet.

## T-19: Coverage and mutation testing

**Gap.** Test counts per module ([qa-plan.md section 3](qa-plan.md#3-coverage-by-feature-area))
are a rough guide. Which lines no test runs, and which tests pass whatever the code does, isn't
known.

**What to build.** `cargo llvm-cov nextest` in the nightly run, with the container suite merged in
(both produce profiles; `llvm-cov` merges them), publishing the HTML report as an artefact. No
percentage gate: read it for T0 and T1 code no test reaches. Then `cargo mutants` one area at a time
in the weekly run (start with `engine/`, `menu/`, `gui/checks.rs` and `config/store.rs`), fixing
each surviving mutant with a test or a reason it can't matter.

**Status.** Built. The nightly workflow's `coverage` job runs `cargo llvm-cov nextest --no-report`
for the CI tests, then again as root with `--run-ignored ignored-only` for the container suite
(minus the profiling tests), and `cargo llvm-cov report --html` merges the profiles; the HTML and
`lcov.info` are the `coverage-html` artefact and a summary goes on the run page. No gate. Locally:
`cargo llvm-cov nextest --no-report` then `cargo llvm-cov report --html`. Mutation testing:
`scripts/mutants.sh engine|menu|gui-checks|config-store` (settings in `.cargo/mutants.toml`), and
`.github/workflows/weekly.yml` runs the four areas as a matrix on Sundays, uploading
`mutants-<area>` artefacts with `missed.txt`. Left: working through the survivors, and checking the
privileged coverage step on a real runner.
