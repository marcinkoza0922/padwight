# QA plan

How Padwight is tested: what can go wrong and what it costs, which layer of testing catches each
kind of fault, what runs on every pull request and what before a release, and the checks that still
need a person, a controller or a game. The tools this plan needs but that don't exist yet are in
[qa-tooling.md](qa-tooling.md); each gap below names the tool that closes it (`T-n`).

Fix-by-fix findings live in [code-review-plan.md](code-review-plan.md). Its hardware checks are
written `CR H-n` here, to keep them apart from this plan's own IDs.

**Scope.** Wayland sessions on x86_64 Linux. X11 and XLibre sessions aren't supported and aren't
tested; a failure that shows only there is not a bug. X11 *clients* inside a Wayland session are
in scope: most games, and every Proton game, run through Xwayland, so focus tracking, Force Quit
and the overlay are checked against them (FOC-11). Beyond the desktop, the rest of the system the
daemon runs on (kernel drivers, other programs that own the pad, sandboxes, keyboard layouts,
audio, graphics, init and seats) is covered in [section 5.10](#510-linux-environment).

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

What exists today, from fastest to slowest. Counts are as of 2026-11-02.

| # | Layer | What it proves | Where it runs | How |
|---|---|---|---|---|
| L1 | Unit tests (1,577 `#[test]`) | Logic of single modules: engine, config, packs, menus, renderer | CI | `cargo nextest run --profile agent …` |
| L2 | Sweeps over many inputs | No stuck output after random input; damaged packs and configs are refused without a panic; ring sectors leave no gaps; every library pack round-trips | CI | `engine::release_tests`, `library::tests::damaged_packs_…`, `config::tests::damaged_configs_…` |
| L3 | Off-screen pictures | What the overlay, the settings window and the controller drawings look like, with no display | Locally, by hand | `debug overlay`, `debug overlay compare`, `debug motion`, `debug gui` |
| L4 | Debug sessions | Mappings end to end through a real daemon, with a virtual controller and recorded output | CI (session tests); `--ignored` daemon tests need `/dev/uinput` | `tests/debug_session/`, `padwight debug --attach` |
| L5 | Kernel tests in a container | Real uinput and uhid: release on exit, hot-plug, idle CPU, process-scan switching, the GUI starting (under Xvfb, an unsupported session type: it proves the window builds, not how it behaves on Wayland) | Docker, by hand | `scripts/kernel-test-docker.sh <filter>` |
| L6 | Headless compositor harness | The overlay process on a real Wayland compositor (KWin): frame pacing, buffers, scale, CPU and memory | Locally, by hand | `scripts/overlay-harness.sh scripts/harness/<scenario>` |
| L7 | Desktop VMs | Focus tracking, rules and the overlay process on GNOME, Cinnamon, Budgie, COSMIC, Plasma, Sway, Hyprland, niri, labwc and Wayfire | QEMU VMs in `vms/`, by hand | `scripts/desktop-test.sh <desktop>` |
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
| On-screen keyboard, numpad, Paste | T2 | keyboard 13 | Units, including the typed line's caret (moving, editing at it, blinking); scenes | Over a fullscreen game on each compositor; the caret with a real pad, and LB/RB sending arrows to a game with a text field | OVL-1, OVL-2 |
| Tools: calculator, doodle, clipboard (`tools/`, `clipboard.rs`) | T2 | 27 | Units; scenes | Never run on hardware; `wl-copy` on each desktop, and Paste into an Xwayland game | OVL-9 |
| Info and log overlays, tokens (`info/`, `inputlog/`) | T4 | 15, 17 | Units; scenes; library token check | Live values on real hardware (battery, wifi, GPU) | OVL-7 |
| Overlay process and renderer (`overlay/render`, `overlay/wayland`) | T2 | overlay 14 | Units; compare with iced; harness pacing, surface, scale, CPU | Never on a real screen (renderer plan step 8.3); ABGR8888 buffers; integer buffer scale; following the focused window's monitor (`wayland/monitors.rs`) has no test | T-8, OVL-3 to OVL-5, OVL-12 |
| GNOME and Cinnamon overlay through the Shell extension (`overlay/gnome.rs`, `gnome-extension/`, `cinnamon-extension/`) | T2 | — | Shell 45 to 51 headless (L7b); GNOME 51 and Cinnamon 6.6 in VMs, by hand | No unit tests; never with a real pad; `Present` blocks Shell's main loop, so a slow frame stalls the desktop | OVL-13 |
| Sounds and rumble cues (`sound/`, `rumble.rs`) | T4 | 14, 6 | Sound trace; loudness audit | Library packs' synthesized cues never listened to; rumble cues on real motors | SND-1 to SND-4 |
| Settings window (`gui/`) | T3 | 8 | Every page builds; tab shots; GUI under Xvfb (L5); one `iced_test` simulator test | Nothing clicks through the window and checks the saved config; dark theme; minimum size; the `gui_` container test runs on X11, so nothing starts the window on a Wayland compositor automatically (the harness's KWin could) | T-10, T-11 |
| On-screen stage editor (`gui/stage/`) | T3 | — | Shots; handle drag in the simulator | Never used with a real mouse; focus and scroll after a pick run only live | GUI-8 |
| In-game editor: Quick Settings, Edit Controls (`system_menu/`, `layout_editor/`) | T3 | 18, 7 | Units; scenes | Typing, live pictures and Save/Discard on hardware; layout editor is thin on tests | T-1, EDIT-1 to EDIT-6 |
| Packs: parse, check, import, export, update (`pack.rs`) | T3 | 18 | Round trips, damaged input, clashes, forks, assets | Coverage-guided fuzzing of parse and of SVG and WAV assets | T-5 |
| Library: 231 packs (`packs/`) | T1 | — | Every pack valid, round-trips, known tokens only, built-in assets only, no shared executables | IDs and executable names unverified for several families; bindings unverified against the games | T-13, T-14, [section 6](#6-pack-qa) |
| Control protocol 1.0 (`ipc.rs`, `docs/protocol.md`) | T3 | 11 | `ipc/tests.rs` pins the wire format | The doc's examples aren't executed; no client outside Rust is tested | T-15 |
| CLI, doctor, tray (`main.rs`, `doctor/`, `tray/`) | T3 | 17, 8 | Docs name real commands (`check-project.py`); doctor units | Doctor against real broken setups; tray Quit in a session | INS-8, INS-10 |
| Screenshots, recording, media (`capture/`, `record/`, `media/`) | T2 | 17, 15, 11 | Units; `--debug` daemon writes to a temp folder | Each desktop's tool; a stuck MPRIS player | INS-11 |
| Input priority and latency (`priority.rs`, `daemon/input_timing.rs`) | T1 | 6 | Profiling suite | No regression gate | T-12 |
| Install, systemd, udev, the desktop extensions, tarball (`dist/install.sh`, `dist/get.sh`, `release.yml`) | T0 | — | `check-install.sh` validates the files and syntax-checks both scripts in CI; the release workflow refuses a tag that doesn't match `Cargo.toml` | A clean install on each distro family; the one-line installer through a pipe, with and without a terminal; `padwight-uninstall` and `--purge`, including the GNOME/Cinnamon extension's removal; the release workflow has never run | INS-1 to INS-7, INS-13 |
| Self-update (`update.rs`, `daemon/updates.rs`, `doctor --files`) | T0 | update 13 | Units: version from the redirect, version order, which copies may update themselves; `doctor --files` stale-file units | Never run against a real release; a checksum mismatch, a cut download or a failed `install.sh --update` leaving the old binary working; the daemon restarting itself mid-update; the daily check (`daemon/updates.rs` has no test) and turning it off | INS-12 |
| Kernel drivers and other owners of the pad (`input/linux.rs`, `pad_identity.rs`, `doctor/`) | T0 | input 12 | Driver name checks for `xpad`'s label order; uhid pads in the container | Out-of-tree `xone` and `xpadneo`, `joycond`, `hid-steam`, the oldest supported kernel; InputPlumber, Handheld Daemon and other remappers grabbing or hiding the pad | ENV-1, ENV-2 |
| Seats and sessions (`daemon/`, udev rules) | T0 | — | Nothing | A second user's session, or a switch to a text console, while the daemon holds the pad: grabbed evdev and uinput file descriptors survive the loss of `uaccess` | ENV-9 |
| Keyboard layouts (`keyboard/xkb.rs`, `output/`) | T1 | — | Units for XKB labels | Mapped keys, typed text and macros under AZERTY, QWERTZ, Dvorak and a non-Latin layout; per-desktop layout detection | ENV-5 |
| Sandboxed and nested game sessions (`focus/identify.rs`, `quit/`) | T1 | — | Flatpak Heroic pid handling (units) | Flatpak and Snap Steam, Bottles; gamescope nested and as Steam's game mode | ENV-3, ENV-4 |
| Audio, graphics and init outside the usual stack (`sound/`, `media/`, `overlay/`, `tray/`) | T2 | — | VMs use virtio GL and PipeWire | PulseAudio alone, ALSA alone; NVIDIA, hybrid laptops, software rendering; non-systemd distros | ENV-6 to ENV-8 |

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
| `dist/`, `scripts/package.sh`, `.github/workflows/release.yml` | `check-install.sh`, and INS-1 or INS-4 on a VM; INS-13 for `get.sh` |
| `update.rs`, `daemon/updates.rs`, `install.sh --update`, `doctor --files` | `update::` and `doctor::` tests; INS-12 on a VM against a pre-release tag on a fork |

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
   every `HW-`, `OVL-`, `EDIT-`, `INS-` and `ENV-` check, or a note of why one was skipped. The
   `ENV-` checks may be spread across releases, but each must have passed within the last three.
2. The pack playtest tier for this release ([section 6](#6-pack-qa)).
3. Install from the release tarball on a clean VM of the oldest supported distro (INS-1), and on
   an immutable distro (INS-4). Once the release is published: the one-line installer (INS-13), and
   `padwight update` from the previous release (INS-12).
4. The security review ([section 7](#7-security-review)) with a second reviewer.
5. No open T0 or T1 bug. Open T2 to T4 bugs are listed in the release notes.

## 5. Manual and hardware checks

### 5.0 Recording a run

Each run records: date, commit, OS and kernel, desktop and its version, display scale and refresh
rate, controller model, connection (USB or Bluetooth) and firmware, the pad's kernel driver
(`udevadm info` on its event node), GPU and driver, audio server, keyboard layout, init system,
whether Steam (and which package: native, Flatpak, Snap) or another remapper was running, and
for each check: pass, fail (with the issue number) or skipped (with why). Attach the output of
`padwight doctor --report <file>` (T-17): it collects the version and commit, OS, kernel, desktop and
version, the pad's driver and connection, GPU and driver, audio server, init system, Steam and other
remappers, the doctor checks, the daemon log and the active setup, scrubbed of personal details. Add
by hand what it doesn't know: display scale and refresh rate, controller firmware, keyboard layout.

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
| HW-1 | On each controller in the matrix, run General's Gamepad passthrough, then a remapped profile. | Every button, stick and trigger gives the right output. Sticks and triggers rest at center. (Automated for the DualSense replica: `hw_1_`, `tests/hardware.rs`; the Xbox and Switch families and real firmware still need a person.) |
| HW-2 | Unplug while a mapped key is held; plug back in. | The key comes up at once; the pad is managed again and works. (Automated with uhid: `hotplug_`.) (Also `hw_11_` covers a stop mid-macro, mid-layer and with a menu open.) |
| HW-3 | Two controllers at once, each with its own setup (see [per-controller-setups.md](per-controller-setups.md)). Unplug the first. | Each pad follows its own mappings; the second keeps working; nothing is held. (Automated with two uhid pads: `hw_3_`, `tests/hardware.rs`.) |
| HW-4 | A PlayStation or Switch pad, first without the motion udev rule, then with it. | Without it, the app says gyro is unavailable; with it, gyro works. The profile doesn't flicker when the motion node appears after the pad. |
| HW-5 | DualSense or DualShock 4 touchpad in each mode: mouse (tap, two-finger scroll, glide), stick, swipes, menu zones, split pad. Also CR H-5 and CR H-16. | Each behaves as the Touchpad tab says. Tap and swipe times feel right. (Partly automated: `tests/touchpad_modes.rs` through a DualSense replica. All eight tests pass: stick, swipe, menu, mouse tap/scroll/glide, CR H-5 and the split pad's right-half click. Two failures were test bugs (a reader left alone through a stroke or glide overflowed its kernel queue and lost the motion; a mid-test `settle` lifted both fingers of CR H-5). The split pad's was a product bug: a click arriving in the same report as the finger landing was read before the finger, so it ran the left half's action; `input/linux/touchpad.rs` now sends the click after the report's fingers. Feel stays manual.) |
| HW-6 | Test rumble on a pad with motors and one without; play a game that rumbles. | Rumble works, or the app says it isn't supported. |
| HW-7 | Leave a pad idle for several hours, then play. Also suspend and resume, and let a Bluetooth pad sleep and wake. | No drift, no lost input, no CPU while idle; the pad comes back after resume and wake. |
| HW-8 | With Steam running and a PlayStation or Switch pad, with and without `dist/72-padwight-hidraw.rules`. | Without the rule, the GUI and a toast warn that Steam reads the pad past Padwight. With it, no warning and no double input. (Automated with a fake `steam` holding the raw node, mode 0000 standing for the rule: `hw_8_`, `tests/hardware.rs`; real Steam stays manual.) |
| HW-9 | Slay the Spire 2 (reads only the first Steam controller): start Steam with the pad already connected; also restart the daemon with the pad connected. | The game reads Padwight's pad in both cases. |
| HW-10 | `padwight disable; padwight enable` in one line, on each family. | The pad is taken back at once; a touchpad still drives the profile (CR H-16). (Automated for a generic Xbox-id pad and a DualSense with touchpad: `hw_10_`, `tests/hardware.rs`; a Switch pad and a DualShock 4 replica are not built.) |
| HW-11 | `systemctl --user restart padwight` mid-macro, mid-layer and with a menu open; then `kill -9` the daemon. | It comes back, releases everything, and doesn't grab twice. After `kill -9` the pad and the virtual devices are free. (Automated with SIGTERM and SIGKILL, mid-macro, mid-layer, menu open: `hw_11_`, `tests/hardware.rs`; `systemctl` itself is manual.) |

### 5.3 Desktops and focus

| # | Steps | Expected |
|---|---|---|
| FOC-1 | On KDE Plasma, a game with a rule: launch, Alt+Tab to the desktop and back. | The game's profile in the game, the default outside it. (Automated: `scripts/desktop-test.sh` opens a window no rule matches and closes it again; run on KDE and Sway VMs, 2026-11-02. The Alt+Tab key itself is not pressed.) |
| FOC-2 | FOC-1 on GNOME, Cinnamon, Budgie, COSMIC, Sway, Hyprland, niri, labwc and Wayfire. | Same. On wlroots compositors, Budgie and COSMIC only class rules match; niri matches executables too. (Automated: the same script, `scripts/desktop-weekly.sh` runs it per VM. Run here on Sway and KDE, and on 2026-11-02 on GNOME, Budgie, COSMIC, Hyprland, niri, labwc and Wayfire (all passed; the other-window step passed on GNOME, COSMIC, Hyprland, niri, labwc, Wayfire and Budgie, and was skipped on KDE, where dolphin, kate and kwrite aren't installed); and on Cinnamon 6.6.9 (three runs, all passed, the other-window step with nemo); Wayfire 0.11.0 (Arch VM, `vms/vm.sh create wayfire wayfire`, ssh port 2525) passed twice on 2026-11-02, kernel 7.2.14, through the wlroots tracker (the log line says `wlroots`). `tests/compositor_focus.rs` covers Sway, Hyprland, niri, KWin, Cinnamon, wlroots and COSMIC against stand-ins, 10 passed.) |
| FOC-3 | On Budgie (and Ubuntu Budgie, whose desktop name may be `Budgie:GNOME`), start the daemon. | The wlroots tracker is used, not GNOME's; no GNOME extension is installed; no `overlay:` errors in the log. (Automated: `compositor_budgie_uses_the_wlroots_tracker_and_installs_no_gnome_extension` against stand-ins, passed; `desktop-test.sh budgie` checks it in the VM, run on Budgie 10.10.2 (Arch), 2026-11-02, passed.) |
| FOC-4 | Games started from Steam (Proton), Heroic (native and Flatpak, CR H-4), Lutris and a plain binary. | The library finds them and `debug focus` names the right executable or `.exe`. (Automated: `tests/launcher_names.rs` runs `debug focus` over fake plain-binary and Proton-like Wine processes, passed. Real Steam, Heroic, Lutris and Flatpak launches stay manual.) |
| FOC-5 | Two setups with rules for the same window. | The import preview shows the clash; the imported one takes over only if chosen. (Automated: `an_imported_rule_for_the_same_window_wins_only_if_chosen` in `src/pack/tests.rs`, passed. The preview dialog itself is not drawn.) |
| FOC-6 | On labwc or Wayfire, restart the daemon with a ruled game in front, then switch between two windows (CR H-6). | The game's profile at once; no `no window has focus` between windows. (Automated: `desktop-test.sh` restarts the daemon with a ruled window in front and counts `no window has focus`; passed on Sway and KDE (the KDE restart check timed out once, so the wait is now 15 s; three more KDE runs on 2026-11-02 passed it every time; GNOME 51 and COSMIC, Hyprland, niri and labwc also passed it). labwc passed; Wayfire 0.11.0 passed it in both runs on 2026-11-02, the restart picking the profile at once and no `no window has focus` between windows; Cinnamon 6.6.9 passed it three times on 2026-11-02, also after a log out and in (`scripts/cinnamon-relogin.sh`).) |
| FOC-7 | On GNOME, log out and in once, then restart the daemon with a ruled game in front (CR H-7). | The game's profile at once. (Automated: the same restart step, `desktop-test.sh gnome`; run on GNOME Shell 51.0, 2026-11-02, passed. The log out and in stays manual.) |
| FOC-8 | On Sway, from a ruled game, go to an empty workspace and back. | Default on the empty one, the game's on return. (Automated: `desktop-test.sh sway` goes to workspace 9 and back; passed.) |
| FOC-9 | Without focus tracking, hold Force Quit over a ruled game, then again just after it closed (CR H-17). | The first names and ends the game; the second says there is nothing to force quit. (Automated: `tests/force_quit.rs`, run in the Sway VM, passed. It found that a still held button never finished the hold; fixed in `daemon/main_loop.rs`. Launcher-started games and Flatpak sandboxes stay manual.) |
| FOC-10 | On Cinnamon, a fresh install: start the daemon, then log out and in. | The daemon installs and enables its Cinnamon extension; switching works after the log in without touching a setting. (Automated: `desktop-test.sh cinnamon` removes the extension, starts the daemon, checks it is installed, enabled and loaded; then `scripts/cinnamon-relogin.sh` restarts the VM's display manager (autologin logs the user in again), starts the daemon, and checks the extension is still enabled and its bus name owned, the tracker is Cinnamon and switching works, with no setting touched. Passed in three runs on 2026-11-02, Cinnamon 6.6.9. The daemon doesn't survive the log out, as it has no autostart in the VM.) |
| FOC-11 | On each desktop, a native Wayland game and an Xwayland one (a Proton game, and on niri one through `xwayland-satellite`), each with a class rule and an executable rule; Force Quit each. | Both rules match both games; the overlay shows over both, fullscreen too; Force Quit ends the right one. (Automated: `desktop-test.sh kde|gnome|hyprland` runs a Wayland and an Xwayland window with class and executable rules and Force Quit; KDE ran: both rules matched both windows, Force Quit ended the Xwayland window and left the Wayland one, then the Wayland one; the second Force Quit failed once in two earlier runs but passed in three runs on 2026-11-02. GNOME and Hyprland ran 2026-11-02 and passed (GNOME's first run failed only because the test's two kgx windows shared one process; the script now gives the Xwayland one its own session bus). Budgie, COSMIC, niri, labwc and Wayfire print SKIP (Wayfire 2026-11-02, foot has no X11 mode) for lack of a way to run the terminal both ways. Cinnamon also prints SKIP. Fullscreen overlay, niri and Proton games stay manual.) |

### 5.4 Overlays and menus

Run each on KDE Plasma and Hyprland at least, at scale 1 and at a fractional scale, over a
windowed game and a fullscreen one.

| # | Steps | Expected |
|---|---|---|
| OVL-1 | Open the on-screen keyboard over a fullscreen game; type into a text field; move the caret with LB/RB and the arrow keys, then type, Backspace and Delete in the middle of the line; use Paste. | Keys reach the game; the overlay never takes focus; the caret blinks, moves and edits where it is, and the game's own cursor moves with it; Paste sends Ctrl+V. (Automated: scenarios `keyboard-text-entry` and `keyboard-paste-and-delete` for typing, caret movement with LB/RB and the board's arrows, Backspace, Delete and Paste, and the keys each sends, read from the transcript. Still by hand: that the keys reach a real game, the overlay never taking focus, the blinking caret and the game's own cursor following.) |
| OVL-2 | Open the numpad while the keyboard is up. | One closes the other. (Automated: scenario `keyboard-numpad-swap`, with `padwight overlay-toggle` and `numpad-toggle` asking, since the pad's own buttons all belong to the keyboard once it is up.) |
| OVL-3 | Open and close each menu style (radial, directional, list, button menu, carousel, grid) many times quickly; hold one open for a minute. | No flicker, no stale frame, fades finish. CPU drops to near zero once it stops moving. (Automated: scenario `menus-open-close-many` for the state of every style over twelve quick rounds, and `scripts/menu-styles-check.sh` on a headless KWin for flicker, stale frames, unfinished fades, frames built while held still and CPU, with limits. Still by hand: how it looks on a real screen.) |
| OVL-4 | Two monitors with different scales; a menu set to each corner of the 3×3 grid. | Each draws sharp, in the right place, on the monitor with the game. (Left manual: needs a focused window on each output of a two-output KWin or Sway, which the headless harness cannot make; the overlay-harness has one output and no Sway or labwc is installed.) |
| OVL-5 | Turn the display off and on with a menu open; change the scale while one is up. | It recovers and draws at the new size. (Left manual, for the same reason as OVL-4.) |
| OVL-6 | Move the Guide opener to a double tap, then to a combo (App settings → Controller shortcuts); open the Guide overlay; use each Guide shortcut. | Each opens what it says; the warnings on the tab match the clashes. (Automated: scenarios `guide-opener-double-tap`, `guide-opener-combo`, `guide-shortcuts` and `guide-keyboard-shortcut` for the opener on a double tap and a combo and the Guide shortcuts that open something. The warnings on the tab are the units in `config/shortcuts/tests.rs` and `gui/shortcuts_tab/tests.rs`. Screenshot, recording and force quit are left to the HW checks.) |
| OVL-7 | An info overlay with every live token (time, profile, layer, mode, app, cpu, ram, gpu, wifi, both batteries with `:icon`). | Each shows a real value and updates every second. (Automated: scenario `info-tokens`: time, profile, layer, mode, CPU and memory show real values and follow the pad each second; GPU, Wi-Fi and the batteries accept "n/a" or a real value since they depend on the machine, and `app` is not covered. Real hardware values are still by hand.) |
| OVL-8 | Hold and toggle layers, stack two, switch mode, change profile while a layer is held. | As [layers.md](layers.md) and [modes.md](modes.md) say; held layers end on a profile change. (Automated: scenarios `layer-hold`, `layers-stack`, `mode-switch` and `layers-across-profiles`. A layer toggled from a menu item surviving a profile change is not covered.) |
| OVL-9 | Guide + right trigger: Tools. Calculator: an expression with × and ÷, then Send and Copy. Doodle: each tool, then the text tool with bold and italic. | The answer is typed into the game, or on the clipboard; Doodle marks stay until cleared; a Guide press afterwards doesn't re-fire a trigger shortcut. (Automated: scenario `tools-calculator-send` for Guide + right trigger, a sum with ×, and Send typing the answer. Copy to the clipboard, the Doodle tools and the text tool's bold and italic stay with the unit tests in `tools/` and a person: Doodle's state is not in `debug overlay state`.) |
| OVL-10 | Turn off every audio output, move a menu's cursor, turn one back on, move again (CR H-13). | One log line about no audio device; cues sound again without a restart. (Automated: `scripts/audio-recovery-check.sh`, which swaps the daemon's ALSA default from a missing card to a recording file device and checks one log line, then cues playing again. Not a real sound server: PipeWire, PulseAudio and Bluetooth stay ENV-6.) |
| OVL-11 | The Guide overlay with Steam running, then without (CR H-18). | The Steam overlay row is there only while Steam runs. (Automated: scenarios `guide-steam-row` and `guide-no-steam-row`, each with the daemon alone in a pid namespace, with a process named steam or without.) |
| OVL-12 | On labwc, Budgie and COSMIC with two monitors: a game on each in turn, the pointer on the other monitor; then move a game to the other monitor with an overlay process running. | The overlay opens on the game's monitor every time. (Checked by hand in VMs, not with a pad; the move with one overlay process running not yet.) (Automated: `compositor_wlroots_overlay_follows_the_focused_windows_output` and `compositor_wlroots_overlay_follows_a_game_moved_to_the_other_output` in `tests/compositor_focus.rs`, against the stand-in wlroots compositor; run in the container. The pointer on the other monitor is not modelled by the stand-in, and the real labwc, Budgie and COSMIC are still by hand.) |
| OVL-13 | On GNOME and Cinnamon, with a real pad: the keyboard, Guide shortcuts and every menu style over a windowed and a fullscreen game, on two monitors and at a fractional scale; watch the desktop's own animations while a menu moves. | The overlay shows on the focused window's monitor, sharp, and hides again; clicks pass through it; the Shell doesn't stutter. |

### 5.5 Feel and timing

Need a person with the controller in hand. Record the values used, so a later run can compare.

| # | Check |
|---|---|
| FEEL-1 | Deadzone, response curve, acceleration and outer-edge boost feel right in a mouse-look game and in a menu. (Automated, numbers only: `engine::feel_tests`, which checks that the response curve is monotonic with fixed endpoints, the deadzone is dead inside and continuous at its edge, the outer boost starts at the edge without a step, acceleration ramps smoothly to its ceiling and a stick at rest sends nothing. Whether it feels right in a mouse-look game or a menu stays manual.) |
| FEEL-2 | Flick stick: set Turn size, press Test turn, then play. One full turn of the stick is one full turn in game; the camera stops cleanly. (Automated, numbers only: `engine::feel_tests` turns the stick one full circle in a simulated daemon and requires exactly the configured Turn size in pixels, both ways and at several sizes and tick rates; a flick ends cleanly and stops ticking; `output::test_turn_steps` sends exactly the pixels asked for. Characterised, not fixed: with Rotate smoothing on, the filter is dropped when the stick is let go, so a turn comes up a little short. Whether the camera turns the right amount in a game stays manual.) |
| FEEL-3 | Gyro mouse, gyro stick and steering; the jitter threshold hides drift without eating slow aim; Calibrate gyro helps a drifting pad. (Automated, numbers only: `engine::feel_tests` shows the jitter threshold hiding a synthetic 0.1 degree/second drift while 5 degree/second aim keeps its full travel, the scaling having no step, and auto-calibration removing 0.5 to 3 degree/second drift while keeping slow real motion. Feel on a real pad and Calibrate gyro on one that drifts stay manual.) |
| FEEL-4 | Combo window (60 ms), double and triple tap, long press, Toggle, Turbo and cycles: chords register without firing single buttons; gestures work for a relaxed press and a quick one. (Automated, timing only: the `--fake-clock` scenarios `combo-chords-never-fire-singles`, `combo-window-edges`, `gesture-quick-and-relaxed-taps`, `gesture-long-press-then-double-tap`, `gesture-double-tap`, `gesture-triple-tap`, `toggle-turbo` and `cycle-steps` fix the 60 ms window, the 250 ms double-tap window and the long-press time to the millisecond. Findings recorded in them as they are: a tap on a button that also has a double tap or long press fires 250 ms after release; the radial-menu leak of the right stick could not be reproduced (see `radial-stick-*`). Whether a relaxed or a quick press feels right stays manual.) |
| FEEL-5 | Input latency feels like a wired pad without Padwight. If in doubt, measure (T-3's rig can, or a high-speed camera). (Automated, the daemon only: `scripts/profile.sh profile_input_latency` then `python3 -I scripts/profile-compare.py --budget target/profile` fails when p50 passes 0.5 ms, p99 2 ms, or the worst case one 60 Hz frame, or a press is missed. It measures the daemon, not the pad or game; "feels like a wired pad" stays manual.) |

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
| INS-1 | Install from the release tarball on a clean VM of the oldest supported distro, and on Fedora, Ubuntu LTS and Arch; with and without `uaccess`. Then `padwight-uninstall`; on a second install, on GNOME or Cinnamon after the daemon has put its extension in, `padwight-uninstall --purge`. | Runs without root; the udev fallback works; uninstall leaves no files, units, rules or extension, and drops the extension from the enabled list; settings stay, unless `--purge`, which also removes `~/.config/padwight` and the daemon log. (Automated: `scripts/distro-matrix.sh run ins1@fedora ins1@ubuntu ins1@ubuntu22 vm-ins1@arch`, rootless podman with systemd plus an Arch QEMU VM with real udev and uaccess; still manual: the GNOME or Cinnamon extension removal, and a real Fedora/Ubuntu VM with udev.) |
| INS-2 | `cargo install` from source, following the README. | Same as INS-1. (Automated: `distro-matrix.sh run ins2@fedora ins2@ubuntu ins2@ubuntu22`, using the binary `cargo install --locked` builds; the README steps run in a systemd container, not on a VM.) |
| INS-3 | A pack from a newer format. | Refused, asking for an update. (Automated: `pack::tests::packs_roundtrip_and_newer_formats_are_refused` for the parser and `gui::packs::tests::importing_a_pack_file_previews_then_adds_the_game` for the window's message; the file picker itself needs a person.) |
| INS-4 | Install on an immutable distro (Silverblue or Bazzite), following README "Immutable and atomic distros". | Works without layering packages; only the rules and `padwight-uinput.conf` land in `/etc`. (Automated: `distro-matrix.sh run ins4@fedora`, a read-only root with /etc, /home and /var writable; a real Silverblue or Bazzite still needs a person.) |
| INS-5 | Log out and in; reboot. | The service starts at login. (Automated: `distro-matrix.sh run ins5@fedora ins5@ubuntu ins5@ubuntu22 vm-ins1@arch`: logout, login and restart in containers, a real reboot in the Arch VM.) |
| INS-6 | Without `XDG_RUNTIME_DIR`: the socket directory is `0700`, the socket `0600`; `padwight status` from another user is refused. | Only this user reaches the daemon. (Automated: `socket_fallback`; run here in the container with `scripts/kernel-test-docker.sh socket_fallback`, 2 passed.) |
| INS-7 | `scripts/check-install.sh`. | Passes (in CI). (Automated: `scripts/check-install.sh`; run here, it passes.) |
| INS-8 | `padwight doctor` on a machine missing each piece in turn: no uinput access, no udev rules, no motion rule, no `wl-copy`, an X11 session, no daemon. Then edit each installed rule and the module list by hand and run `padwight doctor --files`. | Each missing piece is named with the command that fixes it. `--files` lists only the changed system files, with their fix, and prints nothing when all are current. (Automated: `scripts/doctor-test.sh` runs the real `padwight doctor` in a container over a made-up system as an unprivileged user: no `/dev/uinput`, denied `/dev/uinput` with and without the rule, edited rules, no motion or hidraw rule with and without a pad, an unreadable pad or motion sensor, X11 and no session, no `wl-copy`, no daemon, and `--files` for each edited rule and the module list, comments only, and restored files; 39 checks. Still needs a person: that the printed `sudo` commands work on a real distro.) |
| INS-9 | Read the daemon log after a full session of play. | No errors, no panics, messages clear to a player. (Partly automated: every scenario run, `scenarios` in `tests/scenarios.rs`, now fails when the daemon's log has an error, failure or warning beyond the ones a machine without a compositor causes, `common::log_problems`. Still needs a person: a long real session on real hardware, and whether each message reads well to a player.) |
| INS-10 | Outside systemd: the tray icon's Remapping switch, Open settings, Close overlays, Quit. | Each works; Quit releases everything. (Automated: `dbus-run-session -- python3 -I scripts/tray-test.py` plays the panel over D-Bus against a `--fake-clock` daemon: the menu, Remapping off and on (daemon state, checkmark and tooltip), Close overlays, Open settings (starts the window), a left click, Quit (exit 0, the release log line, the icon's bus name gone). Still needs a person: a real panel, and that Quit releases held keys, which a `--fake-clock` daemon has no virtual device to show.) |
| INS-11 | Screenshot and recording on each desktop, in a game whose name has a colon. | The file is saved and plays; the folder name keeps the colon (the replacement was Windows-only and went with Windows support). On a Pictures folder on NTFS or exFAT, which refuses a colon, the save fails with a clear message. (Automated: `capture::tests::a_game_with_a_colon_gets_a_folder_and_both_kinds_of_file_are_saved`; still needs a person: each desktop's tool, and that the recording plays.) |
| INS-12 | **Update.** Install the previous release, then: `padwight update --check`; wait for the daily check (two minutes after start) and look at the settings window's banner and the tray entry; Update from the tray, then from the settings window on a second VM, then `padwight update` in a terminal, with a pad held and a layer active. Repeat with a release that changes a udev rule. Also: Settings → Input → Check for updates off; a copy in `/usr/bin` and a source build; the checksum file edited to not match; the network cut mid-download. | The banner and tray name the new version; the update replaces only `~/.local/bin/padwight` and the user files, never asks for `sudo`, and restarts the service, which comes back managing the pad with nothing held. A changed rule is listed with its command at the end, and by `doctor` until installed. With the check off: no banner, no request to GitHub. A package or source copy is refused with advice and the tray entry is greyed out. A bad checksum or a cut download leaves the old version installed and running. (Automated against a local fake release server: `scripts/update-test.sh update daemon`, built from `scripts/fake-release-server.py` and `scripts/package.sh`, in throwaway homes with `systemctl`, `sudo` and `udevadm` shimmed: `update --check`, no release, server down, a source build refused with advice, the update replacing only the user files with no sudo and a `try-restart` last, a bad checksum, a cut download and a 404 each leaving the old binary, the daemon's check after 120 s with the setting on and off (off: no request at all), Status carrying the update the banner and tray read, and the tray's Update request replacing the binary. Still needs a person, with a real published release: the settings banner and tray entry as drawn, a package copy in `/usr/bin`, a pad held and a layer active during the restart, the service really coming back, and a release that changes a udev rule.) |
| INS-13 | **One-line installer.** `curl -fsSL …/get.sh \| sh` in a terminal; the same with no terminal (over `ssh -T`, or from a script); with `sh -s -- --no-system` and `--yes`; `PADWIGHT_VERSION` set to an older release; with only `wget`; on aarch64. | In a terminal it asks before the system files; with no terminal it skips them and says how to add them; the version asked for is installed; a missing tool or architecture stops with a clear message before anything is installed. (Automated against the same fake server: `scripts/update-test.sh getsh`: no terminal (stdin a pipe, no tty), with `--yes` and with `--no-system`; in a terminal, answering n and y, from a file and through a pipe; `PADWIGHT_VERSION` older, with a `v`, invalid and missing; only `wget` (found a bug: wget 2, Fedora's default, printed no redirect, fixed in `dist/get.sh`), neither tool, no `sha256sum`, no `tar`, `uname -m` aarch64; a bad checksum, a cut download, the server down and no release yet. Still needs a person: a run against the real GitHub release page and a real aarch64 machine.) |

### 5.10 Linux environment

What differs between Linux systems apart from the desktop. Most of these need a VM or a spare
install rather than a particular controller; ENV-1 and ENV-2 need the pads and handhelds named.

| # | Steps | Expected |
|---|---|---|
| ENV-1 | **Pad drivers.** Each Xbox pad on in-kernel `xpad` and on the out-of-tree `xone` (USB, dongle) and `xpadneo` (Bluetooth); Joy-Cons on `hid-nintendo` alone and with `joycond` combining them; a Steam Deck on `hid-steam` in and out of lizard mode; a PlayStation pad on the oldest supported kernel. HW-1 and the glyph check on each. (Automated, without the drivers: `input::linux::variants_tests` writes sysfs and capability sets for xpad, xone, xpadneo, hid-nintendo, hid-steam, hid-playstation and an 8BitDo D-input pad, and checks label order, motion frame, trigger kind and what `doctor` names; the `variants_kernel` test makes them as uinput devices (`scripts/kernel-test-docker.sh variants_kernel`). Driver names and capability lists are written from memory of the drivers, so the real out-of-tree `xone`/`xpadneo`, `joycond`, the Deck and the oldest kernel stay manual. Open question the test pins: only `xpad` gets the Xbox label swap; whether `xone` and `xpadneo` label X and Y the same way needs a real pad.) | Face buttons are positional whatever the driver; gyro and touchpad nodes are found; the glyph family is right; `doctor` names the driver when a node is missing. |
| ENV-2 | **Other programs that own the pad.** On Bazzite or ChimeraOS with InputPlumber running, on a ROG Ally or Legion Go with Handheld Daemon, and with AntiMicroX or a second Padwight-like remapper grabbing the pad. Steam is HW-8. (Automated: `tests/second_grabber.rs`, `scripts/kernel-test-docker.sh second_grabber`; a stub holds the evdev grab and a uinput duplicate stands in for the remapper, and it requires that nothing is typed twice, that the log says "cannot grab", that the daemon asks again no faster than once per 2 s scan, and that it takes the pad when the holder lets go. Finding: every ask logs a line, so a pad held for good fills the log at 30 lines a minute; the line gives the kernel reason but does not name the holder (`doctor` lists the remappers that run). The real InputPlumber, Handheld Daemon and AntiMicroX stay manual.) | Padwight either manages the pad without double input or says which program holds it; it never fights another grab in a loop. |
| ENV-3 | **gamescope.** A game in gamescope nested inside each of KDE Plasma and Hyprland; then Steam's game mode session (Bazzite or a Deck). (Partly automated: `focus::identify::sandbox_tests` checks, on a made-up process tree, that a game inside gamescope is found by its executable or Steam App ID and that a class rule on gamescope matches its window. `scripts/gamescope-nested-check.sh` (a real nested gamescope under Xvfb) is built, not run here: gamescope is not in the apt repository the container uses. Plasma, Hyprland and Steam's game mode session stay manual.) | Nested: the rule matches the game (or, if only gamescope's window is visible, the plan records that a class rule on gamescope is needed). Game mode: whatever works is written down in the README; nothing stays held, the daemon doesn't crash without a supported focus backend. |
| ENV-4 | **Sandboxed launchers.** Steam as a Flatpak and as a Snap (Ubuntu), Lutris and Bottles as Flatpaks, Heroic as a Flatpak (FOC-4). For each: a rule, `debug focus`, Force Quit. (Partly automated: `focus::identify::sandbox_tests` runs made-up Flatpak and Snap process trees through the lookup behind `debug focus` and checks the pid, executable, Steam App ID, rule match and which processes Force Quit would end. The trees are modelled, not read from a machine with the launchers, so the real launchers stay manual.) | The library finds the games; the right executable is named across the pid namespace; Force Quit ends the game, not the launcher or the sandbox's helper. |
| ENV-5 | **Keyboard layouts.** With the desktop set to AZERTY, QWERTZ, Dvorak, and a non-Latin layout (Russian or Greek, with US second): a mapped key, a typed macro, the on-screen keyboard (labels and typing), the calculator's Send, and `PADWIGHT_KEYBOARD_LAYOUT` overriding detection. Switch layout while the daemon runs. | Mapped keys go to the key position the config names; the on-screen keyboard's labels match what reaches the game; typed text and Send produce the intended characters or the docs say what they can't type. (Finding 2026-11-02, documented in manual/troubleshooting.md: with a non-Latin first layout (ru, gr) typed text drops Latin letters, since `key_for` looks characters up in that layout only; mapped keys and macros are keycodes and unaffected; `PADWIGHT_KEYBOARD_LAYOUT=us` is the workaround. Space/Tab/Enter were dropped in every layout and are fixed.) (Automated: `distro-matrix.sh run env5`, using `padwight debug layout` and the keys-and-mouse scenario under each layout; switching layout while the daemon runs is not covered, and the layout is read once per process.) |
| ENV-6 | **Audio.** Overlay cues and media actions (volume, play/pause) on PipeWire with `pipewire-pulse`, on PulseAudio alone, and on ALSA with no sound server; switch output to a Bluetooth headset mid-session. | Cues play on each, or one log line says why not; media actions work or say they're unavailable; nothing blocks input while audio is missing (OVL-10). (Automated: `distro-matrix.sh run env6`: PipeWire with pipewire-pulse, PulseAudio alone and ALSA alone on null sinks, a second sink made default mid-run; a real Bluetooth headset and OVL-10 stay manual.) |
| ENV-7 | **Graphics.** The overlay and the settings window on Mesa (AMD and Intel), on NVIDIA's proprietary driver (current and the oldest branch it still supports), on a hybrid laptop with the game on the discrete GPU, and with software rendering only (`LIBGL_ALWAYS_SOFTWARE=1`, or a VM without GL). Also at 144 Hz or more, with VRR on, with HDR on, and on a rotated panel (the Deck's is portrait). (Partly automated: `scripts/software-render-check.sh` draws every overlay scene and runs the settings-window checks with no display, software GL and no Vulkan, and `gui_software_opens_and_keeps_running_without_a_gpu` opens the real window that way under Xvfb (`scripts/kernel-test-docker.sh gui_software`). NVIDIA, hybrid laptops, Mesa on AMD and Intel, 144 Hz, VRR, HDR and a rotated panel stay manual.) | The overlay draws in the right place, sharp and without flicker; idle CPU stays near zero; with no GPU it either draws in software or logs that it can't and the daemon keeps running. |
| ENV-8 | **Init and permissions without systemd.** Void (runit) and Artix or Gentoo (OpenRC, elogind): install, start from an autostart entry, log out and in. Also a system where `uaccess` isn't applied (seatd only). | The README's steps work; the tray icon appears; `doctor` names any missing access with the fix; the daemon releases everything on logout. (Automated in part: `distro-matrix.sh run env8@void`, install, autostart-style start, doctor and SIGTERM in a Void container without an init; runit and OpenRC supervision, Artix and Gentoo are built, not run here, and seatd-only is covered only as far as doctor goes.) |
| ENV-9 | **Seats and session switching.** With the daemon running and a pad held: switch to a second user's graphical session; switch to a text console; lock the screen. Then come back. | While the session is inactive, input doesn't reach it and the active user's session can use the pad (or the docs say it can't); on return, the pad is managed again. No key stays held in either session. (Automated in part: `distro-matrix.sh run vm-env9@arch`, two seat0 sessions in a VM, `loginctl activate` and `lock-session`, and the /dev/uinput ACL following the active session; held pad keys across the switch still need a person.) |
| ENV-10 | **Security modules.** Fedora with SELinux enforcing and Ubuntu with AppArmor's user-namespace restrictions: install, play, use the overlay, Force Quit, run `scripts/sandbox-pid.sh`. | No denials in the audit log (`ausearch -m avc`, `journalctl -k \| grep apparmor`). (Automated: `distro-matrix.sh run vm-env10@fedora vm-env10@ubuntu`, SELinux enforcing and AppArmor in VMs; playing with a pad still needs a person.) |
| ENV-11 | **Paths.** `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_RUNTIME_DIR` set to unusual places; the config directory a symlink to another filesystem; home on NFS. Save from the settings window. | Files land where XDG says; a save swaps in atomically or fails with a clear message, never a half-written tree (SEC-5). (Automated: `distro-matrix.sh run env11@fedora vm-env11@arch`, covering odd XDG dirs, a symlink to another filesystem, a full disk, a read-only folder and NFS home through `debug install-pack`; the settings window itself is not driven.) |

## 6. Pack QA

The library has 231 packs. The Rust tests already check that every pack parses, passes `check`,
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

Playtests are tiered, since 231 full playtests are out of reach:

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
| SEC-8 | **Helper programs.** Clipboard (`wl-copy`), screenshot and recording tools are run with arguments, never through a shell. Check no text the player types reaches a command line. |
| SEC-9 | **The dashboard example.** It binds to `127.0.0.1` and checks the `Host` header. Re-check when it changes. |
| SEC-10 | **Updates and the one-line installer.** `get.sh` and `padwight update` trust whatever the GitHub release holds: the SHA-256 comes from the same release, so it catches a broken download, not a replaced one. Is that enough, or should releases be signed (minisign, or GitHub's artifact attestations)? The control socket's new `InstallUpdate` lets any program running as the user start an update: harmless while it can only install the latest official release? Check the scratch directory (`~/.cache/padwight/update-<pid>`) can't be raced by another user, and that `get.sh` runs nothing if the download is cut short. |

## 8. Desktop session runs

Wayland sessions only; the VMs have no X11 session to test. Rerun when the daemon's interaction
with the system changes (focus tracking, input, the GNOME or Cinnamon extension, the overlay
window), and when a desktop, the kernel or the input stack has a new release:

```sh
scripts/desktop-versions.sh   # network: latest GNOME, Plasma, Sway and Hyprland; exits 1 if GNOME's major isn't in the extension's shell-version list
DESKTOP_TEST_HOST=dev@127.0.0.1 DESKTOP_TEST_SSH_OPTS="-p <port> -i vms/id_ed25519" scripts/desktop-test.sh gnome   # or cinnamon, budgie, cosmic, kde, sway, hyprland, niri, labwc, wayfire
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
| Wayfire | 0.11.0 (Arch, wlroots 0.20.2) | all pass (2026-11-02, kernel 7.2.14), twice; wlroots tracker; Wayland-only (the Xwayland step is skipped) |

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

## 10. Pass log

One entry per pass, newest first: what ran, what it found, what is still open.

### 2026-11-02 (commit before this pass: 18717b6, kernel 7.3.0, no controller, no display)

Ran: check, clippy, the whole nextest suite (1,492 run, 83 ignored; again on the fixed tree: 1,496
run, all passed), scenarios, `check-project.py`,
`check-install.sh`, `cargo audit`; the container suite (`nothing_stays`, `daemon_`, `hotplug_`,
`idle_`, `process_`, `gui_`, `socket_fallback`, `debug_`; 33 tests, no host daemon running;
`nothing_stays`, `daemon_`, `hotplug_`, `idle_` and `debug_` again on the fixed tree, all passed);
`gnome-versions/run.sh` (Shell 45 to 51); the off-screen pictures (`debug overlay all glyphs`,
`compare`, `motion`, `gui-check --strict`, settings tabs), the sound trace and audit; the pack
audit and binding comparison; a read of SEC-4, 5, 7, 8 and 10 against the source. Not run: every
`HW-`, `FOC-`, `OVL-`, `INS-`, `ENV-` and `FEEL-` check, SEC-1, 2, 3, 6 and 9, `debug overlay diff`
(no baseline exists), the fuzz targets.

Fixed:

- **SEC-4.** The GNOME and Cinnamon extensions took any width, height and fd from any program on
  the bus. `SetBuffer` now refuses sizes the buffer can't back. The overlay process seals its memfd
  against shrinking (`F_SEAL_SHRINK`); the extensions take only a sealed one (they try to shrink it by a
  byte, which fails on a sealed memfd) and map it as before, so a client can't shrink it into a SIGBUS.
  `scripts/gnome-versions/probe.py` checks it on Shell 45 to 50; `bench.py` times Present (reading the
  memfd on each Present instead cost 3 to 5 times as much). Cinnamon 6.6.9 (VM): `desktop-test.sh cinnamon` 10/10 with it. Later the same
  day: both extensions also check the caller. Every method is served only to a sender whose `/proc/<pid>/exe` is the
  executable of the process that owns the daemon's bus name (the overlay process is the daemon's own binary re-run), and a
  refusal is an `AccessDenied` error. `probe.py` checks it on Shell 45, 46, 49 and 50 (it stands in for the daemon by
  owning its name; a `gdbus` call is refused). Not run: the Cinnamon extension with the check, nor the Rust overlay
  against a real Shell. Still open: a program that runs padwight's own binary (or injects into it) passes, as does
  anything that took the daemon's bus name before the daemon.
- **SEC-7.** A window that matched a game rule skipped Force Quit's protected-name check. It no
  longer does; the list gained the session plumbing and the launchers; `force_quit` refuses a
  protected process by its own name; the SIGKILL goes only to processes whose start time hasn't
  changed since the SIGTERM. A sandboxed game's in-sandbox pid (`focus/identify.rs` `host_pid`)
  now resolves only among this user's processes, and an unfitting sandboxed process is taken only
  when it is the single candidate. Still open: a pid that names a process here which doesn't fit the
  class, while the sandboxed candidates don't fit either, is kept as reported.
- **SEC-5.** A save writes a `complete` marker last; the next load finishes a swap cut short after
  it, so General and the setups are never one old and one new.
- **SEC-10.** The update's scratch folder is private (0700) and never in a shared temp folder. An
  update already running refuses a second; the child is reaped; a replaced binary's
  `(deleted)` path is cleaned. `get.sh` rejects a `PADWIGHT_VERSION` that
  isn't a plain version. `InstallUpdate` answers Ok once the child has started (it can't know more:
  the update ends by restarting the daemon); the settings window says so, and a failed update is
  logged.
- **Packs.** `mount-and-blade-napoleonic-wars` had the wrong Steam id (73000; the DLC is 48705);
  `doom-and-doom-2` gained its executable (`doom.exe`, read from an installed copy);
  `hitman-world-of-assassination` said 2016. The audit's other `id_other_game` warnings on those
  packs are intended (they cover several apps) and are noted in `pack-qa.tsv`.
- **Settings window contrast.** `gui-check --strict` went from 225 findings to 0: amber and green
  text deepened on the light theme, one accent color that reads on both, button text moved until it
  reads, lighter muted text in dark mode. The checker no longer judges page text a popup covers,
  short labels inside pictures, or a few disabled labels (`layout_check/scan.rs` `exempt`).
- Counts in section 2 and 3 (1,577 tests, 231 packs).

Open from this pass:

- The pack audit still flags 79 of 231 packs. 85 executables aren't in Discord's list (many are
  likely gaps there); 14 packs have no `steam_app_id` rule; `alan-wake` and `mirrors-edge` have a
  release year that differs from the wiki; `half-life-2`, `quake`, `quake-ii` and `against-the-storm`
  are passthrough packs where the wiki doesn't confirm native support; `control-resonant`
  and `final-doom` have no executable rule.
- `cargo audit`: nine allowed warnings (six unmaintained crates, `lru` and tokio unsound, one
  yanked). Read for reachability afterward, see the follow-up below (SEC-6).
- Old vs new overlay renderer: `doodle-*` scenes differ 4 to 7% (the old renderer draws no doodles,
  probably), `quick-styled` 4.9% (not looked at).
- Motion strips weren't opened. Most settings tabs weren't looked at: the `tab` step needs a
  profile or game opened first.
- ~~Baseline~~, ~~dark theme~~, ~~audit reachability~~: done in the follow-up below.

Follow-up the same day:

- **Baseline** written: 137 pictures in `screenshots/regression/baseline` (untracked). Next pass:
  `debug overlay diff screenshots/regression/baseline`.
- **Dark settings pages** looked at (log overlays, menus, app settings, system check): text reads,
  no clipping seen. The system check dialog dims the page behind it as intended. Other tabs not opened.
- **SEC-6, reachability** (read from `cargo tree` and the crate sources, not run):
  - `tokio` `AsyncFd::new` is called only by evdev's async streams. `Cargo.toml` enables no async
    feature for `evdev`, and `src/` never uses `AsyncFd`. Not reachable.
  - `lru` is used only by `cryoglyph`'s glyph cache (`LruCache::unbounded_with_hasher`); nothing here
    calls `pop()` on it. Not reachable from our code.
  - `rustybuzz`, `ttf-parser`: unmaintained, parse fonts through `resvg`/`cosmic-text` (system and
    bundled fonts, and pack SVG assets via `usvg`). Pack SVGs are untrusted input, so this one
    is worth a fuzz run (T-5 covers SVG assets).
  - `zbus_names`, `zvariant`, `zvariant_derive`, `paste`: unmaintained only, no advisory about a
    defect.
  - `clipboard-win` (yanked) is not in the Linux dependency graph.
- **Fuzz `pack_assets`** (SVG, WAV, font, base64 assets; no sanitizer): two workers, 600 s, 138M
  runs, no crash, hang or OOM. Two things on the way:
  - `src/fuzz_lib.rs` lacked the `clock` and `faults` modules, so the fuzz crate hadn't built
    since they were added. Fixed; keep its list equal to `main.rs`'s.
  - A first 300 s run reported an OOM that didn't replay. Cause: `font::register` keeps every
    distinct valid font, and leaks its family name, for the life of the process, so the fuzzer's
    thousands of valid fonts filled memory. The target now calls `font::family_of` (the parser)
    alone. In the app the growth is bounded by the fonts a user imports. Not looked at: whether
    a pack update that swaps fonts repeatedly in one daemon run adds up.
- **Fuzz, the other six targets** (300 s each, two workers, no sanitizer): `pack_parse` 1.96M runs,
  `config_load` 26k (slow: each run writes and loads a folder), `ipc_request` 110M, `tokens` 3.7M,
  `calculator` 2.1M, `engine_input` 1.3M. No crash, hang or OOM; `fuzz/artifacts` is empty.
  Five minutes each is a smoke run, not coverage: run longer before a release.

Follow-up 2026-11-02 (no hardware): `debug overlay diff` against the baseline: 137 of 137 identical.
Settings tabs looked at, light and dark: touchpad (light), cycles and macros (dark); text reads, nothing
clipped. The pack audit now flags 67 of 231 (was 79); the categories are unchanged. Still open: motion
strips, the other settings tabs, every hardware check.

Pack follow-up 2026-11-02: `scripts/store-check.py` (stores, GOG ids, GitHub for source ports) and
`scripts/find-executables.py` (executables from GOG depot manifests and installed Steam copies; its
`store-executables.tsv` is read by `pack-audit.py`). The audit now flags 51 of 231 (`exe_unseen` 49,
was 69). Packs changed: `alan-wake` and `mirrors-edge` (PC release years in the description),
`banished` (GOG executables) and `kenshi` (`kenshi_gog_x64.exe`). Not changed: rules that GOG's build
doesn't ship (Steam-only or Linux names), and folder-prefixed rules, which name Steam install folders.

SEC pass 2026-11-02 (no hardware; read from the source and docs, plus `dashboard` test, which passed):

- **SEC-1, control socket.** `bind_socket_at` makes the directory 0700 (refusing one another user owns), then
  sets the socket 0600. The socket exists briefly with the umask's mode, but the directory is closed to others
  until then. A second daemon is refused if the socket answers. Reads time out after 2 s, but the line is
  unbounded (`read_line`) and a client that trickles bytes keeps its thread: only this user can do it, so low.
  No peer-credential check; the directory and mode are the whole line, which `docs/protocol.md` ("Security")
  states plainly. Verdict: the right line. Open: cap the request line (say 1 MiB).
- **SEC-2, packs.** No pack action names a program; Screenshot and ToggleRecording run fixed helpers. The
  import preview (`pack_summary`) lists profiles, modes, layers, macros, menus and info overlays by count and
  the rules it switches on, but not what the buttons do. So a pack's macros and key actions, which can press
  shortcuts or type text into the focused window, are not shown before Import, and neither are its
  Screenshot and recording buttons. Open: decide whether the preview should flag macros, typed text and
  host-program actions. SVG, WAV and font parsing: fuzzed (`pack_assets`, 138M runs, no crash).
- **SEC-3, virtual input.** README and the docs say padwight emits through uinput devices and needs write
  access to `/dev/uinput`, but nowhere say that this lets any local program do the same. Open: add a line to
  the README's uinput requirement. `protocol.md` already covers the socket.
- **SEC-6, dependencies.** Licenses of the 540 crates in the graph: all permissive, or dual-licensed with a
  permissive option. The copyleft ones: `option-ext` (MPL-2.0, file-level, compatible with GPLv3),
  `r-efi` (an LGPL option beside MIT) and `self_cell` (a GPL-2.0-only option beside Apache-2.0), all usable under
  their permissive terms. `Cargo.toml` has no `license` field while `LICENSE` is GPLv3. Warnings: the
  reachability read above. Not run: `cargo deny` (not installed).
- **SEC-9, dashboard.** Binds 127.0.0.1; every request needs a `Host` of `127.0.0.1:<port>` or
  `localhost:<port>`, and an `Origin`, when sent, equal to it; POST needs `application/json`; only six
  actions are forwarded, with values checked; `SetConfig` is the daemon's own config with those keys changed.
  Matches what the docs claim. Not run live against a browser or a rebinding page.

Done after that pass: the control socket reads at most 8 MiB per request line (`MAX_REQUEST`, `daemon/serve.rs`;
a `SetConfig` carrying base64 pack sounds can be large, so not 1 MiB; no test for the cap yet). The pack import preview
now says a pack's buttons can press shortcuts outside the game and names the host-program actions it uses
(`Pack::host_powers`: screenshots, recording, force quit; test `the_preview_names_host_program_actions_wherever_they_hide`);
not looked at as a picture. The README's uinput paragraph says any local program can make virtual input devices. Still open:
`Cargo.toml` has no `license` field.
