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
| 13 | [T-7](#t-7-stand-in-compositors) Stand-in compositors | M | Focus backends (Sway, Hyprland, wlroots, KWin) in CI | T1 |
| 14 | [T-19](#t-19-coverage-and-mutation-testing) Coverage and mutation testing | S | Code no test runs; tests that pass whatever the code does | all |
| 15 | [T-14](#t-14-binding-comparison) Binding comparison | M per engine | Pack keys that don't match the game's controls | T1 |
| 16 | [T-17](#t-17-bug-report-bundle) Bug report bundle | S | Missing facts in manual QA runs and user reports | — |
| 17 | [T-15](#t-15-protocol-conformance) Protocol conformance | S | `docs/protocol.md` drifting from the daemon | T3 |
| 18 | [T-11](#t-11-gui-layout-checks) GUI layout checks | M | Clipped and overflowing text, dark-theme contrast | T4 |
| 19 | [T-16](#t-16-windows-integration) Windows integration | L | Every Windows-only path, now unit-tested only | T0, T1 |

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

## T-7: Stand-in compositors

**Gap.** GNOME focus has a stand-in shell (`tests/fixtures/gnome_shell_mock.js`). Sway, Hyprland,
wlroots and KWin have none, so their backends are tested only in the VMs.

**What to build.** Small servers in the test crate that speak just enough of each protocol:

- **Sway**: an i3-IPC socket at `$SWAYSOCK` that answers `subscribe` and sends `window` events.
- **Hyprland**: the `.socket.sock` command socket and `.socket2.sock` event socket.
- **KWin**: the D-Bus calls the KWin script makes, on a private session bus (as `gnome_` does).
- **wlroots**: a minimal Wayland server offering `wlr-foreign-toplevel-management` (the
  `wayland-server` crate does this in a few hundred lines).

Each test starts the stand-in, starts the daemon, sends a focus change for a window whose pid is a
stand-in process named like a game, and checks the profile with `padwight status`. This also
drives T-6's focus changes.

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

## T-15: Protocol conformance

**Gap.** `ipc/tests.rs` pins the wire format from Rust. The examples in `docs/protocol.md` aren't
run, and no client in another language is tested.

**What to build.** A test that pulls each request and reply example out of `docs/protocol.md`,
parses it with the Rust types, and checks it serializes back the same; and an integration test that
runs `examples/dashboard/server.py`'s socket client against a `--debug` daemon and reads `Status`
and a few `WatchInput` lines. `check-project.py` can run the first.

## T-16: Windows integration

**Gap.** On Windows, CI builds and runs the unit tests. Every system path (XInput, ViGEmBus,
`SendInput`, the layered overlay window, the WinEvent focus hook, the tray, GSMTC media, GDI
capture, ffmpeg recording, Force Quit) is unconfirmed by any test.

**What to build.** A Windows VM image in `vms/` with ViGEmBus installed, and a test that uses
ViGEmBus to make a fake XInput pad (the role uhid plays on Linux), starts the daemon, presses
buttons and reads what `SendInput` produced through a low-level keyboard hook. The `debug_session`
tests already run on Windows; with this, the daemon tests can too. GitHub's Windows runners allow
installing ViGEmBus, so it may run in CI as well. Then WIN-2 to WIN-6 one by one.

## T-17: Bug report bundle

**Gap.** Manual QA runs and user reports both need the same facts ([qa-plan.md section 5.0](qa-plan.md#50-recording-a-run)),
gathered by hand.

**What to build.** `padwight doctor --report <file>`: version and commit, OS, kernel, desktop and
version, focus backend, the doctor checks, the controllers with their IDs, drivers and nodes, who
else reads their hidraw nodes, the last thousand lines of the daemon log, and the active setup's
files with rule values kept and nothing else personal. One file to attach to an issue.

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

## T-19: Coverage and mutation testing

**Gap.** Test counts per module ([qa-plan.md section 3](qa-plan.md#3-coverage-by-feature-area))
are a rough guide. Which lines no test runs, and which tests pass whatever the code does, isn't
known.

**What to build.** `cargo llvm-cov nextest` in the nightly run, with the container suite merged in
(both produce profiles; `llvm-cov` merges them), publishing the HTML report as an artefact. No
percentage gate: read it for T0 and T1 code no test reaches. Then `cargo mutants` one area at a time
in the weekly run (start with `engine/`, `menu/`, `gui/checks.rs` and `config/store.rs`), fixing
each surviving mutant with a test or a reason it can't matter.
