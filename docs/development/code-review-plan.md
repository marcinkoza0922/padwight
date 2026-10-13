# Code review plan

Written 2026-10-12, at commit `01e5f4a`. A plan for reviewing the whole codebase one area at a time, with the findings of each pass and the tools that help check them.

The codebase is about 72,000 lines of Rust in 258 files. A single review of all of it only finds the most obvious problems. When asked to review `src/` with a clean working tree, the review tool fell back to the latest commit and never looked at the rest. So the review goes area by area. Each pass names the files it covers, reads them as they are now (not a diff), and records what it finds here.

## How a pass runs

1. **Scope.** Name the files (the table below). If a pass is over about 6,000 lines, split it.
2. **Read for correctness first.** Use the checklist below. Cleanups come second and are only reported if they make a bug less likely.
3. **Check every finding against the code** before recording it. A finding the code doesn't support is dropped, and the reason is noted.
4. **Fix, with a test that fails without the fix.** Put the fix back to its old form once to check that the test catches it (see pass 0). If no test is possible without hardware, add the case to the hardware checks below and to `qa-plan.md` section 3.
5. **Record** each finding in the pass's table: ID, file, what's wrong, status (fixed / open / not a bug / needs hardware).
6. **Leave the tree clean.** `cargo clippy --all-targets` has no warnings, the tests pass, and files the pass grew over 400 lines are split (`scripts/long-files.sh`).

### Checklist

- **Device ownership.** Every path that grabs a controller asks the same question. Every grab has a release on each exit path: error, unplug, disable, quit, signal. Nothing stays pressed.
- **Event-loop stalls.** Nothing slow runs on the daemon thread: file reads, process scans, D-Bus calls, subprocesses. If it has to, it's cached and the cost is measured.
- **State after a change.** When the config, focus, the enabled switch or a device changes, everything derived from the old state is worked out again (scope, hand-off, layers, overlays).
- **Untrusted input.** Packs, config files, `/proc`, Steam's files and IPC messages can be malformed. Parsing can't panic, loop forever or index out of bounds.
- **Platform split.** Linux and Windows paths make the same decisions (`#[cfg]` twins such as `daemon/evdev.rs` and `daemon/xinput.rs`).
- **Concurrency.** Threads, channels and mutexes: no lock held across blocking calls, no poisoned-lock panics, threads stop when their device goes.
- **Error reporting.** Failures reach the log or the GUI, not `let _ =`, unless ignoring them is the point (and a comment says so).
- **Tests.** The behaviour the code promises in its doc comments has a test.

## Passes

Sizes are lines including tests, at the date above. Passes are ordered by the cost of a bug: one that leaves a controller grabbed or a key held down is worse than one that draws a label wrong.

| Pass | Area | Files | Lines | Status |
|---|---|---|---|---|
| 0 | Steam hand-off and game detection (commit `b6111fd`) | `daemon/steam_handoff.rs`, `focus/`, `steam_appinfo.rs`, `steam_hide.rs`, `library.rs` | ~2,600 | Done, 2026-10-12 |
| 1 | Device ownership | `daemon.rs`, `daemon/`, `input/`, `output/`, `rumble/` | ~6,900 | Done, 2026-10-12 |
| 2 | The engine | `engine.rs`, `engine/`, `motion/` | ~6,500 | Done, 2026-10-13 |
| 3 | Game detection and launchers (rest of) | `focus/` (backends), `launchers.rs`, `offer/`, `opener_watch.rs`, `library.rs` | ~2,700 | To do |
| 4 | Config and packs | `config.rs`, `config/`, `pack.rs`, `keycode/` | ~10,600 (split: `config.rs` alone, then the rest) | To do |
| 5 | IPC and process lifecycle | `ipc.rs`, `monitor.rs`, `main.rs`, `tray/`, `quit/`, `record/`, `capture/`, `media/` | ~2,900 | To do |
| 6 | Overlays and menus | `overlay/`, `menu/`, `info/`, `keyboard/`, `system_menu/`, `radial.rs`, `guide_menu.rs`, `nav.rs`, `layout_editor/` | ~14,300 (split in three) | To do |
| 7 | Sound, drawing and fonts | `sound/`, `pad_svg/`, `font.rs`, `style.rs`, `icons.rs`, the widgets | ~4,400 | To do |
| 8 | The GUI | `gui.rs`, `gui/` | ~16,100 (split in three) | To do |
| 9 | Debug tools and integration tests | `debug/`, `tests/` | ~3,200 + tests | To do |

