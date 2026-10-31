# QA plan

How Padwight is tested: what can go wrong and what it costs, which layer of testing catches each
kind of fault, what runs on every pull request and what before a release, and the checks that still
need a person, a controller or a game. The tools this plan needs but that don't exist yet are in
[qa-tooling.md](qa-tooling.md); each gap below names the tool that closes it (`T-n`).

Fix-by-fix findings live in [code-review-plan.md](code-review-plan.md). Its hardware checks are
written `CR H-n` here, to keep them apart from this plan's own IDs.

## 1. Risk tiers

A bug is worth more testing the more it costs the player. Every feature area in
[section 3](#3-coverage-by-feature-area) is ranked by the worst tier it can reach.

| Tier | What goes wrong | Examples | Evidence needed before a release |
|---|---|---|---|
| **T0: device safety and data** | The player loses control of the machine or loses work | A key or mouse button stays held; a controller stays grabbed after the daemon dies; a game sees both the real pad and the virtual one; Force Quit ends the wrong process; a save destroys setups | Automated test at the kernel level (container), plus a hardware check for each controller family |
| **T1: wrong output** | The game gets the wrong input, or the right input at the wrong time | A mapping, gesture, combo, layer or mode does the wrong thing; the wrong profile is active; a macro runs on after a profile switch | Automated end-to-end test through the daemon (debug session or container) |
| **T2: overlays steal or stall** | An overlay gets in the way of play | A menu won't close; the overlay takes keyboard focus; input still reaches the game while a menu is up; flicker; CPU while idle | Off-screen render and harness checks, plus a run on each compositor |
| **T3: editing** | The editors save something other than what the player saw | The GUI or Edit Controls writes a different config; Save & apply's checks miss a broken reference | GUI message tests and round-trip tests |
| **T4: presentation** | It works, but reads or sounds wrong | Overlapping labels, wrong glyph family, unclear copy, a cue too loud | Screenshots reviewed by a person; sound audit |

## 2. Test layers

What exists today, from fastest to slowest. Counts are as of 2026-10-31.

| # | Layer | What it proves | Where it runs | How |
|---|---|---|---|---|
| L1 | Unit tests (1,465 `#[test]`) | Logic of single modules: engine, config, packs, menus, renderer | CI | `cargo nextest run --profile agent …` |
| L2 | Sweeps over many inputs | No stuck output after random input; damaged packs and configs are refused without a panic; ring sectors leave no gaps; every library pack round-trips | CI | `engine::release_tests`, `library::tests::damaged_packs_…`, `config::tests::damaged_configs_…` |
| L3 | Off-screen pictures | What the overlay, the settings window and the controller drawings look like, with no display | Locally, by hand | `debug overlay`, `debug overlay compare`, `debug motion`, `debug gui` |
| L4 | Debug sessions | Mappings end to end through a real daemon, with a virtual controller and recorded output | CI (session tests); `--ignored` daemon tests need `/dev/uinput` | `tests/debug_session/`, `padwight debug --attach` |
| L5 | Kernel tests in a container | Real uinput and uhid: release on exit, hot-plug, idle CPU, process-scan switching, the GUI under Xvfb | Docker, by hand | `scripts/kernel-test-docker.sh <filter>` |
| L6 | Headless compositor harness | The overlay process on a real Wayland compositor (KWin): frame pacing, buffers, scale, CPU and memory | Locally, by hand | `scripts/overlay-harness.sh scripts/harness/<scenario>` |
| L7 | Desktop VMs | Focus tracking, rules and the overlay process on GNOME, Cinnamon, Budgie, COSMIC, Plasma, Sway, Hyprland, niri and labwc | QEMU VMs in `vms/`, by hand | `scripts/desktop-test.sh <desktop>` |
| L7b | GNOME Shell versions | The GNOME extension's overlay shows a shared picture and hides it, on each Shell from 45 to 51, headless | Docker, by hand | `scripts/gnome-versions/run.sh` |
| L8 | Profiling | Latency, CPU, memory, start-up; not pass/fail | Docker, plus host-only parts | `scripts/profile.sh` |
| L9 | Hardware checks | Real controllers, Bluetooth, real compositors and screens, feel | A person with a controller | [Section 5](#5-manual-and-hardware-checks) |
| L10 | Playtests | A pack's bindings work in the real game | A person with the game | [Section 6](#6-pack-qa) |

Layers L3 and L5 to L8 run only when someone remembers to run them. The biggest single
improvement to this plan is to run them on a schedule ([section 4](#4-when-each-layer-runs), T-12).

## 3. Coverage by feature area

"Unit density" is `#[test]` count per thousand lines of the area's code, as a rough sign of where
tests are thin (the engine is at 27; anything under 10 is worth a look). It counts only L1; areas
covered mostly by integration tests (the daemon) score low here for that reason.

| Area | Tier | Unit density | Covered now | Gap | Closes it |
|---|---|---|---|---|---|
| Device ownership: grab, release, hot-plug, exit (`daemon/`, `input/`, `output/`) | T0 | daemon 3, output 8, input 12 | Release on SIGTERM/SIGINT/SIGKILL, unplug and replug, a different pad in the same slot, nothing stays pressed in the kernel (L5) | Suspend and resume; Bluetooth drop and reconnect; two pads at once through uhid; gyro and touchpad nodes from real kernel drivers; rare write errors; bugs seen only on a player's hardware | T-3, T-6, T-9, T-18 |
| Steam first pad (`daemon/first_pad.rs`) | T0 | — | Unit tests | Steam started with a pad already connected; daemon restarted with a pad connected | HW-9 |
| Other readers of the raw device (`daemon/raw_readers.rs`) | T0 | — | Unit tests; GUI banner shot | A real Steam or browser holding `/dev/hidraw*` | HW-8 |
| Force Quit (`quit/`) | T0 | 26 | Process-tree picking; container test | Launcher-started games, Proton, games in Flatpak sandboxes | FOC-9 |
| Config store (`config/store.rs`) | T0 | config 14 | Round trip, staging folder, `0600` files, unsafe names, removal | Disk full or read-only mid-save; a crash between staging and swap | T-9 |
| Engine: buttons, gestures, combos, toggles, turbo (`engine/`) | T1 | 27 | Dense unit tests on an injected clock; random release sweep | Combinations across features (a gesture inside a layer inside a mode while a cycle steps); timing at the daemon's real tick | T-1, T-2, T-5 |
| Sticks, flick stick, rings, zones (`engine/stick*`, `engine/flick.rs`) | T1 | 27 | Units; ring coverage sweep | Feel (FEEL-1 to FEEL-3) | T-1 for regressions |
| Gyro (`engine/gyro*`, `motion/`) | T1 | 22 | Units; debug `gyro` command | Real sensor noise and drift; the motion node arriving after the pad | T-3, HW-4 |
| Touchpad (`engine/touchpad/`, `input/linux/touchpad.rs`) | T1 | — | Units; debug `touch`/`swipe` | Never run on a real DualShock 4 or DualSense; evdev contact slots from the real driver | T-3, HW-5 |
| Modes and layers | T1 | — | Units; layer docs | Cross-feature sequences | T-1, T-2 |
| Macros and cycles | T1 | — | Units (`cycle_tests.rs`) | Cycles never tried on hardware; macros against games that drop fast input | FEEL-4, G-3 |
| Automatic switching and focus (`focus/`, `daemon/auto_switch.rs`) | T1 | 20 | Process scan (L5), GNOME against a mock shell, desktop VMs (L7); Cinnamon and niri parsing units | Sway, Hyprland, niri, COSMIC, Cinnamon and wlroots have no CI test (`cosmic.rs` has no unit test either); Flatpak Heroic; Proton `.exe` names in real launches | T-7, FOC-1 to FOC-10 |
| Menus: radial, directional, list, button, carousel, grid (`menu/`, `radial.rs`) | T2 | 27 | Units; off-screen scenes | Assertions on what the live overlay shows (today only pictures) | T-4 |
| Guide overlay and movable shortcuts (`guide_menu.rs`, `config/shortcuts/`) | T2 | — | Units; scenes | The opener on a gesture or combo with real timing | T-1, OVL-6 |
| On-screen keyboard, numpad, Paste | T2 | keyboard 13 | Units; scenes | Over a fullscreen game on each compositor | OVL-1, OVL-2 |
| Tools: calculator, doodle, clipboard (`tools/`, `clipboard.rs`) | T2 | 27 | Units; scenes | Never run on hardware; clipboard on each desktop (`wl-copy`, `xclip`, `xsel`) | OVL-9 |
| Info and log overlays, tokens (`info/`, `inputlog/`) | T4 | 15, 17 | Units; scenes; library token check | Live values on real hardware (battery, wifi, GPU) | OVL-7 |
| Overlay process and renderer (`overlay/render`, `overlay/wayland`) | T2 | overlay 14 | Units; compare with iced; harness pacing, surface, scale, CPU | Never on a real screen (renderer plan step 8.3); ABGR8888 buffers; integer buffer scale; following the focused window's monitor (`wayland/monitors.rs`) has no test | T-8, OVL-3 to OVL-5, OVL-12 |
| GNOME and Cinnamon overlay through the Shell extension (`overlay/gnome.rs`, `gnome-extension/`, `cinnamon-extension/`) | T2 | — | Shell 45 to 51 headless (L7b); GNOME 51 and Cinnamon 6.6 in VMs, by hand | No unit tests; never with a real pad; `Present` blocks Shell's main loop, so a slow frame stalls the desktop | OVL-13 |
| Sounds and rumble cues (`sound/`, `rumble.rs`) | T4 | 14, 6 | Sound trace; loudness audit | Library packs' synthesized cues never listened to; rumble cues on real motors | SND-1 to SND-4 |
| Settings window (`gui/`) | T3 | 8 | Every page builds; tab shots; GUI under Xvfb (L5); one `iced_test` simulator test | Nothing clicks through the window and checks the saved config; dark theme; minimum size | T-10, T-11 |
| On-screen stage editor (`gui/stage/`) | T3 | — | Shots; handle drag in the simulator | Never used with a real mouse; focus and scroll after a pick run only live | GUI-8 |
| In-game editor: Quick Settings, Edit Controls (`system_menu/`, `layout_editor/`) | T3 | 18, 7 | Units; scenes | Typing, live pictures and Save/Discard on hardware; layout editor is thin on tests | T-1, EDIT-1 to EDIT-6 |
| Packs: parse, check, import, export, update (`pack.rs`) | T3 | 18 | Round trips, damaged input, clashes, forks, assets | Coverage-guided fuzzing of parse and of SVG and WAV assets | T-5 |
| Library: 232 packs (`packs/`) | T1 | — | Every pack valid, round-trips, known tokens only, built-in assets only, no shared executables | IDs and executable names unverified for several families; bindings unverified against the games | T-13, T-14, [section 6](#6-pack-qa) |
| Control protocol 1.0 (`ipc.rs`, `docs/protocol.md`) | T3 | 11 | `ipc/tests.rs` pins the wire format | The doc's examples aren't executed; no client outside Rust is tested | T-15 |
| CLI, doctor, tray (`main.rs`, `doctor/`, `tray/`) | T3 | 17, 8 | Docs name real commands (`check-project.py`); doctor units | Doctor against real broken setups; tray Quit in a session | INS-8, INS-10 |
| Screenshots, recording, media (`capture/`, `record/`, `media/`) | T2 | 17, 15, 11 | Units; `--debug` daemon writes to a temp folder | Each desktop's tool; a stuck MPRIS player | INS-11 |
| Input priority and latency (`priority.rs`, `daemon/input_timing.rs`) | T1 | 6 | Profiling suite | No regression gate | T-12 |
| Install, systemd, udev, the desktop extensions, tarball | T0 | — | `check-install.sh` validates the files in CI | A clean install on each distro family; uninstall | INS-1 to INS-7 |

## 4. When each layer runs

### On every pull request (CI, today)

`.github/workflows/ci.yml`: check, clippy with warnings denied, the tests,
`check-project.py`, `check-install.sh` and `cargo audit`, filtered by the paths a change touches.

Worth adding to CI (see [qa-tooling.md](qa-tooling.md)): the scenario tests (T-1), a short fuzz run
(T-5), the overlay picture regression (T-8), and the container suite where the runner allows
`/dev/uinput` and `/dev/uhid` (GitHub's Ubuntu runners do, with `sudo modprobe uinput uhid`).

### Before merging a change to a risky area

The author runs these by hand until they are in CI. The pull request description says which ran.

| Change touches | Run |
|---|---|
| `daemon/`, `input/`, `output/`, `rumble/` | `scripts/kernel-test-docker.sh` with `nothing_stays`, `daemon_`, `hotplug_`, `idle_`, `debug_` |
| `engine/`, `menu/`, `system_menu/`, `layout_editor/` | The module's tests, then `debug_` in the container |
| `focus/`, `launchers.rs` | `process_` in the container; `scripts/desktop-test.sh` on the backend touched; `debug focus` on a real game |
| `gnome-extension/`, `cinnamon-extension/`, `overlay/gnome.rs` | `scripts/gnome-versions/run.sh` (every Shell the extension claims); `scripts/desktop-test.sh gnome` or `cinnamon`; OVL-13 by hand |
| `overlay/` | `debug overlay all glyphs`, `debug overlay compare all`, `debug motion all`, and the harness's `pacing.sh` and `resources.sh` |
| `sound/`, an overlay's cues | `scripts/sound-trace.sh`, `scripts/sound-audit.py` |
| `gui/` | `debug gui` shots of the tabs touched, light and `--dark`, looked at; `gui_` in the container |
| `config/`, `pack.rs` | `config::`, `pack::`, `library::` tests |
| `packs/` | `library::` tests, `debug overlay packs/<name>.padpack`, and [section 6](#6-pack-qa)'s checks for the pack |
| `dist/`, `scripts/package.sh` | `check-install.sh`, and INS-1 or INS-4 on a VM |

### Weekly, and after a large merge

1. The whole container suite: `scripts/kernel-test-docker.sh` with no filter.
2. `scripts/profile.sh`, compared with the last run on the same machine (T-12 makes the comparison).
3. `scripts/desktop-versions.sh`, then `scripts/desktop-test.sh` on every desktop.
   Update the table in [section 8](#8-desktop-session-runs).
4. The soak run (T-6) overnight.
5. `cargo mutants` on one area in turn (T-19), fixing tests it shows to be weak.

### Release candidate

Everything weekly, plus:

1. The hardware matrix ([section 5.1](#51-controller-matrix)) on every controller at hand, and
   every `HW-`, `OVL-`, `EDIT-` and `INS-` check, or a note of why one was skipped.
2. The pack playtest tier for this release ([section 6](#6-pack-qa)).
3. Install from the release tarball on a clean VM of the oldest supported distro (INS-1), and on
   an immutable distro (INS-4).
4. The security review ([section 7](#7-security-review)) with a second reviewer.
5. No open T0 or T1 bug. Open T2 to T4 bugs are listed in the release notes.

## 5. Manual and hardware checks

### 5.0 Recording a run

Each run records: date, commit, OS and kernel, desktop and its version, display scale and refresh
rate, controller model, connection (USB or Bluetooth) and firmware, whether Steam was running, and
for each check: pass, fail (with the issue number) or skipped (with why). Attach `padwight doctor`
output and `~/.local/state/padwight/daemon.log`. A bug report bundle that collects all of this in one
command is T-17.

### 5.1 Controller matrix

Fill a cell with the date of the last pass. A cell that doesn't apply is "—".

| Controller | Connection | Buttons, sticks, triggers | Guide | Glyphs | Gyro | Touchpad | Paddles | Rumble | Battery token | Hot-plug |
|---|---|---|---|---|---|---|---|---|---|---|
| Xbox Series / One | USB | | | | — | — | | | | |
| Xbox Series / One | Bluetooth | | | | — | — | | | | |
| Xbox Elite 2 | USB / BT | | | | — | — | | | | |
| DualSense | USB / BT | | | | | | — | | | |
| DualSense Edge | USB | | | | | | | | | |
| DualShock 4 | USB / BT | | | | | | — | | | |
| Switch Pro | USB / BT | | | | | — | — | | | |
| Joy-Con pair | BT | | | | | — | — | | | |
| 8BitDo Ultimate (Switch mode) | BT | | | | | — | | | | |
| A generic pad (GameSir or similar) | USB | | | | — | — | — | | | |
| Steam Deck | built in | | | | | | | | | |
| No Guide button (opener on Start + Select) | any | | | | — | — | — | — | — | — |

### 5.2 Device lifecycle

| # | Steps | Expected |
|---|---|---|
| HW-1 | On each controller in the matrix, run General's Gamepad passthrough, then a remapped profile. | Every button, stick and trigger gives the right output. Sticks and triggers rest at center. |
| HW-2 | Unplug while a mapped key is held; plug back in. | The key comes up at once; the pad is managed again and works. (Automated with uhid: `hotplug_`.) |
| HW-3 | Two controllers at once, each with its own setup (see [per-controller-setups.md](per-controller-setups.md)). Unplug the first. | Each pad follows its own mappings; the second keeps working; nothing is held. |
| HW-4 | A PlayStation or Switch pad, first without the motion udev rule, then with it. | Without it, the app says gyro is unavailable; with it, gyro works. The profile doesn't flicker when the motion node appears after the pad. |
| HW-5 | DualSense or DualShock 4 touchpad in each mode: mouse (tap, two-finger scroll, glide), stick, swipes, menu zones, split pad. Also CR H-5 and CR H-16. | Each behaves as the Touchpad tab says. Tap and swipe times feel right. |
| HW-6 | Test rumble on a pad with motors and one without; play a game that rumbles. | Rumble works, or the app says it isn't supported. |
| HW-7 | Leave a pad idle for several hours, then play. Also suspend and resume, and let a Bluetooth pad sleep and wake. | No drift, no lost input, no CPU while idle; the pad comes back after resume and wake. |
| HW-8 | With Steam running and a PlayStation or Switch pad, with and without `dist/72-padwight-hidraw.rules`. | Without the rule, the GUI and a toast warn that Steam reads the pad past Padwight. With it, no warning and no double input. |
| HW-9 | Slay the Spire 2 (reads only the first Steam controller): start Steam with the pad already connected; also restart the daemon with the pad connected. | The game reads Padwight's pad in both cases. |
| HW-10 | `padwight disable; padwight enable` in one line, on each family. | The pad is taken back at once; a touchpad still drives the profile (CR H-16). |
| HW-11 | `systemctl --user restart padwight` mid-macro, mid-layer and with a menu open; then `kill -9` the daemon. | It comes back, releases everything, and doesn't grab twice. After `kill -9` the pad and the virtual devices are free. |

### 5.3 Desktops and focus

| # | Steps | Expected |
|---|---|---|
| FOC-1 | On KDE Plasma, a game with a rule: launch, Alt+Tab to the desktop and back. | The game's profile in the game, the default outside it. |
| FOC-2 | FOC-1 on GNOME, Cinnamon, Budgie, COSMIC, Sway, Hyprland, niri, labwc and Wayfire. | Same. On wlroots compositors, Budgie and COSMIC only class rules match; niri matches executables too. |
| FOC-3 | On Budgie (and Ubuntu Budgie, whose desktop name may be `Budgie:GNOME`), start the daemon. | The wlroots tracker is used, not GNOME's; no GNOME extension is installed; no `overlay:` errors in the log. |
| FOC-4 | Games started from Steam (Proton), Heroic (native and Flatpak, CR H-4), Lutris and a plain binary. | The library finds them and `debug focus` names the right executable or `.exe`. |
| FOC-5 | Two setups with rules for the same window. | The import preview shows the clash; the imported one takes over only if chosen. |
| FOC-6 | On labwc or Wayfire, restart the daemon with a ruled game in front, then switch between two windows (CR H-6). | The game's profile at once; no `no window has focus` between windows. |
| FOC-7 | On GNOME, log out and in once, then restart the daemon with a ruled game in front (CR H-7). | The game's profile at once. |
| FOC-8 | On Sway, from a ruled game, go to an empty workspace and back. | Default on the empty one, the game's on return. |
| FOC-9 | Without focus tracking, hold Force Quit over a ruled game, then again just after it closed (CR H-17). | The first names and ends the game; the second says there is nothing to force quit. |
| FOC-10 | On Cinnamon, a fresh install: start the daemon, then log out and in. | The daemon installs and enables its Cinnamon extension; switching works after the log in without touching a setting. |

### 5.4 Overlays and menus

Run each on KDE Plasma and Hyprland at least, at scale 1 and at a fractional scale, over a
windowed game and a fullscreen one.

| # | Steps | Expected |
|---|---|---|
| OVL-1 | Open the on-screen keyboard over a fullscreen game; type into a text field; use Paste. | Keys reach the game; the overlay never takes focus; Paste sends Ctrl+V. |
| OVL-2 | Open the numpad while the keyboard is up. | One closes the other. |
| OVL-3 | Open and close each menu style (radial, directional, list, button menu, carousel, grid) many times quickly; hold one open for a minute. | No flicker, no stale frame, fades finish. CPU drops to near zero once it stops moving. |
| OVL-4 | Two monitors with different scales; a menu set to each corner of the 3×3 grid. | Each draws sharp, in the right place, on the monitor with the game. |
| OVL-5 | Turn the display off and on with a menu open; change the scale while one is up. | It recovers and draws at the new size. |
| OVL-6 | Move the Guide opener to a double tap, then to a combo (App settings → Controller shortcuts); open the Guide overlay; use each Guide shortcut. | Each opens what it says; the warnings on the tab match the clashes. |
| OVL-7 | An info overlay with every live token (time, profile, layer, mode, app, cpu, ram, gpu, wifi, both batteries with `:icon`). | Each shows a real value and updates every second. |
| OVL-8 | Hold and toggle layers, stack two, switch mode, change profile while a layer is held. | As [layers.md](layers.md) and [modes.md](modes.md) say; held layers end on a profile change. |
| OVL-9 | Guide + right trigger: Tools. Calculator: an expression with × and ÷, then Send and Copy. Doodle: each tool, then the text tool with bold and italic. | The answer is typed into the game, or on the clipboard; Doodle marks stay until cleared; a Guide press afterwards doesn't re-fire a trigger shortcut. |
| OVL-10 | Turn off every audio output, move a menu's cursor, turn one back on, move again (CR H-13). | One log line about no audio device; cues sound again without a restart. |
| OVL-11 | The Guide overlay with Steam running, then without (CR H-18). | The Steam overlay row is there only while Steam runs. |
| OVL-12 | On labwc, Budgie and COSMIC with two monitors: a game on each in turn, the pointer on the other monitor; then move a game to the other monitor with an overlay process running. | The overlay opens on the game's monitor every time. (Checked by hand in VMs, not with a pad; the move with one overlay process running not yet.) |
| OVL-13 | On GNOME and Cinnamon, with a real pad: the keyboard, Guide shortcuts and every menu style over a windowed and a fullscreen game, on two monitors and at a fractional scale; watch the desktop's own animations while a menu moves. | The overlay shows on the focused window's monitor, sharp, and hides again; clicks pass through it; the Shell doesn't stutter. |

### 5.5 Feel and timing

Need a person with the controller in hand. Record the values used, so a later run can compare.

| # | Check |
|---|---|
| FEEL-1 | Deadzone, response curve, acceleration and outer-edge boost feel right in a mouse-look game and in a menu. |
| FEEL-2 | Flick stick: set Turn size, press Test turn, then play. One full turn of the stick is one full turn in game; the camera stops cleanly. |
| FEEL-3 | Gyro mouse, gyro stick and steering; the jitter threshold hides drift without eating slow aim; Calibrate gyro helps a drifting pad. |
| FEEL-4 | Combo window (60 ms), double and triple tap, long press, Toggle, Turbo and cycles: chords register without firing single buttons; gestures work for a relaxed press and a quick one. |
| FEEL-5 | Input latency feels like a wired pad without Padwight. If in doubt, measure (T-3's rig can, or a high-speed camera). |

### 5.6 In-game editing

| # | Steps | Expected |
|---|---|---|
| EDIT-1 | Guide + Start over a ruled game; change stick sensitivity, Invert Y, the profile; Save. | The change applies at once and is in the setup's files after Save. |
| EDIT-2 | Edit Controls: change a button, a stick, a trigger, gyro, the touchpad, a combo, a layer, a macro, a menu; name a new item with the on-screen keyboard. | Each row does what the settings window does; typing works; the live pictures follow the controller. |
| EDIT-3 | Bring another window to the front with the mouse while the menu is open; switch profile on Quick Settings and Discard (CR H-11). | The menu keeps editing the game; Discard leaves the files as they were. |
| EDIT-4 | Save a change in the settings window while the in-game menu is open (CR H-12). | The in-game menu closes and shows the new values when opened again. |
| EDIT-5 | Unplug the pad with a change made; replug; turn remapping off from the tray; kill `padwight overlay` (CR H-19). | As CR H-19 says. |
| EDIT-6 | Edit Controls over a window with no setup. | A change makes a new setup for that window. With no window in front, Edit Controls is refused. |

### 5.7 Settings window

| # | Check |
|---|---|
| GUI-1 | Every page, tab, sub-tab, card and dialog in the light theme and the dark one. Nothing clipped, nothing unreadable. |
| GUI-2 | Every empty state: no setups, no profiles, no macros, cycles, layers, modes, menus or overlays. |
| GUI-3 | The window at its minimum size and at a fractional scale. |
| GUI-4 | Keyboard only: add a setup from the library, change a button, save. |
| GUI-5 | Save & apply with problems: an unknown key, a missing macro, an incomplete combo. Each is flagged on its row and its tab. |
| GUI-6 | `padwight reload` after a hand edit, with the window open and unsaved edits in it. |
| GUI-7 | Import a pack: the preview, a name clash, a rule clash, a pack from a newer format (refused), an update with the player's own changes listed. |
| GUI-8 | Edit on screen with a real mouse: drag, resize, grid snapping, click an item or cell to open it, Also show, fold and dock the panel; without a compositor (black behind it is known). |
| GUI-9 | Find by pressing; the token picker; Copy from another setup; the Manual page's pictures. |
| GUI-10 | The tray icon, the app menu entry and the window icon. |

### 5.8 Sound and rumble

| # | Check |
|---|---|
| SND-1 | Every overlay's default cues, on speakers and headphones, at a normal game volume: none startles, none is lost under game audio. |
| SND-2 | Every synthesized cue kind the library packs use (`grep -ho 'kind = "[a-z_]*"' packs/*.padpack \| sort -u`; lockpick, shing, geiger, radio and the rest), with the volume, pitch and length the packs give them: `padwight debug sound <kind>` plays one. Several were never listened to. A pack's own WAV sounds (`[assets]`, none in the library) get the same once. |
| SND-3 | The Performance card's Sounds switch, and a setup's own sound set, take effect at once (`scripts/sound-trace.sh` shows which cue plays). |
| SND-4 | Rumble cues on a pad with motors, at each strength. |

### 5.9 Install and lifecycle

| # | Steps | Expected |
|---|---|---|
| INS-1 | Install from the release tarball on a clean VM of the oldest supported distro, and on Fedora, Ubuntu LTS and Arch; with and without `uaccess`. Then `./install.sh --uninstall`. | Runs without root; the udev fallback works; uninstall leaves no files, units or rules. |
| INS-2 | `cargo install` from source, following the README. | Same as INS-1. |
| INS-3 | A pack from a newer format. | Refused, asking for an update. |
| INS-4 | Install on an immutable distro (Silverblue or Bazzite), following README "Immutable and atomic distros". | Works without layering packages; only the rules and `padwight-uinput.conf` land in `/etc`. |
| INS-5 | Log out and in; reboot. | The service starts at login. |
| INS-6 | Without `XDG_RUNTIME_DIR`: the socket directory is `0700`, the socket `0600`; `padwight status` from another user is refused. | Only this user reaches the daemon. (Automated: `socket_fallback`.) |
| INS-7 | `scripts/check-install.sh`. | Passes (in CI). |
| INS-8 | `padwight doctor` on a machine missing each piece in turn: no uinput access, no udev rules, no motion rule, no daemon. | Each missing piece is named with the command that fixes it. |
| INS-9 | Read the daemon log after a full session of play. | No errors, no panics, messages clear to a player. |
| INS-10 | Outside systemd: the tray icon's Remapping switch, Open settings, Close overlays, Quit. | Each works; Quit releases everything. |
| INS-11 | Screenshot and recording on each desktop, in a game whose name has a colon. | The file is saved and plays; the folder name replaces the colon. |

## 6. Pack QA

The library has 232 packs. The Rust tests already check that every pack parses, passes `check`,
round-trips through a save, uses only known tokens and built-in assets, and that no two packs claim
the same executable. Those prove a pack is well formed, not that it is right. Three things can still
be wrong, from cheapest to check to dearest:

1. **Identity: the Steam app IDs, executable names and window classes.** Several families were
   written with unverified IDs (Doom, Resident Evil, Remedy, Baldur's Gate, KOTOR, Warcraft III and
   Company of Heroes among them). `pcgamingwiki/` has the facts offline, so this check can be
   automated entirely (T-13). Until then: for each pack, look the game up in `games.tsv` and
   `executables.tsv` as [game-setup-guidelines.md](game-setup-guidelines.md) says.
2. **Bindings: does each key the pack sends do what its label says in the game's default
   controls?** For games whose defaults are in a readable file (Unreal's `User.ini`, Dark engine
   `.bnd`, id Tech `.cfg`, Source `config.cfg`), a tool can compare them (T-14). For the rest, a
   person reads the game's controls screen.
3. **Play: does the scheme work in the game?** Only a playtest shows that a menu is reachable, a
   gesture is comfortable, or the game drops fast macro input.

Playtests are tiered, since 232 full playtests are out of reach:

| Tier | Which packs | Check | When |
|---|---|---|---|
| P0 | Packs changed since the last release | Smoke: the rule switches on launch; movement, camera, the main action and the pause menu work; the Guide overlay's notes are right | Every release |
| P1 | One pack per engine family (packs on one engine share their bindings), and the ten most played | Full: play through a level, use every mapped action at least once, every menu, every gesture | Every release, rotating within the family |
| P2 | Everything else | Smoke, once | Once per pack, then when its bindings change |
| G-3 | Packs with turbo or macros | Fast input is accepted by the game | When those mappings change |
| G-4 | Any pack | Export, import on a clean config, play: it behaves the same | Every release, one pack |
| G-6 | Games with anti-cheat | Record whether it blocks virtual input; never try to get round it | When a pack for one is added |

Track each pack's state (IDs checked, bindings checked, last smoke, last full play) in one table,
generated and kept current by T-13, the way `pack-candidates/build.py` keeps the candidate list.

## 7. Security review

For a reviewer before each release. Automated checks can't settle these.

| # | Question |
|---|---|
| SEC-1 | **The control socket.** It is `0600` in a private directory. The protocol is now public (1.0), so any program running as the user can `SetConfig`, switch profiles or read input. Is that the right line, and should the docs say so more plainly? |
| SEC-2 | **Packs.** A pack can't name a program; the only host-program actions are Screenshot and ToggleRecording. Packs now carry SVG icons and base64 WAV sounds: are the SVG and WAV parsers safe on hostile input (T-5 fuzzes them)? Should the import preview list screenshot and recording actions? |
| SEC-3 | **Virtual input.** The virtual keyboard and mouse can be used by any local program. Is that clear to users? |
| SEC-4 | **Shell extensions.** The daemon installs and enables GNOME and Cinnamon extensions that run inside the Shell, and the overlay hands them a memfd to draw. Can another program on the session bus show pictures through them, or get the Shell to read past the buffer with a wrong size? |
| SEC-5 | **Config files.** Written `0600` through one helper, swapped in from a staging folder. Does a crash at any point leave either the old tree or the new one, never a mix (T-9)? |
| SEC-6 | **Dependencies.** Read `cargo audit`'s warnings and the direct dependencies' licenses. |
| SEC-7 | **Force Quit.** Can a window's reported pid, or a process name, make it end something outside the game's tree (the desktop, a launcher, another user's process)? |
| SEC-8 | **Helper programs.** Clipboard (`wl-copy`, `xclip`, `xsel`), screenshot and recording tools are run with arguments, never through a shell. Check no text the player types reaches a command line. |
| SEC-9 | **The dashboard example.** It binds to `127.0.0.1` and checks the `Host` header. Re-check when it changes. |

## 8. Desktop session runs

Rerun when the daemon's interaction with the system changes (focus tracking, input, the GNOME or
Cinnamon extension, the overlay window), and when a desktop, the kernel or the input stack has a new
release:

```sh
scripts/desktop-versions.sh   # network: latest GNOME, Plasma, Sway and Hyprland; exits 1 if GNOME's major isn't in the extension's shell-version list
DESKTOP_TEST_HOST=dev@127.0.0.1 DESKTOP_TEST_SSH_OPTS="-p <port> -i vms/id_ed25519" scripts/desktop-test.sh gnome   # or cinnamon, budgie, cosmic, kde, sway, hyprland, niri, labwc
scripts/gnome-versions/run.sh   # the GNOME extension on each older Shell, headless in Docker
```

`desktop-test.sh` syncs the checkout to the VM, builds it, and checks in that session that the
tracker is found, a terminal's window is seen, its rule switches the profile, the profile falls
back when the window closes, and the overlay process runs. Set `DESKTOP_TEST_QMP` to the VM's QMP
socket to close the GNOME Overview first. Run it only against a throwaway VM: the daemon grabs
gamepads and creates virtual input devices. niri's VM needs GL (`VM_GL=1 vms/vm.sh start niri`).

Last full run 2026-10-09, kernel 7.2.9; the newer desktops have their own dates:

| Desktop | Version | Result |
|---|---|---|
| GNOME | 51.0 | all pass (2026-10-09; overlay skipped then). The extension overlay since checked by hand in the VM on Shell 51: keyboard, Guide shortcuts and menus over a fullscreen window, two monitors, fractional scale; Shell 45 to 51 headless (L7b) |
| Cinnamon (Wayland) | 6.6.9 | all pass (2026-10-30, kernel 7.2.11); overlay by hand over a fullscreen window, on two monitors (1280x800 at 1.25, 1024x768 at 1) and at scale 1.5 |
| COSMIC | cosmic-comp 1.10.0 (Arch) | all pass (2026-10-31, kernel 7.2.11); overlay by hand over a fullscreen window, on two monitors and at scale 1.5. The surface is made again each time it shows, so it lands on the active output; moving a game between monitors with one overlay process running not checked |
| Budgie (Wayland) | 10.10.2 on labwc 0.20.2 (Arch) | all pass (2026-10-31, kernel 7.2.11); wlroots tracker; overlay follows the focused window's monitor whatever the pointer is on, over a fullscreen window and at scales 1.5 and 1.25; desktop name `Budgie:GNOME` or unset still picks wlroots |
| Ubuntu Budgie | 26.04.1, Budgie 10.10.2, labwc 0.9.3 (kernel 7.0) | all pass (2026-10-31), the same checks by hand. Names its desktop plain `Budgie`; `budgie-power-dialog` owns `org.gnome.Shell`. VM: `vms/vm.sh create ubuntu-budgie ubuntu-budgie` |
| niri | 26.04 | all pass (2026-10-31), also with an executable rule; layer-shell overlay draws. Xwayland games (xwayland-satellite) not checked |
| KDE Plasma | 6.7.5 | all pass |
| Sway | 1.12 | all pass |
| Hyprland | 0.56.2 | all pass |
| labwc | 0.20.2 | all pass |

## 9. Commands

From `CLAUDE.md`, plus the checks CI runs:

```sh
cargo check --all-targets --message-format=short 2>&1 | head -40
cargo clippy --all-targets --message-format=short
RUST_BACKTRACE=0 cargo nextest run --profile agent --hide-progress-bar --cargo-quiet [filter]
python3 scripts/check-project.py
scripts/check-install.sh
cargo audit
```

Container tests (need `/dev/uinput` and `/dev/uhid`; run as root in the container):

```sh
scripts/kernel-test-docker.sh nothing_stays   # stuck output, kernel level
scripts/kernel-test-docker.sh daemon_         # release on exit and on signals
scripts/kernel-test-docker.sh hotplug_        # unplug, replug, a different pad, disable/enable
scripts/kernel-test-docker.sh idle_           # idle CPU
scripts/kernel-test-docker.sh process_        # process-scan switching
scripts/kernel-test-docker.sh gui_            # the settings window under Xvfb
scripts/kernel-test-docker.sh socket_fallback # the socket without XDG_RUNTIME_DIR
scripts/kernel-test-docker.sh debug_          # debug sessions against a --debug daemon
dbus-run-session -- cargo nextest run --run-ignored ignored-only gnome_   # GNOME focus against a mock shell (host, needs gjs)
```

Things that have tripped up container runs:

- **Stale builds across checkouts.** Every checkout is mounted at `/src` and shares the
  `padwight-kernel-target` volume, and Cargo rebuilds by file times. After a run from another
  checkout, files older than that build aren't rebuilt, and the run tests the other checkout's code.
  Check the test count, then `touch` the sources (or `docker volume rm padwight-kernel-target`)
  before trusting the result.
- **A daemon running on the host.** The container sees the host's `/dev/input`. The `debug_` tests
  read the keyboard that appeared after their daemon started, so they run either way. The tests with
  a uhid controller stop at once and say so, since the host daemon would grab their controller and
  type their keys into the desktop. Stop it with `systemctl --user stop padwight`.
- Outside the container, the tests that read virtual devices need the account in the `input` group;
  the others only need `/dev/uinput`.

Performance (not pass/fail): `scripts/profile.sh [filter]` writes JSON per measurement and
`target/profile/report.html` with charts. See [architecture.md](architecture.md#testing-and-checks)
for its settings and the host-only parts, and the overlay harness in
[overlay-debugging.md](overlay-debugging.md#the-harness).