## Pass 0: Steam hand-off and game detection

Reviewed commit `b6111fd` and the code it touches. Findings were checked against the code before fixing.

| ID | File | Finding | Status |
|---|---|---|---|
| R0-1 | `daemon/evdev.rs` | A controller plugged in, or woken from sleep, while Steam had the controllers was grabbed anyway. The check for a pad seen before asked whether Padwight takes pads, but the check for a new one only asked whether remapping was on. | **Fixed.** Every scan path, evdev and XInput, now asks `Daemon::should_manage` (`daemon/steam_handoff.rs`). No automated test: the scan needs `/dev/input`. Hardware check H-1. |
| R0-2 | `focus/identify.rs` | A sandboxed game's window can report its pid inside the sandbox (X11's `_NET_WM_PID`). That pid nearly always names an unrelated process here, and the code took it as it was, so the game wasn't recognised. | **Fixed.** When the process here doesn't look like the window (class against program name or Steam app), the processes with that pid in a namespace below are tried, and the one that looks like the window wins. `pick_pid` and `fits` have unit tests. `identifies_a_window_from_a_real_sandbox` runs a real pid namespace with `unshare`. With the old rule put back, it fails (the window resolved to pid 1, systemd). |
| R0-3 | `steam_appinfo.rs` | Controller support was cached for the daemon's lifetime, including "not listed". So a game installed after the daemon started was never handed to Steam. Each new app also read the whole `appinfo.vdf`. | **Fixed.** The whole file is indexed once and indexed again only when its modification time changes. Test: `a_game_steam_learns_about_later_is_found`. Measured with `padwight debug steam`: 31 ms for a 10 MB cache of 1,838 apps, on the first Steam game focused after a change. |
| R0-4 | `focus/identify.rs` | A Wine program started by its bare name (`gpp.exe`, `.\game.exe`) had no folder in its path, so rules naming a folder never matched it. | **Fixed.** A bare name is replaced by the matching `.exe` mapped into the Wine loader, which has the full path. Test: `wine_programs_started_by_bare_name_report_the_mapped_path`. |
| R0-5 | `daemon.rs` | Saving the config (ticking "Use this setup even when Steam lists the game…") didn't change who had the controllers until the game lost and regained focus. | **Fixed.** `replace_config` calls `recheck_steam_handoff`, which handles the window in front again when the answer changed. Taking a game back also switches to its profile. Hardware check H-2. The original report also named `SetEnabled`, but that is not a bug: `should_manage` already covers the enabled switch. |
| R0-6 | `daemon.rs` | Handing a game to Steam left the previous game's profile active. | **Fixed.** The hand-off leaves the previous game the way any other window does (`leave_game`). The Steam game's launch is still not recorded. That's deliberate: launch handling (info overlays, start toggles) only applies to games Padwight handles. |
| R0-7 | `steam_hide.rs` | Concern: Steam's games might inherit Steam's `SDL_GAMECONTROLLER_IGNORE_DEVICES`, which hides Padwight's controller, and so ignore Padwight's controller too. | **Partly checked.** With Steam Input on (Slay the Spire 2, 2026-10-12), the game runs with Steam's own list of 794 IDs, not the inherited one, as the module comment says. That list also hides Padwight's IDs, which doesn't matter, because Steam Input has the controllers then. Not yet checked with Steam Input off. Hardware check H-3. |
| R0-8 | `focus/mod.rs` | The process scan tried every rule against every process even after a match nothing could beat. | **Fixed.** `best` stops at a match with the highest specificity. Test: `the_search_stops_at_a_match_nothing_can_beat`. |
| R0-9 | `library.rs` | A test repeated the private `names_folder` check, so it could drift from the matching code. | **Fixed.** It calls `focus::names_folder`. A misplaced doc comment in `focus/mod.rs` was moved too. |
| R0-10 | `daemon.rs`, `config.rs`, `pack.rs`, `gui/games.rs` | The commit grew files that are already far over the 400-line limit. | **Open.** Splitting them belongs to passes 1, 4 and 8. This pass split the file it pushed over the limit: the `focus` tests moved to `focus/tests.rs`. |
| R0-11 | `focus/mod.rs` | Seen with `debug focus processes`: a Steam App ID rule matches Steam's `reaper` wrapper as well as the game, and the scan picks the first, so the launch is recorded with reaper's pid. | **Not a bug.** Reaper lives exactly as long as the game. Noted in case launch tracking ever needs the game's own pid. |

The status reported by the daemon now includes `steam_has_pads`, so tools and the GUI can show who has the controllers.

## Pass 1: Device ownership

Read all of `daemon/`, `input/`, `output/` and `rumble.rs`, and in `daemon.rs` the event loop and every grab and release path: the scans, `device_gone`, `release_devices`, `release_all`, profile switches, identity rebuilds, shutdown, and the signal handlers on both platforms.

| ID | File | Finding | Status |
|---|---|---|---|
| R1-1 | `rumble.rs` | A single failed rumble write to the controller (it was unplugged, or Bluetooth dropped for a moment) ended the forwarding thread. Nothing then answered the game's next rumble upload, so the game waited 30 s for the kernel to time it out. A game that rumbles from its main thread freezes for that long, each time it rumbles. | **Fixed.** Failed plays, stops and gain changes are logged once until rumble works again, and forwarding goes on. Kernel test `a_controller_that_stops_rumbling_doesnt_stall_the_game`, which unplugs a stand-in controller. On the old code it failed after waiting 30.4 s. It runs on a host with uinput access: `cargo test -- --ignored rumble`. |
| R1-2 | `daemon.rs`, `config/store.rs` | Every profile switch, including each automatic switch on alt-tab, rewrote the whole config folder from the daemon's thread. The rewrite deleted `setups/` and then renamed the new copy into place, so a crash, kill or power cut between the two lost every setup. The next save then deleted the copy left in `.saving/`. | **Fixed, both halves.** Profile switches, the remapping switch and gyro calibration now write only `config.toml` (`Config::save_settings`). A full save sets the old folder aside as `.setups.old` (and `.general.old`) until the new one is in place, and loading reads the old one if a save stopped in between. Test: `a_save_cut_short_loses_nothing`, which fails on the old load. Side effect: an automatic switch while the in-game editor has unsaved changes no longer writes them to disk, so Discard really discards them. |
| R1-3 | `daemon/evdev.rs`, `daemon/touchpad.rs` | Letting go of a controller stops its reader threads, but they hold the grab for up to 200 ms (their poll timeout). A scan in that window (taking the pads back from Steam right after giving them, or switching remapping off and on quickly) can't grab the pad and gets it on the next scan, 2 s later. If the pad's grab works but its touchpad's doesn't, the touchpad is marked as attached and never retried, so the desktop keeps it as a mouse until the pad reconnects. | **Open, low.** Needs two state changes within 200 ms. A fix would wait for the readers to close their devices, or retry a touchpad grab that failed with EBUSY. |
| R1-4 | `daemon.rs` | 3,019 lines. | **Partly done.** Focus handling and auto-switching moved to `daemon/auto_switch.rs`, and the control-socket requests to `daemon/requests.rs`, with no change in behaviour. `daemon.rs` is now 2,739 lines. The overlay and menu handling is the next block to move (pass 6). |

Checked and found sound:

- Every scan path asks `should_manage` (after R0-1).
- A failed grab, or a failed virtual pad, drops the device, which releases it.
- Readers report `Gone` when a device errors, and the next scan takes it again.
- `release_devices` and `device_gone` release the engine's outputs before dropping the virtual pad.
- When a virtual device is destroyed, even by SIGKILL, the kernel releases its keys.
- `set_profile` rebuilds virtual pads for a new identity before it resyncs.
- `VirtualKbm` sends each key as it changes, so key order is kept. After a failed write it catches up in code order, which only matters if a write failed in the middle of a key combination.
- SIGTERM, SIGINT and the console events on Windows all reach `shut_down`, which releases everything.

## Pass 2: The engine

Read all of `engine.rs` and `engine/` (buttons, combos, gestures, the Guide, actions, toggles, turbo, macros, triggers, zones, sticks, rings, flick, gyro, the touchpad) and `motion/`. Each fix has a test that failed before it.

| ID | File | Finding | Status |
|---|---|---|---|
| R2-1 | `engine.rs` | A trigger mapped to a button, or a stick direction, let go 0.05 below where it pressed. At 0.05, the lowest the settings page offers, that's 0, the resting value, so the action never let go: a key or mouse button stayed held until something else released everything. A pack or config file with a threshold of 0 also pressed every such action at rest. | **Fixed.** `analog_pressed` decides both: nothing presses at rest, and everything lets go there. Tests: `a_trigger_pressing_at_the_lowest_setting_lets_go_at_rest`, `a_stick_direction_pressing_at_the_lowest_setting_lets_go_at_rest`, `nothing_presses_at_rest_whatever_the_threshold`. |
| R2-2 | `engine.rs` | A macro that plays itself, directly or through another macro, recursed without end on the first press: the stack overflowed and the daemon aborted. A pack can contain such a macro. | **Fixed.** `set_macros` finds the macros that reach themselves (`looping_macros`), logs them, and makes them do nothing. A macro that plays another one still works. Test: `a_macro_that_plays_itself_is_refused_rather_than_crashing` (on the old code: `stack overflow`, SIGABRT). |
| R2-3 | `engine/touchpad.rs` | The finger that counts on a side (it moves the pointer, steers the joystick, picks the menu item) was meant to be the first one down, but the code took the lowest contact slot. The pad gives a lifted finger's slot to the next finger down, so after lifting the first of two fingers and putting another down, the new finger took over. | **Fixed.** `primary` picks the finger down longest, and so does the click's side. Test: `the_finger_first_down_keeps_the_pointer_when_another_lands_in_a_lower_slot`. Hardware check H-5. |
| R2-4 | `engine/touchpad.rs` | A layer that switched a touchpad side out of joystick mode while a finger was down left the virtual stick pushed until the finger lifted. Physical sticks recenter in that case. | **Fixed.** Every touchpad event first lets go of sticks whose side is no longer a joystick. Test: `a_joystick_side_switched_to_another_mode_lets_its_stick_go`. |
| R2-5 | `engine.rs` | With the Nintendo face swap on, a face button that has gestures, held past the tap window, sent the button in its position rather than the letter on its label. The other paths swap. | **Fixed.** That path uses `plain_action` too. Test: `the_face_swap_holds_for_a_face_button_with_gestures`. |
| R2-6 | `engine.rs` | A combo that lists the same button twice (`[A, A]`) fires on one press of that button. Only a hand-edited file or a pack can contain one. | **Open, low.** Belongs to pass 4, where loading can reject or deduplicate it. |
| R2-7 | `engine.rs` | 3,095 lines, over half of them tests. | **Partly done.** Tests moved to `engine/tests.rs`, macros and turbo to `engine/macros.rs`, the Guide and mouse mode to `engine/guide.rs`. `engine.rs` is now 1,186 lines; `emit` (marked as predating the size lints) is the next thing to move. Two doc comments on the wrong functions were moved, and one in `config.rs`. |

Checked and found sound:

- `release_all` lets go of everything that can be held: held inputs, toggles, turbos, macros (and their sticks), layers, info and log holds, ForceQuit, pushed stick directions, touchpad clicks and sticks. `release_tests.rs` checks this with random input.
- Release always uses the action recorded at press time (`held`), so a layer or profile change between press and release can't leave the old action held.
- Held combo members and stick directions stay quiet after `release_all` until let go, instead of firing under the new profile.
- Turbo can't spin: its rate is clamped, and a macro turbo waits at least 17 ms. Turbo and macros run on `tick`, which only runs while `needs_tick` says so.
- A toggled gyro goes off when everything is released (profile switch, an overlay taking the controller). That matches toggled actions, which are let go then too.
- Ring sectors, flick and zones ignore a stick at rest, so their thresholds can't hold anything at 0.
- `motion/` only shapes drawing; its styles are fixed presets, so nothing in it comes from a file.

## Tools for checking findings

Tools added in pass 0:

| Tool | What it answers |
|---|---|
| `padwight debug focus` | What the running daemon sees in front: the focus backend, the active profile, whether Steam Input has the controllers, and, for the focused window, its program and path, the Steam app and its controller support, the rule it matches, and the SDL controller variables the process runs with. A long `SDL_GAMECONTROLLER_IGNORE_DEVICES` list is summarised as its length and which of Padwight's controller IDs it hides. |
| `padwight debug focus <pid> [class] [title]` | The same for any window, without the daemon: identifies the pid as the daemon would now, including finding a sandboxed process from its inner pid (it says so when it does). |
| `padwight debug focus processes` | For desktops without focus tracking: which running processes each rule matches, and which one the process scan picks. |
| `padwight debug steam [app id]...` | Reads Steam's app cache as the hand-off does: each cache file's size, how many apps list their controller support, how long reading took, and the support listed for each app ID given. |
| `scripts/sandbox-pid.sh [program [args]]` | Runs a program in its own pid namespace (as a Flatpak or the Steam runtime does) and prints its pid there and here, with the `debug focus` line to try. Needs unprivileged user namespaces. |

Earlier tools that later passes will need are listed in `CLAUDE.md` (overlay and motion screenshots, sound traces, long files), in `qa-plan.md` (project checks, container kernel tests with `scripts/kernel-test-docker.sh`) and in `overlay-debugging.md`.

Tools later passes are likely to want, to be built when a finding needs them:

- **Pass 1 (still worth having):** a way to replay a recorded input stream through the daemon's scan and dispatch without hardware. `daemon::kernel_tests` and `tests/hotplug.rs` cover parts of this in a container. Check that R0-1 has a container test there: a uhid pad that appears while `steam_has_pads` is set must not be grabbed.
- **Pass 4:** a sweep that loads every config folder in `tests/fixtures` and every library pack, saves them and compares the result (A-5 and A-6 cover most of this).

## Hardware checks

These can't be checked without real controllers and Steam. When they pass, they move to `qa-plan.md` section 3.

| ID | Check | From |
|---|---|---|
| H-1 | With a Steam game with full controller support in front (`debug focus` says Steam Input has the controllers), turn a Bluetooth pad off and on, or plug in a second one. Steam must keep it: Padwight's log must not show `managing …`, and the game must respond to the pad. | R0-1 |
| H-2 | With that game in front, tick "Use this setup even when Steam lists the game…" and save. Padwight takes the controllers and the game's profile becomes active straight away, without alt-tabbing. Untick it: Steam gets them back. | R0-5 |
| H-3 | With "Hide Padwight's controller from Steam" on, turn Steam Input off for Slay the Spire 2 and start it. Run `padwight debug focus <the game's pid>`. If the `SDL_GAMECONTROLLER_IGNORE_DEVICES` line says it hides Padwight's IDs, the game can't see Padwight's controller, and the concern in R0-7 holds. | R0-7 |
| H-4 | Start a game from Flatpak Heroic, on each desktop with focus tracking (KWin, GNOME, Sway, Hyprland). `debug focus` names the game's process, and its setup switches on. Where the desktop passes on the pid an X11 window gives itself, this exercises the sandbox lookup. | R0-2 |
| H-5 | On a DualSense or Steam Deck touchpad in mouse mode with scrolling off, put two fingers down, lift the first, put a finger down again, and move each in turn. Only the finger that stayed down moves the pointer. | R2-3 |
